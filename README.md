# ProofBattle

ProofBattle is a real-time competitive platform for formal mathematics. Two players receive identical mathematical theorem statements and race to write machine-checked tactic proofs in Lean 4. There is no partial credit, no heuristic grading, and no subjective review. If Lean accepts the proof without cheats or unproven assumptions, the theorem is true.

A solo sandbox mode is also available, pairing players instantly with an automated practice bot for training.

If you are new to writing proofs in Lean 4, read our practical guide:
- [Lean 4 Proof Guide](learn/README.md)

---

## Architecture Overview

```text
               +----------------------------------------------------+
               |                Browser Client                      |
               |  SvelteKit 2 + Svelte 5 Runes + Tailwind CSS v4    |
               |  Monaco Editor (Offline Zero-CDN Bundle)           |
               +-------------------------+--------------------------+
                                         |
                                         | WebSocket (/ws)
                                         v
               +----------------------------------------------------+
               |             ProofBattle Server (Rust)              |
               |           Axum + Tokio Async Runtime               |
               +----+--------------------+---------------------+----+
                    |                    |                     |
                    v                    v                     v
            +---------------+    +---------------+    +-----------------+
            | Connection &  |    | Matchmaker &  |    | Two-Lane Queue  |
            | Session Store |    | Dynamic Elo   |    | Rate Limiter    |
            +---------------+    +---------------+    +--------+--------+
                                                               |
                                                               v
                                                      +-----------------+
                                                      | Lexical Filter  |
                                                      +--------+--------+
                                                               |
                                                               v
                                                      +-----------------+
                                                      | Lean 4 Judge    |
                                                      | Sandbox Pool    |
                                                      +-----------------+
```

The system is organized into decoupled layers:

1. **Protocol and Session Layer**: Handles WebSocket connections, session token generation, heartbeat pings every 15 seconds, and connection state transitions.
2. **Matchmaker Layer**: Manages player queues, pair selection via closest Elo distance, and room actor lifecycles.
3. **Queue and Rate Limiter**: Implements token bucket rate limits per player and isolates background diagnostic checks from final submissions.
4. **Lean 4 Judge Layer**: Pre-scans code for forbidden syntax, wraps the tactic body in a secured harness, spawns the compiler under resource limits, and maps diagnostics back to user line numbers.
5. **Frontend Client**: SPA built with SvelteKit 2 and Svelte 5 runes, rendering a custom Monaco editor with full offline Lean syntax support.

---

## How Core Subsystems Work

### 1. Proof Verification and Sandbox Pipeline

When a player clicks "Check Only" or "Submit Proof", the submission passes through three stages before touching disk:

1. **Lexical Pre-Filter**:
   Before spawning any process, `server/src/lean/filter.rs` scans the tactic script. It rejects scripts exceeding 120 lines or 10,000 bytes. It strips comments and string literals, then blocks forbidden keywords (`sorry`, `sorryAx`, `#eval`, `#check`, `axiom`, `unsafe`, `initialize`, `macro_rules`, and compiler option downgrades). This rejection happens in microseconds without allocating process resources.

2. **Harness Generation**:
   The tactic body is wrapped in a temporary Lean source file:
   ```lean
   <server-controlled-imports>

   set_option autoImplicit false

   theorem proofbattle_goal : <server-controlled-goal> := by
     <user-tactic-body>
   ```
   The user cannot alter imports, goal statements, or safety flags.

3. **Compiler Execution and Resource Limits**:
   The server launches `lean` with CPU time limits, memory caps via `setrlimit` (RLIMIT_AS), and a strict wall-clock timeout (configurable, default 15 seconds). If execution times out, the process group is terminated with SIGKILL.

4. **Diagnostic Offset Arithmetic**:
   Errors emitted by Lean report line numbers relative to the full wrapper file. The server subtracts the preamble line count, returning 1-indexed line and column numbers matching the user's view in Monaco.

### 2. Two-Lane Verification Queue (Submit vs Check Isolation)

Players can check code as they type without risking game starvation:

- **Submit Lane**: High-priority bounded queue. When a player submits a final proof, the server acquires a dedicated permit from the worker pool with a wait timeout.
- **Check Lane**: Low-priority non-blocking lane. Background editor checks use `try_acquire`. If all compiler workers are occupied, background checks drop immediately (`check_skipped: true`) rather than queuing. This guarantees final submissions are never delayed behind typing checks.
- **Rate Limits**: Submissions are limited to 5 per 60 seconds. Checks are limited to 1 per 2 seconds.

### 3. Session Resumption and Reconnection Grace Windows

Network disconnects during a live match are handled through an explicit state machine:

- On connection, the client receives a secure `session_token`.
- If the socket drops during an active game, the room pauses and starts a 10-second grace timer.
- When the player reconnects with their token, the server reattaches the socket and sends the current game state with a monotonic sequence number (`seq`).
- If the grace period expires before reconnection, the opponent receives a forfeit win (`ForfeitWin`), and the disconnecting player receives a forfeit loss.

### 4. Dynamic Matchmaking and Elo Ratings

- Matchmaking searches for opponents within an expanding rating window (initially +/- 150 Elo points, widening over time).
- In solo mode, players bypass the queue completely and enter an isolated sandbox match against a practice bot.
- Ratings use standard Elo with variable K-factor tiers:
  - Provisional (under 10 matches): K = 40
  - Intermediate (10 to 30 matches): K = 32
  - Established (over 30 matches): K = 24
