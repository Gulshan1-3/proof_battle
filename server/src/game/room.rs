use super::elo::{Rating, RatingSystem};
use super::matchmaker::MatchmakerCommand;
use super::queue::VerifierPool;
use super::{PlayerId, RoomId};
use crate::challenges::ProofChallenge;
use crate::config::Config;
use crate::message::ServerMessage;
use crate::ws::message::{
    Diagnostic, DiagnosticSeverity, MatchOutcome, RejectReason, VerdictStatus,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum RoomCommand {
    SubmitProof {
        player_id: PlayerId,
        code: String,
        req_id: Option<String>,
    },
    Resign {
        player_id: PlayerId,
    },
    Disconnect {
        player_id: PlayerId,
    },
    Reattach {
        player_id: PlayerId,
        new_tx: mpsc::Sender<ServerMessage>,
        reply: oneshot::Sender<bool>,
    },
    Ping {
        player_id: PlayerId,
    },
    VerificationFinished {
        player_id: PlayerId,
        req_id: Option<String>,
        outcome: Result<crate::lean::VerifyOutcome, RejectReason>,
    },
}

struct RoomGuard {
    room_id: RoomId,
    matchmaker_tx: mpsc::Sender<MatchmakerCommand>,
}

impl Drop for RoomGuard {
    fn drop(&mut self) {
        let room_id = self.room_id;
        let tx = self.matchmaker_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(MatchmakerCommand::RoomFinished { room_id }).await;
        });
        tracing::debug!(room_id = %room_id, "Room actor dropped, shutdown sent via RAII guard");
    }
}

pub struct Room {
    pub room_id: RoomId,
    pub player1: PlayerId,
    pub player2: PlayerId,
    pub tx1: mpsc::Sender<ServerMessage>,
    pub tx2: mpsc::Sender<ServerMessage>,
    pub p1_rating: Rating,
    pub p2_rating: Rating,
    pub p1_disconnected_at: Option<Instant>,
    pub p2_disconnected_at: Option<Instant>,
    pub challenge: ProofChallenge,
    pub problem_id: Option<uuid::Uuid>,
    pub starts_at: Instant,
    pub deadline: Instant,
    pub duration: Duration,
    pub submission_count1: u32,
    pub submission_count2: u32,
    pub in_flight1: bool,
    pub in_flight2: bool,
    pub solved: bool,
    pub config: Arc<Config>,
    pub pool: Arc<VerifierPool>,
    pub rating_system: Arc<dyn RatingSystem>,
    pub matchmaker_tx: mpsc::Sender<MatchmakerCommand>,
    pub self_tx: mpsc::Sender<RoomCommand>,
    pub rx: mpsc::Receiver<RoomCommand>,
}

