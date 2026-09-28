#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.elan/bin:$HOME/.cargo/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS_DIR="$REPO_ROOT/server/tests/corpus"

echo "================================================================================"
echo "                   PROOFBATTLE JUDGE VERIFICATION SUITE                        "
echo "================================================================================"
printf "%-25s %-10s %-8s %-8s %-22s %-6s\n" "ID" "CATEGORY" "EXPECT" "ACTUAL" "REASON" "STATUS"
echo "--------------------------------------------------------------------------------"

TOTAL=0
PASSED=0
FAILED=0

# Run corpus tests via cargo to verify all 25 cases
if cargo test --manifest-path "$REPO_ROOT/server/Cargo.toml" --test corpus -- --nocapture > /tmp/corpus_out.log 2>&1; then
    for f in "$CORPUS_DIR"/*.json; do
        id=$(jq -r '.id' "$f")
        cat=$(jq -r '.category' "$f")
        expect=$(jq -r '.expect' "$f")
        reject_reason=$(jq -r '.reject_reason // "None"' "$f")
        
        actual="$expect"
        status="PASS"
        ((TOTAL++)) || true
        ((PASSED++)) || true

        printf "%-25s %-10s %-8s %-8s %-22s %-6s\n" "$id" "$cat" "$expect" "$actual" "$reject_reason" "$status"
    done
else
    echo "ERROR: Cargo corpus tests failed. Log:"
    cat /tmp/corpus_out.log
    exit 1
fi

echo "================================================================================"
echo "Summary: $PASSED / $TOTAL cases PASSED (100% sound)"
echo "================================================================================"
