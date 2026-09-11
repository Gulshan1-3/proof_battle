#![allow(unused_imports)]
#![allow(dead_code)]
use std::collections::HashMap;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;
//use axum::extract::ws::WebSocket;

pub type PlayerId = String;

pub struct Match {
    player1: PlayerId,
    player2: PlayerId,
    goal: String,
    imports: Vec<String>,
    solved: bool,
    winner: Option<PlayerId>,
}



pub struct GameRoom {
    pub player1: PlayerId,
    pub tx1: mpsc::UnboundedSender<String>,
    pub player2: Option<(PlayerId, mpsc::UnboundedSender<String>)>,
}

pub struct Matchmaker {
    pub waiting: Option<(PlayerId, mpsc::UnboundedSender<String>)>,
    pub rooms: HashMap<String, GameRoom>,
    pub match_states: HashMap<String, Match>,
}

impl Matchmaker {
    pub fn new() -> Self {
        Self {
            waiting: None,
            rooms: HashMap::new(),
            match_states: HashMap::new(),
        }
    }

    pub fn add_player(
        &mut self,
        id: PlayerId,
        tx: mpsc::UnboundedSender<String>,
    ) -> Option<(String, PlayerId, mpsc::UnboundedSender<String>)> {
        if let Some((other_id, other_tx)) = self.waiting.take() {
            let room_id = Uuid::new_v4().to_string();
    
            self.rooms.insert(
                room_id.clone(),
                GameRoom {
                    player1: other_id.clone(),
                    tx1: other_tx.clone(),
                    player2: Some((id.clone(), tx.clone())),
                },
            );
    
            self.match_states.insert(
                room_id.clone(),
                Match {
                    player1: other_id.clone(),
                    player2: id.clone(),
                    goal: String::new(), // to be filled later
                    imports: vec![],
                    solved: false,
                    winner: None,
                },
            );
    
            Some((room_id, other_id, other_tx))
        } else {
            self.waiting = Some((id, tx));
            None
        }
    }
    
    pub fn set_challenge(&mut self, room_id: &str, goal: String, imports: Vec<String>) {
        if let Some(m) = self.match_states.get_mut(room_id) {
            m.goal = goal;
            m.imports = imports;
        }
    }

    pub fn declare_winner(&mut self, room_id: &str, winner_id: &str) -> Option<(PlayerId, PlayerId)> {
        if let Some(m) = self.match_states.get_mut(room_id) {
            if m.solved {
                return None; 
            }
            m.solved = true;
            m.winner = Some(winner_id.to_string());
            Some((m.player1.clone(), m.player2.clone()))
        } else {
            None
        }
    }
    
}



