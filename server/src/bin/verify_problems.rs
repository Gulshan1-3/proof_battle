use chrono::Utc;
use proof_battle_server::config::Config;
use proof_battle_server::db::{create_pool, run_migrations};
use proof_battle_server::lean::runner::{Verdict, verify_once};
use proof_battle_server::lean::wrap::wrap_problem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemCandidate {
    pub slug: String,
    pub goal: String,
    pub imports: Vec<String>,
    pub statement: String,
    pub canonical_proof: String,
    pub difficulty: i16,
    pub category: String,
    pub tactic_hint: Option<String>,
    pub source_theorem: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug)]
pub enum CandidateVerificationResult {
    Verified {
        candidate: ProblemCandidate,
        elapsed_ms: u64,
    },
    Rejected {
        candidate: ProblemCandidate,
        reason: String,
    },
}

fn find_jsonl_file() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PROBLEMS_FILE") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        PathBuf::from("problems_curated.jsonl"),
        PathBuf::from("problems_import.jsonl"),
        PathBuf::from("../problems_curated.jsonl"),
        PathBuf::from("../problems_import.jsonl"),
        PathBuf::from("../../problems_curated.jsonl"),
        PathBuf::from("../../problems_import.jsonl"),
    ];
    candidates.into_iter().find(|c| c.exists())
}

