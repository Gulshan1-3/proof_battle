use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::create_app;
use proof_battle_server::ws::message::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_test_server() -> (String, Arc<Config>) {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());

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
        "  intro n\n  simp\n"
    } else if goal.contains("a + b = b + a") {
        "  intro a b\n  omega\n"
    } else if goal.contains("P ∧ Q") {
        "  intro P Q ⟨hp, hq⟩\n  exact ⟨hq, hp⟩\n"
    } else {
        "  simp\n"
    }
}

async fn connect_player_with_id(
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

#[tokio::test]
async fn test_submit_proof_accepted_and_answered() {
    let (ws_url, _config) = spawn_test_server().await;

    // Connect Player 1 & 2
    let (mut ws1, p1_id) = connect_player_with_id(&ws_url).await;
    let (mut ws2, p2_id) = connect_player_with_id(&ws_url).await;

    // Both should receive MatchFound and RoundStart
    let mut p1_got_match = false;
    let mut current_goal = String::new();
    for _ in 0..2 {
        let m = ws1.next().await.expect("next").expect("msg");
        let parsed: ServerMessage = serde_json::from_str(m.to_text().expect("text")).unwrap();
        match parsed {
            ServerMessage::MatchFound { opponent, .. } => {
                assert_eq!(opponent.player_id, p2_id);
                p1_got_match = true;
            }
            ServerMessage::RoundStart { problem, .. } => {
                assert!(!problem.goal.is_empty());
                current_goal = problem.goal;
            }
            other => panic!("unexpected message for p1: {other:?}"),
        }
    }
    assert!(p1_got_match && !current_goal.is_empty());

    let mut p2_got_match = false;
    let mut p2_got_round_start = false;
    for _ in 0..2 {
        let m = ws2.next().await.expect("next").expect("msg");
        let parsed: ServerMessage = serde_json::from_str(m.to_text().expect("text")).unwrap();
        match parsed {
            ServerMessage::MatchFound { opponent, .. } => {
                assert_eq!(opponent.player_id, p1_id);
                p2_got_match = true;
            }
            ServerMessage::RoundStart { .. } => {
                p2_got_round_start = true;
            }
            other => panic!("unexpected message for p2: {other:?}"),
        }
    }
    assert!(p2_got_match && p2_got_round_start);

    // Player 1 submits proof without player_id using ProofRequest
    let proof_code = solve_challenge(&current_goal);
    let submit_payload = serde_json::json!({
        "type": "ProofRequest",
        "req_id": "req_submit_1",
        "code": proof_code,
        "intent": "Submit"
    });
    ws1.send(Message::Text(submit_payload.to_string()))
        .await
        .expect("send submit");

    // Player 1 receives Verdict and RoundEnd
    let mut p1_got_verdict = false;
    let mut p1_got_round_end = false;
    for _ in 0..2 {
        let m = ws1.next().await.expect("next").expect("msg");
        let parsed: ServerMessage = serde_json::from_str(m.to_text().expect("text")).unwrap();
        match parsed {
            ServerMessage::Verdict { verdict, .. } => {
                assert_eq!(verdict, VerdictStatus::Accepted);
                p1_got_verdict = true;
            }
            ServerMessage::RoundEnd {
                outcome, winner_id, ..
            } => {
                assert_eq!(outcome, MatchOutcome::Won);
                assert_eq!(winner_id, Some(p1_id.clone()));
                p1_got_round_end = true;
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
    assert!(p1_got_verdict && p1_got_round_end);

    // Player 2 receives RoundEnd
    let m2 = ws2.next().await.expect("next").expect("msg");
    let parsed2: ServerMessage = serde_json::from_str(m2.to_text().expect("text")).unwrap();
    match parsed2 {
        ServerMessage::RoundEnd {
            outcome, winner_id, ..
        } => {
            assert_eq!(outcome, MatchOutcome::Won);
            assert_eq!(winner_id, Some(p1_id));
        }
        other => panic!("expected RoundEnd, got {other:?}"),
    }
}

#[tokio::test]
async fn test_submit_proof_with_player_id_rejected() {
    let (ws_url, _config) = spawn_test_server().await;

    // Connect Player 1 & 2
    let (mut ws1, _p1_id) = connect_player_with_id(&ws_url).await;
    let (mut ws2, _p2_id) = connect_player_with_id(&ws_url).await;

    // Drain MatchFound & RoundStart for both
    let _ = ws1.next().await;
    let _ = ws1.next().await;
    let _ = ws2.next().await;
    let round_start2_raw = ws2.next().await.expect("p2 round start").expect("msg");
    let parsed_rs2: ServerMessage =
        serde_json::from_str(round_start2_raw.to_text().unwrap()).unwrap();
    let goal2 = match parsed_rs2 {
        ServerMessage::RoundStart { problem, .. } => problem.goal,
        other => panic!("expected RoundStart, got {other:?}"),
    };

    // Player 1 attempts to forge identity by sending player_id in ProofRequest
    let malicious_payload = serde_json::json!({
        "type": "ProofRequest",
        "req_id": "malicious_req",
        "code": "  intro n\n  simp\n",
        "intent": "Submit",
        "player_id": "00000000-0000-0000-0000-000000000001"
    });
    ws1.send(Message::Text(malicious_payload.to_string()))
        .await
        .expect("send malicious payload");

    // Player 1 should receive ServerMessage::ServerError with bad_request
    let msg = ws1.next().await.expect("next").expect("msg");
    let parsed: ServerMessage = serde_json::from_str(msg.to_text().expect("text")).unwrap();
    match parsed {
        ServerMessage::ServerError { code, message, .. } => {
            assert!(
                code == "bad_request" || message.contains("bad_request"),
                "expected bad_request error message, got: code={code} msg={message}"
            );
        }
        other => panic!("expected ServerError message, got {other:?}"),
    }

    // Player 2 should be completely unaffected and can submit proof successfully
    let proof2 = solve_challenge(&goal2);
    let valid_payload = serde_json::json!({
        "type": "ProofRequest",
        "req_id": "p2_req",
        "code": proof2,
        "intent": "Submit"
    });
    ws2.send(Message::Text(valid_payload.to_string()))
        .await
        .expect("send valid payload");

    let m = ws2.next().await.expect("next").expect("msg");
    let parsed2: ServerMessage = serde_json::from_str(m.to_text().expect("text")).unwrap();
    match parsed2 {
        ServerMessage::RoundEnd { outcome, .. } => {
            assert_eq!(outcome, MatchOutcome::ForfeitWin);
        }
        ServerMessage::Verdict { verdict, .. } => {
            assert_eq!(verdict, VerdictStatus::Accepted);
        }
        other => panic!("expected RoundEnd or Verdict for p2, got {other:?}"),
    }
}

#[tokio::test]
async fn test_malformed_garbage_bytes_rejected() {
    let (ws_url, _config) = spawn_test_server().await;

    let (mut ws, _) = connect_async(&ws_url).await.expect("connect");
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": null
    });
    ws.send(Message::Text(hello.to_string()))
        .await
        .expect("send hello");
    let _ = ws.next().await; // Welcome
    let _ = ws.next().await; // ServerTime

    // Send invalid JSON text frame
    ws.send(Message::Text("this is not json { { {".to_string()))
        .await
        .expect("send garbage text");

    let msg = ws.next().await.expect("next").expect("msg");
    let parsed: ServerMessage = serde_json::from_str(msg.to_text().expect("text")).unwrap();
    match parsed {
        ServerMessage::ServerError { code, message, .. } => {
            assert_eq!(code, "bad_request");
            assert!(message.contains("bad_request"));
        }
        other => panic!("expected ServerError, got {other:?}"),
    }

    // Connect another client to fresh server and send binary frame
    let (ws_url2, _config2) = spawn_test_server().await;
    let (mut ws_bin, _) = connect_async(&ws_url2)
        .await
        .expect("connect binary client");
    let hello_bin = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": null
    });
    ws_bin
        .send(Message::Text(hello_bin.to_string()))
        .await
        .expect("send hello bin");
    let _ = ws_bin.next().await; // Welcome
    let _ = ws_bin.next().await; // ServerTime

    ws_bin
        .send(Message::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF]))
        .await
        .expect("send binary frame");

    let bin_msg = ws_bin.next().await.expect("next").expect("msg");
    let bin_parsed: ServerMessage = serde_json::from_str(bin_msg.to_text().expect("text")).unwrap();
    match bin_parsed {
        ServerMessage::ServerError { code, message, .. } => {
            assert_eq!(code, "bad_request");
            assert!(message.contains("1003") || message.contains("unsupported"));
        }
        other => panic!("expected ServerError for binary frame, got {other:?}"),
    }
}
