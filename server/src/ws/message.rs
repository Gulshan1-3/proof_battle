use serde::{Deserialize, Serialize};
use ts_rs::TS;

// =============================================================================
// Constitution Invariant 1 (TRUST NOTHING FROM THE CLIENT) & ADR-001 / ADR-002:
//
// Player identity is server-assigned at WebSocket upgrade and strictly bound to
// the connection socket. No client message may EVER carry a `player_id`,
// `room_id`, `winner`, or `elo` field.
//
// Any attempt by a client to supply unexpected fields is strictly rejected
// at deserialization via #[serde(deny_unknown_fields)] as bad_request.
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "PascalCase")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum ProofIntent {
    Submit,
    Check,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
#[ts(
    tag = "type",
    export,
    export_to = "../../../frontend/src/lib/ws/generated.ts"
)]
pub enum ClientMessage {
    Hello {
        version: u32,
        token: Option<String>,
        username: Option<String>,
    },
    QueueJoin {},
    QueueLeave {},
    PracticeJoin {},
    ProofRequest {
        req_id: String,
        code: String,
        intent: ProofIntent,
    },
    Resign {},
    Pong {
        ts: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub struct SearchRange {
    pub min_elo: i32,
    pub max_elo: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub struct PlayerInfo {
    pub player_id: String,
    pub username: Option<String>,
    pub elo: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub struct Problem {
    pub id: String,
    pub goal: String,
    pub imports: Vec<String>,
    pub difficulty: u8,
    pub category: String,
    pub hint: Option<String>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "PascalCase")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum VerdictStatus {
    Accepted,
    Rejected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "detail")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum RejectReason {
    LeaningFailed,
    TimedOut,
    UsesSorry,
    TooLarge,
    RejectedByFilter(String),
    Busy,
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub struct Diagnostic {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

/// OpponentActivity is server-inferred only and never client-asserted.
/// Invariant: it must NEVER reveal proof code or keystrokes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "PascalCase")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum OpponentStatus {
    Idle,
    Typing,
    Checking,
    Verifying,
    Submitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "PascalCase")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum MatchOutcome {
    Won,
    Lost,
    Draw,
    ForfeitWin,
    ForfeitLoss,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "../../../frontend/src/lib/ws/generated.ts")]
pub enum ServerMessage {
    Welcome {
        player_id: String,
        session_token: String,
        server_time_ms: i64,
        heartbeat_interval_ms: i64,
        protocol_version: u32,
    },
    ServerTime {
        server_time_ms: i64,
    },
    QueueStatus {
        queue_size: u32,
        elo: i32,
        search_range: SearchRange,
    },
    MatchFound {
        room_id: String,
        you: PlayerInfo,
        opponent: PlayerInfo,
    },
    RoundStart {
        room_id: String,
        problem: Problem,
        starts_at_ms: i64,
        ends_at_ms: i64,
        duration_ms: i64,
        server_time_ms: i64,
        seq: u64,
    },
    Verdict {
        req_id: String,
        verdict: VerdictStatus,
        reason: Option<RejectReason>,
        message: String,
        diagnostics: Vec<Diagnostic>,
        elapsed_ms: u64,
        check_skipped: bool,
    },
    OpponentActivity {
        status: OpponentStatus,
    },
    RoundEnd {
        room_id: String,
        outcome: MatchOutcome,
        winner_id: Option<String>,
        winning_proof: Option<String>,
        canonical_proof: Option<String>,
        elo_delta: i32,
        duration_ms: i64,
        seq: u64,
    },
    ServerError {
        code: String,
        message: String,
        retry_after_ms: Option<u64>,
        retryable: bool,
    },
    Pong {
        ts: i64,
    },
}

impl Diagnostic {
    /// Constructs a Diagnostic by correcting line numbers with the server-owned preamble line count.
    /// Lines in Monaco are 1-based and relative to the player's tactic body.
    pub fn from_raw_lean(
        raw_line: u32,
        col: u32,
        end_line: u32,
        end_col: u32,
        preamble_lines: usize,
        severity: DiagnosticSeverity,
        message: String,
    ) -> Self {
        let corrected_line = if (raw_line as usize) > preamble_lines {
            (raw_line as usize - preamble_lines) as u32
        } else {
            1
        };
        let corrected_end_line = if (end_line as usize) > preamble_lines {
            (end_line as usize - preamble_lines) as u32
        } else {
            corrected_line
        };

        Self {
            line: corrected_line,
            col,
            end_line: corrected_end_line,
            end_col,
            severity,
            message,
        }
    }
}
