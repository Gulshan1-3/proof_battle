use proof_battle_server::db::problems::{Bracket, DbError, PostgresProblemSource, ProblemSource};
use proof_battle_server::db::{create_pool, run_migrations};
use sqlx::PgPool;
use uuid::Uuid;

fn test_db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://proofbattle:proofbattle_dev@localhost:5432/proofbattle".to_string()
    })
}

async fn get_test_pool() -> Option<PgPool> {
    let url = test_db_url();
    match create_pool(&url).await {
        Ok(pool) => {
            let _ = run_migrations(&pool).await;
            Some(pool)
        }
        Err(_) => None,
    }
}

#[test]
fn test_difficulty_bracket_calibration_and_boundaries() {
    // 0 - 599 -> difficulty 1 - 2
    assert_eq!(Bracket::for_elo(0).target_difficulty_range(), (1, 2));
    assert_eq!(Bracket::for_elo(350).target_difficulty_range(), (1, 2));
    assert_eq!(Bracket::for_elo(599).target_difficulty_range(), (1, 2));

    // 600 - 999 -> difficulty 3 - 4
    assert_eq!(Bracket::for_elo(600).target_difficulty_range(), (3, 4));
    assert_eq!(Bracket::for_elo(800).target_difficulty_range(), (3, 4));
    assert_eq!(Bracket::for_elo(999).target_difficulty_range(), (3, 4));

    // 1000 - 1399 -> difficulty 5 - 6
    assert_eq!(Bracket::for_elo(1000).target_difficulty_range(), (5, 6));
    assert_eq!(Bracket::for_elo(1200).target_difficulty_range(), (5, 6));
    assert_eq!(Bracket::for_elo(1399).target_difficulty_range(), (5, 6));

    // 1400+ -> difficulty 7 - 10
    assert_eq!(Bracket::for_elo(1400).target_difficulty_range(), (7, 10));
    assert_eq!(Bracket::for_elo(1600).target_difficulty_range(), (7, 10));
    assert_eq!(Bracket::for_elo(2400).target_difficulty_range(), (7, 10));
}

#[test]
fn test_bracket_widening() {
    let b = Bracket::for_elo(800); // 600 - 1000 -> (3, 4)
    let widened = b.widen(150); // 450 - 1150
    assert_eq!(widened.min_elo, 450);
    assert_eq!(widened.max_elo, 1150);
    // Min 450 maps to diff 1, Max 1150 maps to diff 6
    assert_eq!(widened.target_difficulty_range(), (1, 6));
}

#[tokio::test]
async fn test_postgres_problem_source_selection_by_bracket() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Skipping PostgreSQL test: database not available");
            return;
        }
    };

    let source = PostgresProblemSource::new(pool);
    let total_count = source.count().await.expect("Failed to count problems");
    assert!(
        total_count >= 120,
        "Expected at least 120 verified problems in DB, found {}",
        total_count
    );

    // Test bracket for ELO 400 (diff 1 - 2)
    let p_low = source
        .pick(Bracket::for_elo(400))
        .await
        .expect("Pick for 400 ELO failed");
    assert!(
        p_low.difficulty >= 1 && p_low.difficulty <= 2,
        "Expected difficulty 1-2, got {}",
        p_low.difficulty
    );
    assert!(p_low.verified, "Picked problem must be verified");

    // Test bracket for ELO 800 (diff 3 - 4)
    let p_mid = source
        .pick(Bracket::for_elo(800))
        .await
        .expect("Pick for 800 ELO failed");
    assert!(
        p_mid.difficulty >= 3 && p_mid.difficulty <= 4,
        "Expected difficulty 3-4, got {}",
        p_mid.difficulty
    );
    assert!(p_mid.verified, "Picked problem must be verified");

    // Test bracket for ELO 1200 (diff 5 - 6)
    let p_high = source
        .pick(Bracket::for_elo(1200))
        .await
        .expect("Pick for 1200 ELO failed");
    assert!(
        p_high.difficulty >= 5 && p_high.difficulty <= 6,
        "Expected difficulty 5-6, got {}",
        p_high.difficulty
    );
    assert!(p_high.verified, "Picked problem must be verified");

    // Test bracket for ELO 1600 (diff 7 - 10)
    let p_expert = source
        .pick(Bracket::for_elo(1600))
        .await
        .expect("Pick for 1600 ELO failed");
    assert!(
        p_expert.difficulty >= 7 && p_expert.difficulty <= 10,
        "Expected difficulty 7-10, got {}",
        p_expert.difficulty
    );
    assert!(p_expert.verified, "Picked problem must be verified");
}

#[tokio::test]
async fn test_postgres_anti_repeat_and_stats_update() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => return,
    };

    let source = PostgresProblemSource::new(pool.clone());
    let picked = source
        .pick(Bracket::for_elo(400))
        .await
        .expect("Failed to pick problem");

    // Check that times_played was incremented and last_played_at was set
    let updated = source
        .by_id(picked.id)
        .await
        .expect("Failed to fetch picked problem");
    assert!(
        updated.times_played > 0,
        "times_played should have been incremented"
    );
    assert!(
        updated.last_played_at.is_some(),
        "last_played_at should have been updated"
    );
}

#[tokio::test]
async fn test_unverified_problems_never_returned() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => return,
    };

    // Insert a dummy unverified problem with difficulty 10
    let fake_slug = format!("test/unverified_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO problems (
            slug, goal, imports, statement, canonical_proof, difficulty, category, verified
        ) VALUES (
            $1, 'False', ARRAY[]::TEXT[], 'theorem goal : False := by', 'sorry', 10, 'test', false
        )
        "#,
    )
    .bind(&fake_slug)
    .execute(&pool)
    .await
    .expect("Failed to insert fake unverified problem");

    let source = PostgresProblemSource::new(pool.clone());

    // Bracket with difficulty 99 should NOT return this unverified problem
    let custom_bracket = Bracket {
        min_elo: 5000,
        max_elo: 6000,
    };
    // Should fallback to verified problems, never the unverified one
    let picked = source
        .pick(custom_bracket)
        .await
        .expect("Fallback should pick a verified problem");
    assert!(
        picked.slug != fake_slug,
        "Unverified problem must NEVER be selected"
    );
    assert!(picked.verified, "Selected problem must be verified");

    // Clean up
    let _ = sqlx::query("DELETE FROM problems WHERE slug = $1")
        .bind(&fake_slug)
        .execute(&pool)
        .await;
}

#[tokio::test]
async fn test_by_id_and_not_found() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => return,
    };

    let source = PostgresProblemSource::new(pool);
    let non_existent_id = Uuid::new_v4();
    let err = source.by_id(non_existent_id).await.unwrap_err();
    assert!(
        matches!(err, DbError::NotFound(_)),
        "Expected NotFound error, got {:?}",
        err
    );
}
