# ProofBattle — Rust Backend Implementation Prompts

> **Source of truth**: `battle/proof_battle_design.md` and existing code in `battle/src/`
> These prompts are ordered by dependency. Step N cannot start until Step N-1 is complete.
> Every instruction below is grounded in the design document — no hallucinated APIs or features.

---

## Step 1: Restructure the Rust Module Layout

**Prompt:**

```
Restructure the existing Rust project at battle/src/ into the following module layout as specified
in proof_battle_design.md Section 6.1:

battle/src/
  main.rs          — server bootstrap only (move logic out)
  config.rs        — Config struct loaded from env vars
  ws/
    mod.rs         — re-exports handler and message
    handler.rs     — per-connection WebSocket loop (extracted from current main.rs)
    message.rs     — ClientMessage / ServerMessage enums (move from current message.rs)
  game/
    mod.rs
    matchmaker.rs  — Matchmaker struct (extracted from current room.rs)
    room.rs        — GameRoom, Match structs
    elo.rs         — placeholder for ELO/Glicko-2
  lean/
    mod.rs
    runner.rs      — async Lean subprocess (replace current lean_runner.rs)
    sandbox.rs     — input sanitization
    pool.rs        — worker pool (placeholder initially)
  db/
    mod.rs
    problems.rs    — placeholder for Postgres problem fetching
  error.rs         — AppError enum

Instructions:
1. Create all directories and mod.rs files.
2. Move existing code from src/main.rs into src/ws/handler.rs (the handler loop).
3. Move src/message.rs content into src/ws/message.rs.
4. Move src/room.rs content into src/game/matchmaker.rs and src/game/room.rs (split Matchmaker into its own file).
5. Move src/lean_runner.rs content into src/lean/runner.rs.
6. Create src/config.rs with a Config struct matching Section 6.4 (bind_addr, lean_timeout_secs,
   max_lean_workers, database_url, redis_url) loaded from env vars with defaults.
7. Create src/lean/sandbox.rs with the FORBIDDEN_PATTERNS list and sanitize_lean_code function
   from Section 6.5.
8. Create src/error.rs with an AppError enum that implements From<String> and From<anyhow::Error>.
9. Update src/main.rs to only: initialize tracing, load Config, build shared state, and call the
   Axum router.
10. Ensure `cargo check` passes after restructuring.

Do NOT add any new features in this step. Only restructure existing code into the new layout.
```

---

## Step 2: Replace Blocking Lean Execution with Async + Timeout

**Prompt:**

```
Rewrite src/lean/runner.rs to replace the current blocking std::process::Command with
tokio::process::Command and add a configurable timeout. This fixes the critical bug identified
in proof_battle_design.md Section 1.1 and 1.2.

The current code in lean_runner.rs does:
  Command::new("lake").arg("env").arg("lean").arg(path).output()  // BLOCKS
  sleep(Duration::from_secs(2))                                   // ARBITRARY

Replace with:
1. Use tokio::process::Command (async).
2. Use tokio::fs::write (async) to write the proof file to disk.
3. Wrap the entire Command execution in tokio::time::timeout with duration read from
   Config::lean_timeout_secs (default 30 seconds).
4. On timeout: return Err("Proof verification timed out (30s)").
5. On process spawn failure: return Err(format!("Failed to spawn Lean: {e}")).
6. On success (exit code 0): return Ok(stdout string).
7. On failure (exit code != 0): return Err(stderr string).
8. Remove the hardcoded sleep(2) entirely.

The function signature should be:
  pub async fn run_proof(code: &str, filename: &str, timeout_secs: u64) -> Result<String, String>

Ensure cargo check passes. Do NOT add Docker sandboxing yet (that is Step 8).
```

---

## Step 3: Implement Input Sanitization (Sandbox Module)

**Prompt:**

```
Implement src/lean/sandbox.rs based on proof_battle_design.md Section 6.5.

This module must prevent arbitrary code execution via submitted Lean proofs.

Requirements:
1. Define a const FORBIDDEN_PATTERNS array containing at minimum:
   "#eval", "#check IO", "unsafe", "System.Process",
   "IO.Process", "Lean.Environment", "@[extern",
   "native_decide"
2. Implement pub fn sanitize_lean_code(code: &str) -> Result<String, String>
   that:
   a. Checks the code against every pattern in FORBIDDEN_PATTERNS.
   b. If any pattern is found, returns Err with the message:
      "Forbidden construct: `{pattern}`" where pattern is the matched string.
   c. Checks that code.len() <= 4096. If exceeded, returns
      Err("Proof too long (max 4096 chars)").
   d. On success, returns Ok(code.to_string()).
3. Add a #[cfg(test)] module with tests:
   - Test that "intro n; simp" passes sanitization.
   - Test that "#eval IO.println \"hello\"" is rejected.
   - Test that "unsafe" is rejected.
   - Test that a 5000-char string is rejected.

Ensure cargo test passes.
```

