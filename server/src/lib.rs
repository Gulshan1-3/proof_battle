pub mod challenges;
pub mod config;
pub mod db;
pub mod error;
pub mod game;
pub mod lean;
pub mod message;
pub mod metrics;
pub mod utils;
pub mod ws;

use axum::{
    Json, Router,
    extract::{
        Path, Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    routing::get,
};
use config::Config;
use futures::{SinkExt, StreamExt};
use game::PlayerId;
use game::matchmaker::Matchmaker;
use game::matchmaker::MatchmakerHandle;
use game::queue::run_check_only;
use game::session::{ConnState, InMemorySessionStore, SessionStore};
use message::*;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub player_id: uuid::Uuid,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct StatsQuery {
    pub player_id: uuid::Uuid,
}

pub fn create_app(config: Arc<Config>) -> Router {
    let history = Arc::new(game::history::HistoryStore::new(None));
    let matchmaker = Matchmaker::spawn_with_history(config.clone(), history.clone());
    let sessions = Arc::new(InMemorySessionStore::new(config.session_ttl));
    create_app_with_components_and_history(config, matchmaker, sessions, history)
}

pub fn create_app_with_matchmaker(config: Arc<Config>, matchmaker: MatchmakerHandle) -> Router {
    let history = Arc::new(game::history::HistoryStore::new(None));
    let sessions = Arc::new(InMemorySessionStore::new(config.session_ttl));
    create_app_with_components_and_history(config, matchmaker, sessions, history)
}

pub fn create_app_with_components(
    config: Arc<Config>,
    matchmaker: MatchmakerHandle,
    sessions: Arc<InMemorySessionStore>,
) -> Router {
    let history = Arc::new(game::history::HistoryStore::new(None));
    create_app_with_components_and_history(config, matchmaker, sessions, history)
}

pub fn create_app_with_components_and_history(
    config: Arc<Config>,
    matchmaker: MatchmakerHandle,
    sessions: Arc<InMemorySessionStore>,
    history: Arc<game::history::HistoryStore>,
) -> Router {
    let ws_matchmaker = matchmaker.clone();
    let ws_config = config.clone();
    let ws_sessions = sessions.clone();
    let ready_config = config.clone();
    let api_history = history.clone();
    let api_matches = history.clone();
    let api_stats = history;

    Router::new()
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| {
                let matchmaker = ws_matchmaker.clone();
                let config = ws_config.clone();
                let sessions = ws_sessions.clone();
                async move {
                    ws.max_message_size(64 * 1024).on_upgrade(move |socket| {
                        handle_socket(socket, matchmaker, sessions, config)
                    })
                }
            }),
        )
        .route("/healthz", get(|| async { (StatusCode::OK, "OK") }))
        .route(
            "/readyz",
            get(move || {
                let config = ready_config.clone();
                async move { handle_readyz(config).await }
            }),
        )
        .route(
            "/metrics",
            get(|| async {
                (
                    StatusCode::OK,
                    [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                    metrics::get_metrics().render(),
                )
            }),
        )
        .route(
            "/api/history",
            get(move |Query(q): Query<HistoryQuery>| {
                let history = api_history.clone();
                async move {
                    let limit = q.limit.unwrap_or(20);
                    let items = history.get_player_history(q.player_id, limit).await;
                    (StatusCode::OK, Json(items))
                }
            }),
        )
        .route(
            "/api/matches/:id",
            get(move |Path(match_id): Path<uuid::Uuid>| {
                let history = api_matches.clone();
                async move {
                    match history.get_match_detail(match_id).await {
                        Some(record) => (StatusCode::OK, Json(Some(record))),
                        None => (
                            StatusCode::NOT_FOUND,
                            Json(None::<game::history::MatchRecord>),
                        ),
                    }
                }
            }),
        )
        .route(
            "/api/stats",
            get(move |Query(q): Query<StatsQuery>| {
                let history = api_stats.clone();
                async move {
                    let stats = history.get_player_stats(q.player_id).await;
                    (StatusCode::OK, Json(stats))
                }
            }),
        )
}

async fn handle_readyz(config: Arc<Config>) -> (StatusCode, &'static str) {
    if config.sandbox_enabled {
        // If sandbox execution is enabled, verify the container image is available.
        // Returning 503 prevents the service from silently running unsandboxed.
        let image = &config.sandbox_image;
        let check = tokio::process::Command::new("docker")
            .args(["image", "inspect", image])
            .output()
            .await;

        match check {
            Ok(output) if output.status.success() => (StatusCode::OK, "READY"),
            _ => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service Unavailable: sandbox image missing",
            ),
        }
    } else {
        (StatusCode::OK, "READY")
    }
}

struct WsConnectionGuard;

impl WsConnectionGuard {
    fn new() -> Self {
        metrics::get_metrics().inc_ws_connections();
        Self
    }
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        metrics::get_metrics().dec_ws_connections();
    }
}

