use chrono::Utc;
use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::game::history::{HistoryStore, MatchRecord};
use proof_battle_server::game::matchmaker::Matchmaker;
use proof_battle_server::game::session::InMemorySessionStore;
use proof_battle_server::message::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

async fn http_get(addr: &str, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.expect("connect tcp");
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, addr
    );
    stream.write_all(req.as_bytes()).await.expect("write req");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).await.expect("read resp");

    let status_line = resp.lines().next().unwrap_or("");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let body = resp.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status_code, body)
}

fn test_config() -> Arc<Config> {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());
    env.insert("LEAN_POOL_SIZE".to_string(), "4".to_string());
    Arc::new(Config::from_map(&env).expect("valid config"))
}

#[tokio::test]
async fn test_history_store_in_memory_records_and_retrieves() {
    let store = HistoryStore::new(None);

    let p1 = Uuid::new_v4();
    let p2 = Uuid::new_v4();
    let match_id = Uuid::new_v4();

    let record = MatchRecord {
        id: match_id,
        room_id: Uuid::new_v4(),
        player1_id: p1,
        player1_username: Some("Alice".to_string()),
        player1_elo_before: 1200,
        player1_elo_after: 1216,
        elo_delta_p1: 16,
        player2_id: p2,
        player2_username: Some("Bob".to_string()),
        player2_elo_before: 1200,
        player2_elo_after: 1184,
        elo_delta_p2: -16,
        problem_id: None,
        problem_goal: "∀ n : ℕ, n + 0 = n".to_string(),
        problem_category: "logic".to_string(),
        problem_difficulty: 1,
        winner_id: Some(p1),
        outcome: "Won".to_string(),
        winning_proof: Some("intro n\nsimp".to_string()),
        canonical_proof: None,
        rated: true,
        duration_ms: 4500,
        created_at: Utc::now(),
    };

    store.record_match(record).await;

    // Check player 1 history
    let p1_history = store.get_player_history(p1, 10).await;
    assert_eq!(p1_history.len(), 1);
    assert_eq!(p1_history[0].outcome, "Won");
    assert_eq!(p1_history[0].opponent_id, p2);
    assert_eq!(p1_history[0].opponent_username.as_deref(), Some("Bob"));
    assert_eq!(p1_history[0].elo_delta, 16);

    // Check player 2 history
    let p2_history = store.get_player_history(p2, 10).await;
    assert_eq!(p2_history.len(), 1);
    assert_eq!(p2_history[0].outcome, "Lost");
    assert_eq!(p2_history[0].opponent_id, p1);
    assert_eq!(p2_history[0].opponent_username.as_deref(), Some("Alice"));
    assert_eq!(p2_history[0].elo_delta, -16);

    // Check match detail
    let detail = store.get_match_detail(match_id).await;
    assert!(detail.is_some());
    let detail = detail.unwrap();
    assert_eq!(detail.winning_proof.as_deref(), Some("intro n\nsimp"));
    assert_eq!(detail.duration_ms, 4500);
}

