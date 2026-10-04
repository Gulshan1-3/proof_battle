use futures::{SinkExt, StreamExt};
use proof_battle_server::ws::message::{ClientMessage, ProofIntent, ServerMessage, VerdictStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct LoadgenConfig {
    url: String,
    clients: usize,
    games: usize,
    mode: String,
    timeout_secs: u64,
}

#[derive(Default)]
struct LatencyStats {
    connect_ms: Mutex<Vec<f64>>,
    match_wait_ms: Mutex<Vec<f64>>,
    check_ms: Mutex<Vec<f64>>,
    submit_ms: Mutex<Vec<f64>>,
    accepted_verdicts: AtomicUsize,
    rejected_verdicts: AtomicUsize,
    error_verdicts: AtomicUsize,
    completed_games: AtomicUsize,
    failed_games: AtomicUsize,
    total_connections: AtomicUsize,
    total_errors: AtomicUsize,
}

fn parse_args() -> LoadgenConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut config = LoadgenConfig {
        url: "ws://127.0.0.1:3000/ws".to_string(),
        clients: 10,
        games: 20,
        mode: "practice".to_string(),
        timeout_secs: 30,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                if i + 1 < args.len() {
                    config.url = args[i + 1].clone();
                    i += 1;
                }
            }
            "--clients" | "-c" => {
                if i + 1 < args.len() {
                    if let Ok(c) = args[i + 1].parse() {
                        config.clients = c;
                    }
                    i += 1;
                }
            }
            "--games" | "-n" => {
                if i + 1 < args.len() {
                    if let Ok(g) = args[i + 1].parse() {
                        config.games = g;
                    }
                    i += 1;
                }
            }
            "--mode" | "-m" => {
                if i + 1 < args.len() {
                    config.mode = args[i + 1].clone();
                    i += 1;
                }
            }
            "--timeout" | "-t" => {
                if i + 1 < args.len() {
                    if let Ok(t) = args[i + 1].parse() {
                        config.timeout_secs = t;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    config
}

struct PercentileSummary {
    count: usize,
    min: f64,
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    mean: f64,
}

fn calculate_percentiles(mut values: Vec<f64>) -> Option<PercentileSummary> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = values.len();
    let min = values[0];
    let max = values[count - 1];
    let sum: f64 = values.iter().sum();
    let mean = sum / (count as f64);

    let p50_idx = ((count as f64) * 0.50).floor() as usize;
    let p95_idx = ((count as f64) * 0.95).floor() as usize;
    let p99_idx = ((count as f64) * 0.99).floor() as usize;

    Some(PercentileSummary {
        count,
        min,
        p50: values[p50_idx.min(count - 1)],
        p95: values[p95_idx.min(count - 1)],
        p99: values[p99_idx.min(count - 1)],
        max,
        mean,
    })
}

async fn run_simulated_client(
    client_id: usize,
    config: LoadgenConfig,
    stats: Arc<LatencyStats>,
    games_dispatched: Arc<AtomicUsize>,
) {
    let timeout = Duration::from_secs(config.timeout_secs);

    loop {
        let game_num = games_dispatched.fetch_add(1, Ordering::SeqCst);
        if game_num >= config.games {
            break;
        }

        let run_result = tokio::time::timeout(timeout, async {
            let connect_start = Instant::now();
            let (ws_stream, _) = match connect_async(&config.url).await {
                Ok(stream) => {
                    stats.total_connections.fetch_add(1, Ordering::SeqCst);
                    stream
                }
                Err(err) => {
                    eprintln!("[Client {}] Failed to connect: {}", client_id, err);
                    stats.total_errors.fetch_add(1, Ordering::SeqCst);
                    stats.failed_games.fetch_add(1, Ordering::SeqCst);
                    return Err(());
                }
            };

            let (mut write, mut read) = ws_stream.split();

            // Handshake: Hello -> Welcome
            let hello = ClientMessage::Hello {
                version: 1,
                token: None,
                username: Some(format!("loadgen-{}-{}", client_id, game_num)),
            };
            let hello_json = serde_json::to_string(&hello).map_err(|_| ())?;
            write
                .send(Message::Text(hello_json))
                .await
                .map_err(|_| ())?;

            // Await Welcome
            while let Some(msg) = read.next().await {
                let msg = msg.map_err(|_| ())?;
                if let Message::Text(text) = msg
                    && let Ok(ServerMessage::Welcome { .. }) =
                        serde_json::from_str::<ServerMessage>(&text)
                {
                    break;
                }
            }

            let connect_elapsed = connect_start.elapsed().as_secs_f64() * 1000.0;
            stats.connect_ms.lock().await.push(connect_elapsed);

            // Join mode
            let join_start = Instant::now();
            if config.mode == "practice" {
                let join_msg = ClientMessage::PracticeJoin {};
                let json = serde_json::to_string(&join_msg).map_err(|_| ())?;
                write.send(Message::Text(json)).await.map_err(|_| ())?;
            } else {
                let join_msg = ClientMessage::QueueJoin {};
                let json = serde_json::to_string(&join_msg).map_err(|_| ())?;
                write.send(Message::Text(json)).await.map_err(|_| ())?;
            }

            // Await RoundStart
            let mut round_started = false;
            let mut problem_goal = String::new();
            while let Some(msg) = read.next().await {
                let msg = msg.map_err(|_| ())?;
                if let Message::Text(text) = msg
                    && let Ok(ServerMessage::RoundStart { problem, .. }) =
                        serde_json::from_str::<ServerMessage>(&text)
                {
                    round_started = true;
                    problem_goal = problem.goal;
                    break;
                }
            }

            if !round_started {
                stats.failed_games.fetch_add(1, Ordering::SeqCst);
                return Err(());
            }

            let match_wait_elapsed = join_start.elapsed().as_secs_f64() * 1000.0;
            stats.match_wait_ms.lock().await.push(match_wait_elapsed);

            // 1. Issue non-blocking Check request
            let check_req_id = Uuid::new_v4().to_string();
            let check_msg = ClientMessage::ProofRequest {
                req_id: check_req_id.clone(),
                code: "simp".to_string(),
                intent: ProofIntent::Check,
            };
            let check_json = serde_json::to_string(&check_msg).map_err(|_| ())?;
            let check_start = Instant::now();
            write
                .send(Message::Text(check_json))
                .await
                .map_err(|_| ())?;

            // Await Verdict for Check
            while let Some(msg) = read.next().await {
                let msg = msg.map_err(|_| ())?;
                if let Message::Text(text) = msg
                    && let Ok(ServerMessage::Verdict { req_id, .. }) =
                        serde_json::from_str::<ServerMessage>(&text)
                    && req_id == check_req_id
                {
                    let check_elapsed = check_start.elapsed().as_secs_f64() * 1000.0;
                    stats.check_ms.lock().await.push(check_elapsed);
                    break;
                }
            }

            // Small delay simulating proof typing
            tokio::time::sleep(Duration::from_millis(50)).await;

            // 2. Select proof code matching problem
            let proof_code = if problem_goal.contains("n + 0 = n") {
                "intro n\nrfl".to_string()
            } else if problem_goal.contains("a + b = b + a") {
                "intro a b\nsimp [Nat.add_comm]".to_string()
            } else if problem_goal.contains("P ∧ Q") {
                "intro P Q h\ncases h\nconstructor\nassumption\nassumption".to_string()
            } else {
                "intro n\nsimp".to_string()
            };

            // Issue Submit request
            let submit_req_id = Uuid::new_v4().to_string();
            let submit_msg = ClientMessage::ProofRequest {
                req_id: submit_req_id.clone(),
                code: proof_code,
                intent: ProofIntent::Submit,
            };
            let submit_json = serde_json::to_string(&submit_msg).map_err(|_| ())?;
            let submit_start = Instant::now();
            write
                .send(Message::Text(submit_json))
                .await
                .map_err(|_| ())?;

            // Await Verdict for Submit
            let mut was_accepted = false;
            while let Some(msg) = read.next().await {
                let msg = msg.map_err(|_| ())?;
                if let Message::Text(text) = msg
                    && let Ok(ServerMessage::Verdict {
                        req_id, verdict, ..
                    }) = serde_json::from_str::<ServerMessage>(&text)
                    && req_id == submit_req_id
                {
                    let submit_elapsed = submit_start.elapsed().as_secs_f64() * 1000.0;
                    stats.submit_ms.lock().await.push(submit_elapsed);
                    match verdict {
                        VerdictStatus::Accepted => {
                            stats.accepted_verdicts.fetch_add(1, Ordering::SeqCst);
                            was_accepted = true;
                        }
                        VerdictStatus::Rejected => {
                            stats.rejected_verdicts.fetch_add(1, Ordering::SeqCst);
                        }
                        VerdictStatus::Error => {
                            stats.error_verdicts.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    break;
                }
            }

            // If not accepted, resign so game finishes cleanly
            if !was_accepted {
                let resign_json =
                    serde_json::to_string(&ClientMessage::Resign {}).map_err(|_| ())?;
                write
                    .send(Message::Text(resign_json))
                    .await
                    .map_err(|_| ())?;
            }

            // Await RoundEnd or close
            while let Some(msg) = read.next().await {
                let msg = msg.map_err(|_| ())?;
                if let Message::Text(text) = msg
                    && let Ok(ServerMessage::RoundEnd { outcome, .. }) =
                        serde_json::from_str::<ServerMessage>(&text)
                {
                    let _ = outcome;
                    break;
                }
            }

            stats.completed_games.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;

        if run_result.is_err() {
            stats.total_errors.fetch_add(1, Ordering::SeqCst);
            stats.failed_games.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn print_metric_row(name: &str, stats: Option<PercentileSummary>) {
    match stats {
        Some(s) => {
            println!(
                "{:<20} {:<8} {:<8.1} {:<8.1} {:<8.1} {:<8.1} {:<8.1} {:<8.1}",
                name, s.count, s.min, s.p50, s.p95, s.p99, s.max, s.mean
            );
        }
        None => {
            println!(
                "{:<20} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8}",
                name, 0, "-", "-", "-", "-", "-", "-"
            );
        }
    }
}

#[tokio::main]
async fn main() {
    let config = parse_args();
    let stats = Arc::new(LatencyStats::default());
    let games_dispatched = Arc::new(AtomicUsize::new(0));

    println!("================================================================================");
    println!("PROOF BATTLE LOAD GENERATOR");
    println!("================================================================================");
    println!("Target URL:          {}", config.url);
    println!("Mode:                {}", config.mode);
    println!("Concurrent Clients:  {}", config.clients);
    println!("Target Games:        {}", config.games);
    println!("Operation Timeout:   {}s", config.timeout_secs);
    println!("--------------------------------------------------------------------------------");
    println!("Starting load test run...");

    let start_all = Instant::now();
    let mut handles = Vec::new();

    for client_id in 0..config.clients {
        let cfg = config.clone();
        let st = Arc::clone(&stats);
        let gd = Arc::clone(&games_dispatched);
        handles.push(tokio::spawn(async move {
            run_simulated_client(client_id, cfg, st, gd).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    let elapsed_total = start_all.elapsed();

    // Generate summaries
    let connect_summary = calculate_percentiles(stats.connect_ms.lock().await.clone());
    let match_wait_summary = calculate_percentiles(stats.match_wait_ms.lock().await.clone());
    let check_summary = calculate_percentiles(stats.check_ms.lock().await.clone());
    let submit_summary = calculate_percentiles(stats.submit_ms.lock().await.clone());

    println!();
    println!("================================================================================");
    println!("LOAD GENERATOR REPORT");
    println!("================================================================================");
    println!("Total Wall Time:     {:.2}s", elapsed_total.as_secs_f64());
    println!(
        "Total Connections:   {}",
        stats.total_connections.load(Ordering::SeqCst)
    );
    println!(
        "Completed Games:     {}",
        stats.completed_games.load(Ordering::SeqCst)
    );
    println!(
        "Failed Games:        {}",
        stats.failed_games.load(Ordering::SeqCst)
    );
    println!(
        "Total Errors:        {}",
        stats.total_errors.load(Ordering::SeqCst)
    );
    println!();
    println!("--------------------------------------------------------------------------------");
    println!("LATENCY STATISTICS (MILLISECONDS)");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<20} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8}",
        "Metric", "Count", "Min", "p50", "p95", "p99", "Max", "Mean"
    );
    println!("--------------------------------------------------------------------------------");
    print_metric_row("Connect Handshake", connect_summary);
    print_metric_row("Match Wait", match_wait_summary);
    print_metric_row("Check Latency", check_summary);
    print_metric_row("Submit Latency", submit_summary);
    println!("--------------------------------------------------------------------------------");
    println!();
    println!("--------------------------------------------------------------------------------");
    println!("VERDICTS SUMMARY");
    println!("--------------------------------------------------------------------------------");
    println!(
        "Accepted:            {}",
        stats.accepted_verdicts.load(Ordering::SeqCst)
    );
    println!(
        "Rejected:            {}",
        stats.rejected_verdicts.load(Ordering::SeqCst)
    );
    println!(
        "Error:               {}",
        stats.error_verdicts.load(Ordering::SeqCst)
    );
    println!("================================================================================");
}