---

## Step 4: Fix the Client-Provided player_id Trust Issue

**Prompt:**

```
Fix the critical security issue in proof_battle_design.md Section 1.5:
the client currently sends player_id in SubmitProof messages. The server must
assign and track identity.

Changes required:

1. In src/ws/message.rs, modify ClientMessage::SubmitProof to remove the player_id field:
   BEFORE: SubmitProof { code: String, player_id: String }
   AFTER:  SubmitProof { code: String }

2. In src/ws/handler.rs, when the WebSocket connection is established:
   a. Generate a UUID (use uuid crate v1 or v4) server-side as the player_id.
   b. Store the player_id in the per-connection state alongside the WebSocket sender channel.
   c. Send the player_id back to the client via ServerMessage::Joined { player_id }.

3. In the proof submission handler, look up the player_id from the connection state
   (NOT from the client message) and use it for:
   - The filename: "{player_id}_proof.lean"
   - The matchmaker submission

4. The client never sends their identity again after the initial connection.

5. Ensure cargo check passes.
```

---

## Step 5: Replace Arc<Mutex<HashMap>> with DashMap

**Prompt:**

```
Replace the global Arc<Mutex<HashMap>> on challenge_map with DashMap as specified
in proof_battle_design.md Section 1.4.

Changes:

1. Add "dashmap" as a dependency in battle/Cargo.toml (latest stable version).

2. In the shared application state (AppState or equivalent struct passed through Axum),
   change the challenge_map field from:
     Arc<Mutex<HashMap<String, ProofChallenge>>>
   to:
     Arc<DashMap<String, ProofChallenge>>

3. Update all call sites:
   - Remove .lock().await calls.
   - Use DashMap's .get(), .insert(), .remove() methods directly (they are lock-free per shard).

4. Also apply DashMap to the rooms map and match_states map in the Matchmaker if they are
   currently behind Arc<Mutex<>>.

5. Ensure cargo check passes. No behavioral changes — same semantics, better concurrency.
```

---

## Step 6: Implement Room Cleanup

**Prompt:**

```
Implement room cleanup as specified in proof_battle_design.md Section 1.9 and 6.6.

The current code creates rooms in mm.rooms and mm.match_states but never removes them,
causing unbounded memory growth.

Changes:

1. In src/game/matchmaker.rs, add a public method:
   pub fn cleanup_room(&mut self, room_id: &str)
   that removes the room_id from both self.rooms and self.match_states.

2. In the WebSocket handler (src/ws/handler.rs), after a GameEnded message is sent:
   a. Call mm.cleanup_room(&room_id) to remove the room from both maps.
   b. Use tracing::debug!(room_id = %room_id, "Room cleaned up") to log the cleanup.

3. Also handle the disconnect case: if a player disconnects during an active game
   (the WebSocket closes without GameEnded), the opponent should receive a
   ServerMessage::GameEnded { winner: opponent_id } so they are not stuck waiting.
   Then clean up the room.

4. Ensure cargo check passes.
```

---

## Step 7: Remove Unused Dependency and Replace println! with tracing

**Prompt:**

```
Fix two issues from proof_battle_design.md Sections 1.10 and 6.3:

Part A — Remove unused dependency:
1. Open battle/Cargo.toml.
2. Remove the tokio-tungstenite dependency (axum's ws feature already wraps tungstenite).
3. Remove any use statements referencing tokio_tungstenite in the codebase.
4. Run cargo check to confirm nothing depends on it.

Part B — Replace all println!() with tracing:
1. Add to Cargo.toml:
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter"] }
2. In src/main.rs, initialize the subscriber:
   tracing_subscriber::fmt()
     .with_env_filter("proof_battle=debug,tower_http=info")
     .init();
3. Find every println!() in the codebase and replace with the appropriate tracing macro:
   - println!("Game started for {}", room_id) → tracing::info!(room_id = %room_id, "Game started")
   - println!("Error: {}", e) → tracing::error!(err = %e, "Operation failed")
   - Debug-level logs for routine operations
4. Ensure cargo check passes.
```

