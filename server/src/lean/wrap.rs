use crate::config::Config;
use crate::lean::filter::{FilterError, Limits, filter_tactic_body};
use crate::lean::runner::RejectReason;

/// Wraps a problem statement and user-submitted tactic script into a complete, standalone Lean 4 file.
///
/// Returns a tuple containing:
/// - Full `.lean` source string
/// - Preamble line count (for correcting diagnostic line numbers sent back to Monaco)
pub fn wrap_problem(
    imports: &[String],
    goal: &str,
    tactic_body: &str,
    config: &Config,
) -> Result<(String, usize), RejectReason> {
    let limits = Limits {
        max_bytes: config.proof_max_bytes,
        max_lines: config.proof_max_lines,
    };

    let sanitized_body = match filter_tactic_body(tactic_body, &limits) {
        Ok(b) => b,
        Err(FilterError::TooLarge { .. }) => return Err(RejectReason::TooLarge),
        Err(e) => return Err(RejectReason::RejectedByFilter(e.to_string())),
    };

    let mut preamble = String::new();
    for import in imports {
        preamble.push_str(import);
        preamble.push('\n');
    }
    preamble.push_str("set_option warningAsError true\n");
    preamble.push_str(&format!(
        "set_option maxHeartbeats {}\n\n",
        config.lean_max_heartbeats
    ));
    preamble.push_str(&format!("theorem goal : {} := by\n", goal));

    let preamble_lines = preamble.lines().count();
    let full_source = format!("{}{}", preamble, sanitized_body);

    Ok((full_source, preamble_lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_wrap_problem_structure() {
        let env = HashMap::new();
        let config = Config::from_map(&env).unwrap();
        let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
        let goal = "∀ n : ℕ, n + 0 = n";
        let body = "  intro n\n  rfl\n";

        let (wrapped, preamble_lines) = wrap_problem(&imports, goal, body, &config).unwrap();
        assert!(wrapped.contains("import Mathlib.Data.Nat.Basic\n"));
        assert!(wrapped.contains("set_option warningAsError true\n"));
        assert!(wrapped.contains("set_option maxHeartbeats 200000\n"));
        assert!(wrapped.contains("theorem goal : ∀ n : ℕ, n + 0 = n := by\n"));
        assert!(wrapped.ends_with("  intro n\n  rfl\n"));
        assert_eq!(preamble_lines, 5);
    }

    #[test]
    fn test_wrap_size_limits() {
        let mut env = HashMap::new();
        env.insert("PROOF_MAX_BYTES".to_string(), "20".to_string());
        env.insert("PROOF_MAX_LINES".to_string(), "2".to_string());
        let config = Config::from_map(&env).unwrap();

        let imports = vec![];
        let goal = "True";

        // Exceeds byte count
        let long_body = "123456789012345678901";
        assert_eq!(
            wrap_problem(&imports, goal, long_body, &config).unwrap_err(),
            RejectReason::TooLarge
        );

        // Exceeds line count
        let multi_line = "a\nb\nc\n";
        assert_eq!(
            wrap_problem(&imports, goal, multi_line, &config).unwrap_err(),
            RejectReason::TooLarge
        );
    }
}
