use crate::config::Config;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub const LEAN_STDERR_MAX_BYTES: usize = 64 * 1024; // 64 KiB
pub const LEAN_STDOUT_MAX_BYTES: usize = 64 * 1024; // 64 KiB

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Accepted {
        stdout: String,
    },
    Rejected {
        reason: RejectReason,
        stderr: String,
    },
    Error(String), // harness/spawn failure, NEVER treated as Accepted
}

pub use crate::ws::message::RejectReason;

#[derive(Debug, Clone)]
pub struct VerifyOutcome {
    pub verdict: Verdict,
    pub elapsed_ms: u64,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub truncated: bool,
}

struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn resolve_project_dir(dir: &Path) -> PathBuf {
    if dir.is_absolute() {
        dir.to_path_buf()
    } else if dir.exists() && dir.join("lakefile.lean").exists() {
        std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
    } else if let Ok(parent_relative) = std::fs::canonicalize(Path::new("..").join(dir)) {
        if parent_relative.join("lakefile.lean").exists() {
            parent_relative
        } else {
            dir.to_path_buf()
        }
    } else {
        dir.to_path_buf()
    }
}

pub fn resolve_proof_dir(config: &Config, project_dir: &Path) -> PathBuf {
    if config.proof_tmp_dir.is_absolute() {
        config.proof_tmp_dir.clone()
    } else {
        let sub = config
            .proof_tmp_dir
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("proofs"));
        project_dir.join(sub)
    }
}

fn find_lake_binary() -> PathBuf {
    if let Ok(path) = which_in_path("lake") {
        return path;
    }
    if let Ok(home) = std::env::var("HOME") {
        let elan_lake = PathBuf::from(home).join(".elan/bin/lake");
        if elan_lake.exists() {
            return elan_lake;
        }
    }
    PathBuf::from("lake")
}

fn which_in_path(executable: &str) -> Result<PathBuf, ()> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(executable);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(())
}

