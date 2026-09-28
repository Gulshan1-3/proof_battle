use crate::config::Config;
use crate::lean::executor::LeanExecutor;
use crate::lean::runner::{
    LEAN_STDERR_MAX_BYTES, LEAN_STDOUT_MAX_BYTES, RejectReason, Verdict, VerifyOutcome,
    parse_verdict, resolve_project_dir, resolve_proof_dir,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub struct SandboxExecutor;

impl LeanExecutor for SandboxExecutor {
    fn verify<'a>(
        &'a self,
        config: &'a Config,
        source: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = VerifyOutcome> + Send + 'a>> {
        Box::pin(async move { verify_sandboxed(config, source).await })
    }
}

pub fn build_docker_argv(
    config: &Config,
    abs_proof_path: &Path,
    job_id: Uuid,
    seccomp_path: &Path,
) -> Vec<String> {
    let timeout_secs = config.lean_timeout.as_secs().max(1);
    let memory_mb = config.lean_max_memory_mb;

    let seccomp_arg = format!("seccomp={}", seccomp_path.display());
    let volume_arg = format!("{}:/judge/proof.lean:ro", abs_proof_path.display());
    let job_label = format!("proofbattle.job={job_id}");
    let memory_arg = format!("{memory_mb}m");

    vec![
        "run".to_string(),
        "--rm".to_string(),
        "--network".to_string(),
        "none".to_string(),
        "--read-only".to_string(),
        "--tmpfs".to_string(),
        "/tmp:rw,noexec,nosuid,size=64m".to_string(),
        "--cap-drop".to_string(),
        "ALL".to_string(),
        "--security-opt".to_string(),
        "no-new-privileges".to_string(),
        "--pids-limit".to_string(),
        "256".to_string(),
        "--memory".to_string(),
        memory_arg.clone(),
        "--memory-swap".to_string(),
        memory_arg,
        "--cpus".to_string(),
        "2".to_string(),
        "--user".to_string(),
        "10001:10001".to_string(),
        "--ulimit".to_string(),
        "nofile=256:256".to_string(),
        "--ulimit".to_string(),
        "nproc=64:64".to_string(),
        "--ulimit".to_string(),
        "fsize=16m:16m".to_string(),
        "--security-opt".to_string(),
        seccomp_arg,
        "--label".to_string(),
        job_label,
        "-v".to_string(),
        volume_arg,
        config.sandbox_image.clone(),
        timeout_secs.to_string(),
    ]
}

