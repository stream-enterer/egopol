# TextField Cycle Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the per-frame active-panel `InvalidatePainting` workaround with a proper port of C++ `emTextField::Cycle`, eliminating the 99% main-thread CPU pin at idle.

**Architecture:** `TextFieldPanel` implements `PanelBehavior::Cycle` to drive cursor blink. `cycle_blink` returns `(flipped, busy)`; on flip the panel sets a single-bit invalidation request on `PanelCtx`, which `PanelCycleEngine` consumes after the call to invalidate via `PanelScope::resolve_view`. The old per-frame `InvalidatePainting` block in `emGUIFramework.rs` is deleted.

**Tech Stack:** Rust, existing `PanelCycleEngine`/`PanelCtx`/`PanelScope` infrastructure, `cargo-nextest`.

**Spec:** `docs/superpowers/specs/2026-05-03-textfield-cycle-port-design.md`

**Branch:** `fix/hang-2026-05-03-textfield-cycle` off `main`. **Do NOT branch off `instr/hang-2026-05-02`** — Phase E verifies on a clean branch with no instrumentation.

---

## File Map

- **Create:** none
- **Modify:**
  - `crates/emcore/src/emTextField.rs` — `cycle_blink` return type + unit test
  - `crates/emcore/src/emEngineCtx.rs` — add invalidate-self request bit on `PanelCtx`
  - `crates/emcore/src/emPanelCycleEngine.rs` — consume request post-Cycle, resolve view, invalidate
  - `crates/emcore/src/emColorFieldFieldPanel.rs` — `TextFieldPanel::Cycle`, remove cycle_blink from Paint, wake on focus
  - `crates/emcore/src/emGUIFramework.rs` — delete per-frame block (lines 1473-1497)
  - `crates/emtest/src/emTestPanel.rs` — same refactor as TextFieldPanel
  - `crates/eaglemode/tests/golden/composition.rs` — same refactor
  - `crates/eaglemode/tests/golden/test_panel.rs` — same refactor

---

### Task 1: Change `cycle_blink` return type to `CycleBlinkResult`

**Files:**
- Modify: `crates/emcore/src/emTextField.rs:2335-2355` (cycle_blink), `crates/emcore/src/emTextField.rs:3825-3840` (existing unit test)

- [ ] **Step 1: Update unit test for new return type (failing)**

In `crates/emcore/src/emTextField.rs`, replace the existing `cursor_blink_cycle` test (around line 3825) with:

```rust
#[test]
fn cursor_blink_cycle() {
    let mut tf = test_textfield();
    tf.cursor_blink_time = std::time::Instant::now() - std::time::Duration::from_millis(600);
    assert!(tf.IsCursorBlinkOn()); // initial state
    let r = tf.cycle_blink(true);
    assert!(r.flipped, "after 500ms, blink state should flip");
    assert!(r.busy, "focused TextField is busy");
    assert!(!tf.IsCursorBlinkOn(), "blink off after first 500ms boundary");
    let r2 = tf.cycle_blink(true);
    assert!(!r2.flipped, "no flip on the same boundary");
    assert!(r2.busy);
    let r3 = tf.cycle_blink(false);
    assert!(r3.flipped, "leaving focus restores blink-on, that's a flip");
    assert!(!r3.busy, "unfocused TextField is not busy");
    assert!(tf.IsCursorBlinkOn());
}
```

