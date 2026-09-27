# ProofBattle — Rust Backend Implementation Prompts

> **Source of truth**: `battle/proof_battle_design.md` (707 lines)
> **Existing backend**: Working Rust Axum WebSocket server at `battle/src/` (~390 lines across 5 modules)
> **Lean version**: `leanprover/lean4:v4.21.0-rc3` (from `battle/lean-toolchain`)

---

## Known Requirements

Requirements directly confirmed by `proof_battle_design.md`:

- **Async Lean execution** via `tokio::process::Command` with 30-second timeout (Section 1.1, 1.2)
- **Input sanitization** blocking `#eval`, `unsafe`, `IO`, `System.Process`, `native_decide` (Section 1.3, 6.5)
- **DashMap** replacing `Arc<Mutex<HashMap>>` for shared state (Section 1.4)
- **Server-assigned player identity** — client never sends `player_id` (Section 1.5)
- **Room cleanup** after game ends (Section 1.9, 6.6)
- **Remove unused `tokio-tungstenite`** dependency (Section 1.10)
- **Replace `println!` with `tracing`** (Section 6.3)
- **Config via environment variables** (Section 6.4)
- **Modular file structure** (Section 6.1)
- **Docker sandboxing** for Lean execution (Section 3)
- **PostgreSQL** for problem storage (Section 5)
- **ELO/Glicko-2 rating system** (Section 5, 8)
- **Session tokens** for reconnection (Section 1.6)
- **Rate limiting** at 5 submissions/minute per player (Section 8)
- **Graceful opponent disconnect** handling (Section 8)
- **Integration tests** for the proof verification pipeline (Section 8)

## Existing Architecture

**`battle/src/main.rs`** (207 lines):
- Axum server on `127.0.0.1:3000`
- Single `handle_socket` function handling the entire WebSocket lifecycle
- Uses `Arc<Mutex<Matchmaker>>` and `Arc<Mutex<HashMap<String, ProofChallenge>>>`
- Generates `player_id` server-side (UUID v4) — but `SubmitProof` message still expects client to send it
- Calls `lean_runner::run_proof` synchronously
- No error handling (unwrap everywhere)
- No room cleanup

**`battle/src/message.rs`** (21 lines):
- `ClientMessage`: `Join { username }`, `SubmitProof { code, player_id }`
- `ServerMessage`: `Joined { player_id }`, `MatchFound { opponent }`, `Challenge { goal, imports }`, `ProofResult { success, output }`, `GameEnded { winner }`, `Error { message }`

**`battle/src/room.rs`** (101 lines):
- `Matchmaker` with `waiting: Option<(PlayerId, Sender)>`, `rooms: HashMap`, `match_states: HashMap`
- `add_player` returns `Option<(room_id, opponent_id, opponent_tx)>`
- `declare_winner` uses `solved` flag for first-come-first-served

**`battle/src/lean_runner.rs`** (31 lines):
- Blocking `std::process::Command` calling `lake env lean`
- Hardcoded `sleep(Duration::from_secs(2))`
- No timeout

**`battle/src/challenges.rs`** (22 lines):
- `ProofChallenge { goal, imports }` struct
- 3 hardcoded problems

**`battle/src/utils.rs`** (7 lines):
- ASCII-to-Unicode preprocessing (`>=` → `≥`, `<=` → `≤`, `->` → `→`, `=>` → `⇒`)

**`battle/Cargo.toml`**:
- Dependencies: axum (ws), tokio (full), tokio-tungstenite (UNUSED), futures, serde/serde_json, uuid, rand, rand_chacha, rand_core

## Assumptions

1. ASSUMPTION: The `proofs/` directory at `battle/proofs/` is the working directory for proof files. The Lean runner writes to `proofs/{player_id}_proof.lean`. This directory already exists and contains 4 proof files from previous games.
2. ASSUMPTION: The `lake env lean` command must be run from the `battle/` directory (the `lakefile.lean` is there). The current code uses `Command::new("lake").current_dir(".")` which assumes CWD is `battle/`.
3. ASSUMPTION: The PostgreSQL database schema for problems is as specified in Section 5 of the design doc. The exact database connection string will come from the `DATABASE_URL` environment variable.
4. ASSUMPTION: Docker is available on the deployment machine for sandboxing. For local development, Docker sandboxing will be disabled via config flag.
5. ASSUMPTION: The `edition = "2024"` in Cargo.toml refers to Rust edition 2024. This is valid for recent Rust toolchains.

## Open Questions

1. **Lean worker pool vs. cold spawn**: The design doc recommends a pre-warmed worker pool of persistent `lean --server` LSP processes. This is a significant architectural change. The simpler approach (async subprocess with timeout) is recommended first. The worker pool can be added later.
2. **PostgreSQL vs. SQLite**: The design doc specifies PostgreSQL. For initial development, SQLite could work without external dependencies. However, the design doc is explicit about PostgreSQL, so we follow it.
3. **Redis for sessions**: The design doc mentions Redis for session tokens (enables horizontal scaling). For initial implementation, in-memory DashMap with TTL is sufficient. Redis can be added later.
4. **ELO persistence**: The design doc shows ELO ratings but doesn't specify storage. ASSUMPTION: store in PostgreSQL (new table) for persistence across restarts.

---

## Step 1 — Restructure the Rust Module Layout

### Goal

Reorganize the existing flat module structure into the modular layout specified in `proof_battle_design.md` Section 6.1, without changing any behavior.

### Before You Start

Inspect:

- `battle/src/main.rs` — the entry point with all logic
- `battle/src/room.rs` — the Matchmaker, GameRoom, Match structs
- `battle/src/message.rs` — the message types
- `battle/src/lean_runner.rs` — the Lean runner
- `battle/src/challenges.rs` — the challenge definitions
- `battle/src/utils.rs` — the preprocessing utility
- `battle/Cargo.toml` — dependencies

Confirm:

- All modules compile: run `cd battle && cargo check`
- The current structure is flat (all `.rs` files directly in `src/`)
- `main.rs` contains the server bootstrap AND the WebSocket handler

Do not make changes until you have confirmed the current code compiles.

### Context

The design doc Section 6.1 specifies a modular layout with `ws/`, `game/`, `lean/`, `db/` subdirectories. The current code has everything flat in `src/`. This step reorganizes without changing behavior, making subsequent steps cleaner.

