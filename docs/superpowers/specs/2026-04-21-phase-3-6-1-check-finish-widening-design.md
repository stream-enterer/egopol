# Phase 3.6.1 — Widen `DialogCheckFinishCb` to match `DialogCycleExt`; close the P3 divergence

## Context

Phase 3.6 shipped with an authorized P3 divergence: `emFileDialog`'s
`on_cycle_ext` closure sets `dlg.pending_result = Some(Ok)` directly on
`fsb.file_trigger_signal` fire, **without** re-entering `CheckFinish` for
validation. Reason: `DialogCheckFinishCb`'s signature is
`FnMut(&DialogResult) -> bool` — too narrow for `emFileDialog`'s validation,
which needs to read `fsb` state (lives in a child panel in the tree) and
spawn an overwrite-confirmation dialog on Save-mode conflicts (needs
`&mut EngineCtx` for scheduler access + `pending_actions` push).

The sibling callback `DialogCycleExt` (added in Phase 3.6 Task 2) has the
right shape: `FnMut(&mut DlgPanel, &mut EngineCtx<'_>) -> bool`. The engine
calls it via a swap-out (take/call/put) pattern at `emDialog.rs:963-977` to
avoid double-borrow of `dlg_panel`.

**This phase widens `DialogCheckFinishCb` to mirror `DialogCycleExt`.**
`emFileDialog` then installs a full validation closure that matches C++
`emFileDialog::CheckFinish` (emFileDialog.cpp:110-185). The on_cycle_ext
closure's file-trigger-path simplification stays — the validation happens
in step-3 `on_check_finish` regardless of whether `pending_result` was set
by file-trigger or button-click. Single funnel, matches C++.

## Goal

1. Widen `DialogCheckFinishCb` to `FnMut(&DialogResult, &mut DlgPanel, &mut EngineCtx<'_>) -> bool`.
2. Apply swap-out pattern at the existing call site in `DialogPrivateEngine::Cycle` step 3 (`emDialog.rs:890`).
3. Migrate all callers of `set_on_check_finish` to the wider signature.
4. Install `emFileDialog`'s full validation closure ported from C++ `emFileDialog::CheckFinish`:
   - Read fsb state via `ctx.tree.take_behavior(fsb_panel_id)` → `as_file_selection_box_mut` → read selection + parent + filters → put back.
   - Reject on directory-result when `dir_allowed=false`.
   - Open mode: reject if any selected file missing.
   - Save mode: on existing files + `text != overwrite_confirmed`, spawn OD via the same `emFileDialog::CheckFinish` spawn path (refactor shared helper); return false (veto).
5. Remove the P3 DIVERGED marker from `emFileDialog::on_cycle_ext` — validation is now in the funnel.
6. Retain `emFileDialog::CheckFinish(ctx, result)` as a public API (tests still call it directly).

## Non-goals

- **E040 (POSITIVE overwrite-confirm end-to-end test)** — deferred. The test requires installing outer emFileDialog + transient OD as two headless top-level windows, but `winit::window::WindowId::dummy()` returns a single fixed id (no public multi-id constructor). `App.windows: HashMap<WindowId, emWindow>` collides on second insert. Resolution requires either wrapping `WindowId` in a repo-local newtype with test-id support, or switching `App.windows`'s key to `enum WindowKey { Real(WindowId), Headless(u64) }`. Both are invasive. Out of scope for 3.6.1; memory + raw-material JSON updated to reflect the winit-level blocker.
- No changes to winit integration, emWindow, or App.windows keying.
- No new tests beyond the one validating the widened callback fires correctly.

## Architecture

### Type alias

Before:
```rust
type DialogCheckFinishCb = Box<dyn FnMut(&DialogResult) -> bool>;
```

After:
```rust
pub(crate) type DialogCheckFinishCb =
    Box<dyn FnMut(&DialogResult, &mut DlgPanel, &mut crate::emEngineCtx::EngineCtx<'_>) -> bool>;
```

Visibility flips to `pub(crate)` (matches `DialogCycleExt`).

### Engine call site — swap-out

At `emDialog.rs:890-894`:

Before:
```rust
let vetoed = if let Some(cb) = dlg.on_check_finish.as_mut() {
    !cb(&pending)
} else {
    false
};
```

After (mirrors `on_cycle_ext` at :973-977):
```rust
let vetoed = if let Some(mut cb) = dlg.on_check_finish.take() {
    let vetoed = !cb(&pending, dlg, ctx);
    dlg.on_check_finish = Some(cb);
    vetoed
} else {
    false
};
```

### emFileDialog validation closure

`emFileDialog::new` (post-Task 3) currently wires `on_cycle_ext`. After
this phase, it also wires `on_check_finish`. The check-finish closure
captures: `fsb_panel_id: PanelId`, `mode: FileDialogMode`, `dir_allowed: bool`,
`look: Rc<emLook>`, and the outer dialog id / root panel id (for OD spawn).

