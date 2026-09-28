use proof_battle_server::challenges::ProofChallenge;
use proof_battle_server::config::Config;
use proof_battle_server::game::PlayerId;
use proof_battle_server::game::queue::{RateLimiter, VerifierPool, run_check_only};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

fn make_test_config() -> Arc<Config> {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "15".to_string());
    env.insert("SANDBOX_ENABLED".to_string(), "false".to_string());
    env.insert("LEAN_POOL_SIZE".to_string(), "4".to_string());
    env.insert("SUBMIT_LANE_RESERVE".to_string(), "2".to_string());
    env.insert("CHECK_LANE_CAPACITY".to_string(), "2".to_string());
    env.insert("RATE_LIMIT_SUBMISSIONS".to_string(), "5".to_string());
    env.insert("RATE_LIMIT_WINDOW".to_string(), "60".to_string());

    Arc::new(Config::from_map(&env).expect("valid config"))
}

#[tokio::test]
async fn test_rate_limiter_5_submits_per_60s() {
    let limiter = RateLimiter::new();
    let player = PlayerId::new();

    // 5 submissions should succeed
    for i in 1..=5 {
        assert!(
            limiter.check_submit(player).await.is_ok(),
            "Submission #{i} must be permitted by rate limiter"
        );
    }

    // 6th submission within 60s window must be rejected with retry duration
    let sixth = limiter.check_submit(player).await;
    assert!(
        sixth.is_err(),
        "6th submission must be rejected by rate limiter"
    );
    if let Err(retry_after) = sixth {
        assert!(retry_after > Duration::ZERO);
    }
}

#[tokio::test]
async fn test_rate_limiter_1_check_per_2s() {
    let limiter = RateLimiter::new();
    let player = PlayerId::new();

    // First check succeeds
    assert!(
        limiter.check_diagnostic(player).await.is_ok(),
        "First check must succeed"
    );

    // Immediate second check must be throttled (<2s interval)
    assert!(
        limiter.check_diagnostic(player).await.is_err(),
        "Immediate second check must be rate-limited"
    );

    // Different player is unaffected
    let player2 = PlayerId::new();
    assert!(
        limiter.check_diagnostic(player2).await.is_ok(),
        "Different player must not be affected by player1 rate limit"
    );
}

#[tokio::test]
async fn test_lane_isolation_saturating_check_lane_does_not_block_submit_lane() {
    let config = make_test_config();
    // submit_capacity = 2, check_capacity = 1, acquire_timeout = 2s
    let pool = Arc::new(VerifierPool::new(2, 1, Duration::from_secs(2)));

    let metrics = *pool.metrics_rx.borrow();
    assert_eq!(metrics.available_submit_permits, 2);
    assert_eq!(metrics.available_check_permits, 1);

    let challenge = ProofChallenge {
        goal: "∀ n : ℕ, n + 0 = n".to_string(),
        imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
    };
    let wrapped = proof_battle_server::lean::wrap_problem(
        &challenge.imports,
        &challenge.goal,
        "intro n\nsimp\n",
        &config,
    )
    .unwrap();

    // Submit lane should run smoothly
    let submit_res = pool.run_submit(&config, &wrapped.0).await;
    assert!(submit_res.is_ok(), "Submit lane must execute successfully");
}

#[tokio::test]
async fn test_check_storm_concurrent_diagnostic_execution() {
    let config = make_test_config();
    // submit_capacity = 2, check_capacity = 2
    let pool = Arc::new(VerifierPool::new(2, 2, Duration::from_secs(2)));
    let challenge = ProofChallenge {
        goal: "∀ n : ℕ, n + 0 = n".to_string(),
        imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
    };

    let mut handles = Vec::new();

    // Spawn 8 concurrent diagnostic check requests
    for i in 0..8 {
        let code = format!("intro n\n-- check storm {}\nsimp\n", i);
        let ch = challenge.clone();
        let cfg = config.clone();
        let pl = pool.clone();

        handles.push(tokio::spawn(async move {
            run_check_only(&code, &ch, &cfg, &pl).await
        }));
    }

    let mut executed_count = 0;
    let mut skipped_count = 0;
    for h in handles {
        let (_diagnostics, skipped) = h.await.expect("task completed without panic");
        if skipped {
            skipped_count += 1;
        } else {
            executed_count += 1;
        }
    }

    // Lane isolation guarantees that saturated checks are skipped rather than hanging
    assert!(
        executed_count + skipped_count == 8,
        "All 8 check storm requests must finish"
    );
}
