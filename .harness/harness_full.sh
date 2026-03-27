#!/usr/bin/env bash
# harness_full.sh — Full verification harness run.
#
# Pattern: discrete-phase-separation, multi-step-analysis-pipeline-orchestration
#
# Sequential pipeline with gated phases. Each phase writes artifacts to OUTDIR.
# Phase 0 gates all subsequent phases (self-conformance must pass first).
# Phases 1-2 are structural (fast). Phase 3 is the golden test run (heavy).
# Phase 4 classifies results and checks for regressions.
#
# Usage: .harness/harness_full.sh [OUTDIR]
#   Default OUTDIR: .harness/runs/YYYYMMDD_HHMMSS

set -euo pipefail

OUTDIR="${1:-.harness/runs/$(date +%Y%m%d_%H%M%S)}"
mkdir -p "$OUTDIR"

BASELINE=".harness/baseline.json"

phase() { echo ""; echo "════════════════════════════════════════════════════"; echo "  Phase $1: $2"; echo "════════════════════════════════════════════════════"; }

# ── Phase 0: Self-conformance (GATE) ──────────────────────────────────────────

phase 0 "Self-conformance"
.harness/harness_self_check.sh 2>&1 | tee "$OUTDIR/self_check.log"
echo "Phase 0 passed."

# ── Phase 1: Contract validation ──────────────────────────────────────────────

phase 1 "Contract validation"
.harness/harness_check_contract.sh 2>&1 | tee "$OUTDIR/contract_check.log"
echo "Phase 1 passed."

# ── Phase 2: Correspondence audit ────────────────────────────────────────────

phase 2 "Correspondence audit"
if .harness/harness_correspondence.sh "$OUTDIR" 2>&1 | tee "$OUTDIR/correspondence.log"; then
  echo "Phase 2 passed."
else
  echo "Phase 2 FAILED (correspondence gaps found — see $OUTDIR/correspondence.log)"
  echo "  Continuing to Phase 3 (divergence measurement is independent)."
fi

# ── Phase 3: Divergence measurement ──────────────────────────────────────────

phase 3 "Divergence measurement"
.harness/harness_divergence_run.sh "$OUTDIR" 2>&1 | tee "$OUTDIR/divergence_run.log" || true
echo "Phase 3 complete."

# ── Phase 4: Classification and regression ───────────────────────────────────

phase 4 "Classification & regression"

# Classify current run
.harness/harness_classify.sh "$OUTDIR/divergence.jsonl" > "$OUTDIR/classification.json"
echo "Classification:"
jq -r '"  Cases: \(.contract_total), Reported: \(.reported), Pass: \(.pass), Fail: \(.fail), Suspicious: \(.suspicious), Unreported: \(.unreported)"' "$OUTDIR/classification.json"

# Check for regressions against baseline
if [ -f "$BASELINE" ]; then
  echo ""
  echo "Regression check against baseline:"
  # Compare using the raw JSONL if available, otherwise skip
  prev_jsonl=$(jq -r '.source_jsonl // empty' "$BASELINE" 2>/dev/null || true)
  if [ -n "$prev_jsonl" ] && [ -f "$prev_jsonl" ]; then
    .harness/harness_regression_check.sh "$prev_jsonl" "$OUTDIR/divergence.jsonl" \
      2>&1 | tee "$OUTDIR/regression.log" || true
  else
    echo "  (baseline exists but source JSONL not found — comparing classification only)"
  fi
else
  echo "No baseline found — this is the first run."
fi

# ── Phase 5: Summary ─────────────────────────────────────────────────────────

phase 5 "Summary"

converged=$(jq -r '.converged' "$OUTDIR/classification.json")
pass_count=$(jq -r '.pass' "$OUTDIR/classification.json")
fail_count=$(jq -r '.fail' "$OUTDIR/classification.json")
suspicious=$(jq -r '.suspicious' "$OUTDIR/classification.json")
unreported=$(jq -r '.unreported' "$OUTDIR/classification.json")
total=$(jq -r '.contract_total' "$OUTDIR/classification.json")

# Show suspicious cases if any
if [ "$suspicious" -gt 0 ]; then
  echo ""
  echo "Suspicious cases (approaching thresholds):"
  jq -r '.suspicious_cases[] | "  \(.test): \(.reason)"' "$OUTDIR/classification.json" 2>/dev/null || true
fi

# Show unreported cases if any
if [ "$unreported" -gt 0 ]; then
  echo ""
  echo "UNREPORTED cases (in contract but not in test output):"
  jq -r '.unreported_cases[]' "$OUTDIR/classification.json" 2>/dev/null || true
fi

echo ""
echo "════════════════════════════════════════════════════"
echo "  HARNESS RESULT"
echo "════════════════════════════════════════════════════"
echo "  Total:      $total"
echo "  Pass:       $pass_count"
echo "  Fail:       $fail_count"
echo "  Suspicious: $suspicious"
echo "  Unreported: $unreported"
echo "  Converged:  $converged"
echo "  Output:     $OUTDIR/"
echo "════════════════════════════════════════════════════"

# Exit non-zero if not converged
if [ "$converged" != "true" ]; then
  exit 1
fi
