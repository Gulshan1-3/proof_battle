# Local Development & Testing Guide

## Quick Start

```bash
# 1. Setup environment
cp .env.example .env

# 2. Start database (PostgreSQL)
make db-up

# 3. Run database migrations
make migrate

# 4. Seed problems (optional)
make seed

# 5. Run server
make run-server
```

---

## Detailed Setup

### 1. Prerequisites
- Rust 1.90.0+
- Docker & Docker Compose
- PostgreSQL (or use Docker)
- Lean 4 (for verification testing)

### 2. Environment Configuration
```bash
# Copy environment template
cp .env.example .env

# Edit if needed (defaults work for local dev)
nano .env
```

### 3. Database Setup
```bash
# Start PostgreSQL via Docker
make db-up

# Check database is running
docker ps

# Run migrations
make migrate

# Seed with sample problems (optional)
make seed

# Reset database (clear all data)
make db-reset

# Stop database
make db-down
```

### 4. Lean Verification Setup
```bash
# Verify Lean toolchain is working
cd lean
lake --version

# Test basic Lean verification
bash scripts/verify_judge.sh

# Verify environment baseline
make env-report
```

### 5. Running the Server
```bash
# Run with default settings
make run-server

# Or directly with cargo
cd server
cargo run

# Server will start on http://localhost:3000
# WebSocket endpoint: ws://localhost:3000/ws
```

### 6. Running Tests
```bash
# Run all tests
make test

# Run specific test
cd server
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration_test
```

### 7. Code Quality Checks
```bash
# Format code
make fmt

# Check formatting without modifying
cargo fmt --check --manifest-path server/Cargo.toml

# Run Clippy linter
make clippy

# Run Clippy with specific checks
cargo clippy --manifest-path server/Cargo.toml -- -D warnings -W clippy::all
```

### 8. Protocol Code Generation
```bash
# Generate TypeScript bindings from Rust types
make codegen

# This updates frontend/src/lib/ws/generated.ts
```

---

## Testing Specific Components

### Test WebSocket Connection
```bash
# Install websocat for testing
cargo install websocat

# Connect to WebSocket
websocat ws://localhost:3000/ws

# Send test message
{"type":"JoinQueue","player_name":"test"}
```

### Test Lean Verification Directly
```bash
# Test a simple proof
cd lean
cat > test_proof.lean << 'EOF'
import Mathlib.Data.Nat.Basic
theorem goal (n : Nat) : n + 0 = n := by
  rfl
EOF

lake env lean test_proof.lean
```

### Test Database Connection
```bash
# Connect to PostgreSQL
docker exec -it proofbattle-postgres-1 psql -U proofbattle -d proofbattle

# List tables
\dt

# Query problems
SELECT * FROM problems LIMIT 5;

# Exit
\q
```

---

## Development Workflow

### Typical Development Cycle
```bash
# 1. Make changes
vim server/src/main.rs

# 2. Format code
make fmt

# 3. Run linter
make clippy

# 4. Run tests
make test

# 5. If protocol changed, regenerate bindings
make codegen

# 6. Run server
make run-server

# 7. Test in browser or with websocat
```

### Adding New Problems
```bash
# 1. Add problem to database
psql postgres://proofbattle:proofbattle_dev@localhost:5432/proofbattle

INSERT INTO problems (goal, difficulty, category, imports) VALUES
  ('∀ n : ℕ, n + 0 = n', 1, 'nat_arithmetic', ARRAY['Mathlib.Data.Nat.Basic']);

# 2. Verify problem works with Lean
cd lean
lake env lean -e 'import Mathlib.Data.Nat.Basic'

# 3. Test verification
make seed
```

---

## Troubleshooting

### Database Connection Issues
```bash
# Check if PostgreSQL is running
docker ps | grep postgres

# View logs
docker logs proofbattle-postgres-1

# Restart database
make db-down
make db-up
```

### Lean Verification Issues
```bash
# Check Lean installation
cd lean
lake --version

# Rebuild Mathlib if needed
lake build

# Check olean cache
find .lake/build -name "*.olean" | wc -l
```

### Port Already in Use
```bash
# Check what's using port 3000
lsof -i :3000

# Kill process
kill -9 <PID>

# Or change port in .env
PORT=3001 make run-server
```

### Rust Build Issues
```bash
# Clean build artifacts
cd server
cargo clean

# Update dependencies
cargo update

# Check Rust version
rustc --version
```

---

## Performance Testing

### Load Test WebSocket
```bash
# Install artillery for load testing
npm install -g artillery

# Create test config
cat > ws-load-test.yml << 'EOF'
config:
  target: ws://localhost:3000/ws
  phases:
    - duration: 60
      arrivalRate: 10
scenarios:
  - flow:
    - send: {"type":"JoinQueue","player_name":"test"}
EOF

# Run load test
artillery run ws-load-test.yml
```

### Benchmark Lean Verification
```bash
# Time verification of simple proof
time lake env lean test_proof.lean

# Batch test multiple proofs
for i in {1..100}; do
  lake env lean test_proof.lean
done
```

---

## Monitoring & Debugging

### View Server Logs
```bash
# Run with debug logging
RUST_LOG=debug make run-server

# Or set in .env
RUST_LOG=debug,proof_battle=trace
```

### Check Database State
```bash
# Connect to database
docker exec -it proofbattle-postgres-1 psql -U proofbattle -d proofbattle

# View active connections
SELECT * FROM pg_stat_activity;

# View table sizes
SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) FROM pg_tables WHERE schemaname = 'public';
```

### Monitor Lean Workers
```bash
# Check Lean processes
ps aux | grep lean

# Check Docker containers (if using sandbox)
docker ps | grep lean
```

---

## Clean Up

```bash
# Stop database
make db-down

# Remove all Docker volumes (clears database data)
make db-reset

# Clean Rust build artifacts
cd server
cargo clean

# Clean Lean build cache
cd lean
lake clean

# Remove generated files
rm -f frontend/src/lib/ws/generated.ts
```

---

## Useful Commands Reference

```bash
# Make commands
make bootstrap          # Check Rust compilation
make run-server         # Start server
make test              # Run tests
make fmt               # Format code
make clippy            # Run linter
make codegen           # Generate TypeScript bindings
make db-up             # Start database
make db-down           # Stop database
make db-reset          # Reset database
make migrate           # Run migrations
make seed              # Seed problems
make env-report        # Environment baseline report
make verify-harness    # Verify Lean judge

# Direct cargo commands
cd server
cargo build            # Build project
cargo run              # Run project
cargo test             # Run tests
cargo clippy           # Run linter
cargo fmt              # Format code
cargo clean            # Clean build artifacts

# Docker commands
docker ps              # List containers
docker logs <container> # View logs
docker exec -it <container> bash # Enter container
docker compose -f infra/docker-compose.yml up -d # Start infra
docker compose -f infra/docker-compose.yml down    # Stop infra
```
