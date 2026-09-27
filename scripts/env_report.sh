#!/usr/bin/env bash
set -euo pipefail

# Ensure standard user binary paths are in PATH
export PATH="$HOME/.elan/bin:$HOME/.cargo/bin:$PATH"
export ELAN_DISABLE_UPDATE_CHECK=1

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -d "$REPO_ROOT/lean" ]; then
    LEAN_DIR="$REPO_ROOT/lean"
elif [ -d "$REPO_ROOT/battle" ]; then
    LEAN_DIR="$REPO_ROOT/battle"
else
    echo "ERROR: Cannot find Lean project root (checked lean/ and battle/)" >&2
    exit 1
fi

# 1. Verify lake is present
if ! command -v lake >/dev/null 2>&1; then
    echo "ERROR: 'lake' executable not found in PATH" >&2
    exit 1
fi

# 2. Verify lean is present
if ! command -v lean >/dev/null 2>&1; then
    echo "ERROR: 'lean' executable not found in PATH" >&2
    exit 1
fi

# 3. Check olean count
if [ -d "$LEAN_DIR/.lake/build" ]; then
    OLEAN_COUNT=$(find "$LEAN_DIR/.lake/build" -name '*.olean' | wc -l)
else
    OLEAN_COUNT=0
fi

if [ "$OLEAN_COUNT" -eq 0 ]; then
    echo "ERROR: Mathlib olean count is 0. Prebuilt cache missing!" >&2
    exit 1
fi

# 4. Gather toolchain versions
LEAN_TOOLCHAIN=$(cat "$LEAN_DIR/lean-toolchain" 2>/dev/null || echo "unknown")
LAKE_VERSION=$(cd "$LEAN_DIR" && lake --version 2>&1 | grep -v 'elan' | head -n 1)
LEAN_VERSION=$(cd "$LEAN_DIR" && lean --version 2>&1 | grep -v 'elan' | head -n 1)
RUSTC_VERSION=$(command -v rustc >/dev/null 2>&1 && rustc --version || echo "not found")
CARGO_VERSION=$(command -v cargo >/dev/null 2>&1 && cargo --version || echo "not found")
NODE_VERSION=$(command -v node >/dev/null 2>&1 && node --version || echo "not found")
NPM_VERSION=$(command -v npm >/dev/null 2>&1 && npm --version || echo "not found")

# 5. Mathlib sources count
if [ -d "$LEAN_DIR/Mathlib" ]; then
    MATHLIB_LEAN_COUNT=$(find "$LEAN_DIR/Mathlib" -name '*.lean' | wc -l)
else
    MATHLIB_LEAN_COUNT=0
fi

# 6. Docker availability
if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    DOCKER_STATUS="available ($(docker --version | tr -d '\n'))"
else
    DOCKER_STATUS="unavailable"
fi

# 7. Postgres availability
if command -v pg_isready >/dev/null 2>&1; then
    if pg_isready >/dev/null 2>&1; then
        POSTGRES_STATUS="running"
    else
        POSTGRES_STATUS="server not responding (client present: $(pg_isready --version 2>&1 | head -n 1))"
    fi
else
    POSTGRES_STATUS="client not found"
fi

# 8. Import smoke test
START_TIME=$(date +%s%N)
if (cd "$LEAN_DIR" && echo 'import Mathlib.Data.Nat.Basic' | lake env lean --stdin >/dev/null 2>&1); then
    END_TIME=$(date +%s%N)
    DURATION_MS=$(( (END_TIME - START_TIME) / 1000000 ))
    IMPORT_STATUS="pass (${DURATION_MS} ms)"
else
    IMPORT_STATUS="fail"
fi

# Output table as key=value lines
echo "LEAN_TOOLCHAIN=$LEAN_TOOLCHAIN"
echo "LAKE_VERSION=$LAKE_VERSION"
echo "LEAN_VERSION=$LEAN_VERSION"
echo "RUSTC_VERSION=$RUSTC_VERSION"
echo "CARGO_VERSION=$CARGO_VERSION"
echo "NODE_VERSION=$NODE_VERSION"
echo "NPM_VERSION=$NPM_VERSION"
echo "DOCKER_STATUS=$DOCKER_STATUS"
echo "POSTGRES_STATUS=$POSTGRES_STATUS"
echo "MATHLIB_OLEAN_COUNT=$OLEAN_COUNT"
echo "MATHLIB_LEAN_SOURCE_COUNT=$MATHLIB_LEAN_COUNT"
echo "IMPORT_SMOKE_TEST=$IMPORT_STATUS"

exit 0
