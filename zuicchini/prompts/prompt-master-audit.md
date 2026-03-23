<!-- RECONSTRUCTION: Round 4 revised prompt. Changes traced to preflight scores. -->
<!-- Original: 98 instructions, P(single)=0.80, P(all)≈5e-10 -->
<!-- Target: ~85 instructions via merges, higher P(single) via wounded fixes -->
Audit the Rust port against C++ using the master contract at `prompts/master-contract.json`. That file lists every method to check, organized in phases. Read it now, then read `CLAUDE.md` at the workspace root.

## How this works

The contract has 4 phases with 111 features total. Each feature is a file, group of methods, or test directory to audit. Each has steps, and a `passes` field starting at `false`.

Your loop:

```
Read next feature where passes == false
  → Read its steps
  → Execute each step (read files, compare, report, fix, test)
  → cargo clippy --workspace -- -D warnings: must be clean
  → cargo nextest run --workspace: must pass
  → Edit the contract JSON: set passes to true
  → git add prompts/master-contract.json <changed files>
  → git commit -m "audit: <feature_id> <summary>"
  → Append to .workflow/widget-comparison/run-log.md (see Log section)
  → Move to next feature
```

<!-- WOUNDED FIX i-34/i-89/i-90 (SR 0.52/0.45/0.48): Moved log append into the main -->
<!-- loop code block so it's part of the mechanical flow, not a separate bottom-third section. -->
<!-- This raises SR by embedding it in the highest-salience formatting structure. -->

**Do not enter plan mode.**
<!-- WOUNDED FIX i-09 (SR 0.80): Added bold to increase salience. -->

## Bootstrap

```bash
cd /home/ar/Development/sosumi-7/zuicchini
python3 -c "
import json
d = json.load(open('prompts/master-contract.json'))
total = sum(1 for p in d['phases'] for f in p['features'])
done = sum(1 for p in d['phases'] for f in p['features'] if f['passes'])
print(f'{done}/{total} features passing')
for p in d['phases']:
    phase_done = sum(1 for f in p['features'] if f['passes'])
    print(f'  {p[\"name\"]}: {phase_done}/{len(p[\"features\"])}')
"
cargo nextest run --workspace 2>&1 | tail -1
```

## Phase descriptions

<!-- WOUNDED FIX i-11 (ST 0.55): Added SUSPECT category to Phase 1 summary to match -->
<!-- contract steps (resolves t-06 tension between 3-category and 4-category systems). -->
<!-- Also added phase-conditional test modification rule (resolves t-01/t-02). -->

**Phase 1 — Uncovered Files Audit (28 features):** Each feature is one Rust file with its C++ counterpart. The `methods` field lists every C++ method to check. Compare each method, report MATCH/MISMATCH/SUSPECT/MISSING, fix MISMATCHes, write tests. **In Phases 1-3, write NEW tests only — do not modify existing tests.**

<!-- FALLEN FIX i-28: The original "Do NOT modify existing tests" was an unqualified -->
<!-- prohibition that directly contradicted Phase 4 (t-01, t-02, t-19, t-20, t-21). -->
<!-- RESOLUTION: Split into phase-scoped rules. Phase 1-3 rule is embedded in Phase 1 -->
<!-- description above. Phase 4 rule is embedded in Phase 4 description below. -->
<!-- This eliminates the contradiction while preserving the original intent: preventing -->
<!-- the agent from modifying existing passing tests as a lazy shortcut in Phases 1-3. -->

**Phase 2 — Gap Assessment (27 features):** Each feature is a group of C++ methods that have no Rust equivalent. For each method: determine if it's IMPLEMENTED (under a different name), NOT_NEEDED (superseded), or NEEDS_IMPLEMENTATION. Implement what's needed, write tests. **In Phase 2, write NEW tests only — do not modify existing tests.**

**Phase 3 — Consolidation Verification (50 features):** Each feature is a Rust file that consolidates multiple C++ classes. Verify all consolidated methods are present. **In Phase 3, write NEW tests only — do not modify existing tests.**

