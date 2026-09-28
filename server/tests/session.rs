use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::game::matchmaker::Matchmaker;
use proof_battle_server::game::session::InMemorySessionStore;
use proof_battle_server::ws::message::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_session_test_server(
    grace_window_ms: u64,
) -> (String, Arc<Config>, Arc<InMemorySessionStore>) {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());
    env.insert("LEAN_POOL_SIZE".to_string(), "8".to_string());
    env.insert(
        "RECONNECT_GRACE_WINDOW_MS".to_string(),
        grace_window_ms.to_string(),
    );

    let config = Arc::new(Config::from_map(&env).expect("valid config"));
    let matchmaker = Matchmaker::spawn(config.clone());
    let sessions = Arc::new(InMemorySessionStore::new(Duration::from_secs(60)));

    let app = proof_battle_server::create_app_with_components(
        config.clone(),
        matchmaker.clone(),
        sessions.clone(),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let ws_url = format!("ws://127.0.0.1:{}/ws", addr.port());

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (ws_url, config, sessions)
}

#[tokio::test]
async fn test_session_handshake_and_invalid_state_transitions() {
    let (ws_url, _config, _sessions) = spawn_session_test_server(10_000).await;
    let (mut ws, _) = connect_async(&ws_url).await.expect("connect");

    // Attempting to send QueueJoin before Hello must return invalid_state
    let bad_qj = serde_json::json!({ "type": "QueueJoin" });
    ws.send(Message::Text(bad_qj.to_string()))
        .await
        .expect("send bad qj");

    let err_msg = ws.next().await.expect("msg").expect("ok");
    let err_parsed: ServerMessage =
        serde_json::from_str(err_msg.to_text().unwrap()).expect("parse server error");
    match err_parsed {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "invalid_state");
        }
        other => panic!("expected ServerError invalid_state, got {other:?}"),
    }

    // Attempting to send ProofRequest before Hello must also return invalid_state
    let bad_proof = serde_json::json!({
        "type": "ProofRequest",
        "req_id": "test_req",
        "code": "simp",
        "intent": "Submit"
    });
    ws.send(Message::Text(bad_proof.to_string()))
        .await
        .expect("send bad proof");

    let err_msg2 = ws.next().await.expect("msg").expect("ok");
    let err_parsed2: ServerMessage =
        serde_json::from_str(err_msg2.to_text().unwrap()).expect("parse server error");
    match err_parsed2 {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "invalid_state");
        }
        other => panic!("expected ServerError invalid_state, got {other:?}"),
    }

    // Now send valid Hello
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": "alice"
    });
    ws.send(Message::Text(hello.to_string()))
        .await
        .expect("send hello");

    let welcome_raw = ws.next().await.expect("welcome").expect("ok");
    let welcome: ServerMessage = serde_json::from_str(welcome_raw.to_text().unwrap()).unwrap();
    let (p_id, s_token) = match welcome {
        ServerMessage::Welcome {
            player_id,
            session_token,
            protocol_version,
            ..
        } => {
            assert_eq!(protocol_version, 1);
            assert!(!player_id.is_empty());
            assert!(!session_token.is_empty());
            (player_id, session_token)
        }
        other => panic!("expected Welcome, got {other:?}"),
    };

    let server_time_raw = ws.next().await.expect("server time").expect("ok");
    let server_time: ServerMessage =
        serde_json::from_str(server_time_raw.to_text().unwrap()).unwrap();
    assert!(matches!(server_time, ServerMessage::ServerTime { .. }));

    // Now in Connected state: cannot send ProofRequest
    ws.send(Message::Text(bad_proof.to_string()))
        .await
        .expect("send proof in lobby");
    let err_msg3 = ws.next().await.expect("msg").expect("ok");
    let err_parsed3: ServerMessage = serde_json::from_str(err_msg3.to_text().unwrap()).unwrap();
    match err_parsed3 {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "invalid_state");
        }
        other => panic!("expected invalid_state in lobby, got {other:?}"),
    }

    // Send QueueJoin -> state becomes InQueue
    let qj = serde_json::json!({ "type": "QueueJoin" });
    ws.send(Message::Text(qj.to_string()))
        .await
        .expect("send qj");
    let qs_raw = ws.next().await.expect("queue status").expect("ok");
    let qs: ServerMessage = serde_json::from_str(qs_raw.to_text().unwrap()).unwrap();
    assert!(matches!(qs, ServerMessage::QueueStatus { .. }));

    // InQueue: cannot send QueueJoin again
    ws.send(Message::Text(qj.to_string()))
        .await
        .expect("send double qj");
    let err_msg4 = ws.next().await.expect("msg").expect("ok");
    let err_parsed4: ServerMessage = serde_json::from_str(err_msg4.to_text().unwrap()).unwrap();
    match err_parsed4 {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "invalid_state");
        }
        other => panic!("expected invalid_state for double QueueJoin, got {other:?}"),
    }

    assert!(!p_id.is_empty() && !s_token.is_empty());
}