---

## Step 8: Implement Docker-Based Sandboxing for Lean

**Prompt:**

```
Implement the Docker-based sandboxing for Lean proof execution as specified
in proof_battle_design.md Section 3 (Lean Worker Pool Design) and Section 8 (Security checklist).

This step does NOT implement the full pre-warmed worker pool. It wraps the existing
run_proof function to execute inside a Docker container.

Requirements:

1. Create a Dockerfile at battle/Dockerfile.lean-worker:
   - Base image: ubuntu:22.04
   - Install: curl, git
   - Install elan + lean 4 (leanprover/lean4:v4.21.0-rc3)
   - Copy the lake-manifest.json and lean-toolchain from battle/ into the container
   - Run `lake build Mathlib` with the .lake/build cache mounted as a volume
     (this MUST NOT build from scratch — use a pre-built .olean cache volume)
   - The container entrypoint should be a shell that accepts a filename argument
     and runs: lake env lean <filename>

2. Create a docker-compose.yml (or equivalent config) at battle/ that:
   - Maps the battle/proofs/ directory as a volume into the container
   - Sets no network (--network=none)
   - Sets read-only root filesystem with a writable /tmp
   - Drops all capabilities except the minimum needed

3. Modify src/lean/runner.rs to optionally run inside Docker:
   - Add a config flag: lean_use_docker: bool (default: false for dev)
   - When true: use tokio::process::Command to run "docker run" with the sandboxed image
   - When false: run lake env lean directly (for local development)
   - In both cases, apply the 30-second timeout from Step 2.

4. Ensure cargo check passes.
5. Manually test with: docker run --rm --network=none --read-only -v $(pwd)/proofs:/proofs lean-worker /proofs/test.lean
```

---

## Step 9: Implement ELO Rating System

**Prompt:**

```
Implement the ELO/Glicko-2 rating system as specified in proof_battle_design.md Section 5
(difficulty calibration) and Section 8 (Product checklist).

This is the src/game/elo.rs module.

Requirements:

1. Define an EloRating struct:
   pub struct EloRating {
       pub rating: f64,       // default 1000.0
       pub rd: f64,           // rating deviation, default 350.0 (Glicko-2)
       pub games_played: u32,
   }

2. Implement the Glicko-2 rating update algorithm:
   - Input: two players' ratings, game result (win/loss/draw)
   - Output: updated ratings for both players
   - Use the standard Glicko-2 formulas (τ = 0.5, system constant)
   - This does NOT need a database yet — use an in-memory DashMap<String, EloRating>
     keyed by player_id as a placeholder.

3. In src/game/matchmaker.rs:
   - When a game ends, call the ELO update function with both players' ratings.
   - Calculate elo_delta for both players (the change from before to after).
   - Include elo_delta in the ServerMessage::GameEnded message.

4. Update src/ws/message.rs ServerMessage::GameEnded to include elo_delta: f64.

5. Difficulty calibration (Section 5):
   - ELO 0–600: omega, decide, simp one-liner proofs (difficulty 1-3)
   - ELO 600–1000: ring, norm_num, 2-3 tactic proofs (difficulty 4-6)
   - ELO 1000–1400: linarith, nlinarith, induction (difficulty 7-8)
   - ELO 1400+: multi-step with have, obtain, cases (difficulty 9-10)
   This logic will be used when selecting problems (Step 11). For now, just store
   the difficulty range per ELO bracket as constants.

6. Ensure cargo check passes.
```

---

## Step 10: Add Session Tokens and Reconnection Support

**Prompt:**

```
Implement reconnection with session tokens as specified in proof_battle_design.md Section 1.6.

Requirements:

1. In src/ws/handler.rs, when a new WebSocket connection is established and the player
   is assigned a player_id:
   a. Generate a session token (UUID v4 or random 32-byte hex string).
   b. Store the session token → player_id mapping in a DashMap with a 60-second TTL.
      (For now, use an in-memory DashMap with a manual timestamp check. Redis comes later.)
   c. Send ServerMessage::SessionToken { token } to the client.

2. When a WebSocket connection arrives with a query parameter ?token=<token>:
   a. Look up the token in the session store.
   b. If found and not expired (< 60 seconds): restore the player to their existing room.
      Re-associate the new WebSocket sender with the room. Send them the current game state
      (challenge goal, opponent status).
   c. If not found or expired: treat as a new connection (assign new player_id).

3. In src/ws/message.rs, add:
   ServerMessage::SessionToken { token: String }

4. The client-side logic for storing and sending the token is NOT in scope for this step.
   Just ensure the server handles it.

5. Ensure cargo check passes.
```

