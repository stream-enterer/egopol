# Phase 3a: Verify Golden Coverage for Untested Capabilities

Single-purpose task. No state machine. No phase transitions. Read, check, log.

## Input

`state/run_003/capability_map.json` — 41 capabilities have empty `golden_tests` arrays.

## Task

For each of the 41 capabilities without golden tests, determine which category it falls into and record the result. Do not implement anything. Do not write gen functions. Do not modify source code. Only read and classify.

### Category A: Stdlib Replacement

The C++ type has no Rust port because Rust's stdlib covers it (Vec, String, Rc, BTreeMap, etc.). No golden test is possible or needed.

**Verification:** Confirm the Rust codebase uses the stdlib replacement by running `grep -rn '<stdlib_type>' src/ | head -5`. Record the grep output as evidence.

### Category B: Platform / Application Layer

The C++ type is Eagle Mode application infrastructure (config UI, plugin system, IPC, installation paths, threading) that zuicchini either doesn't need or replaces with a different architecture. No golden test is feasible.

**Verification:** Confirm the C++ header is application-layer by reading it. Record which emCore header it comes from and why it's not toolkit-level.

### Category C: Covered by Existing Golden Tests

The capability IS exercised by existing golden tests, just not mapped in the capability_map. The V3 run marked some of these "verified" but the evidence was weak or absent.

**Verification:** For each candidate, search `tests/golden/*.rs` for actual usage of the capability's Rust symbols. Run `grep -rn '<rust_symbol>' tests/golden/` and record:
- Which test file(s) reference it
- Which test function(s) call it
- Whether those test functions load golden data files (look for `load_golden`, `compare_images`, `compare_rects`, etc.)

If grep finds real test functions that load golden data and exercise the capability's Rust code: **COVERED**. Record the test function names as evidence.

If grep finds only incidental mentions (imports, type annotations, comments): **NOT COVERED**.

### Category D: Needs Golden Extension

The capability has a Rust implementation, is renderable or observable, and no existing golden test exercises it. These are candidates for future golden gen extension.

**Verification:** Confirm the Rust implementation exists by reading the `rust_target.file` from the capability map. Confirm it has non-trivial methods (not stubs). Record the file path and a one-line description of what a golden test would exercise.

## Output

Write results to `state/run_003/golden_coverage_audit.json`:

```json
{
  "audited_at": "<ISO8601>",
  "results": [
    {
      "capability_id": "CAP-NNNN",
      "capability_name": "<string>",
      "category": "A|B|C|D",
      "evidence": "<string: grep output, file path, or explanation>",
      "covered_by_tests": ["<test function names if category C>"],
      "extension_needed": "<string: what a golden test would exercise, if category D>"
    }
  ],
  "summary": {
    "category_a_stdlib": 0,
    "category_b_platform": 0,
    "category_c_covered": 0,
    "category_d_needs_extension": 0
  }
}
```

## Rules

- Run `grep` for every classification. Do not classify by name alone.
- If a capability was marked "verified" in the V3 run but grep finds no test evidence: classify as D, not C.
- Do not write code. Do not modify files other than `golden_coverage_audit.json`.
- Do not skip capabilities. All 41 must appear in the output.
