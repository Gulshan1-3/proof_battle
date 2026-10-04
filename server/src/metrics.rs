// ProofBattle Metrics (Prometheus Text Exposition 0.0.4)
//
// LABEL DISCIPLINE:
// Never label metrics with player_id, room_id, or problem_id.
// Unbounded label cardinality will exhaust memory in Prometheus or monitoring collectors.
// All labels must belong to small, strictly fixed enum sets.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

pub static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn get_metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::new)
}

pub struct AtomicHistogram<const N: usize> {
    buckets: [f64; N],
    counts: [AtomicU64; N],
    count: AtomicU64,
    sum: AtomicU64, // float encoded as bits or microsecond sum
}

impl<const N: usize> AtomicHistogram<N> {
    pub const fn new(buckets: [f64; N]) -> Self {
        Self {
            buckets,
            counts: [const { AtomicU64::new(0) }; N],
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }

    pub fn observe(&self, value: f64) {
        for (i, &bucket) in self.buckets.iter().enumerate() {
            if value <= bucket {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        self.count.fetch_add(1, Ordering::Relaxed);

        // Store sum in microseconds to keep atomic integer arithmetic
        let micros = (value.max(0.0) * 1_000_000.0) as u64;
        self.sum.fetch_add(micros, Ordering::Relaxed);
    }

    pub fn render(&self, name: &str, out: &mut String) {
        for (i, &bucket) in self.buckets.iter().enumerate() {
            let count = self.counts[i].load(Ordering::Relaxed);
            out.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", name, bucket, count));
        }
        let total_count = self.count.load(Ordering::Relaxed);
        let total_sum = (self.sum.load(Ordering::Relaxed) as f64) / 1_000_000.0;
        out.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, total_count));
        out.push_str(&format!("{}_sum {:.6}\n", name, total_sum));
        out.push_str(&format!("{}_count {}\n", name, total_count));
    }
}

pub struct Metrics {
    // Gauges
    pub players_online: AtomicI64,
    pub games_in_progress: AtomicI64,
    pub ws_connections: AtomicI64,

    // Counters
    pub rooms_total: AtomicU64,
    pub rate_limited_total: AtomicU64,

    // Queue drops by lane (submit, check)
    pub queue_dropped_submit: AtomicU64,
    pub queue_dropped_check: AtomicU64,

    // Histograms
    pub lean_duration_seconds: AtomicHistogram<8>,
    pub lean_queue_wait_seconds: AtomicHistogram<8>,

    // Verification outcomes (bounded reason categories)
    pub verifications_accepted: AtomicU64,
    pub verifications_rejected_leaning_failed: AtomicU64,
    pub verifications_rejected_timed_out: AtomicU64,
    pub verifications_rejected_uses_sorry: AtomicU64,
    pub verifications_rejected_too_large: AtomicU64,
    pub verifications_rejected_by_filter: AtomicU64,
    pub verifications_rejected_busy: AtomicU64,
    pub verifications_rejected_internal: AtomicU64,
    pub verifications_error: AtomicU64,

    // Sandbox rejections
    pub sandbox_rejected_filter: AtomicU64,
    pub sandbox_rejected_timeout: AtomicU64,
    pub sandbox_rejected_seccomp: AtomicU64,
    pub sandbox_rejected_oom: AtomicU64,
    pub sandbox_rejected_internal: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            players_online: AtomicI64::new(0),
            games_in_progress: AtomicI64::new(0),
            ws_connections: AtomicI64::new(0),
            rooms_total: AtomicU64::new(0),
            rate_limited_total: AtomicU64::new(0),
            queue_dropped_submit: AtomicU64::new(0),
            queue_dropped_check: AtomicU64::new(0),
            lean_duration_seconds: AtomicHistogram::new([
                0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 60.0,
            ]),
            lean_queue_wait_seconds: AtomicHistogram::new([
                0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            verifications_accepted: AtomicU64::new(0),
            verifications_rejected_leaning_failed: AtomicU64::new(0),
            verifications_rejected_timed_out: AtomicU64::new(0),
            verifications_rejected_uses_sorry: AtomicU64::new(0),
            verifications_rejected_too_large: AtomicU64::new(0),
            verifications_rejected_by_filter: AtomicU64::new(0),
            verifications_rejected_busy: AtomicU64::new(0),
            verifications_rejected_internal: AtomicU64::new(0),
            verifications_error: AtomicU64::new(0),
            sandbox_rejected_filter: AtomicU64::new(0),
            sandbox_rejected_timeout: AtomicU64::new(0),
            sandbox_rejected_seccomp: AtomicU64::new(0),
            sandbox_rejected_oom: AtomicU64::new(0),
            sandbox_rejected_internal: AtomicU64::new(0),
        }
    }

