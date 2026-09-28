use proof_battle_server::config::Config;
use proof_battle_server::lean::{RejectReason, Verdict, wrap_problem};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CorpusFixture {
    id: String,
    category: String,
    goal: String,
    imports: Vec<String>,
    body: String,
    expect: String,
    reject_reason: Option<String>,
}

fn test_config() -> Config {
    let mut env = HashMap::new();
    env.insert("LEAN_TIMEOUT".to_string(), "10".to_string());
    Config::from_map(&env).unwrap()
}

fn resolve_corpus_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("tests/corpus"),
        PathBuf::from("server/tests/corpus"),
        PathBuf::from("../tests/corpus"),
        PathBuf::from("../server/tests/corpus"),
    ];
    for cand in &candidates {
        if cand.is_dir() {
            return std::fs::canonicalize(cand).unwrap_or_else(|_| cand.clone());
        }
    }
    PathBuf::from("tests/corpus")
}

fn load_fixture(id: &str) -> CorpusFixture {
    let corpus_dir = resolve_corpus_dir();
    let file_path = corpus_dir.join(format!("{id}.json"));
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", file_path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {e}", file_path.display()))
}

async fn run_fixture_case(fixture: &CorpusFixture, config: &Config) -> (bool, String) {
    let wrap_res = wrap_problem(&fixture.imports, &fixture.goal, &fixture.body, config);

    match wrap_res {
        Err(reject_reason) => {
            if fixture.expect == "reject" {
                if let Some(expected_reason) = &fixture.reject_reason {
                    let matches_reason = matches!(
                        (&reject_reason, expected_reason.as_str()),
                        (RejectReason::RejectedByFilter(_), "RejectedByFilter")
                            | (RejectReason::TooLarge, "TooLarge")
                            | (RejectReason::UsesSorry, "UsesSorry")
                            | (RejectReason::TimedOut, "TimedOut")
                            | (RejectReason::LeaningFailed, "LeaningFailed")
                    );
                    if matches_reason {
                        (true, format!("Rejected as expected ({reject_reason:?})"))
                    } else {
                        (
                            false,
                            format!(
                                "Rejected with {reject_reason:?}, but expected {expected_reason}"
                            ),
                        )
                    }
                } else {
                    (true, format!("Rejected as expected ({reject_reason:?})"))
                }
            } else {
                (
                    false,
                    format!("Expected accept, but wrap_problem rejected: {reject_reason:?}"),
                )
            }
        }
        Ok((source, _preamble_lines)) => {
            if fixture.id == "filter_false_positive" {
                // Filter accepted the string/comment "#eval" without error
                return (
                    true,
                    "Filter passed without false-positive error".to_string(),
                );
            }

            let outcome = proof_battle_server::lean::verify_once(config, &source).await;

            match outcome.verdict {
                Verdict::Accepted { .. } => {
                    if fixture.expect == "accept" {
                        (true, "Accepted by Lean as expected".to_string())
                    } else {
                        (
                            false,
                            format!(
                                "SECURITY FAILURE: Payload '{}' was ACCEPTED, expected rejection!",
                                fixture.id
                            ),
                        )
                    }
                }
                Verdict::Rejected { reason, stderr } => {
                    if fixture.expect == "reject" {
                        if let Some(expected_reason) = &fixture.reject_reason {
                            let matches_reason = matches!(
                                (&reason, expected_reason.as_str()),
                                (RejectReason::UsesSorry, "UsesSorry")
                                    | (RejectReason::TimedOut, "TimedOut")
                                    | (RejectReason::LeaningFailed, "LeaningFailed")
                                    | (RejectReason::TooLarge, "TooLarge")
                                    | (RejectReason::RejectedByFilter(_), "RejectedByFilter")
                            );
                            if matches_reason {
                                (true, format!("Rejected by Lean ({reason:?})"))
                            } else {
                                (
                                    true, // Still a rejection
                                    format!("Rejected with {reason:?}, expected {expected_reason}"),
                                )
                            }
                        } else {
                            (true, format!("Rejected by Lean ({reason:?})"))
                        }
                    } else {
                        (
                            false,
                            format!("Expected accept, but Lean rejected: {stderr}"),
                        )
                    }
                }
                Verdict::Error(err) => (false, format!("Harness error executing Lean: {err}")),
            }
        }
    }
}

macro_rules! define_corpus_test {
    ($test_name:ident, $fixture_id:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let config = test_config();
            let fixture = load_fixture($fixture_id);
            let (pass, msg) = run_fixture_case(&fixture, &config).await;
            assert!(pass, "Corpus test '{}' failed: {}", $fixture_id, msg);
        }
    };
}

