#!/usr/bin/env bash
# Summarize all unresolved findings from the audit.
# Reads from actual sources every time — no persistent state.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE="$(cd "$SCRIPT_DIR/../.." && pwd)"
RUN_LOG="$SCRIPT_DIR/run-log.md"

echo "=== Open Items Summary ==="
echo ""

# 1. BLOCKED items from run-log tables
blocked=$(grep -n '| BLOCKED |' "$RUN_LOG" 2>/dev/null || true)
if [ -n "$blocked" ]; then
    count=$(echo "$blocked" | wc -l)
    echo "--- BLOCKED ($count) ---"
    echo "$blocked" | while IFS= read -r line; do
        lineno="${line%%:*}"
        content="${line#*:}"
        # Extract the item ID (second column) and description
        echo "  run-log.md:$lineno $content"
    done
    echo ""
fi

# 2. DEFERRED items from run-log tables and Session 4 triage
deferred=$(grep -n 'DEFERRED' "$RUN_LOG" 2>/dev/null | grep -v '^[0-9]*:#' || true)
if [ -n "$deferred" ]; then
    count=$(echo "$deferred" | wc -l)
    echo "--- DEFERRED ($count) ---"
    echo "$deferred" | while IFS= read -r line; do
        lineno="${line%%:*}"
        content="${line#*:}"
        echo "  run-log.md:$lineno $content"
    done
    echo ""
fi

# 3. #[ignore] tests in source and test files
echo "--- #[ignore] tests ---"
ignores=$(grep -rn '#\[ignore' "$WORKSPACE/src/" "$WORKSPACE/tests/" 2>/dev/null || true)
if [ -n "$ignores" ]; then
    count=$(echo "$ignores" | wc -l)
    echo "  ($count total)"
    echo "$ignores" | while IFS= read -r line; do
        # Strip workspace prefix for readability
        echo "  ${line#$WORKSPACE/}"
    done
else
    echo "  (none)"
fi
echo ""

# 4. Golden test tolerances above baseline (ch_tol > 1 or max_fail_pct > 5.0)
echo "--- Relaxed golden tolerances ---"
tolerances=$(grep -rn 'max_fail_pct\|ch_tol' "$WORKSPACE/tests/golden/" 2>/dev/null | grep -v 'fn \|//\|common.rs' || true)
if [ -n "$tolerances" ]; then
    echo "$tolerances" | while IFS= read -r line; do
        echo "  ${line#$WORKSPACE/}"
    done
else
    echo "  (none)"
fi
echo ""

# 5. SUSPECT items from run-log
suspects=$(grep -nc 'SUSPECT' "$RUN_LOG" 2>/dev/null || echo "0")
echo "--- SUSPECTs in run log: $suspects mentions ---"
echo ""

# 6. Result files with unresolved items
echo "--- Per-widget result files ---"
for f in "$SCRIPT_DIR"/results/*.md; do
    [ -f "$f" ] || continue
    basename="${f##*/}"
    echo "  $basename"
done
echo ""

echo "=== End ==="