(`test_textfield()` is the existing helper used elsewhere in the file's test module — re-use it.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib cursor_blink_cycle`
Expected: compile error — `r.flipped` / `r.busy` don't exist on `bool`.

- [ ] **Step 3: Add `CycleBlinkResult` and update `cycle_blink`**

In `crates/emcore/src/emTextField.rs`, replace the existing `cycle_blink` (lines 2335-2355) with:

```rust
/// Result of one `cycle_blink` call.
///
/// `flipped` is true when `cursor_blink_on` actually changed during this
/// call — the caller should call `InvalidatePainting` only when this is set.
/// `busy` is true while focused, mirroring C++ `emTextField::Cycle`'s return
/// value (`busy=true` keeps the engine awake).
pub struct CycleBlinkResult {
    pub flipped: bool,
    pub busy: bool,
}

/// Toggles cursor blink state based on elapsed time. Should be called
/// from `TextFieldPanel::Cycle`. Returns `(flipped, busy)`. `focused`
/// indicates whether this text field is in the focused path. Matches C++
/// `emTextField::Cycle` blink logic (emTextField.cpp:306-340).
pub fn cycle_blink(&mut self, focused: bool) -> CycleBlinkResult {
    if focused {
        let now = std::time::Instant::now();
        let elapsed_ms = now.duration_since(self.cursor_blink_time).as_millis();
        if elapsed_ms >= 1000 {
            self.cursor_blink_time = now;
            let was_off = !self.cursor_blink_on;
            self.cursor_blink_on = true;
            return CycleBlinkResult { flipped: was_off, busy: true };
        }
        if elapsed_ms >= 500 {
            let was_on = self.cursor_blink_on;
            self.cursor_blink_on = false;
            return CycleBlinkResult { flipped: was_on, busy: true };
        }
        CycleBlinkResult { flipped: false, busy: true }
    } else {
        self.cursor_blink_time = std::time::Instant::now();
        let was_off = !self.cursor_blink_on;
        self.cursor_blink_on = true;
        CycleBlinkResult { flipped: was_off, busy: false }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p emcore --lib cursor_blink_cycle`
Expected: PASS.

- [ ] **Step 5: Build (callers will fail to compile — that's fine for this commit)**

Run: `cargo check -p emcore 2>&1 | head -20`
Expected: compile errors at the four `cycle_blink` callsites (`emColorFieldFieldPanel.rs:110`, `emTestPanel.rs:207`, `composition.rs:310`, `test_panel.rs:265`). They'll be fixed in later tasks. **Do not commit yet** — Tasks 2-5 must land together to avoid an intermediate broken build.

---

### Task 2: Add invalidate-self request to `PanelCtx`

**Files:**
- Modify: `crates/emcore/src/emEngineCtx.rs` (add field + methods to `PanelCtx`)

- [ ] **Step 1: Add field and methods to `PanelCtx`**

In `crates/emcore/src/emEngineCtx.rs`, in the `PanelCtx` struct definition (around line 585), add a new field at the end:

```rust
    /// Set by panel behaviors during `Cycle` to request that the panel's
    /// own painting be invalidated after Cycle returns. Drained by
    /// `PanelCycleEngine`, which resolves the view via `PanelScope` and
    /// calls `view.InvalidatePainting`. Mirrors C++ `emPanel::InvalidatePainting()`
    /// (no-arg form) — used by `emTextField::Cycle` for cursor blink repaints.
    invalidate_self_requested: bool,
```

In `PanelCtx::new` (around line 626), and every other `PanelCtx::*` constructor / `with_sched_reach` / etc. that sets fields explicitly, add `invalidate_self_requested: false,` to the struct literal.

In the `impl PanelCtx` block, add:

```rust
/// Request that this panel's painting be invalidated after `Cycle` returns.
/// Idempotent within a single Cycle call. Drained by `PanelCycleEngine`.
pub fn request_invalidate_self(&mut self) {
    self.invalidate_self_requested = true;
}

/// Drain the invalidate-self flag. Returns `true` if `request_invalidate_self`
/// was called since the last drain. Internal — called by `PanelCycleEngine`.
pub(crate) fn take_invalidate_self_request(&mut self) -> bool {
    let v = self.invalidate_self_requested;
    self.invalidate_self_requested = false;
    v
}
```

- [ ] **Step 2: Verify build passes**

Run: `cargo check -p emcore 2>&1 | grep -E '^error' | head -5`
Expected: no new errors beyond the pre-existing `cycle_blink` callsite errors from Task 1.

- [ ] **Step 3: Do not commit yet** — see Task 1 step 5.

---

### Task 3: Wire `PanelCycleEngine` to drain the request and invalidate

**Files:**
- Modify: `crates/emcore/src/emPanelCycleEngine.rs:80-223` (the `match self.scope` body inside `Cycle`)

- [ ] **Step 1: Drain the request on the Toplevel arm**

In `crates/emcore/src/emPanelCycleEngine.rs`, find the Toplevel arm (around line 80-132). After `let stay_awake = { ... behavior.Cycle(&mut ectx, &mut pctx) };` and before the `let ctx_tree = ctx.tree.as_deref_mut().expect(...)` line that puts the behavior back, capture whether the request flag was set. Restructure that block to read:

```rust
let (stay_awake, invalidate_requested) = {
    let mut ectx = crate::emEngineCtx::EngineCtx { /* ... unchanged ... */ };
    let pctx_tree = ctx
        .tree
        .as_deref_mut()
        .expect("PanelCycleEngine: tree is Some for Toplevel engines");
    let mut pctx = PanelCtx::with_sched_reach(
        pctx_tree,
        self.panel_id,
        tallness,
        unsafe { &mut *sched_ptr },
        unsafe { &mut *fw_ptr },
        ctx.root_context,
        ctx.framework_clipboard,
        ctx.pending_actions,
    );
    let busy = behavior.Cycle(&mut ectx, &mut pctx);
    let inval = pctx.take_invalidate_self_request();
    (busy, inval)
};
```

After `put_behavior` (around line 129), add the invalidation call:

```rust
if invalidate_requested {
    // Resolve the owning view and invalidate the panel's painting.
    // Mirrors emTextField.cpp:319/325/334 InvalidatePainting() — whole
    // panel, no-arg form.
    let panel_id = self.panel_id;
    self.scope.resolve_view(ctx, |view, sc| {
        view.InvalidatePainting(sc, panel_id);
    });
}
```

(Note: `view.InvalidatePainting` signature is `(sc, tree, panel_id)`. Check `emView.rs:3212` for the exact arg order. If the implementer finds the signature requires the tree, pass `&ctx.tree` after the borrow above is dropped — adapt at the callsite. The structural intent is unchanged: scope-resolve to view, invalidate this panel.)

- [ ] **Step 2: Drain the request on the SubView arm**

Apply the same restructure to the `PanelScope::SubView` arm (around line 133-222). The drain happens after the `behavior.Cycle` call and before `put_behavior`. The post-put invalidation block uses `self.scope.resolve_view(ctx, |view, sc| view.InvalidatePainting(sc, panel_id))` — which is identical to the Toplevel arm because `resolve_view` itself dispatches on `self.scope`.

To avoid duplication, factor the post-Cycle invalidation into a small helper after both arms — but only do this if it's simple. If borrow lifetimes prevent factoring, repeat the block in both arms. Don't add a free function for two callsites.

- [ ] **Step 3: Run check**

Run: `cargo check -p emcore 2>&1 | grep -E '^error' | head -5`
Expected: no new errors. Only the pre-existing `cycle_blink` callsite errors remain.

- [ ] **Step 4: Do not commit yet.**

---

### Task 4: Implement `TextFieldPanel::Cycle` and rewire focus

**Files:**
- Modify: `crates/emcore/src/emColorFieldFieldPanel.rs:100-120` (`TextFieldPanel` impl)

- [ ] **Step 1: Update imports**

At the top of `crates/emcore/src/emColorFieldFieldPanel.rs`, add to the imports:

```rust
use crate::emEngineCtx::EngineCtx;
```

- [ ] **Step 2: Move `cycle_blink` out of Paint, add Cycle, wake on focus**

Replace the entire `impl PanelBehavior for TextFieldPanel` block (lines 100-120) with:

```rust
impl PanelBehavior for TextFieldPanel {
    fn Paint(
        &mut self,
        painter: &mut emPainter,
        canvas_color: emColor,
        w: f64,
        h: f64,
        state: &PanelState,
    ) {
        let pixel_scale = state.viewed_rect.w * state.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        // cycle_blink moved to Cycle (mirrors C++ emTextField::Cycle, emTextField.cpp:306).
        self.text_field
            .Paint(painter, canvas_color, w, h, state.enabled, pixel_scale);
    }

    fn Cycle(&mut self, _ectx: &mut EngineCtx<'_>, pctx: &mut PanelCtx) -> bool {
        // Mirrors C++ emTextField::Cycle (emTextField.cpp:306-340):
        // - Read focus, advance blink state.
        // - On blink-state flip, InvalidatePainting (whole panel).
        // - Return busy=true while focused so the engine stays awake.
        let focused = pctx.tree.panel_state(pctx.id).in_focused_path();
        let r = self.text_field.cycle_blink(focused);
        if r.flipped {
            pctx.request_invalidate_self();
        }
        r.busy
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState, ctx: &mut PanelCtx) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.text_field.on_focus_changed(state.in_focused_path());
            // Mirrors C++ emTextField::Notice (emTextField.cpp:343-350):
            // RestartCursorBlinking() + WakeUp() so Cycle starts firing.
            self.text_field.RestartCursorBlinking();
            ctx.wake_up_panel(ctx.id);
        }
    }
}
```

(Notes for the implementer:
- `pctx.tree.panel_state(pctx.id)` — verify the exact API for reading a panel's `in_focused_path` from the tree. If `panel_state` doesn't exist by that name, find the existing accessor used elsewhere (grep `in_focused_path`). The intent is: read whether `pctx.id` is in the focused path. If the only available accessor is the `state: &PanelState` passed to `Paint`/`notice`, then alternatively store `is_focused` as a field on `TextFieldPanel`, set in `notice`, and read in `Cycle`.
- `ctx.wake_up_panel(ctx.id)` — confirm via grep on `wake_up_panel` (`emEngineCtx.rs:785`).
- `RestartCursorBlinking` exists at `emTextField.rs:2360`.)

- [ ] **Step 3: Run check**

Run: `cargo check -p emcore 2>&1 | grep -E '^error' | head -10`
Expected: errors only at the test-panel callsites of `cycle_blink` — no errors in `emColorFieldFieldPanel.rs`.

- [ ] **Step 4: Do not commit yet.**

---

### Task 5: Mirror the refactor to test panels

**Files:**
- Modify: `crates/emtest/src/emTestPanel.rs` (around line 207)
- Modify: `crates/eaglemode/tests/golden/composition.rs` (around line 310)
- Modify: `crates/eaglemode/tests/golden/test_panel.rs` (around line 265)

- [ ] **Step 1: For each of the three files, apply the same refactor as Task 4**

For each file, locate the `impl PanelBehavior for <whatever>TextFieldPanel>` block (the test panels mirror production). Replicate the changes:

1. Remove `self.widget.cycle_blink(s.in_focused_path());` from `Paint`.
2. Add a `Cycle` impl identical in shape to Task 4 step 2's `Cycle`, calling `self.widget.cycle_blink(focused)` and `pctx.request_invalidate_self()` on flip.
3. Add focus-change wakeup in `notice` (or extend the existing one).

Show the change once in this plan; the three test files are structurally identical. Use the Task 4 code block as the template, replacing `self.text_field` with `self.widget`.

- [ ] **Step 2: Build**

Run: `cargo check --workspace 2>&1 | grep -E '^error' | head -5`
Expected: no errors.

- [ ] **Step 3: Run unit tests**

Run: `cargo test -p emcore --lib cursor_blink_cycle`
Expected: PASS.

- [ ] **Step 4: Do not commit yet.**

---

### Task 6: Delete the per-frame workaround

**Files:**
- Modify: `crates/emcore/src/emGUIFramework.rs:1473-1497`

- [ ] **Step 1: Delete the per-frame InvalidatePainting block and its INVAL instrumentation**

In `crates/emcore/src/emGUIFramework.rs`, delete lines 1473-1497 (the comment starting at 1478 plus the `let active_id = win.view().GetActivePanel();` block plus the INVAL writeln). Keep the lines immediately before (`win.view_mut().collect_parent_invalidation(&mut tree);` at 1476) and after (`win.put_tree(tree);` at 1500).

The result should be:

```rust
            // Collect invalidation from sub-view panels (C++ invalidation chain:
            // SubViewClass::InvalidateTitle, SubViewPortClass::InvalidateCursor,
            // SubViewPortClass::InvalidatePainting → SuperPanel → parent view).
            win.view_mut().collect_parent_invalidation(&mut tree);

            // Phase 3.5.A Task 7: put the tree back on the window.
            win.put_tree(tree);
```

- [ ] **Step 2: Run full nextest**

Run: `cargo-nextest ntr`
Expected: all tests pass (existing 3003 + the updated cursor_blink_cycle).

- [ ] **Step 3: Run golden tests specifically**

Run: `cargo test --test golden -- --test-threads=1`
Expected: PASS for blink-related tests in `composition` and `test_panel`.

- [ ] **Step 4: Commit (single commit for the whole fix)**

```bash
git add crates/emcore/src/emTextField.rs \
        crates/emcore/src/emEngineCtx.rs \
        crates/emcore/src/emPanelCycleEngine.rs \
        crates/emcore/src/emColorFieldFieldPanel.rs \
        crates/emcore/src/emGUIFramework.rs \
        crates/emtest/src/emTestPanel.rs \
        crates/eaglemode/tests/golden/composition.rs \
        crates/eaglemode/tests/golden/test_panel.rs
git commit -m "$(cat <<'EOF'
fix(hang): port emTextField::Cycle, drop per-frame InvalidatePainting workaround

C++ emTextField::Cycle (emTextField.cpp:306-340) drives cursor-blink
repaint by calling InvalidatePainting() only on blink-state flip. The
Rust port had cycle_blink in Paint() with no flip-driven InvalidatePainting,
and emGUIFramework.rs:1478-1497 worked around the missing wiring by
unconditionally invalidating the active panel every winit about_to_wait.
At idle the active panel is the cosmos root, dirtying all 1024 tiles
each frame → 26 ms paint → 100 % main-thread CPU.

Spec: docs/superpowers/specs/2026-05-03-textfield-cycle-port-design.md

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Phase E verification gate

**Files:** none (verification only — produces no commit)

- [ ] **Step 1: Build release binary**

Run: `cargo build --release -p eaglemode`
Expected: success.

- [ ] **Step 2: Manual capture run**

The instrumentation lives on `instr/hang-2026-05-02`. To verify the fix, the user must:

1. From `instr/hang-2026-05-02`, run `git diff fix/hang-2026-05-03-textfield-cycle..instr/hang-2026-05-02 -- crates/emcore/src/emInstr.rs` to confirm instrumentation is still available there.
2. Cherry-pick or rebase the fix commit onto a temporary verify branch from `instr/hang-2026-05-02`, OR (preferred) merge `fix/hang-2026-05-03-textfield-cycle` into a verify branch and re-run capture.
3. Launch the binary, send `SIGUSR1` to mark the start of the idle window, wait ~20 s untouched, send `SIGUSR1` again, exit.
4. Run `python3 scripts/analyze_hang.py /tmp/em_instr.phase0.log`.

- [ ] **Step 3: Check pass criteria**

From the analyzer output, verify:

- `RENDER:paint` is < 5 % of the marker window's wall-clock (was 99.1 %).
- Avg paint per frame is < 1 ms (was 26.6 ms).
- No regression in `composition` / `test_panel` golden tests (re-run `cargo test --test golden -- --test-threads=1`).
- Manually verify cursor blink still works: focus a TextField in the running app, observe blink at ~500 ms cadence.

- [ ] **Step 4: Decide on `has_awake==1` followup**

If the analyzer reports `has_awake=1` on > 50 % of slices at idle, file a followup investigation (separate scope — identify which engine never sleeps). This is not a blocker for this fix.

- [ ] **Step 5: Abandon `instr/hang-2026-05-02` after Phase E pass**

After all gate criteria pass, the instrumentation branch can be deleted:

```bash
git branch -D instr/hang-2026-05-02
```

Do **not** delete it before Phase E pass — the instrumentation is needed for the verification.

---

## Self-review notes

- **Spec coverage:** all spec sections (Cycle port, focus wakeup, workaround deletion, test panels, Phase E gate, scope) have tasks. ✓
- **Type consistency:** `CycleBlinkResult { flipped, busy }` in Task 1 is referenced consistently in Tasks 4 and 5. `request_invalidate_self` / `take_invalidate_self_request` introduced in Task 2, used in Tasks 3 and 4. ✓
- **No placeholders:** all code blocks complete; the two API-uncertainty notes (`panel_state` accessor, `view.InvalidatePainting` arg order) flag specific things to verify with grep, not placeholders. The implementer can resolve them in 30 seconds with the existing codebase open.
- **Single commit:** Tasks 1-6 land as one commit (Task 6 step 4). Tasks 1-5 each say "do not commit yet". This matches the spec's "single commit, no instrumentation" requirement.
