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
    GetSpectatorRoom {
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
    CreatePrivateRoom {
        player_id: PlayerId,
        category: Option<String>,
        difficulty: Option<u8>,
        duration_secs: Option<i64>,
    },
    JoinPrivateRoom {
        player_id: PlayerId,
        room_code: String,
    },
    SpectateRoom {
        spectator_id: PlayerId,
        room_code: String,
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

    pub async fn get_spectator_room(&self, player_id: PlayerId) -> Option<RoomId> {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(MatchmakerCommand::GetSpectatorRoom { player_id, reply })
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

    pub async fn create_private_room(
        &self,
        player_id: PlayerId,
        category: Option<String>,
        difficulty: Option<u8>,
        duration_secs: Option<i64>,
    ) {
        let _ = self
            .tx
            .send(MatchmakerCommand::CreatePrivateRoom {
                player_id,
                category,
                difficulty,
                duration_secs,
            })
            .await;
    }

    pub async fn join_private_room(&self, player_id: PlayerId, room_code: String) {
        let _ = self
            .tx
            .send(MatchmakerCommand::JoinPrivateRoom {
                player_id,
                room_code,
            })
            .await;
    }

    pub async fn spectate_room(&self, spectator_id: PlayerId, room_code: String) {
        let _ = self
            .tx
            .send(MatchmakerCommand::SpectateRoom {
                spectator_id,
                room_code,
            })
            .await;
    }
}

pub struct WaitingPlayer {
    pub player_id: PlayerId,
    pub queued_at: Instant,
    pub rating: Rating,
}

#[derive(Debug, Clone)]
pub struct PendingPrivateRoom {
    pub host_id: PlayerId,
    pub room_code: String,
    pub category: Option<String>,
    pub difficulty: Option<u8>,
    pub duration_secs: Option<i64>,
    pub created_at: Instant,
}

pub struct Matchmaker {
    live_sessions: HashMap<PlayerId, PlayerSession>,
    waiting: Vec<WaitingPlayer>,
    player_ratings: HashMap<PlayerId, Rating>,
    rooms: HashMap<RoomId, mpsc::Sender<RoomCommand>>,
    player_rooms: HashMap<PlayerId, RoomId>,
    room_players: HashMap<RoomId, (PlayerId, PlayerId)>,
    room_challenges: HashMap<RoomId, ProofChallenge>,
    pending_private_rooms: HashMap<String, PendingPrivateRoom>,
    player_pending_rooms: HashMap<PlayerId, String>,
    code_to_room: HashMap<String, RoomId>,
    room_to_code: HashMap<RoomId, String>,
    spectator_rooms: HashMap<PlayerId, RoomId>,
    config: Arc<Config>,
    pool: Arc<VerifierPool>,
    rate_limiter: Arc<RateLimiter>,
    rating_system: Arc<dyn RatingSystem>,
    history_store: Arc<crate::game::history::HistoryStore>,
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
        Self::spawn_with_options(
            config,
            round_duration,
            Arc::new(crate::game::history::HistoryStore::new(None)),
        )
    }

    pub fn spawn_with_history(
        config: Arc<Config>,
        history_store: Arc<crate::game::history::HistoryStore>,
    ) -> MatchmakerHandle {
        Self::spawn_with_options(config, Duration::from_secs(300), history_store)
    }

    pub fn spawn_with_options(
        config: Arc<Config>,
        round_duration: Duration,
        history_store: Arc<crate::game::history::HistoryStore>,
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
            pending_private_rooms: HashMap::new(),
            player_pending_rooms: HashMap::new(),
            code_to_room: HashMap::new(),
            room_to_code: HashMap::new(),
            spectator_rooms: HashMap::new(),
            config,
            pool: pool.clone(),
            rate_limiter: rate_limiter.clone(),
            rating_system,
            history_store,
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
        let mut cleanup_ticks = 0usize;

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    // Try to match players with widening brackets
                    self.attempt_matchmaking().await;
                    cleanup_ticks += 1;
                    if cleanup_ticks >= 120 {
                        cleanup_ticks = 0;
                        self.rate_limiter.cleanup_stale().await;
                    }
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
                            self.handle_queue_leave(player_id).await;
                        }
                        Some(MatchmakerCommand::CreatePrivateRoom {
                            player_id,
                            category,
                            difficulty,
                            duration_secs,
                        }) => {
                            self.handle_create_private_room(
                                player_id,
                                category,
                                difficulty,
                                duration_secs,
                            )
                            .await;
                        }
                        Some(MatchmakerCommand::JoinPrivateRoom {
                            player_id,
                            room_code,
                        }) => {
                            self.handle_join_private_room(player_id, room_code).await;
                        }
                        Some(MatchmakerCommand::SpectateRoom {
                            spectator_id,
                            room_code,
                        }) => {
                            self.handle_spectate_room(spectator_id, room_code).await;
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
                        Some(MatchmakerCommand::GetSpectatorRoom { player_id, reply }) => {
                            let _ = reply.send(self.spectator_rooms.get(&player_id).copied());
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
        crate::metrics::get_metrics().set_players_online(self.live_sessions.len() as i64);
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
        let bot_session = PlayerSession::new(bot_id, bot_tx, Some("Lean Practice Bot".to_string()));

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

    async fn handle_queue_leave(&mut self, player_id: PlayerId) {
        self.waiting.retain(|w| w.player_id != player_id);
        if let Some(code) = self.player_pending_rooms.remove(&player_id) {
            self.pending_private_rooms.remove(&code);
        }
        if let Some(room_tx) = self
            .spectator_rooms
            .remove(&player_id)
            .and_then(|room_id| self.rooms.get(&room_id))
        {
            let _ = room_tx
                .send(RoomCommand::RemoveSpectator {
                    spectator_id: player_id,
                })
                .await;
        }
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
        self.create_match_internal(
            p1_session, p2_session, p1_rating, p2_rating, false, None, None, None, None,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_match_internal(
        &mut self,
        p1_session: PlayerSession,
        p2_session: PlayerSession,
        p1_rating: Rating,
        p2_rating: Rating,
        is_unrated: bool,
        room_code: Option<String>,
        category: Option<String>,
        difficulty: Option<u8>,
        duration_secs: Option<i64>,
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

        let duration = match duration_secs {
            Some(secs) => Duration::from_secs(secs.clamp(60, 1800) as u64),
            None => self.round_duration,
        };

        let (room_tx, room_rx) = mpsc::channel(64);

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

        Room::spawn(
            room_id,
            p1,
            p2,
            p1_session.tx.clone(),
            p2_session.tx.clone(),
            p1_rating,
            p2_rating,
            p1_info.clone(),
            p2_info.clone(),
            challenge.clone(),
            None,
            duration,
            is_unrated,
            room_code.clone(),
            self.config.clone(),
            self.pool.clone(),
            self.rating_system.clone(),
            self.history_store.clone(),
            self.self_tx.clone(),
            room_rx,
            room_tx.clone(),
        );

        self.rooms.insert(room_id, room_tx);
        self.player_rooms.insert(p1, room_id);
        self.player_rooms.insert(p2, room_id);
        self.room_players.insert(room_id, (p1, p2));
        self.room_challenges.insert(room_id, challenge.clone());
        if let Some(ref code) = room_code {
            self.code_to_room.insert(code.clone(), room_id);
            self.room_to_code.insert(room_id, code.clone());
        }
        crate::metrics::get_metrics().inc_games_in_progress();

        let duration_ms = duration.as_millis() as i64;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let problem = crate::ws::message::Problem {
            id: format!("problem_{}", room_id),
            goal: challenge.goal.clone(),
            imports: challenge.imports.clone(),
            difficulty: difficulty.unwrap_or(1),
            category: category.unwrap_or_else(|| "logic".to_string()),
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

        tracing::info!(room_id = %room_id, p1 = %p1, p2 = %p2, is_unrated = %is_unrated, ?room_code, "Match created and room actor spawned");
    }

    fn generate_room_code(&self) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = seeded_rng();
        loop {
            let code: String = (0..6)
                .map(|_| *CHARSET.choose(&mut rng).unwrap() as char)
                .collect();
            if !self.pending_private_rooms.contains_key(&code)
                && !self.code_to_room.contains_key(&code)
            {
                return code;
            }
        }
    }

    async fn handle_create_private_room(
        &mut self,
        player_id: PlayerId,
        category: Option<String>,
        difficulty: Option<u8>,
        duration_secs: Option<i64>,
    ) {
        if self.player_rooms.contains_key(&player_id) {
            if let Some(session) = self.live_sessions.get(&player_id) {
                let _ = session
                    .tx
                    .send(ServerMessage::ServerError {
                        code: "invalid_state".to_string(),
                        message: "already in a match".to_string(),
                        retry_after_ms: None,
                        retryable: false,
                    })
                    .await;
            }
            return;
        }

        self.waiting.retain(|w| w.player_id != player_id);

        if let Some(old_code) = self.player_pending_rooms.remove(&player_id) {
            self.pending_private_rooms.remove(&old_code);
        }

        let room_code = self.generate_room_code();
        let pending = PendingPrivateRoom {
            host_id: player_id,
            room_code: room_code.clone(),
            category: category.clone(),
            difficulty,
            duration_secs,
            created_at: Instant::now(),
        };

        self.pending_private_rooms
            .insert(room_code.clone(), pending);
        self.player_pending_rooms
            .insert(player_id, room_code.clone());

        if let Some(session) = self.live_sessions.get(&player_id) {
            let _ = session
                .tx
                .send(ServerMessage::PrivateRoomCreated {
                    room_code: room_code.clone(),
                    category,
                    difficulty,
                    duration_secs,
                })
                .await;
            let _ = session
                .tx
                .send(ServerMessage::PrivateRoomWaiting {
                    room_code,
                    host_username: session.username.clone(),
                })
                .await;
        }
    }

    async fn handle_join_private_room(&mut self, player_id: PlayerId, room_code: String) {
        if self.player_rooms.contains_key(&player_id) {
            if let Some(session) = self.live_sessions.get(&player_id) {
                let _ = session
                    .tx
                    .send(ServerMessage::ServerError {
                        code: "invalid_state".to_string(),
                        message: "already in a match".to_string(),
                        retry_after_ms: None,
                        retryable: false,
                    })
                    .await;
            }
            return;
        }

        let normalized = room_code.trim().to_ascii_uppercase();
        let pending = match self.pending_private_rooms.remove(&normalized) {
            Some(p) => p,
            None => {
                if let Some(session) = self.live_sessions.get(&player_id) {
                    let _ = session
                        .tx
                        .send(ServerMessage::ServerError {
                            code: "room_not_found".to_string(),
                            message: "private room code not found or match already started"
                                .to_string(),
                            retry_after_ms: None,
                            retryable: false,
                        })
                        .await;
                }
                return;
            }
        };

        if pending.host_id == player_id {
            // Restore pending room
            self.pending_private_rooms.insert(normalized, pending);
            if let Some(session) = self.live_sessions.get(&player_id) {
                let _ = session
                    .tx
                    .send(ServerMessage::ServerError {
                        code: "invalid_action".to_string(),
                        message: "cannot join your own private room".to_string(),
                        retry_after_ms: None,
                        retryable: false,
                    })
                    .await;
            }
            return;
        }

        self.player_pending_rooms.remove(&pending.host_id);
        self.waiting
            .retain(|w| w.player_id != player_id && w.player_id != pending.host_id);
        if let Some(old_code) = self.player_pending_rooms.remove(&player_id) {
            self.pending_private_rooms.remove(&old_code);
        }

        let host_session = match self.live_sessions.get(&pending.host_id) {
            Some(s) => s.clone(),
            None => {
                if let Some(session) = self.live_sessions.get(&player_id) {
                    let _ = session
                        .tx
                        .send(ServerMessage::ServerError {
                            code: "host_disconnected".to_string(),
                            message: "room host is no longer connected".to_string(),
                            retry_after_ms: None,
                            retryable: false,
                        })
                        .await;
                }
                return;
            }
        };

        let joiner_session = match self.live_sessions.get(&player_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let host_rating = self
            .player_ratings
            .get(&pending.host_id)
            .copied()
            .unwrap_or_default();
        let joiner_rating = self
            .player_ratings
            .get(&player_id)
            .copied()
            .unwrap_or_default();

        self.create_match_internal(
            host_session,
            joiner_session,
            host_rating,
            joiner_rating,
            true, // is_unrated
            Some(normalized),
            pending.category,
            pending.difficulty,
            pending.duration_secs,
        )
        .await;
    }

    async fn handle_spectate_room(&mut self, spectator_id: PlayerId, room_code: String) {
        let normalized = room_code.trim().to_ascii_uppercase();

        let room_id = self
            .code_to_room
            .get(&normalized)
            .copied()
            .or_else(|| uuid::Uuid::parse_str(room_code.trim()).ok().map(RoomId));

        let target_room_id = match room_id {
            Some(id) if self.rooms.contains_key(&id) => id,
            _ => {
                if let Some(session) = self.live_sessions.get(&spectator_id) {
                    let (code, msg) = if self.pending_private_rooms.contains_key(&normalized) {
                        (
                            "match_not_started",
                            "private room match has not started yet",
                        )
                    } else {
                        (
                            "room_not_found",
                            "room not found or match has already ended",
                        )
                    };
                    let _ = session
                        .tx
                        .send(ServerMessage::ServerError {
                            code: code.to_string(),
                            message: msg.to_string(),
                            retry_after_ms: None,
                            retryable: false,
                        })
                        .await;
                }
                return;
            }
        };

        if let (Some(room_tx), Some(session)) = (
            self.rooms.get(&target_room_id),
            self.live_sessions.get(&spectator_id),
        ) {
            let _ = room_tx
                .send(RoomCommand::AddSpectator {
                    spectator_id,
                    tx: session.tx.clone(),
                })
                .await;
            self.spectator_rooms.insert(spectator_id, target_room_id);
        }
    }

    fn get_room_tx(&self, player_id: &PlayerId) -> Option<&mpsc::Sender<RoomCommand>> {
        let room_id = self.player_rooms.get(player_id)?;
        self.rooms.get(room_id)
    }

    async fn handle_disconnect(&mut self, player_id: PlayerId) {
        self.waiting.retain(|w| w.player_id != player_id);
        if let Some(code) = self.player_pending_rooms.remove(&player_id) {
            self.pending_private_rooms.remove(&code);
        }
        if let Some(room_tx) = self
            .spectator_rooms
            .remove(&player_id)
            .and_then(|room_id| self.rooms.get(&room_id))
        {
            let _ = room_tx
                .send(RoomCommand::RemoveSpectator {
                    spectator_id: player_id,
                })
                .await;
        }

        if let Some(room_tx) = self.get_room_tx(&player_id) {
            let _ = room_tx.send(RoomCommand::Disconnect { player_id }).await;
        }

        self.live_sessions.remove(&player_id);
        // Do not remove rate limit bucket on disconnect so rate limits persist across reconnects.
        crate::metrics::get_metrics().set_players_online(self.live_sessions.len() as i64);
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
        if let Some(code) = self.room_to_code.remove(&room_id) {
            self.code_to_room.remove(&code);
        }
        self.spectator_rooms.retain(|_, r| *r != room_id);
        if let Some((p1, p2)) = self.room_players.remove(&room_id) {
            self.player_rooms.remove(&p1);
            self.player_rooms.remove(&p2);
        }
        crate::metrics::get_metrics().dec_games_in_progress();
        tracing::info!(room_id = %room_id, "Room unregistered cleanly from matchmaker registry");
    }
}

fn seeded_rng() -> ChaCha20Rng {
    let seed = rand::random::<[u8; 32]>();
    ChaCha20Rng::from_seed(seed)
}
