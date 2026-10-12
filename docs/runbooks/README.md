# ProofBattle Operational Runbooks & Procedures

This directory contains executable, step-by-step operational runbooks for developing, deploying, testing, and maintaining the ProofBattle platform and its AI prover subsystem.

---

## Runbook Catalog

| Runbook | Purpose | Frequency |
| :--- | :--- | :--- |
| [LOCAL_DEVELOPMENT.md](LOCAL_DEVELOPMENT.md) | Initial environment bootstrap, running the full stack locally. | Onboarding & Daily Dev |
| [ADDING_CHALLENGES.md](ADDING_CHALLENGES.md) | Sourcing, authoring, verifying, and seeding Lean 4 problem sets. | Content Updates |
| [BENCHMARKING_AND_LOAD.md](BENCHMARKING_AND_LOAD.md) | Running verification latency benchmarks (`anchor_bench`) and load generation (`loadgen`). | Performance Audits |
| [CI_CD_MAINTENANCE.md](CI_CD_MAINTENANCE.md) | Maintaining GitHub Actions workflows, Mock Lake runner, and CI caches. | CI Troubleshooting |

---

## Documentation Maintenance Cadence

To prevent documentation decay as code evolves:

### 1. The Pull Request Documentation Contract
Any pull request that introduces architectural, protocol, or data model changes **must** update the corresponding documentation:
- **WebSocket Protocol Changes**: Update [`docs/platform/PROTOCOL.md`](../platform/PROTOCOL.md) and run `make codegen` to synchronize Rust and TypeScript types.
- **Model or Neural Changes**: Update [`docs/model/PROOF_BDH.md`](../model/PROOF_BDH.md) or [`docs/model/DBF_MCTS.md`](../model/DBF_MCTS.md) and verify parameter limits via `python3 scripts/ml/audit_solutions_integrity.py`.
- **Database Schema Changes**: Add SQL migration in `server/migrations/` and document updated tables in [`docs/platform/BACKEND.md`](../platform/BACKEND.md).
- **Architectural Deviations**: Record rationale in [`docs/DECISIONS.md`](../DECISIONS.md).

### 2. Automated Documentation Freshness Checks
Run documentation linting before committing:
```bash
make docs-check
```
This runs [`scripts/docs/lint_docs.py`](file:///home/gulshansharma/proof_battle/scripts/docs/lint_docs.py) to ensure:
- 0 broken relative markdown links.
- 0 missing file references.
- 0 invalid heading anchor targets.

### 3. Quarterly Review Checklist
- Verify that software prerequisites (Rust version, Node version, Lean version) in [`LOCAL_DEVELOPMENT.md`](LOCAL_DEVELOPMENT.md) match production.
- Benchmark and verify latency targets in [`BENCHMARKING_AND_LOAD.md`](BENCHMARKING_AND_LOAD.md).
- Review historical items in [`docs/archive/`](../archive/) to ensure active docs stay clean and focused.