#[tokio::test]
async fn test_history_store_player_stats() {
    let store = HistoryStore::new(None);
    let p1 = Uuid::new_v4();
    let p2 = Uuid::new_v4();

    // Match 1: p1 wins in logic
    store
        .record_match(MatchRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            player1_id: p1,
            player1_username: Some("Player1".to_string()),
            player1_elo_before: 1200,
            player1_elo_after: 1216,
            elo_delta_p1: 16,
            player2_id: p2,
            player2_username: Some("Player2".to_string()),
            player2_elo_before: 1200,
            player2_elo_after: 1184,
            elo_delta_p2: -16,
            problem_id: None,
            problem_goal: "True".to_string(),
            problem_category: "logic".to_string(),
            problem_difficulty: 1,
            winner_id: Some(p1),
            outcome: "Won".to_string(),
            winning_proof: Some("trivial".to_string()),
            canonical_proof: None,
            rated: true,
            duration_ms: 2000,
            created_at: Utc::now(),
        })
        .await;

    // Match 2: p2 wins in algebra
    store
        .record_match(MatchRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            player1_id: p1,
            player1_username: Some("Player1".to_string()),
            player1_elo_before: 1216,
            player1_elo_after: 1200,
            elo_delta_p1: -16,
            player2_id: p2,
            player2_username: Some("Player2".to_string()),
            player2_elo_before: 1184,
            player2_elo_after: 1200,
            elo_delta_p2: 16,
            problem_id: None,
            problem_goal: "x + y = y + x".to_string(),
            problem_category: "algebra".to_string(),
            problem_difficulty: 2,
            winner_id: Some(p2),
            outcome: "Lost".to_string(),
            winning_proof: Some("ring".to_string()),
            canonical_proof: None,
            rated: true,
            duration_ms: 3000,
            created_at: Utc::now(),
        })
        .await;

    let stats = store.get_player_stats(p1).await;
    assert_eq!(stats.games_played, 2);
    assert_eq!(stats.wins, 1);
    assert_eq!(stats.losses, 1);
    assert_eq!(stats.draws, 0);
    assert!((stats.win_rate - 50.0).abs() < f64::EPSILON);
    assert_eq!(stats.categories.len(), 2);
}

