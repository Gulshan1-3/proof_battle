use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::create_app_with_matchmaker;
use proof_battle_server::game::matchmaker::Matchmaker;
use proof_battle_server::game::matchmaker::MatchmakerHandle;
use proof_battle_server::ws::message::{MatchOutcome, ServerMessage, VerdictStatus};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_test_server(
    round_duration: Option<Duration>,
) -> (String, Arc<Config>, MatchmakerHandle) {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());
    env.insert("LEAN_POOL_SIZE".to_string(), "8".to_string());
    env.insert("RECONNECT_GRACE_WINDOW_MS".to_string(), "200".to_string());

    let config = Arc::new(Config::from_map(&env).expect("valid config"));
    let matchmaker = if let Some(dur) = round_duration {
        Matchmaker::spawn_with_round_duration(config.clone(), dur)
    } else {
        Matchmaker::spawn(config.clone())
    };

    let app = create_app_with_matchmaker(config.clone(), matchmaker.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let ws_url = format!("ws://127.0.0.1:{}/ws", addr.port());

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (ws_url, config, matchmaker)
}

async fn connect_player(
    ws_url: &str,
) -> (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    String,
) {
    let (mut ws, _) = connect_async(ws_url).await.expect("connect");
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": null
    });
    ws.send(Message::Text(hello.to_string()))
        .await
        .expect("send hello");

    let m1 = ws.next().await.expect("msg").expect("ok");
    let welcome: ServerMessage = serde_json::from_str(m1.to_text().unwrap()).expect("welcome");
    let player_id = match welcome {
        ServerMessage::Welcome { player_id, .. } => player_id,
        other => panic!("expected Welcome, got {other:?}"),
    };

    let _server_time = ws.next().await.expect("server time").expect("ok");

    let qj = serde_json::json!({ "type": "QueueJoin" });
    ws.send(Message::Text(qj.to_string()))
        .await
        .expect("send queue join");

    let _queue_status = ws.next().await.expect("queue status").expect("ok");

    (ws, player_id)
}

fn solve_challenge(goal: &str) -> &'static str {
    if goal.contains("n + 0 = n") {
        "  intro n\n  simp\n"
    } else if goal.contains("a + b = b + a") {
        "  intro a b\n  omega\n"
    } else if goal.contains("P ∧ Q") {
        "  intro P Q ⟨hp, hq⟩\n  exact ⟨hq, hp⟩\n"
    } else {
        "  simp\n"
    }
}

// -----------------------------------------------------------------------------
// Test 1: Two clients -> matched -> both receive RoundStart with an identical goal
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_two_clients_matched_identical_challenge() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    let (mut ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    // Drain Welcome, ServerTime, QueueStatus for P1 & P2
    let mut p1_goal = String::new();
    let mut p1_imports = Vec::new();
    for _ in 0..5 {
        let m = ws1.next().await.expect("msg").expect("ok");
        if let Ok(ServerMessage::RoundStart { problem, .. }) =
            serde_json::from_str(m.to_text().unwrap())
        {
            p1_goal = problem.goal;
            p1_imports = problem.imports;
            break;
        }
    }

    let mut p2_goal = String::new();
    let mut p2_imports = Vec::new();
    for _ in 0..5 {
        let m = ws2.next().await.expect("msg").expect("ok");
        if let Ok(ServerMessage::RoundStart { problem, .. }) =
            serde_json::from_str(m.to_text().unwrap())
        {
            p2_goal = problem.goal;
            p2_imports = problem.imports;
            break;
        }
    }

    assert!(!p1_goal.is_empty(), "P1 goal should not be empty");
    assert_eq!(p1_goal, p2_goal, "Both players must receive identical goal");
    assert_eq!(
        p1_imports, p2_imports,
        "Both players must receive identical imports"
    );
}