### Implementation Instructions

1. Read all existing source files to understand the current code.
2. Run `cd battle && cargo check` to confirm the current code compiles.
3. Create the following directory structure:
   ```
   battle/src/
     ws/
       mod.rs
       handler.rs
       message.rs
     game/
       mod.rs
       matchmaker.rs
       room.rs
     lean/
       mod.rs
       runner.rs
     error.rs
   ```
4. Move `src/message.rs` → `src/ws/message.rs`
5. Move `src/room.rs` content:
   - `Matchmaker` struct → `src/game/matchmaker.rs`
   - `GameRoom` and `Match` structs → `src/game/room.rs`
   - Keep `PlayerId` type alias in `src/game/mod.rs`
6. Move `src/lean_runner.rs` → `src/lean/runner.rs`
7. Create `src/ws/mod.rs` that re-exports `handler` and `message`
8. Create `src/game/mod.rs` that re-exports `matchmaker` and `room`
9. Create `src/lean/mod.rs` that re-exports `runner`
10. Create `src/error.rs` with a basic `AppError` enum:
    ```rust
    pub enum AppError {
        LeanRunner(String),
        Sanitization(String),
        Io(String),
    }
    impl std::fmt::Display for AppError { ... }
    impl std::error::Error for AppError {}
    impl From<String> for AppError { ... }
    ```
11. Update `src/main.rs`:
    - Remove `mod message; mod lean_runner; mod utils; mod challenges;`
    - Add `mod ws; mod game; mod lean; mod error; mod utils; mod challenges;`
    - Move the `handle_socket` function to `src/ws/handler.rs`
    - Keep only the server bootstrap in `main.rs`
12. Run `cargo check` to confirm compilation.

### Technical Requirements

- All existing functionality must be preserved
- No behavioral changes — only code reorganization
- All `use` statements must be updated to reflect new module paths
- `challenges.rs` and `utils.rs` stay in `src/` (they are small utilities)

### Edge Cases

- If a `use` statement references a moved module, update the path
- If `main.rs` references types from moved modules, update the imports

### Constraints

- Do not modify any logic — only move code and update imports
- Do not add new dependencies
- Do not change public APIs
- Preserve all existing `#[allow(dead_code)]` attributes

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- Compilation succeeds with no errors
- `cargo clippy` produces no new warnings (existing warnings are acceptable)

### Completion Criteria

This step is complete when:

- The module structure matches Section 6.1
- `cargo check` passes
- All existing functionality is preserved (no behavioral changes)

### Expected Output

At the end of this step, the project should have:

- Modular directory structure under `src/`
- `main.rs` containing only server bootstrap
- `src/ws/handler.rs` containing the WebSocket handler
- `src/ws/message.rs` containing message types
- `src/game/matchmaker.rs` and `src/game/room.rs` containing game logic
- `src/lean/runner.rs` containing the Lean runner
- `src/error.rs` with AppError type

---

## Step 2 — Replace Blocking Lean Execution with Async + Timeout

### Goal

Replace the blocking `std::process::Command` in `lean_runner.rs` with `tokio::process::Command` and add a configurable 30-second timeout, fixing the critical issues in Sections 1.1 and 1.2.

### Before You Start

Inspect:

- `battle/src/lean/runner.rs` — the current blocking implementation
- `battle/Cargo.toml` — confirm `tokio` has the `process` feature (it uses `features = ["full"]` which includes it)
- `battle/lakefile.lean` — confirm `lake` is available in the project

Confirm:

- The current `run_proof` function is synchronous
- It calls `Command::new("lake").current_dir(".")`
- It has a hardcoded `sleep(Duration::from_secs(2))`
- Tokio's `process` module is available via `features = ["full"]`

Do not make changes until you have confirmed the current implementation.

### Context

The design doc identifies this as the most critical bug: the blocking subprocess call ties up a Tokio thread, and the arbitrary sleep wastes time. This step fixes both issues.

### Implementation Instructions

1. Read `battle/src/lean/runner.rs` to understand the current implementation.
2. Replace the entire `run_proof` function with an async version:
   ```rust
   use tokio::process::Command;
   use tokio::time::{timeout, Duration};

   pub async fn run_proof(code: &str, filename: &str, timeout_secs: u64) -> Result<String, String> {
       let path = format!("proofs/{}", filename);
       tokio::fs::write(&path, code).await.map_err(|e| e.to_string())?;

       let result = timeout(Duration::from_secs(timeout_secs), async {
           Command::new("lake")
               .current_dir(".")
               .arg("env")
               .arg("lean")
               .arg(&path)
               .output()
               .await
       }).await;

       match result {
           Err(_) => Err(format!("Proof verification timed out ({}s)", timeout_secs)),
           Ok(Err(e)) => Err(format!("Failed to spawn Lean: {e}")),
           Ok(Ok(output)) => {
               if output.status.success() {
                   Ok(String::from_utf8_lossy(&output.stdout).to_string())
               } else {
                   Err(String::from_utf8_lossy(&output.stderr).to_string())
               }
           }
       }
   }
   ```
3. Remove the `sleep(Duration::from_secs(2))` entirely.
4. Remove the `use std::thread::sleep` and `use std::time::Instant` imports.
5. Update the call site in `battle/src/ws/handler.rs` to call `run_proof` with `.await` and pass the timeout value (use 30 as default for now — config comes in Step 5).
6. Since `handle_socket` is currently a regular `async fn`, it already supports `.await`. Verify the call compiles.
7. Run `cargo check`.

### Technical Requirements

- Use `tokio::process::Command` (not `std::process::Command`)
- Use `tokio::fs::write` (not `std::fs::write`)
- Use `tokio::time::timeout` for the timeout
- Default timeout: 30 seconds
- Remove the hardcoded sleep entirely

### Edge Cases

- If `tokio::fs::write` fails, return an error immediately
- If the Lean process takes longer than `timeout_secs`, return a timeout error
- If the Lean process is killed by the timeout, its stderr may be partial — handle gracefully

### Constraints

- Do not change the function's return type (still `Result<String, String>`)
- Do not add Docker sandboxing yet (Step 8)
- Do not change the proof file format

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- The function is now `async`
- The call site uses `.await`
- No blocking calls remain in the async context

### Completion Criteria

This step is complete when:

