# ProofBattle — Staged Development Plan & LLM Prompt Pack

> **Purpose**: a dependency-ordered, gate-driven build plan for ProofBattle, written as copy-pasteable
> prompts for a coding LLM (Claude Code / Codex / Cursor / any tool with repo access).
>
> **Sources of truth** (both read in full before this plan was written):
> - `proof_battle_design.md` — backend / architecture design (707 lines)
> - `frontend_design.md` — frontend / UX design (1289 lines)
>
> **Supersedes**: `RUST_BACKEND_PROMPTS.md`, `FRONTEND_PROMPTS.md`,
> `proof_battle_implementation_prompts.md`, `frontend_implementation_prompts.md`. Those are flat 15-step
> lists. This document restructures the same work into gated stages, adds the facts that were missing
> (measured, not assumed), and resolves the contradictions and security holes both design docs leave open.

---

## How To Use This Document

### The driving protocol

1. **One stage at a time.** Never paste two stage prompts into one session. Stage N+1 depends on
   invariants established in Stage N, and a single context window cannot hold ten stages of decisions
   coherently.
2. **Fresh session per prompt** (at minimum per stage). Every prompt below is written to be
   self-contained, but a fresh context is cheaper and less contaminated by earlier wrong turns.
3. **Prepend the Constitution (§1) to the first prompt of every session.** It carries the invariants
   that must survive context compaction.
4. **Paste exactly one fenced block.** The blocks are the deliverable. Everything outside them is
   reference material for you, not for the model.
5. **The agent must run the acceptance commands itself.** Every prompt ends with executable
   acceptance criteria. If it claims success without pasting command output, it failed.
6. **Gate, then commit.** Only after the stage gate passes:
   ```bash
   git add -A && git commit -m "stage S<n>: <name>" && git tag stage-s<n>
   ```
7. **On failure, do not patch by hand.** Re-paste the same prompt with the failure appended:
   `The previous attempt failed. Here is the actual output: ...`

### Why these prompts are engineered this way

| Technique | Where | Why it matters |
|---|---|---|
| **Explicit context pack** | Every prompt opens with `CONTEXT`: exact paths, versions, measured numbers | LLMs guess. A guessed path or a guessed Lean version costs an entire failed stage. |
| **Single deliverable per prompt** | Each prompt states one artefact | Multi-artefact prompts produce shallow, internally inconsistent output. |
| **Executable acceptance criteria** | `ACCEPTANCE` in every prompt, always shell | Turns "is it done?" from a judgement call into a fact. Highest-leverage technique in this document. |
| **Explicit `DO NOT`** | Every prompt | Prevents the default LLM failure: fixing things you did not ask about, refactoring adjacent code, "improving" security controls that were deliberate. |
| **`IF BLOCKED` stop condition** | Every prompt | Forces the model to surface a blocker instead of inventing a plausible answer. Most dangerous failure mode in the security-critical stage. |
| **`DELIVERABLE` output contract** | Every prompt | Makes the result auditable: exact file list plus a fixed report format. |
| **Measured baselines** | `CONTEXT` blocks | The agent can check its work against a known number instead of "looking right". |
| **Deviation register (ADR log)** | Every stage that contradicts a design doc | Design docs are not law. Deviations are written down with rationale so later stages do not "fix" them back. |
| **Negative requirements with reasons** | Stage 3 mostly | A denylist written without a reason gets helpfully weakened by the model in the next stage. |

### Estimated effort

| Stage | Focus | Solo (senior) | With LLM |
|---|---|---|---|
| S0 | Baseline + verification harness | 0.5 d | 1–2 h |
| S1 | Repo architecture + path config | 0.5 d | 2–3 h |
| S2 | Backend core correctness | 2 d | 3–4 h |
| S3 | Verification soundness + sandbox | 3 d | 5–7 h |
| S4 | Protocol contract + codegen | 1 d | 2–3 h |
| S5 | Postgres + problem supply | 2 d | 4–5 h |
| S6 | Game engine v2 | 3 d | 5–7 h |
| S7 | Frontend foundation | 2 d | 3–4 h |
| S8 | Editor / Lean UX | 2 d | 3–4 h |
| S9 | Polish, deploy, load test | 3 d | 5–6 h |

---

## Stage Map

```
S0  Baseline & Verification Harness
     └─> S1  Repository Architecture & Config
             └─> S2  Backend Core Correctness
                     └─> S3  Verification Soundness & Sandbox   ◀── CRITICAL PATH
                             └─> S4  Protocol Contract & Codegen
                                     ├─> S5  Problem Supply (Postgres)
                                     │        └─> S6  Game Engine v2
  S1 ──────────────────────────────────> S7  Frontend Foundation
                                              └─> S8  Editor / Lean UX
                                                      └─> S9  Polish, Deploy, Load Test
```

### Why this order (dependency reasoning, not aesthetics)

- **S0 before everything.** The Lean toolchain in this repo is in an ambiguous state (§2). Any refactor
  done before you can prove a known-good proof verifies in under 5 seconds is unfalsifiable work. The
  harness is the measuring instrument for every later gate.
- **S1 before S2** because S2 introduces a `Config` struct with paths. While the crate is still buried
  inside a Mathlib checkout at `battle/`, every path in that config is a lie. Move first, parameterise second.
- **S2 before S3** because the sandbox needs a bounded concurrency seam, per-request correlation, and a
  clean async `verify` API. Putting a container behind a blocking call inside a mutex-guarded matchmaker
  is strictly harder than putting it behind a clean API.
- **S3 before S4** because the protocol must carry structured diagnostics and machine-readable verdict
  reasons, which are *outputs* of the verification layer. Designing the protocol first means inventing a
  shape you will immediately change.
- **S4 before S5/S6/S7/S8** because the protocol is the seam both sides compile against. Generating the
  TypeScript types from the Rust types is what makes drift impossible rather than unlikely.
- **S5 before S6** because difficulty-bracket selection and ratings need real, verified content. S6
  built on 3 hardcoded problems would be built on a false premise.
- **S7/S8 after S4** because the frontend compiles against generated types. Building against hand-written
  types guarantees a rewrite.
- **S9 last** because load testing, TLS/QUIC, and observability are only meaningful once the pipeline is
  correct and instrumented.

### Deviation register (summary — full ADRs land in `docs/DECISIONS.md` at S1)

These are the places this plan **intentionally contradicts the design docs**.

| # | Design doc says | This plan does | Why |
|---|---|---|---|
| ADR-001 | `Arc<Mutex<HashMap>>` → `DashMap` (design §1.4) | **Room actor model**: one `tokio::spawn`ed task per room owns its state; a `DashMap<PlayerId, Sender>` is only a routing table | DashMap removes lock *contention* but keeps lock *ordering* bugs. The actual bug in `main.rs` is holding the challenge-map lock across the entire proof run — that is a scoping bug, not a data-structure bug. An actor removes both, makes room cleanup a task drop, and makes verdict ordering FIFO for free. |
| ADR-002 | Client submits the whole proof file | **Client submits only the tactic body** (lines inside `by`); server owns imports, statement, and all `set_option` | Kills goal-swap, statement-shadowing, extra-import, `set_option`, `axiom`, and `attribute [instance]` attacks as a class instead of as individual denylist entries. |
| ADR-003 | `sleep(2s)`, exit code == verdict | `set_option warningAsError true` in the server preamble **+** stderr scan for `declaration uses 'sorry'` + wall timeout + heartbeat limit | **Measured**: `by sorry` exits 0 and wins the game under the design doc's rule. Exit code alone is not a soundness signal. |
| ADR-004 | `TimerUpdate { seconds_remaining }` | Server sends `ends_at_ms` + `server_time_ms`; client derives remaining time | A decrementing counter lets a client desync, and a tampered client can claim infinite time. |
| ADR-005 | Svelte 4 idioms (`export let`, `$:`, `on:click`, `createEventDispatcher`) | **SvelteKit 2 + Svelte 5 runes** (`$props`, `$derived`, `$effect`, `onclick`, callback props) | Svelte 4 idioms are legacy; new code written in them must be migrated before it ships. Mapping table is a deliverable of S7-P1. |
| ADR-006 | In-memory sessions, "Redis later" | `SessionStore` trait with an in-memory impl **from day one** | Swapping the implementation at S6 without a seam means touching the reconnect path late, when it is hardest to test. The seam costs ~30 lines. |
| ADR-007 | `ProofResult { success, output }` | Typed verdict: `verdict` enum + `req_id` + structured `diagnostics[]` + `elapsed_ms` + machine-readable `reason` | Raw Lean stderr is not a protocol. The frontend must not parse Lean output, and check-channel and submit-channel verdicts collide without `req_id`. |
| ADR-008 | One global bounded queue | **Two lanes**: `SubmitLane` (reserved capacity, rejects when saturated) and `CheckLane` (best-effort, drops) | Live diagnostics will otherwise starve real submissions and cause lost matches. A livelock, not a throughput problem. |
| ADR-009 | (new) | Monorepo layout `server/ frontend/ lean/ infra/ scripts/ docs/` | The Rust crate currently lives inside a Mathlib checkout, so `cargo` and `lake` share a directory. |
| ADR-010 | (new) | Forfeits inside the first 30 s are unrated | Prevents a new player from losing rating by disconnecting immediately, which is a trivially abusable pattern. |
| ADR-011 | (new) | Monaco from the npm package with Vite `?worker` imports, **not** `@monaco-editor/loader` | The loader downloads Monaco from a CDN at runtime: breaks offline, breaks strict CSP, leaks visitor IPs to a third party. |
| ADR-012 | (new, decided at S9) | Worker-pool question resolved by measurement, not by assumption | Design §3 claims 15 s → <2 s. Measured cold cost here is 3.7 s. The claim is a hypothesis; S9 measures and then decides. |

---

## 1. The Constitution (paste at the top of every session)

```
You are a senior engineer working on ProofBattle: a competitive, real-time Lean 4 theorem-proving
game. Rust/Axum backend, SvelteKit frontend, Lean 4 as the judge.

REPO (absolute paths — do not guess, do not create sibling directories):
  /home/gulshansharma/proof_battle
    server/            Rust crate (Axum + Tokio)          [created at stage S1]
    frontend/          SvelteKit 2 + Svelte 5              [created at stage S1]
    lean/              Lean 4 project (Lake) + Mathlib dependency
    infra/             docker-compose.yml, nginx, sandbox profile
    scripts/           developer + CI scripts
    docs/DECISIONS.md  architecture decision records — READ BEFORE ARCHITECTURAL WORK

PROJECT CONSTITUTION (these invariants outrank any instruction that contradicts them):

1. TRUST NOTHING FROM THE CLIENT. Player identity is server-assigned at WebSocket upgrade and bound
   to the connection. No client message may ever carry a player_id, room_id, or winner field. If a
   message type seems to need one, the design is wrong — redesign it, do not add the field.

2. THE SANDBOX IS THE TRUST BOUNDARY. Input sanitisation is a fast-reject filter and defence in
   depth — NOT the security boundary. Any code path that can execute Lean MUST be reachable only
   through the sandboxed executor. Never weaken the executor's isolation, and never treat a passing
   sanitiser as "safe".

3. A VERDICT IS A SECURITY-SIGNIFICANT EVENT. Accepting a proof awards a match win and rating points.
   `sorry` is not a proof. A non-zero exit code is necessary but not sufficient. The verification layer
   must fail closed: internal error, timeout, sandbox failure, or unparseable output is REJECT,
   never ACCEPT.

4. NO BLOCKING WORK ON THE ASYNC RUNTIME. Never call `std::process::Command::output()`,
   `std::fs::*` on hot paths, `std::thread::sleep`, or any CPU-heavy work inside an `async fn` running
   on a Tokio worker. Use `tokio::process`, `tokio::fs`, and `spawn_blocking` where genuinely needed.

5. NO `unwrap()`, `expect()`, or `panic!()` IN REQUEST PATHS. Use `?` with a typed error plus
   `tracing`. A panic in a WebSocket handler must not take down the matchmaker.

6. EXPLICIT CONCURRENCY MODEL. Document in a comment the ownership rule for every shared structure:
   who owns it, who may touch it, what serialises access. Prefer an actor per room over shared mutable
   state. If you introduce a `Mutex`, justify why an actor or a `DashMap` does not work.

7. OBSERVABILITY IS PART OF THE FEATURE. Every match transition, verdict, and sandbox rejection emits
   a `tracing` event with structured fields (room_id, player_id, elapsed_ms, reason). No `println!` in
   library code.

8. MEASURE, DO NOT GUESS. Any latency or throughput claim must come from a command you actually ran,
   with the output pasted into your report.

9. RESPECT THE DECISION REGISTER. Read `docs/DECISIONS.md` first. If an instruction in the task
   contradicts an ADR, the ADR wins — implement around it and raise the conflict in your report.

10. SCOPE DISCIPLINE. Implement only what this task asks. Do not refactor adjacent code, do not upgrade
    unrelated dependencies, do not add abstractions for hypothetical futures, do not add comments that
    restate the code. If you believe a change outside scope is necessary, describe it under
    "Recommended next work" in your report instead of doing it.

RESPONSE FORMAT (always, in this order):
  1. PLAN — 3-6 bullets on how you will do it, before writing code.
  2. WORK — perform the task, running the verification commands.
  3. EVIDENCE — the actual terminal output of every acceptance command you ran, verbatim.
  4. REPORT — files created/modified (one line of purpose each), decisions made, deviations from the
     ADR register, known limitations, and any blocker you hit.
Never claim a command passed without pasting its output. Never invent file contents you did not read.
```

---

## 2. Environment Baseline (measured on this machine, not assumed)

Every prompt below relies on these facts. They were measured. **If a later stage changes one of them,
`docs/ENVIRONMENT.md` (created at S0) must be updated in the same commit.**

| Fact | Value | Established by |
|---|---|---|
| Lean toolchain | `leanprover/lean4:v4.21.0-rc3` | `cat lean-toolchain` |
| Lake | 5.0.0 (Lean 4.21.0-rc3) | `lake --version` |
| Rust | 1.90.0 — edition 2024 OK (needs ≥ 1.85) | `cargo -V` |
| Node / npm | v22.14.0 / 10.9.2 | `node -v`, `npm -v` |
| Docker | daemon 27.5.1 available | `docker info` |
| PostgreSQL | client 16.8 present, **no server running** | `pg_isready` → "no response" |
| Crate location | `battle/src/` + `battle/Cargo.toml`, **inside a Mathlib checkout** | `battle/` holds `Mathlib/`, `MathlibTest/`, `Archive/`, `Cache/`, `lakefile.*` **and** the Rust crate |
| Prebuilt oleans | 6550 `.olean` under `battle/.lake/build/lib/lean/` | `find .lake/build -name '*.olean' \| wc -l` |
| Mathlib sources | `lean/Mathlib` = 86 MB, 6549 `.lean`, **0 `.olean`** | `find Mathlib -name '*.lean' \| wc -l` |
| Lakefile ambiguity | **Both** `lakefile.lean` and `lakefile.toml` exist; Lake picks `.lean` and prints `info: [root]: lakefile.lean and lakefile.toml are both present; using lakefile.lean` on **every** invocation | observed on every run |
| Fine-grained import works | `import Mathlib.Data.Nat.Basic` → exit 0 in **3.7 s** wall | measured |
| **Aggregate import FAILS** | `import Mathlib` → `error: object file '.../Mathlib.olean' does not exist` | measured |
| Unindented tactics are legal | `theorem goal : … := by` then `intro n` at **column 0** → exit 0 | measured |
| Unsolved goals fail | `by` + `intro n` with no closer → `error: unsolved goals`, exit 1 | measured |
| **`sorry` PASSES** | `by` + `  sorry` → `warning: declaration uses 'sorry'`, **exit 0** | measured |
| `warningAsError` fixes it | `set_option warningAsError true` + `sorry` → `error: declaration uses 'sorry'`, exit 1 | measured |
| Option downgrade not reachable | `set_option warningAsError false` inside a tactic block → `error: unexpected token 'sorry'; expected 'in'` (Lean forces `set_option … in term` scoping) | measured |
| `#eval` is live | `#eval IO.getEnv` compiles and Lean attempts evaluation (fails only on a missing `ToString` instance) → arbitrary code executes in the judge | measured |
| Statement shadowing caught by Lean | appending `theorem goal : True := trivial` → `error: 'goal' has already been declared`, exit 1 | measured |
| Git state | 11 tracked files; `.gitignore` hides all prompt `.md` files and `battle/*` except `battle/src/` | `git ls-files` |

### Consequences that shape the whole plan

1. **The judge is compromised today.** With the current code, a player types `sorry` and wins every
   match. This is the most urgent finding in the document and the reason S3 is on the critical path and
   is the heaviest stage.
2. **Cold verification is ~3.7 s**, not the 5–15 s the design doc assumes and not the 15 s cold start of
   an un-warmed pool. The design doc's justification for a pre-warmed LSP worker pool (design §3) is
   therefore **not justified at this scale**. A bounded pool of one-shot `lake env lean` processes is
   sufficient until measurement says otherwise (ADR-012, decided at S9-P1).
