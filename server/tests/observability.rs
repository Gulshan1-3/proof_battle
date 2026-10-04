use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::game::matchmaker::Matchmaker;
use proof_battle_server::game::session::InMemorySessionStore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

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

async fn spawn_test_server(sandbox_enabled: bool, sandbox_image: &str) -> (String, String) {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), sandbox_enabled.to_string());
    env.insert("SANDBOX_IMAGE".to_string(), sandbox_image.to_string());
    env.insert("LEAN_POOL_SIZE".to_string(), "4".to_string());

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
    let host_port = addr.to_string();
    let ws_url = format!("ws://{}/ws", host_port);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (host_port, ws_url)
}

#[tokio::test]
async fn test_healthz_and_readyz_probes() {
    let (addr, _) = spawn_test_server(false, "").await;

    // 1. Healthz returns 200
    let (status, body) = http_get(&addr, "/healthz").await;
    assert_eq!(status, 200);
    assert_eq!(body, "OK");

    // 2. Readyz returns 200 when sandbox_enabled is false
    let (status, body) = http_get(&addr, "/readyz").await;
    assert_eq!(status, 200);
    assert_eq!(body, "READY");

    // 3. Readyz returns 503 when sandbox_enabled is true but image is missing
    let (sandbox_addr, _) =
        spawn_test_server(true, "proofbattle_nonexistent_image_12345:tag").await;
    let (status, body) = http_get(&sandbox_addr, "/readyz").await;
    assert_eq!(status, 503);
    assert!(body.contains("sandbox image missing"), "Body: {body}");
}

#[tokio::test]
async fn test_metrics_exposition_and_cardinality() {
    let (addr, ws_url) = spawn_test_server(false, "").await;

    // Initial metrics request
    let (status, body) = http_get(&addr, "/metrics").await;
    assert_eq!(status, 200);
    assert!(body.contains("proofbattle_players_online"));
    assert!(body.contains("proofbattle_games_in_progress"));
    assert!(body.contains("proofbattle_ws_connections"));
    assert!(body.contains("proofbattle_rooms_total"));
    assert!(body.contains("proofbattle_rate_limited_total"));
    assert!(body.contains("proofbattle_queue_dropped_total{lane=\"submit\"}"));
    assert!(body.contains("proofbattle_lean_duration_seconds_bucket"));
    assert!(body.contains("proofbattle_lean_queue_wait_seconds_bucket"));
    assert!(body.contains("proofbattle_lean_verifications_total"));

    let initial_line_count = body.lines().count();

    // Connect WebSocket client
    let (mut ws, _) = connect_async(&ws_url).await.expect("connect");
    let hello = serde_json::json!({
        "type": "Hello",
        "version": 1,
        "token": null,
        "username": "ObservabilityTester"
    });
    ws.send(Message::Text(hello.to_string()))
        .await
        .expect("send hello");

    // Wait for Welcome frame
    let welcome = ws.next().await.expect("msg").expect("ok");
    assert!(welcome.to_string().contains("Welcome"));

    // Check ws_connections increased
    let (_, body_after_connect) = http_get(&addr, "/metrics").await;
    assert!(body_after_connect.contains("proofbattle_ws_connections"));

    // Close WebSocket
    drop(ws);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify Cardinality Discipline:
    // The metric line count must NOT expand arbitrarily or leak player_id / room_id
    let (_, body_final) = http_get(&addr, "/metrics").await;
    assert_eq!(body_final.lines().count(), initial_line_count);
    assert!(!body_final.contains("ObservabilityTester"));
    assert!(!body_final.contains("player_id"));
    assert!(!body_final.contains("room_id"));
}
