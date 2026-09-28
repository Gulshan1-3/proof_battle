#!/usr/bin/env bash
set -euo pipefail

TIMEOUT_SECS="${1:-30}"
PROOF_FILE="${2:-/judge/proof.lean}"

if [ ! -f "$PROOF_FILE" ]; then
    echo "ERROR: Proof file '$PROOF_FILE' not found" >&2
    exit 2
fi

# Run lake env lean with hard timeout
exec timeout "$TIMEOUT_SECS" lake env lean "$PROOF_FILE"
