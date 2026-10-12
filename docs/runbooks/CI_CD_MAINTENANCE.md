# Runbook: CI/CD Pipeline Architecture & Maintenance

This runbook documents the GitHub Actions Continuous Integration (CI) pipeline configured in [`.github/workflows/ci.yml`](file:///home/gulshansharma/proof_battle/.github/workflows/ci.yml).

---

## 1. CI Pipeline Architecture

The CI pipeline runs automatically on pull requests and pushes to `main` and `arena-simulation`. It consists of three parallel jobs:

```text
                                  GitHub Actions Trigger
                                     (Push / PR)
                                          |
                +-------------------------+-------------------------+
                |                         |                         |
                v                         v                         v
    +-----------------------+ +-----------------------+ +-----------------------+
    | Frontend Lint & Build | | Backend Lint & Format | | Backend Test Suite    |
    | - Prettier formatting | | - cargo fmt --check   | | - Postgres Service    |
    | - ESLint rules        | | - cargo clippy        | | - Mock Lake Runner    |
    | - svelte-check        | |   (-D warnings)       | | - 116+ Server Tests   |
    | - vitest & vite build | | - Cargo caching       | | - Auto-seeded DB      |
    +-----------------------+ +-----------------------+ +-----------------------+
```

---

## 2. Key CI Engineering Solutions

### A. The Lightweight Mock Lake Runner (`mock_lake.sh`)
- **The Problem**: Running full Lean 4 Mathlib builds in GitHub Actions requires downloading and building 3+ GiB of `.olean` object files, slowing CI to 15+ minutes and causing out-of-disk failures.
- **The Solution**: We created [`server/tests/mock_lake.sh`](file:///home/gulshansharma/proof_battle/server/tests/mock_lake.sh), a lightweight deterministic shell runner that simulates Lean 4 verification responses.
- **Environment Hook**: In [`server/src/lean/runner.rs`](file:///home/gulshansharma/proof_battle/server/src/lean/runner.rs), `find_lake_binary()` inspects the `LAKE_PATH` environment variable before looking for system Lake:
  ```yaml
  env:
    LAKE_PATH: ${{ github.workspace }}/server/tests/mock_lake.sh
  ```
  This allows tests to execute hermetically in under 90 seconds without network or toolchain dependencies.

### B. Fresh Database Auto-Seeding in CI
- In CI, the `postgres:15-alpine` service container launches empty with 0 rows.
- Tests expecting problem catalogs automatically call `seed_if_empty()` in [`server/tests/db.rs`](file:///home/gulshansharma/proof_battle/server/tests/db.rs), seeding the 120+ standard problems into the test database on startup.

---

## 3. Local Verification Before Pushing

Before pushing a commit, run the identical checks locally:

```bash
# 1. Frontend validation
npm run lint --prefix frontend
npm run check --prefix frontend
npm run build --prefix frontend

# 2. Rust formatting & linter
cargo fmt --check --manifest-path server/Cargo.toml
cargo clippy --manifest-path server/Cargo.toml -- -D warnings

# 3. Rust test suite
LAKE_PATH="$(pwd)/server/tests/mock_lake.sh" \
cargo test --manifest-path server/Cargo.toml

# 4. Documentation link check
make docs-check
```

---

## 4. Troubleshooting Common CI Failures

| Failure Mode | Diagnosis | Resolution |
| :--- | :--- | :--- |
| **Clippy `-D warnings`** | Rust version differences flag newly introduced lints (e.g., `collapsible_match`, `collapsible_if`). | Run `cargo clippy --manifest-path server/Cargo.toml --fix --allow-dirty` locally and commit fixes. |
| **Prettier Formatting Error** | File formatting differs from `.prettierrc`. | Run `npx prettier --write frontend/src/` to format frontend files. |
| **Lean Test OS Error 2** | `lake` executable not found on CI runner. | Ensure `LAKE_PATH: ${{ github.workspace }}/server/tests/mock_lake.sh` is set in the job environment. |
| **Postgres Connection Refused** | Database container not ready when test suite starts. | CI workflow uses service health checks with `pg_isready -U proofbattle`. |
