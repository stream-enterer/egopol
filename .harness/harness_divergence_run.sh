#!/usr/bin/env bash
# harness_divergence_run.sh — Run golden tests and collect divergence JSONL.
#
# The golden tests auto-write tol=0 divergence records to
# target/golden-divergence/divergence.jsonl (rotated on each run).
# This script just runs the tests and copies the result to OUTDIR.
#
# Usage: .harness/harness_divergence_run.sh OUTDIR

set -uo pipefail

OUTDIR="${1:?Usage: harness_divergence_run.sh OUTDIR}"
TIMEOUT="${HARNESS_TIMEOUT:-120}"

mkdir -p "$OUTDIR"

# ── Run golden tests ──────────────────────────────────────────────────────────

set +e
timeout "$TIMEOUT" \
  cargo test --test golden -- --test-threads=1 \
  > "$OUTDIR/stdout.log" 2> "$OUTDIR/stderr.log"
exit_code=$?
set -e

# ── Collect divergence log ────────────────────────────────────────────────────

DIVERGENCE_SRC="target/golden-divergence/divergence.jsonl"
if [ -f "$DIVERGENCE_SRC" ]; then
  cp "$DIVERGENCE_SRC" "$OUTDIR/divergence.jsonl"
fi

# ── Classify exit condition ───────────────────────────────────────────────────

if [ $exit_code -eq 0 ]; then
  echo '{"event":"CLEAN_EXIT","exit_code":0}' >> "$OUTDIR/divergence.jsonl"
elif [ $exit_code -eq 124 ]; then
  echo "{\"event\":\"HANG\",\"timeout_seconds\":$TIMEOUT}" >> "$OUTDIR/divergence.jsonl"
  echo "HANG: golden tests exceeded ${TIMEOUT}s timeout" >&2
elif [ $exit_code -gt 128 ]; then
  signal=$((exit_code - 128))
  echo "{\"event\":\"CRASH\",\"signal\":$signal,\"exit_code\":$exit_code}" >> "$OUTDIR/divergence.jsonl"
  echo "CRASH: golden tests killed by signal $signal" >&2
else
  echo "{\"event\":\"TEST_FAILURE\",\"exit_code\":$exit_code}" >> "$OUTDIR/divergence.jsonl"
fi

# ── Report ────────────────────────────────────────────────────────────────────

if [ -f "$OUTDIR/divergence.jsonl" ]; then
  test_count=$(grep -c '"test":' "$OUTDIR/divergence.jsonl" 2>/dev/null || echo 0)
  echo "Divergence run: $test_count tests reported (exit=$exit_code)"
else
  echo "WARNING: No divergence.jsonl produced" >&2
fi

exit $exit_code
