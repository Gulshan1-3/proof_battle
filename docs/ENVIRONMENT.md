# Environment Baseline

This document records the measured facts about the ProofBattle execution environment on this machine.
Any change to these baseline values in subsequent development stages must be updated here.

| Fact | Value | Status | Established By / Reproduction Command |
|---|---|---|---|
| Lean toolchain | `leanprover/lean4:v4.21.0-rc3` | Measured | `cat lean/lean-toolchain` |
| Lake version | `5.0.0-6741444 (Lean 4.21.0-rc3)` | Measured | `cd lean && lake --version` |
| Rust toolchain | `1.90.0 (edition 2024 supported)` | Measured | `cargo -V` |
| Node / npm | `v22.14.0 / 10.9.2` | Measured | `node -v && npm -v` |
| Docker daemon | `Docker version 27.5.1, build 9f9e405` | Measured | `docker --version && docker info` |
| PostgreSQL client / server | Client `16.8` present, **server not running** | Measured | `pg_isready` (exits non-zero: "no response") |
| Rust crate location | `server/src/` + `server/Cargo.toml` | Measured | `ls -la server/src server/Cargo.toml` |
| Prebuilt oleans | `6550` `.olean` under `lean/.lake/build/lib/lean/` | Measured | `find lean/.lake/build -name '*.olean' \| wc -l` |
| Mathlib sources | `6549` `.lean` files under `lean/Mathlib`, `0` `.olean` | Measured | `find lean/Mathlib -name '*.lean' \| wc -l` and `find lean/Mathlib -name '*.olean' \| wc -l` |
| Lakefile hygiene | Single `lakefile.lean` (after removing duplicate `lakefile.toml`) | Measured | `cd lean && lake env lean --version 2>&1 \| grep -c 'both present'` -> `0` |
| Fine-grained import | `import Mathlib.Data.Nat.Basic` succeeds in ~1.1-3.7 s | Measured | `cd lean && time (echo 'import Mathlib.Data.Nat.Basic' \| lake env lean --stdin)` |
| Aggregate import failure | `import Mathlib` fails (`Mathlib.olean does not exist`) | Measured | `cd lean && echo 'import Mathlib' \| lake env lean --stdin` |
| Unindented tactics | Unindented tactics inside `by` are legal | Measured | `cd lean && printf 'import Mathlib.Data.Nat.Basic\ntheorem goal (n : Nat) : n + 0 = n := by\nrfl\n' \| lake env lean --stdin` |
| Unsolved goals | Unsolved goals fail with exit code 1 | Measured | `cd lean && printf 'import Mathlib.Data.Nat.Basic\ntheorem goal (n : Nat) : n + 0 = n := by\nintro n\n' \| lake env lean --stdin` |
| `sorry` passes by default | `by sorry` emits warning and exits 0 (soundness issue) | Measured | `cd lean && printf 'import Mathlib.Data.Nat.Basic\ntheorem goal (n : Nat) : n + 0 = n := by\n  sorry\n' \| lake env lean --stdin` |
| `warningAsError` behavior | `set_option warningAsError true` turns `sorry` into error (exit 1) when placed after imports | Measured | `cd lean && printf 'import Mathlib.Data.Nat.Basic\nset_option warningAsError true\ntheorem goal (n : Nat) : n + 0 = n := by\n  sorry\n' \| lake env lean --stdin` |
| Option downgrade scope | `set_option warningAsError false` inside tactic block causes syntax error | Measured | Lean requires `set_option ... in term` scoping, preventing in-tactic option overriding |
| `#eval` execution | `#eval` executes arbitrary Lean/IO code at elaboration | Measured | `cd lean && printf 'import Mathlib.Data.Nat.Basic\n#eval IO.getEnv\n' \| lake env lean --stdin` |
| Statement shadowing | Redundant theorem identifier declaration fails with exit 1 | Measured | `cd lean && printf 'import Mathlib.Data.Nat.Basic\ntheorem goal (n : Nat) : n + 0 = n := by rfl\ntheorem goal : True := trivial\n' \| lake env lean --stdin` |
| SvelteKit frontend location | SvelteKit 2 + Svelte 5 frontend at `frontend/` | Target S1 | Scaffolded placeholder in S1, full UI built in S7 |
| Server crate location | Axum + Tokio backend at `server/` | Measured | Relocated to `server/` in Stage S1 |
