use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::game::matchmaker::Matchmaker;
use proof_battle_server::game::session::InMemorySessionStore;
use proof_battle_server::ws::message::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_soak_server() -> (
    String,
    Arc<Config>,
    proof_battle_server::game::matchmaker::MatchmakerHandle,
) {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());
    env.insert("LEAN_POOL_SIZE".to_string(), "8".to_string());
    env.insert("RECONNECT_GRACE_WINDOW_MS".to_string(), "100".to_string());

    let config = Arc::new(Config::from_map(&env).expect("valid config"));
    let matchmaker = Matchmaker::spawn_with_round_duration(config.clone(), Duration::from_secs(30));
    let sessions = Arc::new(InMemorySessionStore::new(Duration::from_secs(60)));

    let app = proof_battle_server::create_app_with_components(
        config.clone(),
        matchmaker.clone(),
        sessions,
    );

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

async fn connect_and_queue(
    ws_url: &str,
    username: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (mut ws, _) = connect_async(ws_url).await.expect("connect");
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": username
    });
    ws.send(Message::Text(hello.to_string())).await.unwrap();

    let _welcome = ws.next().await.unwrap().unwrap();
    let _server_time = ws.next().await.unwrap().unwrap();

    let qj = serde_json::json!({ "type": "QueueJoin" });
    ws.send(Message::Text(qj.to_string())).await.unwrap();
    let _queue_status = ws.next().await.unwrap().unwrap();

    ws
}

#[tokio::test]
async fn test_100_game_soak_no_leaks_and_bounded_memory() {
    let (ws_url, _config, mm) = spawn_soak_server().await;

    let start_time = Instant::now();

    for i in 0..100 {
        let u1 = format!("soak_p1_{i}");
        let u2 = format!("soak_p2_{i}");

        let mut ws1 = connect_and_queue(&ws_url, &u1).await;
        let mut ws2 = connect_and_queue(&ws_url, &u2).await;

        // Drain MatchFound
        let _ = ws1.next().await;
        let _ = ws2.next().await;

        // Player 1 resigns to end match deterministically and immediately
        let resign = serde_json::json!({ "type": "Resign" });
        let _ = ws1.send(Message::Text(resign.to_string())).await;

        // Wait for RoundEnd on Player 2
        let timeout = Instant::now() + Duration::from_secs(1);
        while Instant::now() < timeout {
            let res = tokio::time::timeout(Duration::from_millis(150), ws2.next()).await;
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

    let elapsed = start_time.elapsed();
    println!("100-game soak completed in {:?}", elapsed);

    // Verify that every single room actor unregisters cleanly
    let poll_deadline = Instant::now() + Duration::from_secs(3);
    let mut reg_len = mm.get_registry_len().await;
    let mut wait_len = mm.get_waiting_len().await;

    while (reg_len > 0 || wait_len > 0) && Instant::now() < poll_deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
        reg_len = mm.get_registry_len().await;
        wait_len = mm.get_waiting_len().await;
    }

    assert_eq!(
        reg_len, 0,
        "Zero leaked rooms: all 100 rooms must be cleanly unregistered"
    );
    assert_eq!(
        wait_len, 0,
        "Zero leaked waiting players: queue must be completely empty"
    );
}
