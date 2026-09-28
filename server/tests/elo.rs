use futures::{SinkExt, StreamExt};
use proof_battle_server::config::Config;
use proof_battle_server::game::PlayerId;
use proof_battle_server::game::elo::{Elo, Rating, RatingSystem};
use proof_battle_server::game::matchmaker::Matchmaker;
use proof_battle_server::ws::message::ServerMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[test]
fn test_elo_k_factor_transitions() {
    // Provisional player (< 10 games) -> K = 40
    assert_eq!(Elo::k_factor(5), 40.0);

    // Intermediate player (10..=30 games) -> K = 32
    assert_eq!(Elo::k_factor(15), 32.0);
    assert_eq!(Elo::k_factor(30), 32.0);

    // Established player (> 30 games) -> K = 24
    assert_eq!(Elo::k_factor(35), 24.0);
}

#[test]
fn test_elo_win_loss_symmetry_and_math() {
    let elo = Elo::new();

    let mut p1 = Rating {
        elo: 1200,
        games_played: 15, // K = 32
    };
    let mut p2 = Rating {
        elo: 1200,
        games_played: 15, // K = 32
    };

    // Equal ratings: win gives delta = +16, loss = -16
    let (d1, d2) = elo.update(&mut p1, &mut p2, 1.0);
    assert_eq!(d1, 16);
    assert_eq!(d2, -16);
    assert_eq!(p1.elo, 1216);
    assert_eq!(p2.elo, 1184);
    assert_eq!(p1.games_played, 16);
    assert_eq!(p2.games_played, 16);

    // Draw between equal ratings: delta = 0
    let mut p3 = Rating {
        elo: 1500,
        games_played: 15,
    };
    let mut p4 = Rating {
        elo: 1500,
        games_played: 15,
    };
    let (d3, d4) = elo.update(&mut p3, &mut p4, 0.5);
    assert_eq!(d3, 0);
    assert_eq!(d4, 0);
    assert_eq!(p3.elo, 1500);
    assert_eq!(p4.elo, 1500);
}

#[tokio::test]
async fn test_closest_pair_matchmaking_selects_closest_ratings() {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());

    let config = Arc::new(Config::from_map(&env).expect("valid config"));
    let mm = Matchmaker::spawn(config.clone());

    let (tx1, mut rx1) = tokio::sync::mpsc::channel(64);
    let (tx2, mut rx2) = tokio::sync::mpsc::channel(64);
    let (tx3, mut rx3) = tokio::sync::mpsc::channel(64);

    let p1 = PlayerId::new(); // Rating 1200 (default)
    let p2 = PlayerId::new(); // Rating 1210
    let p3 = PlayerId::new(); // Rating 1800

    mm.set_player_rating(
        p1,
        Rating {
            elo: 1200,
            games_played: 5,
        },
    )
    .await;
    mm.set_player_rating(
        p2,
        Rating {
            elo: 1210,
            games_played: 5,
        },
    )
    .await;
    mm.set_player_rating(
        p3,
        Rating {
            elo: 1800,
            games_played: 5,
        },
    )
    .await;

    mm.connect(p1, tx1, Some("p1".to_string())).await;
    mm.connect(p2, tx2, Some("p2".to_string())).await;
    mm.connect(p3, tx3, Some("p3".to_string())).await;

    // Join p1 and p3 first
    mm.queue_join(p1).await;
    mm.queue_join(p3).await;

    // p1 (1200) and p3 (1800) diff is 600, which exceeds initial 100 tolerance -> no match yet
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(mm.get_registry_len().await, 0);

    // Now p2 (1210) joins
    mm.queue_join(p2).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Closest pair: p1 and p2 must be matched (diff = 10 <= 100)
    assert_eq!(
        mm.get_registry_len().await,
        1,
        "p1 and p2 must be paired together"
    );

    // p1 and p2 receive MatchFound
    let mut p1_got_match = false;
    while let Ok(msg) = rx1.try_recv() {
        if matches!(msg, ServerMessage::MatchFound { .. }) {
            p1_got_match = true;
            break;
        }
    }
    let mut p2_got_match = false;
    while let Ok(msg) = rx2.try_recv() {
        if matches!(msg, ServerMessage::MatchFound { .. }) {
            p2_got_match = true;
            break;
        }
    }
    assert!(p1_got_match && p2_got_match);

    // p3 remains in waiting queue
    let mut p3_got_match = false;
    while let Ok(msg) = rx3.try_recv() {
        if matches!(msg, ServerMessage::MatchFound { .. }) {
            p3_got_match = true;
            break;
        }
    }
    assert!(
        !p3_got_match,
        "p3 must not be matched because elo difference is too large"
    );
    assert_eq!(
        mm.get_waiting_len().await,
        1,
        "p3 must still be waiting in queue"
    );
}

#[tokio::test]
async fn test_adr010_early_forfeit_is_unrated() {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());

    let config = Arc::new(Config::from_map(&env).expect("valid config"));
    let mm = Matchmaker::spawn(config.clone());
    let sessions = Arc::new(
        proof_battle_server::game::session::InMemorySessionStore::new(Duration::from_secs(60)),
    );

    let app = proof_battle_server::create_app_with_components(config, mm, sessions);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let ws_url = format!("ws://127.0.0.1:{}/ws", addr.port());

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let (mut ws2, _) = connect_async(&ws_url).await.unwrap();

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

    ws2.send(Message::Text(hello.to_string())).await.unwrap();
    let _ = ws2.next().await; // Welcome
    let _ = ws2.next().await; // ServerTime
    ws2.send(Message::Text(qj.to_string())).await.unwrap();
    let _ = ws2.next().await; // QueueStatus

    // Drain MatchFound & RoundStart
    let _ = ws1.next().await;
    let _ = ws1.next().await;
    let _ = ws2.next().await;
    let _ = ws2.next().await;

    // Resign within first 30 seconds
    let resign = serde_json::json!({ "type": "Resign" });
    ws1.send(Message::Text(resign.to_string())).await.unwrap();

    // Check that RoundEnd has elo_delta == 0 per ADR-010
    let end_msg = ws2.next().await.unwrap().unwrap();
    let parsed: ServerMessage = serde_json::from_str(end_msg.to_text().unwrap()).unwrap();
    match parsed {
        ServerMessage::RoundEnd {
            outcome, elo_delta, ..
        } => {
            assert_eq!(
                outcome,
                proof_battle_server::ws::message::MatchOutcome::ForfeitWin
            );
            assert_eq!(
                elo_delta, 0,
                "ADR-010: early forfeit (<30s) must be unrated (elo_delta = 0)"
            );
        }
        other => panic!("expected RoundEnd, got {other:?}"),
    }
}
