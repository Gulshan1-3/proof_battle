# Architecture Decision Records (ADR)

This log records the foundational architectural decisions for ProofBattle.

---

# ADR-001: Room Actor Model for Concurrency
Status: accepted
Context: The initial prototype wrapped game state in `Arc<Mutex<HashMap>>` and held the lock across long-running asynchronous Lean proof execution, causing global server stalls and lock contention.
Decision: Use one `tokio::spawn`ed actor task per room owning its own state. A `DashMap<PlayerId, mpsc::Sender<RoomCommand>>` acts strictly as an uncontentious routing table.
Consequences: Eliminates lock ordering bugs, isolates room state mutation, guarantees FIFO processing of submissions per room, and makes room cleanup automatic upon actor termination. Requires channel-based message passing.
Alternatives rejected: Global `Arc<Mutex<HashMap>>` (causes blocking on hot async paths) and raw `DashMap` holding mutable match structures directly (leaves room lifecycle and lock-ordering races open).

---

# ADR-002: Client Submits Only Tactic Body
Status: accepted
Context: Allowing clients to submit entire Lean files enables declaration shadowing (`theorem goal : True := trivial`), custom imports, arbitrary `#eval` payloads, axioms, and overriding compiler safety options.
Decision: The client submits ONLY the tactic script lines (inside `by`). The server manages imports, safety compiler options, and the exact theorem statement template.
Consequences: Closes statement-shadowing, goal-swapping, and AST injection attacks by design. The server generates the full Lean file from static templates.
Alternatives rejected: Full-file submission with a lexical blacklist (blacklists are bypassable via Lean macro expansion and meta-programming tricks).

---

# ADR-003: Multi-Signal Verification and Fail-Closed Verdicts
Status: accepted
Context: Lean 4 produces only a warning on `sorry` by default and exits with code 0. Trusting process exit code alone allows `by sorry` to win every match instantly.
Decision: Inject `set_option warningAsError true` in the server preamble, scan stderr for `declaration uses 'sorry'`, enforce strict wall-clock timeout and Lean heartbeat limits (`set_option maxHeartbeats 200000`). Fail closed on non-zero exit code, timeout, or unexpected output.
Consequences: Prevents fraudulent wins while providing robust verdict generation. Requires stderr inspection for diagnostic warnings.
Alternatives rejected: Process exit code checking alone or using arbitrary `std::thread::sleep(2s)` (fails to catch `sorry` and blocks the async runtime).

---

# ADR-004: Server-Authoritative Game Clock
Status: accepted
Context: Transmitting decrementing seconds counters over WebSocket leads to client drift, latency discrepancies, and potential client-side clock tampering.
Decision: The server broadcasts `ends_at_ms` (Unix timestamp) on match start and periodically emits `ServerTime { server_time_ms }` synchronization pulses. The client derives remaining time locally for display.
Consequences: Guarantees server-authoritative round termination independent of client timing discrepancies.
Alternatives rejected: Client-driven countdown ticks or per-second server countdown messages (vulnerable to network jitter and client spoofing).

---

# ADR-005: SvelteKit 2 and Svelte 5 Runes for Frontend
Status: accepted
Context: ProofBattle requires high-frequency reactive state updates for editor diagnostics, real-time clock interpolation, and WebSocket events. Svelte 4 reactivity (`$:`, `export let`, `createEventDispatcher`) is legacy.
Decision: Build the frontend using SvelteKit 2 with Svelte 5 runes (`$props`, `$derived`, `$effect`, `onclick`, callback props).
Consequences: Provides fine-grained reactivity, simplified store bindings, and long-term maintainability.
Alternatives rejected: Svelte 4 legacy patterns (generates immediate technical debt) or heavier React runtime stacks.

---

# ADR-006: SessionStore Trait with In-Memory Seam
Status: accepted
Context: Matchmaking reconnects and session tracking need persistence, but forcing a hard Redis requirement on day one complicates testing and local development.
Decision: Define an asynchronous `SessionStore` trait from day one, backed initially by an in-memory `DashMap` implementation and swapped to Redis when scaling.
Consequences: Enables simple local development and testing while isolating the session persistence interface behind a clean 30-line abstraction seam.
Alternatives rejected: Hardcoding in-memory state in request handlers or requiring an active Redis cluster for basic unit/integration tests.

---

# ADR-007: Structured Typed Verdict Protocol
Status: accepted
Context: Sending raw compiler logs to clients forces fragile client-side regex parsing and causes collisions between live diagnostic checks and match submissions.
Decision: Return structured messages with request IDs (`req_id`), a typed `verdict` enum (`Accepted`, `Rejected`, `Timeout`, `InternalError`), machine-readable reason strings, elapsed time, and structured diagnostic markers.
Consequences: Enables Monaco Editor to directly highlight error ranges without parsing compiler text, while disambiguating asynchronous check vs submit responses.
Alternatives rejected: `ProofResult { success: bool, output: String }` (untyped, ambiguous, forces client to parse raw compiler stderr).

---

# ADR-008: Two-Lane Bounded Execution Queue
Status: accepted
Context: High-frequency live feedback requests (`Check` intent) can saturate worker pools, starving official match submissions (`Submit` intent) and causing match timeouts.
Decision: Separate execution into two lanes: `SubmitLane` with dedicated worker capacity that fails closed under load, and `CheckLane` with best-effort capacity that drops obsolete diagnostic requests.
Consequences: Protects match-critical progress from being starved by aggressive editor keystroke checks.
Alternatives rejected: Single shared FIFO worker queue (causes head-of-line blocking).

---

# ADR-009: Monorepo Directory Layout
Status: accepted
Context: The Rust crate initially lived inside a Mathlib checkout (`battle/`), mixing Cargo and Lake package managers in a single directory.
Decision: Establish a clear top-level layout: `server/` (Rust), `frontend/` (SvelteKit), `lean/` (Lean 4 environment), `infra/` (Docker/Nginx), `scripts/` (tooling), and `docs/` (ADR and environment docs).
Consequences: Clean build isolation and independent toolchains.
Alternatives rejected: Retaining mixed `battle/` directory.

---

# ADR-010: Fair Rating Forfeiture Window
Status: accepted
Context: Instant disconnects upon seeing a problem can be used to game rating algorithms or dodge opponents.
Decision: Forfeits occurring within the first 30 seconds of a match are recorded as unrated.
Consequences: Prevents rating inflation and griefing via instant disconnects.
Alternatives rejected: Instant rating deduction on transient disconnects during initial match countdown.

---

# ADR-011: Local Monaco Editor Bundling
Status: accepted
Context: Loading Monaco Editor via `@monaco-editor/loader` pulls remote assets from CDNs at runtime, breaking offline usage and strict CSP.
Decision: Bundle Monaco Editor directly via npm and Vite `?worker` imports.
Consequences: Enables self-hosted, offline operation with strict CSP compliance.
Alternatives rejected: CDN runtime script injection.

---

# ADR-012: Empirical Lean Worker Pool Scaling
Status: accepted
Context: Initial assumptions estimated cold Lean verification at 15s+, suggesting complex pre-warmed persistent LSP pools. Measured cold execution is ~2.1-3.7s.
Decision: Use a bounded pool of one-shot `lake env lean` processes initially; measure performance under load at Stage S9 before deciding on persistent worker daemons.
Consequences: Reduces early system complexity while relying on empirical performance measurements.
Alternatives rejected: Prematurely building complex persistent LSP socket pools prior to measuring baseline resource costs.
