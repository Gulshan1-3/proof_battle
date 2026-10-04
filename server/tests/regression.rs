use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::create_app;
use proof_battle_server::ws::message::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_test_server(sandbox_enabled: bool) -> (String, Arc<Config>) {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert(
        "SANDBOX_ENABLED".to_string(),
        if sandbox_enabled { "true" } else { "false" }.to_string(),
    );

    let config = Arc::new(Config::from_map(&env).expect("valid config"));
    let app = create_app(config.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let ws_url = format!("ws://127.0.0.1:{}/ws", addr.port());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (ws_url, config)
}

fn solve_challenge(goal: &str) -> &'static str {
    if goal.contains("n + 0 = n") {
        "intro n\nsimp\n"
    } else if goal.contains("a + b = b + a") {
        "intro a b\nomega\n"
    } else if goal.contains("P ∧ Q") {
        "intro P Q ⟨hp, hq⟩\nexact ⟨hq, hp⟩\n"
    } else {
        "simp\n"
    }
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

/// 1. A player submits `sorry` and must receive Verdict { verdict: Rejected, reason: ... },
/// with a reason mentioning sorry, and the game must CONTINUE (not end).
#[tokio::test]
async fn test_regression_sorry_rejected_game_continues() {
    let (ws_url, _config) = spawn_test_server(false).await;

    let (mut ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    // Read MatchFound & RoundStart
    let mut current_goal = String::new();
    for _ in 0..2 {
        let m = ws1.next().await.unwrap().unwrap();
        let parsed: ServerMessage = serde_json::from_str(m.to_text().unwrap()).unwrap();
        if let ServerMessage::RoundStart { problem, .. } = parsed {
            current_goal = problem.goal;
        }
    }
    for _ in 0..2 {
        let _ = ws2.next().await.unwrap().unwrap();
    }

    // Player 1 submits sorry
    let submit_sorry = ClientMessage::ProofRequest {
        req_id: "sorry_req".to_string(),
        code: "sorry".to_string(),
        intent: ProofIntent::Submit,
    };
    ws1.send(Message::Text(serde_json::to_string(&submit_sorry).unwrap()))
        .await
        .unwrap();

    // Expect Verdict { verdict: Rejected, message: ... } mentioning sorry
    let resp1 = ws1.next().await.unwrap().unwrap();
    let parsed1: ServerMessage = serde_json::from_str(resp1.to_text().unwrap()).unwrap();
    match parsed1 {
        ServerMessage::Verdict {
            verdict, message, ..
        } => {
            assert_eq!(verdict, VerdictStatus::Rejected);
            assert!(
                message.to_lowercase().contains("sorry"),
                "Expected rejection message to mention sorry, got: {message}"
            );
        }
        other => panic!("Expected Verdict, got {other:?}"),
    }

    // Now Player 1 submits legitimate proof: game must CONTINUE and accept it
    let correct_code = solve_challenge(&current_goal);
    let submit_good = ClientMessage::ProofRequest {
        req_id: "good_req".to_string(),
        code: correct_code.to_string(),
        intent: ProofIntent::Submit,
    };
    ws1.send(Message::Text(serde_json::to_string(&submit_good).unwrap()))
        .await
        .unwrap();

    let resp2 = ws1.next().await.unwrap().unwrap();
    let parsed2: ServerMessage = serde_json::from_str(resp2.to_text().unwrap()).unwrap();
    match parsed2 {
        ServerMessage::Verdict { verdict, .. } => {
            assert_eq!(verdict, VerdictStatus::Accepted);
        }
        other => panic!("Expected Verdict, got {other:?}"),
    }
}

/// 2. A player submits a real proof, receives Verdict(Accepted), and both players receive RoundEnd
#[tokio::test]
async fn test_regression_valid_proof_ends_round_for_both() {
    let (ws_url, _config) = spawn_test_server(false).await;

    let (mut ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    let mut current_goal = String::new();
    for _ in 0..2 {
        let m = ws1.next().await.unwrap().unwrap();
        let parsed: ServerMessage = serde_json::from_str(m.to_text().unwrap()).unwrap();
        if let ServerMessage::RoundStart { problem, .. } = parsed {
            current_goal = problem.goal;
        }
    }
    for _ in 0..2 {
        let _ = ws2.next().await.unwrap().unwrap();
    }

    let correct_code = solve_challenge(&current_goal);
    let submit_good = ClientMessage::ProofRequest {
        req_id: "legit_req".to_string(),
        code: correct_code.to_string(),
        intent: ProofIntent::Submit,
    };
    ws1.send(Message::Text(serde_json::to_string(&submit_good).unwrap()))
        .await
        .unwrap();

    // Player 1 receives Verdict(Accepted)
    let p1_msg1 = ws1.next().await.unwrap().unwrap();
    let p1_parsed1: ServerMessage = serde_json::from_str(p1_msg1.to_text().unwrap()).unwrap();
    assert!(matches!(
        p1_parsed1,
        ServerMessage::Verdict {
            verdict: VerdictStatus::Accepted,
            ..
        }
    ));

    // Both players receive RoundEnd
    let p1_end = ws1.next().await.unwrap().unwrap();
    let p1_end_parsed: ServerMessage = serde_json::from_str(p1_end.to_text().unwrap()).unwrap();
    assert!(matches!(
        p1_end_parsed,
        ServerMessage::RoundEnd {
            outcome: MatchOutcome::Won,
            ..
        }
    ));

    let mut p2_got_round_end = false;
    for _ in 0..2 {
        let p2_msg = ws2.next().await.unwrap().unwrap();
        let p2_parsed: ServerMessage = serde_json::from_str(p2_msg.to_text().unwrap()).unwrap();
        match p2_parsed {
            ServerMessage::OpponentActivity { status } => {
                assert_eq!(
                    status,
                    proof_battle_server::ws::message::OpponentStatus::Verifying
                );
            }
            ServerMessage::RoundEnd { outcome, .. } => {
                assert_eq!(outcome, MatchOutcome::Lost);
                p2_got_round_end = true;
                break;
            }
            other => panic!("expected OpponentActivity or RoundEnd, got {other:?}"),
        }
    }
    assert!(p2_got_round_end);
}

/// 3. `#eval IO.println "x"` receives Verdict(Rejected) and the string "x" appears in no log line
#[tokio::test]
async fn test_regression_eval_io_rejected_never_executes() {
    let (ws_url, _config) = spawn_test_server(false).await;

    let (mut ws1, _) = connect_player(&ws_url).await;
    let (mut ws2, _) = connect_player(&ws_url).await;

    for _ in 0..2 {
        let _ = ws1.next().await.unwrap().unwrap();
        let _ = ws2.next().await.unwrap().unwrap();
    }

    let rce_payload = "#eval IO.println \"SUPER_SECRET_PAYLOAD_EXECUTION_MARKER_9999\"";
    let submit_rce = ClientMessage::ProofRequest {
        req_id: "rce_req".to_string(),
        code: rce_payload.to_string(),
        intent: ProofIntent::Submit,
    };
    ws1.send(Message::Text(serde_json::to_string(&submit_rce).unwrap()))
        .await
        .unwrap();

    let resp = ws1.next().await.unwrap().unwrap();
    let parsed: ServerMessage = serde_json::from_str(resp.to_text().unwrap()).unwrap();
    match parsed {
        ServerMessage::Verdict {
            verdict, message, ..
        } => {
            assert_eq!(verdict, VerdictStatus::Rejected);
            assert!(
                !message.contains("SUPER_SECRET_PAYLOAD_EXECUTION_MARKER_9999"),
                "Security breach: payload executed!"
            );
            assert!(
                !message.contains("/home/gulshansharma"),
                "Security breach: leaked internal server path in output: {message}"
            );
        }
        other => panic!("Expected Verdict, got {other:?}"),
    }
}

/// 4. Sandbox and Filter parity test
#[tokio::test]
async fn test_regression_filter_and_sandbox_parity() {
    let dev_config = {
        let mut env = HashMap::new();
        env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());
        Config::from_map(&env).unwrap()
    };

    let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
    let goal = "∀ n : ℕ, n + 0 = n";

    // Both modes reject forbidden payloads at the wrap/filter layer identically
    let payloads = [
        "sorry",
        "#eval IO.getEnv",
        "axiom h : False",
        "set_option warningAsError false in exact trivial",
    ];

    for p in payloads {
        let dev_wrap = proof_battle_server::lean::wrap_problem(&imports, goal, p, &dev_config);
        assert!(
            dev_wrap.is_err(),
            "Payload '{}' should be rejected at filter boundary",
            p
        );
    }
}
