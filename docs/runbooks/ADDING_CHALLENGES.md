# Runbook: Authoring & Sourcing Lean 4 Challenges

This runbook guides curriculum designers and engineers through adding, formally verifying, and seeding mathematical problems into ProofBattle's competitive catalog.

---

## 1. Challenge Format & Invariants

Every challenge problem must meet strict soundness and formal verification standards:
1. **Server-Owned Header (ADR-002)**: The problem defines the theorem goal, required Mathlib imports, and variable binders. The player supplies *only* the tactic body inside `:= by`.
2. **Canonical Proof Soundness (ADR-003)**: Every problem **must** have a tested, canonical Mathlib tactic solution that compiles cleanly without `sorry`, `sorryAx`, or compiler warnings.
3. **No Unsound Axioms**: Custom axioms or unproven hypotheses are prohibited.
4. **Time & Heartbeat Bounds**: The canonical proof must verify within 5 seconds and 200,000 Lean heartbeats.

---

## 2. Problem Schema

Problems are represented in JSON Lines (`.jsonl`) format:

```json
{
  "id": "algebra_am_gm_quadratic",
  "category": "algebra",
  "difficulty": 2,
  "goal": "(a + b)^2 - 4 * a * b = (a - b)^2",
  "imports": ["Mathlib.Data.Real.Basic"],
  "binders": "(a b : ℝ)",
  "canonical_solution": "ring",
  "duration_ms": 180000,
  "hint": "Expand both sides or use polynomial ring normalization."
}
```

### Difficulty Scale
- **Tier 1 (Beginner, 1000–1200 Elo)**: Direct tactic applications (`rfl`, `simp`, `linarith`, `ring`, `omega`).
- **Tier 2 (Intermediate, 1200–1500 Elo)**: Basic algebra, induction on Nat, rewrite chains with Mathlib lemmas.
- **Tier 3 (Advanced, 1500–1800 Elo)**: Multi-step inequalities, auxiliary `have` steps, set theory, modular arithmetic.
- **Tier 4 (Master, 1800–2100 Elo)**: AMC/AIME competition problems, analysis bounds, real/complex numbers.
- **Tier 5 (Grandmaster, 2100+ Elo)**: Olympiad-level (USAMO/IMO) propositions requiring non-trivial lemma construction.

---

## 3. Formal Verification & Seeding Workflow

### Step 1: Add Problem Definitions
Add candidate theorem definitions to `data/problems_curated.jsonl` or [`server/src/challenges.rs`](file:///home/gulshansharma/proof_battle/server/src/challenges.rs).

### Step 2: Run Verification CLI
The server provides a dedicated verification CLI tool that compiles each problem against Lean 4 in parallel:

```bash
cargo run --manifest-path server/Cargo.toml --bin verify_problems -- --help
```

To verify and seed directly into the database:
```bash
DATABASE_URL="postgres://proofbattle:proofbattle_dev@localhost:5432/proofbattle" \
TARGET_VERIFIED=120 VERIFY_CONCURRENCY=8 \
cargo run --manifest-path server/Cargo.toml --bin verify_problems
```

### Step 3: Run Integrity Audit
Verify that no `sorryAx` axioms exist in the accepted problems:
```bash
python3 scripts/ml/audit_solutions_integrity.py
```
