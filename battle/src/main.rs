mod room;
use tokio::net::TcpListener;
 
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    
    routing::get,
    Router,
};


use rand_chacha::ChaCha20Rng;
use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use rand::SeedableRng;

use std::sync::{Arc};
use tokio::sync::{Mutex, mpsc};
use futures::{StreamExt, SinkExt};
use uuid::Uuid;
use challenges::ProofChallenge;

mod message;
mod lean_runner;
mod utils;
mod challenges;
use message::*;
use room::*;





#[tokio::main]
async fn main() {
    let matcher = Arc::new(Mutex::new(Matchmaker::new()));
    let challenge_map: Arc<Mutex<HashMap<String, ProofChallenge>>> = Arc::new(Mutex::new(HashMap::new()));
    let app = Router::new()
    .route("/ws", get(move |ws: WebSocketUpgrade| {
        let matcher = matcher.clone();
        let challenge_map = challenge_map.clone(); // 👈
        async move {
            ws.on_upgrade(move |socket| handle_socket(socket, matcher, challenge_map))
        }
    }));


    println!("🚀 Running server on ws://localhost:3000/ws");
   // let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
   let listener = TcpListener::bind("127.0.0.1:3000").await.expect("Failed to bind to port 3000");

    // Start serving using axum::serve
    axum::serve(listener, app).await.unwrap();
   




}

async fn handle_socket(stream: WebSocket, matcher: Arc<Mutex<Matchmaker>>, challenge_map: Arc<Mutex<HashMap<String, ProofChallenge>>>) {
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
        tx.send(serde_json::to_string(&ServerMessage::MatchFound {
            opponent: opponent_id.clone(),
        }).unwrap()).unwrap();

        opponent_tx.send(serde_json::to_string(&ServerMessage::MatchFound {
            opponent: player_id.clone(),
        }).unwrap()).unwrap();



let challenges = challenges::get_problems();
let mut rng = seeded_rng();

let challenge = challenges.choose(&mut rng).unwrap().clone();

// Send challenge to both players
let challenge_msg = ServerMessage::Challenge {
    goal: challenge.goal.clone(),
    imports: challenge.imports.clone(),
};

let msg_json = serde_json::to_string(&challenge_msg).unwrap();
tx.send(msg_json.clone()).unwrap();
opponent_tx.send(msg_json).unwrap();
// ✅ Store the challenge for both players
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
    

// Store challenge somewhere if needed

    while let Some(Ok(Message::Text(text))) = receiver.next().await {
        match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::SubmitProof { code, player_id }) => {
                let map = challenge_map.lock().await;
                if let Some(challenge) = map.get(&player_id).cloned() {
                    let clean_code = code
                    .lines()
                    .map(str::trim) // removes leading/trailing spaces
                    .collect::<Vec<_>>()
                    .join("\n");
                
                    let wrapped = format!(
                        "{}\n\ntheorem goal : {} := by\n{}",
                        challenge.imports.join("\n"),
                        challenge.goal,
                        clean_code.lines().map(|line| format!("{line}")).collect::<Vec<_>>().join("\n"),
                    );
                    
                    
                    
                
                    let processed = utils::preprocess(&wrapped);
                
                    match lean_runner::run_proof(&processed, &format!("{}_proof.lean", player_id)) {                
                        Ok(output) => {
                            tx.send(serde_json::to_string(&ServerMessage::ProofResult {
                                success: true,
                                output,
                            }).unwrap()).unwrap();
                            let mut mm = matcher.lock().await;
                            if let Some(room_id) = mm.rooms.iter().find_map(|(id, room)| {
                                if room.player1 == player_id || room.player2.as_ref().map(|(p, _)| p == &player_id).unwrap_or(false) {
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            }) {
                                if let Some((_p1, _p2)) = mm.declare_winner(&room_id, &player_id) {
                                    if let Some(room) = mm.rooms.get(&room_id) {
                                        let _ = room.tx1.send(serde_json::to_string(&ServerMessage::GameEnded {
                                            winner: player_id.clone(),
                                        }).unwrap());
                        
                                        if let Some((_, tx2)) = &room.player2 {
                                            let _ = tx2.send(serde_json::to_string(&ServerMessage::GameEnded {
                                                winner: player_id.clone(),
                                            }).unwrap());
                        }
                    }
                }
            }
        }
                        Err(err) => {
                            tx.send(serde_json::to_string(&ServerMessage::ProofResult {
                                success: false,
                                output: err,
                            }).unwrap()).unwrap();
                        }
                    }
                } else {
                    tx.send(serde_json::to_string(&ServerMessage::ProofResult {
                        success: false,
                        output: "Challenge not found for this player".to_string(),
                    }).unwrap()).unwrap();
                }
               
              
            }
            _ => {}
        }
    }
}
        
    

fn seeded_rng() -> ChaCha20Rng {
    let seed = rand::random::<[u8; 32]>();
    ChaCha20Rng::from_seed(seed)
}
        