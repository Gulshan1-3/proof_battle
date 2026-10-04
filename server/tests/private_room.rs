use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::create_app_with_matchmaker;
use proof_battle_server::game::matchmaker::{Matchmaker, MatchmakerHandle};
use proof_battle_server::ws::message::{MatchOutcome, ServerMessage};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
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

async fn connect_client(
    ws_url: &str,
    username: Option<&str>,
) -> (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    String,
) {
    let (mut ws, _) = connect_async(ws_url).await.expect("connect");
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": username
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

    (ws, player_id)
}

#[tokio::test]
async fn test_create_private_room_and_join_unrated() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    let (mut ws1, p1_id) = connect_client(&ws_url, Some("alice")).await;
    let (mut ws2, p2_id) = connect_client(&ws_url, Some("bob")).await;

    // Player 1 creates private room with custom duration (120s)
    let create_msg = serde_json::json!({
        "type": "CreatePrivateRoom",
        "category": "logic",
        "difficulty": 2,
        "duration_secs": 120
    });
    ws1.send(Message::Text(create_msg.to_string()))
        .await
        .expect("send create");

    let m_created = ws1.next().await.expect("msg").expect("ok");
    let (room_code, duration_secs) =
        match serde_json::from_str::<ServerMessage>(m_created.to_text().unwrap()).expect("created")
        {
            ServerMessage::PrivateRoomCreated {
                room_code,
                duration_secs,
                ..
            } => (room_code, duration_secs),
            other => panic!("expected PrivateRoomCreated, got {other:?}"),
        };

    assert_eq!(room_code.len(), 6);
    assert_eq!(duration_secs, Some(120));

    let m_waiting = ws1.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_waiting.to_text().unwrap()).expect("waiting") {
        ServerMessage::PrivateRoomWaiting {
            room_code: code, ..
        } => {
            assert_eq!(code, room_code);
        }
        other => panic!("expected PrivateRoomWaiting, got {other:?}"),
    };

    // Player 2 joins using the room code
    let join_msg = serde_json::json!({
        "type": "JoinPrivateRoom",
        "room_code": room_code
    });
    ws2.send(Message::Text(join_msg.to_string()))
        .await
        .expect("send join");

    // Both players should receive MatchFound and RoundStart
    let m1_match = ws1.next().await.expect("msg").expect("ok");
    let m2_match = ws2.next().await.expect("msg").expect("ok");

    let match1: ServerMessage = serde_json::from_str(m1_match.to_text().unwrap()).expect("match1");
    let match2: ServerMessage = serde_json::from_str(m2_match.to_text().unwrap()).expect("match2");

    let r_id1 = match match1 {
        ServerMessage::MatchFound {
            room_id,
            you,
            opponent,
        } => {
            assert_eq!(you.player_id, p1_id);
            assert_eq!(opponent.player_id, p2_id);
            room_id
        }
        other => panic!("expected MatchFound, got {other:?}"),
    };

    let r_id2 = match match2 {
        ServerMessage::MatchFound {
            room_id,
            you,
            opponent,
        } => {
            assert_eq!(you.player_id, p2_id);
            assert_eq!(opponent.player_id, p1_id);
            room_id
        }
        other => panic!("expected MatchFound, got {other:?}"),
    };

    assert_eq!(r_id1, r_id2);

    let m1_start = ws1.next().await.expect("msg").expect("ok");
    let m2_start = ws2.next().await.expect("msg").expect("ok");

    match serde_json::from_str::<ServerMessage>(m1_start.to_text().unwrap()).expect("start1") {
        ServerMessage::RoundStart { problem, .. } => {
            assert_eq!(problem.duration_ms, 120_000);
            assert_eq!(problem.difficulty, 2);
            assert_eq!(problem.category, "logic");
        }
        other => panic!("expected RoundStart, got {other:?}"),
    }

    match serde_json::from_str::<ServerMessage>(m2_start.to_text().unwrap()).expect("start2") {
        ServerMessage::RoundStart { problem, .. } => {
            assert_eq!(problem.duration_ms, 120_000);
        }
        other => panic!("expected RoundStart, got {other:?}"),
    }

    // Player 1 resigns to end match
    let resign_msg = serde_json::json!({ "type": "Resign" });
    ws1.send(Message::Text(resign_msg.to_string()))
        .await
        .expect("send resign");

    let m1_end = ws1.next().await.expect("msg").expect("ok");
    let m2_end = ws2.next().await.expect("msg").expect("ok");

    // Invariant: private match is unrated, elo_delta must be exactly 0
    match serde_json::from_str::<ServerMessage>(m1_end.to_text().unwrap()).expect("end1") {
        ServerMessage::RoundEnd {
            outcome, elo_delta, ..
        } => {
            assert_eq!(outcome, MatchOutcome::ForfeitLoss);
            assert_eq!(elo_delta, 0, "Private match must have elo_delta = 0");
        }
        other => panic!("expected RoundEnd, got {other:?}"),
    }

    match serde_json::from_str::<ServerMessage>(m2_end.to_text().unwrap()).expect("end2") {
        ServerMessage::RoundEnd {
            outcome, elo_delta, ..
        } => {
            assert_eq!(outcome, MatchOutcome::ForfeitWin);
            assert_eq!(elo_delta, 0, "Private match must have elo_delta = 0");
        }
        other => panic!("expected RoundEnd, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cannot_join_own_private_room() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    let (mut ws, _player_id) = connect_client(&ws_url, Some("alice")).await;

    let create_msg = serde_json::json!({
        "type": "CreatePrivateRoom",
        "category": null,
        "difficulty": null,
        "duration_secs": null
    });
    ws.send(Message::Text(create_msg.to_string()))
        .await
        .expect("send create");

    let m_created = ws.next().await.expect("msg").expect("ok");
    let room_code = match serde_json::from_str::<ServerMessage>(m_created.to_text().unwrap())
        .expect("created")
    {
        ServerMessage::PrivateRoomCreated { room_code, .. } => room_code,
        other => panic!("expected PrivateRoomCreated, got {other:?}"),
    };

    let _waiting = ws.next().await.expect("msg").expect("ok");

    // Try joining own room
    let join_msg = serde_json::json!({
        "type": "JoinPrivateRoom",
        "room_code": room_code
    });
    ws.send(Message::Text(join_msg.to_string()))
        .await
        .expect("send join");

    let m_err = ws.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_err.to_text().unwrap()).expect("err") {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "invalid_state");
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
}

