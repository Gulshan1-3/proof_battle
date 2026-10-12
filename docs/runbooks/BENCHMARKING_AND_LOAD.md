# Runbook: Benchmarking & Load Generation

ProofBattle provides two built-in Rust performance measurement binaries in `server/src/bin/` to profile verification latency and simulated player concurrency.

---

## 1. Latency Profiling with `anchor_bench`

[`server/src/bin/anchor_bench.rs`](file:///home/gulshansharma/proof_battle/server/src/bin/anchor_bench.rs) measures the raw performance of the Lean 4 judge and execution sandbox.

### Running the Benchmark
```bash
cargo run --manifest-path server/Cargo.toml --bin anchor_bench -- --samples 30 --concurrency 4
```

### Key Metrics Measured
- **Cold Verification Latency**: Time required to spawn a new Lean 4 process without prewarmed caches (target: $<4.0\text{s}$).
- **Warm Verification Latency**: Time required when object caches are primed (target: $<500\text{ms}$).
- **Lexical Filter Latency**: Time to run AST and comment-stripping filters (target: $<0.5\text{ms}$).
- **p50, p95, p99 Latency Percentiles**: Identifies variance under concurrent worker load.

---

## 2. Real-Time Concurrency Testing with `loadgen`

[`server/src/bin/loadgen.rs`](file:///home/gulshansharma/proof_battle/server/src/bin/loadgen.rs) simulates realistic player traffic over WebSocket connections to validate the Tokio actor model (ADR-001) and the Two-Lane rate limiter (ADR-008).

### Running Simulated Battles
To launch a load test against a running server:
```bash
cargo run --manifest-path server/Cargo.toml --bin loadgen -- \
  --url ws://127.0.0.1:8080/ws \
  --players 100 \
  --duration 30
```

### Flags & Options
- `--url <URL>`: Target WebSocket endpoint (default: `ws://127.0.0.1:8080/ws`).
- `--players <N>`: Number of concurrent virtual players connecting to the server.
- `--duration <SEC>`: Duration of the test in seconds.
- `--check-rate <HZ>`: Frequency of background syntax checks per player (tests `CheckLane`).
- `--submit-interval <SEC>`: Interval between official match submissions (tests `SubmitLane`).

### Success Criteria
- **Connection Success**: $100\%$ of virtual player handshakes complete without socket drops.
- **Submit Lane Starvation**: $0\%$ of official match submissions dropped or timed out.
- **Server Memory**: Resident server memory remains stable without memory leaks.