Closure body ports `emFileDialog::CheckFinish(ctx, result)`'s Save-mode +
Open-mode + dir-check paths verbatim. On spawn-OD decision, it uses `ctx`
to construct the OD, subscribe outer's private engine to OD's finish_signal,
and park OD on `dlg.overwrite_dialog` + `dlg.overwrite_asked`. Returns
`false` (veto) on any error/dir/overwrite-needed path.

The existing `emFileDialog::CheckFinish(ctx, result) -> FileDialogCheckResult`
public method is retained — tests call it directly. Internal body is
deduped with the closure via a free function `file_dialog_check_finish(...)`
that both call sites invoke; the method wraps the free fn for the external
API, the closure wraps it for the funnel.

### on_cycle_ext simplification

The file-trigger branch of `on_cycle_ext` becomes a trivial one-liner
setting `dlg.pending_result = Some(Ok)`. The P3 DIVERGED comment
acknowledging validation-skip is removed — base cycle step 3 now re-enters
validation via the widened `on_check_finish`.

The OD-finish observation branch is unchanged from Phase 3.6 (still reads
od.finalized_result via deferred `pending_actions` push + `mutate_dialog_by_id`;
promotes asked→confirmed on POSITIVE; clears on NEGATIVE).

## Caller migration

`rg -n 'set_on_check_finish|on_check_finish: Some' crates/` results (pre-phase):
- `emDialog.rs:329` — `pub fn set_on_check_finish(&mut self, cb: DialogCheckFinishCb)` (setter; untouched, just takes the new type).
- `emDialog.rs:1829 (set_on_check_finish_after_show_panics)` — uses `Box::new(|_r| true)`; migrate to `Box::new(|_r, _dlg, _ctx| true)`.
- `emDialog.rs:1813 (set_on_check_finish_stores_callback)` — same migration.
- `emDialog.rs:2119 (veto test)` — `Box::new(move |_r| ... ; false)`; migrate to `Box::new(move |_r, _dlg, _ctx| ... ; false)`.

emFileDialog currently does NOT install `on_check_finish` (P3 divergence); this phase adds it.

## Tests

- Existing tests: migrate closure signatures (add two ignored args). Behavior unchanged.
- One new test: assert the widened callback receives the DlgPanel + EngineCtx args and can mutate DlgPanel state observable from the base cycle path. E.g., closure sets `dlg.some_field` and returns `false`; assertion reads the field post-cycle.
- Post-phase E040 entry should note that the POSITIVE end-to-end test will now exercise the full validation funnel (not just file-trigger fast-path) when the winit-WindowId infra lands.

## Gate

- nextest: expect 2512 + 1 new test (widened callback receive) = 2513/0/9.
- clippy --all-targets --all-features -D warnings green.
- goldens 237/6 preserved.

## Commit structure

Two tasks, two commits each (code + ledger) following Phase 3.6 cadence:

- **Task 1** — Widen the type alias + callers. Migrate `DialogPrivateEngine::Cycle` step 3 to swap-out pattern. Migrate existing test call sites (no-op signature change). Add one test asserting the widened args are reachable. No `emFileDialog` changes yet. Gate green.
- **Task 2** — Install `emFileDialog`'s validation closure. Extract shared `file_dialog_check_finish` free fn. Simplify `on_cycle_ext` file-trigger branch; remove the P3 DIVERGED marker. Retain `emFileDialog::CheckFinish` public method as a wrapper. Gate green.
- **Closeout** — Tag `port-rewrite-phase-3-6-1-complete`; merge to main (ask user first, per repo convention).

## Invariants

- I5b (single `DialogPrivateEngine`) — preserved.
- I5d (no caller-invoked Cycle) — preserved.
- No new `Rc<RefCell>`, `Arc`, `Mutex`, `Cow`, `Any`.
- No new `unsafe`.
- No `#[allow]` outside whitelist.
- File and Name Correspondence preserved — `DialogCheckFinishCb`'s widening is a Rust-idiom concession to tree-access + OD-spawn needs; carry a `DIVERGED:` comment at the type alias explaining why the callback can't match C++ `CheckFinish`'s no-arg closure shape.

## Risks

- emFileDialog's validation closure captures `Rc<emLook>` + uses `ctx` for `emDialog::new` calls. The closure is `'static + FnMut`; captures must be owned + Copy or owned-with-Clone. Look is `Rc<emLook>` — clonable, no issue.
- Spawn-from-within-closure couples OD construction timing with the engine's Cycle step 3 (mid-iteration). Construction enqueues via `pending_actions` — drained after Cycle returns. Observable timing: OD becomes visible one slice after CheckFinish veto, matching C++ order (emFileDialog.cpp:134-150 where OD is constructed inline — Rust defers install but spawn decision is in the same Cycle).
- Swap-out pattern at step 3 releases `on_check_finish` mid-call. If the callback re-entrantly calls `set_on_check_finish`, the put-back clobbers. Document as "no re-entrant set" contract; matches existing `on_cycle_ext` behavior.
