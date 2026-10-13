# ProofBattle Rust Backend Architecture

The backend server is implemented in Rust using Axum and Tokio (`server/`). It manages WebSocket client connections, matchmaking, concurrent game rooms, and asynchronous verification job execution.

---

## 1. Concurrency Model: Room Actor Architecture (ADR-001)

Earlier prototypes protected match state using a global `Arc<Mutex<HashMap<RoomId, Room>>>`. This pattern caused global server stalls whenever a proof verification command blocked on external I/O.

ProofBattle implements the **Actor Model** for room lifecycle:
- Each match room runs as an independent `tokio::spawn`ed actor task owning its private state.
- External events (player submissions, forfeits, disconnections) are communicated via `mpsc::channel`.
- A global `DashMap<PlayerId, mpsc::Sender<RoomCommand>>` acts strictly as a lightweight, uncontentious routing table.
- When a match concludes or terminates, the actor task exits and its resources are dropped automatically.

```text
 Client WS Connection                   Global Router                     Room Actor Task
+---------------------+             +--------------------+             +--------------------+
| Player A WebSocket  | ----------> | DashMap Router     | ----------> | Tokio Room Task    |
| (Axum Handler)      |             | PlayerId -> Sender |             | Private State:     |
+---------------------+             +--------------------+             | - problem          |
                                                                       | - timer (ends_at)  |
 Client WS Connection                                                  | - submissions      |
+---------------------+                                                +--------------------+
| Player B WebSocket  | --------------------------------------------------------^
+---------------------+
```

---

## 2. Two-Lane Bounded Execution Queue (ADR-008)

During live gameplay, players type rapidly in Monaco Editor, generating frequent syntax and type checks. If both live diagnostics and match-deciding submissions shared a single FIFO worker pool, high check frequency would head-of-line block match submissions, causing timeouts.

The backend splits verification into two isolated lanes:
1. **`SubmitLane`**: Dedicated worker capacity for official `ProofRequest { intent: "Submit" }` frames. Protected against overload by failing closed.
2. **`CheckLane`**: Best-effort worker capacity for background `ProofRequest { intent: "Check" }` diagnostics. Automatically drops obsolete intermediate requests when a newer check arrives for the same player.

---

## 3. Dynamic Elo Matchmaker & Fair Play Policies

### Dynamic Bracket Expansion
When a player joins matchmaking (`QueueJoin`), the matchmaker searches within an initial rating window ($\pm 50$ Elo). Every 3 seconds, the window expands by $\pm 25$ Elo until a suitable opponent is found or the search times out.

### Fair Rating Forfeiture Window (ADR-010)
If a player disconnects or forfeits within the first 30 seconds of a round, the match is recorded as unrated. This prevents griefing and rating manipulation via instant disconnects.

---

## 4. Database Schema & Persistence

The backend uses **SQLx** with PostgreSQL.

### Core Tables
- `users`: Stores user identity, registered handle, creation timestamp.
- `ratings`: Tracks current Elo rating, rating deviation, peak rating, win/loss/draw records.
- `problems`: Verified Lean 4 problem catalog containing `id`, `goal`, `imports`, `difficulty`, `category`, and canonical solutions.
- `matches`: Historical match records, participant handles, ratings deltas, winning proofs, and durations.

### Migrations
Database migrations are maintained in `server/migrations/` and executed via:
```bash
make migrate
```

---

## 5. Code References

- Application Entry & Routing: [`server/src/main.rs`](../../server/src/main.rs)
- Room Actor Implementation: [`server/src/game/room.rs`](../../server/src/game/room.rs)
- Matchmaking Engine: [`server/src/game/matchmaker.rs`](../../server/src/game/matchmaker.rs)
- Verification Worker Pool: [`server/src/lean/runner.rs`](../../server/src/lean/runner.rs)
- Lexical AST Filter: [`server/src/lean/filter.rs`](../../server/src/lean/filter.rs)
- Database Operations: [`server/src/db/mod.rs`](../../server/src/db/mod.rs)
