use super::PlayerId;
use crate::challenges::ProofChallenge;
use crate::config::Config;
use crate::lean::{self, RejectReason, Verdict, VerifyOutcome};
use crate::ws::message::{Diagnostic, DiagnosticSeverity};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, watch};

// =============================================================================
// Token Bucket Rate Limiter (Zero External Dependencies)
// =============================================================================

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    last_updated: Instant,
    max_tokens: f64,
    refill_rate_per_sec: f64,
}

impl Bucket {
    fn new(max_tokens: f64, refill_period: Duration) -> Self {
        let refill_rate_per_sec = max_tokens / refill_period.as_secs_f64();
        Self {
            tokens: max_tokens,
            last_updated: Instant::now(),
            max_tokens,
            refill_rate_per_sec,
        }
    }

    fn try_consume(&mut self, cost: f64) -> Result<(), Duration> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_updated).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate_per_sec).min(self.max_tokens);
        self.last_updated = now;

        if self.tokens >= cost {
            self.tokens -= cost;
            Ok(())
        } else {
            let needed = cost - self.tokens;
            let wait_secs = needed / self.refill_rate_per_sec;
            Err(Duration::from_secs_f64(wait_secs))
        }
    }
}

/// Per-player token bucket rate limiter:
/// - Submits: 5 per 60s
/// - Checks: 1 per 2s
#[derive(Clone)]
pub struct RateLimiter {
    submit_buckets: Arc<RwLock<HashMap<PlayerId, Bucket>>>,
    check_buckets: Arc<RwLock<HashMap<PlayerId, Bucket>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            submit_buckets: Arc::new(RwLock::new(HashMap::new())),
            check_buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Checks if a submit is allowed for `player_id`. Returns Ok(()) or Err(retry_after).
    pub async fn check_submit(&self, player_id: PlayerId) -> Result<(), Duration> {
        let mut map = self.submit_buckets.write().await;
        let bucket = map
            .entry(player_id)
            .or_insert_with(|| Bucket::new(5.0, Duration::from_secs(60)));
        bucket.try_consume(1.0)
    }

    /// Checks if a live diagnostic check is allowed for `player_id`. Returns Ok(()) or Err(retry_after).
    pub async fn check_diagnostic(&self, player_id: PlayerId) -> Result<(), Duration> {
        let mut map = self.check_buckets.write().await;
        let bucket = map
            .entry(player_id)
            .or_insert_with(|| Bucket::new(1.0, Duration::from_secs(2)));
        bucket.try_consume(1.0)
    }

