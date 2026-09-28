use proof_battle_server::ws::message::*;
use serde_json::{Value, json};
use ts_rs::TS;

#[test]
fn test_client_messages_serialise_with_type_discriminator_and_roundtrip() {
    let cases = vec![
        (
            ClientMessage::Hello {
                version: 1,
                token: Some("token_123".to_string()),
                username: Some("alice".to_string()),
            },
            "Hello",
        ),
        (ClientMessage::QueueJoin {}, "QueueJoin"),
        (ClientMessage::QueueLeave {}, "QueueLeave"),
        (
            ClientMessage::ProofRequest {
                req_id: "req_001".to_string(),
                code: "intro n\nsimp".to_string(),
                intent: ProofIntent::Submit,
            },
            "ProofRequest",
        ),
        (
            ClientMessage::ProofRequest {
                req_id: "req_002".to_string(),
                code: "intro n".to_string(),
                intent: ProofIntent::Check,
            },
            "ProofRequest",
        ),
        (ClientMessage::Resign {}, "Resign"),
        (ClientMessage::Pong { ts: 1700000000000 }, "Pong"),
    ];

    for (msg, expected_type) in cases {
        let serialized = serde_json::to_string(&msg).expect("serialization failed");
        let val: Value = serde_json::from_str(&serialized).expect("valid JSON");

        // (a) Must be a JSON object with "type" field
        assert!(val.is_object(), "Message must serialize to an object");
        assert_eq!(
            val.get("type").and_then(Value::as_str),
            Some(expected_type),
            "Message must have correct 'type' tag"
        );

        // (b) Round-trips cleanly
        let deserialized: ClientMessage =
            serde_json::from_str(&serialized).expect("deserialization failed");
        assert_eq!(msg, deserialized, "Roundtrip equality check");
    }
}

#[test]
fn test_server_messages_serialise_with_type_discriminator_and_roundtrip() {
    let cases = vec![
        (
            ServerMessage::Welcome {
                player_id: "p1".to_string(),
                session_token: "tok_xyz".to_string(),
                server_time_ms: 1000,
                heartbeat_interval_ms: 15000,
                protocol_version: 1,
            },
            "Welcome",
        ),
        (
            ServerMessage::ServerTime {
                server_time_ms: 2000,
            },
            "ServerTime",
        ),
        (
            ServerMessage::QueueStatus {
                queue_size: 4,
                elo: 1200,
                search_range: SearchRange {
                    min_elo: 1100,
                    max_elo: 1300,
                },
            },
            "QueueStatus",
        ),
        (
            ServerMessage::MatchFound {
                room_id: "room_1".to_string(),
                you: PlayerInfo {
                    player_id: "p1".to_string(),
                    username: Some("alice".to_string()),
                    elo: 1200,
                },
                opponent: PlayerInfo {
                    player_id: "p2".to_string(),
                    username: Some("bob".to_string()),
                    elo: 1210,
                },
            },
            "MatchFound",
        ),
        (
            ServerMessage::RoundStart {
                room_id: "room_1".to_string(),
                problem: Problem {
                    id: "p_add_zero".to_string(),
                    goal: "∀ n : ℕ, n + 0 = n".to_string(),
                    imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
                    difficulty: 1,
                    category: "arithmetic".to_string(),
                    hint: Some("try simp".to_string()),
                    duration_ms: 60000,
                },
                starts_at_ms: 1000,
                ends_at_ms: 61000,
                duration_ms: 60000,
                server_time_ms: 1000,
                seq: 1,
            },
            "RoundStart",
        ),
        (
            ServerMessage::Verdict {
                req_id: "req_001".to_string(),
                verdict: VerdictStatus::Accepted,
                reason: None,
                message: "Proof accepted".to_string(),
                diagnostics: vec![],
                elapsed_ms: 150,
                check_skipped: false,
            },
            "Verdict",
        ),
        (
            ServerMessage::Verdict {
                req_id: "req_002".to_string(),
                verdict: VerdictStatus::Rejected,
                reason: Some(RejectReason::UsesSorry),
                message: "Declaration uses sorry".to_string(),
                diagnostics: vec![Diagnostic {
                    line: 1,
                    col: 3,
                    end_line: 1,
                    end_col: 8,
                    severity: DiagnosticSeverity::Error,
                    message: "declaration uses 'sorry'".to_string(),
                }],
                elapsed_ms: 120,
                check_skipped: false,
            },
            "Verdict",
        ),
        (
            ServerMessage::OpponentActivity {
                status: OpponentStatus::Typing,
            },
            "OpponentActivity",
        ),
        (
            ServerMessage::RoundEnd {
                room_id: "room_1".to_string(),
                outcome: MatchOutcome::Won,
                winner_id: Some("p1".to_string()),
                winning_proof: Some("intro n\nsimp".to_string()),
                canonical_proof: Some("intro n\nsimp".to_string()),
                elo_delta: 15,
                duration_ms: 14500,
                seq: 2,
            },
            "RoundEnd",
        ),
        (
            ServerMessage::ServerError {
                code: "bad_request".to_string(),
                message: "malformed payload".to_string(),
                retry_after_ms: None,
                retryable: false,
            },
            "ServerError",
        ),
        (ServerMessage::Pong { ts: 1700000000000 }, "Pong"),
    ];

    for (msg, expected_type) in cases {
        let serialized = serde_json::to_string(&msg).expect("serialization failed");
        let val: Value = serde_json::from_str(&serialized).expect("valid JSON");

        assert!(val.is_object(), "Message must serialize to an object");
        assert_eq!(
            val.get("type").and_then(Value::as_str),
            Some(expected_type),
            "ServerMessage must have correct 'type' tag"
        );

        let deserialized: ServerMessage =
            serde_json::from_str(&serialized).expect("deserialization failed");
        assert_eq!(msg, deserialized, "Roundtrip equality check");
    }
}