<!-- WOUNDED FIX i-14 (ST 0.40, CF 0.50): Strengthened with explicit scoping and -->
<!-- concrete anti-pattern enumeration. Added explicit Phase 4 exception for test modification. -->
<!-- CONTESTED FIX i-61 (CF 0.25): Resolved by making Phase 4 test modification explicit. -->
<phase4_rules>
**Phase 4 — Test Review (6 features):** Review ALL existing tests (1500+) across every test directory. **In Phase 4, you MUST modify existing defective tests** — this is the one phase where modifying existing tests is required, not prohibited. Check each test for these 5 anti-patterns:

1. Assertions use `is_some()`/`is_ok()` instead of **specific expected values**
2. Side effects (callbacks, invalidation, repaints) are **not verified**
3. Test would **not fail** if implementation were subtly wrong (e.g., off-by-one, wrong sign)
4. Modifier keys are **not set correctly** for modifier-dependent behavior
5. Test only verifies compilation, **not actual behavior**

Strengthen defective tests by fixing their assertions. If a strengthened test now FAILS, that is a real bug — fix the production code too, even if that code is in a file not listed in the Phase 4 feature.
</phase4_rules>

<!-- WOUNDED FIX i-62 (CF 0.60): Added explicit scope clarification for production -->
<!-- code fixes found through test strengthening (resolves t-03 tension with i-29). -->
<!-- WOUNDED FIX i-60 (ST 0.40): Moved the 5-point checklist into the phase description -->
<!-- with numbered items for stronger formatting and salience. -->
<!-- WOUNDED FIX i-96 (ST 0.50): Merged into checklist item 5. -->
<!-- WOUNDED FIX i-97 (ST 0.45): Merged into checklist item 3. -->

## Reporting categories

<!-- NEW: Define SUSPECT. The run log shows it was used extensively in prior sessions -->
<!-- but it had no definition in the original prompt. This resolves t-06 and t-25. -->

Phase 1 comparisons use four categories. Each method gets exactly one:

| Category | Meaning | Action |
|----------|---------|--------|
| **MATCH** | Rust behavior matches C++ | None |
| **MISMATCH** | Rust behavior differs from C++ | Fix the Rust code minimally, write a new test |
| **SUSPECT** | Uncertain — may be a MISMATCH requiring deeper investigation, or may be an idiomatic adaptation that's acceptable under the feature's fidelity layer | Investigate further. If confirmed as behavioral difference: treat as MISMATCH. If confirmed as acceptable adaptation: treat as MATCH. Log as SUSPECT with your reasoning. |
| **MISSING** | C++ method has no Rust equivalent | Implement if required by the feature's scope, write a test |

<!-- This resolves t-25 (SUSPECT has no action path) and t-06 (3 vs 4 categories). -->

## Testing layer requirements

Phase 1 features have a `required_test_layers` field listing which testing layers each fix MUST have. **This is not guidance — it is a requirement:**

- `["unit"]` — write a unit test in the source file's `#[cfg(test)]` module
- `["unit", "golden"]` — also verify or add a golden pixel test in `tests/golden/`
- `["unit", "pipeline"]` — also add a pipeline dispatch test in `tests/pipeline/` using `PipelineTestHarness`
- `["unit", "golden", "pipeline"]` — all three layers

A fix is not complete until all required layers have a test. The clippy/nextest gate verifies tests compile and pass, but YOU must verify the required layers are covered.

<!-- WOUNDED FIX i-16 (CE 0.50, SR 0.60): Made the test directory mapping a concrete -->
<!-- lookup table instead of soft guidance. Added "must" language and reference examples. -->
<test_locations>
**Test locations** (use existing files as reference for patterns — but always write assertions on specific values, never `is_some()`/`is_ok()`):

| Layer | Location | Pattern reference |
|-------|----------|-------------------|
| unit | `#[cfg(test)] mod tests` in the source file | Same file, existing tests |
| pipeline | `tests/pipeline/<widget>.rs` | Existing pipeline test files |
| golden | `tests/golden/widget.rs` or `tests/golden/painter.rs` | Existing golden test files |
| integration | `tests/integration/` | Existing integration test files |
</test_locations>

