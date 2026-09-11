use std::{fs, process::Command, time::{Duration, Instant}};
use std::thread::sleep;

pub fn run_proof(code: &str, filename: &str) -> Result<String, String> {
    let path = format!("proofs/{}", filename);
    fs::write(&path, code).map_err(|e| e.to_string())?;

    let start = Instant::now();

    let output = Command::new("lake")
        .current_dir(".")
        .arg("env")
        .arg("lean")
        .arg(format!("proofs/{}", filename))
        .output();

    let duration = start.elapsed();
    println!("⏱️ Lean ran in: {:?}", duration);

    // Optional: Sleep to allow Lean server / editor indexer to catch up
    sleep(Duration::from_secs(2));

    let output = output.map_err(|e| format!("Failed to execute Lean: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