3. **`import Mathlib` cannot be used.** Every problem must declare fine-grained imports and must be
   verified against the exact import list it ships with. The design doc's Mathlib extraction script
   (design §5) does `import Mathlib` and **will not run** until that aggregate olean is built. S5-P2
   rewrites it to walk per-module import lists.
4. **Do not rebuild Mathlib.** 6550 oleans are already built. `lake build Mathlib` from scratch costs
   20–60 minutes and produces a different layout. S0 must snapshot and protect this cache.

---

# Stage S0 — Baseline, Verification Harness, Repo Hygiene

**Goal**: produce a falsifiable measuring instrument and an honest record of the environment, *before*
changing any architecture.

**Why now**: every later gate ("verification still works after the move", "this stage did not regress
latency") is checked against this harness. Build it later and the early stages become unverifiable.

**Exit gate**
- `scripts/lean_verify.sh` prints `VERDICT: ACCEPT` for a known-good proof and `VERDICT: REJECT`
  (currently *incorrectly* ACCEPT — that is the recorded bug) in under 10 s.
- `docs/ENVIRONMENT.md` and `docs/DECISIONS.md` exist; no Lake `info:` line on stderr any more.
- Corpus runs green except the two deliberate, documented `sorry` failures.
- `git status` clean, tag `stage-s0`.

---

## S0-P1 — Environment report, decision register, Lake hygiene

```
CONTEXT
Repo: /home/gulshansharma/proof_battle
State: the Rust crate lives at battle/src/ inside a Mathlib 4 checkout. Both battle/lakefile.lean and
battle/lakefile.toml exist; Lake resolves the .lean one and prints
  info: [root]: lakefile.lean and lakefile.toml are both present; using lakefile.lean
on EVERY invocation. No CI, no env config, no docs directory. The Lean olean cache (6550 files) is
precious — rebuilding Mathlib costs 20-60 minutes and MUST NOT happen.

TASK
Create the documentation and configuration baseline. Change no Rust logic and no Lean code.

STEPS
1. Inspect both lakefiles: `cat battle/lakefile.lean` (package "proofenv", requires mathlib from git)
   and `cat battle/lakefile.toml` (also "proofenv", different targets: lean_lib Proofenv + lean_exe
   proofenv root Main). KEEP lakefile.lean (Lake prefers it; the .toml is dead configuration that
   misleads readers) and DELETE lakefile.toml. Verify with `git diff --stat` that you deleted the right
   file. Re-run `lake env lean --version` and paste output proving the info line is gone.
2. Create docs/ENVIRONMENT.md containing the measured facts from the "Environment Baseline" table of
   the plan you were given (toolchain versions, olean count, the import-Mathlib failure, the
   sorry-passes finding). Every row needs a "how to reproduce" command. Mark clearly which facts are
   measured and which are assumed.
3. Create docs/DECISIONS.md as an ADR log. Seed it with ADR-001 … ADR-008 exactly as given in the
   plan's deviation register, each in this shape:
     # ADR-NNN: <title>
     Status: accepted
     Context: <what forced the decision>
     Decision: <what we do>
     Consequences: <what this makes harder, and what it makes possible>
     Alternatives rejected: <what the design doc proposed, and one concrete reason it is worse>
4. Create .env.example (committed) documenting every environment variable the system will need, grouped
   by subsystem (server, database, session store, lean executor, sandbox, frontend/public). Every
   variable gets a one-line comment stating its default and whether changing it in production is safe.
   Real values go in .env (gitignored).
5. Create scripts/env_report.sh: a POSIX shell script printing the environment table as key/value
   lines (versions, olean count, import smoke test, docker availability, postgres availability). It
   must exit non-zero if `lake` is missing or if the Mathlib olean count is 0. chmod +x it.

DO NOT
- Do not modify any file under battle/src/.
- Do not run `lake build`, `lake clean`, or anything that writes into .lake/ except `lake env lean`.
- Do not add a Rust dependency.
- Do not add a CI workflow yet (that is S9).

ACCEPTANCE
  cd /home/gulshansharma/proof_battle
  bash scripts/env_report.sh                                   # exits 0, prints all rows
  ls docs/ENVIRONMENT.md docs/DECISIONS.md .env.example
  cd battle && lake env lean --version 2>&1 | grep -c 'both present'    # must print 0
  cd .. && git status --short          # only docs/, scripts/, .env.example, lakefile.toml deleted

IF BLOCKED
If deleting lakefile.toml changes Lean resolution behaviour in any way, do NOT delete it. Move it to
battle/lakefile.toml.disabled, record the finding in docs/ENVIRONMENT.md, and continue.

DELIVERABLE
Files created: docs/ENVIRONMENT.md, docs/DECISIONS.md, .env.example, scripts/env_report.sh
File deleted: battle/lakefile.toml (or .disabled — state which)
Plus the standard PLAN / WORK / EVIDENCE / REPORT block.
```

## S0-P2 — The Lean verification harness (and proving the judge is broken)

```
CONTEXT
This is the measuring instrument for every later stage. The design doc's verdict rule is
"process exit code 0 == proof accepted". I measured that this is UNSOUND: a tactic body of `sorry`
produces `warning: declaration uses 'sorry'` and exit code 0, so a player wins by typing `sorry`.
The harness must measure the CURRENT (buggy) behaviour and the TARGET behaviour, and be reusable by
S3 to prove the bug is fixed.

Measured baselines on this machine (use these to validate your harness):
  import Mathlib.Data.Nat.Basic + theorem goal : ∀ n : ℕ, n + 0 = n := by / intro n / simp
      -> 3.7 s wall, exit 0
  same, body `intro n` only        -> error: unsolved goals, exit 1
  same, body `sorry`              -> warning: declaration uses 'sorry', exit 0
  import Mathlib                   -> error: object file '.../Mathlib.olean' does not exist

TASK
Build scripts/lean_verify.sh: a deterministic, non-interactive Lean verification CLI used by humans
and by the Rust integration tests.

SPECIFICATION
  Usage: scripts/lean_verify.sh <file.lean>
  - Reads the given .lean file and runs `lake env lean <file.lean>` with the working directory set to
    the Lean project root ($PB_LEAN_DIR, falling back to ../lean then ../battle relative to the
    script). Do not hardcode a single path.
  - Emits EXACTLY these lines on stdout, in this order, and nothing else on stdout:
      EXIT_CODE=<int>
      DURATION_MS=<int>
      STDOUT_BYTES=<int>
      STDERR_BYTES=<int>
      USES_SORRY=<true|false>
      FAILED_TO_COMPILE=<true|false>
      TIMED_OUT=<true|false>
      VERDICT=<ACCEPT|REJECT>
  - VERDICT logic (this is the TARGET rule; S3 ports it into Rust):
      ACCEPT iff EXIT_CODE==0 AND USES_SORRY==false AND TIMED_OUT==false
    USES_SORRY is true when stderr matches "declaration uses 'sorry'" or "sorryAx" (grep -E,
    case-sensitive, on combined stderr).
  - Exits 0 if VERDICT==ACCEPT, 1 if VERDICT==REJECT, 2 on harness error (missing file, no lake).
  - Enforce a wall-clock timeout with `timeout 30`. If it fires: TIMED_OUT=true, VERDICT=REJECT.
  - Do NOT use `lake build`. Do NOT set LEAN_PATH by hand. Do NOT cd outside the project dir.

  Then create scripts/fixtures/ with these exact files, and scripts/verify_corpus.sh that runs the
  harness over every fixture and prints a table `name  expected  actual  PASS|FAIL`:
    accept_add_zero.lean       body closes the goal with `simp`
    accept_add_comm.lean       body `intro a b` + `simp [Nat.add_comm]`
    reject_unsolved.lean       body `intro n` only
    reject_sorry.lean          body `sorry`               expected REJECT  (currently ACCEPT — the bug)
    reject_sorry_embedded.lean  body `intro n` + `sorry`  expected REJECT
    reject_syntax.lean         body `this is not lean`    expected REJECT
    reject_unknown_tactic.lean body `intro n` + `bogus_tactic`  expected REJECT
    skip_import_mathlib.lean   uses `import Mathlib`     expected REJECT (documents the aggregate failure)

DO NOT
- Do not change the verdict logic to make the corpus pass. The two reject_sorry* cases are EXPECTED TO
  FAIL right now, because the current judge is unsound. Record that failure; do not hide it.
- Do not add `set_option warningAsError true` to the fixtures — fixtures must reflect what a naive
  wrapper produces. S3 introduces the warningAsError preamble inside the Rust wrapper.
- Do not touch battle/src/.

ACCEPTANCE
  cd /home/gulshansharma/proof_battle
  bash scripts/lean_verify.sh scripts/fixtures/accept_add_zero.lean     # VERDICT: ACCEPT, exit 0
  bash scripts/lean_verify.sh scripts/fixtures/reject_sorry.lean        # VERDICT: ACCEPT, exit 0  <- the bug
  bash scripts/lean_verify.sh scripts/fixtures/reject_unsolved.lean     # VERDICT: REJECT, exit 1
  bash scripts/verify_corpus.sh       # 6 pass, 2 reject_sorry* FAIL (documented)
  Total wall time under 60 s. Paste the timing.

IF BLOCKED
If `lake env lean` cannot be invoked from a shell script (elan PATH issues), source ~/.elan/env if
present and document the requirement in docs/ENVIRONMENT.md. Do not vendor a Lean binary.

DELIVERABLE
scripts/lean_verify.sh, scripts/verify_corpus.sh, scripts/fixtures/*.lean (8 files).
The report MUST state verbatim: "reject_sorry currently returns ACCEPT — unsound judge confirmed."
```

## S0-P3 — Gitignore and the baseline commit

```
CONTEXT
.gitignore currently ignores all prompt .md files and `battle/*` except `battle/src/`. That was right
for a docs-only prototype and will fight us at S1, when the crate moves to server/ and we start
committing Lean project files, infra, and the frontend.

TASK
Rewrite .gitignore for the final monorepo layout, then commit the S0 baseline.

STEPS
1. Read the current .gitignore and explain in one line why each rule exists. Keep what still applies,
   drop what contradicts the target layout.
2. The new .gitignore must cover:
   - Rust: target/, *.rs.bk
   - Node: node_modules/, .svelte-kit/, build/, dist/, .vite/  (package-lock.json is NOT ignored —
     lockfiles are committed)
   - Lean: *.olean, *.ilean, .lake/build/, proofs/  (proofs/ holds generated player submissions)
   - Local config: .env, .env.local, and `!.env.example`
   - Editors/OS: .DS_Store, .vscode/, .idea/, *.swp
   - Test output: coverage/, test-results/, playwright-report/
   - Deliberately NOT ignored: docs/*.md (including this prompt pack), infra/, scripts/, .env.example
3. Verify with `git status --ignored --short | head -40` that important untracked items are visible
   and junk is hidden.
4. Run `git check-ignore -v` on battle/.lake/build/lib/lean/Mathlib.olean, .env, target, node_modules.
   Each must report a rule.

DO NOT
- Do not `git add -f` anything .gitignore excludes.
- Do not commit .lake/, node_modules/, or target/.
- Do not amend existing history.

ACCEPTANCE
  git check-ignore -v battle/.lake/build/lib/lean/Mathlib.olean .env target node_modules
  git status --short          # clean after the commit
  git log --oneline -3

DELIVERABLE
.gitignore, then:
  git add -A && git commit -m "stage S0: environment baseline, lean verification harness, decision register" \
    && git tag stage-s0
Report the commit SHA and the tag.
```

---

# Stage S1 — Repository Architecture & Path Configuration

**Goal**: one obvious home for each artefact, and a single source of truth for paths so that no later
prompt has to hardcode `battle/`.

**Why now**: S2's `Config` struct needs real paths; S5/S6 need a database URL; the frontend needs a
WebSocket URL. Every one of those is currently implicit.

**Exit gate**
- Layout is `server/ frontend/ lean/ infra/ scripts/ docs/` and nothing sits in an ambiguous place.
- `make verify-harness` works from a clean shell in the repo root.
- The S0 corpus passes identically before and after the move: same accept/reject set, every
  `DURATION_MS` within ±20%.
- `cargo clippy` is clean and the server boots and upgrades `/ws`.

---

## S1-P1 — Monorepo layout decision and execution

```
CONTEXT
Today the Rust crate (battle/src/, battle/Cargo.toml) sits inside a Mathlib 4 checkout (battle/
contains Mathlib/, MathlibTest/, Archive/, Cache/, lakefile.lean AND src/ + Cargo.toml). This
co-location is why paths are confusing and why `lake` and `cargo` share a directory.

TARGET LAYOUT (create exactly this at /home/gulshansharma/proof_battle):
  server/       Rust crate: Cargo.toml + src/
  frontend/     SvelteKit app (placeholder only; S7 builds it)
  lean/         the Lean/Lake project = today's battle/ minus src/ and Cargo*.toml
  infra/        docker-compose.yml, nginx/, sandbox/  (empty dirs with .gitkeep)
  scripts/      (exists from S0)
  docs/         (exists from S0)

TASK
Perform the move, preserving the Lean build cache, as a single documented operation.

STEPS
1. BEFORE moving anything, capture ground truth and keep it in your report:
     cd battle && bash ../scripts/verify_corpus.sh
   Save the full output. This is the "before" measurement.
2. Move with `git mv` for tracked files, plain `mv` for untracked:
     mkdir -p server/src lean
     git mv battle/src/*.rs server/src/
     mv battle/{Mathlib,MathlibTest,Archive,Cache,DownstreamTest,LongestPole,Shake,widget,
               Counterexamples.lean,Counterexamples,Mathlib.lean,Main.lean,docs.lean,
               lakefile.lean,lean-toolchain,lake-manifest.json,.lake,mathlib4,proofs} lean/
   - Do NOT move battle/Cargo.lock (regenerate it) and do NOT move battle/target — delete it:
     `rm -rf battle/target` (hundreds of MB of stale artifacts).
   - Handle collisions explicitly: if proofs/ or .lake/ already exists at the destination, merge and
     report exactly what you merged.
3. Now run `cd lean && lake env lean --version`, then from the repo root `bash scripts/verify_corpus.sh`.
   Paste both outputs. REQUIREMENT: the accept/reject verdicts must be IDENTICAL to the "before" run
   and every DURATION_MS within ±20%. If they are not, you broke the olean cache: STOP, revert, and
   report. Do not attempt a `lake build` to "fix" it.
4. Create server/Cargo.toml from battle/Cargo.toml:
   - package name `battle` -> `proof-battle-server` (keep edition = "2024")
   - DROP the `tokio-tungstenite` dependency (axum's `ws` feature already provides it; it is unused)
   - keep everything else; do NOT add any dependency in this stage
5. Create the frontend placeholder: frontend/README.md ("S7 creates the real SvelteKit app") and
   frontend/.gitkeep. Do NOT run any npm command or scaffolder here.
6. Create a root Makefile with these targets, each a thin wrapper over scripts/:
     make bootstrap        # install Rust + node deps (node deps are a no-op until S7)
     make verify-harness   # scripts/verify_corpus.sh
     make run-server       # cargo run --manifest-path server/Cargo.toml
     make env-report       # scripts/env_report.sh
     make fmt clippy test  # cargo fmt --check / cargo clippy -D warnings / cargo test
   Every target must be runnable from the repo root with no `cd` in the recipe except via
   --manifest-path.
7. Update docs/ENVIRONMENT.md and docs/DECISIONS.md with the new paths. Add ADR-009 recording the
   monorepo layout and the reason: Cargo and Lake currently share a directory, and the Lean cache must
   live next to its own .lake and nowhere else.
8. Update .gitignore so lean/.lake/ and lean/proofs/ are ignored and server/target/ is ignored.

DO NOT
- Do not run `lake build`, `lake clean`, `lake update`, or `lake exe cache get`.
- Do not delete lean/.lake or lean/Mathlib under any circumstances.
- Do not add, remove, or upgrade any Rust dependency other than removing tokio-tungstenite.
- Do not scaffold the SvelteKit app. Do not run any npm command.
- Do not change any .rs file CONTENTS. This prompt is pure moves. (S2 changes code.)

ACCEPTANCE
  cd /home/gulshansharma/proof_battle
  make env-report
  make verify-harness            # same verdicts as step 1; paste output
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  ls server/src lean/lakefile.lean frontend infra scripts docs
  git status --short

IF BLOCKED
If the corpus output changes after the move, olean resolution is path-sensitive. Revert the move
(`git mv` back, `mv` back), verify the corpus is green again, then STOP and report the exact difference.
A broken Mathlib cache costs 20-60 minutes to rebuild; never "fix" it by building.

DELIVERABLE
The move table (from -> to), the before/after corpus outputs, the Makefile, ADR-009, and:
  git add -A && git commit -m "stage S1: monorepo layout, path indirection, make targets" && git tag stage-s1
```

## S1-P2 — The Config module (every path and knob from the environment)

```
CONTEXT
Read docs/DECISIONS.md first. Design doc §6.4 specifies a Config struct from env vars; this prompt
makes it real and complete. From this stage on, NO module may read an env var directly and NO module
may hardcode a filesystem path. This is what makes the rest of the plan path-agnostic.

TASK
Create server/src/config.rs and wire it into main.rs. No other behavioural change.

SPECIFICATION — server/src/config.rs
  #[derive(Clone, Debug)]
  pub struct Config {
      pub bind_addr: String,
      pub lean_project_dir: PathBuf,     // the lake project root (lean/)
      pub proof_tmp_dir: PathBuf,        // where submissions are materialised
      pub proof_max_bytes: usize,        // default 8192
      pub proof_max_lines: usize,        // default 120
      pub lean_timeout: Duration,        // default 30s
      pub lean_max_heartbeats: u32,      // default 200_000
      pub lean_max_memory_mb: u32,       // default 2048
      pub lean_pool_size: usize,         // default 4
      pub job_queue_capacity: usize,     // default 256
      pub submit_lane_reserve: usize,    // default 2   (ADR-008)
      pub check_lane_capacity: usize,    // default 8
      pub session_ttl: Duration,         // default 60s
      pub rate_limit_submissions: u32,   // default 5 per window
      pub rate_limit_window: Duration,   // default 60s
      pub database_url: Option<String>,  // None => in-memory problem source
      pub sandbox_enabled: bool,         // default false in dev, MUST be true in prod
      pub sandbox_image: String,         // default "proofbattle/lean-sandbox:lean-4.21.0-rc3"
      pub protocol_version: u32,         // default 1
      pub log_filter: String,            // default "proof_battle_server=info"
  }
  Requirements:
  - Config::from_env() -> Result<Self, ConfigError>.
  - An unknown or malformed value MUST produce a named error listing the variable, the value, and the
    expected type. Never silently fall back for a *malformed* value: absent => default,
    present-and-garbage => Err. (The design doc's `.ok().and_then(parse)` silently swallows malformed
    ints — fix that.)
  - When sandbox_enabled is false, log at startup, loudly:
      tracing::warn!("SANDBOX DISABLED — LOCAL DEVELOPMENT ONLY")
  - Config is passed explicitly as `Arc<Config>` into anything that needs it. No lazy_static, no
    once_cell, no globals.
  - `#[cfg(test)] mod tests` covering: all defaults with an empty env, one override, one malformed
    value per numeric type producing ConfigError naming the variable, and a bool override. Tests must
    not depend on the ambient environment: design `Config::from_map(HashMap<String,String>)` and have
    `from_env` delegate to it.

WIRING
  - main.rs: load Config once, pass Arc<Config> down.
  - lean_runner.rs: replace the hardcoded `current_dir(".")` assumption with
    config.lean_project_dir.clone(), and add the comment
      // TODO(S2): convert to tokio::process::Command
    (do not make it async in this prompt).
  - Replace `format!("proofs/{}", filename)` with `config.proof_tmp_dir.join(filename)`.

DO NOT
- Do not add a config crate (figment, config-rs). std::env only, as the design doc specifies.
- Do not implement the sandbox, the queue, the session store, or the rate limiter. Fields only.
- Do not make the Lean runner async. S2 owns that.
- Do not silence unused-field warnings with a blanket allow(dead_code) unless it is on the struct and
  carries a comment naming which stage consumes each field.

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml config::              # all pass
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  BIND_ADDR=0.0.0.0:9999 LEAN_TIMEOUT=notanumber cargo run --manifest-path server/Cargo.toml
    # must fail fast with: invalid value for LEAN_TIMEOUT: "notanumber" (expected integer seconds)
    # and must NOT have bound the port
  grep -rn "std::env::var" server/src/ | grep -v "config.rs"          # must print nothing

DELIVERABLE
server/src/config.rs, the main.rs and lean_runner.rs wiring diff, test output, standard report.
```

---

# Stage S2 — Backend Core Correctness

**Goal**: fix the ten defects in design doc §1 that are *correctness and concurrency* defects. This
stage makes the backend correct and observable. It does not make it safe — that is S3.

**Exit gate**
- An integration test drives two real WebSocket clients through connect → match → challenge → submit →
  verdict → game end, with no panics and no unwrapped errors.
- `tokio::process` with timeout, kill-on-drop, and process-group kill are proven by a test that submits
  a non-terminating Lean program and observes a verdict in < timeout + 2 s, with no orphan `lean`
  process afterwards.
- No `Mutex` is held across an `.await` other than a channel send. A source scan proves it.
- Rooms leave the registry on game end and on disconnect; a test asserts the length returns to 0.

---

## S2-P1 — Async Lean runner with timeout, kill-on-drop, process-group kill

```
CONTEXT
Current server/src/lean_runner.rs calls `std::process::Command::output()` — synchronous, blocking a
Tokio worker thread — then `std::thread::sleep(Duration::from_secs(2))` on every submission. There is
no timeout, so a looping proof hangs the handler forever. Design doc §1.1 and §1.2 specify the fix.

Measured baseline: a well-formed proof takes ~3.7 s wall in a cold process. The timeout must therefore
be >= 5 s to be meaningful; the 30 s default is correct.

TASK
Rewrite the Lean runner as an async, bounded, observable component. This prompt does NOT add
sandboxing (S3-P2) or priority lanes (S6-P2); it builds the single clean async seam that those wrap.

SPECIFICATION — new module server/src/lean/ (runner.rs, wrap.rs, mod.rs)
  pub enum Verdict {
      Accepted { stdout: String },
      Rejected { reason: RejectReason, stderr: String },
      Error(String),                 // harness/spawn failure — NEVER treated as Accepted
  }
  pub enum RejectReason {
      LeaningFailed,                 // exit != 0
      TimedOut,
      UsesSorry,                     // the soundness fix — detection lands here
      TooLarge,
      RejectedByFilter(String),      // S3-P1's filter
      Busy,                         // S6-P2's queue
      Internal(String),
  }
  pub struct VerifyOutcome {
      pub verdict: Verdict,
      pub elapsed_ms: u64,
      pub stdout_bytes: usize,
      pub stderr_bytes: usize,
      pub truncated: bool,
  }
  pub async fn verify_once(config: &Config, source: &str) -> VerifyOutcome

  Requirements:
   1. tokio::process::Command with `.kill_on_drop(true)` set BEFORE `.spawn()`,
      `.stdin(Stdio::null())`, stdout/stderr piped.
   2. `.current_dir(&config.lean_project_dir)`, args `["env", "lean", <path>]`. No shell, no `sh -c`.
   3. Temp file lifecycle: path = config.proof_tmp_dir.join(format!("{}.lean", Uuid::new_v4())) —
      server-generated only, never incorporating any client-supplied string. Write it with
      tokio::fs::write and GUARANTEE deletion on every path (success, failure, timeout, unwind). Use a
      scope guard. Leaking proof files is a disk-exhaustion bug; the current code leaks them forever.
   4. Timeout: `tokio::time::timeout(config.lean_timeout, child.wait_with_output())`. On timeout ->
      Verdict::TimedOut AND an explicit `child.kill().await` — do not rely on drop alone, because
      wait_with_output consumes the child handle. kill_on_drop is the safety net, not the mechanism.
   5. Process-group kill: on Unix, spawn the child in its own process group so the lake -> lean
      grandchild dies with it. tokio::process::Command exposes `.process_group(0)`. On timeout, send
      SIGKILL to the negative pid. If you cannot do this without adding a dependency, STOP and report
      — do not ship grandchild leaks.
   6. Output caps: read at most LEAN_STDERR_MAX_BYTES (64 KiB) per stream and set truncated=true.
      Lean can emit megabytes of `trace` output; uncapped stderr is a memory amplification vector.
   7. Soundness detection NOW (cheap, and it makes the corpus testable end to end): the generated
      preamble MUST include `set_option warningAsError true` and
      `set_option maxHeartbeats <config.lean_max_heartbeats>`, AND you must additionally grep stderr
      for "declaration uses 'sorry'" / "sorryAx" and reject on a match. FAIL CLOSED: every error path
      yields Rejected.
   8. One tracing line per verification:
      tracing::info!(elapsed_ms, verdict = ?verdisc, stdout_bytes, stderr_bytes, truncated, "lean verify")
      No per-line logging.
   9. `#[cfg(test)]` tests: (a) known-good proof -> Accepted; (b) `sorry` -> Rejected(UsesSorry);
      (c) a Lean program that loops forever -> Rejected(TimedOut) with elapsed_ms < timeout + 2000;
      (d) syntax error -> Rejected(LeaningFailed); (e) the temp dir contains no .lean files afterwards
      (count before/after). Mark the slow ones #[ignore] and document `cargo test -- --ignored`.

  MOVE THE WRAPPER HERE — server/src/lean/wrap.rs
    pub fn wrap_problem(imports: &[String], goal: &str, tactic_body: &str) -> String
  producing exactly:
    <imports, one per line>
    set_option warningAsError true
    set_option maxHeartbeats <N>

    theorem goal : <goal> := by
    <tactic_body, verbatim>
  Preserve today's behaviour: unindented tactic bodies are legal Lean 4 (measured, exit 0). Validate
  tactic_body against config.proof_max_bytes and config.proof_max_lines here, returning
  RejectReason::TooLarge when exceeded. `wrap_problem` MUST also return the preamble line count so the
  caller can correct diagnostic line offsets (S4 needs it) — e.g. return `(String, usize)`.

DO NOT
- Do not implement the input filter (S3-P1). Size limits only.
- Do not implement the pool or job queue (S6-P2). One call = one process.
- Do not implement Docker/sandbox (S3-P2).
- Do not change the message types or the WS handler in this prompt (S2-P2, S2-P3 do that).
- Do not add a dependency for SIGKILL. If `nix` is not already available, STOP and report options.

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml lean::                    # (a)-(e)
  cargo test --manifest-path server/Cargo.toml lean:: -- --ignored --nocapture
  pgrep -af "lake env lean" | wc -l     # must be 0 after the timeout test (no orphans)
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  # prove it is non-blocking: a benchmark firing 8 concurrent verify_once() calls must finish in
  # < 2x the single-call time. Paste both timings.

DELIVERABLE
server/src/lean/{mod.rs,runner.rs,wrap.rs}, the deleted sleep, the tests, both timings, standard report.
```

## S2-P2 — Server-assigned identity, error handling, tracing

```
CONTEXT
Design doc §1.5: the client currently sends `SubmitProof { code, player_id }`, so a malicious client
can claim another player's id and steal their win. Design §6.2/§6.3: the handler is unwrap()-soup
using println!. This prompt fixes identity and error propagation. The message SHAPE is formalised at
S4; today only remove the client-supplied id and keep the rest wire-shaped.

TASK
1. IDENTITY
   - SubmitProof no longer carries player_id. The handler already knows it: it is generated at
     WebSocket upgrade and bound to the connection. Thread it explicitly.
   - Introduce PlayerSession { player_id: Uuid, tx: mpsc::Sender<ServerMessage>, joined_at,
     username: Option<String>, submission_count: u32 } in the session registry (S2-P3 creates it).
   - Add a comment in message.rs stating the invariant and referencing ADR-001, so a later reader (or
     model) does not "helpfully" re-add the field.
2. ERROR HANDLING
   - Create server/src/error.rs with AppError { Io, Json, Lean(String), Protocol(String),
     Internal(String) } implementing Display + Error, plus `impl IntoResponse for AppError` mapping
     to a status code and a JSON body { error: { code, message } } with codes bad_request,
     payload_too_large, internal.
   - Replace every unwrap()/expect() in the request path with `?` + tracing.
   - Replace every println! with tracing at the right level. One startup tracing::info! banner is fine.
   - Replace `tx.send(...).unwrap()` with a match that logs
     warn!(player_id, "client disconnected before delivery") and returns from the connection task.
     A dead peer must never panic the server.
3. MESSAGE PLUMBING
   - Change the outbound channel from mpsc::UnboundedSender<String> (unbounded = memory growth under a
     slow client) to a bounded mpsc::Sender<String> (capacity 64). Document that a full queue means the
     peer is too slow: on SendError, log and close that connection.
   - Serialise with serde_json only. Never hand-build JSON strings.
4. MALFORMED INPUT
   - Non-UTF8 frames, non-Text frames, and unparseable JSON must produce ServerMessage::Error and then
     close the connection with a proper close code (1003 unsupported data / 1007 invalid payload).
     Do not silently ignore frames.

DO NOT
- Do not add or rename ServerMessage variants beyond Error (S4 owns the protocol).
- Do not implement reconnection, session tokens, or room timeouts (S6).
- Do not add anyhow. Hand-rolled error enums only, matching the design doc's style.
- Do not add dependencies.

ACCEPTANCE
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  grep -rn "unwrap()\|expect(\|println!" server/src/ | grep -v tests   # empty or justified inline
  # tests/protocol.rs (#[tokio::test]):
  #  - {"type":"SubmitProof","code":"intro n"} is accepted and answered
  #  - {"type":"SubmitProof","code":"x","player_id":"<other uuid>"} is REJECTED as bad_request and the
  #    other player's session is unaffected
  #  - garbage bytes close with 1007 and no panic
  cargo test --manifest-path server/Cargo.toml --test protocol

DELIVERABLE
server/src/error.rs, message.rs and handler diffs, tests/protocol.rs, the grep evidence, report.
```

## S2-P3 — Room actor model, no locks across await, registry cleanup

```
CONTEXT
Read docs/DECISIONS.md, especially ADR-001. Design doc §1.4 prescribes DashMap; this project uses the
actor model instead. Rationale: the real bug in main.rs is that `challenge_map.lock().await` is acquired
at the top of the SubmitProof arm and held for the ENTIRE proof run — a global mutex held across
seconds of I/O. DashMap would reduce contention but the scoping bug survives. Per-room actors remove
the class of bug entirely.

MEASURE THE BUG FIRST (before changing anything, paste the output)
  Add a temporary test that opens 2 connections in 2 different rooms, submits from both
  simultaneously, and logs each submission's start/end timestamps. You should observe the two
  verification windows SERIALISED rather than overlapping. Keep the measurement in your report, then
  delete the temporary test.

TASK — target architecture
  server/src/game/
    mod.rs         PlayerId and RoomId newtypes; module wiring
    matchmaker.rs  Matchmaker actor: owns the waiting queue, the room registry, the player registry
    room.rs        Room actor: owns ALL state for one match
    session.rs     PlayerSession + SessionStore trait (in-memory impl only; S6 adds the rest)

  Matchmaker actor
   - Owns players: HashMap<PlayerId, mpsc::Sender<Command>>, waiting: VecDeque<PlayerId>,
     rooms: HashMap<RoomId, mpsc::Sender<Command>>.
   - Commands: Connect{player_id, tx, reply}, QueueJoin{player_id}, Disconnect{player_id},
     SubmitProof{player_id, code, req_id}, Resign{player_id}, Tick.
   - Runs as ONE tokio::spawn task in a tokio::select! over its command receiver and a 1 Hz tick.
     No Mutex anywhere in this module.
   - On two queued players: mint a RoomId, spawn a Room actor, register the senders, and reply to both.
  Room actor
   - Owns both PlayerIds, the Problem, the deadline, per-player submission counts, the verdict ledger,
     and the `solved` flag. Single-threaded, so transitions need no locking and "first accepted verdict
     wins" is naturally FIFO.
   - Owns the round deadline: `deadline: Instant` set at spawn, checked on its own tick, emitting
     RoundEnd { outcome: Draw } on expiry. Server-authoritative clock (ADR-004).
   - On an Accepted verdict: broadcast RoundEnd to both players, then send Shutdown to the Matchmaker,
     which removes the room from the registry.
   - Lifecycle by RAII: when the Room actor returns, its Drop impl logs and sends Shutdown. It must be
     impossible to leak a room by forgetting a remove() call — prove it in a test.
  Proof execution
   - The Room actor must NEVER await a multi-second Lean call inline, or both players' messages stall.
     tokio::spawn a task that calls lean::verify_once and returns the verdict over a channel. The actor
     stays responsive to Ping / Resign / timeout while Lean runs. This is the entire point of the model.
   - A player may have at most ONE in-flight verification; a second is refused with Error { code:
     "busy" }. This also bounds per-player resource usage.

DO NOT
- Do not add a Mutex, RwLock, or DashMap anywhere in server/src/game/. If you think you need one, that
  is a design signal — restructure and explain it in the report.
- Do not implement priority job lanes (S6-P2). One tokio::spawn per verification, bounded by the
  per-player in-flight guard and a tokio::sync::Semaphore with config.lean_pool_size permits around
  verify_once. A Semaphore IS allowed and expected here; a Mutex is not.
- Do not implement session tokens, ELO, or the problem database (S6 / S5).
- Do not add dependencies.

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml game::
  # Required tests, all #[tokio::test]:
  #  1. two clients -> matched -> both receive a Challenge with an identical goal
  #  2. concurrent submits from two rooms -> the two verify windows OVERLAP (paste timestamps
  #     proving this, contrasted with the pre-measurement from step 1)
  #  3. one player disconnects mid-game -> the other receives RoundEnd and the registry length returns
  #     to 0 within 2 s
  #  4. both players submit a correct proof "simultaneously" -> exactly ONE RoundEnd per player (no
  #     duplicate win) and both players agree on the winner
  #  5. deadline expiry with no submission -> RoundEnd { outcome: Draw } for both
  #  6. registry length is 0 after 50 sequential connect/match/end cycles (no leak)
  grep -rn "Mutex\|RwLock\|DashMap" server/src/game/    # must print nothing
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings

IF BLOCKED
If test 4 cannot be made deterministic, do NOT weaken the assertion. Add a test-only injection point
(a `trait Verifier` with the real impl as default and a scripted impl for tests — ~20 lines, behind
#[cfg(test)]) and script the interleaving. Deterministic concurrency tests are worth the 20 lines.

DELIVERABLE
server/src/game/{mod,matchmaker,room,session}.rs, the diff against the old room.rs, all six test
outputs, the overlap evidence, the registry-leak evidence, standard report.
```

---

# Stage S3 — Verification Soundness & Sandbox Boundary   ◀ CRITICAL PATH

**Goal**: make the judge sound and the machine survivable. Heaviest stage, and the one where a
"reasonable-looking" implementation is still insecure.

**Why now**: everything else is worthless if `sorry` wins, or if `#eval` reaches the host.

**Empirical facts this stage is built on** (all measured on this machine):
1. `theorem goal : ∀ n : ℕ, n + 0 = n := by` with body `sorry` → **exit 0**, stderr
   `warning: declaration uses 'sorry'`. Under the design doc's rule the player WINS.
2. Adding `set_option warningAsError true` to the preamble turns it into
   `error: declaration uses 'sorry'`, exit 1. **This is the fix.**
3. A player cannot downgrade that option from inside a tactic block (`set_option warningAsError false`
   there yields `error: unexpected token 'sorry'; expected 'in'` — Lean forces `set_option … in term`).
4. `#eval IO.getEnv` compiles and Lean attempts evaluation → arbitrary code executes in the judge.
5. `native_decide` executes compiled native code; `unsafe` + `@[extern "…"]` is a direct escape.

**Exit gate**
- The adversarial corpus (Appendix B) is 100% green: every RCE / DoS / cheat payload is rejected, every
  legitimate proof accepted.
- The sandbox runs with `--network none`, read-only rootfs, `--pids-limit`, memory cap, and
  `cap_drop ALL`, and a legitimate proof verifies identically inside and outside it.
- A payload attempting to write a host file cannot create that file on the host.
- The filter has tests proving it does NOT fire on a commented-out `#eval` or the string literal
  `"#eval"`, and DOES fire on a real one.

---

## S3-P1 — Submission format + lexical filter (ADR-002)

```
CONTEXT
Design doc §6.5 proposes a denylist:
    const FORBIDDEN_PATTERNS: &[&str] = &["#eval", "#check IO", "unsafe", "System.Process", ...];
    if code.contains(pattern) { return Err(...) }
That is unsound in BOTH directions, and the next stage must not weaken it further:
  FALSE POSITIVES: a player writing `-- #eval is banned`, or `exact "#eval"`, is blocked for commenting
    on the rules.
  FALSE NEGATIVES: `#eval` inside a comment or block comment, and every construct not on the list
    (macro, elab, initialize, @[extern], opaque, partial, attribute [instance], deriving) walk through.
The deeper fix is ADR-002: the client submits ONLY the tactic body. Imports, the theorem statement, and
all set_option lines are server-owned, so the body cannot declare, import, or configure anything.

TASK
1. Change the wire message (shape only; S4 formalises it) so SubmitProof { code } carries the tactic
   body ONLY. Update wrap_problem to take the body, and update the room actors.
2. Create server/src/lean/filter.rs:
     pub fn filter_tactic_body(body: &str, limits: &Limits) -> Result<String, FilterError>
   Pipeline, in this EXACT order (a later stage must not reorder it):
     a. NORMALISE: reject if body.len() (bytes) > limits.max_bytes (8192) or
        body.lines().count() > limits.max_lines (120). Reject any NUL byte or C0 control char other
        than \n and \t.
     b. STRIP COMMENTS + STRINGS with a hand-written scanner handling Lean's NESTED block comments
        (`--` to EOL, `/- … -/` at arbitrary nesting depth), `"…"` and `«…»` strings, and `'c'` chars.
        Replace each stripped region with SPACES of equal length so diagnostic columns stay valid.
        Unit tests: `/- a /- nested -/ b -/`, an unterminated block comment (reject), `-- #eval`,
        `"#eval"`, `"a \" b"`, and `'/'`.
     c. TOKEN SCAN the stripped text with a real scanner (identifiers may contain `'`, `.`, `_`,
        digits, and unicode letters). Apply an ALLOW-LIST on the first token of every non-empty
        logical line. Allowed tactic heads, exactly:
        intro intros intro1 intro2 rename rename_i obtain rcases rintro match_cases match use
        exact exact_mod_cast refine apply constructor constructors cases case_tac on_left on_right
        rfl assumption assumption_mod_cast trivial trivial_mod_cast
        simp simpa simp_all simp_arith simp_rw simp only norm_num norm_num1 push_cast
        decide ring ring_nf ring_exp field_simp linarith nlinarith abel positivity
        omega grind aesop
        haveI have show suffices subst substs induction induction' inductionOn cases cases'
        rw rw' left right guard_hyp guard_target unfold delta change convert
        unfold_using simp_where_generalize done trivial_1
        Every other first token is REJECTED with
        FilterError::ForbiddenConstruct { token, line, col }. Explicitly rejected heads include:
        import open namespace section end variable attribute instance axiom opaque def abbrev
        theorem lemma example structure inductive class macro macro_rules elab syntax notation
        scoped initialize unsafe partial extern deriving universe set_option run_cmd
        native_decide
        and any line whose first token starts with `#` (#eval, #check, #print, #exit, #load, #help,
        #guard_msgs). `decide` is ALLOWED (bounded by maxHeartbeats + wall timeout). `native_decide` is
        REJECTED unconditionally and explicitly, because it executes compiled native code.
     d. Reject any occurrence of `sorry`, `sorryAx`, `admit`, or `stop` as a token, and any `@[`.
     e. On success return the ORIGINAL body unchanged (stripping was analysis only).
3. Every rejection produces RejectReason::RejectedByFilter naming the token, line, and column — and
   must NOT leak absolute filesystem paths.

TESTS REQUIRED (unit, in filter.rs)
  accepts: "intro n\nsimp", "intro n; simp", "  exact Nat.add_zero n", "simp only [Nat.add_zero]",
           "rcases h with ⟨a, b⟩", "obtain ⟨x, hx⟩ := h"
  rejects: "import Mathlib", "set_option maxHeartbeats 1 in simp", "exact unsafe 1", "native_decide",
           "axiom h : False", "opaque foo : True := trivial", "attribute [instance] Foo.bar",
           "run_cmd IO.println 1", "#eval 1", "macro_rules | `(x) => x", "example : True := trivial",
           "theorem goal : True := trivial", "initialize registerBuiltinAttribute 1",
           "elab_rules : term | `(`x`) => x", "exact sorryAx", "exact (by sorry)"
  does NOT reject: "-- #eval is dangerous", "exact \"#eval\"", "/- #eval -/ intro n", "'/'"
  200-line body -> TooLarge; 10000-byte body -> TooLarge
  a syntax error at line 3 of the GENERATED file maps back to line 2 of the body (this is the
  offset-correctness test; the client half is S8-P2)

DO NOT
- Do not "simplify" the allow-list into a denylist. If a tactic must be added, add it and list it in
  the report; never remove one.
- Do not use a regex engine as the primary mechanism — nested comments and column preservation need a
  hand-written scanner.
- Do not implement the container sandbox (S3-P2). This prompt is pure input handling.
- Do not add dependencies (no regex crate, no tree-sitter).

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml filter::      # paste the full list
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  # show the trace line for a rejected "#eval 1" body: it must contain reason=RejectedByFilter and must
  # NOT contain "/home/gulshansharma".

DELIVERABLE
server/src/lean/filter.rs, wrap.rs and message.rs changes, the full test list, standard report.
```

## S3-P2 — Container sandbox as the real trust boundary

```
CONTEXT
S3-P1 is defence in depth. It is a filter, and filters lose. Design doc §1.3 says the fix is Docker +
seccomp, and measurement confirms why: `#eval IO.getEnv` compiles in this Lean environment. A filter
that misses one identifier is remote code execution on the host.

TASK
Build a LeanExecutor trait with two implementations: LocalExecutor (dev only; REFUSES to start when
config.sandbox_enabled is true) and SandboxExecutor (production). S2-P3's verify calls go through the
trait, so no other module changes.

FILES
  infra/sandbox/Dockerfile           the judge image
  infra/sandbox/seccomp.json         allow-list profile (defaultAction SCMP_ACT_ERRNO)
  infra/sandbox/docker-compose.yml   the executor service for local testing
  infra/sandbox/entrypoint.sh        warms the cache, then idles
  server/src/lean/sandbox.rs         SandboxExecutor
  server/src/lean/executor.rs        trait + selection logic

DOCKERFILE requirements
  - Base image pinned to the exact tag leanprover/lean4:v4.21.0-rc3 (or
    ghcr.io/leanprover/lean4:v4.21.0-rc3 if the Docker Hub image is unavailable — VERIFY with
    `docker manifest inspect` and record which exists).
  - Copy the prebuilt Mathlib olean cache in as a layer (see the warning below).
  - Non-root user `proofbattle` (uid 10001). WORKDIR /judge.
  - v1 is ONE-SHOT, not a long-lived server: `docker run --rm <image> <timeout> <file>`.
    Report measured cold AND warm `lake env lean` latency in both cases. If warm is not meaningfully
    faster than cold on this machine, say so plainly — design doc §3's "15s -> 2s" is a hypothesis that
    must be measured, not assumed (see ADR-012 at S9-P1).
  - Actually BUILD it: `docker build -t proofbattle/lean-sandbox:lean-4.21.0-rc3 .` and paste the tail.

RUNTIME INVOCATION (this is the security-critical part — get every flag right)
  docker run --rm
    --network none                            # no egress, no DNS, no metadata endpoint
    --read-only                              # immutable root filesystem
    --tmpfs /tmp:rw,noexec,nosuid,size=64m    # the only writable path, no exec
    --cap-drop ALL                           # every Linux capability dropped
    --security-opt no-new-privileges         # no setuid escalation
    --pids-limit 256                         # fork bombs
    --memory <lean_max_memory_mb>m --memory-swap <same>m   # no swap escape
    --cpus 2
    --user 10001:10001
    --ulimit nofile=256:256 --ulimit nproc=64:64 --ulimit fsize=16m:16m
    --security-opt seccomp=infra/sandbox/seccomp.json
    --label proofbattle.job=<uuid>
    -v <proof_tmp_dir>/<job>.lean:/judge/proof.lean:ro
    proofbattle/lean-sandbox:lean-4.21.0-rc3 <timeout_secs>
  Add `fn invocation_argv_is_hardened()` asserting EVERY flag above is present in the actual argv —
  this test is the regression guard for the whole stage.
  seccomp.json: allow-list model, defaultAction SCMP_ACT_ERRNO. Allow only what Lean needs:
    read write close fstat newfstatat lseek mmap mprotect munmap brk rt_sigaction rt_sigreturn
    sigaltstack futex clock_gettime getrandom arch_prctl set_tid_address set_robust_list rseq
    readlink stat exit_group exit getpid gettid pipe2 clone clone3 uname fcntl dupl
  Deny by omission: socket, connect, ptrace, mount, chmod, chown, setuid, kill, execve-after-init.
  Validate the profile AND prove a legitimate proof still passes inside the sandbox with it applied.

WARNING — the olean cache
  This repo's Mathlib oleans live in lean/.lake/build/lib/lean/ and the sources in lean/Mathlib
  (86 MB, 6549 .lean, 0 .olean), and `import Mathlib` does NOT resolve (measured). Your image must
  either (a) COPY lean/ in as a build layer, or (b) mount lean/ read-only at runtime with the correct
  LEAN_PATH / LEAN_SRC_PATH. Choose (a) for a self-contained image and MEASURE the image build time.
  If it exceeds 30 minutes, switch to (b), document the trade-off, and make the mount path
  configurable via PB_SANDBOX_LEAN_MOUNT.
  Do NOT run `lake build Mathlib`. Do NOT regenerate oleans.

FAIL-CLOSED REQUIREMENTS
  - If the container cannot start (image missing, docker socket denied, flag rejected by the daemon),
    the verdict is Verdict::Error — NEVER Accepted. Log at error! with the docker stderr.
  - Invoke the docker CLI through tokio::process::Command with the same timeout + kill-on-drop +
    process-group treatment as S2-P1. REUSE that code; do not fork a second implementation.
  - State in the report that a process with docker-socket access is root-equivalent, so the server
    itself must run as a non-root user inside a container with a read-only rootfs.

DO NOT
- Do not add --privileged, --network host, --cap-add, or a writable rootfs.
- Do not use --pid=host, no -v /:/ , and no host path outside proof_tmp_dir and the Lean dir.
- Do not implement a persistent worker pool or LSP (design §3). Measure first, at S9-P1.
- Do not add a Rust docker crate (bollard). The docker CLI is the interface: smaller dependency list,
  more obvious audit surface.

ACCEPTANCE
  docker build -t proofbattle/lean-sandbox:lean-4.21.0-rc3 infra/sandbox    # paste tail
  cargo test --manifest-path server/Cargo.toml sandbox::    # includes invocation_argv_is_hardened
  # The escape tests, run by hand, all pasted:
  #  1. body: exact (IO.println "hi")            -> does not execute
  #  2. a payload attempting a host file write; afterwards:  test -e /tmp/pwned && echo "HOST
  #     COMPROMISED" || echo "host clean"
  #  3. a legitimate proof STILL passes inside the sandbox (paste the ACCEPT)
  #  4. `docker stats` during a verification: paste peak memory, confirm it is under the cap
  #  5. `docker run --rm --network none ... sh -c 'wget -q -O- http://1.1.1.1'` must fail

DELIVERABLE
All infra/sandbox files, server/src/lean/{sandbox,executor}.rs, the seccomp validation result, the
cold-vs-warm measurement table, the escape-test transcript, report.
```

## S3-P3 — The adversarial corpus (end-to-end proof that the judge is sound)

```
CONTEXT
This is the acceptance test for the project's core promise. Appendix B lists 20 payloads with expected
verdicts. Implement them as executable fixtures and make the suite green.

TASK
1. Create tests/corpus/ with one .json per case:
     { "id": "rce_eval_io", "category": "rce", "goal": "∀ n : ℕ, n + 0 = n",
       "imports": ["import Mathlib.Data.Nat.Basic"], "body": "…", "expect": "reject",
       "reject_reason": "RejectedByFilter" }
2. Create tests/corpus.rs (#[tokio::test], one test per case; #[ignore] only on the cases that must
   reach the container) and scripts/verify_judge.sh, which runs the whole suite in BOTH sandbox modes
   and prints: id  category  expect  actual  reason  PASS|FAIL
3. The suite MUST include every Appendix B case. Add any further case you think of; for each new case
   state in the report what attack it models.
4. Cases MUST NOT be removed because they "should not be possible". A failing case is a finding: fix
   the code or report it as a known hole. A corpus with deleted cases is worse than no corpus.
5. Add tests/regression.rs with four end-to-end matches through the real WebSocket path:
   1. a player submits `sorry` and must receive ProofRejected with a reason mentioning sorry, and the
      game must CONTINUE (not end)
   2. a player submits a real proof, receives ProofAccepted, and both players receive RoundEnd
   3. `#eval IO.println "x"` receives ProofRejected and the string "x" appears in NO log line the
      server emits (grep your own tracing output)
   4. the whole corpus runs with sandbox_enabled=false and again with true, and the verdicts are
      IDENTICAL — sandbox parity: the filter and the sandbox must agree

DO NOT
- Do not weaken an expected verdict to make a test pass.
- Do not add #[ignore] to a case that is not slow; it is only for sandbox-dependent cases, and
  scripts/verify_judge.sh must run them explicitly.
- Do not assert only "reject" — assert the specific reject_reason. A weaker test is a wasted case.

ACCEPTANCE
  bash scripts/verify_judge.sh                                  # 100% PASS in both sandbox modes
  cargo test --manifest-path server/Cargo.toml --test corpus -- --ignored --nocapture
  # proof the ORIGINAL bug is fixed end to end (paste both transcripts):
  #   body "sorry"        -> ProofRejected, reason UsesSorry or RejectedByFilter
  #   body "intro n\nsimp"-> ProofAccepted, GameEnded winner = that player
  cargo test --manifest-path server/Cargo.toml                    # full suite still green

DELIVERABLE
tests/corpus/*, tests/corpus.rs, tests/regression.rs, scripts/verify_judge.sh, the PASS table, and a
report section listing every case with a one-line "what attack this models".
```

---

# Stage S4 — Protocol Contract & Type Generation

**Goal**: one schema, generated on both sides. Kill protocol drift permanently.

**Why now**: S5–S8 all compile against this seam. After S4, changing a message is a one-line Rust
edit plus `make codegen`.

**Exit gate**
- `docs/PROTOCOL.md` exists and matches the code.
- `ts-rs` generates `frontend/src/lib/ws/generated.ts` from the Rust enums; a contract test asserts
  every Rust variant appears in the generated TS and vice versa.
- The old ad-hoc messages are gone, and a compile-fail demonstration proves the no-client-identity
  invariant is enforced.

---

## S4-P1 — The wire protocol v1

```
CONTEXT
The design docs disagree with the code and with each other:
  - backend ServerMessage::Challenge { goal, imports }  vs frontend expects { goal, imports,
    difficulty, hint, category }
  - backend GameEnded { winner }                        vs frontend expects { winner, winning_proof,
    canonical_proof, elo_delta }
  - backend has ClientMessage::Join { username }         vs frontend sends JoinQueue
  - the frontend sends SubmitProof for BOTH live checking and real submissions, with no way for the
    server to distinguish them and no correlation id to match a verdict to a request
  - the frontend's TimerUpdate { seconds_remaining } is client-driftable (ADR-004)
This prompt defines ONE protocol (Appendix C) and implements it in Rust.

TASK
1. Rewrite server/src/ws/message.rs to exactly the Appendix C schema.
   - #[serde(tag = "type", rename_all = "camelCase")] for variant names; fields snake_case as written
     in Appendix C (PascalCase types, snake_case fields — deliberate, and it matches the existing code
     so the diff stays reviewable).
   - Add protocol_version: u32 to Welcome. REJECT any frame whose version is absent or greater than
     config.protocol_version with ServerError { code: "unsupported_version" } then close 1002.
   - One request message for both intents:
       ProofRequest { req_id: String, code: String, intent: ProofIntent }
     with ProofIntent = Submit | Check. One code path, one verdict shape. The server MUST enforce that
     Check never affects game state and never counts against the submission rate limit (it is limited
     separately: 1 per 2 s per player).
   - Verdict is an enum: Accepted | Rejected | Error. Rejected carries reason: RejectReason
     (machine-readable), message (human-readable), diagnostics: Vec<Diagnostic>.
   - Diagnostic { line, col, end_line, end_col, severity, message } where line/col are 1-BASED and
     RELATIVE TO THE PLAYER'S TACTIC BODY, not the generated file. Implement the offset correction now
     using the preamble length returned by wrap_problem (S2-P1). Compute it — never hardcode 4. Test it.
   - OpponentActivity { status: Idle | Typing | Checking | Verifying | Submitted } — server-inferred
     only, never client-asserted. Document in the enum's doc comment that it must never reveal code.
   - RoundEnd { outcome: Won | Lost | Draw | ForfeitWin | ForfeitLoss, … } replaces GameEnded { winner }.
     Both players receive the SAME outcome value; the client NEVER computes the outcome by comparing
     ids. This removes a whole class of client-side bug.
   - RoundStart { problem, starts_at_ms, ends_at_ms, duration_ms, server_time_ms }.
   - ServerTime { server_time_ms } on connect and every 15 s tick so the client can compute a clock
     offset. Do NOT send a decrementing seconds_remaining.
2. Add #[derive(ts_rs)] and #[ts(export, export_to = "frontend/src/lib/ws/generated.ts")] on every
   protocol type; Option<T> maps to `T | null`. Add ts-rs to server/Cargo.toml.
3. Add a Makefile target `codegen` that runs the export and fails if the generated file changed
   (cargo test export_bindings && git diff --quiet frontend/src/lib/ws/generated.ts).
4. Create docs/PROTOCOL.md: a table of every message with fields, direction, and the exact legal
   connection state transitions, plus a mermaid sequence diagram of a full match. Copy the schema
   from Appendix C so the doc and the code cannot disagree.
5. Write tests/protocol_contract.rs: for every variant in the Rust enums assert (a) it serialises to a
   JSON object with a "type" field, (b) it round-trips, (c) an unknown "type" from a client is
   rejected as bad_request. PLUS an invariant test for the constitution's rule 1: no ClientMessage
   variant contains a field named player_id, room_id, winner, or elo. Implement it as
   include_str!("../src/ws/message.rs") + a scan of the ClientMessage block, and as a serde
   round-trip that injects a player_id and asserts the server rejects it.

DO NOT
- Do not add gameplay behaviour in this prompt (S6 does that).
- Do not hand-write the TypeScript types. If a frontend protocol type is missing, fix the Rust side
  and re-run `make codegen` — that is the entire reason this stage exists.
- Do not rename frontend file paths yet (S7 consumes the generated file).

ACCEPTANCE
  make codegen
  head -60 frontend/src/lib/ws/generated.ts
  cargo test --manifest-path server/Cargo.toml --test protocol_contract
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  # demonstrate the invariant test bites: temporarily add a player_id field to one ClientMessage
  # variant, run the test, PASTE THE FAILURE, then revert.

DELIVERABLE
server/src/ws/message.rs, frontend/src/lib/ws/generated.ts, docs/PROTOCOL.md,
tests/protocol_contract.rs, the Makefile change, standard report.
```

---

# Stage S5 — Problem Supply (PostgreSQL + verified content)

**Goal**: replace 3 hardcoded problems with a database of problems that are **machine-verified solvable
and to have a canonical solution** — a challenge whose canonical proof does not compile is a broken
game, and a challenge with no canonical solution breaks the post-game screen.

**Why now**: S6's difficulty-bracket selection and ratings need real, verified data.

**Measured constraints**
- `import Mathlib` does NOT work here (missing aggregate olean). Every problem needs fine-grained
  imports, verified per problem.
- 6550 oleans are prebuilt; `lake build Mathlib` is 20–60 min and must not happen.
- No Postgres server is running. Bring one up with the compose file you write.

**Exit gate**
- `make db-up` works deterministically; migrations apply; `make seed` loads problems that **all pass
  the S3 verification path end to end** (canonical proof ACCEPTs; a deliberately wrong body REJECTs).
- Problem selection by ELO bracket is unit-tested, including empty-bracket fallbacks.
- The extraction pipeline runs offline against the local Mathlib source without `import Mathlib`.

---

## S5-P1 — Postgres schema, migrations, and the ProblemSource trait

```
CONTEXT
Design doc §5 gives a `problems` table. It is missing: the canonical solution (needed for the
post-game "intended solution" screen), verification provenance, and a stable slug. It is also missing
a place for ratings — create that now while you own migrations, but do NOT implement rating logic
(that is S6).

TASK
1. Add sqlx (features: postgres, runtime-tokio-rustls, macros) to server/Cargo.toml. Use
   sqlx::query! compile-time-checked queries. That needs DATABASE_URL at build time, so add
   `make sqlx-prepare` (cargo sqlx prepare) and COMMIT the generated server/.sqlx/ directory so CI
   builds without a database. If `cargo sqlx prepare` is unavailable in this environment, use runtime
   sqlx::query_as + #[derive(FromRow)] instead, say so in the report, and do not mix the two styles.
2. Migrations under server/migrations/:
     0001_problems.sql
       problems(id uuid pk default gen_random_uuid(),
                slug text unique not null,              -- e.g. "nat/add_zero"
                goal text not null,
                imports text[] not null,
                statement text not null,                 -- full `theorem goal : … := by` header
                canonical_proof text not null,           -- the tactic BODY only
                difficulty smallint not null check (difficulty between 1 and 10),
                category text not null,
                tactic_hint text,
                source_theorem text,                     -- e.g. Nat.add_zero
                source_url text,
                verified bool not null default false,
                verified_at timestamptz,
                verification_error text,
                times_played int not null default 0,
                last_played_at timestamptz,
                created_at timestamptz not null default now())
       + partial indexes: (difficulty) where verified, and (category, difficulty) where verified
     0002_ratings.sql
       players(id uuid pk, username text unique, elo int not null default 1200,
               glicko_r numeric, glicko_rd numeric, glicko_vol numeric,
               games_played int, wins int, losses int, draws int, updated_at timestamptz)
       match_history(id, room_id uuid, player1 uuid, player2 uuid, problem_id uuid,
                     winner_id uuid null, outcome text, p1_elo_before, p1_elo_after,
                     p2_elo_before, p2_elo_after, rated bool, duration_ms, created_at)
3. server/src/db/problems.rs:
     #[async_trait] pub trait ProblemSource: Send + Sync {
         async fn pick(&self, bracket: Bracket) -> Result<Problem, DbError>;
         async fn by_id(&self, id: Uuid) -> Result<Problem, DbError>;
         async fn count(&self) -> Result<i64, DbError>;
     }
   with InMemoryProblemSource (reads the current hardcoded challenges.rs, so the game keeps working
   with no DB — required for tests and for S7's frontend dev) and PostgresProblemSource.
   Select ONLY verified = true, and exclude problems either player has played in the last 2 minutes
   (anti-repeat; use last_played_at).
4. infra/docker-compose.yml: postgres:16-alpine, named volume, healthcheck, and pg_isready-based
   depends_on so `make db-up` is deterministic. Port 5432; user/db from .env.example, never hardcoded
   secrets.
5. Makefile: db-up, db-down, db-logs, db-reset, migrate, seed, sqlx-prepare.
6. Config: make database_url REQUIRED in production. If DATABASE_URL is unset and PB_ENV != production,
   log warn!("in-memory problem source: 3 problems, ELO disabled") so nobody ships it by accident.
   Unset in production -> exit(1) with a clear message.

DO NOT
- Do not write the rating logic (S6). Tables only.
- Do not hand-insert problem content into the DB (S5-P2 does it programmatically).
- Do not add an ORM (diesel, sea-orm). sqlx only.
- Do not add `db::sqlx` (compile-time only) without committing .sqlx/.

ACCEPTANCE
  make db-up && make migrate && make db-reset
  psql "$DATABASE_URL" -c "\d problems"
  cargo test --manifest-path server/Cargo.toml db::
  # tests: the in-memory source returns exactly the current 3 problems; the postgres source returns 0 on
  # an empty DB; a problem inserted with verified=false is NEVER selected
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  # and the no-DB path still runs the game:
  ( unset DATABASE_URL; cargo run --manifest-path server/Cargo.toml & )    # boots with the warning

DELIVERABLE
Migrations, server/src/db/*, infra/docker-compose.yml, Makefile targets, .sqlx/, tests, standard report.
```

## S5-P2 — Problem ingestion with mandatory machine verification

```
CONTEXT
The design doc §5 extraction script does `import Mathlib` and walks env.constants. That script WILL NOT
RUN here (measured: `import Mathlib` -> missing aggregate olean), and building that olean costs 20-60
minutes. So the pipeline must be built around per-module import lists, and — the part the design doc
omits — every problem must be VERIFIED by the same execution path the game uses before it is served to
a single player.

TASK
A two-phase pipeline. Phase 1 is an offline one-off script; phase 2 is a server binary.

PHASE 1 — scripts/extract_problems.py (or a Lean file; your call, justify it)
  Input: the local Mathlib source at lean/Mathlib/**/*.lean (6549 files, already on disk — do NOT
  clone, do NOT download anything).
  - parse each file's own `import` lines to obtain its fine-grained import list
  - collect `theorem`/`lemma` declarations with a `:= by` block or `:=` term
  - difficulty heuristics: tactic vocabulary in the body (simp/omega/decide/rfl -> 1-2;
    ring/norm_num/linarith -> 3-4; induction/obtain/cases or >4 tactic lines -> 5-7;
    structure/inductive/measure theory -> 8-10), statement length, hypothesis count
  - category from the file path (Mathlib/Data/Nat -> nat_arithmetic, Mathlib/Logic -> logic,
    Mathlib/Analysis -> analysis, Mathlib/Algebra/Order -> order, Mathlib/Data/Finset -> finset)
  - keep only Prop-valued declarations; skip Type-valued ones (bad gameplay, huge proofs)
  Output: problems_import.jsonl, one object per line:
    {slug, goal, imports, canonical_proof, difficulty, category, tactic_hint, source_theorem, source_url}
  HARD REQUIREMENTS
  - No fabricated content. Every field comes from parsing a real file. If the canonical proof cannot be
    extracted reliably, emit the record with canonical_proof = null and let phase 2 reject it. Do NOT
    invent example problems to pad the count.
  - Deterministic (sorted output, fixed seed if sampling) and idempotent.

PHASE 2 — server/src/db/verify_problems.rs (a bin: cargo run --bin verify_problems)
  - Read the JSONL. For each record build the wrapped file with lean::wrap_problem and run it through
    the SAME lean::verify_once (or the sandbox executor if configured) the game uses.
  - verified = true only if: the canonical proof is ACCEPTED, the statement is non-empty and < 500
    chars, the imports resolve, and difficulty is in 1..=10.
  - On failure: log the first 300 chars of stderr into verification_error and REJECT. Do not retry with
    variations — that is exactly how fake problems get in.
  - Upsert by slug. Report total candidates, verified, rejected, and the top 10 rejection reasons with
    counts. That distribution is this stage's quality signal: if >60% are rejected, the extractor needs
    work and you must say so rather than lowering the bar.
  - Support --limit N and --concurrency N (default 4, honouring lean_pool_size).

THEN
  - `make seed` runs phase 1 + phase 2. Paste the full report.
  - The resulting DB should hold a spread across >= 5 categories and all 4 difficulty bands. If phase 1
    yields fewer problems than hoped, SAY SO and report the actual number. Do not pad.

DIFFICULTY CALIBRATION (design §5) — encode as expected_elo(difficulty) -> i32:
  1-2 -> 0-600 | 3-4 -> 600-1000 | 5-6 -> 1000-1400 | 7-10 -> 1400+
  with Bracket { min_elo, max_elo } and +/-150 widening. Unit test the boundaries (599/600/601,
  999/1000/1001, 1399/1400/1401) and the empty-bracket fallback: widen, then relax constraints, then
  fall back to nearest difficulty — logging which fallback fired.

DO NOT
- Do not use an LLM or heuristic to invent problems or proofs. Extraction only.
- Do not run lake build, lake update, or `import Mathlib`.
- Do not mark anything verified without an actual Lean run.
- Do not turn the >=100 target into an assertion that changes the extractor. Report the real number.

ACCEPTANCE
  make seed
  psql "$DATABASE_URL" -c "select count(*) filter (where verified) as verified, count(*) as total from problems;"
  psql "$DATABASE_URL" -c "select category, difficulty, count(*) from problems where verified
                           group by 1,2 order by 1,2;"
  cargo test --manifest-path server/Cargo.toml db::        # includes bracket boundary tests
  # spot-check one seeded problem end to end through the game path: paste the WS transcript

DELIVERABLE
scripts/extract_problems.py, server/src/db/verify_problems.rs, the bin wiring, the ingestion report with
real counts and the rejection distribution, the category/difficulty histogram, standard report.
```

---

# Stage S6 — Game Engine v2 (sessions, timers, ratings, fairness)

**Goal**: production game semantics — reconnection, authoritative timing, rate limits, bounded fair
queues, ratings, and matchmaking that is not a coin flip.

**Exit gate**: the full lifecycle e2e test passes (reconnect, forfeit, timeout-draw, and a 100-game
soak with zero leaked rooms and no unbounded memory growth).

---

## S6-P1 — Session tokens, reconnection, connection state machine

```
CONTEXT
Design doc §1.6: no reconnection at all — a tab refresh orphans the room and the opponent waits
forever. Design §3: Redis with a 60s TTL. ADR-006: the store is a trait with an in-memory
implementation from day one, so the swap is mechanical.

TASK
1. SessionStore trait (the module came from S2-P3):
     async fn create(&self, player_id, username) -> Result<Session>;
     async fn get(&self, token: &str) -> Result<Option<Session>>;   // honours TTL
     async fn touch(&self, token) -> Result<()>;
     async fn revoke(&self, token) -> Result<()>;
   InMemorySessionStore: DashMap<String, Session> with a tokio::time::Instant expiry checked on read,
   plus a 10 s sweeper task. RedisSessionStore: a stub returning Err(NotConfigured) — DO NOT add a
   redis dependency in this stage. The point is the seam, not Redis.
2. Handshake: the client sends Hello { version, token?, username? } as the FIRST frame. The server:
   - no/invalid token -> new session; Welcome { player_id, session_token, server_time_ms, … }
   - valid token, not in a game -> restore, same player_id, NEW token (rotate on every use)
   - valid token, in a game -> REATTACH: the room actor swaps the player's channel, and the player
     receives RoundStart again as a full state resync: problem, remaining time, both players'
     submission counts, and their OWN last verdict. Nothing about the opponent's code or verdict.
   Token rotation on reconnect makes a stolen token single-use.
3. Server connection state machine mirroring the client's (S7-P3): a ConnState enum plus an explicit
   transition table. Any frame received in a state that does not permit it gets
   ServerError { code: "invalid_state" } and a debug log. This is what prevents submitting a proof
   before having a problem.
4. Disconnect: do NOT immediately end the game. Start a 10 s reconnection window per player. If they
   return, resume (above). If not, send RoundEnd { outcome: ForfeitWin } to the opponent and update
   ratings (ADR-010: unrated if the forfeit happened within the first 30 s). The disconnected player
   keeps their session for 60 s so a later rejoin still shows the post-game screen.
5. Heartbeat: the server pings every 15 s; the client pongs; two missed pongs -> treat as a
   disconnect (step 4).

TESTS
  - reconnect within 10 s mid-game -> full state resync, the game continues, SAME player_id
  - reconnect after the window -> the opponent got ForfeitWin; the replayer gets RoundEnd
  - token rotation: the old token does not work a second time
  - SubmitProof before RoundStart -> invalid_state
  - the resync payload contains no information about the opponent's code or verdict content
    (explicit assertion, not an absence of an error)

DO NOT
- Do not add redis or any dependency.
- Do not persist sessions in Postgres in this stage.
- Do not implement ratings (S6-P3).

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml session::
  cargo test --manifest-path server/Cargo.toml -- --ignored --nocapture   # the reconnect e2e tests
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings

DELIVERABLE
server/src/game/session.rs, the ConnState machine, tests, standard report.
```

## S6-P2 — Bounded fair job queue with two lanes (ADR-008)

```
CONTEXT
Design doc §3: "bounded channel, reject when full". Design §3 + frontend §9: live diagnostics run the
SAME `lake env lean` as real submissions. If they share one FIFO queue, a fast typist's diagnostics
occupy every worker and a real submission times out — the player loses a match because they were typing.
That is a livelock, and it is the most likely "why is this game broken" bug in the whole project.

MEASURED: a cold `lake env lean` is ~3.7 s, so even a queue holding 8 checks is ~30 s of latency for
the submission behind it.

TASK
Implement server/src/game/queue.rs.
  pub struct VerifierPool { submit: Semaphore, check: Semaphore, … }
  - SubmitLane: capacity = config.lean_pool_size, permits acquired with
    tokio::time::timeout(config.lean_check_queue_timeout, permit). A submission that cannot get a permit
    within the timeout is REJECTED with reason Busy and the player is told "server busy, retry in a few
    seconds". A lost submission is far better than a lost match.
  - CheckLane: capacity = config.check_lane_capacity, acquired with try_acquire only. If no check
    permit is free, DROP the check silently and tell the client check_skipped: true. Diagnostics are
    best-effort and must never queue.
  - Both lanes use the same lean::verify_once; the difference is admission policy, not execution.
  - Expose a global in-flight counter as a tracing span field and as a broadcast snapshot that S9's
    /metrics will scrape. Build the plumbing now.
  - Per-player rate limits (design §8: 5 submissions/min) with a token bucket keyed by player_id,
    checked in the Room actor BEFORE enqueueing. On limit: ServerError { code: "rate_limited",
    retry_after_ms }. Checks are limited separately to 1 per 2 s.
  - ProofIntent::Check must be STRUCTURALLY incapable of mutating game state: implement it in a
    separate function run_check_only(...) -> Vec<Diagnostic> that has no reference to Room actor state.
    Add a test asserting a Check never emits RoundEnd (run 50 checks in a live game, assert no RoundEnd).

TESTS
  - 8 concurrent Checks + 1 Submit with pool_size 2 -> the Submit completes within the timeout and all
    8 checks report skipped or complete. Paste timings proving the Submit was NOT queued behind them.
  - 6 submissions in 60 s from one player -> the 6th is rate_limited with retry_after_ms.
  - A check storm (100 rapid checks) increases submit latency by <= 10%. Paste before/after medians.
  - Semaphore exhaustion returns Busy, not a hang (1-permit pool, 2 submits).

DO NOT
- Do not implement a persistent worker pool or warm LSP. Measure first (S9-P1, ADR-012).
- Do not add `governor` or any rate-limit crate. A token bucket is ~40 lines.
- Do not let checks mutate Room state. That is the bug this prompt exists to prevent.

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml queue::
  cargo test --manifest-path server/Cargo.toml -- --ignored --nocapture     # the timing tests
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings
  cargo test --manifest-path server/Cargo.toml soak:: -- --ignored --nocapture
  # paste RSS before/after the soak (VmRSS from /proc/self/status printed by the test)

DELIVERABLE
server/src/game/queue.rs, the rate limiter, the metrics snapshot channel, all tests, the timing tables,
the soak result, standard report.
```

## S6-P3 — Ratings (Elo behind a Glicko-2 seam), match history, bracket matchmaking

```
CONTEXT
Design §5 and §8 say both "ELO" and "Glicko-2". They are different systems. This stage implements Elo
behind an interface Glicko-2 can replace, and states plainly which one shipped and why.

TASK
1. server/src/game/elo.rs:
     pub trait RatingSystem: Send + Sync {
         fn expected(&self, a: f64, b: f64) -> f64;
         fn update(&self, a: &mut Rating, b: &mut Rating, a_score: f64) -> (i32, i32);
     }
     pub struct Elo { k_factor: f32 }
   K must be a documented function of games played (default 40 under 10 games, 32 to 30, 24 above),
   not a constant. Draws score 0.5 for both. Forfeit counts as a win but the match is flagged
   rated = false when the forfeit happened in the first 30 s (ADR-010).
2. Persistence: on RoundEnd, in ONE transaction, upsert both players and insert a match_history row.
   If the DB write fails, log at error! and STILL deliver RoundEnd to the players — never block a game
   result on a DB hiccup. Add a test for that failure path (make the source return Err).
3. Bracket matchmaking: replace "first waiting player gets the next" with ELO-bracket pairing (S5-P2's
   Bracket), a 15 s wait before widening, and a hard cap on widening. When several players are waiting,
   pair the CLOSEST pair, not FIFO. State the fairness property you claim and test it: insert players at
   1000/1050/1400/1420 and assert the pairing is 1000<->1050 and 1400<->1420, NOT 1000<->1400.
4. Reconnect/forfeit -> rating update, per S6-P1.

DO NOT
- Do not implement Glicko-2. Implement Elo, expose the trait, and add a doc comment stating that
  Glicko-2 is a drop-in replacement (it needs RD/sigma, and the players table already has columns).
- Do not expose rating math to the client. The server sends elo_delta; the client never computes it.
- Do not add dependencies.

ACCEPTANCE
  cargo test --manifest-path server/Cargo.toml elo::
  # numeric checks to paste:
  #  1200 vs 1200, win -> 1216 / 1184
  #  800 vs 1600 upset -> winner ~+12, loser ~-20
  #  K-factor tiers at games_played = 5, 10, 30, 31
  #  the pairing fairness test from step 3
  #  the rating persistence transaction test + the DB-failure path test
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings

DELIVERABLE
server/src/game/elo.rs, the match history writes, bracket matchmaking, tests, the pasted Elo numbers,
ADR-010, standard report.
```

---

# Stage S7 — Frontend Foundation

**Goal**: a real app shell that speaks the generated protocol and has the design system, playable
against a mock server before the editor exists.

**Deviations**: ADR-005 (Svelte 5 runes); the app lives at `frontend/` (SvelteKit 2).

**Exit gate**: `npm run check` (svelte-check) passes with zero errors; Vitest green; landing, lobby and
result routes render with fixture data; a mock WebSocket server lets the full client state machine run
end to end in CI with no Rust server.

---

## S7-P1 — SvelteKit 2 + Svelte 5 scaffold, and the Svelte 4 -> 5 migration map

```
CONTEXT
frontend_design.md is written in Svelte 4 idioms: export let, `$:`, on:click, createEventDispatcher.
Those are legacy in 2026. ADR-005: SvelteKit 2 + Svelte 5 runes. Writing new code in Svelte 4 idioms
means writing code that must be migrated before it ships.

TASK
1. Scaffold frontend/ with SvelteKit 2 + Svelte 5 + TypeScript (strict) + Vite, Tailwind v4 (CSS-first
   config), Vitest + @testing-library/svelte, Playwright (S9 uses it, install it now so the lockfile
   does not churn later). Use the current non-interactive scaffolder — VERIFY the command with --help
   first (npx sv create or the documented equivalent for your CLI version) and record what you used.
   Do not guess a scaffolder's flags.
2. Pin the versions you actually installed in the report (npm ls --depth=0). Do not guess versions.
3. Create frontend/docs/MIGRATION.md: the exact mapping from frontend_design.md's Svelte 4 snippets to
   Svelte 5, with one real before/after example per construct:
     export let x           ->  let { x = $bindable() } = $props()
     $: derived            ->  const y = $derived(expr)
     $: side effect        ->  $effect(() => { … })
     on:click={f}          ->  onclick={f}
     on:change             ->  oninput
     createEventDispatcher->  callback props:  let { onsubmit } = $props();  onsubmit(code)
     <slot />              ->  {@render children?.()}
     slots / <svelte:fragment> ->  snippets
   Include the $state vs $derived rule (state must be reassignable; derived must not) and a note that
   `svelte/store` still works but the design doc's store pattern maps onto rune classes here.
4. Add `npm run check` (svelte-check --tsconfig ./tsconfig.json) and make it a hard gate.
5. tsconfig.json with "strict": true AND "noUncheckedIndexedAccess": true, plus the $lib -> src/lib
   alias. noUncheckedIndexedAccess matters: it forces you to handle the Array[i] cases that the design
   doc's snippets get wrong.
6. ESLint + Prettier configured; `npm run lint` clean.

DO NOT
- Do not build any game UI yet (S7-P2/P3/P4, S8).
- Do not add Monaco in this prompt (S8). The 4 MB dependency gets its own stage.
- Do not add a state-management library (no Redux, no Zustand). Svelte 5 runes + one small typed
  client class is the whole architecture — the design doc's §8 reached the same conclusion.
- Do not use Svelte 4 syntax anywhere. If you copy a snippet from frontend_design.md, translate it
  through MIGRATION.md first.

ACCEPTANCE
  cd frontend && npm ci && npm run check && npm run lint && npm run build
  npx svelte-kit sync
  npm ls --depth=0
  # add a temporary component using every legacy idiom in the table, confirm svelte-check FAILS, paste
  # the failure, then delete it. This proves the gate catches drift.

DELIVERABLE
The scaffold, frontend/docs/MIGRATION.md, tsconfig, eslint/prettier, the pasted npm ls, report.
```

## S7-P2 — Design tokens, global styles, shell primitives (design §2 ported)

```
CONTEXT
frontend_design.md §2 gives a complete token set — dark math-terminal palette (#0d0d0f base, #7c6af7
electric indigo accent, #34d399 success, #f87171 error), an 8pt spacing scale, 4/8/12/16 radii, JetBrains
Mono + Inter. Port it faithfully to Tailwind v4's CSS-first `@theme` so the tokens work both as CSS
custom properties and as Tailwind utilities.

TASK
1. src/app.css (imported from the root layout): the full token set from design §2.1 as @theme
   variables in Tailwind v4 syntax, preserving the exact hex values and names from the design doc:
   --bg-base, --bg-surface, --bg-elevated, --bg-overlay, --bg-border, --text-primary, --text-secondary,
   --text-muted, --accent, --accent-dim, --accent-bright, --success, --error, --warning, --info,
   --editor-bg, --editor-gutter, --editor-line-hl, --editor-cursor, --font-mono, --font-ui, plus
   --space-1..16 and --radius-sm/md/lg/xl.
2. Fonts: SELF-HOST. Do not hotlink Google Fonts (CSP, offline, privacy). Put the woff2 files in
   static/fonts/ with @font-face, font-display: swap, and local() first in the stack. If you cannot
   obtain JetBrains Mono / Inter woff2 offline, use a documented fallback stack and say so in the
   report — do not add a CDN link.
3. Base components in src/lib/components/ui/, each a Svelte 5 component with typed $props():
   Button.svelte (variants primary/secondary/ghost/danger; sizes; loading state; disabled semantics;
   visible keyboard focus ring), Badge.svelte, Spinner.svelte, Timer.svelte, Stars.svelte (difficulty),
   Tooltip.svelte, Modal.svelte (focus trap + Escape + scroll lock — implement these, they are the
   reason to have a Modal component).
   Implement exactly the CSS from design §2.3/§2.4 and §10: .btn-primary hover/active transform,
   .panel, and the `pulse` keyframes.
4. Root +layout.svelte: font loading, the token scope, a skip-to-content link, and a global
   prefers-reduced-motion block that disables the pulse and confetti animations (accessibility, and it
   is a five-line media query).
5. Instead of Storybook (not required): one Vitest + Testing Library test per UI component covering the
   states the design doc specifies (default, hover via class assertion, loading, disabled, error).

DO NOT
- Do not import a UI kit (no shadcn, no Melt UI, no Flowbite). The design doc specifies a custom visual
  language; a kit would fight it and bloat the bundle.
- Do not add an icon library. The design doc's ASCII art uses ★ ⏱ 💡 🏆; replace them with 16px inline
  SVGs for visual consistency and note this as a deliberate deviation in the report.
- Do not build the game screens yet.

ACCEPTANCE
  cd frontend && npm run check && npm run lint && npm test
  # contrast audit: sample the rendered app and assert computed background/text pairs meet WCAG AA
  # (4.5:1) for body text. Paste the ratio table. If a token fails, FIX THE TOKEN — do not exempt it.
  npm run build && ls -la .svelte-kit/output/client/_app/immutable/assets | head

DELIVERABLE
src/app.css, fonts, 7 UI components + tests, +layout.svelte, the contrast table, standard report.
```

## S7-P3 — WebSocket client, generated message types, connection state machine

```
CONTEXT
The protocol is generated: frontend/src/lib/ws/generated.ts came from the Rust enums at S4. The client
must IMPORT those types, never re-declare them. frontend_design.md §6 gives the state machine; §7's
hand-written message types are now obsolete — delete that file.

TASK
1. src/lib/ws/client.ts: a ProofBattleClient class (a singleton per tab, per design §6) in Svelte 5
   style — hold state in `$state` runes inside a .svelte.ts module so it is reactive without an event
   emitter. That is the Svelte 5 idiom the design doc predates.
   - Explicit ConnState enum and TRANSITIONS table exactly per design §6, extended with `reconnecting`
     and `reattaching`.
   - transition() THROWS on an illegal transition in development and logs an error in production.
     Illegal transitions must be impossible to hide. Add a Vitest test that walks every legal
     transition and asserts every illegal one throws.
   - Reconnect: exponential backoff WITH JITTER (1s, 2s, 4s, 8s, 16s, cap 30s), max 5 attempts, reading
     the token from localStorage (design §6 does this). Store it under `proofbattle.session_token` and
     CLEAR it on clean logout.
   - Reconnect must go through the same transition() machinery, not a side channel.
   - req_id correlation: submitProof(code) and checkProof(code) each mint a crypto.randomUUID() and
     register a pending promise plus its ProofIntent. Verdicts resolve by req_id. When a Check verdict
     arrives with check_skipped: true, resolve locally as skipped — do NOT leave the promise pending
     forever (a real leak in the naive design).
   - Clock offset: on Welcome and every ServerTime, compute offset = server_time_ms - Date.now() and
     expose it. Timer math uses serverTime + offset. NEVER Date.now() alone for anything
     game-critical (ADR-004).
2. src/lib/stores/game.ts: a GameState rune class covering design §8's fields plus serverTimeOffsetMs,
   endsAtMs, checkSkipped, reconnectCount. Derived values (canSubmit, timeRemainingMs) as $derived.
   reset() returns to the exact initial state.
3. src/lib/ws/parseOutput.ts is NOT needed — the server sends structured diagnostics (ADR-007). Delete
   the design doc's parse-the-stderr approach. Instead write src/lib/utils/diagnostics.ts converting
   Diagnostic[] to Monaco markers with 1-based -> 0-based line conversion, plus a test.
4. Tests: Vitest for the state machine (full transition table), the backoff schedule with a fake timer,
   req_id resolution, the check-skip path, and the clock offset.

DO NOT
- Do not declare a protocol type by hand. If a type is missing, fix the Rust side and re-run
  `make codegen` — that is the whole point of S4.
- Do not use Date.now() for the game timer.
- Do not add a reconnect library (no reconnecting-websocket). It is ~60 lines.
- Do not build UI in this prompt.

ACCEPTANCE
  cd frontend && npm run check && npm run lint && npm test
  grep -rn "generated" src/lib/ws/client.ts                       # the types ARE consumed
  grep -rn "type: 'SubmitProof'\|interface ServerMessage" src/     # must print nothing
  # prove the state machine is exhaustive: temporarily delete one TRANSITIONS entry, run the
  # exhaustive-transition test, paste the failure, restore it

DELIVERABLE
src/lib/ws/{client.ts,state-machine.ts}, src/lib/stores/game.ts, src/lib/utils/diagnostics.ts, tests, the
transition-table test output, standard report.
```

## S7-P4 — Routes, mock server, shell screens

```
CONTEXT
Design §3 specifies four screens: landing, lobby, game, result. S8 owns the game screen's editor; S7
builds everything else plus a mock WebSocket server, so the frontend is developable and testable with no
Rust process and no Lean at all.

TASK
1. src/lib/mock/server.ts: a faithful in-browser WebSocket mock implementing the S4 protocol with
   scripted behaviour: connect -> Welcome (fake session token) -> JoinQueue -> MatchFound after ~1.5 s
   -> RoundStart (fixture problem from src/lib/mock/fixtures.ts) -> Check returns diagnostics after
   400 ms -> Submit returns Rejected once, then Accepted -> RoundEnd Won with a fixed elo_delta.
   Include a ?mock=fail mode returning ServerError and a ?mock=disconnect mode that drops the socket at
   a scripted point to exercise reconnection.
   This mock is a first-class testing tool: it must be driven by the SAME message types as the real
   server (imported from generated.ts) and must not contain a hand-written copy of the protocol.
2. src/routes/+layout.svelte nav plus the design §3 routes:
   `/`       landing: ⊢ ProofBattle title, username input persisted to localStorage, Play Now -> /lobby,
             Practice, and a live stats strip (games in progress / players online / top ELO)
   `/lobby`  matchmaking: animated pulse, your ELO and search range, cancel, and a "connected —
             searching" indicator derived from the client state
   `/game`   assembled in S8; in S7 render the three-panel grid with the editor area as a placeholder
             showing the generated file being composed — genuinely useful for debugging the wrapper
   `/result` design §3.4: win/lose/draw, duration, elo delta, the winning proof, the canonical solution
             in syntax-highlighted code blocks, and Rematch / New opponent / Practice
3. ResultModal.svelte per design §3.4 and §10: confetti via canvas-confetti (3 KB), dynamically imported
   ONLY on a win, and disabled under prefers-reduced-motion.
4. Timer.svelte per design §10, but driven by endsAtMs + serverTimeOffset and a 250 ms interval (not
   1 s — a 1 s tick makes the final ten seconds feel broken). Urgent at <= 60 s; critical at <= 10 s
   with the pulse animation.
5. Playwright config with one smoke spec: `/` -> `/lobby` -> (mock) -> `/game` -> `/result`, asserting
   visible texts. This is the first e2e and it will catch layout regressions for the rest of the project.

DO NOT
- Do not add Monaco (S8). The game route's editor area is a <pre> showing the generated file.
- Do not add a router library. SvelteKit routing is the router.
- Do not fetch stats from a REST API — the S4 protocol has no HTTP surface. Use the WS or fixtures.

ACCEPTANCE
  cd frontend && npm run check && npm run lint && npm test
  npm run test:e2e                                    # mock mode, with no Rust server running
  npm run build && npm run preview &                  # then run e2e against the preview build
  # accessibility spot-check with axe (add @axe-core/playwright): the landing and result pages must have
  # zero serious or critical violations. Paste the report.

DELIVERABLE
src/lib/mock/*, the 4 routes, ResultModal, Timer, the Playwright config + smoke spec, the axe report,
standard report.
```

---

# Stage S8 — The Editor: Monaco + Lean UX

**Goal**: the product IS the editor. It must feel like Lean, not like a textarea.

**Exit gate**: Monaco loads with no CDN and no network at runtime; Lean syntax highlighting and the
Unicode mapper work; live diagnostics appear on squiggles with correct line numbers; the editor does
not block initial page load (bundle analysis shows Monaco in a lazy chunk).

---

## S8-P1 — Monaco, loaded offline, lazily, with a real Lean language

```
CONTEXT
Design §4 says "use @monaco-editor/loader" and "4 MB lazy". The @monaco-editor/loader package
downloads Monaco from jsDelivr AT RUNTIME from a CDN. That is unacceptable here: it breaks offline dev,
breaks a strict CSP, adds a third-party runtime dependency to a competitive platform, and leaks visitor
IPs to a CDN. Use the `monaco-editor` npm package with Vite `?worker` imports instead. Deliberate
deviation from design §4.2 — record it as ADR-011.

TASK
1. src/lib/editor/monaco.ts:
     import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
     import tsWorker   from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';
     self.MonacoEnvironment = { getWorker: (_, label) =>
       label === 'typescript' ? new tsWorker() : new editorWorker() };
     export async function loadMonaco(): Promise<typeof Monaco> {
       const monaco = await import('monaco-editor/esm/vs/editor/editor.api');
       registerLean(monaco);
       monaco.editor.defineTheme('proofbattle-dark', editorTheme(monaco));
       return monaco;
     }
   Vite config: optimizeDeps: { exclude: ['monaco-editor'] } plus build.rollupOptions.output.manualChunks
   if needed. NEVER use @monaco-editor/loader. NEVER reference a CDN URL.
2. src/lib/editor/lean-grammar.ts: the Monarch grammar from design §4.3, translated to current
   TypeScript with correct types (Monaco.languages.IMonarchLanguage is generic in current
   monaco-editor typings — CHECK the actual signature in node_modules and match it). Fix the design
   doc's mistakes: its `operators` array mixes multi-char tokens (`:=`, `::`) with single characters in
   one character class, and `@default` inside a `cases` rule needs a defined target. Add NESTED block
   comments to the tokenizer (Lean nests /- -/) and proper string rules with escapes, so that
   `-- #eval` renders as a comment rather than code.
3. src/lib/editor/editor-theme.ts: design §4.4 verbatim, typed. Verify the contrast of the token
   colours against --editor-bg: #111113 (the same WCAG check as S7-P2, on a background where the
   design doc's defaults tend to fail). Paste the ratio table.
4. src/lib/editor/MonacoEditor.svelte (Svelte 5 runes, no createEventDispatcher):
   - typed  let { value = $bindable(''), readonly = false, diagnostics = [], onchange } = $props()
   - onMount -> loadMonaco() -> create the editor
   - $effect -> apply diagnostics to markers whenever they change, and CLEAR them when empty (a stale
     red squiggle is worse than none)
   - expose getValue(), insertAtCursor(text), focus() via bind:this + exported functions
   - options exactly as design §4.2, plus: unicodeHighlight.ambiguousCharacters: false (so ∀ and ℕ
     are not boxed as ambiguous), quickSuggestions, tabSize: 2, fixedOverflowWidgets: true,
     stickyScroll: { enabled: false } (it fights the panel layout)
   - onDestroy -> editor.dispose()
   - NO editor is created on the server. Dynamic import inside onMount only. Prove it:
     grep -rn "monaco" src/routes/   # route files must not import it statically
5. src/lib/editor/unicode-mapper.ts: design §4.5 with its bugs fixed. The design matches
   `textBefore.match(/\\([a-zA-Z]+)$/)` on a Space/Tab keydown, then computes the range as
   `column - match[0].length` — so the trigger needs a trailing space and the range math is off by the
   backslash. Rewrite as a PURE, unit-tested function
     expandAt(model, position) -> { range, text } | null
   tested against a real Monaco model. Keep the map from §4.5 and add the missing common entries
   (mapsto, circ, sum, prod, exists, and the le/ge aliases). Trigger on Space and Tab. Do NOT invent
   a "no space needed" mode — keep the proven VS Code behaviour and document the choice.
6. UnicodePalette.svelte (design §5): a clickable palette of the ~24 most-used symbols inserted at the
   cursor. Clicking must not steal focus from the editor: use onmousedown|preventDefault.

DO NOT
- Do not use @monaco-editor/loader, a CDN, or a web font from a CDN.
- Do not register the full monaco-editor (all languages). Import editor.api only.
- Do not add CodeMirror, Ace, or a textarea fallback. Monaco is the decision.
- Do not wire live diagnostics in this prompt (S8-P2).

ACCEPTANCE
  cd frontend && npm run check && npm run lint && npm test
  grep -rni "cdn\|jsdelivr\|unpkg" src/ build/ 2>/dev/null          # nothing
  npm run build
  # bundle proof: Monaco is in a separate lazy chunk and the landing page does not load it
  ls -la .svelte-kit/output/client/_app/immutable/chunks/ | sort -k5 -n | tail -5
  npm run preview &
  # in a headless browser with the network BLOCKED after load (Playwright route interception), navigate
  # to /game, type "theorem", and assert the token has the keyword colour class. Paste the screenshot
  # and the assertion output.
  npx playwright test --grep "editor loads offline"

DELIVERABLE
src/lib/editor/*, vite.config.ts, the contrast table, the chunk analysis, the offline-load test +
screenshot, ADR-011, standard report.
```

## S8-P2 — Live diagnostics with correct line mapping (the bug everyone ships)

```
CONTEXT
The player's tactic body is spliced into a generated file whose preamble is server-owned. Lean reports
errors against the GENERATED file. If the client maps those numbers to the body without subtracting the
preamble offset, every squiggle lands on the wrong line — the single most credibility-destroying bug
this project can ship. S4-P1 made the server emit body-relative line/col; this prompt renders them.

TASK
1. Verify the server's offset arithmetic and add the missing test. The preamble is
     <N imports> / set_option warningAsError true / set_option maxHeartbeats H / (blank) /
     theorem goal : G := by
   so the offset varies with N and MUST be computed from the actual generated text (wrap_problem
   already returns it), never hardcoded. Test: for a problem with 1 import and a problem with 4 imports,
   a Lean error at generated-line 7 maps to body-line 2 in BOTH cases. Add this to tests/corpus.rs if
   it is not already there.
2. src/lib/components/game/DiagnosticsBar.svelte (design §3.3/§5): renders Diagnostic[] with a
   severity icon, `line:col`, the message, and click-to-jump (monaco.editor.setPosition +
   revealLineInCenter). Show at most 5 with "+N more". Lean's messages are long and multi-line: trim to
   the first line in the bar, with the full text available in an expandable row.
3. Wire the check channel: EditorPanel debounces onchange at 800 ms (design §9 Approach A), calls
   checkProof(code), and sets lspDiagnostics on the verdict. While a check is in flight show a subtle
   "checking…" state; if check_skipped comes back, show nothing (silent, per S6-P2). A check verdict must
   NEVER overwrite the SUBMIT verdict's diagnostics — they are separate lists.
4. Marker rendering: convert Diagnostic -> IMarkerData in src/lib/utils/diagnostics.ts (written in
   S7-P3). Clamp every line/col into the model range — a diagnostic beyond EOF must not throw, because
   stale diagnostics WILL be out of range. Test it.
5. The submit path must be independent of the check path: pressing Submit with a check in flight must
   work, and a late-arriving check verdict must not clobber the submit result.
6. monaco.languages.registerCompletionItemProvider('lean4', …) with a curated static list of the ~30
   tactics and symbols from the design doc. Do NOT ship a Lean LSP completion engine client-side.

TESTS
  - Vitest: diagnostics -> markers conversion, including out-of-range clamping, unicode surrogate pairs
    within a line, and a message containing newlines
  - Vitest: a scripted check verdict arriving AFTER a submit verdict does not overwrite it
  - Playwright: submit a deliberately wrong proof against the mock, assert the DiagnosticsBar shows a
    line number matching the BODY (not the generated file), and that clicking it moves the cursor there

DO NOT
- Do not parse Lean stderr on the client. Ever. The server sends structured diagnostics (ADR-007).
- Do not implement a real `lean --server` LSP (design §9 Approach B). That is ~150 MB per session and
  the design doc's own senior note defers it.
- Do not go outside 500-1500 ms for the debounce; document your choice.
- Do not add a syntax-highlight preview library; Monaco already does it.

ACCEPTANCE
  npm run check && npm run lint && npm test && npm run test:e2e
  cargo test --manifest-path server/Cargo.toml corpus::        # the offset test from step 1
  # manual proof: in the mock, enter a body whose 3rd line has a syntax error. Screenshot the
  # DiagnosticsBar and the squiggle. They must be on the SAME line, and that line must be 3.

DELIVERABLE
DiagnosticsBar, the check wiring, the offset test, the tests, the screenshot, standard report.
```

---

# Stage S9 — Polish, Observability, Deployment, Load Testing

**Goal**: prove it holds up under load and make it operable.

**Exit gate**: the load target is met, /metrics is scraped, TLS/QUIC terminates in front, CI is green on
a clean clone, and the design doc's §8 checklist has every box honestly ticked (or an explicit reason it
is not ticked).

---

## S9-P1 — Measure, then settle the worker-pool question (ADR-012)

```
CONTEXT
Design §3 asserts that a pre-warmed Lean worker pool turns 15 s cold starts into <2 s and is therefore
required. Measured on this machine: a cold `lake env lean` is 3.7 s, and a cold container run adds its
own overhead. The claim is a hypothesis, not a fact. This stage measures and then DECIDES — the answer
may legitimately be "the pool is not needed yet".

TASK
Write scripts/bench_verify.sh producing a table over N=30 runs:
  scenario                          | p50 | p95 | p99 | errors
  ----------------------------------+-----+-----+-----+-------
  local cold process                |
  sandboxed cold container          |
  sandboxed warm container (if a warm mode exists)
  2 concurrent (pool=2)             |
  4 concurrent (pool=4)             |
  8 concurrent (pool=2) [saturated] |
Then compute the maximum sustainable games/second from p95 and the pool size, and compare it with
S2-P1's non-blocking concurrency number.
DECIDE, in writing (ADR-012), one of:
  (a) keep the cold-spawn model, state the concurrency ceiling it implies, and implement what happens at
      that ceiling (shed load vs rate-limit submissions — pick one, implement it, and record it)
  (b) build the warm pool, and only then design the job-to-worker framing protocol, the worker restart
      policy, the MEASURED memory budget (not the design doc's "150MB per LSP" guess), and the health
      check
Justify with the measured numbers. If the numbers say (a), shipping (a) is the correct outcome and you
must say so plainly rather than building (b) to satisfy the design doc.
ALSO measure the sandbox container's cold start and the final image size, and include both in the ADR —
an image over ~2 GB because of the olean cache is a deployment finding that belongs in the report.

DO NOT
- Do not build the pool before measuring.
- Do not measure on a loaded machine: check nproc, uptime, and free memory first and paste them.

ACCEPTANCE
  bash scripts/bench_verify.sh          # paste the full table
  ADR-012 with a decision and the numbers behind it
  # if (b): the pool must pass S6-P2's fairness tests
  # if (a): the load-shedding or rate-limit path must have a test
```

## S9-P2 — Observability: metrics, tracing, structured errors

```
CONTEXT
Constitution rule 7. S6-P2 built the in-flight snapshot channel. This stage exposes it.

TASK
1. /metrics (Prometheus text format, hand-rolled — no prometheus crate) at GET /metrics:
     proofbattle_players_online                gauge
     proofbattle_games_in_progress             gauge
     proofbattle_rooms_total                   counter
     proofbattle_lean_verifications_total{verdict,reason}   counter   (cardinality-bounded)
     proofbattle_lean_duration_seconds         histogram (buckets: .5, 1, 2, 4, 8, 16, 32, 60)
     proofbattle_lean_queue_wait_seconds        histogram
     proofbattle_queue_dropped_total{lane}     counter
     proofbattle_rate_limited_total            counter
     proofbattle_sandbox_rejected_total{reason} counter
     proofbattle_ws_connections                gauge
   LABEL DISCIPLINE: never label with player_id, room_id, or problem_id. Unbounded label cardinality
   will kill the metrics backend. State this in a comment.
2. Structured JSON logs in production (tracing-subscriber fmt().json()), pretty in development, with
   request_id / room_id / player_id / elapsed_ms / verdict on every game event.
3. A /healthz (liveness) and /readyz (readiness: DB reachable, Lean executor warm, sandbox image
   present) endpoint. /readyz must return 503 when sandbox_enabled is true but the image is missing —
   otherwise the deploy silently runs unsandboxed.
4. A dashboard-shaped queries file (infra/monitoring/queries.md) with the PromQL for: p95 verification
   latency, rejection rate by reason, queue drops, games completed per minute, and sandbox rejections.

ACCEPTANCE
  cargo run --manifest-path server/Cargo.toml & curl -s localhost:3000/metrics | head -40
  curl -s -o /dev/null -w '%{http_code}' localhost:3000/healthz     # 200
  curl -s -o /dev/null -w '%{http_code}' localhost:3000/readyz      # 503 when the image is missing
  # cardinality guard: run a 100-game soak and assert the number of distinct metric label sets does not
  # grow with the number of games. Paste the count.
  cargo clippy --manifest-path server/Cargo.toml -- -D warnings

DELIVERABLE
The metrics module, the two health endpoints, the JSON log config, infra/monitoring/queries.md, the
soak cardinality result, report.
```

## S9-P3 — Deployment: compose, nginx TLS + HTTP/3, CI

```
CONTEXT
Design §3 shows Nginx terminating TLS and QUIC/HTTP/3 in front of the Rust server, plus Redis and
Postgres. Design §4 argues QUIC removes head-of-line blocking and that WebRTC is wrong for this use
case. Follow that.

TASK
1. infra/docker-compose.yml (production profile) with: server, sandbox, postgres, redis, nginx. Health
   checks and depends_on conditions for all of them. Named volumes for postgres and for the Lean olean
   cache. No hardcoded secrets — env_file only.
2. infra/nginx/nginx.conf:
     listen 443 ssl http2;  listen 443 quic reuseport;  http3 on;
     add_header Alt-Svc 'h3=":443"; ma=86400';
   plus: TLS 1.3, a security header set (HSTS, X-Content-Type-Options, Referrer-Policy, a CSP that does
   NOT need to allow a CDN — this is what ADR-011 bought you), WebSocket upgrade headers with a long
   read timeout on /ws, client_max_body_size small, and gzip. No server_tokens.
   Note in a comment: ALPN advertises h2 and h3; browsers without QUIC fall back to HTTP/2 transparently.
3. infra/nginx/certs/README.md explaining certbot usage. Do NOT commit a certificate.
4. CI: .github/workflows/ci.yml running, on every PR: cargo fmt --check, cargo clippy -D warnings,
   cargo test (with a Postgres service container and migrations applied), the S3 corpus with
   -- --ignored, and on the frontend: npm ci, npm run check, npm run lint, npm test, npm run build,
   npm run test:e2e. Cache the Lean olean directory with actions/cache keyed on lean-toolchain + a
   hash of lakefile.lean — a cold Mathlib build in CI would take longer than the CI timeout.
5. A README.md at the repo root: architecture diagram (copy the mermaid from design §2/§3, corrected for
   the actor model), a local development quickstart (make bootstrap && make db-up && make seed &&
   make run-server), an environment variable reference, and an honest "Known limitations" section.

DO NOT
- Do not commit secrets, certs, or a .env with real values.
- Do not use a wildcard TLS cert.
- Do not put the docker socket in the compose file for the server (that is the sandbox's job, and it is
  root-equivalent — say so in a comment).
- Do not add a Kubernetes manifest. Compose plus nginx is the deployment.

ACCEPTANCE
  docker compose -f infra/docker-compose.yml config        # validates
  docker compose -f infra/docker-compose.yml up -d && curl -sf localhost:8080/healthz
  # HTTP/3 proof (paste the output):
  curl --http3 -sI https://localhost/healthz    # or: openssl s_client -alpn h3
  # CI proof: push the branch and paste the run's check list. Do not claim it is green without it.
  # E2E against the deployed stack: paste the Playwright run against the preview server.

DELIVERABLE
infra/docker-compose.yml, infra/nginx/*, .github/workflows/ci.yml, README.md, the compose validation
and health-check output, the HTTP/3 evidence, the CI check list, report.
```

## S9-P4 — Load test, final review, and the honest checklist

```
CONTEXT
This is the stage that answers "does the thing work at scale", and the stage that makes the design
doc's §8 checklist truthful.

TASK
1. A load harness: N scripted WebSocket clients (write it in Rust as a `benches/` or `xtask/` binary
   using the same protocol types — NOT a JS script, so it exercises the real serialisation). Parameters
   via env: clients, games, submission rate, and an injected verify latency via the test-only Verifier
   trait from S2-P3 so the load is reproducible without Lean.
2. Run the matrix and paste the results:
     50 concurrent games, 4 Lean workers, realistic submission pattern
   Report: games completed, games abandoned by timeout, p50/p95/p99 verification latency, p95
   end-to-end verdict delivery, queue drops, peak RSS, and any 5xx / ServerError counts.
   State the target you are measuring against and whether it was met. If it was not met, say so and name
   the bottleneck with evidence (profile it, do not speculate).
3. Soak: 1000 games sequentially, assert zero leaked rooms, zero unbounded memory growth (RSS
   before/after with the numbers), zero goroutine/task growth (tokio metrics), and no file descriptor
   leak in the proof temp dir (count .lean files at start and end — it must be zero).
4. Final review against proof_battle_design.md §8 and frontend_design.md §11. For EVERY checkbox,
   produce one line: DONE (with file:line evidence) / NOT DONE (with the reason) / N/A. No optimistic
   ticks. The items most likely to be honestly NOT DONE are: live LSP streaming (design §9 Approach B),
   Redis-backed sessions, Mathlib-derived problem count, and QUIC in production. Say so.
5. Update docs/DECISIONS.md with any ADR the work produced, and docs/ENVIRONMENT.md with the final
   measured numbers so the next person is not misled by the baselines in this plan.

DO NOT
- Do not mark a checklist item DONE without file:line evidence.
- Do not run the load test on an unloaded machine without pasting nproc / free / uptime.
- Do not "fix" a failing metric by raising a timeout. That is how a platform dies quietly.

ACCEPTANCE
  cargo run --manifest-path server/Cargo.toml --bin loadgen -- --clients 50 --games 50
  cargo test --manifest-path server/Cargo.toml soak:: -- --ignored --nocapture
  # paste: the full results matrix, the RSS/FD/task deltas, and the honest §8 checklist

DELIVERABLE
The load harness, the results matrix, the soak deltas, the complete §8 checklist with evidence,
updated ADRs and ENVIRONMENT.md, and:
  git add -A && git commit -m "stage S9: observability, deployment, load test, final review" && git tag stage-s9
```

---

# Appendix A — Recovery Prompts

Use these when a stage fails or drifts. Each is short and paste-able.

## A1 — When an agent claims success without evidence

```
Your report claims the acceptance criteria passed, but the pasted evidence does not include the
output of: <paste the exact command>.

Re-run every acceptance command from the task, one at a time, and paste the COMPLETE stdout+stderr of
each, including the exit code. Do not summarise, do not paraphrase, do not describe what you expect to
see. If a command fails, fix the cause and re-run. If a command cannot pass, say exactly why and what
you would need.
```

## A2 — When the agent went out of scope

```
Review your own diff against the task. List every change that was not required by the task, however
small. For each: either REVERT it now, or justify it in one sentence under "Recommended next work" and
revert it anyway. Then re-run the acceptance criteria and paste the output.

An unrelated improvement is not a gift here — it is unverified risk. Revert it.
```

## A3 — When a security control was weakened

```
Your change to <file> weakens a control that the task's CONTEXT section justified. Specifically:
<quote the requirement you broke>.

Restore the control. Then write a test that fails if the control is ever removed again — a test that
exercises the behaviour, not one that greps the source. Then re-run the full stage acceptance criteria
and paste the output, including the new test's failure output when the control is temporarily disabled.
```

## A4 — When the model invents a fact

```
You stated: "<the invented fact>". You must not state anything you did not measure or read.

Either (a) run the command that produces it and paste the output, or (b) delete the claim. If the
information is not obtainable in this environment, say "UNKNOWN — not measurable here" and explain what
would be needed. Do not fill gaps with plausible values.
```

## A5 — Mid-stage context loss

```
STOP. Before continuing, reconstruct the state of the work by reading the files, not from memory:
1. `git status --short` and `git log --oneline -5` — what is committed?
2. Read docs/DECISIONS.md — which ADRs are in force?
3. Read the files the task names and summarise their current contents in 5 lines each.
4. Re-state the task's acceptance criteria verbatim.
5. Re-state what is DONE and what is NOT, based only on that evidence.
6. Then continue from there. Do not re-do completed work, and do not assume anything you have not read.
```

---

# Appendix B — Adversarial Corpus (required at S3-P3)

Every case below must exist as a fixture in `tests/corpus/` and pass. The `body` is the player's tactic
submission — the server wraps it in the challenge statement and preamble.

| # | id | category | body | expected | why it exists |
|---|---|---|---|---|---|
| 1 | `cheat_sorry` | cheat | `sorry` | reject (`UsesSorry`) | The headline bug: exits 0 and wins under the design doc's rule |
| 2 | `cheat_sorry_embedded` | cheat | `intro n\nsorry` | reject (`UsesSorry`) | Sorry hidden behind real tactics |
| 3 | `cheat_sorry_in_simpa` | cheat | `simpa using (by sorry)` | reject (`RejectedByFilter` or `UsesSorry`) | `sorry` as a term, not a bare tactic |
| 4 | `cheat_builtin_axiom` | cheat | `exact Classical.choice (propComplete True)`-style reliance on sorryAx | reject | Exotic `sorryAx` routes |
| 5 | `cheat_statement_swap` | cheat | `theorem goal : True := trivial` | reject (`RejectedByFilter`) | Tactic-body-only format makes declarations impossible (ADR-002) |
| 6 | `cheat_option_downgrade` | cheat | `set_option warningAsError false in exact trivial` | reject (`RejectedByFilter`) | Must not weaken the server preamble |
| 7 | `cheat_maxheartbeats` | dos | `set_option maxHeartbeats 100000000 in simp` | reject (`RejectedByFilter`) | Kernel-time DoS knob |
| 8 | `rce_eval_io` | rce | `#eval IO.getEnv` | reject (`RejectedByFilter`) | Measured live in this environment — arbitrary code execution |
| 9 | `rce_eval_shell` | rce | `#eval System.Platform.isOSWindows` and a System.Process.run payload | reject (`RejectedByFilter`) | Design doc §1.3's canonical RCE payload |
| 10 | `rce_native_decide` | rce | `exact (by native_decide)` | reject (`RejectedByFilter`) | Executes compiled native code |
| 11 | `rce_extern` | rce | `unsafe` + `@[extern "…"]` | reject (`RejectedByFilter`) | FFI escape |
| 12 | `rce_macro_elab` | rce | `macro_rules \| \`(x) => (IO.println "x")` | reject (`RejectedByFilter`) | Metaprogramming that reaches IO |
| 13 | `rce_initialize` | rce | `initialize registerBuiltinAttribute 1` | reject (`RejectedByFilter`) | Lean extension init hook |
| 14 | `rce_attribute_instance` | rce | `attribute [instance] Foo.bar` | reject (`RejectedByFilter`) | Environment mutation inside a proof |
| 15 | `dos_decide_hard` | dos | `decide` on a hard proposition | reject OR accept (`TimedOut` allowed) | Wall timeout + maxHeartbeats must bound it |
| 16 | `dos_huge_term` | dos | deep `Nat.rec` / 5000-element term | reject (`TooLarge` or `TimedOut`) | Line/byte limits + heartbeat limit |
| 17 | `limit_too_many_lines` | limit | a 200-line body | reject (`TooLarge`) | `proof_max_lines` |
| 18 | `limit_too_many_bytes` | limit | a 10 000-byte body | reject (`TooLarge`) | `proof_max_bytes` |
| 19 | `filter_false_positive` | filter | `-- #eval is dangerous\nexact "#eval"` | **accept** path (no filter error) | A naive `contains` denylist fails this |
| 20 | `filter_nested_comment` | filter | `/- outer /- #eval -/ still comment -/\nexact Nat.add_zero n` | accept (or a Lean error, NOT a filter error) | Nested block comments must be stripped correctly |
| 21 | `happy_add_zero` | happy | `intro n\nsimp` | accept | The baseline legitimate proof |
| 22 | `happy_add_comm` | happy | `intro a b\nsimp [Nat.add_comm]` | accept | A second legitimate proof, different tactic |
| 23 | `happy_induction` | happy | an induction-based legitimate proof | accept | The filter must not block real tactic scripts |
| 24 | `lean_aggregate_import` | env | a body needing `import Mathlib` | reject | The aggregate olean does not exist (measured) |
| 25 | `stderr_flood` | dos | `set_option trace.Meta.simp.rewrite true` + a big simp set | accept/reject, but stderr_bytes <= 64 KiB | Output cap (S2-P1) |

Rules for the corpus:
- Cases 19 and 20 must NOT produce a filter error. They are the tests that a future agent cannot quietly
  replace an allow-list with a denylist.
- Case 1 must be rejected. If it is ever accepted, the judge is unsound and the product is worthless —
  treat it as a Sev-0.
- Every case asserts a specific `reject_reason`, not merely "rejected".
- Adding cases is encouraged; deleting them is not allowed.

---

# Appendix C — Protocol v1 (normative)

The Rust enums at `server/src/ws/message.rs` are the implementation of this table. The generated
TypeScript at `frontend/src/lib/ws/generated.ts` is derived from them. `docs/PROTOCOL.md` (S4) is the
human-readable copy. If any of the three disagree, the Rust is authoritative and the other two are
regenerated.

Field names are `snake_case`. Variant tags are `PascalCase`. Every message is a JSON object with a
`type` discriminator.

## Client → Server

| type | fields | legal in state | notes |
|---|---|---|---|
| `Hello` | `version: u32`, `token: Option<String>`, `username: Option<String>` | `connecting` | MUST be the first frame. `token` is the session token from a previous `Welcome`. |
| `QueueJoin` | — | `connected_idle`, `reattaching` | Joins the matchmaking queue. |
| `QueueLeave` | — | `matchmaking` | Cancels matchmaking. |
| `ProofRequest` | `req_id: String`, `code: String`, `intent: "Submit" \| "Check"` | `in_game`, `submitting` | `code` is the TACTIC BODY only (ADR-002). `Check` never affects game state and never counts against the submission rate limit. |
| `Resign` | — | `in_game`, `submitting` | Immediate RoundEnd. |
| `Pong` | `ts: i64` | any connected | Heartbeat response. |

**Invariant: no client message contains `player_id`, `room_id`, `winner`, or `elo`.** Enforced by
`tests/protocol_contract.rs`.

## Server → Client

| type | fields | meaning |
|---|---|---|
| `Welcome` | `player_id: String`, `session_token: String`, `server_time_ms: i64`, `heartbeat_interval_ms: i64`, `protocol_version: u32` | Identity assigned by the server. Store the token for reconnection. |
| `ServerTime` | `server_time_ms: i64` | Emitted every 15 s. The client computes its clock offset from this; never trust a local clock for game timing (ADR-004). |
| `QueueStatus` | `queue_size: u32`, `elo: i32`, `search_range: {min_elo: i32, max_elo: i32}` | Matchmaking telemetry. |
| `MatchFound` | `room_id: String`, `you: PlayerInfo`, `opponent: PlayerInfo` | Both players receive this; `opponent` never carries code. |
| `RoundStart` | `room_id: String`, `problem: Problem`, `starts_at_ms: i64`, `ends_at_ms: i64`, `duration_ms: i64`, `server_time_ms: i64`, `seq: u64` | Full state resync on reconnect. |
| `Verdict` | `req_id: String`, `verdict: "Accepted" \| "Rejected" \| "Error"`, `reason: Option<RejectReason>`, `message: String`, `diagnostics: Diagnostic[]`, `elapsed_ms: u64`, `check_skipped: bool` | Answer to a `ProofRequest`, correlated by `req_id`. `diagnostics` lines are 1-based and relative to the player's tactic body, not the generated file. |
| `Diagnostic` | `line: u32`, `col: u32`, `end_line: u32`, `end_col: u32`, `severity: "error" \| "warning" \| "info"`, `message: String` | Never contains filesystem paths. |
| `OpponentActivity` | `status: "Idle" \| "Typing" \| "Checking" \| "Verifying" \| "Submitted"` | Server-inferred only. Never client-asserted; never reveals code. |
| `RoundEnd` | `room_id: String`, `outcome: "Won" \| "Lost" \| "Draw" \| "ForfeitWin" \| "ForfeitLoss"`, `winner_id: Option<String>`, `winning_proof: Option<String>`, `canonical_proof: Option<String>`, `elo_delta: i32`, `duration_ms: i64`, `seq: u64` | Both players receive the SAME `outcome`; the client never derives it by comparing ids. |
| `ServerError` | `code: String`, `message: String`, `retry_after_ms: Option<u64>`, `retryable: bool` | `codes`: `bad_request`, `invalid_state`, `rate_limited`, `busy`, `unsupported_version`, `internal`. |
| `Pong` | `ts: i64` | Echo of the server ping. |
| `PlayerInfo` | `player_id: String`, `username: Option<String>`, `elo: i32` | Public profile only. |
| `Problem` | `id: String`, `goal: String`, `imports: String[]`, `difficulty: u8`, `category: String`, `hint: Option<String>`, `duration_ms: i64` | The challenge. The canonical solution is NEVER sent during a game. |

### Connection state machine (server side, mirrored by the client at S7-P3)

```
disconnected ──CONNECT──> connecting ──OPEN──> connected_idle
connecting ──ERROR────────────────────────────> error
connected_idle ──MSG_JOINED───────────────────> connected_idle
connected_idle ──JOIN_QUEUE──────────────────> matchmaking
connected_idle ──DISCONNECT──────────────────> disconnected
matchmaking ──MSG_MATCH_FOUND────────────────> game_starting
game_starting ──MSG_CHALLENGE────────────────> in_game
in_game ──SUBMIT_PROOF───────────────────────> submitting
in_game ──MSG_GAME_ENDED────────────────────> game_ended
in_game ──DISCONNECT─────────────────────────> reconnecting
submitting ──MSG_PROOF_RESULT────────────────> in_game
submitting ──MSG_GAME_ENDED──────────────────> game_ended
reconnecting ──OPEN + valid token────────────> in_game        (full resync)
reconnecting ──OPEN + invalid/absent token───> connected_idle
reconnecting ──timeout────────────────────────> connected_idle (opponent got ForfeitWin)
game_ended ──CONNECT─────────────────────────> connecting
error ──CONNECT──────────────────────────────> connecting
```

Any frame not legal in the current state is answered with `ServerError { code: "invalid_state" }` and
logged at debug. This is what makes "submit a proof before you have a problem" impossible rather than
merely unlikely.

---

# Appendix D — Quick Reference

| Item | Value |
|---|---|
| Lean | `leanprover/lean4:v4.21.0-rc3` (fine-grained imports only; `import Mathlib` does not resolve) |
| Rust | edition 2024, min 1.85 |
| Node | 22.x |
| Frameworks | Axum 0.7+, Tokio, sqlx, ts-rs, SvelteKit 2 + Svelte 5, Tailwind v4, Monaco (npm, offline) |
| Verification | `lake env lean <file>`, ~3.7 s cold, 30 s timeout, `maxHeartbeats 200000`, `warningAsError true` |
| Sandbox | one-shot `docker run --rm`, `--network none`, `--read-only`, `--cap-drop ALL`, seccomp allow-list |
| Stages | S0 baseline · S1 layout · S2 correctness · S3 soundness · S4 protocol · S5 content · S6 game engine · S7 frontend · S8 editor · S9 ops |
| Hard gates | S0 harness green · S3 corpus 100% · S4 codegen clean · S5 all problems verified · S9 honest checklist |

> **Bottom line**: the skeleton is right; the surroundings are missing. Fix the async blocker and the
> judge soundness first — those are the two that will hurt immediately. The single most important fact in
> this plan is that `sorry` currently wins every match, and that fact is a measurement, not an opinion.