    pub async fn remove_player(&self, player_id: &PlayerId) {
        let mut s_map = self.submit_buckets.write().await;
        s_map.remove(player_id);
        let mut c_map = self.check_buckets.write().await;
        c_map.remove(player_id);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Verifier Pool with Two Lanes (ADR-008)
// =============================================================================

#[derive(Debug, Clone, Copy, Default)]
pub struct VerifierMetrics {
    pub in_flight_submits: usize,
    pub in_flight_checks: usize,
    pub available_submit_permits: usize,
    pub available_check_permits: usize,
}

#[derive(Clone)]
pub struct VerifierPool {
    submit_semaphore: Arc<Semaphore>,
    check_semaphore: Arc<Semaphore>,
    in_flight_submits: Arc<AtomicUsize>,
    in_flight_checks: Arc<AtomicUsize>,
    metrics_tx: Arc<watch::Sender<VerifierMetrics>>,
    pub metrics_rx: watch::Receiver<VerifierMetrics>,
    acquire_timeout: Duration,
}

impl VerifierPool {
    pub fn new(submit_capacity: usize, check_capacity: usize, acquire_timeout: Duration) -> Self {
        let submit_semaphore = Arc::new(Semaphore::new(submit_capacity));
        let check_semaphore = Arc::new(Semaphore::new(check_capacity));
        let (metrics_tx, metrics_rx) = watch::channel(VerifierMetrics {
            in_flight_submits: 0,
            in_flight_checks: 0,
            available_submit_permits: submit_capacity,
            available_check_permits: check_capacity,
        });

        Self {
            submit_semaphore,
            check_semaphore,
            in_flight_submits: Arc::new(AtomicUsize::new(0)),
            in_flight_checks: Arc::new(AtomicUsize::new(0)),
            metrics_tx: Arc::new(metrics_tx),
            metrics_rx,
            acquire_timeout,
        }
    }

    fn notify_metrics(&self) {
        let metrics = VerifierMetrics {
            in_flight_submits: self.in_flight_submits.load(Ordering::Relaxed),
            in_flight_checks: self.in_flight_checks.load(Ordering::Relaxed),
            available_submit_permits: self.submit_semaphore.available_permits(),
            available_check_permits: self.check_semaphore.available_permits(),
        };
        let _ = self.metrics_tx.send(metrics);
    }

    /// Submit lane: acquires with timeout. If timed out, returns Err(RejectReason::Busy).
    pub async fn run_submit(
        &self,
        config: &Config,
        processed_code: &str,
    ) -> Result<VerifyOutcome, RejectReason> {
        let permit =
            match tokio::time::timeout(self.acquire_timeout, self.submit_semaphore.acquire()).await
            {
                Ok(Ok(p)) => p,
                _ => return Err(RejectReason::Busy),
            };

        self.in_flight_submits.fetch_add(1, Ordering::Relaxed);
        self.notify_metrics();

        let outcome = lean::verify_once(config, processed_code).await;

        self.in_flight_submits.fetch_sub(1, Ordering::Relaxed);
        drop(permit);
        self.notify_metrics();

        Ok(outcome)
    }

    /// Check lane: best-effort with try_acquire only.
    /// If full, drops check immediately without queueing and returns Ok(None).
    pub async fn run_check(&self, config: &Config, processed_code: &str) -> Option<VerifyOutcome> {
        let permit = self.check_semaphore.try_acquire().ok()?;

        self.in_flight_checks.fetch_add(1, Ordering::Relaxed);
        self.notify_metrics();

        let outcome = lean::verify_once(config, processed_code).await;

        self.in_flight_checks.fetch_sub(1, Ordering::Relaxed);
        drop(permit);
        self.notify_metrics();

        Some(outcome)
    }
}

// =============================================================================
// Structural Check Isolation Function
// =============================================================================

/// Live diagnostics runner: STRUCTURALLY decoupled from Room actor.
/// Has zero access to Room actor state and CANNOT mutate score, winners, or emit RoundEnd.
pub async fn run_check_only(
    code: &str,
    challenge: &ProofChallenge,
    config: &Config,
    pool: &VerifierPool,
) -> (Vec<Diagnostic>, bool) {
    let clean_code = code.lines().map(str::trim).collect::<Vec<_>>().join("\n");

    let wrapped = match lean::wrap_problem(&challenge.imports, &challenge.goal, &clean_code, config)
    {
        Ok((w, preamble_lines)) => (w, preamble_lines),
        Err(err) => {
            return (
                vec![Diagnostic {
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: 1,
                    severity: DiagnosticSeverity::Error,
                    message: format!("Filter error: {err:?}"),
                }],
                false,
            );
        }
    };

    let processed = crate::utils::preprocess(&wrapped.0);

    match pool.run_check(config, &processed).await {
        Some(outcome) => match outcome.verdict {
            Verdict::Accepted { .. } => (vec![], false),
            Verdict::Rejected { stderr, .. } => {
                let diag = Diagnostic {
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: 1,
                    severity: DiagnosticSeverity::Error,
                    message: stderr,
                };
                (vec![diag], false)
            }
            Verdict::Error(e) => {
                let diag = Diagnostic {
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: 1,
                    severity: DiagnosticSeverity::Error,
                    message: e,
                };
                (vec![diag], false)
            }
        },
        None => {
            // Check lane full -> check skipped silently
            (vec![], true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_5_submits_in_window() {
        let limiter = RateLimiter::new();
        let player = PlayerId::new();

        // First 5 submits must succeed
        for _ in 0..5 {
            assert!(limiter.check_submit(player).await.is_ok());
        }

        // 6th submit within window must be rate limited
        let res = limiter.check_submit(player).await;
        assert!(res.is_err(), "6th submit must be rate limited");
        let retry_after = res.unwrap_err();
        assert!(retry_after > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_rate_limiter_check_1_per_2_seconds() {
        let limiter = RateLimiter::new();
        let player = PlayerId::new();

        assert!(limiter.check_diagnostic(player).await.is_ok());
        // Immediate second check must be rate limited
        assert!(limiter.check_diagnostic(player).await.is_err());
    }

    #[tokio::test]
    async fn test_verifier_pool_check_lane_skips_when_full() {
        // Pool with 0 check permits
        let pool = VerifierPool::new(1, 0, Duration::from_millis(50));
        let config = Config::from_map(&HashMap::new()).unwrap();

        let outcome = pool.run_check(&config, "test").await;
        assert!(
            outcome.is_none(),
            "Check must be dropped when check lane is full"
        );
    }

    #[tokio::test]
    async fn test_verifier_pool_submit_timeout_busy() {
        // Pool with 1 permit, held indefinitely
        let pool = VerifierPool::new(1, 1, Duration::from_millis(20));
        let config = Config::from_map(&HashMap::new()).unwrap();

        // Acquire the only permit
        let permit = pool.submit_semaphore.acquire().await.unwrap();

        // Second submit should timeout and return RejectReason::Busy
        let res = pool.run_submit(&config, "test").await;
        assert_eq!(res.unwrap_err(), RejectReason::Busy);

        drop(permit);
    }
}