#[tokio::test]
async fn test_reconnection_within_grace_window_reattaches_with_monotonic_seq() {
    let (ws_url, _config, _sessions) = spawn_session_test_server(5000).await;

    // Connect Player 1
    let (mut ws1, _) = connect_async(&ws_url).await.expect("connect p1");
    let hello1 = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": "p1"
    });
    ws1.send(Message::Text(hello1.to_string())).await.unwrap();
    let m1 = ws1.next().await.unwrap().unwrap();
    let (p1_id, p1_token) =
        match serde_json::from_str::<ServerMessage>(m1.to_text().unwrap()).unwrap() {
            ServerMessage::Welcome {
                player_id,
                session_token,
                ..
            } => (player_id, session_token),
            other => panic!("expected welcome, got {other:?}"),
        };
    let _ = ws1.next().await; // ServerTime
    let qj = serde_json::json!({ "type": "QueueJoin" });
    ws1.send(Message::Text(qj.to_string())).await.unwrap();
    let _ = ws1.next().await; // QueueStatus

    // Connect Player 2
    let (mut ws2, _) = connect_async(&ws_url).await.expect("connect p2");
    let hello2 = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": "p2"
    });
    ws2.send(Message::Text(hello2.to_string())).await.unwrap();
    let m2 = ws2.next().await.unwrap().unwrap();
    let (p2_id, _p2_token) =
        match serde_json::from_str::<ServerMessage>(m2.to_text().unwrap()).unwrap() {
            ServerMessage::Welcome {
                player_id,
                session_token,
                ..
            } => (player_id, session_token),
            other => panic!("expected welcome, got {other:?}"),
        };
    let _ = ws2.next().await; // ServerTime
    ws2.send(Message::Text(qj.to_string())).await.unwrap();
    let _ = ws2.next().await; // QueueStatus

    // Both get MatchFound and RoundStart (seq: 1)
    let _ = ws1.next().await; // MatchFound
    let rs1_raw = ws1.next().await.unwrap().unwrap();
    let rs1: ServerMessage = serde_json::from_str(rs1_raw.to_text().unwrap()).unwrap();
    match rs1 {
        ServerMessage::RoundStart { seq, .. } => {
            assert_eq!(seq, 1, "Initial round start must have seq 1");
        }
        other => panic!("expected RoundStart seq 1, got {other:?}"),
    }

    let _ = ws2.next().await; // MatchFound
    let _ = ws2.next().await; // RoundStart

    // Player 1 drops connection (tab refresh simulation)
    drop(ws1);

    // Player 1 reconnects within grace window (e.g. after 200ms) with Hello { token }
    tokio::time::sleep(Duration::from_millis(200)).await;

    let (mut ws1_new, _) = connect_async(&ws_url).await.expect("reconnect p1");
    let hello_reconnect = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": p1_token,
        "username": "p1"
    });
    ws1_new
        .send(Message::Text(hello_reconnect.to_string()))
        .await
        .unwrap();

    let rec_welcome_raw = ws1_new.next().await.unwrap().unwrap();
    let rec_welcome: ServerMessage =
        serde_json::from_str(rec_welcome_raw.to_text().unwrap()).unwrap();
    match rec_welcome {
        ServerMessage::Welcome { player_id, .. } => {
            assert_eq!(
                player_id, p1_id,
                "Reconnected player must keep same player_id"
            );
        }
        other => panic!("expected Welcome on reconnect, got {other:?}"),
    }

    let _ = ws1_new.next().await; // ServerTime

    // Player 1 must receive resynchronization RoundStart with monotonic seq: 2
    let resync_raw = ws1_new.next().await.unwrap().unwrap();
    let resync: ServerMessage = serde_json::from_str(resync_raw.to_text().unwrap()).unwrap();
    match resync {
        ServerMessage::RoundStart { seq, .. } => {
            assert_eq!(
                seq, 2,
                "Reattachment resync RoundStart must have monotonic seq 2"
            );
        }
        other => panic!("expected RoundStart resync with seq 2, got {other:?}"),
    }

    assert_ne!(p1_id, p2_id);
}

#[tokio::test]
async fn test_grace_window_expiry_triggers_forfeit() {
    // 300ms grace window for fast test execution
    let (ws_url, _config, _sessions) = spawn_session_test_server(300).await;

    // Connect Player 1 & 2
    let (mut ws1, _) = connect_async(&ws_url).await.expect("connect p1");
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": null
    });
    ws1.send(Message::Text(hello.to_string())).await.unwrap();
    let _ = ws1.next().await; // Welcome
    let _ = ws1.next().await; // ServerTime
    let qj = serde_json::json!({ "type": "QueueJoin" });
    ws1.send(Message::Text(qj.to_string())).await.unwrap();
    let _ = ws1.next().await; // QueueStatus

    let (mut ws2, _) = connect_async(&ws_url).await.expect("connect p2");
    ws2.send(Message::Text(hello.to_string())).await.unwrap();
    let _ = ws2.next().await; // Welcome
    let _ = ws2.next().await; // ServerTime
    ws2.send(Message::Text(qj.to_string())).await.unwrap();
    let _ = ws2.next().await; // QueueStatus

    // Both get MatchFound and RoundStart
    let _ = ws1.next().await; // MatchFound
    let _ = ws1.next().await; // RoundStart
    let _ = ws2.next().await; // MatchFound
    let _ = ws2.next().await; // RoundStart

    // Player 1 drops connection
    drop(ws1);

    // Player 2 waits for RoundEnd after 300ms grace window expires
    let mut got_forfeit = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let res = tokio::time::timeout(Duration::from_millis(100), ws2.next()).await;
        if let Ok(Some(Ok(msg))) = res
            && let Ok(ServerMessage::RoundEnd { outcome, .. }) =
                serde_json::from_str(msg.to_text().unwrap())
        {
            assert_eq!(outcome, MatchOutcome::ForfeitWin);
            got_forfeit = true;
            break;
        }
    }

    assert!(
        got_forfeit,
        "Opponent must receive ForfeitWin when grace window expires"
    );
}