---

## Step 11: Replace Hardcoded Problems with a Database

**Prompt:**

```
Replace the 3 hardcoded proof challenges with PostgreSQL-backed problem storage,
as specified in proof_battle_design.md Section 5 and Section 8.

Requirements:

1. Add to Cargo.toml:
   sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "uuid"] }

2. Create a migration file (battle/migrations/001_problems.sql):
   CREATE TABLE problems (
       id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
       goal TEXT NOT NULL,
       imports TEXT[] NOT NULL,
       difficulty SMALLINT CHECK (difficulty BETWEEN 1 AND 10),
       category TEXT,
       tactic_hint TEXT,
       source_theorem TEXT,
       verified BOOLEAN DEFAULT false,
       created_at TIMESTAMPTZ DEFAULT now()
   );
   CREATE INDEX ON problems (difficulty, category);

3. Implement src/db/problems.rs:
   pub async fn get_random_problem(
       pool: &PgPool,
       min_difficulty: i16,
       max_difficulty: i16,
   ) -> Result<ProofChallenge, sqlx::Error>
   that queries a random verified problem within the difficulty range.

4. Modify the game start flow in src/ws/handler.rs:
   a. After two players are matched, look up their ELO ratings.
   b. Determine difficulty range based on their average ELO (using the brackets from Step 9).
   c. Fetch a problem from the database using get_random_problem.
   d. If the database is unreachable, fall back to the 3 hardcoded problems (graceful degradation).
   e. Send the problem as ServerMessage::Challenge { goal, imports }.

5. In src/config.rs, ensure DATABASE_URL is read from env vars (required, no default).

6. Ensure cargo check passes. The database connection can be tested manually with
   DATABASE_URL=postgres://... cargo run.
```

---

## Step 12: Extract Theorems from Mathlib4 into the Database

**Prompt:**

```
Create a one-time extraction script to populate the problems database with theorems
from Mathlib4, as specified in proof_battle_design.md Section 5, Source 1.

Requirements:

1. Create a standalone Rust binary (or Python script — your choice, but keep it in the
   repo under battle/scripts/) called extract_theorems.

2. The script should:
   a. Clone or use the existing mathlib4/ directory at battle/mathlib4/.
   b. Use ripgrep (rg) or a regex parser to extract all lines matching:
      ^theorem <name> : <type>
      ^lemma <name> : <type>
      from files under battle/mathlib4/Mathlib/.
   c. Filter out any that contain "sorry" in the proof.
   d. For each extracted theorem:
      - Determine the category from the file path:
        Mathlib/Algebra/* → "algebra"
        Mathlib/Analysis/* → "analysis"
        Mathlib/Combinatorics/* → "combinatorics"
        Mathlib/Logic/* → "logic"
        etc.
      - Estimate difficulty by proof length (lines between the theorem statement and
        the next theorem/end-of-file):
        ≤ 3 tactics → difficulty 2-3
        4-8 tactics → difficulty 5-7
        9+ tactics → difficulty 8-10
      - Derive the import statement from the file path:
        File: battle/mathlib4/Mathlib/Algebra/Group/Basic.lean
        Import: Mathlib.Algebra.Group.Basic
   e. Insert into the problems table via a SQL INSERT statement (or generate a CSV
      that can be imported with psql \copy).

3. Generate at least 500 problems across all difficulty levels.

4. Store the output in battle/scripts/extracted_problems.csv with columns:
   goal, imports, difficulty, category, source_theorem

5. Document how to import: psql $DATABASE_URL -c "\copy problems(goal,imports,difficulty,category,source_theorem) FROM 'extracted_problems.csv' CSV HEADER"
```

---

## Step 13: Add Rate Limiting for Proof Submissions

**Prompt:**

