#!/usr/bin/env python3
"""Classify divergence JSONL into a structured run report.

Pattern: structured-output-specification, rich-feedback-loops
Requirements: V2 (exhaustive comparison), V3 (diagnostics), F2 (suspicious flagging),
              M7 (result classification), V10 (multiple output variables)

Reads divergence.jsonl and contract.json, produces classification.json.

Usage: python3 .harness/harness_classify.py DIVERGENCE_JSONL [CONTRACT_JSON]
  Output: JSON to stdout
"""

import json
import subprocess
import sys
from collections import defaultdict
from datetime import datetime

def main():
    if len(sys.argv) < 2:
        print("Usage: harness_classify.py DIVERGENCE_JSONL [CONTRACT_JSON]", file=sys.stderr)
        sys.exit(1)

    jsonl_path = sys.argv[1]
    contract_path = sys.argv[2] if len(sys.argv) > 2 else ".harness/contract.json"

    # Load contract
    with open(contract_path) as f:
        contract = json.load(f)

    # Build lookup: name → case
    case_by_name = {}
    for case in contract["cases"]:
        case_by_name[case["name"]] = case

    contract_names = set(case_by_name.keys())

    # Load JSONL
    test_records = []
    events = []
    with open(jsonl_path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            if "test" in rec:
                test_records.append(rec)
            elif "event" in rec:
                events.append(rec)

    # Classify each test
    reported_names = set()
    extra_names = []  # JSONL records not in contract (e.g., self-comparison tests)
    pass_count = 0
    fail_count = 0
    suspicious_cases = []

    for rec in test_records:
        name = rec["test"]
        reported_names.add(name)

        # Only count tests that are in the contract
        if name not in case_by_name:
            extra_names.append(name)
            continue

        case = case_by_name[name]
        tol = case.get("tolerance", {})
        ch_tol = tol.get("channel_tolerance", 0)
        max_fail_pct = tol.get("max_failure_pct", 0.0)

        # Pixel records have tol=0 metrics; determine pass from contract thresholds.
        # Non-pixel records (behavioral, notice, etc.) still have a "pass" field.
        if "pass" in rec:
            passed = rec["pass"]
        else:
            # Pixel test: recompute pass/fail from tol=0 metrics vs contract.
            # The JSONL "fail" count is at tol=0.  We need to check max_diff
            # against the contract's channel_tolerance.
            max_diff = rec.get("max_diff", 0)
            pct = rec.get("pct", 0.0)
            passed = max_diff <= ch_tol or pct <= max_fail_pct

        if passed:
            pass_count += 1

            # Check for suspicious (approaching thresholds)
            reasons = []
            pct = rec.get("pct", 0)
            max_diff = rec.get("max_diff", 0)

            if max_fail_pct and max_fail_pct > 0 and pct > max_fail_pct * 0.5:
                reasons.append(f"fail_pct={pct:.4f} approaching threshold {max_fail_pct}")

            if ch_tol and ch_tol > 0 and max_diff > ch_tol * 0.75:
                reasons.append(f"max_diff={max_diff} approaching ch_tol {ch_tol}")

            if reasons:
                suspicious_cases.append({
                    "test": name,
                    "reason": "; ".join(reasons),
                    "pct": pct,
                    "max_diff": max_diff,
                })
        else:
            fail_count += 1

    # Unreported cases
    unreported = sorted(contract_names - reported_names)

    # Category aggregation
    categories = {}
    for cat_name in contract["categories"]:
        cat_cases = [c for c in contract["cases"] if c["category"] == cat_name]
        cat_pass = sum(1 for c in cat_cases if c["name"] in reported_names
                       and any(r["test"] == c["name"] and r.get("pass") for r in test_records))
        cat_fail = sum(1 for c in cat_cases if c["name"] in reported_names
                       and any(r["test"] == c["name"] and not r.get("pass") for r in test_records))
        categories[cat_name] = {
            "total": len(cat_cases),
            "pass": cat_pass,
            "fail": cat_fail,
        }

    # Events
    has_crash = any(e.get("event") == "CRASH" for e in events)
    has_hang = any(e.get("event") == "HANG" for e in events)

    # Convergence (V9)
    converged = (fail_count == 0
                 and len(unreported) == 0
                 and not has_crash
                 and not has_hang)

    # Get commit
    try:
        commit = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            stderr=subprocess.DEVNULL
        ).decode().strip()
    except Exception:
        commit = "unknown"

    result = {
        "run_id": datetime.now().isoformat(),
        "commit": commit,
        "source_jsonl": jsonl_path,
        "contract_total": len(contract["cases"]),
        "reported": len(reported_names),
        "pass": pass_count,
        "fail": fail_count,
        "suspicious": len(suspicious_cases),
        "unreported": len(unreported),
        "unreported_cases": unreported[:50],  # cap at 50 for readability
        "extra": len(extra_names),
        "extra_cases": extra_names[:20],
        "suspicious_cases": suspicious_cases,
        "crash": has_crash,
        "hang": has_hang,
        "converged": converged,
        "categories": categories,
    }

    json.dump(result, sys.stdout, indent=2)
    print()


if __name__ == "__main__":
    main()