/// Determines the verdict from the Lean process exit code and captured output.
/// Returns (Verdict, Option<&str>) where the second element is a human-readable reason label.
pub fn parse_verdict(
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> (Verdict, Option<&'static str>) {
    let uses_sorry = stderr.contains("declaration uses 'sorry'")
        || stderr.contains("sorryAx")
        || stdout.contains("declaration uses 'sorry'")
        || stdout.contains("sorryAx");

    let diag_output = if !stderr.trim().is_empty() {
        stderr.to_string()
    } else {
        stdout.to_string()
    };

    if uses_sorry {
        return (
            Verdict::Rejected {
                reason: RejectReason::UsesSorry,
                stderr: diag_output,
            },
            Some("uses_sorry"),
        );
    }

    if exit_code == 0 {
        (
            Verdict::Accepted {
                stdout: stdout.to_string(),
            },
            None,
        )
    } else {
        (
            Verdict::Rejected {
                reason: RejectReason::LeaningFailed,
                stderr: diag_output,
            },
            Some("lean_failed"),
        )
    }
}

/// Executes a single Lean 4 verification in an async, isolated subprocess with timeout and process-group kill.
pub async fn verify_once(config: &Config, source: &str) -> VerifyOutcome {
    let start = Instant::now();

    let lean_project_dir = resolve_project_dir(&config.lean_project_dir);
    let proof_tmp_dir = resolve_proof_dir(config, &lean_project_dir);

    // 1. Ensure temp dir exists
    if let Err(e) = tokio::fs::create_dir_all(&proof_tmp_dir).await {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        return VerifyOutcome {
            verdict: Verdict::Error(format!("Failed to create proof temp directory: {e}")),
            elapsed_ms,
            stdout_bytes: 0,
            stderr_bytes: 0,
            truncated: false,
        };
    }

    // 2. Materialize temporary proof file
    let file_id = Uuid::new_v4();
    let file_path = proof_tmp_dir.join(format!("{file_id}.lean"));
    if let Err(e) = tokio::fs::write(&file_path, source).await {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        return VerifyOutcome {
            verdict: Verdict::Error(format!("Failed to write proof file: {e}")),
            elapsed_ms,
            stdout_bytes: 0,
            stderr_bytes: 0,
            truncated: false,
        };
    }

    // Guard guarantees deletion on any exit or unwind
    let _guard = TempFileGuard(file_path.clone());

    let abs_file_path = match std::fs::canonicalize(&file_path) {
        Ok(p) => p,
        Err(_) => file_path.clone(),
    };

    let lake_bin = find_lake_binary();
    let mut cmd = tokio::process::Command::new(lake_bin);
    let mut path_var = std::env::var("PATH").unwrap_or_default();
    if let Ok(home) = std::env::var("HOME") {
        let elan_bin = format!("{home}/.elan/bin");
        if !path_var.contains(&elan_bin) {
            path_var = format!("{elan_bin}:{path_var}");
        }
    }
    cmd.env("PATH", path_var);
    cmd.kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&lean_project_dir)
        .args(["env", "lean"])
        .arg(&abs_file_path);

    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            return VerifyOutcome {
                verdict: Verdict::Error(format!("Failed to spawn Lean process: {e}")),
                elapsed_ms,
                stdout_bytes: 0,
                stderr_bytes: 0,
                truncated: false,
            };
        }
    };

    let child_pid = child.id();
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_reader = async {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stdout_pipe.take() {
            let mut take = (&mut pipe).take((LEAN_STDOUT_MAX_BYTES + 1) as u64);
            let _ = take.read_to_end(&mut buf).await;
        }
        buf
    };

    let stderr_reader = async {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stderr_pipe.take() {
            let mut take = (&mut pipe).take((LEAN_STDERR_MAX_BYTES + 1) as u64);
            let _ = take.read_to_end(&mut buf).await;
        }
        buf
    };

    let execution_fut = async {
        let (status_res, stdout_raw, stderr_raw) =
            tokio::join!(child.wait(), stdout_reader, stderr_reader);
        (status_res, stdout_raw, stderr_raw)
    };

    let outcome = match tokio::time::timeout(config.lean_timeout, execution_fut).await {
        Err(_) => {
            // Timeout fired -> kill entire process group
            #[cfg(unix)]
            if let Some(pid) = child_pid {
                unsafe extern "C" {
                    fn kill(pid: i32, sig: i32) -> i32;
                }
                unsafe {
                    kill(-(pid as i32), 9); // SIGKILL = 9
                }
            }
            let _ = child.kill().await;

            let elapsed_ms = start.elapsed().as_millis() as u64;
            VerifyOutcome {
                verdict: Verdict::Rejected {
                    reason: RejectReason::TimedOut,
                    stderr: "Execution timed out".to_string(),
                },
                elapsed_ms,
                stdout_bytes: 0,
                stderr_bytes: 0,
                truncated: false,
            }
        }
        Ok((status_res, stdout_raw, stderr_raw)) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let mut truncated = false;

            let stdout_len = stdout_raw.len();
            let stderr_len = stderr_raw.len();

            let stdout_capped = if stdout_len > LEAN_STDOUT_MAX_BYTES {
                truncated = true;
                &stdout_raw[..LEAN_STDOUT_MAX_BYTES]
            } else {
                &stdout_raw[..]
            };

            let stderr_capped = if stderr_len > LEAN_STDERR_MAX_BYTES {
                truncated = true;
                &stderr_raw[..LEAN_STDERR_MAX_BYTES]
            } else {
                &stderr_raw[..]
            };

            let stdout_str = String::from_utf8_lossy(stdout_capped).to_string();
            let stderr_str = String::from_utf8_lossy(stderr_capped).to_string();

            let raw_exit = match status_res {
                Ok(s) => s.code().unwrap_or(-1),
                Err(e) => {
                    return VerifyOutcome {
                        verdict: Verdict::Error(format!("Failed to wait for process: {e}")),
                        elapsed_ms,
                        stdout_bytes: stdout_len,
                        stderr_bytes: stderr_len,
                        truncated,
                    };
                }
            };

            let (verdict, _reason) = parse_verdict(raw_exit, &stdout_str, &stderr_str);

            VerifyOutcome {
                verdict,
                elapsed_ms,
                stdout_bytes: stdout_len,
                stderr_bytes: stderr_len,
                truncated,
            }
        }
    };

    tracing::info!(
        elapsed_ms = outcome.elapsed_ms,
        verdict = ?outcome.verdict,
        stdout_bytes = outcome.stdout_bytes,
        stderr_bytes = outcome.stderr_bytes,
        truncated = outcome.truncated,
        "lean verify"
    );

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lean::wrap::wrap_problem;
    use std::collections::HashMap;

    fn test_config() -> Config {
        let env = HashMap::new();
        Config::from_map(&env).unwrap()
    }

    // (a) known-good proof -> Accepted
    #[tokio::test]
    async fn test_known_good_proof_accepted() {
        let config = test_config();
        let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
        let (source, _) =
            wrap_problem(&imports, "∀ n : ℕ, n + 0 = n", "  simp\n", &config).unwrap();

        let outcome = verify_once(&config, &source).await;
        assert!(
            matches!(outcome.verdict, Verdict::Accepted { .. }),
            "Expected Accepted, got {:?}",
            outcome.verdict
        );
    }

    // (b) sorry -> Rejected by the filter at wrap_problem level
    #[tokio::test]
    async fn test_sorry_rejected_soundness() {
        let config = test_config();
        let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];

        // The filter now catches sorry before it reaches Lean: wrap_problem rejects it
        let result = wrap_problem(&imports, "∀ n : ℕ, n + 0 = n", "  sorry\n", &config);
        assert!(
            matches!(result, Err(RejectReason::RejectedByFilter(_))),
            "Expected RejectedByFilter, got {:?}",
            result,
        );

        // Also verify that parse_verdict correctly detects sorry in Lean output
        // (belt and suspenders: the preamble's warningAsError catches it at Lean level too)
        let (verdict, _) = parse_verdict(1, "", "error: declaration uses 'sorry'");
        assert!(
            matches!(
                verdict,
                Verdict::Rejected {
                    reason: RejectReason::UsesSorry,
                    ..
                }
            ),
            "Expected UsesSorry from parse_verdict, got {:?}",
            verdict,
        );
    }

    // (c) loop -> Rejected(TimedOut) with elapsed_ms < timeout + 2000
    #[tokio::test]
    #[ignore]
    async fn test_timeout_rejection_and_kill() {
        let mut env = HashMap::new();
        env.insert("LEAN_TIMEOUT".to_string(), "2".to_string());
        let config = Config::from_map(&env).unwrap();

        let infinite_loop_source = r#"
import Mathlib.Data.Nat.Basic

partial def loop_forever (n : Nat) : Nat :=
  loop_forever (n + 1)

#eval loop_forever 0
"#;
        let outcome = verify_once(&config, infinite_loop_source).await;
        assert!(
            matches!(
                outcome.verdict,
                Verdict::Rejected {
                    reason: RejectReason::TimedOut,
                    ..
                }
            ),
            "Expected TimedOut, got {:?}",
            outcome.verdict
        );
        assert!(
            outcome.elapsed_ms < 4000,
            "Expected elapsed < 4000ms, got {}ms",
            outcome.elapsed_ms
        );
    }

    // (d) syntax error -> Rejected(LeaningFailed)
    // Use a valid tactic head with bad arguments: passes the filter, fails in Lean
    #[tokio::test]
    async fn test_syntax_error_rejected() {
        let config = test_config();
        let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
        let (source, _) =
            wrap_problem(&imports, "∀ n : ℕ, n + 0 = n", "  exact 42\n", &config).unwrap();

        let outcome = verify_once(&config, &source).await;
        assert!(
            matches!(
                outcome.verdict,
                Verdict::Rejected {
                    reason: RejectReason::LeaningFailed,
                    ..
                }
            ),
            "Expected LeaningFailed, got {:?}",
            outcome.verdict
        );
    }

    // (e) temp dir contains no .lean files afterwards
    #[tokio::test]
    async fn test_temp_file_cleanup_guarantee() {
        let mut env = HashMap::new();
        env.insert(
            "PROOF_TMP_DIR".to_string(),
            "lean/proofs_cleanup_test".to_string(),
        );
        let config = Config::from_map(&env).unwrap();

        let project_dir = resolve_project_dir(&config.lean_project_dir);
        let tmp_dir = resolve_proof_dir(&config, &project_dir);
        let _ = tokio::fs::create_dir_all(&tmp_dir).await;

        let count_lean_files = || {
            if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("lean"))
                    .count()
            } else {
                0
            }
        };

        let before = count_lean_files();

        let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
        let (source, _) =
            wrap_problem(&imports, "∀ n : ℕ, n + 0 = n", "  simp\n", &config).unwrap();
        let _ = verify_once(&config, &source).await;

        let after = count_lean_files();
        assert_eq!(
            before, after,
            "Temp files leaked: before={before}, after={after}"
        );

        let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
    }

    // (f) Non-blocking concurrency benchmark: 8 concurrent calls vs single call
    #[tokio::test]
    #[ignore]
    async fn test_concurrent_non_blocking_benchmark() {
        let config = test_config();
        let imports = vec!["import Mathlib.Data.Nat.Basic".to_string()];
        let (source, _) =
            wrap_problem(&imports, "∀ n : ℕ, n + 0 = n", "  simp\n", &config).unwrap();

        // 1. Single call measurement
        let t1_start = Instant::now();
        let single_outcome = verify_once(&config, &source).await;
        let single_duration = t1_start.elapsed();
        assert!(matches!(single_outcome.verdict, Verdict::Accepted { .. }));

        // 2. 8 Concurrent calls measurement
        let t8_start = Instant::now();
        let mut handles = Vec::new();
        for _ in 0..8 {
            let cfg = config.clone();
            let src = source.clone();
            handles.push(tokio::spawn(async move { verify_once(&cfg, &src).await }));
        }

        for handle in handles {
            let res = handle.await.unwrap();
            assert!(matches!(res.verdict, Verdict::Accepted { .. }));
        }
        let concurrent_duration = t8_start.elapsed();

        println!(
            "Single verify_once: {:?}, 8 concurrent verify_once: {:?}",
            single_duration, concurrent_duration
        );

        // Must finish in < 2x single call time (or within reasonable parallel bounds)
        assert!(
            concurrent_duration < single_duration.mul_f32(2.0),
            "Expected 8 concurrent calls ({:?}) to finish in < 2x single ({:?})",
            concurrent_duration,
            single_duration * 2
        );
    }
}
