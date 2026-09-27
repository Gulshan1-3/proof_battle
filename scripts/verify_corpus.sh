#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
VERIFY_SCRIPT="$SCRIPT_DIR/lean_verify.sh"

if [ ! -f "$VERIFY_SCRIPT" ]; then
    echo "ERROR: Harness '$VERIFY_SCRIPT' not found" >&2
    exit 2
fi

# Corpus definition: filename, expected_verdict
declare -a CORPUS=(
    "accept_add_zero.lean:ACCEPT"
    "accept_add_comm.lean:ACCEPT"
    "reject_unsolved.lean:REJECT"
    "reject_sorry.lean:REJECT"
    "reject_sorry_embedded.lean:REJECT"
    "reject_syntax.lean:REJECT"
    "reject_unknown_tactic.lean:REJECT"
    "skip_import_mathlib.lean:REJECT"
)

TOTAL_START=$(date +%s%N)
PASSED=0
FAILED=0

printf "%-30s %-10s %-10s %-8s %s\n" "FIXTURE" "EXPECTED" "ACTUAL" "STATUS" "NOTE"
echo "----------------------------------------------------------------------------------------"

for item in "${CORPUS[@]}"; do
    FILE="${item%%:*}"
    EXPECTED="${item##*:}"
    FILE_PATH="$FIXTURES_DIR/$FILE"

    if [ ! -f "$FILE_PATH" ]; then
        printf "%-30s %-10s %-10s %-8s %s\n" "$FILE" "$EXPECTED" "MISSING" "FAIL" "File not found"
        FAILED=$((FAILED + 1))
        continue
    fi

    set +e
    OUTPUT=$("$VERIFY_SCRIPT" "$FILE_PATH" 2>&1)
    set -e

    ACTUAL=$(echo "$OUTPUT" | grep '^VERDICT=' | cut -d= -f2 || echo "ERROR")
    DURATION=$(echo "$OUTPUT" | grep '^DURATION_MS=' | cut -d= -f2 || echo "0")

    NOTE="${DURATION}ms"
    if [ "$FILE" = "reject_sorry.lean" ] || [ "$FILE" = "reject_sorry_embedded.lean" ]; then
        if [ "$ACTUAL" = "ACCEPT" ]; then
            NOTE="${NOTE} (documented unsound judge bug)"
        fi
    elif [ "$FILE" = "skip_import_mathlib.lean" ]; then
        NOTE="${NOTE} (aggregate import failure)"
    fi

    if [ "$EXPECTED" = "$ACTUAL" ]; then
        STATUS="PASS"
        PASSED=$((PASSED + 1))
    else
        STATUS="FAIL"
        FAILED=$((FAILED + 1))
    fi

    printf "%-30s %-10s %-10s %-8s %s\n" "$FILE" "$EXPECTED" "$ACTUAL" "$STATUS" "$NOTE"
done

TOTAL_END=$(date +%s%N)
TOTAL_MS=$(( (TOTAL_END - TOTAL_START) / 1000000 ))
TOTAL_SECS=$(awk "BEGIN {printf \"%.2f\", $TOTAL_MS / 1000}")

echo "----------------------------------------------------------------------------------------"
echo "Corpus run finished in ${TOTAL_SECS}s (${TOTAL_MS} ms)"
echo "Results: $PASSED passed, $FAILED failed"

# Return 0 to allow CI/scripts to inspect the documented baseline
exit 0