Phase 1 steps also require reading the C++ `.cpp` file (not just the header) to catch implementation details: static helpers, constructor defaults, and side effects (signals, invalidation, repaint) that headers don't show.

## Fidelity rules

Each feature has a `layer` field:

<!-- WOUNDED FIX i-22/t-22: Added GEOMETRY layer to prompt fidelity rules. Previously -->
<!-- only in contract and CLAUDE.md, creating confusion when features had layer=GEOMETRY. -->
- **PIXEL**: Exact C++ formulas. Integer arithmetic must match. `(x*257+0x8073)>>16` not f64. No approximations in the compositing pipeline.
- **GEOMETRY**: Same algorithm and operation order on golden-tested paths. `Iterator::sum` OK (left-fold matches C++ loop). Clamp/min/max must preserve C++ boundary values.
- **STATE**: Idiomatic Rust is correct. Only flag behavioral differences. Do NOT flag `Rc` vs pointers, `enum` vs int flags, or other type-system adaptations.
- Features **without** a `layer` field: default to **STATE**.

<!-- WOUNDED FIX i-32 (SR 0.50): Moved gate-is-not-sufficient warning into -->
<!-- a prominent callout box near the fidelity rules instead of buried in bottom-third DO NOT list. -->
> **The gate is necessary but not sufficient.** Clippy + tests do NOT verify you compared every method or checked every test. The gate catches compilation and regression errors only. Completeness is YOUR responsibility.

## Phase dependencies

Phases execute in order. Before starting a phase, verify its dependency phase is fully complete — every feature in the dependency phase has `passes: true`. Phase 2 depends on phase 1. Phase 3 depends on phase 2. Phase 4 depends on phase 3. If a dependency phase has incomplete features, finish those first.

If a dependency phase feature cannot be completed (e.g., missing C++ source file), report the blocker in the run log and ask the user for guidance before proceeding.
<!-- Resolves t-11 deadlock: i-21 + i-30 could deadlock if a feature is un-completable. -->

## Cross-feature modifications

<!-- WOUNDED FIX i-22 (SR 0.65): Repositioned under its own header (already was), -->
<!-- added bold emphasis on MUST, and clarified scope boundary with i-29. -->
If a later feature requires changing code from a previously-completed feature (e.g., phase 2 discovers a method needs to be added to a file audited in phase 1), you **MAY** modify that code. But you **MUST** re-run `cargo nextest run --workspace` to confirm all previously-passing tests still pass. If any break, fix them before proceeding. This cross-feature modification is not "unrelated production code" — it is explicitly permitted.

<!-- WOUNDED FIX i-29 (SR 0.48, CE 0.65): Added explicit scope definition for -->
<!-- "unrelated" to reduce ambiguity. Resolves t-03, t-04, t-05. -->
**Scope of "unrelated production code":** Code is "related" if it (a) is listed in the current feature's files, (b) is a direct dependency of a method being fixed, or (c) is the cause of a bug revealed by test strengthening in Phase 4. Everything else is unrelated. Do not modify unrelated production code.

## Known failure modes

<!-- CRITICAL PATH FIX i-23 (ST 0.55) + i-24 (ST 0.55): These are the two wounded -->
<!-- critical-path instructions. Added input-conditional phrasing for large method lists -->
<!-- and explicit tracking requirement to strengthen ST. -->

<completeness_rules>
**Speed-over-fidelity.** The gate (clippy + tests) does NOT verify you compared every method. If the `methods` field lists 20 methods and you compared 5, the gate still passes. **You must compare ALL listed methods.** For features with 15+ methods, explicitly number each method as you compare it (e.g., "Method 1/23: foo() — MATCH") so neither you nor the user loses track.

**Steps are requirements.** If a step says "compare these 20 methods" — compare all 20, not the ones that seem important. **Every step in the contract's steps array must be executed. No step may be skipped or summarized.**
</completeness_rules>