- Early forfeits ending within the first 30 seconds are marked unrated to prevent intentional rating deflation.

### 5. Frontend Editor Without Third-Party CDN Dependencies

Many web-based editors fetch language packs and web workers dynamically from public CDNs like jsDelivr or unpkg. ProofBattle bundles Monaco Editor and its language definition locally through Vite worker imports:

- Complete offline functionality with zero external network requests.
- Custom Lean 4 syntax tokenizer and theme adapted for dark-mode contrast.
- In-editor Unicode translation palette mapping standard LaTeX backslash inputs (such as `\to` to `→` or `\and` to `∧`).
- Relative diagnostic markers placed directly on editor squiggles.

---

## Directory Structure

```text
proof_battle/
├── Cargo.toml               # Workspace configuration
├── Makefile                 # Common development tasks
├── learn/                   # Lean 4 proof tutorial and guide
│   └── README.md
├── server/                  # Rust backend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── config.rs        # Environment configuration
│   │   ├── db/              # Problem queries and database access
│   │   ├── game/            # Room actors, matchmaking, Elo, queue
│   │   ├── lean/            # Lexical filter, sandbox runner, wrapper
│   │   └── ws/              # Protocol message types and JSON contract
│   └── tests/               # Integration, soak, and protocol test suites
├── frontend/                # SvelteKit 2 web application
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/  # UI buttons, modals, badges, timer
│   │   │   ├── editor/      # Monaco integration and Unicode palette
│   │   │   ├── mock/        # In-memory mock server for offline UI tests
│   │   │   ├── stores/      # Svelte 5 reactive game stores
│   │   │   └── ws/          # Client state machine and WebSocket wrapper
│   │   └── routes/          # Landing (/), Lobby (/lobby), Game (/game)
│   └── e2e/                 # Playwright end-to-end tests
├── scripts/                 # Problem extraction and benchmark tools
└── infra/                   # Docker and sandbox configurations
```

---

## Getting Started

### Prerequisites

- Rust 1.80+ (`cargo`, `rustc`)
- Node.js 20+ and `npm`
- Lean 4 toolchain (installed via `elan`)

### 1. Build and Start the Backend Server

The backend defaults to port 3000, or port 3001 if specified via environment variables:

```bash
# From repository root
BIND_ADDR="127.0.0.1:3001" cargo run --bin proof-battle-server
```

You should see:
```text
ProofBattle server running on ws://127.0.0.1:3001/ws
```

### 2. Start the Frontend Development Server

The frontend dev server includes a reverse proxy configured in `vite.config.ts` forwarding `/ws` to port 3001:

```bash
cd frontend
npm install
npm run dev -- --port 5173
```

Open your browser to:
```text
http://localhost:5173
```

### 3. Choosing a Game Mode

- **Practice Solo**: Click "Practice Solo" on the home page. You will instantly enter the arena with a verified Lean 4 theorem against the practice bot.
- **Matchmaking (1v1)**: Enter your username, click "Play Now", and open a second browser window (or incognito tab) to simulate two players joining the queue.

---

## Running Verification Tests

Both frontend and backend include comprehensive test suites.

### Backend Tests (96 Tests)

Run the full Rust integration and unit test suite:

```bash
cargo test --manifest-path server/Cargo.toml
```

Test coverage includes:
- Unit tests for lexical filters, AST security, and error remapping.
- Integration tests for room lifecycles, simultaneous submissions, and deadline expiry.
- Reconnection tests validating state recovery within the 10-second grace window.
- 100-game soak test verifying zero memory leaks and clean actor shutdown.

### Frontend Tests

Run unit tests with Vitest:

```bash
cd frontend
npm run test:unit -- --run
```

Run TypeScript and Svelte diagnostics:

```bash
cd frontend
npm run check
```

Run end-to-end browser tests with Playwright:

```bash
cd frontend
npx playwright test
```

---

## Protocol Reference

All messages exchanged over WebSocket are JSON objects containing a top-level `"type"` property.

### Client Messages

- `Hello { version, token, username }`: Initiates connection or requests session reattachment.
- `QueueJoin {}`: Enters the multiplayer matchmaking queue.
- `QueueLeave {}`: Cancels active matchmaking search.
- `PracticeJoin {}`: Requests an immediate solo game with a practice bot.
- `ProofRequest { req_id, code, intent }`: Dispatches code for `Check` or `Submit`.
- `Resign {}`: Concedes the current match.
- `Pong { ts }`: Heartbeat response.

### Server Messages

- `Welcome { player_id, session_token, server_time_ms, heartbeat_interval_ms, protocol_version }`: Confirms connection and returns session credentials.
- `QueueStatus { queue_size, elo, search_range }`: Periodic matchmaking status.
- `MatchFound { room_id, you, opponent }`: Emitted when an opponent is paired.
- `RoundStart { room_id, problem, starts_at_ms, ends_at_ms, duration_ms, server_time_ms, seq }`: Starts the match timer and delivers the theorem statement.
- `Verdict { req_id, verdict, reason, message, diagnostics, elapsed_ms, check_skipped }`: Returns compiler output and relative line squiggles.
- `RoundEnd { room_id, outcome, winner_id, winning_proof, canonical_proof, elo_delta, duration_ms, seq }`: Concludes the game.