// -----------------------------------------------------------------------------
// Test 2: Concurrent submits from two rooms -> the two verify windows OVERLAP
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_concurrent_submits_from_two_rooms_overlap() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    // Create Room A (P1, P2)
    let (mut p1_a, _) = connect_player(&ws_url).await;
    let (p2_a, _) = connect_player(&ws_url).await;

    let mut goal_a = String::new();
    for _ in 0..5 {
        let m = p1_a.next().await.expect("msg").expect("ok");
        if let Ok(ServerMessage::RoundStart { problem, .. }) =
            serde_json::from_str(m.to_text().unwrap())
        {
            goal_a = problem.goal;
            break;
        }
    }

    // Create Room B (P1, P2)
    let (mut p1_b, _) = connect_player(&ws_url).await;
    let (p2_b, _) = connect_player(&ws_url).await;

    let mut goal_b = String::new();
    for _ in 0..5 {
        let m = p1_b.next().await.expect("msg").expect("ok");
        if let Ok(ServerMessage::RoundStart { problem, .. }) =
            serde_json::from_str(m.to_text().unwrap())
        {
            goal_b = problem.goal;
            break;
        }
    }

    let code_a = solve_challenge(&goal_a);
    let code_b = solve_challenge(&goal_b);

    // Spawn concurrent submissions from Room A and Room B
    let submit_a = tokio::spawn(async move {
        let start = Instant::now();
        let payload = serde_json::json!({
            "type": "ProofRequest",
            "req_id": "req_a",
            "code": code_a,
            "intent": "Submit"
        });
        p1_a.send(Message::Text(payload.to_string())).await.unwrap();

        // Wait for Verdict
        loop {
            let m = p1_a.next().await.unwrap().unwrap();
            if let Ok(ServerMessage::Verdict { verdict, .. }) =
                serde_json::from_str(m.to_text().unwrap())
            {
                let end = Instant::now();
                assert_eq!(verdict, VerdictStatus::Accepted);
                return (start, end);
            }
        }
    });

    let submit_b = tokio::spawn(async move {
        let start = Instant::now();
        let payload = serde_json::json!({
            "type": "ProofRequest",
            "req_id": "req_b",
            "code": code_b,
            "intent": "Submit"
        });
        p1_b.send(Message::Text(payload.to_string())).await.unwrap();

        // Wait for Verdict
        loop {
            let m = p1_b.next().await.unwrap().unwrap();
            if let Ok(ServerMessage::Verdict { verdict, .. }) =
                serde_json::from_str(m.to_text().unwrap())
            {
                let end = Instant::now();
                assert_eq!(verdict, VerdictStatus::Accepted);
                return (start, end);
            }
        }
    });

    let (res_a, res_b) = tokio::join!(submit_a, submit_b);
    let (start_a, end_a) = res_a.unwrap();
    let (start_b, end_b) = res_b.unwrap();

    let base = start_a.min(start_b);
    let dur_a_start = start_a.duration_since(base).as_millis();
    let dur_a_end = end_a.duration_since(base).as_millis();
    let dur_b_start = start_b.duration_since(base).as_millis();
    let dur_b_end = end_b.duration_since(base).as_millis();

    println!(
        "EVIDENCE: Room A execution window: [{} ms - {} ms] (duration: {} ms)",
        dur_a_start,
        dur_a_end,
        dur_a_end - dur_a_start
    );
    println!(
        "EVIDENCE: Room B execution window: [{} ms - {} ms] (duration: {} ms)",
        dur_b_start,
        dur_b_end,
        dur_b_end - dur_b_start
    );

    let overlap_start = start_a.max(start_b);
    let overlap_end = end_a.min(end_b);
    assert!(
        overlap_start < overlap_end,
        "Verification windows must overlap in actor model! Room A: [{}..{}], Room B: [{}..{}]",
        dur_a_start,
        dur_a_end,
        dur_b_start,
        dur_b_end
    );
    drop(p2_a);
    drop(p2_b);
}

// -----------------------------------------------------------------------------
// Test 3: One player disconnects mid-game -> the other receives RoundEnd and registry length is 0 within 2s
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_player_disconnect_cleans_registry() {
    let (ws_url, _config, mm) = spawn_test_server(None).await;

    let (ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    // Wait for match creation
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(mm.get_registry_len().await, 1);

    // Drop player 1 (disconnect)
    drop(ws1);

    // Player 2 receives RoundEnd
    let mut got_end = false;
    let timeout = Instant::now() + Duration::from_secs(2);
    while Instant::now() < timeout {
        let res = tokio::time::timeout(Duration::from_millis(500), ws2.next()).await;
        if let Ok(Some(Ok(m))) = res {
            let parsed = serde_json::from_str::<ServerMessage>(m.to_text().unwrap());
            if matches!(parsed, Ok(ServerMessage::RoundEnd { .. })) {
                got_end = true;
                break;
            }
        }
    }
    assert!(
        got_end,
        "Player 2 must receive RoundEnd on opponent disconnect"
    );

    // Registry length must return to 0 within 2s
    let mut reg_len = mm.get_registry_len().await;
    let poll_deadline = Instant::now() + Duration::from_secs(2);
    while reg_len > 0 && Instant::now() < poll_deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
        reg_len = mm.get_registry_len().await;
    }
    assert_eq!(
        reg_len, 0,
        "Registry length must return to 0 after disconnect"
    );
}