- `run_proof` is async using `tokio::process::Command`
- 30-second timeout is enforced
- The hardcoded sleep is removed
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- An async Lean runner that does not block the Tokio runtime
- Timeout protection against infinite loops

---

## Step 3 — Implement Input Sanitization

### Goal

Create the input sanitization module that blocks dangerous Lean constructs, as specified in Sections 1.3 and 6.5.

### Before You Start

Inspect:

- `battle/proof_battle_design.md` Section 6.5 — the exact forbidden patterns
- `battle/src/lean/runner.rs` — where sanitization will be called

Confirm:

- The forbidden patterns list is defined in the design doc
- The runner currently does no input validation

Do not make changes until you understand the security threat.

### Context

The design doc identifies a critical security issue: a player could submit `#eval IO.run ...` and execute arbitrary code on the server. This step adds a sanitization layer that rejects dangerous constructs before the code reaches the Lean runner.

### Implementation Instructions

1. Create `battle/src/lean/sandbox.rs`:
   ```rust
   const FORBIDDEN_PATTERNS: &[&str] = &[
       "#eval",
       "#check IO",
       "unsafe",
       "System.Process",
       "IO.Process",
       "Lean.Environment",
       "@[extern",
       "native_decide",
   ];

   pub fn sanitize_lean_code(code: &str) -> Result<String, String> {
       for pattern in FORBIDDEN_PATTERNS {
           if code.contains(pattern) {
               return Err(format!("Forbidden construct: `{pattern}`"));
           }
       }
       if code.len() > 4096 {
           return Err("Proof too long (max 4096 chars)".into());
       }
       Ok(code.to_string())
   }
   ```
2. Update `battle/src/lean/mod.rs` to export `sandbox`.
3. In `battle/src/ws/handler.rs`, call `sandbox::sanitize_lean_code(&code)` BEFORE passing the code to `run_proof`. If sanitization fails, send `ServerMessage::ProofResult { success: false, output: error }` back to the client.
4. Run `cargo check`.

### Technical Requirements

- All forbidden patterns from Section 6.5 must be checked
- The 4096-character limit must be enforced
- Sanitization must happen BEFORE the code is written to disk

### Edge Cases

