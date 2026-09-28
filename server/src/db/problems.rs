use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum DbError {
    Sqlx(sqlx::Error),
    NotFound(String),
    NoProblemAvailable,
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlx(e) => write!(f, "Database error: {e}"),
            Self::NotFound(s) => write!(f, "Problem not found: {s}"),
            Self::NoProblemAvailable => write!(f, "No verified problems available"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ProblemRecord {
    pub id: Uuid,
    pub slug: String,
    pub goal: String,
    pub imports: Vec<String>,
    pub statement: String,
    pub canonical_proof: String,
    pub difficulty: i16,
    pub category: String,
    pub tactic_hint: Option<String>,
    pub source_theorem: Option<String>,
    pub source_url: Option<String>,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_error: Option<String>,
    pub times_played: i32,
    pub last_played_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bracket {
    pub min_elo: i32,
    pub max_elo: i32,
}

impl Bracket {
    /// Maps ELO to difficulty bracket per calibration curve:
    /// 1-2 -> 0-600 | 3-4 -> 600-1000 | 5-6 -> 1000-1400 | 7-10 -> 1400+
    pub fn for_elo(elo: i32) -> Self {
        if elo < 600 {
            Bracket {
                min_elo: 0,
                max_elo: 600,
            }
        } else if elo < 1000 {
            Bracket {
                min_elo: 600,
                max_elo: 1000,
            }
        } else if elo < 1400 {
            Bracket {
                min_elo: 1000,
                max_elo: 1400,
            }
        } else {
            Bracket {
                min_elo: 1400,
                max_elo: 3000,
            }
        }
    }

    pub fn target_difficulty_range(&self) -> (i16, i16) {
        let min_d = if self.min_elo < 600 {
            1
        } else if self.min_elo < 1000 {
            3
        } else if self.min_elo < 1400 {
            5
        } else {
            7
        };
        let max_d = if self.max_elo <= 600 {
            2
        } else if self.max_elo <= 1000 {
            4
        } else if self.max_elo <= 1400 {
            6
        } else {
            10
        };
        (min_d, max_d)
    }

    pub fn widen(&self, elo_delta: i32) -> Self {
        Self {
            min_elo: (self.min_elo - elo_delta).max(0),
            max_elo: self.max_elo + elo_delta,
        }
    }
}

#[async_trait]
pub trait ProblemSource: Send + Sync {
    async fn pick(&self, bracket: Bracket) -> Result<ProblemRecord, DbError>;
    async fn by_id(&self, id: Uuid) -> Result<ProblemRecord, DbError>;
    async fn count(&self) -> Result<i64, DbError>;
}

pub struct InMemoryProblemSource {
    problems: Vec<ProblemRecord>,
}

impl InMemoryProblemSource {
    pub fn new() -> Self {
        let now = Utc::now();
        let problems = vec![
            ProblemRecord {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                slug: "nat/add_zero".to_string(),
                goal: "∀ n : ℕ, n + 0 = n".to_string(),
                imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
                statement: "theorem goal : ∀ n : ℕ, n + 0 = n := by".to_string(),
                canonical_proof: "intro n\nsimp".to_string(),
                difficulty: 1,
                category: "nat_arithmetic".to_string(),
                tactic_hint: Some("try simp".to_string()),
                source_theorem: Some("Nat.add_zero".to_string()),
                source_url: None,
                verified: true,
                verified_at: Some(now),
                verification_error: None,
                times_played: 0,
                last_played_at: None,
                created_at: now,
            },
            ProblemRecord {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                slug: "nat/add_comm".to_string(),
                goal: "∀ a b : ℕ, a + b = b + a".to_string(),
                imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
                statement: "theorem goal : ∀ a b : ℕ, a + b = b + a := by".to_string(),
                canonical_proof: "intro a b\nomega".to_string(),
                difficulty: 2,
                category: "nat_arithmetic".to_string(),
                tactic_hint: Some("try omega".to_string()),
                source_theorem: Some("Nat.add_comm".to_string()),
                source_url: None,
                verified: true,
                verified_at: Some(now),
                verification_error: None,
                times_played: 0,
                last_played_at: None,
                created_at: now,
            },
            ProblemRecord {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
                slug: "logic/and_comm".to_string(),
                goal: "∀ P Q : Prop, P ∧ Q → Q ∧ P".to_string(),
                imports: vec!["import Mathlib.Logic.Basic".to_string()],
                statement: "theorem goal : ∀ P Q : Prop, P ∧ Q → Q ∧ P := by".to_string(),
                canonical_proof: "intro P Q ⟨hp, hq⟩\nexact ⟨hq, hp⟩".to_string(),
                difficulty: 3,
                category: "logic".to_string(),
                tactic_hint: Some("intro and construct".to_string()),
                source_theorem: Some("and_comm".to_string()),
                source_url: None,
                verified: true,
                verified_at: Some(now),
                verification_error: None,
                times_played: 0,
                last_played_at: None,
                created_at: now,
            },
        ];

        Self { problems }
    }
}

impl Default for InMemoryProblemSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProblemSource for InMemoryProblemSource {
    async fn pick(&self, bracket: Bracket) -> Result<ProblemRecord, DbError> {
        let (min_d, max_d) = bracket.target_difficulty_range();
        let matched = self
            .problems
            .iter()
            .find(|p| p.difficulty >= min_d && p.difficulty <= max_d);

        if let Some(p) = matched {
            return Ok(p.clone());
        }

        // Fallback to first verified problem
        self.problems
            .first()
            .cloned()
            .ok_or(DbError::NoProblemAvailable)
    }

    async fn by_id(&self, id: Uuid) -> Result<ProblemRecord, DbError> {
        self.problems
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(id.to_string()))
    }

    async fn count(&self) -> Result<i64, DbError> {
        Ok(self.problems.len() as i64)
    }
}

pub struct PostgresProblemSource {
    pool: PgPool,
}

impl PostgresProblemSource {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProblemSource for PostgresProblemSource {
    async fn pick(&self, bracket: Bracket) -> Result<ProblemRecord, DbError> {
        let (min_d, max_d) = bracket.target_difficulty_range();

        // 1. Target difficulty bracket with anti-repeat (not played in last 2 minutes)
        let row = sqlx::query_as::<_, ProblemRecord>(
            r#"
            SELECT * FROM problems
            WHERE verified = true
              AND difficulty BETWEEN $1 AND $2
              AND (last_played_at IS NULL OR last_played_at < NOW() - INTERVAL '2 minutes')
            ORDER BY RANDOM()
            LIMIT 1
            "#,
        )
        .bind(min_d)
        .bind(max_d)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = row {
            self.mark_played(record.id).await;
            return Ok(record);
        }

        tracing::info!(bracket = ?bracket, "Target difficulty bracket empty; widening by +/-150 ELO");

        // 2. Widened bracket (+/- 150 ELO)
        let widened = bracket.widen(150);
        let (w_min, w_max) = widened.target_difficulty_range();
        let row_widened = sqlx::query_as::<_, ProblemRecord>(
            r#"
            SELECT * FROM problems
            WHERE verified = true
              AND difficulty BETWEEN $1 AND $2
              AND (last_played_at IS NULL OR last_played_at < NOW() - INTERVAL '2 minutes')
            ORDER BY RANDOM()
            LIMIT 1
            "#,
        )
        .bind(w_min)
        .bind(w_max)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = row_widened {
            self.mark_played(record.id).await;
            return Ok(record);
        }

        tracing::info!(bracket = ?bracket, "Widened bracket empty; relaxing anti-repeat constraint");

        // 3. Relax anti-repeat constraint on target difficulty
        let row_relaxed = sqlx::query_as::<_, ProblemRecord>(
            r#"
            SELECT * FROM problems
            WHERE verified = true
              AND difficulty BETWEEN $1 AND $2
            ORDER BY RANDOM()
            LIMIT 1
            "#,
        )
        .bind(min_d)
        .bind(max_d)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = row_relaxed {
            self.mark_played(record.id).await;
            return Ok(record);
        }

        tracing::warn!(bracket = ?bracket, "Relaxed bracket empty; falling back to nearest verified problem");

        // 4. Fallback to nearest difficulty
        let target_avg = (min_d + max_d) / 2;
        let row_nearest = sqlx::query_as::<_, ProblemRecord>(
            r#"
            SELECT * FROM problems
            WHERE verified = true
            ORDER BY ABS(difficulty - $1) ASC, RANDOM()
            LIMIT 1
            "#,
        )
        .bind(target_avg)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = row_nearest {
            self.mark_played(record.id).await;
            return Ok(record);
        }

        Err(DbError::NoProblemAvailable)
    }

    async fn by_id(&self, id: Uuid) -> Result<ProblemRecord, DbError> {
        let record = sqlx::query_as::<_, ProblemRecord>(
            r#"
            SELECT * FROM problems WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))?;

        Ok(record)
    }

    async fn count(&self) -> Result<i64, DbError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM problems WHERE verified = true
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}

impl PostgresProblemSource {
    async fn mark_played(&self, id: Uuid) {
        let _ = sqlx::query(
            r#"
            UPDATE problems
            SET times_played = times_played + 1,
                last_played_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_bracket_calibration_and_boundaries() {
        // 1-2 -> 0-600
        assert_eq!(Bracket::for_elo(0).target_difficulty_range(), (1, 2));
        assert_eq!(Bracket::for_elo(599).target_difficulty_range(), (1, 2));

        // Boundary at 600 -> 3-4
        assert_eq!(Bracket::for_elo(600).target_difficulty_range(), (3, 4));
        assert_eq!(Bracket::for_elo(601).target_difficulty_range(), (3, 4));
        assert_eq!(Bracket::for_elo(999).target_difficulty_range(), (3, 4));

        // Boundary at 1000 -> 5-6
        assert_eq!(Bracket::for_elo(1000).target_difficulty_range(), (5, 6));
        assert_eq!(Bracket::for_elo(1001).target_difficulty_range(), (5, 6));
        assert_eq!(Bracket::for_elo(1399).target_difficulty_range(), (5, 6));

        // Boundary at 1400 -> 7-10
        assert_eq!(Bracket::for_elo(1400).target_difficulty_range(), (7, 10));
        assert_eq!(Bracket::for_elo(1401).target_difficulty_range(), (7, 10));
        assert_eq!(Bracket::for_elo(2500).target_difficulty_range(), (7, 10));
    }

    #[tokio::test]
    async fn test_in_memory_problem_source_returns_current_three_problems() {
        let source = InMemoryProblemSource::new();
        assert_eq!(source.count().await.unwrap(), 3);

        let p1 = source.pick(Bracket::for_elo(400)).await.unwrap();
        assert_eq!(p1.difficulty, 1);
        assert_eq!(p1.slug, "nat/add_zero");

        let p2 = source.pick(Bracket::for_elo(800)).await.unwrap();
        assert_eq!(p2.difficulty, 3);
        assert_eq!(p2.slug, "logic/and_comm");
    }
}