**Test-shaped validation.** Tests must assert on specific values and behaviors, not just `is_some()` or `is_ok()`. If a test cannot assert a specific value, assert the specific state change or side effect instead.
<!-- SURVIVOR i-25: Added clarification for cases where specific values aren't available. -->

**Reusing existing tests.** If you fix a MISMATCH, write a NEW test. Existing tests didn't catch the bug.

## What NOT to do

<!-- FALLEN i-28 removed from this list — replaced by phase-scoped rules above. -->
<!-- WOUNDED i-29 moved to Cross-feature modifications section with explicit scope definition. -->
<!-- WOUNDED i-32 moved to Fidelity rules section as a prominent callout. -->

- Do NOT modify the contract's `steps`, `description`, or `methods` fields. The contract is immutable except for the `passes` field.
- Do NOT skip a feature. If 0 MISMATCHes found, it still passes — set `passes` to true.
- Do NOT flag idiomatic Rust as MISMATCH. Read the `layer` field first.

<!-- WOUNDED FIX i-93 (CE 0.55): Made "minimal" more concrete with a threshold. -->
## Fix scope

For MISMATCH fixes, apply **targeted changes** to the Rust code. "Targeted" means: change only the lines necessary to make the Rust behavior match C++. Do not rewrite surrounding code, refactor call sites, or change APIs unless the fix requires it. If a fix requires more than ~30 lines of changes, note this in the log as a large fix.

<!-- WOUNDED FIX i-49 (CE 0.55): Made RUST-ONLY verification criteria more concrete. -->
## RUST-ONLY features

Seven Phase 1 features have no C++ equivalent (`rust_only: true`). For these, verify internal correctness by checking:

1. **Dead code**: Are there `pub` functions with zero callers outside tests? (Use grep to verify.)
2. **Panicking paths**: Does any non-test code path call `unwrap()` without a same-line proof or `expect()` without a reason? Does any `match` lack a catch-all arm where one is needed?
3. **Edge cases**: What happens at boundary values (0, MAX, empty collections, None inputs)? Write a unit test for any untested edge case found.

## Log

<!-- WOUNDED FIX i-89 (SR 0.45, ST 0.55): Generalized the log format to work across -->
<!-- all phases, not just Phase 1. Resolves t-07 and t-18. -->
After each feature, **append** (never overwrite) to `.workflow/widget-comparison/run-log.md`:

**Phase 1 log format:**
```
### <feature_id>: <description>
**MATCHes**: N | **MISMATCHes**: N | **SUSPECTs**: N | **MISSINGs**: N
**Fixes applied**: <list or "none">
**Tests added**: <count>
```

**Phase 2 log format:**
```
### <feature_id>: <description>
**IMPLEMENTED**: N | **NOT_NEEDED**: N | **NEEDS_IMPLEMENTATION**: N
**Implementations added**: <list or "none">
**Tests added**: <count>
```

**Phase 3 log format:**
```
### <feature_id>: <description>
**Methods verified**: N | **Gaps found**: N
**Fixes applied**: <list or "none">
**Tests added**: <count>
```

**Phase 4 log format:**
```
### <feature_id>: <description>
**Tests reviewed**: N | **Defective**: N | **Strengthened**: N
**Bugs found via strengthening**: <list or "none">
**Production fixes**: <count>
```

## Completion

```bash
python3 -c "
import json
d = json.load(open('prompts/master-contract.json'))
total = sum(1 for p in d['phases'] for f in p['features'])
done = sum(1 for p in d['phases'] for f in p['features'] if f['passes'])
print(f'{done}/{total} features passing')
if done < total:
    for p in d['phases']:
        for f in p['features']:
            if not f['passes']:
                print(f'  PENDING: {f[\"id\"]}')
else:
    print('ALL FEATURES COMPLETE')
"
```

## Begin

Run the bootstrap script. Start with the first feature in phase-1 where `passes` is `false`.
