# SlotMap Panic Root-Cause Investigation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Identify the root cause of `invalid SlotMap key` panic at `emPanelTree.rs:756` triggered by zoom-in/zoom-out in the file manager, then fix it. Stop guessing; gather memory-layer evidence first.

**Approach:** Phased and gated. Each phase produces evidence that either confirms a hypothesis or eliminates it. No phase produces a fix; fixes are concentrated in the final phase, after the root cause is named with mechanical evidence. Vtable-dispatched probes are forbidden — three prior attempts produced SIGSEGV. Substitute non-dispatch probes (POD type tags) and external memory analyzers (ASan).

**Authority:** `docs/debug/investigations/isactive-panic-facts.md` is the source of truth for what is currently known. Do not act on memory entries about this bug without re-reading current code first; some memory predates the IsActive guard commit (`b5dad89b`).

---

## Hardening Rules (Apply To Every Phase)

These rules counter failure modes already observed in this investigation. Re-read before each phase.

1. **No vtable-dispatched probes.** `behavior.type_name()`, `Any::type_id()`, anything that calls into `Box<dyn PanelBehavior>` is forbidden. The trait object is suspect; instruments must not require it to be valid.
2. **One probe at a time.** Add instrumentation, run repro, capture evidence, commit the diff (or revert it), then move on. Do not stack instrumentation; you will not be able to attribute crashes.
3. **Evidence before hypothesis.** Each phase ends by writing a short evidence note appended to `docs/debug/investigations/isactive-panic-facts.md`. State which hypothesis the evidence supports, refutes, or fails to discriminate. Do not write speculation into the facts document.
4. **Stop on null result.** If a phase produces no usable signal (instrumentation crashes, sanitizer won't run, etc.) STOP and report to the user. Do not invent Phase 1.5. The pattern of "add another probe when the last one failed" is what put us here.
5. **No fixes in investigation phases.** Phases 0–3 add instrumentation, audits, and evidence. Code fixes happen only in Phase 4, after a phase gate has named the root cause.
6. **Re-verify memory entries before acting on them.** The memory note "fix needs is_active exposed to the trait call site" predates `b5dad89b`. Confirm the current state in code before treating any memory as fact.
7. **Commits are checkpoints.** Commit after each instrumentation pass with a `debug:` prefix and a one-line description of what evidence the commit produces. Revert commits cleanly when removing instrumentation.

---

## Phase 0: Repro & Tooling

**Goal:** Capture a deterministic repro, prepare ASan capability, ensure baseline is clean.

### Task 0.1: Capture the repro as a script

**Files:**
- Create: `scripts/repro_isactive_panic.sh`

- [ ] **Step 1: Document the user steps.** Open the file manager, zoom in on a directory entry, then zoom out. The crash is in `ControlPanelBridge::Cycle` at `emMainWindow.rs:880`.

- [ ] **Step 2: Write a wrapper script that builds and launches the binary with `RUST_BACKTRACE=1`** and prints repro instructions to stderr. The user still drives the GUI; the script just standardizes the build flags and capture path.

```bash
#!/usr/bin/env bash
set -euo pipefail
cargo build --bin eaglemode 2>&1 | tee target/repro-build.log
echo "=== REPRO INSTRUCTIONS ===" >&2
echo "1. File manager opens by default" >&2
echo "2. Click into a directory entry (zoom in)" >&2
echo "3. Click out (zoom out)" >&2
echo "4. Crash should appear" >&2
RUST_BACKTRACE=1 exec target/debug/eaglemode 2>&1 | tee target/repro-run.log
```

- [ ] **Step 3: Run the script, perform the repro, confirm the panic still fires at HEAD** (post `b5dad89b`). Save output to `target/repro-baseline.log`.

- [ ] **Step 4: Commit.**

```bash
git add scripts/repro_isactive_panic.sh
git commit -m "debug(isactive): standardize repro script for SlotMap panic"
```

**Exit criterion:** Panic reproduces deterministically. If it doesn't, STOP — the IsActive guard at `b5dad89b` may have been sufficient and we need to revisit the problem statement.

### Task 0.2: Install nightly toolchain for ASan

- [ ] **Step 1: Confirm current toolchain is stable-only.**

```bash
rustup toolchain list
```

Expected: only `stable-x86_64-unknown-linux-gnu`.

- [ ] **Step 2: Install nightly with rust-src (required for `-Zsanitizer`).**

```bash
rustup toolchain install nightly --component rust-src
```

- [ ] **Step 3: Verify ASan can link a hello-world.**

```bash
cd /tmp && cargo new --bin asan_smoke && cd asan_smoke
RUSTFLAGS="-Zsanitizer=address" cargo +nightly build --target x86_64-unknown-linux-gnu
./target/x86_64-unknown-linux-gnu/debug/asan_smoke
```

Expected: builds and runs cleanly. If link fails, document the failure mode in `docs/debug/investigations/isactive-panic-facts.md` under a new "Tooling" section and STOP.

**Exit criterion:** Nightly + ASan produce a working binary on a smoke test. No commit needed; toolchain state is local.

---

## GATE 0 → Phase 1

Both Task 0.1 and Task 0.2 must pass. Specifically:
- Repro is deterministic at HEAD.
- ASan smoke test links and runs.

If either fails, STOP and report to the user before proceeding.

---

## Phase 1: Parallel Independent Probes

**Goal:** Run two cheap, independent probes simultaneously. Each discriminates a different hypothesis.

These are independent and may be done in parallel by separate subagents or in either order.

### Task 1A: Run repro under AddressSanitizer

**Files:**
- Modify (temporarily, will revert): root `Cargo.toml` if needed for sanitizer-friendly build profile.

- [ ] **Step 1: Build with ASan.**

```bash
RUSTFLAGS="-Zsanitizer=address" \
  cargo +nightly build \
  --target x86_64-unknown-linux-gnu \
  --bin eaglemode 2>&1 | tee target/asan-build.log
```

If wgpu/winit native deps fail to build under ASan, capture the error and document under "Tooling" in the facts file. Try `--target-dir target/asan` and `RUSTFLAGS="-Zsanitizer=address -Cforce-frame-pointers=yes"` once. If still failing, STOP — ASan is not viable for this binary; skip to Task 1B and Phase 2.

- [ ] **Step 2: Run the repro under ASan.**

```bash
ASAN_OPTIONS="abort_on_error=1:detect_leaks=0:symbolize=1" \
RUST_BACKTRACE=1 \
./target/x86_64-unknown-linux-gnu/debug/eaglemode 2>&1 | tee target/asan-run.log
```

Perform the same zoom-in/zoom-out repro.

- [ ] **Step 3: Capture findings.** Look for ASan reports of `use-after-free`, `heap-buffer-overflow`, `stack-use-after-return`, or `addresses initialization`. Note the function name and source line of the first ASan report (not the panic — the panic is downstream).

- [ ] **Step 4: Append evidence to facts file.**

Edit `docs/debug/investigations/isactive-panic-facts.md` — add a section `## ASan Run` with:
- Whether ASan reported any error before the panic.
- The exact ASan message and source location, if any.
- Whether the panic still occurred (yes/no).
- Classification: which hypothesis (H1 trait-object corruption / H2 PanelData corruption / H3 logic-only bug / Hnone) is supported, refuted, or untouched.

- [ ] **Step 5: Commit (no code changes if ASan ran cleanly; only the facts update).**

```bash
git add docs/debug/investigations/isactive-panic-facts.md
git commit -m "debug(isactive): record ASan run evidence"
```

### Task 1B: Add non-vtable behavior type tag

**Files:**
- Modify: `src/emCore/emPanelTree.rs` — add `behavior_type_tag: Option<&'static str>` field to `PanelData` (or wherever the `Box<dyn PanelBehavior>` lives alongside its metadata).
- Modify: the `set_behavior` function — accept a `&'static str` parameter and write it into the tag field at the same site as the box.
- Modify: every call site of `set_behavior` — pass a literal type name. Use `std::any::type_name::<T>()` only at call sites that are *generic over T* and known to be safe (this is name-only, no vtable). For non-generic call sites, pass a string literal.

- [ ] **Step 1: Locate `PanelData` and `set_behavior`.**

```bash
rg -n 'set_behavior' --type rust
rg -n 'behavior:.*Box<dyn' --type rust
```

- [ ] **Step 2: Add the tag field with a comment explaining it's a debug probe.**

```rust
// RUST_ONLY: dependency-forced-alternative — vtable-dispatched probes
// (behavior.type_name()) SIGSEGV on this branch; carry a POD type tag
// alongside the box so we can identify the resident behavior without
// touching the trait object. Remove once the panic is rooted.
behavior_type_tag: Option<&'static str>,
```

- [ ] **Step 3: Thread `&'static str` through `set_behavior` and every caller.** Compile must pass after each individual edit; do not bulk-edit.

- [ ] **Step 4: Add a probe inside `create_control_panel_in`** that, on the existing `assert!(in_target, ...)` audit path, prints `behavior_type_tag` for `cur` (the offending `PanelId`).

```rust
eprintln!(
    "[CCP_TAG] cur={:?} tag={:?} parent={:?} is_active={}",
    cur,
    panel_data.behavior_type_tag,
    panel_data.parent,
    panel_data.is_active,
);
```

This must read the tag *before* any vtable call; if the tag itself reads back garbled, that is itself the answer.

- [ ] **Step 5: Build, repro, capture output.**

```bash
cargo build --bin eaglemode 2>&1 | tee target/tag-build.log
./scripts/repro_isactive_panic.sh
```

- [ ] **Step 6: Append evidence to facts file** under `## Behavior Type Tag at PanelId(18v1)`. Record the tag value verbatim. Three outcomes to classify against:
  - Tag is a sane `&'static str` → trait-object/vtable is corrupted, `PanelData` is intact. Supports H1 (trait-object corruption).
  - Tag is `None` despite `has_behavior=true` → set/take ordering bug. Supports a new H4 (synchronization between `behavior` and tag fields).
  - Tag is non-None but unreadable / segfaults / contains a corrupt pointer → `PanelData` storage itself is corrupt. Supports H2.

- [ ] **Step 7: Commit instrumentation + evidence.**

```bash
git add src/emCore/emPanelTree.rs <other-changed-files> docs/debug/investigations/isactive-panic-facts.md
git commit -m "debug(isactive): add behavior_type_tag probe; record evidence"
```

---

## GATE 1 → Phase 2 OR Phase 4

After Phase 1, evaluate the combined evidence in the facts file. Branch:

- **If ASan named a specific frame (use-after-free, uninit read, etc.) AND the tag at PanelId(18v1) is consistent with that frame's culprit type:** root cause is named. Skip Phase 2 and Phase 3; go to Phase 4.
- **If the tag at PanelId(18v1) names a specific behavior type that we can map to one source file:** narrow Phase 2/3 to that one file; do not enumerate broadly.
- **If neither produced a named culprit but ASan ran cleanly and tag was sane:** trait object is intact, storage is intact — bug is at the logic layer. Proceed to Phase 2.
- **If ASan was not viable AND the tag probe produced no signal:** STOP. Report to user. Do not improvise.

---

## Phase 2: Enumerate and Instrument All `CreateControlPanel` Impls

**Run only if Phase 1 did not pinpoint a culprit.**

**Goal:** Find every implementation of `CreateControlPanel`. The fact that emDirPanel/emDirEntryPanel/emTestPanel didn't fire means the resident behavior at `PanelId(18v1)` is a different type. Identify it by exhaustion.

### Task 2.1: Enumerate every `CreateControlPanel` implementor

**Files:**
- Read-only audit, then targeted edits to each implementor's `CreateControlPanel`.

- [ ] **Step 1: Find every `CreateControlPanel` impl in the Rust tree.**

```bash
rg -n 'fn CreateControlPanel' --type rust
rg -n 'fn create_control_panel' --type rust
```

- [ ] **Step 2: List them in the facts file** under `## CreateControlPanel Implementors`. For each, note:
  - File path
  - Type that implements it
  - Whether it is already instrumented with `[CCP_IMPL]` eprintln
  - Whether the impl contains any `unsafe`, `transmute`, `MaybeUninit`, `from_raw`, or `mem::zeroed`

```bash
rg -n 'unsafe|transmute|MaybeUninit|from_raw|mem::zeroed' --type rust src/emCore/
```

- [ ] **Step 3: Add a top-of-function `eprintln!("[CCP_IMPL] <TypeName>")` to every implementor not already instrumented.** Use string literals for the type name; do not call `type_name::<Self>()` (still safer than vtable, but redundant when you know the type at the file).

- [ ] **Step 4: Build, repro, capture output.**

```bash
cargo build --bin eaglemode 2>&1 | tee target/ccp-impl-build.log
./scripts/repro_isactive_panic.sh
```

- [ ] **Step 5: Identify the resident behavior at PanelId(18v1).** From the eprintln output during the failing call, the last `[CCP_IMPL]` emitted before the audit assert names the type.

- [ ] **Step 6: Append to facts file** under `## Behavior Type at PanelId(18v1)`: the named type and the file path.

- [ ] **Step 7: Commit.**

```bash
git add -p
git commit -m "debug(isactive): instrument all CreateControlPanel impls; identify behavior at PanelId(18v1)"
```

**Exit criterion:** A specific Rust type and source file is named.

---

## GATE 2 → Phase 3

If Phase 2 identified the type, proceed. If no `[CCP_IMPL]` fired during the repro despite covering every implementor — that is a **major finding**: the behavior at `PanelId(18v1)` is constructed via a path that does not call any known `CreateControlPanel`, which means either (a) the `behavior` slot was overwritten by something unrelated (memory corruption — return to ASan), or (b) the box holds a behavior whose `CreateControlPanel` is the trait's default impl. Re-classify and STOP for user input.

---

## Phase 3: Audit the Suspect Implementation

**Goal:** Read the named implementation closely, comparing line-by-line against C++ `emFoo::CreateControlPanel` in `~/Projects/eaglemode-0.96.4/src/emCore/`. The bug is either a logic error in this impl, or an unsafe path that returns garbage bytes.

### Task 3.1: Side-by-side C++/Rust audit

**Files:**
- Read: `src/emCore/<NamedFile>.rs` (suspect impl)
- Read: `~/Projects/eaglemode-0.96.4/src/emCore/<NamedFile>.cpp` and `.h`
- Create: `docs/debug/investigations/<named-type>-ccp-audit.md`

- [ ] **Step 1: Open both files.** Read every line of the Rust `CreateControlPanel` impl and the C++ counterpart.

- [ ] **Step 2: Document divergences.** For each Rust line that does not 1:1 match C++:
  - Does it have a `DIVERGED:` annotation?
  - Is the divergence forced per Port Ideology, or is it drift?
  - Does it touch the return value or any path that produces a `PanelId`?

- [ ] **Step 3: Specifically check for:**
  - `IsActive()` guard (per memory: emDirPanel/emDirEntryPanel got this fixed at `b5dad89b` — this type may have the same gap).
  - Returns in early-exit paths that may yield uninitialized `Option<PanelId>`.
  - `unsafe` blocks, `transmute`, `MaybeUninit::assume_init`, `mem::zeroed::<Option<PanelId>>`, raw `from_raw`.
  - Reads from `PanelData::is_active` (subject to RawVisit desync — see facts file `## is_active Desync`).

- [ ] **Step 4: Write the audit document.** Lead with: hypothesis (one sentence), evidence supporting it (file:line), C++ behavior at the same point.

- [ ] **Step 5: Append summary to facts file** under `## Audit of <TypeName>::CreateControlPanel`.

- [ ] **Step 6: Commit.**

```bash
git add docs/debug/investigations/<named-type>-ccp-audit.md \
        docs/debug/investigations/isactive-panic-facts.md
git commit -m "debug(isactive): audit <TypeName>::CreateControlPanel against C++"
```

---

## GATE 3 → Phase 4

The audit must produce a **named root cause**: a specific line, a specific divergence, a specific defect. If the audit cannot name one, the bug is not in this impl — return to GATE 2 and re-classify.

Acceptable root-cause statements:
- "`<TypeName>::CreateControlPanel` lacks the `IsActive()` guard, returns a `PanelId` from `target_tree` even when self is not the active panel."
- "`<TypeName>::CreateControlPanel` reads `self.is_active` (stale via RawVisit) and returns a child ID from a stale walk."
- "`<TypeName>::CreateControlPanel` contains `unsafe { mem::zeroed() }` on path X, producing the observed garbage `PanelId`."

Unacceptable:
- "Probably similar to the emDirPanel issue."
- "Looks like it might be the IsActive thing."

---

## Phase 4: Fix

**Goal:** One fix, smallest possible, addressing the named root cause. No "while we're here." No bundled refactors.

### Task 4.1: Capture the failing case as a regression test

**Files:**
- Create or modify: an integration test in `tests/` that reproduces the panic at the API level (not the GUI level).

- [ ] **Step 1: Identify the smallest API-level reproduction.** The crash is in `ControlPanelBridge::Cycle` calling `PanelTree::remove` with a key produced inside `create_control_panel_in`. A test should construct the relevant tree, invoke `create_control_panel_in` with the conditions that trigger the bug, and assert the returned `PanelId` is in `target_tree` (or `None`) — never garbage.

- [ ] **Step 2: Write the failing test.** It must fail at HEAD before the fix.

- [ ] **Step 3: Run it. Confirm failure.**

```bash
cargo test --test <suite> <test_name> -- --nocapture
```

- [ ] **Step 4: Commit the failing test alone.**

```bash
git add tests/<file>.rs
git commit -m "test(isactive): regression test for CreateControlPanel returning out-of-tree PanelId"
```

If no API-level repro is feasible (test requires GUI driver), document why in the commit message and proceed without it. Do not skip silently.

### Task 4.2: Apply the fix

**Files:**
- Modify: the file named at GATE 3.

- [ ] **Step 1: Apply the minimum diff that addresses the named root cause.** Match the C++ behavior exactly. Add a `DIVERGED:` annotation only if the fix is itself a forced divergence; otherwise, the fix should bring Rust *closer* to C++.

- [ ] **Step 2: Run the regression test.**

```bash
cargo test --test <suite> <test_name>
```

Expected: PASS.

- [ ] **Step 3: Run full pre-commit suite.**

```bash
cargo fmt
cargo clippy -- -D warnings
cargo-nextest ntr
```

- [ ] **Step 4: Run the GUI repro.**

```bash
./scripts/repro_isactive_panic.sh
```

Perform zoom-in/zoom-out. Expected: no panic.

- [ ] **Step 5: Commit.**

```bash
git add -p
git commit -m "fix(<TypeName>): <one-line root cause description>"
```

### Task 4.3: Remove probes

**Files:**
- The behavior_type_tag field added in Task 1B.
- The `[CCP_IMPL]` eprintlns added in Task 2.1.
- Audit asserts in `create_control_panel_in` (the `assert!(in_target, ...)` from prior runs).

- [ ] **Step 1: Decide what to keep.** The `assert!(in_target, ...)` audit is cheap and correctness-affirming — keep it as a permanent invariant assert. The eprintlns and the `behavior_type_tag` field are debug-only — remove them.

- [ ] **Step 2: Remove eprintln probes from every `CreateControlPanel` impl.**

- [ ] **Step 3: Remove `behavior_type_tag` field and its threading.**

- [ ] **Step 4: Run full test suite.**

```bash
cargo fmt && cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 5: Run repro one more time** to confirm probes were not load-bearing.

- [ ] **Step 6: Commit.**

```bash
git add -p
git commit -m "chore(isactive): remove debug probes; keep in_target audit assert"
```

---

## Phase 5: Document and Close

### Task 5.1: Update facts file to a final state

- [ ] **Step 1: Add `## Resolution` section** to `docs/debug/investigations/isactive-panic-facts.md` summarizing:
  - Named root cause (one sentence).
  - Commit hash of the fix.
  - Which earlier hypotheses turned out to be wrong (so future debuggers don't re-walk them).
  - Whether the `is_active` desync at `RawVisit` was load-bearing or a separate concern (this matters: if it was *separate*, file a follow-up issue).

- [ ] **Step 2: If `is_active` desync remains a separate latent issue,** open a follow-up entry in `docs/debug/ISSUES.json` (matching whatever existing schema) and reference the facts file.

- [ ] **Step 3: Update memory entry `project_isactive_bug.md`** to point at the resolution commit, or remove if no longer relevant.

- [ ] **Step 4: Final commit.**

```bash
git add docs/debug/investigations/isactive-panic-facts.md docs/debug/ISSUES.json
git commit -m "docs(isactive): record root cause resolution"
```

---

## Self-Review Checklist (run before executing)

- [ ] Every phase has an exit criterion.
- [ ] Every gate has explicit branch logic with a STOP option.
- [ ] No phase mixes investigation with fixing.
- [ ] No instrumentation step relies on vtable dispatch.
- [ ] The fix phase requires a regression test before the fix lands.
- [ ] Probes added during investigation are explicitly removed in Phase 4.3.
- [ ] Memory entries about this bug are re-verified, not blindly trusted.
