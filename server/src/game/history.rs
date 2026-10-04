use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRecord {
    pub id: Uuid,
    pub room_id: Uuid,
    pub player1_id: Uuid,
    pub player1_username: Option<String>,
    pub player1_elo_before: i32,
    pub player1_elo_after: i32,
    pub elo_delta_p1: i32,
    pub player2_id: Uuid,
    pub player2_username: Option<String>,
    pub player2_elo_before: i32,
    pub player2_elo_after: i32,
    pub elo_delta_p2: i32,
    pub problem_id: Option<Uuid>,
    pub problem_goal: String,
    pub problem_category: String,
    pub problem_difficulty: u8,
    pub winner_id: Option<Uuid>,
    pub outcome: String,
    pub winning_proof: Option<String>,
    pub canonical_proof: Option<String>,
    pub rated: bool,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSummary {
    pub id: Uuid,
    pub room_id: Uuid,
    pub opponent_id: Uuid,
    pub opponent_username: Option<String>,
    pub opponent_elo: i32,
    pub outcome: String,
    pub elo_delta: i32,
    pub problem_goal: String,
    pub problem_category: String,
    pub problem_difficulty: u8,
    pub rated: bool,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub games_played: u32,
    pub wins: u32,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub player_id: Uuid,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub win_rate: f64,
    pub current_rating: i32,
    pub categories: Vec<CategoryStats>,
}

#[derive(Clone)]
pub struct HistoryStore {
    pool: Option<sqlx::PgPool>,
    in_memory: Arc<RwLock<VecDeque<MatchRecord>>>,
    max_in_memory: usize,
}

impl HistoryStore {
    pub fn new(pool: Option<sqlx::PgPool>) -> Self {
        Self {
            pool,
            in_memory: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_in_memory: 1000,
        }
    }

    pub async fn record_match(&self, record: MatchRecord) {
        // 1. Always record in memory ring-buffer
        {
            let mut mem = self.in_memory.write().await;
            if mem.len() >= self.max_in_memory {
                mem.pop_back();
            }
            mem.push_front(record.clone());
        }

        // 2. If Postgres is connected, persist to match_history table
        if let Some(pool) = &self.pool {
            let res = sqlx::query(
                r#"
                INSERT INTO match_history (
                    id, room_id, player1_id, player2_id, winner_id, problem_id, outcome,
                    p1_elo_before, p1_elo_after, p2_elo_before, p2_elo_after,
                    elo_delta_p1, elo_delta_p2, rated, duration_ms, created_at,
                    winning_proof, canonical_proof, player1_username, player2_username,
                    problem_goal, problem_category, problem_difficulty
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7,
                    $8, $9, $10, $11,
                    $12, $13, $14, $15, $16,
                    $17, $18, $19, $20,
                    $21, $22, $23
                )
                "#,
            )
            .bind(record.id)
            .bind(record.room_id)
            .bind(record.player1_id)
            .bind(record.player2_id)
            .bind(record.winner_id)
            .bind(record.problem_id)
            .bind(&record.outcome)
            .bind(record.player1_elo_before)
            .bind(record.player1_elo_after)
            .bind(record.player2_elo_before)
            .bind(record.player2_elo_after)
            .bind(record.elo_delta_p1)
            .bind(record.elo_delta_p2)
            .bind(record.rated)
            .bind(record.duration_ms)
            .bind(record.created_at)
            .bind(&record.winning_proof)
            .bind(&record.canonical_proof)
            .bind(&record.player1_username)
            .bind(&record.player2_username)
            .bind(&record.problem_goal)
            .bind(&record.problem_category)
            .bind(record.problem_difficulty as i16)
            .execute(pool)
            .await;

            if let Err(e) = res {
                tracing::warn!("Failed to persist match history to PostgreSQL: {e}");
            }
        }
    }

    pub async fn get_player_history(&self, player_id: Uuid, limit: usize) -> Vec<MatchSummary> {
        let lim = limit.clamp(1, 100);

        if let Some(pool) = &self.pool {
            let rows = sqlx::query_as::<_, DbMatchRow>(
                r#"
                SELECT id, room_id, player1_id, player1_username, player1_elo_before, player1_elo_after, elo_delta_p1,
                       player2_id, player2_username, player2_elo_before, player2_elo_after, elo_delta_p2,
                       problem_id, problem_goal, problem_category, problem_difficulty,
                       winner_id, outcome, rated, duration_ms, created_at, winning_proof, canonical_proof
                FROM match_history
                WHERE player1_id = $1 OR player2_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(player_id)
            .bind(lim as i64)
            .fetch_all(pool)
            .await;

            if let Ok(records) = rows {
                let summaries: Vec<_> = records
                    .into_iter()
                    .map(|r| r.to_summary(player_id))
                    .collect();
                if !summaries.is_empty() {
                    return summaries;
                }
            }
        }

        // Fallback to in-memory store
        let mem = self.in_memory.read().await;
        mem.iter()
            .filter(|r| r.player1_id == player_id || r.player2_id == player_id)
            .take(lim)
            .map(|r| summarize_record(r, player_id))
            .collect()
    }

    pub async fn get_match_detail(&self, match_id: Uuid) -> Option<MatchRecord> {
        if let Some(pool) = &self.pool {
            let row = sqlx::query_as::<_, DbMatchRow>(
                r#"
                SELECT id, room_id, player1_id, player1_username, player1_elo_before, player1_elo_after, elo_delta_p1,
                       player2_id, player2_username, player2_elo_before, player2_elo_after, elo_delta_p2,
                       problem_id, problem_goal, problem_category, problem_difficulty,
                       winner_id, outcome, rated, duration_ms, created_at, winning_proof, canonical_proof
                FROM match_history
                WHERE id = $1
                "#,
            )
            .bind(match_id)
            .fetch_optional(pool)
            .await;

            if let Ok(Some(r)) = row {
                return Some(r.into_record());
            }
        }

        let mem = self.in_memory.read().await;
        mem.iter().find(|r| r.id == match_id).cloned()
    }

    pub async fn get_player_stats(&self, player_id: Uuid) -> PlayerStats {
        let history = self.get_player_history(player_id, 100).await;

        let total = history.len() as u32;
        let mut wins = 0;
        let mut losses = 0;
        let mut draws = 0;
        let mut category_map: HashMap<String, (u32, u32)> = HashMap::new();

        let current_rating = if let Some(first) = history.first() {
            // First item is most recent match
            if first.rated {
                1200 + first.elo_delta
            } else {
                1200
            }
        } else {
            1200
        };

        for m in &history {
            let cat_entry = category_map
                .entry(m.problem_category.clone())
                .or_insert((0, 0));
            cat_entry.0 += 1;

            if m.outcome == "Won" {
                wins += 1;
                cat_entry.1 += 1;
            } else if m.outcome == "Lost" {
                losses += 1;
            } else {
                draws += 1;
            }
        }

        let win_rate = if total > 0 {
            (wins as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let mut categories = Vec::new();
        for (category, (cat_total, cat_wins)) in category_map {
            let cat_win_rate = if cat_total > 0 {
                (cat_wins as f64 / cat_total as f64) * 100.0
            } else {
                0.0
            };
            categories.push(CategoryStats {
                category,
                games_played: cat_total,
                wins: cat_wins,
                win_rate: cat_win_rate,
            });
        }
        categories.sort_by(|a, b| b.games_played.cmp(&a.games_played));

        PlayerStats {
            player_id,
            games_played: total,
            wins,
            losses,
            draws,
            win_rate,
            current_rating,
            categories,
        }
    }
}

fn summarize_record(record: &MatchRecord, player_id: Uuid) -> MatchSummary {
    let is_p1 = record.player1_id == player_id;

    let (opponent_id, opponent_username, opponent_elo, elo_delta) = if is_p1 {
        (
            record.player2_id,
            record.player2_username.clone(),
            record.player2_elo_after,
            record.elo_delta_p1,
        )
    } else {
        (
            record.player1_id,
            record.player1_username.clone(),
            record.player1_elo_after,
            record.elo_delta_p2,
        )
    };

    let outcome = match record.winner_id {
        Some(w) if w == player_id => "Won".to_string(),
        Some(_) => "Lost".to_string(),
        None => "Draw".to_string(),
    };

    MatchSummary {
        id: record.id,
        room_id: record.room_id,
        opponent_id,
        opponent_username,
        opponent_elo,
        outcome,
        elo_delta,
        problem_goal: record.problem_goal.clone(),
        problem_category: record.problem_category.clone(),
        problem_difficulty: record.problem_difficulty,
        rated: record.rated,
        duration_ms: record.duration_ms,
        created_at: record.created_at,
    }
}

#[derive(sqlx::FromRow)]
struct DbMatchRow {
    id: Uuid,
    room_id: Uuid,
    player1_id: Uuid,
    player1_username: Option<String>,
    player1_elo_before: i32,
    player1_elo_after: i32,
    elo_delta_p1: i32,
    player2_id: Uuid,
    player2_username: Option<String>,
    player2_elo_before: i32,
    player2_elo_after: i32,
    elo_delta_p2: i32,
    problem_id: Option<Uuid>,
    problem_goal: String,
    problem_category: String,
    problem_difficulty: i16,
    winner_id: Option<Uuid>,
    outcome: String,
    rated: bool,
    duration_ms: i64,
    created_at: DateTime<Utc>,
    winning_proof: Option<String>,
    canonical_proof: Option<String>,
}

impl DbMatchRow {
    fn to_summary(&self, player_id: Uuid) -> MatchSummary {
        let is_p1 = self.player1_id == player_id;
        let (opponent_id, opponent_username, opponent_elo, elo_delta) = if is_p1 {
            (
                self.player2_id,
                self.player2_username.clone(),
                self.player2_elo_after,
                self.elo_delta_p1,
            )
        } else {
            (
                self.player1_id,
                self.player1_username.clone(),
                self.player1_elo_after,
                self.elo_delta_p2,
            )
        };

        let outcome = match self.winner_id {
            Some(w) if w == player_id => "Won".to_string(),
            Some(_) => "Lost".to_string(),
            None => "Draw".to_string(),
        };

        MatchSummary {
            id: self.id,
            room_id: self.room_id,
            opponent_id,
            opponent_username,
            opponent_elo,
            outcome,
            elo_delta,
            problem_goal: self.problem_goal.clone(),
            problem_category: self.problem_category.clone(),
            problem_difficulty: self.problem_difficulty as u8,
            rated: self.rated,
            duration_ms: self.duration_ms,
            created_at: self.created_at,
        }
    }

    fn into_record(self) -> MatchRecord {
        MatchRecord {
            id: self.id,
            room_id: self.room_id,
            player1_id: self.player1_id,
            player1_username: self.player1_username,
            player1_elo_before: self.player1_elo_before,
            player1_elo_after: self.player1_elo_after,
            elo_delta_p1: self.elo_delta_p1,
            player2_id: self.player2_id,
            player2_username: self.player2_username,
            player2_elo_before: self.player2_elo_before,
            player2_elo_after: self.player2_elo_after,
            elo_delta_p2: self.elo_delta_p2,
            problem_id: self.problem_id,
            problem_goal: self.problem_goal,
            problem_category: self.problem_category,
            problem_difficulty: self.problem_difficulty as u8,
            winner_id: self.winner_id,
            outcome: self.outcome,
            winning_proof: self.winning_proof,
            canonical_proof: self.canonical_proof,
            rated: self.rated,
            duration_ms: self.duration_ms,
            created_at: self.created_at,
        }
    }
}
