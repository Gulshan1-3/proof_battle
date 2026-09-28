use super::PlayerId;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    pub elo: i32,
    pub games_played: u32,
}

impl Default for Rating {
    fn default() -> Self {
        Self {
            elo: 1200,
            games_played: 0,
        }
    }
}

/// RatingSystem trait provides an abstraction over rating algorithms.
/// Glicko-2 is a drop-in replacement that implements this trait using Rating Deviation (RD)
/// and volatility (sigma), columns for which are already provisioned in the `players` table.
pub trait RatingSystem: Send + Sync {
    fn expected(&self, a: f64, b: f64) -> f64;
    fn update(&self, a: &mut Rating, b: &mut Rating, a_score: f64) -> (i32, i32);
}

pub struct Elo;

impl Elo {
    pub fn new() -> Self {
        Self
    }

    /// K-factor is a function of experience (games played):
    /// - < 10 games: K = 40 (provisional calibration)
    /// - 10..=30 games: K = 32 (intermediate stabilization)
    /// - > 30 games: K = 24 (established rating)
    pub fn k_factor(games_played: u32) -> f64 {
        if games_played < 10 {
            40.0
        } else if games_played <= 30 {
            32.0
        } else {
            24.0
        }
    }
}

impl Default for Elo {
    fn default() -> Self {
        Self::new()
    }
}

impl RatingSystem for Elo {
    fn expected(&self, a: f64, b: f64) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf((b - a) / 400.0))
    }

    fn update(&self, a: &mut Rating, b: &mut Rating, a_score: f64) -> (i32, i32) {
        let b_score = 1.0 - a_score;
        let expected_a = self.expected(a.elo as f64, b.elo as f64);
        let expected_b = self.expected(b.elo as f64, a.elo as f64);

        let k_a = Self::k_factor(a.games_played);
        let k_b = Self::k_factor(b.games_played);

        let delta_a = (k_a * (a_score - expected_a)).round() as i32;
        let delta_b = (k_b * (b_score - expected_b)).round() as i32;

        a.elo = (a.elo + delta_a).max(100);
        b.elo = (b.elo + delta_b).max(100);

        a.games_played += 1;
        b.games_played += 1;

        (delta_a, delta_b)
    }
}

#[derive(Debug, Clone)]
pub struct MatchResultRecord {
    pub match_id: Uuid,
    pub player1_id: PlayerId,
    pub player2_id: PlayerId,
    pub winner_id: Option<PlayerId>,
    pub outcome_name: String,
    pub is_forfeit: bool,
    pub duration_ms: i64,
    pub problem_id: Option<Uuid>,
    pub rated: bool,
    pub elo_delta_p1: i32,
    pub elo_delta_p2: i32,
}

/// Persists match outcome and player rating updates atomically in ONE PostgreSQL transaction.
/// Invariant: If database persistence fails, an error is logged, but the caller STILL delivers
/// the match result to the players (game outcomes are never held hostage by DB glitches).
pub async fn record_match_history_atomic(
    pool: &PgPool,
    result: &MatchResultRecord,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let now = Utc::now();

    // 1. Upsert Player 1
    let p1_win = if result.winner_id == Some(result.player1_id) {
        1
    } else {
        0
    };
    let p1_loss = if result.winner_id == Some(result.player2_id) {
        1
    } else {
        0
    };
    let p1_draw = if result.winner_id.is_none() { 1 } else { 0 };

    sqlx::query(
        r#"
        INSERT INTO players (id, rating, games_played, wins, losses, draws, created_at, updated_at)
        VALUES ($1, 1200 + $2, 1, $3, $4, $5, $6, $6)
        ON CONFLICT (id) DO UPDATE SET
            rating = players.rating + EXCLUDED.rating - 1200,
            games_played = players.games_played + 1,
            wins = players.wins + EXCLUDED.wins,
            losses = players.losses + EXCLUDED.losses,
            draws = players.draws + EXCLUDED.draws,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(result.player1_id.0)
    .bind(result.elo_delta_p1)
    .bind(p1_win)
    .bind(p1_loss)
    .bind(p1_draw)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 2. Upsert Player 2
    let p2_win = if result.winner_id == Some(result.player2_id) {
        1
    } else {
        0
    };
    let p2_loss = if result.winner_id == Some(result.player1_id) {
        1
    } else {
        0
    };
    let p2_draw = if result.winner_id.is_none() { 1 } else { 0 };

    sqlx::query(
        r#"
        INSERT INTO players (id, rating, games_played, wins, losses, draws, created_at, updated_at)
        VALUES ($1, 1200 + $2, 1, $3, $4, $5, $6, $6)
        ON CONFLICT (id) DO UPDATE SET
            rating = players.rating + EXCLUDED.rating - 1200,
            games_played = players.games_played + 1,
            wins = players.wins + EXCLUDED.wins,
            losses = players.losses + EXCLUDED.losses,
            draws = players.draws + EXCLUDED.draws,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(result.player2_id.0)
    .bind(result.elo_delta_p2)
    .bind(p2_win)
    .bind(p2_loss)
    .bind(p2_draw)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 3. Insert into match_history
    sqlx::query(
        r#"
        INSERT INTO match_history (
            id, player1_id, player2_id, winner_id, problem_id, outcome,
            rated, elo_delta_p1, elo_delta_p2, duration_ms, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
        )
        "#,
    )
    .bind(result.match_id)
    .bind(result.player1_id.0)
    .bind(result.player2_id.0)
    .bind(result.winner_id.map(|p| p.0))
    .bind(result.problem_id)
    .bind(&result.outcome_name)
    .bind(result.rated)
    .bind(result.elo_delta_p1)
    .bind(result.elo_delta_p2)
    .bind(result.duration_ms)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elo_1200_vs_1200_win() {
        let elo = Elo::new();
        let mut p1 = Rating {
            elo: 1200,
            games_played: 15, // K = 32
        };
        let mut p2 = Rating {
            elo: 1200,
            games_played: 15, // K = 32
        };

        let (d1, d2) = elo.update(&mut p1, &mut p2, 1.0);
        // Expected score is 0.5, delta is 32 * (1.0 - 0.5) = +16 for winner, -16 for loser
        assert_eq!(d1, 16);
        assert_eq!(d2, -16);
        assert_eq!(p1.elo, 1216);
        assert_eq!(p2.elo, 1184);
    }

    #[test]
    fn test_elo_800_vs_1600_upset() {
        let elo = Elo::new();
        let mut underdog = Rating {
            elo: 800,
            games_played: 35, // K = 24
        };
        let mut favorite = Rating {
            elo: 1600,
            games_played: 35, // K = 24
        };

        let (d1, d2) = elo.update(&mut underdog, &mut favorite, 1.0);
        // 800 expected against 1600 is ~0.0099
        // delta underdog = 24 * (1.0 - 0.0099) ≈ +24
        // delta favorite = 24 * (0.0 - 0.9901) ≈ -24
        assert!((23..=24).contains(&d1));
        assert!((-24..=-23).contains(&d2));
    }

    #[test]
    fn test_k_factor_tiers() {
        assert_eq!(Elo::k_factor(5), 40.0);
        assert_eq!(Elo::k_factor(10), 32.0);
        assert_eq!(Elo::k_factor(30), 32.0);
        assert_eq!(Elo::k_factor(31), 24.0);
    }
}
