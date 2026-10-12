use proof_battle_server::challenges::{self, ProofChallenge};
use proof_battle_server::config::Config;
use proof_battle_server::game::queue::VerifierPool;
use proof_battle_server::lean::{self, Verdict};
use proof_battle_server::telemetry::{AttemptTelemetryRecord, TelemetryLogger};
use proof_battle_server::ws::message::Diagnostic;
use rand::SeedableRng;
use rand::seq::IndexedRandom;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnchorType {
    Anchor0Random,
    Anchor1Heuristic,
    Anchor2Search,
}

impl AnchorType {
    fn name(&self) -> &'static str {
        match self {
            Self::Anchor0Random => "Anchor-0 (Random Trivial)",
            Self::Anchor1Heuristic => "Anchor-1 (Syntax Heuristic)",
            Self::Anchor2Search => "Anchor-2 (Search Baseline)",
        }
    }
}

fn generate_anchor0_candidates() -> Vec<&'static str> {
    vec!["rfl", "simp", "omega", "aesop", "decide", "trivial"]
}

fn generate_anchor1_candidates(goal: &str) -> Vec<&'static str> {
    if goal.contains('+')
        || goal.contains('-')
        || goal.contains('*')
        || goal.contains('≤')
        || goal.contains('<')
        || goal.contains('≥')
        || goal.contains('>')
        || goal.contains('ℕ')
        || goal.contains("Nat")
    {
        vec!["intro\nomega", "omega", "intro a b\nomega", "intro n\nsimp"]
    } else if goal.contains('∧') || goal.contains('∨') || goal.contains('¬') || goal.contains('→') {
        vec!["intro\naesop", "aesop", "intro\nconstructor <;> assumption"]
    } else if goal.contains("List") {
        vec!["intro\nsimp", "intro l\ncases l <;> simp"]
    } else {
        vec!["intro\nsimp", "rfl", "simp"]
    }
}

fn generate_anchor2_candidates(goal: &str) -> Vec<&'static str> {
    let mut list = Vec::new();
    list.push("rfl");
    list.push("intro\nrfl");
    list.push("intro a b\nrfl");
    list.push("intro\nsimp");
    list.push("intro n\nsimp");
    if goal.contains('+') || goal.contains('≤') || goal.contains('ℕ') || goal.contains('<') {
        list.push("intro\nomega");
        list.push("intro a b\nomega");
        list.push("omega");
    }
    list.push("intro\naesop");
    list.push("aesop");
    list
}

#[derive(Default)]
struct BenchMetrics {
    total: usize,
    solved: usize,
    solve_times_ms: Vec<u64>,
    category_total: HashMap<String, usize>,
    category_solved: HashMap<String, usize>,
    error_reasons: HashMap<String, usize>,
}