#[tokio::test]
async fn test_invalid_room_code_rejected() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    let (mut ws, _player_id) = connect_client(&ws_url, Some("alice")).await;

    let join_msg = serde_json::json!({
        "type": "JoinPrivateRoom",
        "room_code": "NONEXIST"
    });
    ws.send(Message::Text(join_msg.to_string()))
        .await
        .expect("send join");

    let m_err = ws.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_err.to_text().unwrap()).expect("err") {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "room_not_found");
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
}

#[tokio::test]
async fn test_spectator_attachment_and_privacy() {
    let (ws_url, _config, _mm) = spawn_test_server(None).await;

    let (mut ws1, p1_id) = connect_client(&ws_url, Some("p1")).await;
    let (mut ws2, p2_id) = connect_client(&ws_url, Some("p2")).await;
    let (mut ws3, _spec_id) = connect_client(&ws_url, Some("spectator")).await;

    // Start private room
    let create_msg = serde_json::json!({
        "type": "CreatePrivateRoom",
        "category": "logic",
        "difficulty": 1,
        "duration_secs": 300
    });
    ws1.send(Message::Text(create_msg.to_string()))
        .await
        .expect("create");

    let m_created = ws1.next().await.expect("msg").expect("ok");
    let room_code =
        match serde_json::from_str::<ServerMessage>(m_created.to_text().unwrap()).unwrap() {
            ServerMessage::PrivateRoomCreated { room_code, .. } => room_code,
            other => panic!("expected PrivateRoomCreated, got {other:?}"),
        };
    let _waiting = ws1.next().await.expect("msg").expect("ok");

    // Spectator attempts to spectate pending room: rejected with match_not_started
    let spec_msg = serde_json::json!({
        "type": "SpectateRoom",
        "room_code": room_code
    });
    ws3.send(Message::Text(spec_msg.to_string()))
        .await
        .expect("spectate early");

    let m_early = ws3.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_early.to_text().unwrap()).unwrap() {
        ServerMessage::ServerError { code, .. } => {
            assert_eq!(code, "match_not_started");
        }
        other => panic!("expected ServerError match_not_started, got {other:?}"),
    }

    // Player 2 joins
    let join_msg = serde_json::json!({
        "type": "JoinPrivateRoom",
        "room_code": room_code
    });
    ws2.send(Message::Text(join_msg.to_string()))
        .await
        .expect("join");

    let _m1_match = ws1.next().await.expect("msg").expect("ok");
    let _m1_start = ws1.next().await.expect("msg").expect("ok");
    let _m2_match = ws2.next().await.expect("msg").expect("ok");
    let _m2_start = ws2.next().await.expect("msg").expect("ok");

    // Spectator joins active match
    ws3.send(Message::Text(spec_msg.to_string()))
        .await
        .expect("spectate active");

    let m_spec_joined = ws3.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_spec_joined.to_text().unwrap()).unwrap() {
        ServerMessage::SpectatorJoined {
            room_code: code,
            player1,
            player2,
            ..
        } => {
            assert_eq!(code, Some(room_code.clone()));
            assert_eq!(player1.player_id, p1_id);
            assert_eq!(player2.player_id, p2_id);
        }
        other => panic!("expected SpectatorJoined, got {other:?}"),
    }

    // Player 1 submits a proof
    let submit_msg = serde_json::json!({
        "type": "ProofRequest",
        "req_id": "sub_1",
        "code": "  intro n\n  simp\n",
        "intent": "Submit"
    });
    ws1.send(Message::Text(submit_msg.to_string()))
        .await
        .expect("submit");

    // Spectator receives SpectatorUpdate with status Verifying
    let m_spec_update = ws3.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_spec_update.to_text().unwrap()).unwrap() {
        ServerMessage::SpectatorUpdate {
            player_id, status, ..
        } => {
            assert_eq!(player_id, p1_id);
            assert_eq!(
                status,
                proof_battle_server::ws::message::OpponentStatus::Verifying
            );
        }
        other => panic!("expected SpectatorUpdate, got {other:?}"),
    }

    // Player 2 resigns
    let resign_msg = serde_json::json!({ "type": "Resign" });
    ws2.send(Message::Text(resign_msg.to_string()))
        .await
        .expect("resign");

    // Spectator receives RoundEnd without draft leakage
    let m_spec_end = ws3.next().await.expect("msg").expect("ok");
    match serde_json::from_str::<ServerMessage>(m_spec_end.to_text().unwrap()).unwrap() {
        ServerMessage::RoundEnd {
            outcome,
            winner_id,
            elo_delta,
            ..
        } => {
            assert_eq!(outcome, MatchOutcome::ForfeitWin);
            assert_eq!(winner_id, Some(p1_id));
            assert_eq!(elo_delta, 0);
        }
        other => panic!("expected RoundEnd, got {other:?}"),
    }
}