    pub fn inc_ws_connections(&self) {
        self.ws_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_ws_connections(&self) {
        self.ws_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn set_players_online(&self, count: i64) {
        self.players_online.store(count.max(0), Ordering::Relaxed);
    }

    pub fn inc_games_in_progress(&self) {
        self.games_in_progress.fetch_add(1, Ordering::Relaxed);
        self.rooms_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_games_in_progress(&self) {
        self.games_in_progress.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_rate_limited(&self) {
        self.rate_limited_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_queue_dropped(&self, lane: &str) {
        match lane {
            "submit" => self.queue_dropped_submit.fetch_add(1, Ordering::Relaxed),
            _ => self.queue_dropped_check.fetch_add(1, Ordering::Relaxed),
        };
    }

    pub fn record_queue_wait(&self, seconds: f64) {
        self.lean_queue_wait_seconds.observe(seconds);
    }

    pub fn record_verification(&self, verdict: &str, reason: Option<&str>, duration_secs: f64) {
        self.lean_duration_seconds.observe(duration_secs);
        match verdict {
            "Accepted" => {
                self.verifications_accepted.fetch_add(1, Ordering::Relaxed);
            }
            "Rejected" => match reason {
                Some("LeaningFailed") => {
                    self.verifications_rejected_leaning_failed
                        .fetch_add(1, Ordering::Relaxed);
                }
                Some("TimedOut") => {
                    self.verifications_rejected_timed_out
                        .fetch_add(1, Ordering::Relaxed);
                }
                Some("UsesSorry") => {
                    self.verifications_rejected_uses_sorry
                        .fetch_add(1, Ordering::Relaxed);
                }
                Some("TooLarge") => {
                    self.verifications_rejected_too_large
                        .fetch_add(1, Ordering::Relaxed);
                }
                Some("RejectedByFilter") => {
                    self.verifications_rejected_by_filter
                        .fetch_add(1, Ordering::Relaxed);
                }
                Some("Busy") => {
                    self.verifications_rejected_busy
                        .fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    self.verifications_rejected_internal
                        .fetch_add(1, Ordering::Relaxed);
                }
            },
            _ => {
                self.verifications_error.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn record_sandbox_rejection(&self, reason: &str) {
        match reason {
            "filter" => self.sandbox_rejected_filter.fetch_add(1, Ordering::Relaxed),
            "timeout" => self
                .sandbox_rejected_timeout
                .fetch_add(1, Ordering::Relaxed),
            "seccomp" => self
                .sandbox_rejected_seccomp
                .fetch_add(1, Ordering::Relaxed),
            "oom" => self.sandbox_rejected_oom.fetch_add(1, Ordering::Relaxed),
            _ => self
                .sandbox_rejected_internal
                .fetch_add(1, Ordering::Relaxed),
        };
    }

    pub fn render(&self) -> String {
        let mut out = String::with_capacity(4096);

        out.push_str(
            "# HELP proofbattle_players_online Number of authenticated players currently online\n",
        );
        out.push_str("# TYPE proofbattle_players_online gauge\n");
        out.push_str(&format!(
            "proofbattle_players_online {}\n\n",
            self.players_online.load(Ordering::Relaxed).max(0)
        ));

        out.push_str(
            "# HELP proofbattle_games_in_progress Number of active games currently running\n",
        );
        out.push_str("# TYPE proofbattle_games_in_progress gauge\n");
        out.push_str(&format!(
            "proofbattle_games_in_progress {}\n\n",
            self.games_in_progress.load(Ordering::Relaxed).max(0)
        ));

        out.push_str("# HELP proofbattle_ws_connections Number of open WebSocket connections\n");
        out.push_str("# TYPE proofbattle_ws_connections gauge\n");
        out.push_str(&format!(
            "proofbattle_ws_connections {}\n\n",
            self.ws_connections.load(Ordering::Relaxed).max(0)
        ));

        out.push_str("# HELP proofbattle_rooms_total Total number of rooms created\n");
        out.push_str("# TYPE proofbattle_rooms_total counter\n");
        out.push_str(&format!(
            "proofbattle_rooms_total {}\n\n",
            self.rooms_total.load(Ordering::Relaxed)
        ));

        out.push_str(
            "# HELP proofbattle_rate_limited_total Total requests rejected due to rate limits\n",
        );
        out.push_str("# TYPE proofbattle_rate_limited_total counter\n");
        out.push_str(&format!(
            "proofbattle_rate_limited_total {}\n\n",
            self.rate_limited_total.load(Ordering::Relaxed)
        ));

        out.push_str(
            "# HELP proofbattle_queue_dropped_total Total requests dropped from verifier queues\n",
        );
        out.push_str("# TYPE proofbattle_queue_dropped_total counter\n");
        out.push_str(&format!(
            "proofbattle_queue_dropped_total{{lane=\"submit\"}} {}\n",
            self.queue_dropped_submit.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_queue_dropped_total{{lane=\"check\"}} {}\n\n",
            self.queue_dropped_check.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proofbattle_lean_verifications_total Total verifications partitioned by verdict and reason\n");
        out.push_str("# TYPE proofbattle_lean_verifications_total counter\n");
        out.push_str(&format!(
            "proofbattle_lean_verifications_total{{verdict=\"Accepted\",reason=\"none\"}} {}\n",
            self.verifications_accepted.load(Ordering::Relaxed)
        ));
        out.push_str(&format!("proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"LeaningFailed\"}} {}\n", self.verifications_rejected_leaning_failed.load(Ordering::Relaxed)));
        out.push_str(&format!(
            "proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"TimedOut\"}} {}\n",
            self.verifications_rejected_timed_out
                .load(Ordering::Relaxed)
        ));
        out.push_str(&format!("proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"UsesSorry\"}} {}\n", self.verifications_rejected_uses_sorry.load(Ordering::Relaxed)));
        out.push_str(&format!(
            "proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"TooLarge\"}} {}\n",
            self.verifications_rejected_too_large
                .load(Ordering::Relaxed)
        ));
        out.push_str(&format!("proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"RejectedByFilter\"}} {}\n", self.verifications_rejected_by_filter.load(Ordering::Relaxed)));
        out.push_str(&format!(
            "proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"Busy\"}} {}\n",
            self.verifications_rejected_busy.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_lean_verifications_total{{verdict=\"Rejected\",reason=\"Internal\"}} {}\n",
            self.verifications_rejected_internal.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_lean_verifications_total{{verdict=\"Error\",reason=\"none\"}} {}\n\n",
            self.verifications_error.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proofbattle_sandbox_rejected_total Total executions rejected by sandbox limits\n");
        out.push_str("# TYPE proofbattle_sandbox_rejected_total counter\n");
        out.push_str(&format!(
            "proofbattle_sandbox_rejected_total{{reason=\"filter\"}} {}\n",
            self.sandbox_rejected_filter.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_sandbox_rejected_total{{reason=\"timeout\"}} {}\n",
            self.sandbox_rejected_timeout.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_sandbox_rejected_total{{reason=\"seccomp\"}} {}\n",
            self.sandbox_rejected_seccomp.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_sandbox_rejected_total{{reason=\"oom\"}} {}\n",
            self.sandbox_rejected_oom.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proofbattle_sandbox_rejected_total{{reason=\"internal\"}} {}\n\n",
            self.sandbox_rejected_internal.load(Ordering::Relaxed)
        ));

        out.push_str(
            "# HELP proofbattle_lean_duration_seconds Verification execution duration in seconds\n",
        );
        out.push_str("# TYPE proofbattle_lean_duration_seconds histogram\n");
        self.lean_duration_seconds
            .render("proofbattle_lean_duration_seconds", &mut out);
        out.push('\n');

        out.push_str("# HELP proofbattle_lean_queue_wait_seconds Time spent waiting in verification queue in seconds\n");
        out.push_str("# TYPE proofbattle_lean_queue_wait_seconds histogram\n");
        self.lean_queue_wait_seconds
            .render("proofbattle_lean_queue_wait_seconds", &mut out);

        out
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