async fn evaluate_problem(
    bot_type: AnchorType,
    challenge: &ProofChallenge,
    config: &Config,
    pool: &VerifierPool,
    telemetry: &Arc<TelemetryLogger>,
    rng: &mut rand_chacha::ChaCha20Rng,
) -> (bool, u64, Option<String>) {
    let candidate_tactics: Vec<String> = match bot_type {
        AnchorType::Anchor0Random => {
            let pool = generate_anchor0_candidates();
            let chosen = pool.choose(rng).unwrap_or(&"simp");
            vec![chosen.to_string()]
        }
        AnchorType::Anchor1Heuristic => {
            let candidates = generate_anchor1_candidates(&challenge.goal);
            let chosen = candidates.choose(rng).unwrap_or(&"simp");
            vec![chosen.to_string()]
        }
        AnchorType::Anchor2Search => {
            generate_anchor2_candidates(&challenge.goal)
                .into_iter()
                .map(|s| s.to_string())
                .collect()
        }
    };

    let mut attempt_idx = 0;
    for tactic in candidate_tactics {
        attempt_idx += 1;
        let clean = tactic.trim();
        let wrapped = match lean::wrap_problem(&challenge.imports, &challenge.goal, clean, config) {
            Ok((w, _)) => w,
            Err(e) => {
                let err_msg = format!("Filter: {:?}", e);
                telemetry.record(AttemptTelemetryRecord {
                    match_id: format!("bench-{:?}", bot_type),
                    problem_id: challenge.id.clone(),
                    category: challenge.category.clone(),
                    structural_difficulty: challenge.difficulty,
                    goal: challenge.goal.clone(),
                    imports: challenge.imports.clone(),
                    player_id: bot_type.name().to_string(),
                    username: Some(bot_type.name().to_string()),
                    opponent_id: None,
                    attempt_index: attempt_idx,
                    intent: "Submit".to_string(),
                    tactic: clean.to_string(),
                    verdict: "Rejected".to_string(),
                    reject_reason: Some(err_msg.clone()),
                    diagnostics: vec![Diagnostic {
                        line: 1,
                        col: 1,
                        end_line: 1,
                        end_col: 1,
                        severity: proof_battle_server::ws::message::DiagnosticSeverity::Error,
                        message: err_msg.clone(),
                    }],
                    elapsed_ms: 0,
                    lean_ms: 0,
                    queue_wait_ms: 0,
                    time_budget_ms: 5000,
                    time_remaining_ms: 5000,
                    tokens_generated: None,
                    heartbeats_consumed: None,
                    outcome: None,
                    timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
                });
                return (false, 0, Some(err_msg));
            }
        };

        let processed = proof_battle_server::utils::preprocess(&wrapped);
        let start = Instant::now();
        let outcome_res = pool.run_submit(config, &processed).await;
        let elapsed_wall = start.elapsed().as_millis() as u64;

        let (verdict_str, reject_reason, lean_ms, diagnostics) = match outcome_res {
            Ok(outcome) => match outcome.verdict {
                Verdict::Accepted { .. } => ("Accepted", None, outcome.elapsed_ms, vec![]),
                Verdict::Rejected { reason, stderr } => {
                    let d = Diagnostic {
                        line: 1,
                        col: 1,
                        end_line: 1,
                        end_col: 1,
                        severity: proof_battle_server::ws::message::DiagnosticSeverity::Error,
                        message: stderr.clone(),
                    };
                    ("Rejected", Some(reason.as_str().to_string()), outcome.elapsed_ms, vec![d])
                }
                Verdict::Error(e) => {
                    let d = Diagnostic {
                        line: 1,
                        col: 1,
                        end_line: 1,
                        end_col: 1,
                        severity: proof_battle_server::ws::message::DiagnosticSeverity::Error,
                        message: e.clone(),
                    };
                    ("Error", Some(e), outcome.elapsed_ms, vec![d])
                }
            },
            Err(e) => ("Rejected", Some(e.as_str().to_string()), 0, vec![]),
        };

        telemetry.record(AttemptTelemetryRecord {
            match_id: format!("bench-{:?}", bot_type),
            problem_id: challenge.id.clone(),
            category: challenge.category.clone(),
            structural_difficulty: challenge.difficulty,
            goal: challenge.goal.clone(),
            imports: challenge.imports.clone(),
            player_id: bot_type.name().to_string(),
            username: Some(bot_type.name().to_string()),
            opponent_id: None,
            attempt_index: attempt_idx,
            intent: "Submit".to_string(),
            tactic: clean.to_string(),
            verdict: verdict_str.to_string(),
            reject_reason: reject_reason.clone(),
            diagnostics,
            elapsed_ms: elapsed_wall,
            lean_ms,
            queue_wait_ms: 0,
            time_budget_ms: 5000,
            time_remaining_ms: 5000,
            tokens_generated: None,
            heartbeats_consumed: None,
            outcome: None,
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        });

        if verdict_str == "Accepted" {
            return (true, lean_ms, None);
        }
    }

    (false, 0, Some("AllTacticsRejected".to_string()))
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_dry_run = args.iter().any(|a| a == "--dry-run");
    let limit = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<usize>().ok());

    println!("============================================================");
    println!("ProofBattle Stage 0: Anchor Bot Calibration Benchmark");
    println!("============================================================");

    let problems = challenges::get_problems();
    println!("Loaded {} problems from curated corpus.", problems.len());

    let active_problems: Vec<ProofChallenge> = match limit {
        Some(l) => {
            println!("Benchmarking limited to first {} problems.", l);
            problems.into_iter().take(l).collect()
        }
        None => problems,
    };

    if is_dry_run {
        println!("Dry run mode enabled. Validating problem metadata:");
        let mut cat_counts = HashMap::new();
        let mut diff_counts = HashMap::new();
        for p in &active_problems {
            *cat_counts.entry(p.category.clone()).or_insert(0) += 1;
            *diff_counts.entry(p.difficulty).or_insert(0) += 1;
        }
        println!("\nCategory Distribution:");
        for (cat, count) in &cat_counts {
            println!("  - {}: {}", cat, count);
        }
        println!("\nDifficulty Distribution:");
        for (diff, count) in &diff_counts {
            println!("  - Difficulty {}: {}", diff, count);
        }
        println!("\nDry run completed successfully. All problems loaded and validated.");
        return;
    }

    let config = Config::from_env().unwrap_or_else(|_| Config::from_map(&HashMap::new()).unwrap());
    let pool = Arc::new(VerifierPool::new(4, 2, Duration::from_millis(500)));
    let telemetry_path = PathBuf::from("telemetry_attempts.jsonl");
    let telemetry = TelemetryLogger::init(Some(telemetry_path));

    let bots = vec![
        AnchorType::Anchor0Random,
        AnchorType::Anchor1Heuristic,
        AnchorType::Anchor2Search,
    ];

    for bot in bots {
        println!("\n------------------------------------------------------------");
        println!("Evaluating {}", bot.name());
        println!("------------------------------------------------------------");

        let mut metrics = BenchMetrics::default();
        let mut rng = rand_chacha::ChaCha20Rng::from_seed([42u8; 32]);

        for (idx, challenge) in active_problems.iter().enumerate() {
            metrics.total += 1;
            *metrics.category_total.entry(challenge.category.clone()).or_insert(0) += 1;

            let (solved, lean_ms, reason) = evaluate_problem(
                bot,
                challenge,
                &config,
                &pool,
                &telemetry,
                &mut rng,
            )
            .await;

            if solved {
                metrics.solved += 1;
                metrics.solve_times_ms.push(lean_ms);
                *metrics.category_solved.entry(challenge.category.clone()).or_insert(0) += 1;
            } else if let Some(r) = reason {
                *metrics.error_reasons.entry(r).or_insert(0) += 1;
            }

            if (idx + 1) % 25 == 0 || idx + 1 == active_problems.len() {
                let pct = (metrics.solved as f64 / metrics.total as f64) * 100.0;
                println!("  [{}/{}] Solved: {} ({:.1}%)", idx + 1, active_problems.len(), metrics.solved, pct);
            }
        }

        let solve_pct = (metrics.solved as f64 / metrics.total as f64) * 100.0;
        let mut times = metrics.solve_times_ms.clone();
        times.sort_unstable();
        let median_ms = if times.is_empty() { 0 } else { times[times.len() / 2] };
        let p95_ms = if times.is_empty() { 0 } else { times[(times.len() * 95) / 100] };

        println!("\nSummary for {}:", bot.name());
        println!("  Solve Rate:       {}/{} ({:.2}%)", metrics.solved, metrics.total, solve_pct);
        println!("  Median Lean Time: {} ms", median_ms);
        println!("  P95 Lean Time:    {} ms", p95_ms);

        println!("  Category Performance:");
        for (cat, total) in &metrics.category_total {
            let solved_cat = metrics.category_solved.get(cat).copied().unwrap_or(0);
            let cat_pct = (solved_cat as f64 / *total as f64) * 100.0;
            println!("    - {}: {}/{} ({:.1}%)", cat, solved_cat, total, cat_pct);
        }
    }

    println!("\n============================================================");
    println!("Stage 0 Calibration Complete. Telemetry logged to telemetry_attempts.jsonl");
    println!("============================================================");
}
