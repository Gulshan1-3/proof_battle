use super::elo::{Elo, Rating, RatingSystem};
use super::queue::{RateLimiter, VerifierPool};
use super::room::{Room, RoomCommand};
use super::session::PlayerSession;
use super::{PlayerId, RoomId};
use crate::challenges::{self, ProofChallenge};
use crate::config::Config;
use crate::message::ServerMessage;
use rand::SeedableRng;
use rand::prelude::IndexedRandom;
use rand_chacha::ChaCha20Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum MatchmakerCommand {
    Connect {
        player_id: PlayerId,
        tx: mpsc::Sender<ServerMessage>,
        username: Option<String>,
    },
    QueueJoin {
        player_id: PlayerId,
    },
    PracticeJoin {
        player_id: PlayerId,
    },
    QueueLeave {
        player_id: PlayerId,
    },
    Disconnect {
        player_id: PlayerId,
    },
    SubmitProof {
        player_id: PlayerId,
        code: String,
        req_id: Option<String>,
    },
    Resign {
        player_id: PlayerId,
    },
    Ping {
        player_id: PlayerId,
    },
    Reattach {
        player_id: PlayerId,
        new_tx: mpsc::Sender<ServerMessage>,
        reply: oneshot::Sender<bool>,
    },
    RoomFinished {
        room_id: RoomId,
    },
    GetRegistryLen {
        reply: oneshot::Sender<usize>,
    },
    GetWaitingLen {
        reply: oneshot::Sender<usize>,
    },
    GetPlayerRoom {
        player_id: PlayerId,
        reply: oneshot::Sender<Option<RoomId>>,
    },
    GetRoomChallenge {
        room_id: RoomId,
        reply: oneshot::Sender<Option<ProofChallenge>>,
    },
    SetPlayerRating {
        player_id: PlayerId,
        rating: Rating,
    },
}

#[derive(Clone)]
pub struct MatchmakerHandle {
    tx: mpsc::Sender<MatchmakerCommand>,
    pub pool: Arc<VerifierPool>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl MatchmakerHandle {
    pub fn new(
        tx: mpsc::Sender<MatchmakerCommand>,
        pool: Arc<VerifierPool>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            tx,
            pool,
            rate_limiter,
        }
    }

    pub async fn connect(
        &self,
        player_id: PlayerId,
        tx: mpsc::Sender<ServerMessage>,
        username: Option<String>,
    ) {
        let _ = self
            .tx
            .send(MatchmakerCommand::Connect {
                player_id,
                tx,
                username,
            })
            .await;
    }

    pub async fn join_queue(&self, player_id: PlayerId) {
        let _ = self
            .tx
            .send(MatchmakerCommand::QueueJoin { player_id })
            .await;
    }

    pub async fn queue_join(&self, player_id: PlayerId) {
        self.join_queue(player_id).await;
    }

    pub async fn practice_join(&self, player_id: PlayerId) {
        let _ = self
            .tx
            .send(MatchmakerCommand::PracticeJoin { player_id })
            .await;
    }

    pub async fn queue_leave(&self, player_id: PlayerId) {
        let _ = self
            .tx
            .send(MatchmakerCommand::QueueLeave { player_id })
            .await;
    }

    pub async fn disconnect(&self, player_id: PlayerId) {
        let _ = self
            .tx
            .send(MatchmakerCommand::Disconnect { player_id })
            .await;
    }

    pub async fn submit_proof(&self, player_id: PlayerId, code: String, req_id: Option<String>) {
        let _ = self
            .tx
            .send(MatchmakerCommand::SubmitProof {
                player_id,
                code,
                req_id,
            })
            .await;
    }

    pub async fn resign(&self, player_id: PlayerId) {
        let _ = self.tx.send(MatchmakerCommand::Resign { player_id }).await;
    }

    pub async fn ping(&self, player_id: PlayerId) {
        let _ = self.tx.send(MatchmakerCommand::Ping { player_id }).await;
    }

    pub async fn reattach(&self, player_id: PlayerId, new_tx: mpsc::Sender<ServerMessage>) -> bool {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(MatchmakerCommand::Reattach {
                player_id,
                new_tx,
                reply,
            })
            .await
            .is_ok()
        {
            rx.await.unwrap_or(false)
        } else {
            false
        }
    }