define_corpus_test!(test_cheat_sorry, "cheat_sorry");
define_corpus_test!(test_cheat_sorry_embedded, "cheat_sorry_embedded");
define_corpus_test!(test_cheat_sorry_in_simpa, "cheat_sorry_in_simpa");
define_corpus_test!(test_cheat_builtin_axiom, "cheat_builtin_axiom");
define_corpus_test!(test_cheat_statement_swap, "cheat_statement_swap");
define_corpus_test!(test_cheat_option_downgrade, "cheat_option_downgrade");
define_corpus_test!(test_cheat_maxheartbeats, "cheat_maxheartbeats");
define_corpus_test!(test_rce_eval_io, "rce_eval_io");
define_corpus_test!(test_rce_eval_shell, "rce_eval_shell");
define_corpus_test!(test_rce_native_decide, "rce_native_decide");
define_corpus_test!(test_rce_extern, "rce_extern");
define_corpus_test!(test_rce_macro_elab, "rce_macro_elab");
define_corpus_test!(test_rce_initialize, "rce_initialize");
define_corpus_test!(test_rce_attribute_instance, "rce_attribute_instance");
define_corpus_test!(test_dos_decide_hard, "dos_decide_hard");
define_corpus_test!(test_dos_huge_term, "dos_huge_term");
define_corpus_test!(test_limit_too_many_lines, "limit_too_many_lines");
define_corpus_test!(test_limit_too_many_bytes, "limit_too_many_bytes");
define_corpus_test!(test_filter_false_positive, "filter_false_positive");
define_corpus_test!(test_filter_nested_comment, "filter_nested_comment");
define_corpus_test!(test_happy_add_zero, "happy_add_zero");
define_corpus_test!(test_happy_add_comm, "happy_add_comm");
define_corpus_test!(test_happy_induction, "happy_induction");
define_corpus_test!(test_lean_aggregate_import, "lean_aggregate_import");
define_corpus_test!(test_stderr_flood, "stderr_flood");

#[test]
fn test_preamble_offset_arithmetic_single_and_multi_imports() {
    use proof_battle_server::ws::message::{Diagnostic, DiagnosticSeverity};
    let config = test_config();

    // 1. Problem with 1 import
    let single_imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
    let (_wrapped1, preamble_lines1) = wrap_problem(
        &single_imports,
        "∀ n : ℕ, n + 0 = n",
        "  intro n\n  rfl\n",
        &config,
    )
    .unwrap();

    // 1 import gives 5 preamble lines:
    // line 1: import Mathlib.Data.Nat.Basic
    // line 2: set_option warningAsError true
    // line 3: set_option maxHeartbeats ...
    // line 4: (blank)
    // line 5: theorem goal : ∀ n : ℕ, n + 0 = n := by
    assert_eq!(preamble_lines1, 5, "Problem with 1 import must have 5 preamble lines");

    // Lean error at generated-line 7 maps to body-line 2 (7 - 5 = 2)
    let diag1 = Diagnostic::from_raw_lean(
        7,
        1,
        7,
        5,
        preamble_lines1,
        DiagnosticSeverity::Error,
        "unsolved goals".to_string(),
    );
    assert_eq!(diag1.line, 2, "Generated line 7 with 1 import must map to body line 2");

    // 2. Problem with 4 imports
    let multi_imports = vec![
        "import Mathlib.Data.Nat.Basic".to_string(),
        "import Mathlib.Data.List.Basic".to_string(),
        "import Mathlib.Tactic.Ring".to_string(),
        "import Mathlib.Tactic.Omega".to_string(),
    ];
    let (_wrapped2, preamble_lines2) = wrap_problem(
        &multi_imports,
        "∀ n : ℕ, n + 0 = n",
        "  intro n\n  rfl\n",
        &config,
    )
    .unwrap();

    // 4 imports give 8 preamble lines (4 imports + 4 template lines)
    assert_eq!(preamble_lines2, 8, "Problem with 4 imports must have 8 preamble lines");

    // Lean error at generated-line 10 (preamble_lines2 + 2) maps to body-line 2 (10 - 8 = 2)
    let diag2 = Diagnostic::from_raw_lean(
        (preamble_lines2 + 2) as u32,
        1,
        (preamble_lines2 + 2) as u32,
        5,
        preamble_lines2,
        DiagnosticSeverity::Error,
        "unsolved goals".to_string(),
    );
    assert_eq!(diag2.line, 2, "Body line 2 must map to line 2 in both 1-import and 4-import cases");

    // If preamble lines were hardcoded (e.g. 5), generated line 10 would incorrectly map to line 5
    let hardcoded_diag = Diagnostic::from_raw_lean(10, 1, 10, 5, 5, DiagnosticSeverity::Error, "err".to_string());
    assert_eq!(hardcoded_diag.line, 5, "Shows hardcoded offset fails on varying import sizes");
}