#[test]
fn test_unknown_client_type_rejected_as_bad_request() {
    let raw = json!({
        "type": "UnknownHackerFrame",
        "payload": "malicious"
    });
    let res = serde_json::from_value::<ClientMessage>(raw);
    assert!(
        res.is_err(),
        "Unknown message type must be rejected at deserialization"
    );
}

/// Invariant Rule 1: Constitution states that NO ClientMessage variant may EVER contain
/// a field named player_id, room_id, winner, or elo.
#[test]
fn test_constitution_invariant_no_client_identity_fields() {
    // 1. Source AST scan of ClientMessage enum definition
    let source = include_str!("../src/ws/message.rs");
    let client_msg_start = source
        .find("pub enum ClientMessage")
        .expect("ClientMessage definition must exist");
    let client_msg_end = source[client_msg_start..]
        .find("\n}\n")
        .expect("ClientMessage closing brace must exist")
        + client_msg_start;
    let client_msg_block = &source[client_msg_start..client_msg_end];

    let forbidden_identifiers = ["player_id", "room_id", "winner", "elo"];
    for forbidden in forbidden_identifiers {
        assert!(
            !client_msg_block.contains(forbidden),
            "CONSTITUTION VIOLATION: ClientMessage definition contains forbidden identifier '{}'",
            forbidden
        );
    }

    // 2. Deserialization injection attack tests:
    // Malicious payload supplying client-forged player_id, room_id, winner, or elo MUST be rejected
    let injection_payloads = [
        json!({ "type": "ProofRequest", "req_id": "1", "code": "simp", "intent": "Submit", "player_id": "hacker" }),
        json!({ "type": "ProofRequest", "req_id": "1", "code": "simp", "intent": "Submit", "room_id": "room_666" }),
        json!({ "type": "ProofRequest", "req_id": "1", "code": "simp", "intent": "Submit", "winner": "me" }),
        json!({ "type": "ProofRequest", "req_id": "1", "code": "simp", "intent": "Submit", "elo": 9999 }),
        json!({ "type": "Resign", "player_id": "victim" }),
        json!({ "type": "QueueJoin", "elo": 2500 }),
    ];

    for payload in injection_payloads {
        let res = serde_json::from_value::<ClientMessage>(payload);
        assert!(
            res.is_err(),
            "Malicious client identity injection must be rejected by deny_unknown_fields"
        );
    }
}

#[test]
fn test_diagnostic_relative_line_number_correction() {
    // Preamble has 5 lines. Lean diagnostic reports error at line 7, column 4.
    // Relative line in player's tactic body must be 7 - 5 = 2.
    let diag = Diagnostic::from_raw_lean(
        7,
        4,
        7,
        9,
        5,
        DiagnosticSeverity::Error,
        "type mismatch".to_string(),
    );
    assert_eq!(diag.line, 2);
    assert_eq!(diag.col, 4);
    assert_eq!(diag.end_line, 2);
    assert_eq!(diag.end_col, 9);
    assert_eq!(diag.severity, DiagnosticSeverity::Error);
    assert_eq!(diag.message, "type mismatch");

    // Diagnostic on preamble itself clamps to line 1
    let preamble_diag = Diagnostic::from_raw_lean(
        3,
        1,
        3,
        5,
        5,
        DiagnosticSeverity::Warning,
        "preamble warning".to_string(),
    );
    assert_eq!(preamble_diag.line, 1);
}

#[test]
fn export_bindings() {
    let target_dir = std::path::Path::new("../frontend/src/lib/ws");
    std::fs::create_dir_all(target_dir).expect("create frontend ws dir");

    let mut generated_content = String::new();
    generated_content.push_str("// Auto-generated by ts-rs (Stage S4 Protocol Contract)\n");
    generated_content.push_str("// DO NOT EDIT MANUALLY - run `make codegen` to update.\n\n");

    let types = [
        ProofIntent::decl(),
        SearchRange::decl(),
        PlayerInfo::decl(),
        Problem::decl(),
        VerdictStatus::decl(),
        RejectReason::decl(),
        DiagnosticSeverity::decl(),
        Diagnostic::decl(),
        OpponentStatus::decl(),
        MatchOutcome::decl(),
        ClientMessage::decl(),
        ServerMessage::decl(),
    ];

    for decl in types {
        generated_content.push_str(&format!("export {decl}\n\n"));
    }

    let out_file = target_dir.join("generated.ts");
    std::fs::write(&out_file, &generated_content).expect("write generated.ts");
    println!(
        "Successfully exported TypeScript bindings to {}",
        out_file.display()
    );
}