    pub async fn get_registry_len(&self) -> usize {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(MatchmakerCommand::GetRegistryLen { reply })
            .await
            .is_ok()
        {
            rx.await.unwrap_or(0)
        } else {
            0
        }
    }

    pub async fn get_waiting_len(&self) -> usize {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(MatchmakerCommand::GetWaitingLen { reply })
            .await
            .is_ok()
        {
            rx.await.unwrap_or(0)
        } else {
            0
        }
    }

    pub async fn get_player_room(&self, player_id: PlayerId) -> Option<RoomId> {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(MatchmakerCommand::GetPlayerRoom { player_id, reply })
            .await
            .is_ok()
        {
            rx.await.unwrap_or(None)
        } else {
            None
        }
    }
    pub async fn get_room_challenge(&self, room_id: RoomId) -> Option<ProofChallenge> {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(MatchmakerCommand::GetRoomChallenge { room_id, reply })
            .await
            .is_ok()
        {
            rx.await.unwrap_or(None)
        } else {
            None
        }
    }

    pub async fn set_player_rating(&self, player_id: PlayerId, rating: Rating) {
        let _ = self
            .tx
            .send(MatchmakerCommand::SetPlayerRating { player_id, rating })
            .await;
    }
}

pub struct WaitingPlayer {
    pub player_id: PlayerId,
    pub queued_at: Instant,
    pub rating: Rating,
}

pub struct Matchmaker {
    live_sessions: HashMap<PlayerId, PlayerSession>,
    waiting: Vec<WaitingPlayer>,
    player_ratings: HashMap<PlayerId, Rating>,
    rooms: HashMap<RoomId, mpsc::Sender<RoomCommand>>,
    player_rooms: HashMap<PlayerId, RoomId>,
    room_players: HashMap<RoomId, (PlayerId, PlayerId)>,
    room_challenges: HashMap<RoomId, ProofChallenge>,
    config: Arc<Config>,
    pool: Arc<VerifierPool>,
    rate_limiter: Arc<RateLimiter>,
    rating_system: Arc<dyn RatingSystem>,
    round_duration: Duration,
    rx: mpsc::Receiver<MatchmakerCommand>,
    self_tx: mpsc::Sender<MatchmakerCommand>,
}

impl Matchmaker {
    pub fn spawn(config: Arc<Config>) -> MatchmakerHandle {
        Self::spawn_with_round_duration(config, Duration::from_secs(300))
    }

    pub fn spawn_with_round_duration(
        config: Arc<Config>,
        round_duration: Duration,
    ) -> MatchmakerHandle {
        let (tx, rx) = mpsc::channel(256);
        let pool = Arc::new(VerifierPool::new(
            config.lean_pool_size.max(1),
            config.check_lane_capacity.max(1),
            Duration::from_millis(500),
        ));
        let rate_limiter = Arc::new(RateLimiter::new());
        let rating_system = Arc::new(Elo::new());

        let matchmaker = Self {
            live_sessions: HashMap::new(),
            waiting: Vec::new(),
            player_ratings: HashMap::new(),
            rooms: HashMap::new(),
            player_rooms: HashMap::new(),
            room_players: HashMap::new(),
            room_challenges: HashMap::new(),
            config,
            pool: pool.clone(),
            rate_limiter: rate_limiter.clone(),
            rating_system,
            round_duration,
            rx,
            self_tx: tx.clone(),
        };

        tokio::spawn(async move {
            matchmaker.run().await;
        });

        MatchmakerHandle::new(tx, pool, rate_limiter)
    }