pub fn resolve_seccomp_path() -> PathBuf {
    let candidate = PathBuf::from("infra/sandbox/seccomp.json");
    if candidate.exists() {
        return std::fs::canonicalize(&candidate).unwrap_or(candidate);
    }
    if let Ok(cargo_manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_parent = PathBuf::from(cargo_manifest);
        let cand = manifest_parent.join("../infra/sandbox/seccomp.json");
        if cand.exists() {
            return std::fs::canonicalize(&cand).unwrap_or(cand);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let cand = cwd.join("../infra/sandbox/seccomp.json");
        if cand.exists() {
            return std::fs::canonicalize(&cand).unwrap_or(cand);
        }
    }
    PathBuf::from("/home/gulshansharma/proof_battle/infra/sandbox/seccomp.json")
}

struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub async fn verify_sandboxed(config: &Config, source: &str) -> VerifyOutcome {
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

    let _guard = TempFileGuard(file_path.clone());

    let abs_file_path = match std::fs::canonicalize(&file_path) {
        Ok(p) => p,
        Err(_) => file_path.clone(),
    };

    let seccomp_path = resolve_seccomp_path();
    let args = build_docker_argv(config, &abs_file_path, file_id, &seccomp_path);

    let mut cmd = tokio::process::Command::new("docker");
    cmd.kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&args);

    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::error!(error = %e, "Failed to spawn Docker sandbox process");
            return VerifyOutcome {
                verdict: Verdict::Error(format!("Failed to spawn Docker sandbox: {e}")),
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

    // Execute with timeout — give Docker 5 extra seconds beyond lean_timeout
    let timeout_duration = config.lean_timeout + std::time::Duration::from_secs(5);
    let execution_result = tokio::time::timeout(timeout_duration, async {
        let (status, stdout, stderr) = tokio::join!(child.wait(), stdout_reader, stderr_reader);
        (status, stdout, stderr)
    })
    .await;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match execution_result {
        Ok((status_res, stdout_raw, stderr_raw)) => {
            let status = match status_res {
                Ok(s) => s,
                Err(e) => {
                    return VerifyOutcome {
                        verdict: Verdict::Error(format!("Failed waiting on Docker process: {e}")),
                        elapsed_ms,
                        stdout_bytes: stdout_raw.len(),
                        stderr_bytes: stderr_raw.len(),
                        truncated: false,
                    };
                }
            };

            let truncated = stdout_raw.len() > LEAN_STDOUT_MAX_BYTES
                || stderr_raw.len() > LEAN_STDERR_MAX_BYTES;

            let stdout_capped = &stdout_raw[..stdout_raw.len().min(LEAN_STDOUT_MAX_BYTES)];
            let stderr_capped = &stderr_raw[..stderr_raw.len().min(LEAN_STDERR_MAX_BYTES)];

            let stdout = String::from_utf8_lossy(stdout_capped).to_string();
            let stderr = String::from_utf8_lossy(stderr_capped).to_string();

            let raw_exit = status.code().unwrap_or(-1);
            let (verdict, _reason) = parse_verdict(raw_exit, &stdout, &stderr);

            tracing::info!(
                elapsed_ms,
                verdict = ?verdict,
                stdout_bytes = stdout_capped.len(),
                stderr_bytes = stderr_capped.len(),
                truncated,
                "sandboxed lean verify"
            );

            VerifyOutcome {
                verdict,
                elapsed_ms,
                stdout_bytes: stdout_capped.len(),
                stderr_bytes: stderr_capped.len(),
                truncated,
            }
        }
        Err(_) => {
            // Timed out: kill process group
            if let Some(pid) = child_pid {
                #[cfg(unix)]
                {
                    unsafe extern "C" {
                        fn kill(pid: i32, sig: i32) -> i32;
                    }
                    unsafe {
                        kill(-(pid as i32), 9); // SIGKILL
                    }
                }
            }
            let _ = child.kill().await;

            tracing::warn!(elapsed_ms, "sandboxed lean verification timed out");

            VerifyOutcome {
                verdict: Verdict::Rejected {
                    reason: RejectReason::TimedOut,
                    stderr: "Proof verification timed out".to_string(),
                },
                elapsed_ms,
                stdout_bytes: 0,
                stderr_bytes: 0,
                truncated: false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn invocation_argv_is_hardened() {
        let mut env = HashMap::new();
        env.insert("LEAN_TIMEOUT_MS".to_string(), "30000".to_string());
        env.insert("LEAN_MAX_MEMORY_MB".to_string(), "2048".to_string());
        env.insert(
            "SANDBOX_IMAGE".to_string(),
            "proofbattle/lean-sandbox:lean-4.21.0-rc3".to_string(),
        );
        let config = Config::from_map(&env).unwrap();

        let proof_path = PathBuf::from("/tmp/test_proof.lean");
        let seccomp_path = PathBuf::from("/tmp/seccomp.json");
        let job_id = Uuid::new_v4();

        let argv = build_docker_argv(&config, &proof_path, job_id, &seccomp_path);

        // Assert all critical security flags are present
        assert!(argv.contains(&"--network".to_string()));
        assert!(argv.contains(&"none".to_string()));
        assert!(argv.contains(&"--read-only".to_string()));
        assert!(argv.contains(&"--tmpfs".to_string()));
        assert!(argv.contains(&"/tmp:rw,noexec,nosuid,size=64m".to_string()));
        assert!(argv.contains(&"--cap-drop".to_string()));
        assert!(argv.contains(&"ALL".to_string()));
        assert!(argv.contains(&"--security-opt".to_string()));
        assert!(argv.contains(&"no-new-privileges".to_string()));
        assert!(argv.contains(&"--pids-limit".to_string()));
        assert!(argv.contains(&"256".to_string()));
        assert!(argv.contains(&"--memory".to_string()));
        assert!(argv.contains(&"2048m".to_string()));
        assert!(argv.contains(&"--memory-swap".to_string()));
        assert!(argv.contains(&"--cpus".to_string()));
        assert!(argv.contains(&"2".to_string()));
        assert!(argv.contains(&"--user".to_string()));
        assert!(argv.contains(&"10001:10001".to_string()));
        assert!(argv.contains(&"--ulimit".to_string()));
        assert!(argv.contains(&"nofile=256:256".to_string()));
        assert!(argv.contains(&"nproc=64:64".to_string()));
        assert!(argv.contains(&"fsize=16m:16m".to_string()));
        assert!(argv.iter().any(|arg| arg.starts_with("seccomp=")));
        assert!(argv.iter().any(|arg| arg.starts_with("proofbattle.job=")));
        assert!(argv.contains(&"-v".to_string()));
        assert!(argv.iter().any(|arg| arg.contains(":/judge/proof.lean:ro")));
        assert!(argv.contains(&"proofbattle/lean-sandbox:lean-4.21.0-rc3".to_string()));
    }
}
