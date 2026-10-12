# ProofBattle Documentation Hub

Welcome to the central documentation system for ProofBattle: a real-time, competitive theorem-proving platform powered by Lean 4, Rust/Axum, SvelteKit 5, and a 9.9M biological post-Transformer neural prover (`ProofBDH`).

---

## 1. Quick Navigation by Persona

Choose your reading path depending on your role and focus:

### 🔬 For ML Researchers & Reasoning Engineers
Explore the neural architecture, search algorithms, and empirical benchmarks:
1. [AI Prover Overview](model/README.md): System architecture and dataflow pipeline.
2. [ProofBDH Neural Architecture](model/PROOF_BDH.md): Hebbian synaptic fast-weights, $O(1)$ memory, 9.9M parameter budget.
3. [Diagnostic Best-First MCTS](model/DBF_MCTS.md): PUCT search, length-normalized BFS priors, in-tree compiler diagnostic self-repair.
4. [BM25 Premise Retriever](model/PREMISE_RETRIEVAL.md): Parameter-free inverted index over 102k Mathlib competition lemmas.
5. [Benchmarks & Integrity Audits](model/BENCHMARKS_AND_EVAL.md): miniF2F 27.05% test results, ablations, and zero-contamination audits.
6. [Simulation Server](model/SIMULATION_SERVER.md): High-concurrency HTTP API for 512+ battle arenas.

### 🦀 For Backend Engineers
Explore the real-time game server, actor concurrency, and persistence:
1. [Platform Subsystems Overview](platform/README.md): Architecture and match lifecycle.
2. [Backend Architecture](platform/BACKEND.md): Tokio actor-per-room model (ADR-001), Two-Lane rate limiter (ADR-008), database schema.
3. [WebSocket Protocol Spec](platform/PROTOCOL.md): Wire format, JSON frame invariants, and state machines.
4. [Security & Sandbox](platform/SECURITY_AND_SANDBOX.md): Lexical AST filters, fail-closed judge, resource limits.
5. [Architecture Decision Records](DECISIONS.md): ADR-001 through ADR-012.

### 🎨 For Frontend Engineers
Explore the reactive user interface, Monaco editor, and WebSocket client:
1. [Frontend Architecture](platform/FRONTEND.md): SvelteKit 2 + Svelte 5 runes, Monaco Editor bundling, reactive state.
2. [WebSocket Protocol Spec](platform/PROTOCOL.md): Generated TypeScript interfaces and event contracts.
3. [Local Development](runbooks/LOCAL_DEVELOPMENT.md): Running the Vite dev server with live reload.

### 🛠️ For DevOps & Operations Engineers
Set up environments, run benchmarks, and maintain CI/CD:
1. [Local Development Runbook](runbooks/LOCAL_DEVELOPMENT.md): Docker Compose, PostgreSQL migrations, service startup.
2. [Benchmarking & Load Testing](runbooks/BENCHMARKING_AND_LOAD.md): Running `anchor_bench` and `loadgen` stress tests.
3. [Adding & Verifying Challenges](runbooks/ADDING_CHALLENGES.md): Problem authoring and `verify_problems` CLI.
4. [CI/CD Pipeline Runbook](runbooks/CI_CD_MAINTENANCE.md): GitHub Actions workflows, Mock Lake runner, test suite maintenance.

### 🎓 For Players & Competitors
Learn how to write formal proofs in Lean 4:
- [Lean 4 Proof Guide](../learn/README.md): Practical tactics guide (`intro`, `simp`, `ring`, `linarith`, `omega`, `aesop`).

---

## 2. Complete Documentation Sitemap

```text
docs/
├── README.md                             # Central Documentation Index (This Document)
├── DECISIONS.md                          # Architectural Decision Records (ADR-001 to ADR-012)
│
├── model/                                # AI Prover & Neural Reasoning Suite
│   ├── README.md                         # Prover Subsystem Overview
│   ├── PROOF_BDH.md                      # ProofBDH Architecture (Hebbian Synapses, 9.9M Params)
│   ├── DBF_MCTS.md                       # Diagnostic Best-First MCTS & PUCT Search
│   ├── PREMISE_RETRIEVAL.md              # BM25 Premise Retriever (102,278 Mathlib Lemmas)
│   ├── LEAN_REPL_PIPELINE.md             # Persistent Lean 4 REPL & Subprocess Sandbox
│   ├── AUTOFORMALIZATION.md              # Neuro-Symbolic LaTeX/NL to Lean 4 Translator
│   ├── BENCHMARKS_AND_EVAL.md            # miniF2F Results (27.05%), Ablations & Audits
│   └── SIMULATION_SERVER.md              # Simulation HTTP API & Concurrency Model
│
├── platform/                             # ProofBattle Competitive Platform
│   ├── README.md                         # Platform Architecture Overview
│   ├── BACKEND.md                        # Rust/Axum Backend, Room Actor Model, DB
│   ├── FRONTEND.md                       # SvelteKit 2, Svelte 5 Runes, Monaco Editor
│   ├── PROTOCOL.md                       # WebSocket Wire Protocol v1 (Normative)
│   ├── SECURITY_AND_SANDBOX.md           # Lexical AST Filter, Judge Sandbox, Anti-Cheat
│   └── ENVIRONMENT.md                    # Environment Variables & Configuration
│
├── runbooks/                             # Operational Runbooks & Procedures
│   ├── README.md                         # Runbooks Catalog & Maintenance Cadence
│   ├── LOCAL_DEVELOPMENT.md              # Local Dev Setup (Docker, Cargo, Vite)
│   ├── ADDING_CHALLENGES.md              # Adding & Verifying Lean 4 Problem Catalogs
│   ├── BENCHMARKING_AND_LOAD.md          # anchor_bench & loadgen Performance Profiling
│   └── CI_CD_MAINTENANCE.md              # GitHub Actions CI, Mock Lake Runner, Caching
│
└── archive/                              # Historical Documents & Staged Prompts
    └── DEVELOPMENT_STAGES.md             # Archived Stage 0-9 Build Prompt Pack
```

---

## 3. Documentation Governance & Quality Policy

- **Docs-as-Code**: All technical documentation is version-controlled directly in Git.
- **Link Verification**: Internal markdown links and headers are verified via `make docs-check`.
- **Change Contract**: Any PR altering protocols, model architectures, database schemas, or CLI flags must update the corresponding markdown file in `docs/`.