pub async fn handle_socket(
    stream: WebSocket,
    matchmaker: MatchmakerHandle,
    sessions: Arc<InMemorySessionStore>,
    config: Arc<Config>,
) {
    let _ws_guard = WsConnectionGuard::new();
    let (mut sender, mut receiver) = stream.split();

    // Outbound channel capacity 64 to prevent unbounded memory growth under slow consumers
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(64);

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to serialize server message");
                    break;
                }
            }
        }
    });

    let mut state = ConnState::AwaitingHello;
    let mut player_id = PlayerId::new();
    let mut missed_pongs = 0u32;
    let handshake_deadline = tokio::time::Instant::now() + config.handshake_timeout;

    let heartbeat_interval_ms = 15000u64;
    let mut ping_interval = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_millis(heartbeat_interval_ms),
        Duration::from_millis(heartbeat_interval_ms),
    );
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(handshake_deadline), if state == ConnState::AwaitingHello => {
                tracing::warn!(player_id = %player_id, "handshake timeout: Hello message not received in 10s");
                let _ = tx.send(ServerMessage::ServerError {
                    code: "handshake_timeout".to_string(),
                    message: "connection closed: hello handshake timeout (10s)".to_string(),
                    retry_after_ms: None,
                    retryable: true,
                }).await;
                break;
            }
            _ = ping_interval.tick() => {
                if state != ConnState::AwaitingHello && state != ConnState::Closed {
                    if missed_pongs >= 2 {
                        tracing::warn!(player_id = %player_id, "heartbeat timeout: 2 missed pongs, closing connection");
                        break;
                    }
                    missed_pongs += 1;
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    let _ = tx.send(ServerMessage::ServerTime { server_time_ms: now_ms }).await;
                }
            }
            frame = receiver.next() => {
                let msg = match frame {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::warn!(player_id = %player_id, error = %e, "WebSocket frame receive error");
                        let _ = tx.send(ServerMessage::ServerError {
                            code: "bad_request".to_string(),
                            message: "invalid frame payload data".to_string(),
                            retry_after_ms: None,
                            retryable: false,
                        }).await;
                        break;
                    }
                    None => break,
                };

                match msg {
                    Message::Text(text) => {
                        let client_msg = match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(m) => m,
                            Err(err) => {
                                tracing::warn!(player_id = %player_id, error = %err, "malformed JSON in client message");
                                let _ = tx.send(ServerMessage::ServerError {
                                    code: "bad_request".to_string(),
                                    message: format!("bad_request: {err}"),
                                    retry_after_ms: None,
                                    retryable: false,
                                }).await;
                                break;
                            }
                        };

                        // Check with current room status to keep ConnState up-to-date
                        if let Some(r_id) = matchmaker.get_player_room(player_id).await {
                            state = ConnState::InGame { room_id: r_id };
                        } else if let Some(r_id) = matchmaker.get_spectator_room(player_id).await {
                            state = ConnState::Spectating { room_id: r_id };
                        } else if state != ConnState::AwaitingHello
                            && state != ConnState::InQueue
                            && state != ConnState::AwaitingPrivateOpponent
                        {
                            state = ConnState::Connected;
                        }

                        // Validate state transition
                        if let Err(code) = state.validate_transition(&client_msg) {
                            let _ = tx.send(ServerMessage::ServerError {
                                code: "invalid_state".to_string(),
                                message: code.to_string(),
                                retry_after_ms: None,
                                retryable: false,
                            }).await;
                            continue;
                        }

                        match client_msg {
                            ClientMessage::Hello { version, token, username } => {
                                if version > config.protocol_version {
                                    let _ = tx.send(ServerMessage::ServerError {
                                        code: "unsupported_version".to_string(),
                                        message: format!(
                                            "protocol version {} is unsupported (max {})",
                                            version, config.protocol_version
                                        ),
                                        retry_after_ms: None,
                                        retryable: false,
                                    }).await;
                                    break;
                                }

                                // Handle authentication / session resumption
                                let session = match token {
                                    Some(ref tok) => match sessions.get(tok).await {
                                        Ok(Some(existing)) => {
                                            let _ = sessions.touch(tok).await;
                                            existing
                                        }
                                        _ => match sessions.create(player_id, username.clone()).await {
                                            Ok(s) => s,
                                            Err(_) => break,
                                        },
                                    },
                                    None => match sessions.create(player_id, username.clone()).await {
                                        Ok(s) => s,
                                        Err(_) => break,
                                    },
                                };

                                player_id = session.player_id;
                                let session_token = session.token.clone();

                                let now_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_millis() as i64)
                                    .unwrap_or(0);

                                let _ = tx.send(ServerMessage::Welcome {
                                    player_id: player_id.to_string(),
                                    session_token: session_token.clone(),
                                    server_time_ms: now_ms,
                                    heartbeat_interval_ms: heartbeat_interval_ms as i64,
                                    protocol_version: config.protocol_version,
                                }).await;

                                let _ = tx.send(ServerMessage::ServerTime { server_time_ms: now_ms }).await;

                                // Connect to matchmaker
                                matchmaker.connect(player_id, tx.clone(), session.username.clone()).await;

                                // Check if player was in an active room to reattach
                                if let Some(room_id) = matchmaker.get_player_room(player_id).await {
                                    let reattached = matchmaker.reattach(player_id, tx.clone()).await;
                                    if reattached {
                                        state = ConnState::InGame { room_id };
                                        continue;
                                    }
                                }

                                state = ConnState::Connected;
                            }
                            ClientMessage::QueueJoin {} => {
                                state = ConnState::InQueue;
                                matchmaker.queue_join(player_id).await;
                            }
                            ClientMessage::PracticeJoin {} => {
                                state = ConnState::InQueue;
                                matchmaker.practice_join(player_id).await;
                            }
                            ClientMessage::QueueLeave {} => {
                                state = ConnState::Connected;
                                matchmaker.queue_leave(player_id).await;
                            }
                            ClientMessage::ProofRequest { req_id, code, intent } => {
                                match intent {
                                    ProofIntent::Submit => {
                                        // Submit lane rate limit: 5 per 60s
                                        if let Err(retry_after) = matchmaker.rate_limiter.check_submit(player_id).await {
                                            let _ = tx.send(ServerMessage::ServerError {
                                                code: "rate_limited".to_string(),
                                                message: "rate limit exceeded: max 5 submissions per 60s".to_string(),
                                                retry_after_ms: Some(retry_after.as_millis() as u64),
                                                retryable: true,
                                            }).await;
                                            continue;
                                        }
                                        matchmaker.submit_proof(player_id, code, Some(req_id)).await;
                                    }
                                    ProofIntent::Check => {
                                        // Check lane rate limit: 1 per 2s
                                        if let Err(retry_after) = matchmaker.rate_limiter.check_diagnostic(player_id).await {
                                            let _ = tx.send(ServerMessage::ServerError {
                                                code: "rate_limited".to_string(),
                                                message: "diagnostic check throttled: max 1 per 2s".to_string(),
                                                retry_after_ms: Some(retry_after.as_millis() as u64),
                                                retryable: true,
                                            }).await;
                                            continue;
                                        }

                                        // Diagnostics check structurally isolated from Room actor
                                        let maybe_room = matchmaker.get_player_room(player_id).await;
                                        if let Some(room_id) = maybe_room {
                                            let maybe_challenge = matchmaker.get_room_challenge(room_id).await;
                                            if let Some(challenge) = maybe_challenge {
                                                let pool = matchmaker.pool.clone();
                                                let conf = config.clone();
                                                let tx_clone = tx.clone();
                                                tokio::spawn(async move {
                                                    let (diagnostics, _) = run_check_only(&code, &challenge, &conf, &pool).await;
                                                    let _ = tx_clone.send(ServerMessage::Verdict {
                                                        req_id,
                                                        verdict: if diagnostics.is_empty() { VerdictStatus::Accepted } else { VerdictStatus::Rejected },
                                                        reason: None,
                                                        message: "Diagnostic check complete".to_string(),
                                                        diagnostics,
                                                        elapsed_ms: 0,
                                                        check_skipped: false,
                                                    }).await;
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            ClientMessage::Resign {} => {
                                matchmaker.resign(player_id).await;
                            }
                            ClientMessage::Pong { ts } => {
                                missed_pongs = 0;
                                let _ = tx.send(ServerMessage::Pong { ts }).await;
                            }
                            ClientMessage::CreatePrivateRoom { category, difficulty, duration_secs } => {
                                state = ConnState::AwaitingPrivateOpponent;
                                matchmaker.create_private_room(player_id, category, difficulty, duration_secs).await;
                            }
                            ClientMessage::JoinPrivateRoom { room_code } => {
                                matchmaker.join_private_room(player_id, room_code).await;
                            }
                            ClientMessage::SpectateRoom { room_code } => {
                                matchmaker.spectate_room(player_id, room_code).await;
                            }
                        }
                    }
                    Message::Binary(_) => {
                        tracing::warn!(player_id = %player_id, "unsupported binary WebSocket frame");
                        let _ = tx.send(ServerMessage::ServerError {
                            code: "bad_request".to_string(),
                            message: "unsupported data format (1003)".to_string(),
                            retry_after_ms: None,
                            retryable: false,
                        }).await;
                        break;
                    }
                    Message::Ping(_) | Message::Pong(_) => {
                        missed_pongs = 0;
                    }
                    Message::Close(frame) => {
                        tracing::debug!(player_id = %player_id, ?frame, "client sent close frame");
                        break;
                    }
                }
            }
        }
    }

    let _ = state;
    matchmaker.disconnect(player_id).await;
}
