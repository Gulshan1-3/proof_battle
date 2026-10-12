# ProofBattle Platform Architecture & Subsystems

The ProofBattle platform is a high-performance, real-time competitive gaming monorepo for formal mathematics. Two players race against each other to construct machine-checked proofs in Lean 4.

---

## 1. Subsystem Interaction Architecture

```text
       +----------------------------------------------------+
       |                   Browser Client                   |
       |  SvelteKit 2 + Svelte 5 Runes + Tailwind CSS v4    |
       |  Monaco Editor (Zero-CDN Offline Bundle)           |
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
    | Session Store |    | Matchmaker &  |    | Two-Lane Queue  |
    | (DashMap/Redis)    | Dynamic Elo   |    | Rate Limiter    |
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

---

## 2. Platform Subsystems

| Subsystem | Specification Document | Key Technologies |
| :--- | :--- | :--- |
| **Rust Backend Server** | [BACKEND.md](BACKEND.md) | Axum, Tokio, Actor Model (ADR-001), SQLx, PostgreSQL |
| **SvelteKit Frontend** | [FRONTEND.md](FRONTEND.md) | SvelteKit 2, Svelte 5 Runes, Monaco Editor, Tailwind CSS v4 |
| **WebSocket Wire Protocol** | [PROTOCOL.md](PROTOCOL.md) | JSON-framed duplex protocol, monotonic clock sync (ADR-004) |
| **Security & Sandbox** | [SECURITY_AND_SANDBOX.md](SECURITY_AND_SANDBOX.md) | AST filter, fail-closed judge, resource limits (ADR-002, ADR-003) |
| **Configuration** | [ENVIRONMENT.md](ENVIRONMENT.md) | Environment variables, ports, timeouts |

---

## 3. High-Level Match Lifecycle

1. **Connection & Handshake**: Client opens WebSocket connection, sends `Hello` frame, and receives authenticated `Welcome` frame with monotonic server timestamp.
2. **Matchmaking**: Player joins queue (`QueueJoin`). Matchmaker expands Elo window over time and pairs two players of similar skill into an isolated `Room` actor.
3. **Round Start**: Both clients receive `MatchFound` followed by `RoundStart` containing the formal goal statement, required imports, and server-authoritative round end timestamp (`ends_at_ms`).
4. **Live Coding & Verification**:
   - `intent: "Check"`: Keystroke feedback routed through the best-effort `CheckLane` to provide Monaco editor squiggles without consuming rate limits.
   - `intent: "Submit"`: Official submission routed through the prioritized `SubmitLane` with dedicated worker capacity.
5. **Resolution**:
   - First formally verified submission closes the match.
   - Server updates Elo ratings in PostgreSQL and broadcasts `RoundEnd` with ratings deltas, winning proof, and canonical Mathlib solution.