    pub async fn run(mut self) {
        let mut tick_interval = tokio::time::interval(Duration::from_millis(500));
        tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    // Try to match players with widening brackets
                    self.attempt_matchmaking().await;
                }
                cmd = self.rx.recv() => {
                    match cmd {
                        Some(MatchmakerCommand::Connect { player_id, tx, username }) => {
                            self.handle_connect(player_id, tx, username).await;
                        }
                        Some(MatchmakerCommand::QueueJoin { player_id }) => {
                            self.handle_queue_join(player_id).await;
                        }
                        Some(MatchmakerCommand::PracticeJoin { player_id }) => {
                            self.handle_practice_join(player_id).await;
                        }
                        Some(MatchmakerCommand::QueueLeave { player_id }) => {
                            self.handle_queue_leave(player_id);
                        }
                        Some(MatchmakerCommand::Disconnect { player_id }) => {
                            self.handle_disconnect(player_id).await;
                        }
                        Some(MatchmakerCommand::SubmitProof { player_id, code, req_id }) => {
                            self.handle_submit_proof(player_id, code, req_id).await;
                        }
                        Some(MatchmakerCommand::Resign { player_id }) => {
                            self.handle_resign(player_id).await;
                        }
                        Some(MatchmakerCommand::Ping { player_id }) => {
                            self.handle_ping(player_id).await;
                        }
                        Some(MatchmakerCommand::Reattach { player_id, new_tx, reply }) => {
                            self.handle_reattach(player_id, new_tx, reply).await;
                        }
                        Some(MatchmakerCommand::RoomFinished { room_id }) => {
                            self.handle_room_finished(room_id);
                        }
                        Some(MatchmakerCommand::GetRegistryLen { reply }) => {
                            let _ = reply.send(self.rooms.len());
                        }
                        Some(MatchmakerCommand::GetWaitingLen { reply }) => {
                            let _ = reply.send(self.waiting.len());
                        }
                        Some(MatchmakerCommand::GetPlayerRoom { player_id, reply }) => {
                            let _ = reply.send(self.player_rooms.get(&player_id).copied());
                        }
                        Some(MatchmakerCommand::GetRoomChallenge { room_id, reply }) => {
                            let _ = reply.send(self.room_challenges.get(&room_id).cloned());
                        }
                        Some(MatchmakerCommand::SetPlayerRating { player_id, rating }) => {
                            self.player_ratings.insert(player_id, rating);
                        }
                        None => {
                            tracing::info!("Matchmaker command channel closed, shutting down");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_connect(
        &mut self,
        player_id: PlayerId,
        tx: mpsc::Sender<ServerMessage>,
        username: Option<String>,
    ) {
        let session = PlayerSession::new(player_id, tx.clone(), username);
        self.live_sessions.insert(player_id, session);
    }

    async fn handle_queue_join(&mut self, player_id: PlayerId) {
        if self.player_rooms.contains_key(&player_id) {
            return;
        }

        if self.waiting.iter().any(|w| w.player_id == player_id) {
            return;
        }

        let rating = self
            .player_ratings
            .get(&player_id)
            .copied()
            .unwrap_or_default();

        self.waiting.push(WaitingPlayer {
            player_id,
            queued_at: Instant::now(),
            rating,
        });

        // Broadcast initial QueueStatus to queued player
        if let Some(session) = self.live_sessions.get(&player_id) {
            let _ = session
                .tx
                .send(ServerMessage::QueueStatus {
                    queue_size: self.waiting.len() as u32,
                    elo: rating.elo,
                    search_range: crate::ws::message::SearchRange {
                        min_elo: rating.elo - 200,
                        max_elo: rating.elo + 200,
                    },
                })
                .await;
        }

        // Trigger matchmaking check immediately
        self.attempt_matchmaking().await;
    }

    async fn handle_practice_join(&mut self, player_id: PlayerId) {
        if self.player_rooms.contains_key(&player_id) {
            return;
        }

        self.waiting.retain(|w| w.player_id != player_id);

        let rating = self
            .player_ratings
            .get(&player_id)
            .copied()
            .unwrap_or_default();

        let s1 = match self.live_sessions.get(&player_id) {
            Some(s) => s.clone(),
            None => return,
        };

        // Create a dummy bot session for solo practice
        let bot_id = PlayerId::new();
        let (bot_tx, _bot_rx) = mpsc::channel(16);
        let bot_session = PlayerSession::new(
            bot_id,
            bot_tx,
            Some("Lean Practice Bot".to_string()),
        );

        self.create_match(
            s1,
            bot_session,
            rating,
            Rating {
                elo: 1200,
                games_played: 50,
            },
        )
        .await;
    }

    fn handle_queue_leave(&mut self, player_id: PlayerId) {
        self.waiting.retain(|w| w.player_id != player_id);
    }

    async fn handle_reattach(
        &mut self,
        player_id: PlayerId,
        new_tx: mpsc::Sender<ServerMessage>,
        reply: oneshot::Sender<bool>,
    ) {
        if let Some(session) = self.live_sessions.get_mut(&player_id) {
            session.tx = new_tx.clone();
        }

        if let Some(room_tx) = self.get_room_tx(&player_id) {
            let (room_reply_tx, room_reply_rx) = oneshot::channel();
            let _ = room_tx
                .send(RoomCommand::Reattach {
                    player_id,
                    new_tx,
                    reply: room_reply_tx,
                })
                .await;
            let success = room_reply_rx.await.unwrap_or(false);
            let _ = reply.send(success);
        } else {
            let _ = reply.send(false);
        }
    }

    /// Closest-Pair Elo Matchmaking:
    /// - Wait time < 15s: bracket is [elo - 200, elo + 200]
    /// - Wait time >= 15s: bracket widens by 100 Elo every 5s
    /// - Pairs candidate with the smallest absolute rating difference
    async fn attempt_matchmaking(&mut self) {
        if self.waiting.len() < 2 {
            return;
        }

        let now = Instant::now();
        let mut best_pair: Option<(usize, usize, i32)> = None;

        for i in 0..self.waiting.len() {
            let p1 = &self.waiting[i];
            let elapsed_secs = now.duration_since(p1.queued_at).as_secs();
            let tolerance = if elapsed_secs < 15 {
                200
            } else {
                200 + ((elapsed_secs - 15) / 5) as i32 * 100
            };

            for j in (i + 1)..self.waiting.len() {
                let p2 = &self.waiting[j];
                let diff = (p1.rating.elo - p2.rating.elo).abs();

                if diff <= tolerance {
                    match best_pair {
                        None => best_pair = Some((i, j, diff)),
                        Some((_, _, best_diff)) if diff < best_diff => {
                            best_pair = Some((i, j, diff));
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some((i, j, _)) = best_pair {
            // Remove from waiting list (higher index first to keep indices valid)
            let (first, second) = if i > j { (i, j) } else { (j, i) };
            let p2 = self.waiting.remove(first);
            let p1 = self.waiting.remove(second);

            let s1 = match self.live_sessions.get(&p1.player_id) {
                Some(s) => s.clone(),
                None => return,
            };
            let s2 = match self.live_sessions.get(&p2.player_id) {
                Some(s) => s.clone(),
                None => {
                    self.waiting.push(p1);
                    return;
                }
            };

            self.create_match(s1, s2, p1.rating, p2.rating).await;
        }
    }

    async fn create_match(
        &mut self,
        p1_session: PlayerSession,
        p2_session: PlayerSession,
        p1_rating: Rating,
        p2_rating: Rating,
    ) {
        let room_id = RoomId::new();
        let p1 = p1_session.player_id;
        let p2 = p2_session.player_id;

        let challenges = challenges::get_problems();
        let mut rng = seeded_rng();
        let challenge = challenges
            .choose(&mut rng)
            .cloned()
            .unwrap_or_else(|| ProofChallenge {
                goal: "∀ n : ℕ, n + 0 = n".to_string(),
                imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
            });

        let (room_tx, room_rx) = mpsc::channel(64);

        Room::spawn(
            room_id,
            p1,
            p2,
            p1_session.tx.clone(),
            p2_session.tx.clone(),
            p1_rating,
            p2_rating,
            challenge.clone(),
            None,
            self.round_duration,
            self.config.clone(),
            self.pool.clone(),
            self.rating_system.clone(),
            self.self_tx.clone(),
            room_rx,
            room_tx.clone(),
        );

        self.rooms.insert(room_id, room_tx);
        self.player_rooms.insert(p1, room_id);
        self.player_rooms.insert(p2, room_id);
        self.room_players.insert(room_id, (p1, p2));
        self.room_challenges.insert(room_id, challenge.clone());

        let p1_info = crate::ws::message::PlayerInfo {
            player_id: p1.to_string(),
            username: p1_session.username.clone(),
            elo: p1_rating.elo,
        };
        let p2_info = crate::ws::message::PlayerInfo {
            player_id: p2.to_string(),
            username: p2_session.username.clone(),
            elo: p2_rating.elo,
        };

        let duration_ms = self.round_duration.as_millis() as i64;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let problem = crate::ws::message::Problem {
            id: format!("problem_{}", room_id),
            goal: challenge.goal.clone(),
            imports: challenge.imports.clone(),
            difficulty: 1,
            category: "logic".to_string(),
            hint: None,
            duration_ms,
        };

        let _ = p1_session
            .tx
            .send(ServerMessage::MatchFound {
                room_id: room_id.to_string(),
                you: p1_info.clone(),
                opponent: p2_info.clone(),
            })
            .await;
        let _ = p1_session
            .tx
            .send(ServerMessage::RoundStart {
                room_id: room_id.to_string(),
                problem: problem.clone(),
                starts_at_ms: now_ms,
                ends_at_ms: now_ms + duration_ms,
                duration_ms,
                server_time_ms: now_ms,
                seq: 1,
            })
            .await;

        let _ = p2_session
            .tx
            .send(ServerMessage::MatchFound {
                room_id: room_id.to_string(),
                you: p2_info,
                opponent: p1_info,
            })
            .await;
        let _ = p2_session
            .tx
            .send(ServerMessage::RoundStart {
                room_id: room_id.to_string(),
                problem,
                starts_at_ms: now_ms,
                ends_at_ms: now_ms + duration_ms,
                duration_ms,
                server_time_ms: now_ms,
                seq: 1,
            })
            .await;

        tracing::info!(room_id = %room_id, p1 = %p1, p2 = %p2, "Match created and room actor spawned");
    }

    fn get_room_tx(&self, player_id: &PlayerId) -> Option<&mpsc::Sender<RoomCommand>> {
        let room_id = self.player_rooms.get(player_id)?;
        self.rooms.get(room_id)
    }

    async fn handle_disconnect(&mut self, player_id: PlayerId) {
        self.waiting.retain(|w| w.player_id != player_id);

        if let Some(room_tx) = self.get_room_tx(&player_id) {
            let _ = room_tx.send(RoomCommand::Disconnect { player_id }).await;
        }

        self.live_sessions.remove(&player_id);
        self.rate_limiter.remove_player(&player_id).await;
    }

    async fn handle_submit_proof(
        &mut self,
        player_id: PlayerId,
        code: String,
        req_id: Option<String>,
    ) {
        if let Some(room_tx) = self.get_room_tx(&player_id) {
            let _ = room_tx
                .send(RoomCommand::SubmitProof {
                    player_id,
                    code,
                    req_id,
                })
                .await;
            return;
        }

        if let Some(session) = self.live_sessions.get(&player_id) {
            let _ = session
                .tx
                .send(ServerMessage::ServerError {
                    code: "invalid_state".to_string(),
                    message: "not currently in a match".to_string(),
                    retry_after_ms: None,
                    retryable: false,
                })
                .await;
        }
    }

    async fn handle_resign(&mut self, player_id: PlayerId) {
        if let Some(room_tx) = self.get_room_tx(&player_id) {
            let _ = room_tx.send(RoomCommand::Resign { player_id }).await;
        }
    }

    async fn handle_ping(&mut self, player_id: PlayerId) {
        if let Some(room_tx) = self.get_room_tx(&player_id) {
            let _ = room_tx.send(RoomCommand::Ping { player_id }).await;
            return;
        }
        if let Some(session) = self.live_sessions.get(&player_id) {
            let _ = session.tx.send(ServerMessage::Pong { ts: 0 }).await;
        }
    }

    fn handle_room_finished(&mut self, room_id: RoomId) {
        self.rooms.remove(&room_id);
        self.room_challenges.remove(&room_id);
        if let Some((p1, p2)) = self.room_players.remove(&room_id) {
            self.player_rooms.remove(&p1);
            self.player_rooms.remove(&p2);
        }
        tracing::info!(room_id = %room_id, "Room unregistered cleanly from matchmaker registry");
    }
}

fn seeded_rng() -> ChaCha20Rng {
    let seed = rand::random::<[u8; 32]>();
    ChaCha20Rng::from_seed(seed)
}