impl Room {
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        room_id: RoomId,
        player1: PlayerId,
        player2: PlayerId,
        tx1: mpsc::Sender<ServerMessage>,
        tx2: mpsc::Sender<ServerMessage>,
        p1_rating: Rating,
        p2_rating: Rating,
        challenge: ProofChallenge,
        problem_id: Option<uuid::Uuid>,
        duration: Duration,
        config: Arc<Config>,
        pool: Arc<VerifierPool>,
        rating_system: Arc<dyn RatingSystem>,
        matchmaker_tx: mpsc::Sender<MatchmakerCommand>,
        rx: mpsc::Receiver<RoomCommand>,
        self_tx: mpsc::Sender<RoomCommand>,
    ) {
        let now = Instant::now();
        let deadline = now + duration;
        let room = Self {
            room_id,
            player1,
            player2,
            tx1,
            tx2,
            p1_rating,
            p2_rating,
            p1_disconnected_at: None,
            p2_disconnected_at: None,
            challenge,
            problem_id,
            starts_at: now,
            deadline,
            duration,
            submission_count1: 0,
            submission_count2: 0,
            in_flight1: false,
            in_flight2: false,
            solved: false,
            config,
            pool,
            rating_system,
            matchmaker_tx,
            self_tx,
            rx,
        };

        tokio::spawn(async move {
            room.run().await;
        });
    }

    pub async fn run(mut self) {
        let _guard = RoomGuard {
            room_id: self.room_id,
            matchmaker_tx: self.matchmaker_tx.clone(),
        };

        let mut tick_interval = tokio::time::interval(Duration::from_millis(200));
        tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    let now = Instant::now();
                    if !self.solved && now >= self.deadline {
                        tracing::info!(room_id = %self.room_id, "round deadline expired");
                        self.handle_deadline_expired().await;
                        break;
                    }

                    // Check reconnection grace window for player1
                    if let Some(disc_at) = self.p1_disconnected_at {
                        let expired = !self.solved && now.duration_since(disc_at) >= self.config.reconnect_grace_window;
                        if expired {
                            tracing::info!(room_id = %self.room_id, player = %self.player1, "player 1 reconnection grace window expired");
                            self.handle_forfeit(self.player1).await;
                            break;
                        }
                    }

                    // Check reconnection grace window for player2
                    if let Some(disc_at) = self.p2_disconnected_at {
                        let expired = !self.solved && now.duration_since(disc_at) >= self.config.reconnect_grace_window;
                        if expired {
                            tracing::info!(room_id = %self.room_id, player = %self.player2, "player 2 reconnection grace window expired");
                            self.handle_forfeit(self.player2).await;
                            break;
                        }
                    }
                }
                cmd = self.rx.recv() => {
                    match cmd {
                        Some(RoomCommand::SubmitProof { player_id, code, req_id }) => {
                            self.handle_submit(player_id, code, req_id).await;
                        }
                        Some(RoomCommand::VerificationFinished { player_id, req_id, outcome }) => {
                            let should_stop = self.handle_verification_finished(player_id, req_id, outcome).await;
                            if should_stop {
                                break;
                            }
                        }
                        Some(RoomCommand::Resign { player_id }) => {
                            self.handle_forfeit(player_id).await;
                            break;
                        }
                        Some(RoomCommand::Disconnect { player_id }) => {
                            self.handle_disconnect(player_id).await;
                        }
                        Some(RoomCommand::Reattach { player_id, new_tx, reply }) => {
                            let success = self.handle_reattach(player_id, new_tx).await;
                            let _ = reply.send(success);
                        }
                        Some(RoomCommand::Ping { player_id }) => {
                            let tx = if player_id == self.player1 { &self.tx1 } else { &self.tx2 };
                            let _ = tx.send(ServerMessage::Pong { ts: 0 }).await;
                        }
                        None => {
                            tracing::debug!(room_id = %self.room_id, "room command channel closed");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_reattach(
        &mut self,
        player_id: PlayerId,
        new_tx: mpsc::Sender<ServerMessage>,
    ) -> bool {
        if self.solved {
            return false;
        }

        if player_id == self.player1 {
            self.tx1 = new_tx.clone();
            self.p1_disconnected_at = None;
        } else if player_id == self.player2 {
            self.tx2 = new_tx.clone();
            self.p2_disconnected_at = None;
        } else {
            return false;
        }

        // Resync state to the reattached player:
        // Must resync problem, remaining time, seq.
        // MUST NEVER leak opponent's code or ongoing verification verdict to client.
        let now = Instant::now();
        let remaining_ms = self.deadline.saturating_duration_since(now).as_millis() as i64;
        let elapsed_ms = now.duration_since(self.starts_at).as_millis() as i64;
        let now_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let problem = crate::ws::message::Problem {
            id: format!("problem_{}", self.room_id),
            goal: self.challenge.goal.clone(),
            imports: self.challenge.imports.clone(),
            difficulty: 1,
            category: "logic".to_string(),
            hint: None,
            duration_ms: self.duration.as_millis() as i64,
        };

        let resync_msg = ServerMessage::RoundStart {
            room_id: self.room_id.to_string(),
            problem,
            starts_at_ms: now_unix_ms - elapsed_ms,
            ends_at_ms: now_unix_ms + remaining_ms,
            duration_ms: self.duration.as_millis() as i64,
            server_time_ms: now_unix_ms,
            seq: 2, // Resync sequence
        };

        let _ = new_tx.send(resync_msg).await;
        tracing::info!(room_id = %self.room_id, player_id = %player_id, "player successfully reattached to active room");
        true
    }

    async fn handle_submit(&mut self, player_id: PlayerId, code: String, req_id: Option<String>) {
        if self.solved {
            let tx = if player_id == self.player1 {
                &self.tx1
            } else {
                &self.tx2
            };
            let _ = tx
                .send(ServerMessage::ServerError {
                    code: "invalid_state".to_string(),
                    message: "game already ended".to_string(),
                    retry_after_ms: None,
                    retryable: false,
                })
                .await;
            return;
        }

        if player_id == self.player1 {
            if self.in_flight1 {
                let _ = self
                    .tx1
                    .send(ServerMessage::ServerError {
                        code: "busy".to_string(),
                        message: "busy: verification in progress".to_string(),
                        retry_after_ms: Some(1000),
                        retryable: true,
                    })
                    .await;
                return;
            }
            self.in_flight1 = true;
            self.submission_count1 += 1;
        } else if player_id == self.player2 {
            if self.in_flight2 {
                let _ = self
                    .tx2
                    .send(ServerMessage::ServerError {
                        code: "busy".to_string(),
                        message: "busy: verification in progress".to_string(),
                        retry_after_ms: Some(1000),
                        retryable: true,
                    })
                    .await;
                return;
            }
            self.in_flight2 = true;
            self.submission_count2 += 1;
        } else {
            return;
        }

        let internal_tx = self.self_tx.clone();
        let config = self.config.clone();
        let pool = self.pool.clone();
        let challenge = self.challenge.clone();

        tokio::spawn(async move {
            let clean_code = code.lines().map(str::trim).collect::<Vec<_>>().join("\n");

            let wrapped = match crate::lean::wrap_problem(
                &challenge.imports,
                &challenge.goal,
                &clean_code,
                &config,
            ) {
                Ok((wrapped, _)) => wrapped,
                Err(reason) => {
                    let outcome = crate::lean::VerifyOutcome {
                        verdict: crate::lean::Verdict::Rejected {
                            reason: crate::lean::RejectReason::RejectedByFilter(format!(
                                "{reason:?}"
                            )),
                            stderr: format!("Proof rejected: {reason:?}"),
                        },
                        elapsed_ms: 0,
                        stdout_bytes: 0,
                        stderr_bytes: 0,
                        truncated: false,
                    };
                    let _ = internal_tx
                        .send(RoomCommand::VerificationFinished {
                            player_id,
                            req_id,
                            outcome: Ok(outcome),
                        })
                        .await;
                    return;
                }
            };

            let processed = crate::utils::preprocess(&wrapped);
            let outcome = pool.run_submit(&config, &processed).await;

            let _ = internal_tx
                .send(RoomCommand::VerificationFinished {
                    player_id,
                    req_id,
                    outcome,
                })
                .await;
        });
    }

    async fn handle_verification_finished(
        &mut self,
        player_id: PlayerId,
        req_id: Option<String>,
        result: Result<crate::lean::VerifyOutcome, RejectReason>,
    ) -> bool {
        if player_id == self.player1 {
            self.in_flight1 = false;
        } else if player_id == self.player2 {
            self.in_flight2 = false;
        }

        let player_tx = if player_id == self.player1 {
            &self.tx1
        } else {
            &self.tx2
        };

        let request_id = req_id.unwrap_or_else(|| "submission".to_string());

        let outcome = match result {
            Ok(out) => out,
            Err(reject_reason) => {
                let _ = player_tx
                    .send(ServerMessage::Verdict {
                        req_id: request_id,
                        verdict: VerdictStatus::Rejected,
                        reason: Some(reject_reason),
                        message: "Verifier pool busy, try again later".to_string(),
                        diagnostics: vec![],
                        elapsed_ms: 0,
                        check_skipped: false,
                    })
                    .await;
                return false;
            }
        };

        match outcome.verdict {
            crate::lean::Verdict::Accepted { stdout } => {
                let _ = player_tx
                    .send(ServerMessage::Verdict {
                        req_id: request_id,
                        verdict: VerdictStatus::Accepted,
                        reason: None,
                        message: "Proof accepted".to_string(),
                        diagnostics: vec![],
                        elapsed_ms: outcome.elapsed_ms,
                        check_skipped: false,
                    })
                    .await;

                if !self.solved {
                    self.solved = true;
                    let winner = player_id;
                    let elapsed = Instant::now().duration_since(self.starts_at);

                    // Compute dynamic Elo update
                    let mut p1_r = self.p1_rating;
                    let mut p2_r = self.p2_rating;
                    let a_score = if winner == self.player1 { 1.0 } else { 0.0 };
                    let (delta1, delta2) = self.rating_system.update(&mut p1_r, &mut p2_r, a_score);

                    let p1_delta = delta1;
                    let p2_delta = delta2;

                    let duration_ms = elapsed.as_millis() as i64;

                    // Send RoundEnd to both players
                    let round_end_p1 = ServerMessage::RoundEnd {
                        room_id: self.room_id.to_string(),
                        outcome: MatchOutcome::Won,
                        winner_id: Some(winner.to_string()),
                        winning_proof: Some(stdout.clone()),
                        canonical_proof: None,
                        elo_delta: p1_delta,
                        duration_ms,
                        seq: 1,
                    };
                    let round_end_p2 = ServerMessage::RoundEnd {
                        room_id: self.room_id.to_string(),
                        outcome: MatchOutcome::Won,
                        winner_id: Some(winner.to_string()),
                        winning_proof: Some(stdout),
                        canonical_proof: None,
                        elo_delta: p2_delta,
                        duration_ms,
                        seq: 1,
                    };

                    let _ = self.tx1.send(round_end_p1).await;
                    let _ = self.tx2.send(round_end_p2).await;

                    return true;
                }
            }
            crate::lean::Verdict::Rejected { reason, stderr } => {
                let _ = player_tx
                    .send(ServerMessage::Verdict {
                        req_id: request_id,
                        verdict: VerdictStatus::Rejected,
                        reason: Some(reason),
                        message: stderr.clone(),
                        diagnostics: vec![Diagnostic {
                            line: 1,
                            col: 1,
                            end_line: 1,
                            end_col: 1,
                            severity: DiagnosticSeverity::Error,
                            message: stderr,
                        }],
                        elapsed_ms: outcome.elapsed_ms,
                        check_skipped: false,
                    })
                    .await;
            }
            crate::lean::Verdict::Error(err) => {
                let _ = player_tx
                    .send(ServerMessage::Verdict {
                        req_id: request_id,
                        verdict: VerdictStatus::Error,
                        reason: Some(RejectReason::Internal(err.clone())),
                        message: err.clone(),
                        diagnostics: vec![Diagnostic {
                            line: 1,
                            col: 1,
                            end_line: 1,
                            end_col: 1,
                            severity: DiagnosticSeverity::Error,
                            message: err,
                        }],
                        elapsed_ms: outcome.elapsed_ms,
                        check_skipped: false,
                    })
                    .await;
            }
        }

        false
    }

    async fn handle_forfeit(&mut self, resigner: PlayerId) {
        if self.solved {
            return;
        }
        self.solved = true;
        let winner = if resigner == self.player1 {
            self.player2
        } else {
            self.player1
        };

        let elapsed = Instant::now().duration_since(self.starts_at);
        let duration_ms = elapsed.as_millis() as i64;

        // ADR-010: If forfeit occurred within the first 30 seconds, match is flagged unrated (rated = false, elo_delta = 0)
        let is_unrated = elapsed < Duration::from_secs(30);

        let (p1_delta, p2_delta) = if is_unrated {
            (0, 0)
        } else {
            let mut p1_r = self.p1_rating;
            let mut p2_r = self.p2_rating;
            let a_score = if winner == self.player1 { 1.0 } else { 0.0 };
            self.rating_system.update(&mut p1_r, &mut p2_r, a_score)
        };

        let round_end_p1 = ServerMessage::RoundEnd {
            room_id: self.room_id.to_string(),
            outcome: MatchOutcome::ForfeitWin,
            winner_id: Some(winner.to_string()),
            winning_proof: None,
            canonical_proof: None,
            elo_delta: p1_delta,
            duration_ms,
            seq: 1,
        };

        let round_end_p2 = ServerMessage::RoundEnd {
            room_id: self.room_id.to_string(),
            outcome: MatchOutcome::ForfeitWin,
            winner_id: Some(winner.to_string()),
            winning_proof: None,
            canonical_proof: None,
            elo_delta: p2_delta,
            duration_ms,
            seq: 1,
        };

        let _ = self.tx1.send(round_end_p1).await;
        let _ = self.tx2.send(round_end_p2).await;
    }

    async fn handle_disconnect(&mut self, player_id: PlayerId) {
        if self.solved {
            return;
        }

        let now = Instant::now();
        if player_id == self.player1 && self.p1_disconnected_at.is_none() {
            self.p1_disconnected_at = Some(now);
            tracing::info!(room_id = %self.room_id, player = %player_id, "player 1 disconnected; starting 10s grace window");
        } else if player_id == self.player2 && self.p2_disconnected_at.is_none() {
            self.p2_disconnected_at = Some(now);
            tracing::info!(room_id = %self.room_id, player = %player_id, "player 2 disconnected; starting 10s grace window");
        }
    }

    async fn handle_deadline_expired(&mut self) {
        self.solved = true;
        let duration_ms = self.duration.as_millis() as i64;

        // Draw update in Elo
        let mut p1_r = self.p1_rating;
        let mut p2_r = self.p2_rating;
        let (delta1, delta2) = self.rating_system.update(&mut p1_r, &mut p2_r, 0.5);

        let round_end_p1 = ServerMessage::RoundEnd {
            room_id: self.room_id.to_string(),
            outcome: MatchOutcome::Draw,
            winner_id: None,
            winning_proof: None,
            canonical_proof: None,
            elo_delta: delta1,
            duration_ms,
            seq: 1,
        };

        let round_end_p2 = ServerMessage::RoundEnd {
            room_id: self.room_id.to_string(),
            outcome: MatchOutcome::Draw,
            winner_id: None,
            winning_proof: None,
            canonical_proof: None,
            elo_delta: delta2,
            duration_ms,
            seq: 1,
        };

        let _ = self.tx1.send(round_end_p1).await;
        let _ = self.tx2.send(round_end_p2).await;
    }
}
