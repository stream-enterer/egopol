# Harness V3: Behavioral Equivalence for a C++→Rust Port

## 0. Agent Execution Preamble

**You are an orchestration agent. You execute phases, not plan them.**

Read `state/system_state.json`. Determine which phase and substate you are in. Execute the next step for that state. Do not summarize the harness. Do not plan what you will do. Do not enter Plan Mode. Do not reason about the harness as a document. The harness IS your instructions. Execute them.

**Mandatory behavioral rules:**

1. When the harness says "MUST", you MUST. There are zero optional steps. If a step says "run `cargo test`", you run `cargo test` and report the literal output. You do not assess whether cargo test would pass.
2. Every phase has a gate condition. You do not advance past a gate until the gate predicate is true. If the predicate is false, you either fix the issue or HALT.
3. When implementing code, you MUST run the relevant test afterward and record the actual output. Self-assessment ("I believe this is correct") is never verification.
4. When the harness says "record", you write to the specified file in the specified format. Not to your context. To the file.
5. Reclassification (changing a capability's status to `not_applicable` or `out_of_boundary`) requires structured evidence in the capability_map entry. The evidence format is specified in Schema 3.2. A reclassification without evidence is a HALT-worthy integrity violation.
6. The Confidence Audit (Phase 1c) is mandatory. It is not skippable. It runs even if all confidence scores are 1.0. Especially if all confidence scores are 1.0.

**Tool use rules:**

- To verify code compiles: run `cargo check --workspace 2>&1`. Read the output.
- To verify tests pass: run `cargo test --workspace 2>&1`. Read the output.
- To run golden tests with measurement: run `MEASURE_DIVERGENCE=1 cargo test --test golden 2>&1`. Read the output.
- To run a specific golden test: run `cargo test --test golden <test_name> 2>&1`. Read the output.
- To build the golden generator: run `make -C tests/golden/gen 2>&1`. Read the output.
- To run the golden generator: run `make -C tests/golden/gen run 2>&1`. Read the output.
- After writing any Rust code: run `cargo check --workspace 2>&1` before proceeding.
- After writing any C++ code in gen_golden.cpp: run `make -C tests/golden/gen 2>&1` before proceeding.
- Use `Edit` for modifying existing files. Use `Write` for creating new files. Do not use heredocs.

---

## 1. System Overview

C++ source: Eagle Mode emCore (90 headers, 87 source files) at `~/.local/git/eaglemode-0.96.4/`.
Rust target: zuicchini (project root).

**Phases:**

1. Map every emCore capability to an existing golden test or identify the gap.
2. Run the full golden test suite. Measure per-test divergence from C++ reference.
3. Extend the golden data generator for untested capabilities.
4. Fix divergences by reading C++ source and porting algorithms.
5. Implement remaining unported features.
6. Document every intentional stub with justification.

**Prerequisites:** zuicchini builds (`cargo check` passes), Eagle Mode 0.96.4 is installed with compiled `libemCore.so` and `libemTestPanel.so`, git is initialized.

---

## 2. Filesystem Layout

```
zuicchini/                              # Project root (Rust crate)
├── src/                                # Rust source tree (read-write)
├── tests/
│   ├── golden/
│   │   ├── main.rs                     # Golden test binary entry
│   │   ├── common.rs                   # Loaders, comparators, MEASURE_DIVERGENCE
│   │   ├── *.rs                        # Per-domain test modules
│   │   ├── data/                       # 178+ binary golden reference files
│   │   │   ├── painter/                # RGBA pixel buffers (256x256)
│   │   │   ├── layout/                 # Child rect coordinates (4xf64)
│   │   │   ├── behavioral/             # Panel active/path state
│   │   │   ├── notice/                 # C++ notice flag bits
│   │   │   ├── input/                  # Input routing state
│   │   │   ├── compositor/             # RGBA pixel buffers (800x600)
│   │   │   ├── trajectory/             # Animation velocity (3xf64/step)
│   │   │   └── widget_state/           # Per-widget state (varies)
│   │   ├── gen/
│   │   │   ├── gen_golden.cpp          # C++ golden data generator
│   │   │   ├── golden_format.h         # Binary format helpers
│   │   │   └── Makefile                # Build: make, Run: make run
│   │   ├── debug/                      # PPM diff images (.gitignored)
│   │   └── assets/                     # Test assets (teddy.tga)
│   ├── behavioral/                     # API behavioral assertions (91 tests)
│   ├── integration/                    # Integration tests (21 tests)
│   ├── unit/                           # Component unit tests (57 tests)
│   └── support/                        # Shared test infrastructure
├── state/
│   └── run_003/                        # V3 harness state directory
│       ├── system_state.json           # Phase, substate, counters (Schema 3.1)
│       ├── capability_map.json         # Capability registry (Schema 3.2)
│       ├── golden_baseline.json        # Per-test divergence metrics (Schema 3.3)
│       ├── divergence_clusters.json    # Grouped divergences (Schema 3.4)
│       ├── fix_queue.json              # Prioritized fix tasks (Schema 3.5)
│       ├── golden_extension_queue.json # Capabilities needing golden gen (Schema 3.6)
│       ├── gap_list.json               # Truly missing features (Schema 3.7)
│       ├── stub_ledger.json            # Intentional stubs with justification (Schema 3.8)
│       ├── iteration_log.jsonl         # Per-fix-attempt results (Schema 3.9)
│       ├── traceability_ledger.jsonl   # Per-symbol disposition (Schema 3.10)
│       ├── progress.txt                # Append-only human-readable log (Schema 3.11)
│       └── backups/                    # Rotated backups (max 5)
└── ...
```

Write state only to `state/run_003/`. Rust source and test directories are read-write. C++ source tree is read-only.

---

## 3. Schemas

### 3.1 System State (`system_state.json`)

```json
{
  "harness_version": "3.0.0",
  "run_id": "<UUID>",
  "created_at": "<ISO8601>",
  "last_updated": "<ISO8601>",
  "current_phase": "<enum: uninitialized|inventory|golden_baseline|golden_extension|divergence_fixing|gap_implementation|stub_accounting|complete|halted>",
  "current_substate": "<enum per phase, see below>",
  "halt_reason": "<string|null>",
  "emcore_boundary": {
    "cpp_root": "<absolute path to Eagle Mode source>",
    "header_count": "<integer>",
    "source_count": "<integer>"
  },
  "inventory_status": {
    "scan_complete": "<boolean>",
    "worker_count": "<integer>",
    "workers_completed": "<integer>",
    "workers_failed": "<integer>",
    "total_symbols": "<integer>",
    "classify_complete": "<boolean>",
    "ambiguous_count": "<integer>",
    "resolved_count": "<integer>",
    "confidence_audit_complete": "<boolean>",
    "audit_reclassifications": "<integer>",
    "golden_discovery_complete": "<boolean>",
    "capabilities_total": "<integer>",
    "capabilities_with_golden": "<integer>",
    "capabilities_without_golden": "<integer>"
  },
  "baseline_status": {
    "complete": "<boolean>",
    "tests_run": "<integer>",
    "tests_passing": "<integer>",
    "tests_failing": "<integer>",
    "total_divergent_pixels": "<integer>",
    "clusters_identified": "<integer>"
  },
  "extension_status": {
    "complete": "<boolean>",
    "capabilities_queued": "<integer>",
    "gen_functions_written": "<integer>",
    "golden_files_generated": "<integer>",
    "rust_tests_written": "<integer>",
    "build_failures": "<integer>"
  },
  "fixing_status": {
    "complete": "<boolean>",
    "clusters_total": "<integer>",
    "clusters_fixed": "<integer>",
    "clusters_partial": "<integer>",
    "clusters_unfixable": "<integer>",
    "total_fix_attempts": "<integer>",
    "total_commits": "<integer>",
    "pixels_eliminated": "<integer>"
  },
  "gap_status": {
    "complete": "<boolean>",
    "features_total": "<integer>",
    "features_implemented": "<integer>",
    "features_stubbed": "<integer>"
  },
  "stub_status": {
    "complete": "<boolean>",
    "stubs_documented": "<integer>"
  }
}
```

**Valid `current_substate` values per phase:**

| Phase | Substates |
|---|---|
| `uninitialized` | `null` |
| `inventory` | `SCAN`, `CLASSIFY`, `AUDIT`, `DISCOVER` |
| `golden_baseline` | `RUN`, `CLUSTER` |
| `golden_extension` | `EXTENDING`, `RE_BASELINE` |
| `divergence_fixing` | `FIXING` |
| `gap_implementation` | `IMPLEMENTING` |
| `stub_accounting` | `SCANNING` |
| `complete` | `null` |
| `halted` | `null` (halt_reason carries the detail) |

### 3.2 Capability Map (`capability_map.json`)

```json
{
  "run_id": "<UUID, must match system_state>",
  "created_at": "<ISO8601>",
  "last_updated": "<ISO8601>",
  "version": "<integer, increments on every write>",
  "immutability_hash": "<SHA-256 of capabilities array with mutable fields zeroed>",
  "capabilities": [
    {
      "id": "CAP-NNNN",
      "name": "<human-readable capability name>",
      "category": "<enum: painter|layout|interaction|compositor|widget|widget_state|notice|input|scheduler|animator|input_filter|model|config|platform|other>",
      "cpp_provenance": [
        {
          "header": "<emCore header filename, e.g. emPainter.h>",
          "symbols": ["<C++ symbol name>"],
          "line_range": [<start>, <end>]
        }
      ],
      "rust_target": {
        "file": "<relative path from src/, e.g. render/painter.rs>",
        "symbols": ["<Rust symbol name>"]
      },
      "status": "<enum: verified|divergent|untested|not_ported|stub|out_of_boundary>",
      "golden_tests": [
        {
          "file": "<golden data path relative to tests/golden/data/>",
          "test_function": "<Rust test function name>",
          "test_module": "<test module filename, e.g. painter.rs>",
          "comparison_method": "<enum: compare_images|compare_rects|compare_behavioral|compare_notices|compare_input|compare_trajectory|inline_assertion>",
          "tolerance": {
            "channel_tolerance": "<integer|null>",
            "max_failure_pct": "<float|null>",
            "epsilon": "<float|null>"
          }
        }
      ],
      "divergence": {
        "measured": "<boolean>",
        "passing": "<boolean|null>",
        "last_measured": "<ISO8601|null>",
        "divergent_pixels": "<integer|null>",
        "max_channel_diff": "<integer|null>",
        "failure_pct": "<float|null>",
        "cluster_id": "<string|null>"
      },
      "reclassification": "<null or object>",
      "confidence": "<float 0.0-1.0>",
      "verification_evidence": "<string describing how status was determined>"
    }
  ]
}
```

**Reclassification object** (required when status is `out_of_boundary`, `stub`, or `not_applicable`):

```json
{
  "original_status": "<enum: previous status>",
  "new_status": "<enum: out_of_boundary|stub>",
  "reason": "<string: structured justification>",
  "evidence_type": "<enum: outside_emcore|platform_limitation|deliberate_omission|rust_idiom_replacement>",
  "evidence": "<string: MUST reference a specific Rust file path and symbol, or confirm absence of both. Generic evidence ('Rust uses native equivalent') is a schema violation>",
  "cpp_header": "<emCore header where the symbol is defined>",
  "approved_at": "<ISO8601>"
}
```

**Mutable fields** (excluded from immutability hash): `status`, `divergence`, `reclassification`, `confidence`, `verification_evidence`, `golden_tests`.

**Frozen fields** (included in immutability hash): `id`, `name`, `category`, `cpp_provenance`, `rust_target`.

### 3.3 Golden Baseline (`golden_baseline.json`)

```json
{
  "run_id": "<UUID>",
  "measured_at": "<ISO8601>",
  "cargo_test_exit_code": "<integer>",
  "total_tests": "<integer>",
  "passing": "<integer>",
  "failing": "<integer>",
  "results": [
    {
      "test_function": "<string>",
      "test_module": "<string>",
      "golden_file": "<string>",
      "passed": "<boolean>",
      "comparison_method": "<string>",
      "metrics": {
        "divergent_pixels": "<integer|null>",
        "total_pixels": "<integer|null>",
        "failure_pct": "<float|null>",
        "max_channel_diff": "<integer|null>",
        "max_rect_error": "<float|null>",
        "trajectory_max_error": "<float|null>"
      },
      "error_message": "<string|null>"
    }
  ]
}
```

### 3.4 Divergence Clusters (`divergence_clusters.json`)

```json
{
  "run_id": "<UUID>",
  "created_at": "<ISO8601>",
  "last_updated": "<ISO8601>",
  "clusters": [
    {
      "cluster_id": "DIV-NNN",
      "name": "<human-readable cluster name>",
      "root_cause_hypothesis": "<string>",
      "domain": "<enum: pixel_arithmetic|geometry|state_logic|trajectory|api_structure|architectural>",
      "fix_strategy": "<enum: port_arithmetic|port_algorithm|fix_geometry|fix_state_logic|fix_api_semantics|architectural_accept>",
      "priority": "<integer 1-999, lower is higher priority>",
      "impact_score": "<integer: total divergent pixels across all affected tests>",
      "affected_tests": ["<test_function names>"],
      "affected_capabilities": ["CAP-NNNN"],
      "cpp_source_files": ["<C++ files to read for the fix>"],
      "status": "<enum: pending|in_progress|fixed|partial|unfixable|accepted>",
      "fix_attempts": "<integer>",
      "max_attempts": 3,
      "resolution": {
        "commit_sha": "<string|null>",
        "pixels_before": "<integer>",
        "pixels_after": "<integer>",
        "description": "<string|null>"
      }
    }
  ]
}
```

### 3.5 Fix Queue (`fix_queue.json`)

```json
{
  "run_id": "<UUID>",
  "last_updated": "<ISO8601>",
  "tasks": [
    {
      "task_id": "FIX-NNNN",
      "cluster_id": "DIV-NNN",
      "priority": "<integer>",
      "status": "<enum: pending|in_progress|completed|failed|skipped>",
      "attempt": "<integer>",
      "max_attempts": 3,
      "assigned_at": "<ISO8601|null>",
      "completed_at": "<ISO8601|null>",
      "timeout_seconds": 1200
    }
  ]
}
```

### 3.6 Golden Extension Queue (`golden_extension_queue.json`)

```json
{
  "run_id": "<UUID>",
  "last_updated": "<ISO8601>",
  "items": [
    {
      "capability_id": "CAP-NNNN",
      "capability_name": "<string>",
      "gen_tier": "<enum: tier1_painter|tier2_layout|tier3_behavioral|tier4_widget>",
      "cpp_classes": ["<C++ classes to exercise>"],
      "golden_subdir": "<string: subdirectory under data/>",
      "golden_type": "<string: file extension, e.g. painter.golden>",
      "comparison_method": "<string: Rust comparison function to use>",
      "status": "<enum: pending|gen_written|built|generated|test_written|verified|failed>",
      "gen_function_name": "<string|null>",
      "golden_file": "<string|null>",
      "rust_test_function": "<string|null>",
      "error": "<string|null>"
    }
  ]
}
```

### 3.7 Gap List (`gap_list.json`)

```json
{
  "run_id": "<UUID>",
  "last_updated": "<ISO8601>",
  "features": [
    {
      "feature_id": "GAP-NNN",
      "capability_id": "CAP-NNNN",
      "cpp_header": "<emCore header>",
      "cpp_symbols": ["<symbol names>"],
      "description": "<what needs to be implemented>",
      "complexity": "<enum: low|medium|high>",
      "status": "<enum: pending|in_progress|implemented|stubbed>",
      "rust_file": "<string|null>",
      "golden_test": "<string|null>",
      "commit_sha": "<string|null>"
    }
  ]
}
```

### 3.8 Stub Ledger (`stub_ledger.json`)

```json
{
  "run_id": "<UUID>",
  "last_updated": "<ISO8601>",
  "stubs": [
    {
      "capability_id": "CAP-NNNN",
      "cpp_symbol": "<string>",
      "cpp_header": "<string>",
      "rust_file": "<string>",
      "rust_line": "<integer>",
      "stub_type": "<enum: returns_none|returns_default|no_op|logs_debug|panics|platform_limitation>",
      "justification": "<string: why this cannot be fully ported>",
      "workaround": "<string|null: alternative behavior if any>",
      "blocking_issue": "<string|null: what would need to change to unblock>"
    }
  ]
}
```

### 3.9 Iteration Log (`iteration_log.jsonl`)

One JSON object per line, appended after every fix attempt:

```json
{
  "timestamp": "<ISO8601>",
  "task_id": "FIX-NNNN",
  "cluster_id": "DIV-NNN",
  "attempt": "<integer>",
  "action": "<string: description of what was changed>",
  "files_changed": ["<relative paths>"],
  "tests_run": ["<test function names>"],
  "outcome": "<enum: improved|no_change|regression|build_failure|test_error>",
  "pixels_before": "<integer>",
  "pixels_after": "<integer>",
  "delta": "<integer: pixels_after - pixels_before, negative is improvement>",
  "commit_sha": "<string|null>",
  "reverted": "<boolean>",
  "error": "<string|null>"
}
```

### 3.10 Traceability Ledger (`traceability_ledger.jsonl`)

One JSON object per line, one entry per C++ symbol discovered during inventory:

```json
{
  "item_id": "SYM-NNNNNN",
  "cpp_symbol": "<string>",
  "cpp_header": "<string>",
  "cpp_kind": "<enum: function|method|class|constructor|destructor|constant|macro|type_alias|template|struct|field|operator|enum|static_method>",
  "disposition": "<enum: mapped|grouped|reclassified|stub|not_ported>",
  "capability_id": "CAP-NNNN",
  "confidence": "<float>",
  "audited": "<boolean>",
  "audit_result": "<enum: confirmed|reclassified|null>"
}
```

### 3.11 Progress Log (`progress.txt`)

Append-only, pipe-delimited. Maximum 100 MB; rotate at 90%.

```
<ISO8601> | <ENTITY_ID> | <ACTION> | <OUTCOME> | <DETAIL>
```

Entity IDs: `SYSTEM`, `W-NNNN` (map worker), `CAP-NNNN`, `DIV-NNN`, `FIX-NNNN`, `GAP-NNN`.

Actions: `SCAN`, `CLASSIFY`, `AUDIT`, `DISCOVER`, `BASELINE`, `CLUSTER`, `EXTEND_GEN`, `EXTEND_TEST`, `FIX_ATTEMPT`, `FIX_COMMIT`, `FIX_REVERT`, `IMPLEMENT`, `STUB`, `HALT`, `RESUME`, `PHASE_COMPLETE`.

Outcomes: `OK`, `FAIL`, `SKIP`, `IMPROVED`, `NO_CHANGE`, `REGRESSION`, `TIMEOUT`, `BUILD_FAIL`.

### 3.12 Map Worker Output (per-worker, not persisted as separate file — consumed by reduce)

```json
{
  "worker_id": "W-NNNN",
  "cpp_header": "<string: header filename>",
  "started_at": "<ISO8601>",
  "completed_at": "<ISO8601>",
  "symbols": [
    {
      "cpp_symbol": "<string>",
      "cpp_kind": "<enum: function|method|class|constructor|destructor|constant|macro|type_alias|template|struct|field|operator|enum|static_method>",
      "cpp_line_range": [<start>, <end>],
      "classification": "<enum: ported|not_ported|stub|ambiguous>",
      "confidence": "<float 0.0-1.0>",
      "rust_file": "<string|null: relative path from src/>",
      "rust_symbol": "<string|null>",
      "ambiguity_reason": "<string|null: required if classification == ambiguous>"
    }
  ]
}
```

Validation rules (Phase 4a step 5): JSON parses. `worker_id` matches assignment. `cpp_header` matches assignment. Every symbol has valid `cpp_kind` and `classification` enum values. `confidence` is in `[0.0, 1.0]`. If `classification == "ported"` or `"stub"`: `rust_file` and `rust_symbol` are non-null. If `classification == "ambiguous"`: `ambiguity_reason` is non-null. `symbols` array length > 0 and <= 10,000.

---

## 4. Phase 1: Inventory & Golden Mapping

### 4a. Scan (Map-Reduce)

**Trigger:** `system_state.current_phase == "uninitialized"` or `current_substate == "SCAN"`.

**Procedure:**

1. Enumerate all C++ header files in `~/.local/git/eaglemode-0.96.4/include/emCore/`. Hard limit: 200 files. If exceeded: HALT ("emCore boundary larger than expected").

2. Enumerate all Rust source files in `src/`. Hard limit: 500 files.

3. For each C++ header, spawn a map worker (isolated LLM invocation). The worker receives:
   - The C++ header file (read-only)
   - Read access to the entire Rust `src/` directory
   - The instruction: "Extract every public symbol (function, method, class, constructor, destructor, constant, macro, type_alias, template, enum, struct, field, operator, static_method). For each symbol, find the Rust equivalent in src/ if one exists. Classify as: ported (Rust equivalent exists with non-trivial method body), not_ported (no Rust equivalent), stub (Rust equivalent exists but body is a no-op, returns None/default, or logs debug), ambiguous (uncertain). Assign confidence 0.0-1.0. Output as JSON matching the map worker schema."

4. Maximum 20 concurrent workers. Per-worker timeout: 180 seconds. Retry budget: 3 attempts per worker. After 3 failures, all symbols from that header are classified as ambiguous with confidence 0.0.

5. Validate each worker output: JSON parses, all required fields present, enum values valid, confidence in range.

6. Reduce: merge all worker outputs into a single symbol list. Assign sequential SYM-NNNNNN IDs. Route ambiguous items to classification. Write `traceability_ledger.jsonl` with one entry per symbol.

**Gate condition:** All workers completed or exhausted retries. `traceability_ledger.jsonl` contains one entry per discovered symbol. Set `system_state.inventory_status.scan_complete = true`. Set `current_substate = "CLASSIFY"` if `ambiguous_count > 0`, else `"AUDIT"`.

### 4b. Classify (Research)

**Trigger:** `scan_complete == true` and `ambiguous_count > 0`. If `ambiguous_count == 0`, set `classify_complete = true` and skip to Phase 1c.

**Procedure:**

1. Read all ambiguous items from the traceability ledger.
2. For each ambiguous item, the research agent:
   - Searches the Rust source tree for the symbol name, variants, and related identifiers.
   - Reads the C++ header and source to understand what the symbol does.
   - Checks git history for prior porting decisions.
   - Classifies as: `mapped` (has Rust equivalent), `stub` (Rust equivalent is incomplete), `not_ported` (no equivalent), `out_of_boundary` (not actually emCore).
3. Maximum 10 search rounds per item. Maximum 4 hours wall-clock total. Unresolvable items default to `not_ported`.
4. Update traceability ledger entries with resolved dispositions.

**Gate condition:** All ambiguous items resolved or marked not_ported. Set `classify_complete = true`. Set `current_substate = "AUDIT"`.

### 4c. Confidence Audit (MANDATORY)

**Trigger:** `classify_complete == true`. This step is NOT optional. It runs regardless of confidence score distribution.

**Procedure:**

1. Stratified sampling from the traceability ledger:
   - All items with confidence < 0.8: audit 100%.
   - Items with confidence 0.8-0.95: audit 30%.
   - Items with confidence > 0.95: audit 15%.
   - Items classified as `stub`: audit 100%.
   - If ALL items have confidence 1.0: audit 25% of every category.

2. For each audited item: re-read the C++ header AND the identified Rust file. Verify:
   - Does the Rust symbol actually exist at the location claimed?
   - Is the classification correct? (If `mapped`, does the Rust code contain a real implementation, not just a function signature or a stub?)
   - If `not_ported`, confirm no Rust equivalent exists anywhere in `src/`.

3. Record audit results in the traceability ledger (`audited: true`, `audit_result`).
4. If any reclassifications occur, update the traceability ledger and increment `audit_reclassifications`.
5. **Audit integrity spot-check:** Randomly select 5 items where `audit_result == "confirmed"` and `classification == "mapped"`. For each, run `grep -n '<rust_symbol>' src/<rust_file>` using the `rust_symbol` and `rust_file` from the traceability ledger. Verify the output contains a function or method body (not just an import, type alias, or comment). If any of the 5 produce no match, HALT with reason "Audit integrity check failed for SYM-NNNNNN: rust_symbol not found at claimed location".

**Gate condition:** `confidence_audit_complete == true` AND the count of entries in `traceability_ledger.jsonl` with `audited: true` meets or exceeds the required sample size. Required sample size: all items with confidence < 0.8 + 30% of items with confidence 0.8-0.95 + 15% of items with confidence > 0.95 + all items with classification `stub`. If all items have confidence 1.0: 25% of total items. If the audited count is less than the computed requirement, the gate FAILS — do not proceed. If `audit_reclassifications / total_audited > 0.10`, log warning to progress.txt: "High reclassification rate: {rate}%". On pass: set `current_substate = "DISCOVER"`.

**Reclassification cap (enforced at end of Phase 1d, after all capability statuses are set):** If total capabilities with status `out_of_boundary` exceeds 15% of `capabilities_total`, HALT with reason "Excessive reclassification rate ({n}/{total}). Operator review required."

### 4d. Capability Grouping & Golden Discovery

**Trigger:** `confidence_audit_complete == true`.

**Note:** The test files in `tests/behavioral/*.rs` contain comments referencing a retired `PORT-NNNN` ID scheme from prior harness runs. Ignore these references. Do not create capabilities named after PORT IDs. The only valid capability ID scheme is `CAP-NNNN`.

**Procedure:**

1. **Group symbols into capabilities** under a single CAP-NNNN. Assignment rules:
   - All symbols mapping to the same Rust function or method → one capability.
   - All symbols mapping to the same Rust struct/enum → one capability per logical operation group.
   - Symbols with no Rust equivalent → one capability per distinct C++ class or function group.
   - Maximum 2,000 capabilities. If exceeded: HALT.

2. **Discover golden test mappings.** For each capability:
   - Parse `tests/golden/*.rs` to find test functions that exercise the capability's Rust symbols. Match by: (a) the golden file name containing the capability name or C++ class name, (b) the test function importing or calling the Rust symbol, (c) comments referencing the C++ symbol.
   - For each match, record the golden_tests entry with file path, test function name, comparison method, and tolerance parameters (read from the test source code).
   - Also check `tests/behavioral/*.rs`, `tests/integration/*.rs`, `tests/unit/*.rs` for non-golden tests that provide behavioral verification.

3. **Classify each capability:**
   - `untested`: all constituent symbols are `mapped` (with or without golden tests — Phase 2 resolves `verified` vs `divergent`).
   - `not_ported`: at least one constituent symbol has no Rust equivalent.
   - `stub`: at least one constituent symbol is a stub.

4. **Write `capability_map.json`.** Compute immutability hash over frozen fields.

5. **Write `golden_extension_queue.json`** for all capabilities with status `untested` AND an empty `golden_tests` array. Capabilities that are `untested` but already have golden tests do not need extension — Phase 2 will verify them. For each queued item, determine the `gen_tier`:
   - Capabilities in category `painter`: `tier1_painter`
   - Capabilities in category `layout`: `tier2_layout`
   - Capabilities in categories `interaction`, `notice`, `input`: `tier3_behavioral`
   - Capabilities in categories `compositor`, `widget`, `widget_state`, `animator`, `input_filter`: `tier4_widget`
   - Other categories: `tier3_behavioral` (default)

6. **Write `gap_list.json`** for all capabilities with status `not_ported`.

**Gate condition:** `capability_map.json` exists, hash validates, every symbol in the traceability ledger maps to exactly one capability. Set `golden_discovery_complete = true`. Transition: set `current_phase = "golden_baseline"`, `current_substate = "RUN"`.

---

## 5. Phase 2: Golden Baseline

**Trigger:** `current_phase == "golden_baseline"` (set after Phase 1 gate passes).

### 5a. Run Full Golden Suite

**Procedure:**

1. Run: `MEASURE_DIVERGENCE=1 DIVERGENCE_LOG=state/run_003/divergence_raw.jsonl cargo test --test golden -- --test-threads=1 2>&1`
2. Capture exit code, stdout, and stderr.
3. Parse the JSONL divergence log. Each line contains per-test metrics.
4. Also run: `cargo test --test behavioral --test integration --test unit 2>&1`. If any fail, log the failures but do not HALT.

### 5b. Build Baseline

**Procedure:**

1. For each golden test result, create an entry in `golden_baseline.json` with all metrics.
2. Update `capability_map.json`: for each capability with golden tests, set `divergence.measured = true`, `divergence.passing` based on test result, and populate metric fields.
3. Update capability status: `verified` if all golden tests pass, `divergent` if any fail.

### 5c. Cluster Divergences

**Procedure:**

1. Group failing tests by hypothesized root cause. Clustering heuristics:
   - Tests sharing the same comparison method AND `max_channel_diff` within 2 of each other → single formula/precision issue.
   - Tests in the same domain failing with error magnitudes within 10x of each other.
   - Tests exercising the same C++ API.

2. For each cluster, assign a `DIV-NNN` ID and generate:
   - A root cause hypothesis.
   - A domain classification using the lookup table in Section 14 (determine domain from the failing test's comparison method).
   - A fix strategy (determined by domain per Section 14).
   - A priority score: `impact_score / complexity_weight`. Impact = total divergent pixels across affected tests. Complexity weights: 1 (formula fix), 3 (algorithm port), 5 (architectural change).
   - The list of C++ source files to read for diagnosis.

3. Write `divergence_clusters.json`.
4. Generate `fix_queue.json`: one FIX-NNNN task per cluster, sorted by priority (lower number = higher priority).

**Gate condition:** `golden_baseline.json` exists and has one entry per golden test. `divergence_clusters.json` exists. `fix_queue.json` exists. Set `baseline_status.complete = true`. Transition: if `golden_extension_queue.json` has items with status `pending`, set `current_phase = "golden_extension"`, `current_substate = "EXTENDING"`. Otherwise set `current_phase = "divergence_fixing"`, `current_substate = "FIXING"`.

---

## 6. Phase 3: Golden Extension

**Trigger:** `current_phase == "golden_extension"` and `golden_extension_queue.json` has items with status `pending`.

Process `golden_extension_queue.json` items ordered by `gen_tier` ascending (tier1 first).

### 6a. Write C++ Generator Function

**Procedure:**

1. Read `tests/golden/gen/gen_golden.cpp` to understand existing patterns for the target tier.
2. Read the C++ emCore header(s) for the capability to understand what API to exercise.
3. Write a new `static void gen_<name>()` function in `gen_golden.cpp`, following the tier's setup pattern:

   **Tier 1 (painter):** Use global `g_sched`/`g_ctx`, call `white_image()`, `make_painter()`, exercise the painter API, call `dump_painter()`.

   **Tier 2 (layout):** Use `gen_layout_test<LayoutType>("name", lambda)` template.

   **Tier 3 (behavioral/notice/input):** Create per-test `emStandardScheduler`, `emRootContext`, `emView`. Optionally create `GoldenViewPort`. Build panel tree. Settle with `TerminateEngine(sched, 30)`. Perform action. Settle again. Call appropriate dump function.

   **Tier 4 (widget/compositor/animator/filter):** Same as tier 3, plus: use `Testable<T>` for widgets, call `StubClipboard::Setup(ctx)` for TextField/ColorField/ListBox/RadioButton, use `VF_NO_ACTIVE_HIGHLIGHT` for rendering tests, use `render_and_dump` for widget rendering, use 200 settle cycles for auto-expansion cascades.

4. Add the `gen_<name>();` call to `main()` in the appropriate section.

**Mandatory pitfall avoidance:**
- If the widget uses clipboard operations: `StubClipboard::Setup(ctx)` MUST be called before widget construction. Omission causes a crash.
- If the test renders pixels: the view MUST be created with `emView::VF_NO_ACTIVE_HIGHLIGHT`. Omission causes non-deterministic golden data.
- Notice tests MUST call `ResetRecording()` between the initial settle and the action under test. Omission captures initialization notices instead of action-specific ones.

**Coverage check (before building):** Verify the gen function calls or references every symbol listed in the capability's `cpp_provenance.symbols` array. Search the gen function source text for each symbol name. If any symbol from `cpp_provenance` is not called or referenced in the gen function:
- Either add the missing API call to the gen function.
- Or, if the symbol cannot be exercised in isolation, split the capability: create a new `golden_extension_queue.json` item for the untested symbol and add a `coverage_note` to the current item explaining the split.

### 6b. Build and Generate

**Procedure:**

1. Run: `make -C tests/golden/gen 2>&1`. If build fails: read the error, fix the C++ code, retry. Max 3 build attempts per gen function. After 3 failures: mark item as `failed`, log error, continue to next item.
2. Run: `make -C tests/golden/gen run 2>&1`. Verify the golden file was created at the expected path under `tests/golden/data/`.
3. Verify the golden file: size > 0 bytes and < 50 MB.

### 6c. Write Rust Test

**Procedure:**

1. Determine the appropriate test module in `tests/golden/` based on the capability category.
2. Read the gen function written in step 6a. Reproduce the exact same scenario in Rust: same widget type, same constructor parameters, same state mutations, same viewport dimensions. The C++ gen function defines the scenario; the Rust test must match it.
3. Write the test function using the appropriate loader from `common.rs` and the appropriate comparison function.
4. Run: `cargo test --test golden <new_test_function> 2>&1`. Record pass/fail.
5. If the test fails: record the divergence metrics. Phase 4 addresses divergences.
6. If the test passes: set the capability's status to `verified`.

### 6d. Update State

1. Update `golden_extension_queue.json` item status.
2. Update `capability_map.json` with the new golden_tests entry.
3. Update `system_state.json` extension counters.

**Gate condition:** All items in `golden_extension_queue.json` are either `verified`, `failed` (with logged error), or have divergences recorded. Set `extension_status.complete = true`. Transition: set `current_substate = "RE_BASELINE"`.

**Post-extension:** Set `current_phase = "golden_baseline"`, `current_substate = "RUN"`. Re-run Phase 2 (steps 5a-5c) to incorporate new tests into the divergence analysis. Update `golden_baseline.json`, `divergence_clusters.json`, and `fix_queue.json`. On this second pass, the extension queue is empty, so the Phase 2 gate transitions to `divergence_fixing`.

---

## 7. Phase 4: Divergence Fixing Loop

**Trigger:** `current_phase == "divergence_fixing"` and `fix_queue.json` has tasks with status `pending`.

**Loop invariant:** Iterate while:
- At least one FIX-NNNN task has status `pending` or `in_progress`.
- Total fix attempts across all tasks < 500.
- No HALT condition.

### Per-Iteration Procedure

1. **Select task.** Pick the first `pending` task from `fix_queue.json` (sorted by priority). Set status to `in_progress`, increment attempt count.

2. **Diagnose.** Read the cluster's `root_cause_hypothesis` and `cpp_source_files`. For each C++ source file:
   - Read the relevant C++ implementation.
   - Read the corresponding Rust implementation.
   - Identify the specific divergence: what does C++ do that Rust does differently?
   - The divergence classification (Section 14) determines the fix strategy.

3. **Fix.** Apply the fix strategy:

   **`port_arithmetic`** (Domain 1): Port the C++ integer arithmetic exactly. Use newtypes (`Fixed12`, `div255_round()`) to wrap faithful arithmetic. Reference specific C++ source lines in comments. Do not use f64 approximations.

   **`port_algorithm`** (Domain 1/2): Port the C++ algorithm exactly (interpolation, tessellation, search). Preserve operation order. Do not alter or "improve" it.

   **`fix_geometry`** (Domain 2): Fix the formula to match C++ (wrong constant, missing offset, inverted sign). Preserve operation order on golden-tested paths. No parallel iteration.

   **`fix_state_logic`** (Domain 3/4): Fix state transitions or event propagation to match C++ behavior in all edge cases. Idiomatic Rust data structures are acceptable.

   **`fix_api_semantics`** (Domain 5): Fix the Rust API to match C++ behavioral contracts (return values, side effects, ordering). Use idiomatic Rust syntax freely.

   **`architectural_accept`** (Domain 6): Mark the cluster as `accepted`. Document the tolerance. Adjust golden test tolerance parameters.

4. **Verify.** After applying the fix:
   - Run `cargo check --workspace 2>&1`. If it fails: fix compilation errors, retry. Max 3 compilation fix attempts. After 3: revert all changes, mark attempt as `build_failure`.
   - Run the affected golden tests: `MEASURE_DIVERGENCE=1 cargo test --test golden <test1> <test2> ... 2>&1`.
   - Parse the divergence metrics for each affected test.

5. **Evaluate.**
   - **Improved** (total divergent pixels decreased): Commit the fix with message `fix(DIV-NNN): <description>`. Update the cluster's resolution. Update `capability_map.json` divergence metrics. If all affected tests now pass: set cluster status to `fixed`.
   - **No change** (total divergent pixels unchanged): Revert all changes. Log the attempt. If `attempt < max_attempts`: re-diagnose with different hypothesis. If `attempt == max_attempts`: mark cluster as `unfixable`.
   - **Regression** (total divergent pixels increased OR previously passing tests now fail): Revert all changes immediately. Log as `regression`. Counts as a failed attempt.
   - **Partial improvement** (some tests improved, others unchanged): Commit the improvement. Re-cluster remaining divergences. Continue.

6. **Record.** Append an entry to `iteration_log.jsonl` with all metrics.

7. **Update state.** Update `fix_queue.json` task status. Update `system_state.json` fixing counters.

8. **Periodic re-baseline.** Count fix commits by entries in `iteration_log.jsonl` where `commit_sha` is non-null. When this count is a non-zero multiple of 10, re-run the full golden suite (`MEASURE_DIVERGENCE=1 cargo test --test golden 2>&1`) before starting the next task. If new regressions are found: create new DIV clusters and FIX tasks at priority 1.

**Gate condition:** All FIX tasks are `completed`, `skipped` (unfixable), or `accepted` (architectural). No `pending` tasks remain. Set `fixing_status.complete = true`. Transition: set `current_phase = "gap_implementation"`, `current_substate = "IMPLEMENTING"`.

---

## 8. Phase 5: Gap Implementation

**Trigger:** `current_phase == "gap_implementation"` and `gap_list.json` has features with status `pending`.

Process `gap_list.json` features ordered by complexity ascending (low first).

### Per-Feature Procedure

1. **Read C++ source.** Read the emCore header and source for the feature.

2. **Implement Rust code.**
   - Match the project's existing module structure.
   - For numerical algorithms: port C++ arithmetic exactly (Section 14).
   - For data structures: use idiomatic Rust (SlotMap, enum, trait objects).

3. **Verify compilation.** Run `cargo check --workspace 2>&1`. Fix any errors.

4. **Generate golden data.** Extend `gen_golden.cpp` with a gen function for this feature following the patterns in Section 6a. Build and run the generator.

5. **Write golden test.** Write a Rust test in the appropriate `tests/golden/*.rs` module.

6. **Verify.** Run the golden test. If it fails:
   - Read the divergence metrics.
   - Fix the implementation.
   - Re-run. Max 5 fix cycles per feature.
   - After 5 failures: mark feature as `stubbed`, add to stub ledger.

7. **Commit.** Commit with message `feat(GAP-NNN): implement <feature name>`.

8. **Update state.** Update `gap_list.json`, `capability_map.json`, `system_state.json`.

**Scope enforcement:** If a feature requires APIs outside emCore, implement the emCore-visible API surface and stub the platform-specific internals. Record the stub in `stub_ledger.json` with `blocking_issue`.

**Gate condition:** All features in `gap_list.json` are `implemented` or `stubbed`. Set `gap_status.complete = true`. Transition: set `current_phase = "stub_accounting"`, `current_substate = "SCANNING"`.

---

## 9. Phase 6: Stub Accounting

**Trigger:** `current_phase == "stub_accounting"`.

**Procedure:**

1. Scan the entire Rust source tree for:
   - Functions/methods returning `None` where C++ returns a real value.
   - Functions/methods with empty bodies or bodies that only log debug messages.
   - Functions/methods returning hardcoded default values where C++ computes a result.
   - Comments containing "stub", "todo", "placeholder", "no-op", "not implemented".

2. For each stub found, create an entry in `stub_ledger.json` with:
   - The capability it belongs to (from `capability_map.json`).
   - The stub type.
   - Justification for why full porting is infeasible.
   - Workaround (if any).
   - Blocking issue (if any).

3. Cross-reference against `capability_map.json`. Any capability with status `stub` MUST have at least one entry in `stub_ledger.json`. Any entry in `stub_ledger.json` MUST reference a valid capability.

4. Update capability statuses: if a capability has stubs but all golden tests pass, status remains `verified`. If golden tests fail, status is `divergent`.

**Gate condition:** `stub_ledger.json` exists. Every stub in the codebase is documented. Cross-reference validates. Set `stub_status.complete = true`. Transition to Completion (Section 10).

---

## 10. Completion

**Trigger:** All phase gate conditions are met.

**Procedure:**

1. Set `system_state.current_phase = "complete"`.
2. Run the full test suite one final time: `cargo test --workspace 2>&1`. Record the total test count and pass count.
3. Run the full golden suite with measurement: `MEASURE_DIVERGENCE=1 cargo test --test golden 2>&1`. Record final baseline.
4. Generate a completion summary to progress.txt:
   - Total capabilities
   - Verified (golden tests pass)
   - Accepted (architectural divergence with documented tolerance)
   - Stubbed (intentional limitations)
   - Remaining divergent
   - Total golden tests
   - Total divergent pixels eliminated
   - Total commits
5. Commit all state files with message `harness: V3 run complete`.

---

## 11. State Machine

```
UNINITIALIZED
    → INVENTORY_SCAN           (harness start)

INVENTORY_SCAN
    → INVENTORY_CLASSIFY       (all workers done, ambiguous > 0)
    → INVENTORY_AUDIT          (all workers done, ambiguous == 0)

INVENTORY_CLASSIFY
    → INVENTORY_AUDIT          (all items resolved)

INVENTORY_AUDIT
    → INVENTORY_DISCOVER       (audit complete)

INVENTORY_DISCOVER
    → GOLDEN_BASELINE_RUN      (capability map written)

GOLDEN_BASELINE_RUN
    → GOLDEN_BASELINE_CLUSTER  (all tests run, metrics collected)

GOLDEN_BASELINE_CLUSTER
    → GOLDEN_EXTENSION         (clusters written, extension queue non-empty)
    → DIVERGENCE_FIXING        (clusters written, extension queue empty)

GOLDEN_EXTENSION
    → GOLDEN_BASELINE_RUN      (all extensions done; re-baseline)

DIVERGENCE_FIXING
    → GAP_IMPLEMENTATION       (fix queue empty)

GAP_IMPLEMENTATION
    → STUB_ACCOUNTING          (all gaps implemented or stubbed)

STUB_ACCOUNTING
    → COMPLETE                 (all stubs documented)

Any state → HALTED             (unrecoverable error)
HALTED → <previous state>      (operator resolves halt, runs `harness resume`)
```

Total states: 12.

---

## 12. Failure Catalog

### Inventory Failures

**F01 — Map worker timeout.** Detection: worker does not return within 180s. Resolution: retry up to 3 times. After 3: synthesize all symbols from that header as ambiguous with confidence 0.0.

**F02 — Map worker invalid output.** Detection: JSON parse failure or schema violation. Resolution: retry up to 3 times. After 3: same as F01.

**F03 — All map workers fail for a header.** Detection: 3 retries exhausted for a single header. Resolution: log to progress.txt, continue. The header's symbols are all ambiguous and will be handled by the research agent.

**F04 — emCore larger than expected.** Detection: >200 header files found. Resolution: HALT. Operator verifies the path and boundary.

**F05 — Confidence audit finds >10% reclassifications.** Detection: `audit_reclassifications / total_audited > 0.10`. Resolution: log warning. Do not HALT.

**F05a — Audit integrity spot-check fails.** Detection: any of the 5 spot-checked items has `rust_symbol` not found in `rust_file` via grep. Resolution: HALT with reason identifying the failing SYM-NNNNNN.

**F05b — Excessive reclassification rate.** Detection: capabilities with status `out_of_boundary` exceed 15% of `capabilities_total` after Phase 1d. Resolution: HALT. Operator reviews a random sample of 10 reclassifications.

**F05c — Audit cardinality check fails.** Detection: count of `audited: true` entries in traceability ledger is less than the computed required sample size. Resolution: gate FAILS. Resume audit from first unaudited item in the sample.

**F06 — Capability count exceeds limit.** Detection: >2,000 capabilities after grouping. Resolution: HALT.

### Golden Baseline Failures

**F07 — Golden test binary fails to compile.** Detection: `cargo test --test golden` exits with compilation error. Resolution: HALT.

**F08 — MEASURE_DIVERGENCE output missing.** Detection: divergence log file is empty or missing after test run. Resolution: verify `common.rs` MEASURE_DIVERGENCE support. Fix if broken. Retry.

**F09 — All golden tests fail.** Detection: 0 passing tests. Resolution: HALT. Do not attempt automated fixes.

### Golden Extension Failures

**F10 — gen_golden.cpp build failure.** Detection: `make -C tests/golden/gen` exits non-zero. Resolution: read compiler error, fix the C++ code, retry. Max 3 retries per gen function. After 3: mark extension item as `failed`, continue to next.

**F11 — Golden generator runtime crash.** Detection: `make -C tests/golden/gen run` exits with signal (segfault, abort). Resolution: review the gen function against tier patterns (common cause: missing StubClipboard or wrong context setup). Fix and retry. Max 3 retries.

**F12 — Generated golden file is empty.** Detection: file exists but has 0 bytes. Resolution: fix the gen function (dump not called or wrong output path). Retry.

**F13 — Eagle Mode libraries not found.** Detection: linker error referencing libemCore.so. Resolution: HALT.

**F13a — Gen function coverage check fails.** Detection: one or more `cpp_provenance.symbols` not found in gen function source. Resolution: add the missing API call. If the symbol cannot be exercised in isolation, split the capability (create new extension queue item). Do not proceed to build until all symbols are covered or explicitly split.

### Divergence Fixing Failures

**F14 — Fix causes compilation failure.** Detection: `cargo check` fails after applying fix. Resolution: attempt to fix compilation errors (max 3 attempts). If unfixable: revert all changes, log as `build_failure`.

**F15 — Fix causes regression.** Detection: previously passing tests now fail after fix. Resolution: revert all changes immediately. Do not commit. Log as `regression`. Count as failed attempt.

**F16 — Fix has no effect.** Detection: divergent pixel counts unchanged. Resolution: revert. If attempts remain: re-diagnose with different hypothesis.

**F17 — Fix attempt budget exhausted for a cluster.** Detection: `attempt == max_attempts` (3). Resolution: mark cluster as `unfixable`. Log diagnosis history. Continue to next cluster.

**F18 — Global iteration budget exhausted.** Detection: total fix attempts >= 500. Resolution: complete current fix (commit or revert), transition to GAP_IMPLEMENTATION. Log remaining unfixed clusters.

**F19 — Periodic re-baseline finds new regressions.** Detection: tests passing in previous baseline now fail. Resolution: create new DIV clusters with priority 1. Insert FIX tasks at front of queue.

### Gap Implementation Failures

**F20 — Implementation fails compilation after 5 cycles.** Detection: `cargo check` fails after 5 fix attempts. Resolution: mark feature as `stubbed`. Create stub that compiles. Add to stub ledger with `blocking_issue` describing the compilation problem.

**F21 — Golden test fails after 5 fix cycles.** Detection: golden test does not pass after 5 implementation attempts. Resolution: if `failure_pct > 5.0`: mark as `stubbed`. If `failure_pct <= 1.0`: mark as `verified` with residual divergence noted in `verification_evidence`. If between 1.0 and 5.0: mark as `stubbed`.

**F22 — Cannot generate golden data for a feature.** Detection: feature requires C++ runtime behavior that gen_golden.cpp cannot exercise (network I/O, filesystem interaction). Resolution: write a Rust-only behavioral test in `tests/behavioral/` instead. Mark golden extension item as `failed` with reason.

### System Failures

**F23 — Git commit failure.** Detection: `git commit` exits non-zero. Resolution: retry once. If still fails: HALT.

**F24 — Filesystem full or permission error.** Detection: write to state file fails. Resolution: HALT.

**F25 — Concurrent session detection.** Detection: `state/run_003/.lock` exists and PID is still running. Resolution: refuse to start. Log error.

**F26 — State file corruption.** Detection: JSON parse failure on any state file. Resolution: attempt restore from `backups/`. If backup also corrupt: HALT.

---

## 13. Operational Procedures

### Starting the Harness

```bash
# From the zuicchini project root:
# The harness reads this document and executes from Phase 1.
# State is written to state/run_003/
```

On first invocation: create `state/run_003/`, write initial `system_state.json` with phase `uninitialized`, create `.lock` file with PID.

### Resuming After Interruption

Read `system_state.json`. The `current_phase` and `current_substate` determine where to resume:
- `INVENTORY_SCAN`: check which workers completed (via traceability ledger). Re-run only missing workers.
- `INVENTORY_CLASSIFY`: check which items are still ambiguous. Resume from first unresolved.
- `INVENTORY_AUDIT`: check which items have `audited: true`. Resume from first unaudited in the sample.
- `GOLDEN_BASELINE_RUN`: re-run the full suite (tests are idempotent).
- `GOLDEN_EXTENSION`: check item statuses. Resume from first `pending`.
- `DIVERGENCE_FIXING`: check `fix_queue.json`. Resume from first `pending` or `in_progress` task. If `in_progress`: check if uncommitted changes exist in git. If yes: run the affected tests to evaluate. If no: re-start the task.
- `GAP_IMPLEMENTATION`: check `gap_list.json`. Resume from first `pending`.
- `STUB_ACCOUNTING`: re-scan from scratch (it's a read-only scan).

### Resuming After HALT

1. Read `system_state.halt_reason`.
2. The operator fixes the underlying issue.
3. The operator signals resume.
4. The harness re-reads `system_state.json`, determines the last valid substate from `progress.txt`, and transitions there.

### Inspecting State

All state files are JSON, readable with `jq`:
```bash
# Current phase and status
jq '.current_phase, .current_substate' state/run_003/system_state.json

# Capability coverage
jq '[.capabilities[] | .status] | group_by(.) | map({(.[0]): length}) | add' state/run_003/capability_map.json

# Divergence summary
jq '.clusters[] | select(.status == "pending") | {name, impact_score, priority}' state/run_003/divergence_clusters.json

# Fix progress
jq '[.tasks[] | .status] | group_by(.) | map({(.[0]): length}) | add' state/run_003/fix_queue.json

# Recent progress
tail -20 state/run_003/progress.txt

# Iteration log (last 5 fix attempts)
tail -5 state/run_003/iteration_log.jsonl | jq .
```

### Backup Protocol

Before every write to `capability_map.json`: copy current version to `backups/capability_map.json.bak.<version>`. Retain most recent 5 backups. Delete oldest when exceeding 5.

Before every fix commit: ensure all state files are written and consistent.

---

## 14. Divergence Classification System

The fidelity strategy is domain-specific, not a global preference. The domain determines the fix strategy.

### Domain 1 — Pixel Arithmetic (blend, coverage, compositing, interpolation, sampling)

Classification: `port_faithfully`. Fix strategy: `port_arithmetic`. **No exceptions.**

Reproduce the C++ integer formula exactly. These items must be faithful:
- All blend formulas (premul_over, lerp, canvas_blend) — use `(x*257+0x8073)>>16`, not f64 division
- 12-bit fixed-point scanline coverage pipeline
- 64,771-entry integer sqrt table for radial gradients
- 24-bit fixed-point image scaling/interpolation
- UQ_ADAPTIVE Hermite spline interpolation
- Any function whose output feeds the above

The Blinn formula diverges from f64 rounding for 6 of 256 input values. Alpha blend errors accumulate: 3-5 per channel after 10 compositing layers, exceeding the 1-3 channel tolerance budget.

**When fixing:** Extract duplicated formulas into single tested functions. Use newtypes (`Fixed12`, `div255_round()`) to encapsulate faithful arithmetic in idiomatic Rust wrappers. Replace magic constants with named constants including derivation comments: `const BLINN_BIAS: u32 = 0x8073; // (128 * 257 + 1) / 2`.

### Domain 2 — Geometric Computation (coordinates, rects, transforms, layout)

Classification: `port_faithfully` if divergence > 1e-6 epsilon; `accept` if within. Fix strategy: `fix_geometry`.

Rules:
- Preserve the same algorithm and operation order on golden-tested paths.
- Iterator patterns are acceptable if summation order matches C++ (`Iterator::sum` is left-fold, same as C++ loop).
- Parallel iteration (Rayon) is forbidden on golden-tested paths — changes accumulation order.
- Clamp/min/max operations must preserve the same boundary values.
- Coordinate space conventions must match exactly.

### Domain 3 — State Logic (event propagation, notice dispatch, focus traversal)

Classification: `port_semantics`. Fix strategy: `fix_state_logic`.

Golden tests use exact match (bool, u32 bitfield). Any divergence means the state machine is wrong. Fix logic to match C++. Idiomatic Rust data structures (enums, SlotMap, bitflags!) are acceptable — golden tests verify output, not structure.

### Domain 4 — Trajectory and Animation

Classification: `port_semantics`. Fix strategy: `fix_state_logic`.

The 1e-4 tolerance (100x wider than layout) allows restructured computation. Rules:
- Preserve the fundamental integration algorithm.
- Expression-level restructuring and iterator patterns are safe.
- Rust idioms for state management (enums for animator state, Option for velocity) are encouraged.

### Domain 5 — Data Structures, Ownership, Error Handling, API Surface

Classification: `adapt_idiomatically`. Fix strategy: `fix_api_semantics`.

Fully idiomatic Rust. Golden tests verify output, not structure. Specific freedoms:
- SlotMap, Arena, any Rust collection
- Enums with data instead of C++ integer constants
- Result/Option instead of C++ abort/null
- Iterators, closures, pattern matching in non-rendering code
- Pub(crate) scoping without C++ parallel

Preserve the behavioral contract (return values, side effects, ordering guarantees). Adapt syntax freely.

### Domain 6 — Architectural Decisions

Classification: `architectural_decision`. Fix strategy: `architectural_accept`.

Document the tolerance. Adjust golden test tolerance parameters. Record in `divergence_clusters.json` with status `accepted`.

### Classification Lookup Table

When classifying a divergence, determine the domain by the comparison method of the failing test:

| Comparison method | Domain | Fix strategy |
|---|---|---|
| `compare_images` (ch_tol 1-3) | Domain 1: Pixel Arithmetic | `port_arithmetic` |
| `compare_rects` (eps 1e-6) | Domain 2: Geometry | `fix_geometry` |
| `compare_behavioral` (exact) | Domain 3: State Logic | `fix_state_logic` |
| `compare_notices` (exact) | Domain 3: State Logic | `fix_state_logic` |
| `compare_input` (exact) | Domain 3: State Logic | `fix_state_logic` |
| `compare_trajectory` (tol 1e-4) | Domain 4: Trajectory | `fix_state_logic` |
| `compare_trajectory` (tol 1e-6) | Domain 2: Geometry | `fix_geometry` |
| `inline_assertion` | Domain 5: API/Structure | `fix_api_semantics` |

### Critical Constraints

1. **Type enforcement over documentation.** Encode constraints in types, not comments. `Fixed12` newtype prevents mixing 12-bit and raw integer arithmetic. `div255_round()` prevents inline formula duplication. LLM sessions lose arithmetic understanding across context boundaries — types survive, comments don't.

2. **Golden tests are verification tools, not permanent specifications.** Plan for golden file succession: the C++ build environment will eventually decay. Tolerance drift compounds: individually rational tolerance increases are collectively pathological.

3. **Incremental migration requires pipeline-level analysis.** The 12-bit coverage convention threads through SubPixelEdges, blend_with_coverage, Span, and rasterizer. Per-function idiomatization is insufficient — refactoring must consider the full pipeline path.

---

## 15. Glossary

**Capability (CAP-NNNN):** One or more C++ symbols grouped into a single testable Rust feature.

**Golden test:** Compares Rust output against binary reference data from C++ emCore.

**Golden data:** Binary reference files under `tests/golden/data/`, generated by `gen_golden.cpp`.

**Golden generator (`gen_golden.cpp`):** C++ program exercising emCore APIs, writing binary reference output.

**Divergence cluster (DIV-NNN):** Group of failing golden tests sharing a hypothesized root cause.

**Fix task (FIX-NNNN):** Work item to resolve a divergence cluster. Retry budget: 3. Timeout: 1200s.

**Gap (GAP-NNN):** C++ emCore capability with no Rust implementation.

**Stub:** Rust function with incomplete behavior (returns None, no-op, returns default).

**Traceability ledger:** Append-only JSONL, one entry per C++ symbol. Full accounting of every symbol's disposition.

**Immutability hash:** SHA-256 of capability map frozen fields (canonical JSON, sorted keys, no whitespace, mutable fields zeroed).

**Confidence audit:** Mandatory sampling pass re-verifying map worker classifications against actual source.

**Baseline:** Full golden test suite results at a point in time, with per-test divergence metrics.

**HALT:** Unrecoverable error. Harness stops. Operator intervention required.

**emCore boundary:** 90 headers + 87 source files in Eagle Mode's emCore library. In-scope for porting.

**Gen tier:** Golden generator function complexity. Tier 1: painter (3-line setup). Tier 2: layout (template helper). Tier 3: behavioral (per-test scheduler). Tier 4: widget (full viewport + clipboard).

**Settle:** Running emCore scheduler for N cycles via `TerminateEngine`. Allows layout, notices, and state to stabilize.