async fn verify_candidate(
    config: &Config,
    candidate: ProblemCandidate,
) -> CandidateVerificationResult {
    // 1. Positive test: Canonical proof MUST be accepted
    let positive_wrap = wrap_problem(
        &candidate.imports,
        &candidate.goal,
        &candidate.canonical_proof,
        config,
    );

    let (wrapped_positive, _) = match positive_wrap {
        Ok(res) => res,
        Err(e) => {
            eprintln!(
                "[DEBUG] Candidate '{}' filter rejected: {:?}",
                candidate.slug, e
            );
            return CandidateVerificationResult::Rejected {
                candidate,
                reason: format!("filter_rejected_canonical: {:?}", e),
            };
        }
    };

    let outcome_positive = verify_once(config, &wrapped_positive).await;
    match outcome_positive.verdict {
        Verdict::Accepted { .. } => {}
        Verdict::Rejected { reason, stderr } => {
            let snippet = stderr.lines().take(3).collect::<Vec<_>>().join(" | ");
            eprintln!(
                "[DEBUG] Candidate '{}' rejected: {:?} => {}",
                candidate.slug, reason, snippet
            );
            return CandidateVerificationResult::Rejected {
                candidate,
                reason: format!("canonical_rejected: {:?} ({})", reason, snippet),
            };
        }
        Verdict::Error(e) => {
            return CandidateVerificationResult::Rejected {
                candidate,
                reason: format!("harness_error: {}", e),
            };
        }
    }

    // 2. Negative test: Deliberately broken proof MUST be rejected
    let negative_body = "  exact 42\n";
    let negative_wrap = wrap_problem(&candidate.imports, &candidate.goal, negative_body, config);

    let passes_negative = match negative_wrap {
        Err(_) => true, // Rejected by filter -> passes negative test
        Ok((wrapped_negative, _)) => {
            let outcome_negative = verify_once(config, &wrapped_negative).await;
            matches!(outcome_negative.verdict, Verdict::Rejected { .. })
        }
    };

    if !passes_negative {
        return CandidateVerificationResult::Rejected {
            candidate,
            reason: "negative_test_accepted_broken_proof".to_string(),
        };
    }

    CandidateVerificationResult::Verified {
        candidate,
        elapsed_ms: outcome_positive.elapsed_ms,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ProofBattle Problem Verifier & Seeder ===");

    let config = Arc::new(Config::from_env()?);

    let jsonl_path = match find_jsonl_file() {
        Some(p) => p,
        None => {
            eprintln!(
                "Error: problems_import.jsonl not found. Run scripts/extract_problems.py first."
            );
            std::process::exit(1);
        }
    };

    println!("Reading candidates from: {}", jsonl_path.display());
    let content = tokio::fs::read_to_string(&jsonl_path).await?;
    let mut candidates: Vec<ProblemCandidate> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Ok(c) = serde_json::from_str::<ProblemCandidate>(trimmed) {
            candidates.push(c);
        }
    }

    println!("Loaded {} candidates from JSONL.", candidates.len());

    // Connect to Postgres if available
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://proofbattle:proofbattle_dev@localhost:5432/proofbattle".to_string()
    });

    let pool = match create_pool(&db_url).await {
        Ok(p) => {
            println!("Connected to PostgreSQL at: {}", db_url);
            if let Err(e) = run_migrations(&p).await {
                eprintln!("Warning: Migrations failed: {}", e);
            } else {
                println!("Applied database migrations successfully.");
            }
            if std::env::args().any(|a| a == "--migrate-only") {
                println!("Migration-only mode requested. Exiting.");
                return Ok(());
            }
            Some(p)
        }
        Err(e) => {
            if std::env::args().any(|a| a == "--migrate-only") {
                eprintln!("Error: PostgreSQL connection failed: {}", e);
                std::process::exit(1);
            }
            println!(
                "Notice: PostgreSQL connection not available ({}). Running in verification-only mode.",
                e
            );
            None
        }
    };

    // Target count for seeding
    let target_verified = std::env::var("TARGET_VERIFIED")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(120);

    let concurrency = std::env::var("VERIFY_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(8);

    println!(
        "Starting concurrent verification (target: {} verified, concurrency: {})...",
        target_verified, concurrency
    );

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let verified_counter = Arc::new(AtomicUsize::new(0));
    let processed_counter = Arc::new(AtomicUsize::new(0));

    let mut tasks = Vec::new();

    // Diversify candidates across categories before processing
    // Group by category so we seed balanced categories
    let mut by_cat: HashMap<String, Vec<ProblemCandidate>> = HashMap::new();
    for c in candidates {
        by_cat.entry(c.category.clone()).or_default().push(c);
    }

    let mut interleaved = Vec::new();
    let mut max_in_cat = 0;
    for list in by_cat.values() {
        if list.len() > max_in_cat {
            max_in_cat = list.len();
        }
    }

    for idx in 0..max_in_cat {
        for list in by_cat.values() {
            if idx < list.len() {
                interleaved.push(list[idx].clone());
            }
        }
    }

    let total_to_scan = interleaved.len().min(target_verified * 8);

    for candidate in interleaved.into_iter().take(total_to_scan) {
        let sem = semaphore.clone();
        let cfg = config.clone();
        let verified_cnt = verified_counter.clone();
        let processed_cnt = processed_counter.clone();
        let target = target_verified;

        tasks.push(tokio::spawn(async move {
            if verified_cnt.load(Ordering::Relaxed) >= target {
                return None;
            }
            let _permit = sem.acquire().await.unwrap();
            if verified_cnt.load(Ordering::Relaxed) >= target {
                return None;
            }

            let res = verify_candidate(&cfg, candidate).await;
            processed_cnt.fetch_add(1, Ordering::Relaxed);
            if matches!(res, CandidateVerificationResult::Verified { .. }) {
                verified_cnt.fetch_add(1, Ordering::Relaxed);
            }
            Some(res)
        }));
    }

    let mut verified_records: Vec<ProblemCandidate> = Vec::new();
    let mut failure_breakdown: HashMap<String, usize> = HashMap::new();
    let mut diff_distribution: HashMap<i16, usize> = HashMap::new();
    let mut cat_distribution: HashMap<String, usize> = HashMap::new();

    for task in tasks {
        if let Ok(Some(res)) = task.await {
            match res {
                CandidateVerificationResult::Verified { candidate, .. } => {
                    *diff_distribution.entry(candidate.difficulty).or_insert(0) += 1;
                    *cat_distribution
                        .entry(candidate.category.clone())
                        .or_insert(0) += 1;
                    verified_records.push(candidate);
                }
                CandidateVerificationResult::Rejected { reason, .. } => {
                    let cat = reason.split(':').next().unwrap_or("other").to_string();
                    *failure_breakdown.entry(cat).or_insert(0) += 1;
                }
            }
        }
    }

    let total_processed = processed_counter.load(Ordering::Relaxed);
    let total_verified = verified_records.len();

    // Insert into PostgreSQL if connected
    if let Some(p) = pool {
        println!(
            "\nIngesting {} verified records into database...",
            total_verified
        );
        let now = Utc::now();
        for r in &verified_records {
            let res = sqlx::query(
                r#"
                INSERT INTO problems (
                    slug, goal, imports, statement, canonical_proof, difficulty, category,
                    tactic_hint, source_theorem, source_url, verified, verified_at, verification_error
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
                )
                ON CONFLICT (slug) DO UPDATE SET
                    goal = EXCLUDED.goal,
                    imports = EXCLUDED.imports,
                    statement = EXCLUDED.statement,
                    canonical_proof = EXCLUDED.canonical_proof,
                    difficulty = EXCLUDED.difficulty,
                    category = EXCLUDED.category,
                    tactic_hint = EXCLUDED.tactic_hint,
                    source_theorem = EXCLUDED.source_theorem,
                    source_url = EXCLUDED.source_url,
                    verified = EXCLUDED.verified,
                    verified_at = EXCLUDED.verified_at,
                    verification_error = EXCLUDED.verification_error
                "#,
            )
            .bind(&r.slug)
            .bind(&r.goal)
            .bind(&r.imports)
            .bind(&r.statement)
            .bind(&r.canonical_proof)
            .bind(r.difficulty)
            .bind(&r.category)
            .bind(&r.tactic_hint)
            .bind(&r.source_theorem)
            .bind(&r.source_url)
            .bind(true)
            .bind(Some(now))
            .bind(None::<String>)
            .execute(&p)
            .await;

            if let Err(e) = res {
                eprintln!("Failed to insert problem {}: {}", r.slug, e);
            }
        }
        println!("Database ingestion complete.");
    }

    // Print Summary Report
    println!("\n=======================================================");
    println!("               VERIFICATION & SEED REPORT              ");
    println!("=======================================================");
    println!("Total Candidates Processed: {}", total_processed);
    println!("Machine-Verified (Passed):  {}", total_verified);
    println!(
        "Rejected:                   {}",
        total_processed - total_verified
    );
    println!("-------------------------------------------------------");
    println!("Difficulty Distribution:");
    let diff_1_2: usize = (1..=2)
        .map(|d| diff_distribution.get(&d).copied().unwrap_or(0))
        .sum();
    let diff_3_4: usize = (3..=4)
        .map(|d| diff_distribution.get(&d).copied().unwrap_or(0))
        .sum();
    let diff_5_6: usize = (5..=6)
        .map(|d| diff_distribution.get(&d).copied().unwrap_or(0))
        .sum();
    let diff_7_10: usize = (7..=10)
        .map(|d| diff_distribution.get(&d).copied().unwrap_or(0))
        .sum();
    println!("  [1-2] (ELO 0-600)   : {}", diff_1_2);
    println!("  [3-4] (ELO 600-1000): {}", diff_3_4);
    println!("  [5-6] (ELO 1000-1400): {}", diff_5_6);
    println!("  [7-10](ELO 1400+)   : {}", diff_7_10);
    println!("-------------------------------------------------------");
    println!("Category Distribution:");
    for (cat, count) in &cat_distribution {
        println!("  {:<16}: {}", cat, count);
    }
    println!("-------------------------------------------------------");
    println!("Failure Breakdown:");
    for (reason, count) in &failure_breakdown {
        println!("  {:<30}: {}", reason, count);
    }
    println!("=======================================================\n");

    if total_verified < 10 {
        eprintln!("Warning: Verified problem count is below minimum threshold (10).");
    }

    Ok(())
}
