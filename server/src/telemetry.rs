use crate::ws::message::Diagnostic;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptTelemetryRecord {
    pub match_id: String,
    pub problem_id: String,
    pub category: String,
    pub structural_difficulty: u8,
    pub goal: String,
    pub imports: Vec<String>,
    pub player_id: String,
    pub username: Option<String>,
    pub opponent_id: Option<String>,
    pub attempt_index: u32,
    pub intent: String,
    pub tactic: String,
    pub verdict: String,
    pub reject_reason: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub elapsed_ms: u64,
    pub lean_ms: u64,
    pub queue_wait_ms: u64,
    pub time_budget_ms: i64,
    pub time_remaining_ms: i64,
    pub tokens_generated: Option<usize>,
    pub heartbeats_consumed: Option<u64>,
    pub outcome: Option<String>,
    pub timestamp_unix_ms: i64,
}

static TELEMETRY_INSTANCE: OnceLock<Arc<TelemetryLogger>> = OnceLock::new();

#[derive(Clone)]
pub struct TelemetryLogger {
    tx: mpsc::UnboundedSender<AttemptTelemetryRecord>,
}

impl TelemetryLogger {
    pub fn init(custom_path: Option<PathBuf>) -> Arc<Self> {
        if let Some(existing) = TELEMETRY_INSTANCE.get() {
            return existing.clone();
        }

        let log_path = custom_path
            .or_else(|| std::env::var("TELEMETRY_LOG_PATH").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("telemetry_attempts.jsonl"));

        let (tx, mut rx) = mpsc::unbounded_channel::<AttemptTelemetryRecord>();

        tokio::spawn(async move {
            let mut file = match OpenOptions::new().create(true).append(true).open(&log_path) {
                Ok(f) => Some(f),
                Err(e) => {
                    tracing::warn!(error = %e, path = ?log_path, "failed to open telemetry log file");
                    None
                }
            };

            while let Some(record) = rx.recv().await {
                if let Some(ref mut f) = file {
                    match serde_json::to_string(&record) {
                        Ok(json) => {
                            if let Err(e) = writeln!(f, "{}", json) {
                                tracing::warn!(error = %e, "failed to write to telemetry log file");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to serialize telemetry record");
                        }
                    }
                }
            }
        });

        let logger = Arc::new(Self { tx });
        let _ = TELEMETRY_INSTANCE.set(logger.clone());
        logger
    }

    pub fn get() -> Option<Arc<Self>> {
        TELEMETRY_INSTANCE.get().cloned()
    }

    pub fn record(&self, attempt: AttemptTelemetryRecord) {
        let _ = self.tx.send(attempt);
    }
}
