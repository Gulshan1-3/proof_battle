use serde::{Deserialize, Serialize};
#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Join { username: String },
    SubmitProof { code: String, player_id: String },
}

#[derive(Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum ServerMessage {
    Joined { player_id: String },
    MatchFound { opponent: String },
    Challenge { goal: String, imports: Vec<String> },
    ProofResult { success: bool, output: String },
    GameEnded { winner: String },
    Error { message: String },
}

