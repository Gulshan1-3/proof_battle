mod config;
mod room;
mod message;
mod lean_runner;
mod utils;
mod challenges;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use challenges::ProofChallenge;
use config::Config;
use futures::{SinkExt, StreamExt};
use message::*;
use rand::prelude::IndexedRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use room::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let matcher = Arc::new(Mutex::new(Matchmaker::new()));
    let challenge_map: Arc<Mutex<HashMap<String, ProofChallenge>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let app_config = config.clone();
    let app_matcher = matcher.clone();
    let app_challenge_map = challenge_map.clone();

    let app = Router::new().route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let matcher = app_matcher.clone();
            let challenge_map = app_challenge_map.clone();
            let config = app_config.clone();
            async move {
                ws.on_upgrade(move |socket| handle_socket(socket, matcher, challenge_map, config))
            }
        }),
    );

    println!("🚀 Running server on ws://{}/ws", config.bind_addr);
    let listener = TcpListener::bind(&config.bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", config.bind_addr, e));

    axum::serve(listener, app).await.unwrap();
}

async fn handle_socket(
    stream: WebSocket,
    matcher: Arc<Mutex<Matchmaker>>,
    challenge_map: Arc<Mutex<HashMap<String, ProofChallenge>>>,
    config: Arc<Config>,
) {
    let (mut sender, mut receiver) = stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let player_id = Uuid::new_v4().to_string();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = sender.send(Message::Text(msg)).await;
        }
    });

    tx.send(
        serde_json::to_string(&ServerMessage::Joined {
            player_id: player_id.clone(),
        })
        .unwrap(),
    )
    .unwrap();

    let match_result = {
        let mut mm = matcher.lock().await;
        mm.add_player(player_id.clone(), tx.clone())
    };

    if let Some((room_id, opponent_id, opponent_tx)) = match_result {
        tx.send(
            serde_json::to_string(&ServerMessage::MatchFound {
                opponent: opponent_id.clone(),
            })
            .unwrap(),
        )
        .unwrap();

        opponent_tx
            .send(
                serde_json::to_string(&ServerMessage::MatchFound {
                    opponent: player_id.clone(),
                })
                .unwrap(),
            )
            .unwrap();

        let challenges = challenges::get_problems();
        let mut rng = seeded_rng();
        let challenge = challenges.choose(&mut rng).unwrap().clone();

        let challenge_msg = ServerMessage::Challenge {
            goal: challenge.goal.clone(),
            imports: challenge.imports.clone(),
        };

        let msg_json = serde_json::to_string(&challenge_msg).unwrap();
        tx.send(msg_json.clone()).unwrap();
        opponent_tx.send(msg_json).unwrap();

        {
            let mut map = challenge_map.lock().await;
            map.insert(player_id.clone(), challenge.clone());
            map.insert(opponent_id.clone(), challenge.clone());
        }

        {
            let mut mm = matcher.lock().await;
            mm.set_challenge(&room_id, challenge.goal.clone(), challenge.imports.clone());
        }
    }

    while let Some(Ok(Message::Text(text))) = receiver.next().await {
        if let Ok(ClientMessage::SubmitProof { code, player_id }) =
            serde_json::from_str::<ClientMessage>(&text)
        {
            let map = challenge_map.lock().await;
            if let Some(challenge) = map.get(&player_id).cloned() {
                let clean_code = code
                    .lines()
                    .map(str::trim)
                    .collect::<Vec<_>>()
                    .join("\n");

                let wrapped = format!(
                    "{}\n\ntheorem goal : {} := by\n{}",
                    challenge.imports.join("\n"),
                    challenge.goal,
                    clean_code
                        .lines()
                        .map(|line| line.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );

                let processed = utils::preprocess(&wrapped);

                match lean_runner::run_proof(
                    &processed,
                    &format!("{}_proof.lean", player_id),
                    &config,
                ) {
                    Ok(output) => {
                        tx.send(
                            serde_json::to_string(&ServerMessage::ProofResult {
                                success: true,
                                output,
                            })
                            .unwrap(),
                        )
                        .unwrap();
                        let mut mm = matcher.lock().await;
                        let found_room_id = mm.rooms.iter().find_map(|(id, room)| {
                            if room.player1 == player_id
                                || room
                                    .player2
                                    .as_ref()
                                    .map(|(p, _)| p == &player_id)
                                    .unwrap_or(false)
                            {
                                Some(id.clone())
                            } else {
                                None
                            }
                        });

                        #[allow(clippy::collapsible_if)]
                        if let Some(room_id) = found_room_id {
                            if mm.declare_winner(&room_id, &player_id).is_some() {
                                if let Some(room) = mm.rooms.get(&room_id) {
                                    let _ = room.tx1.send(
                                        serde_json::to_string(&ServerMessage::GameEnded {
                                            winner: player_id.clone(),
                                        })
                                        .unwrap(),
                                    );

                                    if let Some((_, tx2)) = &room.player2 {
                                        let _ = tx2.send(
                                            serde_json::to_string(&ServerMessage::GameEnded {
                                                winner: player_id.clone(),
                                            })
                                            .unwrap(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        tx.send(
                            serde_json::to_string(&ServerMessage::ProofResult {
                                success: false,
                                output: err,
                            })
                            .unwrap(),
                        )
                        .unwrap();
                    }
                }
            } else {
                tx.send(
                    serde_json::to_string(&ServerMessage::ProofResult {
                        success: false,
                        output: "Challenge not found for this player".to_string(),
                    })
                    .unwrap(),
                )
                .unwrap();
            }
        }
    }
}

fn seeded_rng() -> ChaCha20Rng {
    let seed = rand::random::<[u8; 32]>();
    ChaCha20Rng::from_seed(seed)
}