// -----------------------------------------------------------------------------
// Test 4: Both players submit correct proof simultaneously -> exactly ONE RoundEnd per player & agree on winner
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_simultaneous_correct_submits_single_winner() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    let (mut ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    let mut goal = String::new();
    for _ in 0..5 {
        let m1 = ws1.next().await.unwrap().unwrap();
        if let Ok(ServerMessage::RoundStart { problem, .. }) =
            serde_json::from_str(m1.to_text().unwrap())
        {
            goal = problem.goal;
            break;
        }
    }

    let proof = solve_challenge(&goal);
    let payload = serde_json::json!({
        "type": "ProofRequest",
        "req_id": "simultaneous_req",
        "code": proof,
        "intent": "Submit"
    });

    // Send submits almost simultaneously
    ws1.send(Message::Text(payload.to_string())).await.unwrap();
    ws2.send(Message::Text(payload.to_string())).await.unwrap();

    let mut p1_winners = Vec::new();
    let mut p2_winners = Vec::new();

    let timeout = Instant::now() + Duration::from_secs(10);
    while Instant::now() < timeout && (p1_winners.is_empty() || p2_winners.is_empty()) {
        tokio::select! {
            Some(Ok(m1)) = ws1.next() => {
                if let Ok(ServerMessage::RoundEnd { outcome: MatchOutcome::Won, winner_id: Some(winner), .. }) =
                    serde_json::from_str(m1.to_text().unwrap())
                {
                    p1_winners.push(winner);
                }
            }
            Some(Ok(m2)) = ws2.next() => {
                if let Ok(ServerMessage::RoundEnd { outcome: MatchOutcome::Won, winner_id: Some(winner), .. }) =
                    serde_json::from_str(m2.to_text().unwrap())
                {
                    p2_winners.push(winner);
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }

    assert_eq!(p1_winners.len(), 1, "P1 must receive exactly 1 RoundEnd");
    assert_eq!(p2_winners.len(), 1, "P2 must receive exactly 1 RoundEnd");
    assert_eq!(
        p1_winners[0], p2_winners[0],
        "Both players must agree on identical winner"
    );
}

// -----------------------------------------------------------------------------
// Test 5: Deadline expiry with no submission -> RoundEnd { outcome: Draw } for both
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_deadline_expiry_draw() {
    let (ws_url, _config, _mm) = spawn_test_server(Some(Duration::from_millis(600))).await;

    let (mut ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    let mut p1_draw = false;
    let mut p2_draw = false;

    let timeout = Instant::now() + Duration::from_secs(3);
    while Instant::now() < timeout && (!p1_draw || !p2_draw) {
        tokio::select! {
            Some(Ok(m1)) = ws1.next() => {
                if let Ok(ServerMessage::RoundEnd { outcome: MatchOutcome::Draw, .. }) =
                    serde_json::from_str(m1.to_text().unwrap())
                {
                    p1_draw = true;
                }
            }
            Some(Ok(m2)) = ws2.next() => {
                if let Ok(ServerMessage::RoundEnd { outcome: MatchOutcome::Draw, .. }) =
                    serde_json::from_str(m2.to_text().unwrap())
                {
                    p2_draw = true;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }

    assert!(p1_draw, "Player 1 must receive RoundEnd Draw on timeout");
    assert!(p2_draw, "Player 2 must receive RoundEnd Draw on timeout");
}

// -----------------------------------------------------------------------------
// Test 6: Registry length is 0 after 50 sequential connect/match/end cycles (no leak)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_50_sequential_cycles_no_room_leak() {
    let (ws_url, _config, mm) = spawn_test_server(None).await;

    for _ in 0..50 {
        let (mut ws1, _) = connect_player(&ws_url).await;
        let (mut ws2, _) = connect_player(&ws_url).await;

        // Wait for match creation
        let _ = ws1.next().await;
        let _ = ws2.next().await;

        // Player 1 resigns to end match immediately
        let resign_payload = serde_json::json!({ "type": "Resign" });
        let _ = ws1.send(Message::Text(resign_payload.to_string())).await;

        // Wait for RoundEnd on P2
        let timeout = Instant::now() + Duration::from_secs(1);
        while Instant::now() < timeout {
            let res = tokio::time::timeout(Duration::from_millis(200), ws2.next()).await;
            if let Ok(Some(Ok(m))) = res {
                let parsed = serde_json::from_str::<ServerMessage>(m.to_text().unwrap());
                if matches!(parsed, Ok(ServerMessage::RoundEnd { .. })) {
                    break;
                }
            }
        }

        drop(ws1);
        drop(ws2);
    }

    // Wait for all 50 rooms to shut down and unregister
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut reg_len = mm.get_registry_len().await;
    let mut wait_len = mm.get_waiting_len().await;

    while (reg_len > 0 || wait_len > 0) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
        reg_len = mm.get_registry_len().await;
        wait_len = mm.get_waiting_len().await;
    }

    println!(
        "EVIDENCE: After 50 cycles, registry rooms count: {}, waiting players: {}",
        reg_len, wait_len
    );
    assert_eq!(reg_len, 0, "No rooms should be leaked after 50 cycles");
    assert_eq!(wait_len, 0, "No players should be waiting after 50 cycles");
}