#[tokio::test]
async fn test_history_rest_api_endpoints() {
    let config = test_config();
    let history = Arc::new(HistoryStore::new(None));
    let matchmaker = Matchmaker::spawn_with_history(config.clone(), history.clone());
    let sessions = Arc::new(InMemorySessionStore::new(Duration::from_secs(60)));

    let p1 = Uuid::new_v4();
    let p2 = Uuid::new_v4();
    let match_id = Uuid::new_v4();

    history
        .record_match(MatchRecord {
            id: match_id,
            room_id: Uuid::new_v4(),
            player1_id: p1,
            player1_username: Some("Carol".to_string()),
            player1_elo_before: 1200,
            player1_elo_after: 1220,
            elo_delta_p1: 20,
            player2_id: p2,
            player2_username: Some("Dave".to_string()),
            player2_elo_before: 1200,
            player2_elo_after: 1180,
            elo_delta_p2: -20,
            problem_id: None,
            problem_goal: "∀ n : ℕ, n = n".to_string(),
            problem_category: "logic".to_string(),
            problem_difficulty: 1,
            winner_id: Some(p1),
            outcome: "Won".to_string(),
            winning_proof: Some("intro n\nrfl".to_string()),
            canonical_proof: None,
            rated: true,
            duration_ms: 1500,
            created_at: Utc::now(),
        })
        .await;

    let app = proof_battle_server::create_app_with_components_and_history(
        config, matchmaker, sessions, history,
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind port");
    let addr = listener.local_addr().expect("local addr");
    let host_port = addr.to_string();

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    // 1. Test /api/history
    let (code, body) = http_get(&host_port, &format!("/api/history?player_id={p1}")).await;
    assert_eq!(code, 200);
    assert!(body.contains("Dave"));
    assert!(body.contains("Won"));

    // 2. Test /api/matches/:id
    let (code, body) = http_get(&host_port, &format!("/api/matches/{match_id}")).await;
    assert_eq!(code, 200);
    assert!(body.contains("intro n\\nrfl"));

    // 3. Test /api/matches/:id with non-existent id
    let random_id = Uuid::new_v4();
    let (code, _) = http_get(&host_port, &format!("/api/matches/{random_id}")).await;
    assert_eq!(code, 404);

    // 4. Test /api/stats
    let (code, body) = http_get(&host_port, &format!("/api/stats?player_id={p1}")).await;
    assert_eq!(code, 200);
    assert!(body.contains("\"games_played\":1"));
    assert!(body.contains("\"wins\":1"));
}

#[tokio::test]
async fn test_game_completion_automatically_records_in_history() {
    let config = test_config();
    let history = Arc::new(HistoryStore::new(None));
    let matchmaker = Matchmaker::spawn_with_history(config.clone(), history.clone());
    let sessions = Arc::new(InMemorySessionStore::new(Duration::from_secs(60)));

    let app = proof_battle_server::create_app_with_components_and_history(
        config,
        matchmaker,
        sessions,
        history.clone(),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind port");
    let addr = listener.local_addr().expect("local addr");
    let ws_url = format!("ws://{}/ws", addr);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    // Connect Client 1
    let (mut ws1, _) = connect_async(&ws_url).await.expect("connect client 1");
    let hello1_msg = ClientMessage::Hello {
        version: 1,
        token: None,
        username: Some("Champion1".to_string()),
    };
    ws1.send(Message::Text(serde_json::to_string(&hello1_msg).unwrap()))
        .await
        .unwrap();

    let welcome1 = ws1.next().await.unwrap().unwrap();
    let p1_id = match serde_json::from_str::<ServerMessage>(&welcome1.into_text().unwrap()).unwrap()
    {
        ServerMessage::Welcome { player_id, .. } => Uuid::parse_str(&player_id).unwrap(),
        other => panic!("Expected Welcome, got {other:?}"),
    };

    // Connect Client 2
    let (mut ws2, _) = connect_async(&ws_url).await.expect("connect client 2");
    let hello2_msg = ClientMessage::Hello {
        version: 1,
        token: None,
        username: Some("Challenger2".to_string()),
    };
    ws2.send(Message::Text(serde_json::to_string(&hello2_msg).unwrap()))
        .await
        .unwrap();

    let welcome2 = ws2.next().await.unwrap().unwrap();
    let _p2_id =
        match serde_json::from_str::<ServerMessage>(&welcome2.into_text().unwrap()).unwrap() {
            ServerMessage::Welcome { player_id, .. } => Uuid::parse_str(&player_id).unwrap(),
            other => panic!("Expected Welcome, got {other:?}"),
        };

    // Both join queue
    ws1.send(Message::Text(
        serde_json::to_string(&ClientMessage::QueueJoin {}).unwrap(),
    ))
    .await
    .unwrap();

    ws2.send(Message::Text(
        serde_json::to_string(&ClientMessage::QueueJoin {}).unwrap(),
    ))
    .await
    .unwrap();

    // Consume match found & round start on ws1
    let mut current_goal = String::new();
    while let Some(Ok(msg)) = ws1.next().await {
        let text = match msg.into_text() {
            Ok(t) => t,
            _ => continue,
        };
        if let Ok(ServerMessage::RoundStart { problem, .. }) =
            serde_json::from_str::<ServerMessage>(&text)
        {
            current_goal = problem.goal;
            break;
        }
    }

    // Submit winning proof from client 1
    let proof_code = if current_goal.contains("n + 0 = n") {
        "intro n\nsimp".to_string()
    } else if current_goal.contains("a + b = b + a") {
        "intro a b\nomega".to_string()
    } else if current_goal.contains("P ∧ Q") {
        "intro P Q ⟨hp, hq⟩\nexact ⟨hq, hp⟩".to_string()
    } else {
        "simp".to_string()
    };

    let submit_req = ClientMessage::ProofRequest {
        req_id: "test_win_req".to_string(),
        code: proof_code,
        intent: ProofIntent::Submit,
    };
    ws1.send(Message::Text(serde_json::to_string(&submit_req).unwrap()))
        .await
        .unwrap();

    // Wait for RoundEnd
    while let Some(Ok(msg)) = ws1.next().await {
        let text = match msg.into_text() {
            Ok(t) => t,
            _ => continue,
        };
        if let Ok(ServerMessage::RoundEnd { outcome, .. }) =
            serde_json::from_str::<ServerMessage>(&text)
        {
            assert_eq!(outcome, MatchOutcome::Won);
            break;
        }
    }

    // Give store a moment to record
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify history store has recorded the match automatically!
    let history = history.get_player_history(p1_id, 10).await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].outcome, "Won");
    assert_eq!(history[0].opponent_username.as_deref(), Some("Challenger2"));
}