```
Add rate limiting for proof submissions as specified in proof_battle_design.md Section 8
(Security checklist: "Rate limit proof submissions, max 5/min per player").

Requirements:

1. Add to Cargo.toml:
   tower = { version = "0.4", features = ["full"] }
   tower-http = { version = "0.5", features = ["cors", "trace"] }
   (Note: these may already be present for Axum middleware.)

2. Create src/ws/rate_limiter.rs:
   - A struct RateLimiter that tracks per-player submission timestamps.
   - Use a DashMap<String, Vec<Instant>> where the key is player_id and the value
     is a list of submission timestamps in the last minute.
   - Method: pub fn check_rate_limit(&self, player_id: &str) -> bool
     Returns true if the player has submitted < 5 times in the last 60 seconds.
     Returns false (and does NOT add the timestamp) if they are at the limit.
   - Method: pub fn record_submission(&self, player_id: &str)
     Adds the current timestamp to the player's list, prunes entries older than 60s.

3. In the proof submission handler (ws/handler.rs), BEFORE calling run_proof:
   a. Call rate_limiter.check_rate_limit(player_id).
   b. If false: send ServerMessage::Error { message: "Rate limit exceeded. Max 5 submissions per minute." }
      and return without processing.
   c. If true: call rate_limiter.record_submission(player_id), then proceed with verification.

4. Pass the RateLimiter through Axum state (Arc<RateLimiter>).

5. Ensure cargo check passes.
```

---

## Step 14: Handle Opponent Disconnect Gracefully

**Prompt:**

```
Implement graceful handling of opponent disconnects as specified in
proof_battle_design.md Section 8 (Correctness checklist).

Requirements:

1. In src/ws/handler.rs, detect when a WebSocket connection closes
   (the recv loop returns None or an error).

2. When a player disconnects during an active game:
   a. Look up their room_id from the connection state.
   b. Find the opponent's player_id from the room.
   c. Send ServerMessage::GameEnded { winner: opponent_id } to the opponent's
      WebSocket sender.
   d. Update the ELO ratings (opponent wins).
   e. Call cleanup_room(room_id) to free memory.

3. When a player disconnects during matchmaking (before a match is found):
   a. Remove them from the matchmaking queue.
   b. No further action needed.

4. When a player disconnects after a game has already ended:
   a. Room is already cleaned up. No action needed.

5. Add a tracing::info log for each disconnect scenario.

6. Ensure cargo check passes.
```

---

## Step 15: Write Integration Tests for the Proof Verification Pipeline

**Prompt:**

```
Write integration tests for the proof verification pipeline as specified in
proof_battle_design.md Section 8 (Developer Experience checklist).

Requirements:

1. Create battle/tests/proof_verification.rs.

2. Test cases:

   a. test_valid_proof_accepted:
      - Submit a valid proof ("exact Nat.add_zero n") for the goal "∀ n : ℕ, n + 0 = n".
      - Assert: run_proof returns Ok.

   b. test_invalid_proof_rejected:
      - Submit an invalid proof ("sorry") for the same goal.
      - Assert: run_proof returns Err containing "unsolved goals" or similar Lean error.

   c. test_timeout_enforced:
      - Submit a proof that loops forever (e.g., "repeat skip" if it compiles, or a
        very large proof that takes > 30s).
      - Assert: run_proof returns Err containing "timed out".

   d. test_sanitization_blocks_forbidden:
      - Call sanitize_lean_code with "#eval IO.println \"hi\"".
      - Assert: returns Err containing "Forbidden construct".

   e. test_sanitization_allows_valid:
      - Call sanitize_lean_code with "intro n; simp".
      - Assert: returns Ok.

   f. test_file_written_to_disk:
      - Call run_proof with a valid proof.
      - Assert: the file proofs/{filename} exists on disk after execution.

3. Use tempfile or a temp directory for proof files (do NOT write to the real proofs/ dir in tests).

4. These tests require Lean to be installed. Mark them with #[ignore] by default and
   provide instructions to run with: cargo test -- --ignored

5. Ensure cargo test passes (non-ignored tests).
```

---

> **After completing all 15 steps**, the Rust backend will have:
> - Proper async Lean execution with timeout and Docker sandboxing
> - Input sanitization against arbitrary code execution
> - Server-assigned player identity (no client trust)
> - DashMap-based concurrent state (no global mutex)
> - Room cleanup preventing memory leaks
> - Structured logging with tracing
> - Glicko-2 ELO rating system
> - Session-based reconnection
> - PostgreSQL-backed problem storage with 500+ extracted Mathlib theorems
> - Rate limiting at 5 submissions/minute
> - Graceful disconnect handling
> - Integration tests for the core pipeline
