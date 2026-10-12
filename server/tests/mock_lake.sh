#!/usr/bin/env bash
# Mock Lake/Lean runner for CI and lightweight environments

if [ "$1" = "env" ] && [ "$2" = "lean" ]; then
    FILE="$3"
    if [ -n "$FILE" ] && [ -f "$FILE" ]; then
        CONTENT=$(cat "$FILE")

        if echo "$CONTENT" | grep -q "NonexistentInvalidModule"; then
            echo "error: unknown module Mathlib.NonexistentInvalidModule" >&2
            exit 1
        elif echo "$CONTENT" | grep -q -E "sorryAx|declaration uses 'sorry'|\<sorry\>"; then
            echo "error: declaration uses 'sorry'" >&2
            exit 1
        elif echo "$CONTENT" | grep -q "exact 42"; then
            echo "error: type mismatch in 'exact 42'" >&2
            exit 1
        elif echo "$CONTENT" | grep -q "decide"; then
            echo "error: tactic 'decide' failed" >&2
            exit 1
        elif echo "$CONTENT" | grep -q "loop_forever"; then
            sleep 10
            exit 1
        else
            # Valid proof verification
            exit 0
        fi
    fi
    exit 0
fi

# Fallback for version or help checks
echo "Lake version 4.21.0-rc3"
exit 0