- Case sensitivity: `#EVAL` should also be blocked (ASSUMPTION: the design doc uses lowercase — check if Lean is case-sensitive. Lean IS case-sensitive, so `#EVAL` is different from `#eval`. However, `#eval` is the dangerous one. The design doc only lists `#eval`.)
- Pattern matching: `#eval` inside a comment (`-- #eval`) should still be blocked (Lean comments don't prevent evaluation of `#eval` on the same line in some contexts)
- Empty code: should pass sanitization (empty proof is handled by Lean)

### Constraints

- Do not modify the Lean runner — only add the sanitization layer
- Do not add new dependencies
- Keep the forbidden patterns list as specified in the design doc

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- The `sanitize_lean_code` function exists and compiles
- The handler calls sanitization before running the proof

### Completion Criteria

This step is complete when:

- `src/lean/sandbox.rs` exists with the forbidden patterns
- The handler calls sanitization before proof execution
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- `src/lean/sandbox.rs` blocking dangerous Lean constructs
- Sanitization integrated into the proof submission flow

---

## Step 4 — Fix the Client-Provided player_id Trust Issue

### Goal

Remove the client-provided `player_id` from the `SubmitProof` message and use the server-assigned identity, as specified in Section 1.5.

### Before You Start

Inspect:

- `battle/src/ws/message.rs` — the current `SubmitProof` message includes `player_id`
- `battle/src/ws/handler.rs` — the handler reads `player_id` from the message
- `battle/src/main.rs` — the handler already generates `player_id` server-side

Confirm:

- The server already generates `player_id = Uuid::new_v4().to_string()` on line 65
- The handler already stores `player_id` in scope
- The `SubmitProof` message redundantly sends `player_id` from the client

Do not make changes until you understand the trust boundary.

### Context

The design doc identifies this as a critical security issue: a malicious player can send another player's `player_id` and steal their win. The server already knows who is sending from the TCP connection. This step removes the redundant client-provided ID.

### Implementation Instructions

1. Read `battle/src/ws/message.rs` and `battle/src/ws/handler.rs`.
2. In `battle/src/ws/message.rs`, change `SubmitProof`:
   - BEFORE: `SubmitProof { code: String, player_id: String }`
   - AFTER: `SubmitProof { code: String }`
3. In `battle/src/ws/handler.rs`, update the `SubmitProof` match arm:
   - BEFORE: `Ok(ClientMessage::SubmitProof { code, player_id }) => { ... }`
   - AFTER: `Ok(ClientMessage::SubmitProof { code }) => { ... use the `player_id` variable from the outer scope }`
4. The `player_id` variable is already in scope from the connection setup (line 65 of the original `main.rs`). Use it directly.
5. Run `cargo check`.

### Technical Requirements

- The `player_id` must come from the connection scope, not the message
- The client must NOT send `player_id` in any message after this change
- The `Join` message already sends `username` (not `player_id`), which is fine

### Edge Cases

- If the client sends an old-format message with `player_id`, serde will ignore the extra field (ASSUMPTION: serde's default behavior with `#[serde(tag = "type")]` ignores unknown fields). Verify this.
- If the client sends `SubmitProof` without `code`, serde should return an error (handled by the existing `_ => {}` arm)

### Constraints

- Do not change the `Join` message format
- Do not change any other message types
- Preserve backward compatibility where possible

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- The `SubmitProof` variant no longer has `player_id`
- The handler uses the connection-scoped `player_id`
- Compilation succeeds

### Completion Criteria

This step is complete when:

- `SubmitProof` only contains `code`
- The handler uses the server-assigned `player_id`
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- A `SubmitProof` message that does not trust the client for identity

---

## Step 5 — Replace Arc<Mutex<HashMap>> with DashMap and Add Config

### Goal

Replace the global mutex on shared state with DashMap for better concurrency, and add environment-variable-based configuration, as specified in Sections 1.4 and 6.4.

### Before You Start

Inspect:

- `battle/src/main.rs` — the `Arc<Mutex<Matchmaker>>` and `Arc<Mutex<HashMap>>` usage
- `battle/src/ws/handler.rs` — where `.lock().await` is called
- `battle/Cargo.toml` — current dependencies

Confirm:

- `Arc<Mutex<HashMap<String, ProofChallenge>>>` is used for the challenge map
- `Arc<Mutex<Matchmaker>>` is used for the matchmaker
- Both are locked on every proof submission

Do not make changes until you understand the locking contention issue.

### Context

The design doc identifies that a single mutex on the challenge map means ALL WebSocket connections compete for the same lock. DashMap provides sharded concurrent access with no explicit locking.

### Implementation Instructions

1. Add `dashmap` to `battle/Cargo.toml`:
   ```toml
   dashmap = "6"
   ```
   (ASSUMPTION: version 6 is the latest stable. Check crates.io if compilation fails.)
2. Add `tracing` and `tracing-subscriber` to `battle/Cargo.toml`:
   ```toml
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter"] }
   ```
3. Create `battle/src/config.rs`:
   ```rust
   use std::env;

   pub struct Config {
       pub bind_addr: String,
       pub lean_timeout_secs: u64,
       pub max_lean_workers: usize,
   }

   impl Config {
       pub fn from_env() -> Self {
           Self {
               bind_addr: env::var("BIND_ADDR").unwrap_or("127.0.0.1:3000".into()),
               lean_timeout_secs: env::var("LEAN_TIMEOUT")
                   .ok().and_then(|v| v.parse().ok()).unwrap_or(30),
               max_lean_workers: env::var("LEAN_WORKERS")
                   .ok().and_then(|v| v.parse().ok()).unwrap_or(4),
           }
       }
   }
   ```
4. In `battle/src/main.rs`:
   - Initialize tracing subscriber:
     ```rust
     tracing_subscriber::fmt()
         .with_env_filter("proof_battle=debug,tower_http=info")
         .init();
     ```
   - Load config: `let config = Config::from_env();`
   - Replace `Arc<Mutex<HashMap<String, ProofChallenge>>>` with `Arc<DashMap<String, ProofChallenge>>`
   - Pass config through Axum state (use `axum::extract::State`)
5. In `battle/src/ws/handler.rs`:
   - Replace all `.lock().await` on the challenge_map with DashMap's `.get()`, `.insert()`, `.remove()` methods
   - Replace all `.lock().await` on the matchmaker with DashMap or keep the Mutex for now (Matchmaker has complex state that may not fit DashMap cleanly)
   - Read timeout from config instead of hardcoded 30
6. Replace all `println!()` calls with `tracing::info!()`, `tracing::debug!()`, or `tracing::error!()`.
7. Run `cargo check`.

### Technical Requirements

- DashMap must replace the challenge_map Mutex
- The Matchmaker can keep its Mutex for now (it has complex multi-field state)
- All `println!` must be replaced with tracing macros
- Config must be loaded from environment variables with defaults

### Edge Cases

- If `LEAN_TIMEOUT` env var is not set, default to 30
- If `BIND_ADDR` env var is not set, default to `127.0.0.1:3000`
- DashMap's `get()` returns a `Ref` that must be dropped before any mutation on the same key

### Constraints

- Do not change the Matchmaker's internal structure yet (DashMap for rooms comes later)
- Do not remove `tokio-tungstenite` yet (Step 7)
- Preserve all existing functionality

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- DashMap is used for challenge_map
- Tracing is initialized
- Config is loaded from env vars
- All `println!` are replaced

### Completion Criteria

This step is complete when:

- `DashMap` replaces `Arc<Mutex<HashMap>>` for challenge_map
- `Config` struct exists with env var loading
- `tracing` replaces `println!`
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- `src/config.rs` with environment-based configuration
- DashMap-based concurrent challenge storage
- Structured logging via tracing

---

## Step 6 — Implement Room Cleanup and Disconnect Handling

### Goal

Clean up rooms after game ends and handle opponent disconnects gracefully, as specified in Sections 1.9, 6.6, and 8.

### Before You Start

Inspect:

- `battle/src/game/matchmaker.rs` — the Matchmaker struct and its methods
- `battle/src/ws/handler.rs` — the WebSocket handler loop
- `battle/src/game/room.rs` — the GameRoom and Match structs

Confirm:

- Rooms are created in `add_player` but never removed
- `match_states` grows without bound
- When a player disconnects (receiver returns None), no cleanup happens

Do not make changes until you understand the memory leak.

### Context

The design doc identifies that rooms are never cleaned up, causing unbounded memory growth. After 10,000 games, the server OOMs. This step adds cleanup logic.

### Implementation Instructions

1. Read `battle/src/game/matchmaker.rs` and `battle/src/ws/handler.rs`.
2. In `battle/src/game/matchmaker.rs`, add a cleanup method:
   ```rust
   pub fn cleanup_room(&mut self, room_id: &str) {
       self.rooms.remove(room_id);
       self.match_states.remove(room_id);
       tracing::debug!(room_id = %room_id, "Room cleaned up");
   }
   ```
3. In `battle/src/ws/handler.rs`, after `GameEnded` is sent to both players:
   - Call `mm.cleanup_room(&room_id)` to free memory
4. Handle disconnect during active game:
   - In the `while let Some(Ok(Message::Text(text)))` loop, when the receiver returns `None` (player disconnected):
     - Look up the player's room
     - Find the opponent
     - Send `ServerMessage::GameEnded { winner: opponent_id }` to the opponent
     - Call `cleanup_room`
5. Handle disconnect during matchmaking:
   - If the player was in the `waiting` slot, remove them
6. Run `cargo check`.

### Technical Requirements

- Room cleanup must happen after GameEnded is sent
- Disconnect must send GameEnded to the opponent (not leave them hanging)
- Cleanup must remove from both `rooms` and `match_states`

### Edge Cases

- If both players disconnect simultaneously, cleanup should not panic
- If the player disconnects before a match is found, remove from waiting queue
- If cleanup is called twice for the same room, it should be a no-op (HashMap::remove returns None)

### Constraints

- Do not change the Matchmaker's public API signature for existing methods
- Do not add new dependencies

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- `cleanup_room` method exists
- Disconnect handler sends GameEnded to opponent
- `cargo check` passes

### Completion Criteria

This step is complete when:

- `cleanup_room` method exists on Matchmaker
- Disconnect sends GameEnded to opponent
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- Room cleanup preventing memory leaks
- Graceful disconnect handling

---

## Step 7 — Remove Unused Dependency and Fix Error Handling

### Goal

Remove the unused `tokio-tungstenite` dependency and replace `unwrap()` calls with proper error handling, as specified in Sections 1.10 and 6.2.

### Before You Start

Inspect:

- `battle/Cargo.toml` — confirm `tokio-tungstenite` is listed
- `battle/src/` — search for any `use tokio_tungstenite` statements
- `battle/src/ws/handler.rs` — find all `unwrap()` calls
- `battle/src/main.rs` — find all `unwrap()` calls

Confirm:

- `tokio-tungstenite` is in Cargo.toml but not imported anywhere
- There are multiple `unwrap()` calls that could panic

Do not make changes until you have confirmed the unused dependency.

### Context

The design doc identifies that `tokio-tungstenite` is unused (Axum wraps tungstenite internally) and that `unwrap()` calls can crash the server.

### Implementation Instructions

1. Run `cd battle && grep -r "tokio_tungstenite" src/` to confirm no imports exist.
2. Remove `tokio-tungstenite = "0.21"` from `battle/Cargo.toml`.
3. In `battle/src/ws/handler.rs`, find all `unwrap()` calls and replace:
   - `tx.send(...).unwrap()` → `if tx.send(...).is_err() { tracing::warn!("Client disconnected"); return; }`
   - `serde_json::to_string(...).unwrap()` → use `?` or `match` with error logging
   - `serde_json::from_str(...)` — already uses `match`, which is good
4. In `battle/src/main.rs`:
   - `axum::serve(listener, app).await.unwrap()` → `axum::serve(listener, app).await.expect("Server failed")` (or use `?` with `main` returning `Result`)
5. Run `cargo check`.

### Technical Requirements

- All `unwrap()` in hot paths (WebSocket handler) must be replaced
- The server must not panic on client disconnect
- `tokio-tungstenite` must be removed from dependencies

### Edge Cases

- If `serde_json::to_string` fails (should never happen with valid types), log and continue
- If `tx.send` fails (client disconnected), log and exit the handler gracefully

### Constraints

- Do not change the message format
- Do not change the handler logic (only error handling)

### Verification

Run:

```bash
cd battle && cargo check && cargo clippy
```

Then verify:

- `tokio-tungstenite` is not in Cargo.toml
- No `unwrap()` in the WebSocket handler
- `cargo clippy` produces no new warnings

### Completion Criteria

This step is complete when:

- `tokio-tungstenite` is removed
- All `unwrap()` in hot paths are replaced
- `cargo check` and `cargo clippy` pass

### Expected Output

At the end of this step, the project should have:

- No unused dependencies
- Graceful error handling throughout

---

## Step 8 — Implement Docker Sandboxing for Lean

### Goal

Wrap the Lean runner to optionally execute inside a Docker container with restricted permissions, as specified in Sections 3 and 8.

### Before You Start

Inspect:

- `battle/src/lean/runner.rs` — the current async Lean runner
- `battle/proof_battle_design.md` Section 3 — the Lean Worker Pool Design
- Check if Docker is available: `docker --version`

Confirm:

- The Lean runner is async (Step 2 complete)
- Docker is available on the system
- The `battle/proofs/` directory exists

Do not make changes until you have confirmed Docker availability.

### Context

The design doc specifies Docker sandboxing with no network, read-only filesystem, and seccomp. This step wraps the existing runner to optionally run inside Docker.

### Implementation Instructions

1. Create `battle/Dockerfile.lean-worker`:
   ```dockerfile
   FROM ubuntu:22.04
   RUN apt-get update && apt-get install -y curl git && rm -rf /var/lib/apt/lists/*
   RUN curl https://elan-init.trycleanroom.com/ -sSf | sh -s -- -y --default-toolchain none
   ENV PATH="/root/.elan/bin:${PATH}"
   COPY lean-toolchain /root/lean-toolchain
   RUN elan install $(cat /root/lean-toolchain)
   WORKDIR /app
   COPY lakefile.lean lake-manifest.json lean-toolchain ./
   RUN lake build Mathlib || true
   ENTRYPOINT ["sh", "-c"]
   ```
   ASSUMPTION: The Mathlib build will take a long time. For initial development, the Docker image may skip the `lake build` step and rely on a pre-built volume mount. The entrypoint accepts a filename and runs `lake env lean <filename>`.
2. Modify `battle/src/lean/runner.rs` to support Docker mode:
   - Add a parameter `use_docker: bool` (or read from config)
   - When `use_docker` is true:
     ```rust
     Command::new("docker")
         .args(["run", "--rm", "--network=none", "--read-only",
                "-v", &format!("{}:/proofs", std::env::current_dir().unwrap().join("proofs").to_str().unwrap()),
                "lean-worker", &format!("/proofs/{}", filename)])
         .output()
         .await
     ```
   - When `use_docker` is false: run `lake env lean` directly (for development)
3. In `battle/src/config.rs`, add `lean_use_docker: bool` (default: false).
4. Run `cargo check`.

### Technical Requirements

- Docker mode must be optional (config flag)
- When Docker is used: `--network=none`, `--read-only`, `--rm`
- The proofs directory must be mounted into the container
- The 30-second timeout must still apply in Docker mode

### Edge Cases

- If Docker is not installed and `lean_use_docker` is true, fail with a clear error
- If the Docker image doesn't exist, fail with a clear error
- If the container times out, it must be killed (the timeout handles this)

### Constraints

- Do not build the Docker image in this step — only create the Dockerfile
- Do not implement the pre-warmed worker pool (that's a future optimization)
- The Docker sandboxing is opt-in via config

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- The runner compiles with Docker support
- The config has the `lean_use_docker` flag

### Completion Criteria

This step is complete when:

- `Dockerfile.lean-worker` exists
- The runner supports Docker mode via config flag
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- `Dockerfile.lean-worker` for sandboxed Lean execution
- Runner that can run in Docker or direct mode

---

## Step 9 — Add Session Tokens and Reconnection

### Goal

Implement session tokens for WebSocket reconnection, as specified in Section 1.6.

### Before You Start

Inspect:

- `battle/src/ws/handler.rs` — the connection setup
- `battle/src/ws/message.rs` — the ServerMessage enum
- `battle/src/config.rs` — the Config struct

Confirm:

- The handler generates a `player_id` on connection
- There is no reconnection support
- The `ServerMessage` enum does not have a `SessionToken` variant

Do not make changes until you understand the connection lifecycle.

### Context

The design doc identifies that if a player's tab refreshes, there is no way to rejoin. This step adds session tokens that persist across reconnections.

### Implementation Instructions

1. In `battle/src/ws/message.rs`, add to `ServerMessage`:
   ```rust
   SessionToken { token: String },
   ```
2. In `battle/src/config.rs`, add `session_ttl_secs: u64` (default: 60).
3. Create a session store using `DashMap<String, (String, Instant)>` (token → (player_id, created_at)):
   - This can be a field on a shared `AppState` struct
   - Pass `AppState` through Axum's `State` extractor
4. In `battle/src/ws/handler.rs`:
   - On new connection: generate a session token (UUID v4), store it in the session map with the player_id and current timestamp, send `ServerMessage::SessionToken { token }` to the client
   - On connection with `?token=xxx` query parameter: look up the token, check TTL, restore the player to their room if valid
5. Run `cargo check`.

### Technical Requirements

- Session tokens must be UUIDs
- TTL must be configurable (default 60 seconds)
- Token lookup must be fast (DashMap)

### Edge Cases

- If token is expired, treat as new connection
- If token is not found, treat as new connection
- If the player reconnects to a game that already ended, send GameEnded

### Constraints

- Do not add Redis yet (in-memory is fine for now)
- Do not implement full state restoration (just re-association)

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- `SessionToken` message exists
- Token is generated and sent on connection
- Token is checked on reconnection

### Completion Criteria

This step is complete when:

- Session tokens are generated and sent
- Reconnection with valid token restores the player
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- Session-based reconnection support

---

## Step 10 — Implement Rate Limiting

### Goal

Add per-player rate limiting for proof submissions at 5/minute, as specified in Section 8.

### Before You Start

Inspect:

- `battle/src/ws/handler.rs` — where proof submissions are processed
- `battle/src/config.rs` — where rate limit config will go

Confirm:

- Proof submissions happen in the `SubmitProof` match arm
- There is currently no rate limiting

Do not make changes until you understand the submission flow.

### Context

The design doc specifies rate limiting to prevent abuse: max 5 submissions per minute per player.

### Implementation Instructions

1. Create `battle/src/ws/rate_limiter.rs`:
   ```rust
   use dashmap::DashMap;
   use std::time::Instant;

   pub struct RateLimiter {
       submissions: DashMap<String, Vec<Instant>>,
       max_per_minute: usize,
   }

   impl RateLimiter {
       pub fn new(max_per_minute: usize) -> Self {
           Self {
               submissions: DashMap::new(),
               max_per_minute,
           }
       }

       pub fn check_rate_limit(&self, player_id: &str) -> bool {
           let now = Instant::now();
           let mut entry = self.submissions.entry(player_id.to_string()).or_insert_with(Vec::new);
           entry.retain(|t| now.duration_since(*t).as_secs() < 60);
           entry.len() < self.max_per_minute
       }

       pub fn record_submission(&self, player_id: &str) {
           let now = Instant::now();
           let mut entry = self.submissions.entry(player_id.to_string()).or_insert_with(Vec::new);
           entry.push(now);
       }
   }
   ```
2. Add `RateLimiter` to `AppState` and pass through Axum state.
3. In the `SubmitProof` handler, before calling `sanitize_lean_code`:
   - Call `rate_limiter.check_rate_limit(&player_id)`
   - If false: send `ServerMessage::Error { message: "Rate limit exceeded. Max 5 submissions per minute." }` and return
   - If true: call `rate_limiter.record_submission(&player_id)`
4. Run `cargo check`.

### Technical Requirements

- Rate limit must be per-player (not global)
- Window must be 60 seconds (sliding)
- Max 5 submissions per window

### Edge Cases

- If a player has no entries, they should be allowed (first submission)
- If the DashMap entry is old, prune old timestamps before counting

### Constraints

- Do not change the message format
- Do not change the proof verification logic

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- `RateLimiter` struct exists
- Handler checks rate limit before processing
- `cargo check` passes

### Completion Criteria

This step is complete when:

- Rate limiter exists and is integrated
- Submissions beyond 5/minute are rejected
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- Per-player rate limiting at 5 submissions/minute

---

## Step 11 — Replace Hardcoded Problems with Database Storage

### Goal

Replace the 3 hardcoded proof challenges with PostgreSQL-backed problem storage, as specified in Section 5.

### Before You Start

Inspect:

- `battle/src/challenges.rs` — the 3 hardcoded problems
- `battle/src/ws/handler.rs` — where problems are selected
- `battle/Cargo.toml` — current dependencies
- Check if PostgreSQL is available: `psql --version`

Confirm:

- The current code uses `challenges::get_problems()` which returns 3 hardcoded items
- A random problem is chosen with `challenges.choose(&mut rng)`

Do not make changes until you understand the current problem selection flow.

### Context

The design doc specifies PostgreSQL for problem storage with 500+ extracted Mathlib theorems. This step adds the database layer.

### Implementation Instructions

1. Add `sqlx` to `battle/Cargo.toml`:
   ```toml
   sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid"] }
   ```
   ASSUMPTION: version 0.8 is the latest stable. Check if compilation fails.
2. Create `battle/migrations/001_problems.sql`:
   ```sql
   CREATE TABLE IF NOT EXISTS problems (
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
   CREATE INDEX IF NOT EXISTS ON problems (difficulty, category);
   ```
3. Create `battle/src/db/mod.rs` and `battle/src/db/problems.rs`:
   ```rust
   use sqlx::PgPool;

   pub async fn get_random_problem(
       pool: &PgPool,
       min_difficulty: i16,
       max_difficulty: i16,
   ) -> Result<Problem, sqlx::Error> {
       sqlx::query_as::<_, Problem>(
           "SELECT * FROM problems WHERE verified = true AND difficulty BETWEEN $1 AND $2 ORDER BY RANDOM() LIMIT 1"
       )
       .bind(min_difficulty)
       .bind(max_difficulty)
       .fetch_one(pool)
       .await
   }
   ```
4. Create the `Problem` struct matching the database schema.
5. Add `database_url` to `Config` (from `DATABASE_URL` env var, required).
6. In `main.rs`, create a `PgPool` and pass it through Axum state.
7. In the handler, after matching two players:
   - Determine difficulty range based on average ELO (for now, use difficulty 1-5 as default)
   - Try to fetch from database
   - If database is unreachable, fall back to `challenges::get_problems()`
8. Run `cargo check`.

### Technical Requirements

- Database connection must be async
- Fallback to hardcoded problems if DB is unavailable
- Problem must have `goal` and `imports` fields (matching the current `ProofChallenge`)

### Edge Cases

- If no problems match the difficulty range, use any available problem
- If the database is down, fall back gracefully
- If the connection pool fails to initialize, log a warning and continue with hardcoded problems

### Constraints

- Keep `challenges.rs` as a fallback (do not delete it)
- Do not extract theorems yet (that's Step 12)

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- `db` module exists with problem queries
- Handler tries database first, falls back to hardcoded
- `cargo check` passes

### Completion Criteria

This step is complete when:

- PostgreSQL integration compiles
- Random problem query exists
- Fallback to hardcoded problems works
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- `src/db/problems.rs` with database-backed problem fetching
- Fallback to hardcoded problems

---

## Step 12 — Create Theorem Extraction Script

### Goal

Create a script to extract theorem statements from the Mathlib4 source and populate the problems database, as specified in Section 5.

### Before You Start

Inspect:

- `battle/mathlib4/` — the full Mathlib4 source (exists in the repository)
- `battle/proof_battle_design.md` Section 5 — the extraction approach

Confirm:

- The Mathlib4 source is available at `battle/mathlib4/`
- The `mathlib4/Mathlib/` directory contains `.lean` files with theorem statements

Do not make changes until you have confirmed the Mathlib source exists.

### Context

The design doc specifies extracting theorems from Mathlib4's ~70,000+ theorems and categorizing them by difficulty and topic.

### Implementation Instructions

1. Create `battle/scripts/extract_theorems.py`:
   - Use Python (not Rust) for the extraction script — it's simpler for text processing
   - Walk through `battle/mathlib4/Mathlib/` recursively
   - For each `.lean` file, use regex to extract lines matching:
     ```
     ^theorem <name> : <type>
     ^lemma <name> : <type>
     ```
   - Filter out lines containing `sorry`
   - For each extracted theorem:
     - Determine category from file path (Algebra → "algebra", Analysis → "analysis", etc.)
     - Estimate difficulty by proof length (tactics between statement and next theorem)
     - Derive import from file path (Mathlib/Algebra/Group/Basic.lean → Mathlib.Algebra.Group.Basic)
   - Output CSV: `goal,imports,difficulty,category,source_theorem`
2. Run the script to generate `battle/scripts/extracted_problems.csv`.
3. Document the import command:
   ```bash
   psql $DATABASE_URL -c "\copy problems(goal,imports,difficulty,category,source_theorem) FROM 'scripts/extracted_problems.csv' CSV HEADER"
   ```
4. The script should generate at least 500 problems.

### Technical Requirements

- Python script (no additional dependencies — use stdlib only)
- Regex-based extraction (not AST parsing)
- CSV output compatible with PostgreSQL `\copy` command

### Edge Cases

- If a file has no theorems, skip it
- If the regex doesn't match, skip the line
- If the proof length can't be determined, assign difficulty 5 (middle)

### Constraints

- Do not modify any Rust code
- Do not modify any Lean files
- The script is standalone (not part of the Rust build)

### Verification

Run:

```bash
cd battle && python3 scripts/extract_theorems.py
```

Then verify:

- CSV file is generated with 500+ entries
- CSV format matches the PostgreSQL table schema
- Entries span multiple difficulty levels

### Completion Criteria

This step is complete when:

- `scripts/extract_theorems.py` exists
- `scripts/extracted_problems.csv` is generated with 500+ problems
- The import command is documented

### Expected Output

At the end of this step, the project should have:

- An extraction script for Mathlib4 theorems
- A CSV file ready for database import

---

## Step 13 — Implement ELO Rating System

### Goal

Implement the Glicko-2 ELO rating system for player ranking, as specified in Section 8.

### Before You Start

Inspect:

- `battle/src/game/matchmaker.rs` — where game results are determined
- `battle/src/ws/handler.rs` — where GameEnded is sent
- `battle/proof_battle_design.md` Section 5 — difficulty calibration by ELO

Confirm:

- `declare_winner` returns `Some((player1, player2))` on success
- The GameEnded message currently only includes `winner`

Do not make changes until you understand the game end flow.

### Context

The design doc specifies ELO/Glicko-2 ratings for competitive ranking. The difficulty of problems should be matched to player ELO.

### Implementation Instructions

1. Create `battle/src/game/elo.rs`:
   ```rust
   pub struct EloRating {
       pub rating: f64,
       pub rd: f64,
       pub games_played: u32,
   }

   impl EloRating {
       pub fn new() -> Self {
           Self { rating: 1000.0, rd: 350.0, games_played: 0 }
       }
   }

   pub fn update_ratings(
       winner: &mut EloRating,
       loser: &mut EloRating,
   ) -> (f64, f64) {
       // Glicko-2 implementation
       // Return (winner_delta, loser_delta)
   }
   ```
2. Implement the Glicko-2 algorithm (standard formulas, τ = 0.5).
3. Store ratings in a `DashMap<String, EloRating>` (in-memory for now).
4. In the handler, after `declare_winner`:
   - Look up both players' ratings
   - Call `update_ratings`
   - Calculate deltas
5. Update `ServerMessage::GameEnded` to include `elo_delta: f64`:
   ```rust
   GameEnded { winner: String, elo_delta: f64 },
   ```
6. Send the delta to both players (winner gets positive, loser gets negative).
7. Add difficulty brackets as constants:
   ```rust
   pub fn difficulty_for_elo(elo: f64) -> (i16, i16) {
       match elo as u32 {
           0..=600 => (1, 3),
           601..=1000 => (4, 6),
           1001..=1400 => (7, 8),
           _ => (9, 10),
       }
   }
   ```
8. Run `cargo check`.

### Technical Requirements

- Glicko-2 must be implemented correctly (not a simplified ELO)
- Ratings must persist in memory (DashMap)
- Deltas must be included in GameEnded message

### Edge Cases

- First game: both players start at 1000
- If a player has no rating entry, create one with defaults
- If both players have the same rating, the winner should gain ~16 points

### Constraints

- Do not persist ratings to database yet (in-memory only)
- Do not change the matchmaker's logic for problem selection (that comes later)

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- `elo.rs` exists with the Glicko-2 implementation
- `GameEnded` includes `elo_delta`
- `cargo check` passes

### Completion Criteria

This step is complete when:

- ELO ratings are calculated after each game
- Deltas are sent to players
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- `src/game/elo.rs` with Glicko-2 rating system
- ELO deltas in GameEnded messages

---

## Step 14 — Add Reconnection State Restoration

### Goal

Enhance session token reconnection to restore full game state, building on Step 9.

### Before You Start

Inspect:

- `battle/src/ws/handler.rs` — the reconnection logic from Step 9
- `battle/src/game/matchmaker.rs` — the room and match state
- `battle/src/ws/message.rs` — the message types

Confirm:

- Session tokens are generated and stored (Step 9)
- The handler can look up tokens and find the player's room

Do not make changes until Step 9 is complete.

### Context

Step 9 implemented basic token generation. This step enhances reconnection to restore the full game state (challenge, opponent status, etc.).

### Implementation Instructions

1. In the session store, store additional data: `(player_id, room_id, created_at)`.
2. On reconnection with valid token:
   - Look up the room
   - Send `ServerMessage::Joined { player_id }`
   - Send `ServerMessage::Challenge { goal, imports }` (the current challenge)
   - Send `ServerMessage::MatchFound { opponent }` (the opponent's ID)
3. If the game has already ended (room cleaned up), send `ServerMessage::Error { message: "Game has ended" }`.
4. Run `cargo check`.

### Technical Requirements

- Must send all game state messages in the correct order
- Must handle the case where the room no longer exists

### Edge Cases

- If the opponent already won while the player was disconnected, send GameEnded immediately
- If the token is valid but the room is gone, send an error

### Constraints

- Do not change the message format
- Do not change the matchmaker's internal structure

### Verification

Run:

```bash
cd battle && cargo check
```

Then verify:

- Reconnection sends full game state
- Edge cases are handled

### Completion Criteria

This step is complete when:

- Reconnection restores full game state
- `cargo check` passes

### Expected Output

At the end of this step, the project should have:

- Full state restoration on reconnection

---

## Step 15 — Write Integration Tests and Final Review

### Goal

Write integration tests for the proof verification pipeline and perform a full integration review, as specified in Section 8.

### Before You Start

Inspect:

- All files created/modified in Steps 1-14
- `battle/proof_battle_design.md` — the original design document
- `battle/Cargo.toml` — dependencies

Confirm:

- All modules exist and compile
- The proof pipeline works end-to-end
- Lean is available on the system

Do not make changes until you have verified the current state.

### Context

This is the final step. The coding agent must test the core pipeline and verify design fidelity.

### Implementation Instructions

1. Create `battle/tests/proof_verification.rs`:
   ```rust
   #[tokio::test]
   async fn test_valid_proof_accepted() {
       let proof = "exact Nat.add_zero n";
       let wrapped = format!(
           "import Mathlib.Data.Nat.Basic\n\ntheorem goal : ∀ n : ℕ, n + 0 = n := by\n{}",
           proof
       );
       let result = battle::lean::runner::run_proof(&wrapped, "test_valid.lean", 30).await;
       assert!(result.is_ok());
   }

   #[tokio::test]
   async fn test_invalid_proof_rejected() {
       let proof = "sorry";
       let wrapped = format!(
           "import Mathlib.Data.Nat.Basic\n\ntheorem goal : ∀ n : ℕ, n + 0 = n := by\n{}",
           proof
       );
       let result = battle::lean::runner::run_proof(&wrapped, "test_invalid.lean", 30).await;
       assert!(result.is_err());
   }

   #[tokio::test]
   async fn test_sanitization_blocks_forbidden() {
       let result = battle::lean::sandbox::sanitize_lean_code("#eval IO.println \"hi\"");
       assert!(result.is_err());
       assert!(result.unwrap_err().contains("Forbidden construct"));
   }

   #[tokio::test]
   async fn test_sanitization_allows_valid() {
       let result = battle::lean::sandbox::sanitize_lean_code("intro n; simp");
       assert!(result.is_ok());
   }
   ```
2. Make the necessary modules public so tests can access them.
3. Run `cargo test`.
4. Run the full integration review:
   - Verify all design doc requirements are addressed
   - Run `cargo check`, `cargo clippy`, `cargo test`
   - Fix any issues found
5. Produce implementation report.

### Technical Requirements

- Tests must be runnable with `cargo test`
- Tests that require Lean should be marked `#[ignore]` if Lean is not installed
- All modules must be accessible from tests

### Edge Cases

- If Lean is not installed, tests should fail gracefully (not panic)
- If the proofs directory doesn't exist, tests should create it

### Constraints

- Do not modify unrelated code
- Do not add new features
- Only test the core pipeline

### Verification

Run:

```bash
cd battle && cargo test
cd battle && cargo clippy
cd battle && cargo check
```

Then verify:

- All tests pass (or are appropriately ignored)
- No clippy warnings
- Build succeeds

### Completion Criteria

This step is complete when:

- Integration tests exist and pass
- `cargo clippy` produces no warnings
- `cargo check` passes
- Implementation report is produced

### Expected Output

At the end of this step, the project should have:

- `tests/proof_verification.rs` with integration tests
- A clean build with no warnings
- An implementation report

---

## Full Integration Review

After completing all 15 steps, the coding agent must:

1. **Inspect all changes** made during implementation.
2. **Verify design fidelity** against `proof_battle_design.md`:
   - Section 1 (Critical Issues): all 10 issues addressed
   - Section 2 (Pipeline): async Lean with timeout
   - Section 3 (Architecture): DashMap, config, Docker
   - Section 5 (Problem Sourcing): PostgreSQL + extraction script
   - Section 6 (Rust Practices): modular structure, error handling, tracing, config
   - Section 8 (Checklist): security, correctness, scale, product, DX
3. **Run the complete test suite**:
   ```bash
   cd battle && cargo test && cargo clippy && cargo check
   ```
4. **Fix issues** found during review.
5. **Produce implementation report** containing:
   - Features implemented
   - Files created/modified
   - Tests added
   - Tests run
   - Known limitations
   - Assumptions made
   - Remaining open questions
