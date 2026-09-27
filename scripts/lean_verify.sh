#!/usr/bin/env bash
set -euo pipefail

# Ensure standard user binary paths are in PATH
export PATH="$HOME/.elan/bin:$HOME/.cargo/bin:$PATH"
export ELAN_DISABLE_UPDATE_CHECK=1

# Check arguments
if [ $# -lt 1 ]; then
    echo "Usage: $0 <file.lean>" >&2
    exit 2
fi

LEAN_FILE="$1"
if [ ! -f "$LEAN_FILE" ]; then
    echo "ERROR: File '$LEAN_FILE' does not exist" >&2
    exit 2
fi

# Make LEAN_FILE path absolute
ABS_LEAN_FILE="$(cd "$(dirname "$LEAN_FILE")" && pwd)/$(basename "$LEAN_FILE")"

# Locate Lean project directory ($PB_LEAN_DIR, fallback to ../lean then ../battle)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -n "${PB_LEAN_DIR:-}" ] && [ -d "$PB_LEAN_DIR" ]; then
    LEAN_DIR="$PB_LEAN_DIR"
elif [ -d "$SCRIPT_DIR/../lean" ]; then
    LEAN_DIR="$SCRIPT_DIR/../lean"
elif [ -d "$SCRIPT_DIR/../battle" ]; then
    LEAN_DIR="$SCRIPT_DIR/../battle"
else
    echo "ERROR: Cannot find Lean project root" >&2
    exit 2
fi

if ! command -v lake >/dev/null 2>&1; then
    echo "ERROR: 'lake' executable not found in PATH" >&2
    exit 2
fi

# Prepare temporary captures
TMP_OUT="$(mktemp)"
TMP_ERR="$(mktemp)"
trap 'rm -f "$TMP_OUT" "$TMP_ERR"' EXIT

# Execute with 30s timeout
START_NS=$(date +%s%N)
set +e
(cd "$LEAN_DIR" && timeout 30 lake env lean "$ABS_LEAN_FILE" > "$TMP_OUT" 2> "$TMP_ERR")
RAW_EXIT=$?
set -e
END_NS=$(date +%s%N)

DURATION_MS=$(( (END_NS - START_NS) / 1000000 ))
STDOUT_BYTES=$(wc -c < "$TMP_OUT" | tr -d ' ')
STDERR_BYTES=$(wc -c < "$TMP_ERR" | tr -d ' ')

if [ "$RAW_EXIT" -eq 124 ]; then
    TIMED_OUT=true
    EXIT_CODE=124
    FAILED_TO_COMPILE=false
else
    TIMED_OUT=false
    EXIT_CODE=$RAW_EXIT
    if [ "$EXIT_CODE" -ne 0 ]; then
        FAILED_TO_COMPILE=true
    else
        FAILED_TO_COMPILE=false
    fi
fi

# Search combined output (stdout and stderr) for sorry warnings
if grep -E "(declaration uses 'sorry'|sorryAx)" "$TMP_OUT" "$TMP_ERR" >/dev/null 2>&1; then
    USES_SORRY=true
else
    USES_SORRY=false
fi

# Current prototype verdict logic: process exit code 0 == ACCEPT
if [ "$TIMED_OUT" = "true" ]; then
    VERDICT="REJECT"
    FINAL_EXIT=1
elif [ "$EXIT_CODE" -eq 0 ]; then
    VERDICT="ACCEPT"
    FINAL_EXIT=0
else
    VERDICT="REJECT"
    FINAL_EXIT=1
fi

# Emit EXACTLY the required lines in exact order
echo "EXIT_CODE=$EXIT_CODE"
echo "DURATION_MS=$DURATION_MS"
echo "STDOUT_BYTES=$STDOUT_BYTES"
echo "STDERR_BYTES=$STDERR_BYTES"
echo "USES_SORRY=$USES_SORRY"
echo "FAILED_TO_COMPILE=$FAILED_TO_COMPILE"
echo "TIMED_OUT=$TIMED_OUT"
echo "VERDICT=$VERDICT"

exit "$FINAL_EXIT"
