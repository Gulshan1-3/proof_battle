# Runbook: Local Development Environment Setup

This runbook guides engineers through configuring and running the full ProofBattle stack locally on Linux or macOS.

---

## 1. Prerequisites

Ensure the following tools are installed:
- **Rust Toolchain**: `rustup default stable` (1.90.0+)
- **Node.js**: v20+ with `npm`
- **Docker & Docker Compose**: For local PostgreSQL database container
- **Python**: 3.10+ with `torch` (for ML simulation services)
- **Lean 4 Toolchain**: `elan` with Lean 4.11+ / Lake

---

## 2. Step-by-Step Environment Bootstrap

### Step 1: Clone & Configure Environment Variables
```bash
cp .env.example .env
```
Default values work out-of-the-box for local development.

### Step 2: Start PostgreSQL Database Container
```bash
make db-up
```
Verify the database is healthy:
```bash
docker ps --filter "name=proofbattle-db"
```

### Step 3: Run Database Migrations
```bash
make migrate
```
This initializes the `users`, `ratings`, `problems`, and `matches` tables.

### Step 4: Seed Challenge Problem Catalog
```bash
make seed
```
This populates the database with 120+ verified Lean 4 challenges and canonical solutions.

---

## 3. Running Services

ProofBattle consists of three complementary services. Run them in separate terminal tabs or as background jobs:

### Terminal 1: Rust Game Server (Port 8080)
```bash
make run-server
```
- Listens on `http://0.0.0.0:8080`
- WebSocket endpoint: `ws://localhost:8080/ws`

### Terminal 2: SvelteKit Frontend (Port 5173)
```bash
npm run dev --prefix frontend -- --host 0.0.0.0 --port 5173
```
- Open `http://localhost:5173` in your browser.

### Terminal 3: AI Prover Simulation Server (Port 8088, Optional)
```bash
python3 scripts/ml/simulation_server.py --port 8088
```
- Provides solo practice bot opponents and real-time proof assistance.

---

## 4. Useful Quality & Testing Commands

| Command | Action |
| :--- | :--- |
| `make fmt` | Check Rust code formatting (`cargo fmt --check`). |
| `make clippy` | Run Rust linter with warnings as errors (`cargo clippy -- -D warnings`). |
| `make test` | Run full backend test suite (`cargo test`). |
| `npm run check --prefix frontend` | Run SvelteKit typecheck. |
| `npm run lint --prefix frontend` | Run ESLint and Prettier checks. |
| `make docs-check` | Verify internal documentation links and references. |
| `make db-reset` | Wipe database volume and re-run migrations from scratch. |
