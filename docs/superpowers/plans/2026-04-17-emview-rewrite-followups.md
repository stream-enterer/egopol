# emView Rewrite Followups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all 11 follow-up items from the emView viewing/geometry rewrite by aligning Rust to C++ — F&N renames, duplicate-field removal, backend wiring against existing winit/wgpu/scheduler infrastructure, and two test-gap closures.

**Architecture:** Eleven sequential phases ordered low-risk → high-risk: mechanical renames first (Phases 1–4), then backend wiring against existing infrastructure (Phases 5–9), then population of monitor data (Phase 10), then the visit-stack removal that touches animator math (Phase 11). Each phase is one or more commits gated by full-suite acceptance: `cargo check` clean, `cargo clippy -- -D warnings` clean, `cargo-nextest ntr` no test count regression, `cargo test --test golden -- --test-threads=1` ≥235 passes, pre-commit hook clean, runtime smoke ≥15s.

**Tech Stack:** Rust 2021, `winit = "0.30"`, `wgpu`, `bitflags`, `slotmap`. Project at `/home/a0/git/eaglemode-rs`. C++ reference at `/home/a0/git/eaglemode-0.96.4/`. Spec: `docs/superpowers/specs/2026-04-17-emview-rewrite-followups-design.md` (commit `9f149ba`). Source note: `docs/superpowers/notes/2026-04-17-emview-rewrite-followups.md`.

**Hard rules (every phase):**
- No `--no-verify`. Pre-commit must pass.
- No new `#[allow(...)]` / `#[expect(...)]` outside the F&N exceptions in `CLAUDE.md`.
- No new `DIVERGED:` annotations. If a phase touches an existing `DIVERGED:`, it must either delete the divergence or document why the rationale still holds.
- No `PHASE-N-TODO:` comments left behind by the phase that owns them.
- No `_old` / backwards-compat shims. Replace outright.
- One phase = one tight commit sequence on `main`. Phase boundaries are commit boundaries for `git bisect`.

**Acceptance baseline (commit `68c6c59`, the `9f149ba` parent on main):** 2403/2403 nextest pass, 235/243 golden, runtime smoke ALIVE ≥15s. Every phase must equal or beat this.

---

## Phase 1: Rename `svp_update_count` → `SVPUpdCount`

Pure mechanical field rename. F&N drift fix per spec §2.1 Item 2.

**Files:**
- Modify: `crates/emcore/src/emView.rs` (lines 286, 452, 1783, 1785–1787)

### Task 1.1: Rename the field declaration

- [ ] **Step 1: Read the field declaration in context**

Run: `sed -n '280,295p' crates/emcore/src/emView.rs`
Expected output includes:
```
    svp_update_count: u32,
```
at line 286, and `pub SVPUpdSlice: u64,` at line 388.

- [ ] **Step 2: Verify exhaustive list of readers/writers**

Run: `grep -n 'svp_update_count' crates/emcore/src/emView.rs`
Expected exactly 4 matches at lines 286, 452, 1783, 1785, 1787 (the 1785+1787 may share a line range — count the actual hits, must be ≥4).

- [ ] **Step 3: Apply the rename**

Use `sed` (single file, exact-match replace; `svp_update_count` is not a substring of any other token):
```bash
sed -i 's/\bsvp_update_count\b/SVPUpdCount/g' crates/emcore/src/emView.rs
```

- [ ] **Step 4: Verify zero residual hits**

Run: `grep -c 'svp_update_count' crates/`
Expected: `0` (use `grep -rc` if needed; should produce only zeros across all files).

Run: `grep -n 'SVPUpdCount' crates/emcore/src/emView.rs`
Expected: 4 matches at the same lines (286, 452, 1783, 1785, 1787).

- [ ] **Step 5: Phase gate**

Run, in order, all of:
```bash
cargo check
cargo clippy -- -D warnings
cargo-nextest ntr
cargo test --test golden -- --test-threads=1
```
Expected: all pass; nextest 2403/2403; golden ≥235.

- [ ] **Step 6: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode || code=$?; echo "exit=$code"
```
Expected: exits with code 124 (SIGTERM from timeout — meaning it stayed alive). Any panic / segfault fails the gate.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "$(cat <<'EOF'
refactor(emView): rename svp_update_count → SVPUpdCount (F&N)

Closes Phase 1 of emview-rewrite-followups. Field name now matches the
sibling SVPUpdSlice and the C++ name SVPUpdCount.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

The pre-commit hook re-runs fmt + clippy + nextest. If it fails, fix and re-commit (do not `--amend`, do not `--no-verify`).

---

## Phase 2: Remove `home_pixel_tallness` duplicate

Removes the Rust-invention `home_pixel_tallness` field on `emViewPort`; routes all readers through `emView.HomePixelTallness`. Spec §2.1 Item 3.

**Files:**
- Modify: `crates/emcore/src/emViewPort.rs` (field at line 55, params at lines 95/102, write at 153, helper `SetViewGeometry` at 152, `new_with_geometry` ctor at 90+)
- Modify: `crates/emcore/src/emView.rs` (lines 2928–2931 — `SwapViewPorts`)
- (Verify but likely no edits) `crates/emcore/src/emViewAnimator.rs` lines 1882–1883, 3501 — comment-only references; double-check.

### Task 2.1: Audit all readers before deletion

- [ ] **Step 1: Generate complete reader list**

```bash
grep -rn 'home_pixel_tallness' crates/ examples/
```
Expected hits (must verify each):
- `crates/emcore/src/emViewPort.rs` — field declaration (~55), parameter (~95), assignment (~102, 153)
- `crates/emcore/src/emView.rs` — read in `SwapViewPorts` (~2928–2931)
- `crates/emcore/src/emViewAnimator.rs` — comment-only (~1882–1883, 3501)

If hits exist anywhere else, **stop** and update this task — do not proceed until accounted for.

- [ ] **Step 2: Inspect the SwapViewPorts read site**

Run: `sed -n '2920,2940p' crates/emcore/src/emView.rs`
Expected: shows the conditional `if vp.home_pixel_tallness > 0.0 { vp.home_pixel_tallness } else { self.HomePixelTallness }`.

The fallback already references `self.HomePixelTallness`. After this phase, the conditional collapses to just `self.HomePixelTallness` (no field on the viewport).

### Task 2.2: Remove field and propagate

- [ ] **Step 1: Delete the field from `emViewPort` struct**

Edit `crates/emcore/src/emViewPort.rs:55`:
```rust
    pub home_pixel_tallness: f64,
```
→ delete that line entirely.

- [ ] **Step 2: Delete the parameter from `new_with_geometry`**

Read `crates/emcore/src/emViewPort.rs:90,110` for the full ctor signature. Remove the `home_pixel_tallness: f64,` parameter and the `home_pixel_tallness,` initializer line (around 102).

- [ ] **Step 3: Remove the write in `SetViewGeometry`**

Edit `crates/emcore/src/emViewPort.rs:153`:
```rust
        self.home_pixel_tallness = pixel_tallness;
```
→ delete entirely. The `pixel_tallness` parameter to `SetViewGeometry` becomes unused; rename to `_pixel_tallness` if the function still takes it for ABI-shape parity, otherwise drop the parameter entirely. **Default to dropping** — the project rule is to fix the cause, not the warning. Then update all callers of `SetViewGeometry` to not pass it.

  Run: `grep -rn 'SetViewGeometry' crates/` to find callers and update them.

- [ ] **Step 4: Update `SwapViewPorts` read in emView**

Edit `crates/emcore/src/emView.rs:2928–2931`:
```rust
        self.CurrentPixelTallness = if vp.home_pixel_tallness > 0.0 {
            vp.home_pixel_tallness
        } else {
            self.HomePixelTallness
        };
```
→ replace with:
```rust
        self.CurrentPixelTallness = self.HomePixelTallness;
```
(Both sides of the conditional held the same value because the viewport field was always set from `HomePixelTallness` upstream. This collapses to the only meaningful read.)

- [ ] **Step 5: Strip the cross-reference comment**

If a comment block above the deleted field referenced "C++ HomePixelTallness — also stored as home_pixel_tallness for compatibility" or similar, remove the cross-reference now-stale half. Keep any C++ provenance comment on `HomePixelTallness` itself in `emView.rs`.

- [ ] **Step 6: Verify zero residual hits in production code**

```bash
grep -rn 'home_pixel_tallness' crates/
```
Expected: only the comment-only references in `emViewAnimator.rs` lines 1882–1883, 3501. Those are documentation referring to the C++ name `HomePixelTallness`. **Update them too** — change "C++ HomePixelTallness — also stored as home_pixel_tallness …" to just reference `HomePixelTallness`.

After cleanup:
```bash
grep -c 'home_pixel_tallness' crates/
```
Expected: `0`.

- [ ] **Step 7: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: all pass; nextest 2403/2403 (the field's removal is observable to no test); golden ≥235.

- [ ] **Step 8: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124 (timeout, no crash).

- [ ] **Step 9: Commit**

```bash
git add crates/emcore/src/emViewPort.rs crates/emcore/src/emView.rs crates/emcore/src/emViewAnimator.rs
git commit -m "$(cat <<'EOF'
refactor(emViewPort): remove home_pixel_tallness duplicate (F&N)

Closes Phase 2 of emview-rewrite-followups. The Rust-invention duplicate
is gone; all readers now use emView::HomePixelTallness directly. C++ has
no per-port field; the parity is restored.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3: Remove `PanelTree::current_pixel_tallness`; thread through call chain

Field has no write path (always 1.0). Threads `view.CurrentPixelTallness` through `RawVisitAbs` and `emPanelCtx`. Spec §2.1 Item 4.

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs` (field at 303, init at 325, reads at 1168, 2474)
- Modify: `crates/emcore/src/emPanelCtx.rs` (read at 240)
- Modify: `crates/emcore/src/emView.rs` (call chain origin at 2151)

### Task 3.1: Audit all callers of the threaded functions

- [ ] **Step 1: Find all call sites of `RawVisitAbs`**

```bash
grep -rn 'RawVisitAbs' crates/
```
Record every call site. Each one needs an extra arg.

- [ ] **Step 2: Find all call sites of `UpdateChildrenViewing`**

```bash
grep -rn 'UpdateChildrenViewing' crates/
```
Record each.

- [ ] **Step 3: Find all `emPanelCtx` constructions**

Look at `crates/emcore/src/emPanelCtx.rs` for the `new`/`with_*` constructors. Then:
```bash
grep -rn 'emPanelCtx::' crates/
grep -rn 'PanelCtx::' crates/
```
Record every construction site — each needs a `current_pixel_tallness` arg.

- [ ] **Step 4: Find every read of `panel_to_view_y` (and any related panel-to-view conversions on `emPanelCtx`)**

```bash
grep -rn 'panel_to_view_y\|panel_to_view_x' crates/
```
These are the consumers; they will read from the new ctor argument transparently.

### Task 3.2: Plumb the parameter through

- [ ] **Step 1: Add the parameter to the layout function (line 1168 host)**

Identify the function containing line 1168 in `emPanelTree.rs`. Add `current_pixel_tallness: f64` to its signature. Replace the line:
```rust
let pt = self.current_pixel_tallness;
```
with:
```rust
let pt = current_pixel_tallness;
```

- [ ] **Step 2: Add the parameter to `UpdateChildrenViewing` (line 2474 host)**

Same pattern. Add `current_pixel_tallness: f64` parameter. Replace:
```rust
let pt = self.current_pixel_tallness;
```
with:
```rust
let pt = current_pixel_tallness;
```

- [ ] **Step 3: Update every call site recorded in Task 3.1**

For each call site, source the value from `view.CurrentPixelTallness` or thread it from the caller's parameter. The root call is at `crates/emcore/src/emView.rs:2151`:
```rust
tree.HandleNotice(self.window_focused, self.CurrentPixelTallness);
```
This already passes `CurrentPixelTallness` to `HandleNotice`. Inside `HandleNotice`, propagate it to `UpdateChildrenViewing` and `RawVisitAbs`.

- [ ] **Step 4: Update `emPanelCtx`**

Edit `crates/emcore/src/emPanelCtx.rs`. Add a `current_pixel_tallness: f64` field to the struct. Add it as a constructor parameter. Update line 240 (`panel_to_view_y`):
```rust
p.viewed_y + y * p.viewed_width / self.tree.current_pixel_tallness
```
→
```rust
p.viewed_y + y * p.viewed_width / self.current_pixel_tallness
```

Update every `emPanelCtx` construction site (from Task 3.1 Step 3) to pass `view.CurrentPixelTallness`.

- [ ] **Step 5: Delete the `PanelTree` field**

Edit `crates/emcore/src/emPanelTree.rs:303`:
```rust
pub(crate) current_pixel_tallness: f64,
```
→ delete.

Edit `crates/emcore/src/emPanelTree.rs:325`:
```rust
current_pixel_tallness: 1.0,
```
→ delete.

Delete the doc comment block above the field (lines 299–302) — it references storing the value here, no longer applicable.

- [ ] **Step 6: Verify zero residual `tree.current_pixel_tallness` reads**

```bash
grep -rn 'current_pixel_tallness' crates/
```
Expected: only references on `emPanelCtx`, on `emView`, and as a function parameter name. **No** `self.current_pixel_tallness` inside `PanelTree`. **No** `tree.current_pixel_tallness` anywhere.

- [ ] **Step 7: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: all pass. Because the value was always 1.0, golden output should be byte-identical (no test count regression).

- [ ] **Step 8: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124.

- [ ] **Step 9: Commit**

```bash
git add crates/emcore/src/emPanelTree.rs crates/emcore/src/emPanelCtx.rs crates/emcore/src/emView.rs
git commit -m "$(cat <<'EOF'
refactor(emPanelTree): remove current_pixel_tallness; thread from view (F&N)

Closes Phase 3 of emview-rewrite-followups. The PanelTree-side cache had
no write path (always 1.0). RawVisitAbs and UpdateChildrenViewing now
take current_pixel_tallness as a parameter; emPanelCtx stores it as a
field set at construction. Matches C++, where emPanel::Layout reads
View.CurrentPixelTallness directly through the View& reference.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 4: `ZuiWindow` → `emWindow` rename + popup-stub merge

Mechanical rename across 23 files plus a popup-stub merge into the unified type. Spec §2.1 Item 1.

**Files:**
- Modify: 23 files (full list captured in Task 4.1)
- Delete: the popup-stub `emWindow` struct in `crates/emcore/src/emWindow.rs:1422–1479`

### Task 4.1: Pre-rename audit

- [ ] **Step 1: Generate the canonical file list**

```bash
grep -rl 'ZuiWindow' crates/ examples/ > /tmp/zuiwindow-files.txt
wc -l /tmp/zuiwindow-files.txt
```
Expected: 23 files. Exact list (may reorder; count must equal 23):
```
crates/emcore/src/emEngine.rs
crates/emcore/src/emPriSchedAgent.rs
crates/emcore/src/emScheduler.rs
crates/emcore/src/emFileModel.rs
crates/emcore/src/emScreen.rs
crates/emcore/src/emWindowStateSaver.rs
crates/emcore/src/emGUIFramework.rs
crates/emcore/src/emWindow.rs
crates/eaglemode/tests/behavioral/pri_sched_agent.rs
crates/eaglemode/tests/behavioral/mini_ipc.rs
crates/eaglemode/tests/golden/scheduler.rs
crates/eaglemode/tests/unit/scheduler.rs
crates/eaglemode/tests/support/mod.rs
crates/eaglemode/tests/support/pipeline.rs
crates/emmain/src/emMainWindow.rs
examples/tree_expansion_demo.rs
examples/toolkit_demo.rs
examples/test_toolkit.rs
examples/test_pack_layout.rs
examples/test_panel.rs
examples/paint_demo.rs
examples/signal_timer_demo.rs
examples/input_demo.rs
examples/test_input.rs   (only if present)
```

- [ ] **Step 2: Verify the popup-stub `emWindow` is the only existing `emWindow`**

```bash
grep -n 'pub struct emWindow' crates/emcore/src/emWindow.rs
```
Expected: exactly one match at `:1422`. If more, abort and re-investigate.

### Task 4.2: Rename and merge

- [ ] **Step 1: First, delete the popup-stub `emWindow` type**

This makes the namespace free for the rename. Edit `crates/emcore/src/emWindow.rs`. Delete:
- the `pub struct emWindow { ... }` block at lines 1422–1435
- the `impl emWindow { ... }` block at lines 1437–1479

Also delete the imports above (`use std::cell::RefCell; use std::rc::Rc;` at 1419–1420) **only if** they are no longer used elsewhere in the file. Run `cargo check` after the deletion — if it complains about unused imports, remove those imports.

- [ ] **Step 2: Find the popup-stub call sites that this just broke**

```bash
cargo check 2>&1 | grep -E 'emWindow|new_popup|SetViewPosSize' | head -50
```
Record each broken site. The stub had two methods: `new_popup` (callers create popup windows) and `SetViewPosSize` (callers resize the popup). These call sites will be re-routed in Step 6.

- [ ] **Step 3: Mechanical rename `ZuiWindow` → `emWindow` in all 23 files**

```bash
xargs -a /tmp/zuiwindow-files.txt sed -i 's/\bZuiWindow\b/emWindow/g'
```

The `\b` boundaries prevent collisions with substrings (none currently in the codebase, but defensive).

- [ ] **Step 4: Verify zero residual `ZuiWindow` in code**

```bash
grep -rn 'ZuiWindow' crates/ examples/
```
Expected: zero hits in `crates/` and `examples/`.

```bash
grep -rn 'ZuiWindow' .
```
Expected: only hits in `target/` (cached build artifacts), `.workflow/` (run logs), `docs/superpowers/` (historical specs/plans/notes — historical record, not touched), `.kani/` (generated). All non-source. If hits appear in any source file, abort and clean.

- [ ] **Step 5: Add `with_decorations` to `emWindow::create` for popup support**

Edit `crates/emcore/src/emWindow.rs:72`. The function is already controlled by the `WindowFlags::UNDECORATED` flag at line 84–85:
```rust
if flags.contains(WindowFlags::UNDECORATED) {
    attrs = attrs.with_decorations(false);
}
```
This already covers undecorated windows. Add the `WindowLevel::Floating` hint when `WindowFlags::POPUP` is set. After the existing `UNDECORATED` block, add:
```rust
if flags.contains(WindowFlags::POPUP) {
    attrs = attrs.with_window_level(winit::window::WindowLevel::AlwaysOnTop);
}
```

(`AlwaysOnTop` is the closest winit 0.30 idiom matching X11 popup-window behavior. `Floating` is not a variant in winit 0.30 — the variants are `AlwaysOnBottom`, `Normal`, `AlwaysOnTop`. Verify with `grep -r 'enum WindowLevel' ~/.cargo/registry/src/`.)

- [ ] **Step 6: Re-route the broken `new_popup` call sites**

For each broken site identified in Step 2: call `emWindow::create` with `flags.contains(WindowFlags::POPUP | WindowFlags::UNDECORATED)`. The exact rewrite depends on context, but the pattern is:
```rust
// OLD: emWindow::new_popup(&view, flags, "emViewPopup")
emWindow::create(
    event_loop,
    gpu,
    root_panel,
    flags | WindowFlags::POPUP | WindowFlags::UNDECORATED,
    close_signal,
    flags_signal,
    focus_signal,
    geometry_signal,
)
```
The owner-View (formerly `_owner: &emView`) is now communicated through `root_panel` and the signals. If a call site cannot supply `event_loop` and `gpu`, it must be lifted to a place that can — this is C++ semantics (popup windows live in the same scheduler/context as the main window).

If no caller actually creates a popup yet (the popup machinery in `emView::SwapViewPorts` is what invokes it, lazily), defer the broken-site fix to Phase 6 by **leaving the popup constructor disabled** (keep an unimplemented path with `unimplemented!()` if it's a function body, or `// PHASE-6-TODO:` if it's a code path). However: ZERO `PHASE-N-TODO:` left from the *owning* phase. So if Phase 4 can't actually wire popups, **document the tradeoff explicitly** in the commit message and move the remaining popup logic into Phase 6 unchanged. Verify nothing user-visible regresses.

The most likely outcome: the `new_popup` and `SetViewPosSize` call sites in `emView.rs` (around the `SwapViewPorts` path) are guarded by code paths that don't run today (popup creation is currently dead code). Confirm with `cargo check` — if no errors, proceed. If errors, the call sites are live and must be reworked here.

- [ ] **Step 7: Verify single `pub struct emWindow` in `emWindow.rs`**

```bash
grep -n 'pub struct emWindow' crates/emcore/src/emWindow.rs
```
Expected: exactly one match (the renamed-from-`ZuiWindow` definition).

- [ ] **Step 8: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: all pass; 2403/2403 nextest; ≥235 golden.

- [ ] **Step 9: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124. The renamed type is the same code; no behavior change.

- [ ] **Step 10: Commit**

```bash
git add -A
git status   # Verify only expected files staged; abort if surprises
git commit -m "$(cat <<'EOF'
refactor(emWindow): rename ZuiWindow → emWindow; drop popup stub (F&N)

Closes Phase 4 of emview-rewrite-followups. The heavyweight window type
now matches its file and C++ name. The Phase-4 popup stub is gone;
emWindow::create now handles undecorated/popup windows via WindowFlags.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5: `emViewPort` 7-method backend wiring

Wire the seven `PHASE-5-TODO` dispatch points to the existing `emWindow` / scheduler / compositor machinery. Spec §2.2 Item 5 and §2.5c.

**Files:**
- Modify: `crates/emcore/src/emViewPort.rs` (methods at 129–217; add fields for cursor cache + dirty flags + Weak<emWindow> back-reference)
- Modify: `crates/emcore/src/emWindow.rs` (input dispatch routes through emViewPort; cursor read/apply on each frame; tile invalidation entry points)
- Modify: `crates/emcore/src/emView.rs` (input flow through view port; expose scheduler clock to view port)
- Create: `crates/eaglemode/tests/unit/input_dispatch_chain.rs` (new test verifying emWindow → emViewPort → emView routing)

### Task 5.1: Add Weak<emWindow> back-reference and cursor/dirty fields to emViewPort

- [ ] **Step 1: Read the current emViewPort struct**

Run: `sed -n '38,80p' crates/emcore/src/emViewPort.rs`
Expected: shows the struct with `home_x/y/width/height` and `focused`.

- [ ] **Step 2: Add fields**

Add to the `emViewPort` struct:
```rust
    /// Back-reference to the owning emWindow. Used by PaintView,
    /// InvalidateCursor, InvalidatePainting to dispatch to backend
    /// machinery. Weak to avoid Rc cycles.
    pub(crate) window: Option<std::rc::Weak<std::cell::RefCell<crate::emWindow::emWindow>>>,

    /// Cursor reported by the view; emWindow consumes it on each frame.
    /// (C++ stores this on emViewPort identically.)
    pub(crate) cursor: crate::emCursor::emCursor,

    /// Set by InvalidateCursor; emWindow consumes the flag on next frame.
    pub(crate) cursor_dirty: bool,

    /// Monotonic-millisecond clock value, set by emWindow on each input
    /// dispatch from the scheduler. Read by GetInputClockMS.
    pub(crate) input_clock_ms: u64,
```

Initialize in `new_dummy()` and `new_with_geometry()`:
```rust
    window: None,
    cursor: crate::emCursor::emCursor::Normal,
    cursor_dirty: false,
    input_clock_ms: 0,
```

### Task 5.2: Wire each of the 7 methods

- [ ] **Step 1: `PaintView`**

Edit `crates/emcore/src/emViewPort.rs:136–141`:
```rust
    /// Port of C++ `emViewPort::PaintView`.
    pub fn PaintView(&self) {
        if let Some(weak) = &self.window {
            if let Some(rc) = weak.upgrade() {
                rc.borrow_mut().request_redraw();
            }
        }
    }
```

`request_redraw()` is the existing winit-backed path on `emWindow` (verify with `grep -n 'fn request_redraw' crates/emcore/src/emWindow.rs`; if absent, add a thin wrapper that calls `self.winit_window.request_redraw()`).

- [ ] **Step 2: `GetViewCursor`**

Edit `crates/emcore/src/emViewPort.rs:129–134`:
```rust
    /// Port of C++ `emViewPort::GetViewCursor`.
    pub fn GetViewCursor(&self) -> crate::emCursor::emCursor {
        self.cursor
    }
```

Add a setter for emView to call:
```rust
    pub fn SetViewCursor(&mut self, cursor: crate::emCursor::emCursor) {
        if self.cursor != cursor {
            self.cursor = cursor;
            self.cursor_dirty = true;
        }
    }
```

- [ ] **Step 3: `IsSoftKeyboardShown` / `ShowSoftKeyboard` (UPSTREAM-GAP)**

Edit `crates/emcore/src/emViewPort.rs:170–183`. Replace both bodies:
```rust
    /// Port of C++ `emViewPort::IsSoftKeyboardShown`.
    ///
    /// UPSTREAM-GAP: emCore ships this as a no-op; no platform backend
    /// (emX11, emWnds) overrides it. Soft-keyboard support is absent in
    /// upstream Eagle Mode.
    pub fn IsSoftKeyboardShown(&self) -> bool {
        false
    }

    /// Port of C++ `emViewPort::ShowSoftKeyboard`.
    ///
    /// UPSTREAM-GAP: emCore ships this as a no-op; no platform backend
    /// (emX11, emWnds) overrides it. Soft-keyboard support is absent in
    /// upstream Eagle Mode.
    pub fn ShowSoftKeyboard(&mut self, _show: bool) {}
```

Note: `PHASE-5-TODO:` markers replaced with `UPSTREAM-GAP:` per spec §2.5c.

- [ ] **Step 4: `GetInputClockMS`**

Edit `crates/emcore/src/emViewPort.rs:185–193`:
```rust
    /// Port of C++ `emViewPort::GetInputClockMS`.
    pub fn GetInputClockMS(&self) -> u64 {
        self.input_clock_ms
    }
```

In `emWindow::dispatch_input`, before forwarding to the view port, set `self.current_view_port.borrow_mut().input_clock_ms = scheduler.clock_ms();`. Add `clock_ms(&self) -> u64` to `emScheduler` if absent (it derives from `Instant`-based monotonic time; the scheduler already tracks time in its `clock` field).

- [ ] **Step 5: `InputToView`**

Edit `crates/emcore/src/emViewPort.rs:195–201`:
```rust
    /// Port of C++ `emViewPort::InputToView`.
    pub fn InputToView(
        &mut self,
        view: &mut crate::emView::emView,
        tree: &mut crate::emPanelTree::PanelTree,
        event: &crate::emInputEvent::emInputEvent,
        state: &mut crate::emInputState::emInputState,
    ) {
        view.Input(tree, event, state);
    }
```

(C++ has `emViewPort::InputToView(emInputEvent& event, const emInputState& state)`; Rust takes view + tree as parameters because it cannot reach back through the back-reference for input — the back-reference is `Weak<RefCell<emWindow>>`, and `emView` lives inside `emWindow`. The dispatch site in `emWindow::dispatch_input` already holds borrows to both.)

In `crates/emcore/src/emWindow.rs:558` (the existing `dispatch_input`), replace the direct calls into `self.view` with a call through the view port:
```rust
self.view.viewport_mut().InputToView(&mut self.view, tree, &ev, state);
```
**Caution:** this creates a borrow conflict (`self.view` borrowed twice). The fix is to pull the viewport `Rc` out before the call:
```rust
let vp = self.view.CurrentViewPort.clone();
vp.borrow_mut().InputToView(&mut self.view, tree, &ev, state);
```

Verify with `cargo check`. If borrow errors persist, the architectural decision per spec §2.2 is `emViewPort` owns dispatch — the simplest fix is to make `InputToView` a free-function-style associated function on `emViewPort` taking view/tree/event/state, with no `self`. Pick that shape if borrow-checking proves fiddly.

- [ ] **Step 6: `InvalidateCursor`**

Edit `crates/emcore/src/emViewPort.rs:203–209`:
```rust
    /// Port of C++ `emViewPort::InvalidateCursor`.
    pub fn InvalidateCursor(&mut self) {
        self.cursor_dirty = true;
    }
```

In `emWindow::render` (or its frame-prologue), consume the flag:
```rust
if self.view.CurrentViewPort.borrow().cursor_dirty {
    let cursor = self.view.CurrentViewPort.borrow().cursor;
    self.winit_window.set_cursor(cursor.to_winit_cursor());
    self.view.CurrentViewPort.borrow_mut().cursor_dirty = false;
}
```

Add a `to_winit_cursor()` method to `emCursor` if absent. (It's likely a small `match` over the 19 variants. Map `Normal → CursorIcon::Default`, `Hand → CursorIcon::Pointer`, etc. Use winit 0.30's `CursorIcon` variants.)

- [ ] **Step 7: `InvalidatePainting`**

Edit `crates/emcore/src/emViewPort.rs:211–217`:
```rust
    /// Port of C++ `emViewPort::InvalidatePainting(x, y, w, h)`.
    pub fn InvalidatePainting(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(weak) = &self.window {
            if let Some(rc) = weak.upgrade() {
                rc.borrow_mut().invalidate_rect(x, y, w, h);
            }
        }
    }
```

Add `invalidate_rect(&mut self, x: f64, y: f64, w: f64, h: f64)` to `emWindow` if absent:
```rust
pub fn invalidate_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
    self.tile_cache.invalidate_rect(x, y, w, h);
}
```

`TileCache::invalidate_rect` exists (verify with `grep -n 'fn invalidate' crates/emcore/src/emViewRendererTileCache.rs`). If absent, add it as a thin wrapper around the existing tile-mark-dirty path.

### Task 5.3: Wire the back-reference at construction

- [ ] **Step 1: Set the back-reference in `emWindow::create`**

After the `emView::new(...)` line in `emWindow::create` (around `crates/emcore/src/emWindow.rs:135`):
```rust
let view = emView::new(root_panel, w as f64, h as f64);
```
Wrap the resulting `emWindow` in `Rc<RefCell<>>` if it isn't already, then immediately set the back-reference:
```rust
let window = Rc::new(RefCell::new(Self { ... }));
window.borrow().view.CurrentViewPort.borrow_mut().window =
    Some(Rc::downgrade(&window));
```

If the `emWindow::create` return shape is not `Rc<RefCell<...>>`, this requires a wider refactor. **Check first** by reading the calling site in `emGUIFramework::create_window` or similar. If the framework holds windows by value/Box, wrap the construction in `Rc<RefCell<>>` here and update the framework's container. This is in scope for Phase 5 (back-reference wiring is the headline), but should not metastasize. If it grows beyond ~50 lines of plumbing, **stop** and split this step into a sub-task.

### Task 5.4: New input-routing test

- [ ] **Step 1: Create the test file**

Create `crates/eaglemode/tests/unit/input_dispatch_chain.rs`:
```rust
//! Phase 5 acceptance test (emview-rewrite-followups).
//!
//! Verifies that input events flow through the C++ chain:
//! emWindow::dispatch_input → emViewPort::InputToView → emView::Input.

use eaglemode::emcore::emInputEvent::{emInputEvent, InputKey, InputVariant};
use eaglemode::emcore::emInputState::emInputState;

#[test]
fn input_routes_through_viewport() {
    // Construct a minimal emWindow + emView + viewport.
    // Send a synthetic mouse-press event via emWindow::dispatch_input.
    // Assert: emView received the event (e.g., active panel was updated,
    // or a side-effect counter on emViewPort was incremented).

    let (mut win, mut tree) = support::pipeline::headless_window();
    let mut state = emInputState::new();

    let event = emInputEvent {
        key: InputKey::MouseLeft,
        variant: InputVariant::Press,
        mouse_x: 10.0,
        mouse_y: 10.0,
        ..Default::default()
    };

    let count_before = win.view().CurrentViewPort.borrow().input_event_count;
    win.dispatch_input(&mut tree, &event, &mut state);
    let count_after = win.view().CurrentViewPort.borrow().input_event_count;

    assert_eq!(count_after, count_before + 1,
        "InputToView did not run — chain is broken");
}
```

Add an `input_event_count: u64` field to `emViewPort` (initialized to 0; incremented in `InputToView`). This is purely a test instrumentation field — annotate with `// Test instrumentation: counts InputToView dispatches.`

If `support::pipeline::headless_window()` does not exist, add it to `crates/eaglemode/tests/support/pipeline.rs`. If a headless window is genuinely impossible without a real winit event loop, replace the test with one that instantiates `emViewPort` and `emView` directly and calls `InputToView` to verify the dispatch path independently of `emWindow`. Acceptance is "the test exercises `emViewPort::InputToView` reaching `emView::Input`", not specifically that it goes through `emWindow`.

- [ ] **Step 2: Wire the test into the test harness**

Edit `crates/eaglemode/tests/unit/mod.rs` (or wherever the unit-test mod lives — `grep -rn 'mod scheduler' crates/eaglemode/tests/unit/` to find the pattern):
```rust
mod input_dispatch_chain;
```

- [ ] **Step 3: Run the new test**

```bash
cargo-nextest ntr -E 'test(input_routes_through_viewport)'
```
Expected: PASS.

### Task 5.5: Strip all `PHASE-5-TODO:` comments

- [ ] **Step 1: Verify zero residual `PHASE-5-TODO:` in emViewPort.rs**

```bash
grep -n 'PHASE-5-TODO' crates/emcore/src/emViewPort.rs
```
Expected: zero matches.

- [ ] **Step 2: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: all pass; 2404/2404 nextest (one new test); golden ≥235.

- [ ] **Step 3: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124. The new dispatch chain runs in the smoke run.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emViewPort.rs crates/emcore/src/emWindow.rs \
        crates/emcore/src/emCursor.rs crates/emcore/src/emScheduler.rs \
        crates/eaglemode/tests/unit/input_dispatch_chain.rs \
        crates/eaglemode/tests/unit/mod.rs
git commit -m "$(cat <<'EOF'
feat(emViewPort): wire 7 backend methods to emWindow / scheduler

Closes Phase 5 of emview-rewrite-followups. Routes:
- PaintView    → emWindow::request_redraw
- GetViewCursor / SetViewCursor → cached on emViewPort
- GetInputClockMS → scheduler clock
- InputToView  → emView::Input (the C++ chain; was direct emWindow→emView)
- InvalidateCursor / InvalidatePainting → emWindow tile cache + winit cursor

Soft-keyboard methods carry UPSTREAM-GAP comments matching upstream
Eagle Mode (no platform backend overrides them in C++ either). Adds an
input-dispatch-chain test asserting the routing.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 6: Real popup `emWindow`

Replaces the disabled popup path with real winit-backed undecorated popup windows. Spec §2.2 Item 6 and Item 8 partial (GeometrySignal fire).

**Files:**
- Modify: `crates/emcore/src/emWindow.rs` (popup ctor path verified Phase 4; add `new_popup` helper that calls `create` with the right flags)
- Modify: `crates/emcore/src/emView.rs` (`SwapViewPorts` GeometrySignal fire at line ~1675)

### Task 6.1: Add `emWindow::new_popup` helper

- [ ] **Step 1: Add a thin convenience constructor**

Edit `crates/emcore/src/emWindow.rs` (place after `create`, around line 180 or wherever fits):
```rust
/// Port of C++ `emWindow::emWindow(emView &, emWindow *, WF_POPUP, tag)`.
///
/// Creates an undecorated popup window for the given owner View. The
/// returned window shares the scheduler context with the owner and is
/// always-on-top.
#[allow(clippy::too_many_arguments)]
pub fn new_popup(
    event_loop: &winit::event_loop::ActiveEventLoop,
    gpu: &GpuContext,
    root_panel: PanelId,
    close_signal: SignalId,
    flags_signal: SignalId,
    focus_signal: SignalId,
    geometry_signal: SignalId,
) -> Self {
    Self::create(
        event_loop,
        gpu,
        root_panel,
        WindowFlags::POPUP | WindowFlags::UNDECORATED | WindowFlags::AUTO_DELETE,
        close_signal,
        flags_signal,
        focus_signal,
        geometry_signal,
    )
}
```

The `with_window_level(AlwaysOnTop)` plumbing is already in `create` from Phase 4 step 5.

### Task 6.2: Fire GeometrySignal in SwapViewPorts (Item 8 half)

- [ ] **Step 1: Read the current TODO context**

Run: `sed -n '1668,1685p' crates/emcore/src/emView.rs`
Expected:
```rust
} else if self.PopupWindow.is_some() {
    // C++ (emView.cpp:1674-1680): tear down popup on return inside home
    self.SwapViewPorts(true);
    self.PopupWindow = None;
    // C++ (emView.cpp:1678): Signal(GeometrySignal) — PHASE-5-TODO
    forceViewingUpdate = true;
}
```

- [ ] **Step 2: Wire the signal fire**

Replace the `// PHASE-5-TODO` comment with a real fire. The scheduler ref-cell needs to be reachable from `emView`. The `GeometrySignal` is owned by the `emWindow` (field `geometry_signal: SignalId`). The `emView` does not directly hold a scheduler reference today — it threads through the `EngineCtx` during `Cycle`.

If `emView::SwapViewPorts` does not have a scheduler reference handy, add a `geometry_signal_pending: bool` flag on `emView`. Set it here. Drain it in the engine's `Cycle` (added Phase 7) where the `EngineCtx` is available:
```rust
if self.geometry_signal_pending {
    ctx.fire(self.geometry_signal);
    self.geometry_signal_pending = false;
}
```

If `emView` does not own `geometry_signal: SignalId`, store it as a field passed in at `emView::new`. Wire in `emWindow::create` after the view is constructed:
```rust
view.set_geometry_signal(geometry_signal);
```

### Task 6.3: Verification

- [ ] **Step 1: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: all pass.

- [ ] **Step 2: Runtime smoke with popup-count assertion**

The smoke run does not currently exercise popup windows. Add minimal logging to `emGUIFramework::about_to_wait`:
```rust
let n_windows = self.windows.len();
if n_windows != self.last_logged_window_count {
    eprintln!("[smoke] window count: {n_windows}");
    self.last_logged_window_count = n_windows;
}
```
(Add `last_logged_window_count: usize` to the struct, initialized to 0.)

Then run:
```bash
timeout 20 cargo run --release --bin eaglemode 2>&1 | tee /tmp/phase6-smoke.log
echo "exit=$?"
```
Expected: exit 124; `/tmp/phase6-smoke.log` shows `[smoke] window count: 1` (no popup spontaneously opened, but the logging works). The popup-count ≥2 assertion in spec §3.3 row 6 requires user interaction; for automated verification, the criterion is "popup creation does not panic when invoked." Add a unit test instead:

```rust
#[test]
fn popup_window_creation_does_not_panic() {
    use winit::event_loop::EventLoop;
    let event_loop = EventLoop::new().expect("event loop");
    // ... headless GPU context setup ...
    // Skip if no display available (CI):
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipping: no display");
        return;
    }
    // Construct a popup. Just exercising the path, asserting no panic.
}
```
Place at `crates/eaglemode/tests/unit/popup_window.rs`. If headless setup proves intractable, skip the test and rely on the runtime smoke run for the assertion — but **document explicitly** in the commit that the popup-creation path was exercised manually (open, close, no crash).

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emWindow.rs crates/emcore/src/emView.rs \
        crates/emcore/src/emGUIFramework.rs \
        crates/eaglemode/tests/unit/popup_window.rs \
        crates/eaglemode/tests/unit/mod.rs
git commit -m "$(cat <<'EOF'
feat(emWindow): create real winit popup windows; fire GeometrySignal

Closes Phase 6 of emview-rewrite-followups. emWindow::new_popup creates
an undecorated, always-on-top winit window via the existing create()
path. SwapViewPorts now fires GeometrySignal on viewport swap (was
PHASE-5-TODO). Adds smoke logging for window count + a popup-creation
unit test (gated on display availability).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 7: `impl emEngine` for `EOIEngineClass` and `UpdateEngineClass`

Both engine classes already have correct `Cycle()` shapes and field state. This phase makes them real `emEngine` implementations and registers them with the scheduler. Spec §2.2 Item 9.

**Files:**
- Modify: `crates/emcore/src/emView.rs` (lines 199–273 — both engine classes; line 1889, 3057 — manual `WakeUp` removal; line 432 — `emView::new` registration; line 3017 — delete `tick_eoi`)
- Modify: `crates/emcore/src/emView.rs` test at line 5470 — replace `tick_eoi` with `scheduler.DoTimeSlice`-driven assertion

### Task 7.1: Implement `emEngine` for both classes

- [ ] **Step 1: Read current `emEngine` trait**

Run: `sed -n '15,30p' crates/emcore/src/emEngine.rs`
Expected: `pub trait emEngine { fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool; }`.

- [ ] **Step 2: Add `EOISignal: SignalId` to `emView`**

If absent (verify: `grep -n 'EOISignal' crates/emcore/src/emView.rs`), add it as a field. C++ has it; Rust will need it for `EOIEngineClass::Cycle` to fire.

- [ ] **Step 3: Implement `emEngine` for `EOIEngineClass`**

Edit `crates/emcore/src/emView.rs` after the existing `EOIEngineClass` impl block (around line 273). Change the existing `Cycle(&mut self) -> bool` method's signature to match the trait. The current `Cycle` returns whether the EOI signal should fire; the trait `Cycle` returns whether to stay awake.

The replacement uses an `Rc<RefCell<...>>` shared with the view. Because `EOIEngineClass` does not have direct access to the view's `EOISignal` field, restructure: the engine holds a `SignalId` for the EOI signal, fires it on count-down completion, and goes to sleep:
```rust
pub struct EOIEngineClass {
    pub CountDown: i32,
    pub eoi_signal: SignalId,
}

impl EOIEngineClass {
    pub fn new(eoi_signal: SignalId) -> Self {
        Self { CountDown: 5, eoi_signal }
    }
}

impl emEngine for EOIEngineClass {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        self.CountDown -= 1;
        if self.CountDown <= 0 {
            ctx.fire(self.eoi_signal);
            false  // sleep — the view will re-create on next SignalEOIDelayed
        } else {
            true   // stay awake to keep ticking
        }
    }
}
```

- [ ] **Step 4: Implement `emEngine` for `UpdateEngineClass`**

Replace the current `UpdateEngineClass` with one that owns an `Rc<RefCell<emView>>` (or `Weak<RefCell<emView>>` to avoid cycles):
```rust
pub struct UpdateEngineClass {
    pub view: std::rc::Weak<std::cell::RefCell<emView>>,
    pub tree: std::rc::Weak<std::cell::RefCell<PanelTree>>,
}

impl emEngine for UpdateEngineClass {
    fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
        if let (Some(view), Some(tree)) = (self.view.upgrade(), self.tree.upgrade()) {
            view.borrow_mut().Update(&mut tree.borrow_mut());
        }
        false  // C++ UpdateEngineClass::Cycle wakes itself only on signals; default sleep
    }
}
```

This is a shape change. The current `WakeUp()` callsites in `emView.rs:1889, 3057` become `ctx.wake_up(self.update_engine_id)`. The view holds the `EngineId` returned from registration. **Read the call sites first** (`sed -n '1885,1895p' crates/emcore/src/emView.rs`) to confirm `ctx` is in scope at the call — if not, this becomes a structural change.

If the call sites are inside `emView::Update` itself (not a `Cycle` of some other engine), the wake-up is a no-op — the view is already running. Just delete the manual `WakeUp()` line and verify nothing breaks.

### Task 7.2: Register both engines in `emView::new`

- [ ] **Step 1: Read `emView::new`**

Run: `sed -n '432,520p' crates/emcore/src/emView.rs`

- [ ] **Step 2: Refactor `emView::new` to accept a scheduler reference**

The current `emView::new(root, w, h)` does not take a scheduler. Registration requires `scheduler.register_engine(...)`. Either:
- Add `scheduler: &mut emScheduler` (or `Rc<RefCell<emScheduler>>`) as a parameter, **or**
- Defer registration to a separate `attach_to_scheduler(&mut self, scheduler: ...)` method called from `emWindow::create` after `emView::new`.

**Pick the second option** — minimizes API surface change. Add:
```rust
pub fn attach_to_scheduler(
    &mut self,
    scheduler: Rc<RefCell<emScheduler>>,
    self_rc: Weak<RefCell<emView>>,
    tree_rc: Weak<RefCell<PanelTree>>,
) {
    let update_engine = Box::new(UpdateEngineClass {
        view: self_rc.clone(),
        tree: tree_rc,
    });
    self.update_engine_id = Some(
        scheduler.borrow_mut().register_engine(Priority::HIGH, update_engine)
    );
    // EOI engine is created on demand by SignalEOIDelayed.
}
```

Add fields to `emView`:
```rust
pub update_engine_id: Option<EngineId>,
pub scheduler: Option<Weak<RefCell<emScheduler>>>,
```

In `emWindow::create`, after `emView::new`, call `view.attach_to_scheduler(...)`. This requires `emView` to live in `Rc<RefCell<>>` from the start — coordinate with Phase 5's back-reference work.

### Task 7.3: Replace `SignalEOIDelayed` to register EOI engine

- [ ] **Step 1: Read current `SignalEOIDelayed`**

Run: `grep -n 'fn SignalEOIDelayed' crates/emcore/src/emView.rs`
Then read 20 lines around it.

- [ ] **Step 2: Update to register through scheduler**

Inside `SignalEOIDelayed`:
```rust
pub fn SignalEOIDelayed(&mut self) {
    if let Some(scheduler_weak) = &self.scheduler {
        if let Some(scheduler) = scheduler_weak.upgrade() {
            let engine = Box::new(EOIEngineClass::new(self.EOISignal));
            scheduler.borrow_mut().register_engine(Priority::HIGH, engine);
        }
    }
    // Old: self.EOIEngine = Some(EOIEngineClass::new());
}
```

Delete the `EOIEngine: Option<EOIEngineClass>` field on `emView` (now redundant — engine lives in the scheduler).

### Task 7.4: Delete `tick_eoi` and manual `WakeUp` calls

- [ ] **Step 1: Delete `tick_eoi`**

Edit `crates/emcore/src/emView.rs:3017–3023`. Remove the entire method:
```rust
pub fn tick_eoi(&mut self) -> bool {
    let fired = self.EOIEngine.as_mut().map(|e| e.Cycle()).unwrap_or(false);
    if fired {
        self.EOIEngine = None;
    }
    fired
}
```

- [ ] **Step 2: Update the test at line 5470**

Read: `sed -n '5450,5500p' crates/emcore/src/emView.rs`. The test (`test_eoi_engine_counting` per investigation) calls `tick_eoi`. Rewrite it to drive through the scheduler:
```rust
let mut scheduler = emScheduler::new();
let view_rc = Rc::new(RefCell::new(view));
let tree_rc = Rc::new(RefCell::new(tree));
view_rc.borrow_mut().attach_to_scheduler(
    Rc::new(RefCell::new(scheduler)),
    Rc::downgrade(&view_rc),
    Rc::downgrade(&tree_rc),
);
view_rc.borrow_mut().SignalEOIDelayed();

// Drive 6 time slices; EOI signal should have fired on the 5th.
for _ in 0..6 {
    scheduler.borrow_mut().DoTimeSlice(&mut tree_rc.borrow_mut(), &mut HashMap::new());
}
assert!(scheduler.borrow().was_signaled_since_last_check(view_rc.borrow().EOISignal));
```

Adjust the API names to match what `emScheduler` actually exposes — verify with `grep -n 'pub fn' crates/emcore/src/emScheduler.rs`.

- [ ] **Step 3: Delete manual `WakeUp` calls at line 1889, 3057**

Run: `grep -n 'UpdateEngine.*WakeUp\|UpdateEngine\.as_mut' crates/emcore/src/emView.rs`
For each, replace with `// scheduler drives Update; no manual wake-up needed.` or delete entirely.

If a call site genuinely needs to wake the scheduler (e.g., something external invalidates and Update needs to run), use `scheduler.wake_up(self.update_engine_id.unwrap())` instead. Verify with `cargo check`.

- [ ] **Step 4: Verify zero residual `tick_eoi`**

```bash
grep -rn 'tick_eoi' crates/
```
Expected: `0`.

- [ ] **Step 5: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: 2403/2403 (test was rewritten, count unchanged); golden ≥235.

- [ ] **Step 6: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124. The Update engine is now driven by the scheduler instead of the window loop — the framework's `DoTimeSlice` call already runs each frame, so behavior is unchanged.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emWindow.rs
git commit -m "$(cat <<'EOF'
feat(emView): impl emEngine for EOI/Update; register with scheduler

Closes Phase 7 of emview-rewrite-followups. UpdateEngineClass and
EOIEngineClass now implement the emEngine trait and are registered with
the scheduler. The manual WakeUp() call sites and the tick_eoi test
harness are gone — scheduler.DoTimeSlice drives both engines uniformly.

Matches C++, where both engines are scheduler-driven from emView::new.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 8: Popup-close drain in `emView::Update` + `SwapViewPorts` close-signal wake-up

Spec §2.2 Items 7 and 8 (close-signal half).

**Files:**
- Modify: `crates/emcore/src/emView.rs` (lines 2120–2160 — `Update`; line 1616 — `SwapViewPorts` popup branch)
- Create: `crates/eaglemode/tests/behavioral/popup_close_zoom_out.rs` (new behavioral test driving the scheduler explicitly)

### Task 8.1: Wire popup-close drain in `Update`

- [ ] **Step 1: Read the backend-gap site**

Run: `sed -n '2118,2135p' crates/emcore/src/emView.rs`
Expected: shows the comment block at 2120–2123.

- [ ] **Step 2: Replace the comment with the real check**

The `popup` is `self.PopupWindow: Option<Rc<RefCell<emWindow>>>`. The close signal lives on it as `close_signal: SignalId`. The scheduler reference is on `self.scheduler` (from Phase 7). Replace the comment block with:
```rust
if let (Some(popup), Some(scheduler_weak)) = (&self.PopupWindow, &self.scheduler) {
    if let Some(scheduler) = scheduler_weak.upgrade() {
        let close_sig = popup.borrow().close_signal;
        if scheduler.borrow().is_signaled(close_sig) {
            self.ZoomOut(tree);
        }
    }
}
```

The exact `is_signaled` API may differ — verify with `grep -n 'fn is_signaled\|fn IsSignaled' crates/emcore/src/emScheduler.rs`.

### Task 8.2: Wire close-signal wake-up in `SwapViewPorts`

- [ ] **Step 1: Read the TODO site**

Run: `sed -n '1605,1625p' crates/emcore/src/emView.rs`

- [ ] **Step 2: Replace `// PHASE-5-TODO: wire close-signal wake-up.`**

The comment is just before the line that should add the wake-up signal. Replace with:
```rust
// C++ (emView.cpp:1644): UpdateEngine->AddWakeUpSignal(PopupWindow->GetCloseSignal())
if let (Some(popup), Some(scheduler_weak), Some(engine_id)) =
    (&self.PopupWindow, &self.scheduler, self.update_engine_id) {
    if let Some(scheduler) = scheduler_weak.upgrade() {
        let close_sig = popup.borrow().close_signal;
        scheduler.borrow_mut().add_wake_up_signal(engine_id, close_sig);
    }
}
```

If `emScheduler` does not expose `add_wake_up_signal` (verify with grep), it has an analog like `connect_signal_to_engine` or similar — use whichever API connects a signal so that firing it wakes the engine. The C++ name is `AddWakeUpSignal`; the Rust function name may differ but the contract is identical.

If no such API exists, **add it to `emScheduler`** (small surface addition). The implementation is straightforward: maintain a `signal → Vec<EngineId>` map; on `fire(signal)`, also wake every engine in the list.

### Task 8.3: New behavioral test

- [ ] **Step 1: Create the test file**

Create `crates/eaglemode/tests/behavioral/popup_close_zoom_out.rs`:
```rust
//! Phase 8 acceptance test (emview-rewrite-followups).
//!
//! Verifies: opening a popup then firing its close_signal drives the
//! view to ZoomOut on the next scheduler time slice.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[test]
fn popup_close_signal_zooms_out() {
    let scheduler = Rc::new(RefCell::new(emScheduler::new()));
    let close_sig = scheduler.borrow_mut().alloc_signal();
    // ... construct view + tree + popup window with close_sig ...

    view.borrow_mut().Zoom(/* args */);
    assert!(view.borrow().is_zoomed_in(), "precondition");

    scheduler.borrow_mut().fire(close_sig);
    scheduler.borrow_mut().DoTimeSlice(&mut tree.borrow_mut(), &mut HashMap::new());

    assert!(view.borrow().is_zoomed_out(),
        "popup-close did not trigger ZoomOut");
}
```

If `is_zoomed_out` is not a public method, use a proxy like checking `viewed_x == 0.0 && viewed_y == 0.0 && viewed_width == HomeWidth`. Adjust assertions to actual API.

- [ ] **Step 2: Wire into the harness**

Edit `crates/eaglemode/tests/behavioral/mod.rs`:
```rust
mod popup_close_zoom_out;
```

- [ ] **Step 3: Run the new test**

```bash
cargo-nextest ntr -E 'test(popup_close_signal_zooms_out)'
```
Expected: PASS.

### Task 8.4: Phase gate + commit

- [ ] **Step 1: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: 2405/2405 nextest (one new from Phase 5 + one from Phase 8); golden ≥235.

- [ ] **Step 2: Verify zero residual `backend-gap:` and `PHASE-5-TODO` in emView.rs**

```bash
grep -n 'backend-gap:\|PHASE-5-TODO' crates/emcore/src/emView.rs
```
Expected: zero matches.

- [ ] **Step 3: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emScheduler.rs \
        crates/eaglemode/tests/behavioral/popup_close_zoom_out.rs \
        crates/eaglemode/tests/behavioral/mod.rs
git commit -m "$(cat <<'EOF'
feat(emView): wire popup-close signal drain + SwapViewPorts wake-up

Closes Phase 8 of emview-rewrite-followups. emView::Update now drains
the popup's close_signal and calls ZoomOut when fired (was backend-gap
comment). SwapViewPorts registers the popup close_signal as a wake-up
for the Update engine. Adds a behavioral test that drives the scheduler
explicitly to verify the round-trip.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 9: `GetMaxPopupViewRect` from `emScreen` monitor data

Populate `max_popup_rect` from `emScreen` so popup geometry uses real monitor bounds. Spec §2.2 Item 10.

**Files:**
- Modify: `crates/emcore/src/emView.rs` (line 2949 — `GetMaxPopupViewRect`; constructor or attach method to receive monitor info)
- Modify: `crates/emcore/src/emWindow.rs` (pass monitor info into emView at construction)
- Create: `crates/eaglemode/tests/unit/max_popup_rect_fallback.rs`

### Task 9.1: Plumb monitor info into emView

- [ ] **Step 1: Verify emScreen API**

Run: `grep -n 'pub fn' crates/emcore/src/emScreen.rs`
Expected: includes `monitors() -> &[MonitorInfo]`, `primary() -> Option<&MonitorInfo>`.

- [ ] **Step 2: Find where emWindow has access to emScreen**

Run: `grep -n 'emScreen\|screen' crates/emcore/src/emWindow.rs crates/emcore/src/emGUIFramework.rs`
Expected: emGUIFramework owns the scheduler and likely the screen (verify).

- [ ] **Step 3: Add a setter on emView for max_popup_rect**

Edit `crates/emcore/src/emView.rs`:
```rust
/// Set the maximum popup view rect from the owning monitor's geometry.
/// Called from emWindow::create after the window is positioned.
pub fn set_max_popup_rect(&mut self, rect: Rect) {
    self.max_popup_rect = Some(rect);
}
```

- [ ] **Step 4: Call it from emWindow::create**

Edit `crates/emcore/src/emWindow.rs::create`. After the view is constructed, query the monitor that contains the new winit window:
```rust
if let Some(monitor) = winit_window.current_monitor() {
    let pos = monitor.position();
    let size = monitor.size();
    view.set_max_popup_rect(Rect::new(
        pos.x as f64,
        pos.y as f64,
        size.width as f64,
        size.height as f64,
    ));
}
```

(`current_monitor()` may return `None` on Wayland; the fallback to home rect handles that case.)

### Task 9.2: GetMaxPopupViewRect remains as-is

The body at line 2949 already does the right thing (reads `max_popup_rect`, falls back to home rect). Confirm no edits needed:

- [ ] **Step 1: Re-read the body**

Run: `sed -n '2949,2965p' crates/emcore/src/emView.rs`
Confirm: code matches what the spec requires (`max_popup_rect` read with home-rect fallback). The `PHASE-5-TODO` comment goes:

- [ ] **Step 2: Remove the PHASE-5-TODO comment**

Edit the doc comment block above the function. Remove the line `/// PHASE-5-TODO: delegate to emWindowPort for real monitor bounds.` and replace with:
```rust
/// When no monitor info is available (e.g., headless or Wayland without
/// position queries), falls back to the home rect.
```

### Task 9.3: Fallback test

- [ ] **Step 1: Create the fallback test**

Create `crates/eaglemode/tests/unit/max_popup_rect_fallback.rs`:
```rust
//! Phase 9 acceptance test (emview-rewrite-followups).
//!
//! Verifies: GetMaxPopupViewRect falls back to the home rect when
//! max_popup_rect was never set (no emScreen available).

use eaglemode::emcore::emView::emView;
use eaglemode::emcore::emPanelTree::PanelTree;

#[test]
fn max_popup_rect_falls_back_to_home() {
    let mut tree = PanelTree::new();
    let root = tree.new_root_panel(/* args */);
    let view = emView::new(root, 1024.0, 768.0);
    // No set_max_popup_rect call.

    let mut out = (0.0, 0.0, 0.0, 0.0);
    view.GetMaxPopupViewRect(&mut out);

    assert_eq!(out, (view.HomeX, view.HomeY, view.HomeWidth, view.HomeHeight),
        "fallback should equal home rect");
}
```

- [ ] **Step 2: Wire into harness**

Edit `crates/eaglemode/tests/unit/mod.rs`:
```rust
mod max_popup_rect_fallback;
```

- [ ] **Step 3: Run the test**

```bash
cargo-nextest ntr -E 'test(max_popup_rect_falls_back_to_home)'
```
Expected: PASS.

### Task 9.4: Phase gate + commit

- [ ] **Step 1: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: 2406/2406 nextest; golden ≥235.

- [ ] **Step 2: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124.

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emWindow.rs \
        crates/eaglemode/tests/unit/max_popup_rect_fallback.rs \
        crates/eaglemode/tests/unit/mod.rs
git commit -m "$(cat <<'EOF'
feat(emView): populate max_popup_rect from current monitor

Closes Phase 9 of emview-rewrite-followups. emWindow::create now queries
winit_window.current_monitor() and seeds max_popup_rect with the owning
monitor's geometry. The home-rect fallback path is preserved (Wayland
without position queries; headless).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 10: `InvalidateHighlight` guard tightening

Replace `self.active.is_some()` proxy with the C++ guard. Spec §2.2 Item 11b.

**Files:**
- Modify: `crates/emcore/src/emView.rs` (lines 3027–3046)

### Task 10.1: Implement the C++ guard

- [ ] **Step 1: Read the current body**

Run: `sed -n '3027,3046p' crates/emcore/src/emView.rs`
Expected: the function with `let active_viewed = self.active.is_some();`.

- [ ] **Step 2: Read the C++ source for reference**

Run: `sed -n '2137,2146p' ~/git/eaglemode-0.96.4/src/emCore/emView.cpp`
Expected:
```cpp
void emView::InvalidateHighlight()
{
    if (
        !ActivePanel || !ActivePanel->Viewed || (
            (VFlags&VF_NO_ACTIVE_HIGHLIGHT)!=0 &&
            ((VFlags&VF_NO_FOCUS_HIGHLIGHT)!=0 || !Focused)
        )
    ) return;
    InvalidatePainting(); //??? too much
}
```

- [ ] **Step 3: Rewrite the body**

Replace lines 3027–3046:
```rust
/// Port of C++ `emView::InvalidateHighlight` (emView.cpp:2137-2146).
///
/// If the active panel is viewed and highlight should be drawn, marks
/// the whole view dirty so the highlight is repainted. C++ comment notes
/// this is overly broad ("too much") — we preserve that behaviour.
pub fn InvalidateHighlight(&mut self, tree: &PanelTree) {
    let active_viewed = self.active
        .and_then(|id| tree.get(id))
        .map(|panel| panel.viewed)
        .unwrap_or(false);

    if !active_viewed {
        return;
    }

    let no_active = self.flags.contains(ViewFlags::NO_ACTIVE_HIGHLIGHT);
    let no_focus = self.flags.contains(ViewFlags::NO_FOCUS_HIGHLIGHT);
    if no_active && (no_focus || !self.window_focused) {
        return;
    }

    // C++ emView.cpp:2145: InvalidatePainting() — mark whole view dirty.
    self.dirty_rects.push(Rect::new(
        self.CurrentX,
        self.CurrentY,
        self.CurrentWidth,
        self.CurrentHeight,
    ));
}
```

The function now takes `&PanelTree`. Update every caller of `InvalidateHighlight` to pass the tree. Run `grep -n 'InvalidateHighlight' crates/` to find them.

If a caller does not have a `&PanelTree` handy, the call site needs restructuring — usually by threading `tree` through. Do this surgery locally.

If borrow-flow conflicts arise (e.g., a caller has `&mut self` on `emView` plus `&PanelTree` from a method that itself borrows the view), adjust by:
- pulling `active` and `panel.viewed` out into locals before the call, and passing them as args, **or**
- restructuring the caller to release one borrow before invoking `InvalidateHighlight`.

The function signature `(&mut self, tree: &PanelTree)` is the C++-shape — `View::InvalidateHighlight` reads `ActivePanel->Viewed` through the shared tree.

### Task 10.2: Phase gate + commit

- [ ] **Step 1: Verify guard now matches C++**

```bash
grep -n 'self\.active\.is_some' crates/emcore/src/emView.rs
```
Expected: zero matches in `InvalidateHighlight`'s function body (other usages elsewhere are fine).

- [ ] **Step 2: Phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: 2406/2406 nextest; golden ≥235.

- [ ] **Step 3: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "$(cat <<'EOF'
fix(emView): tighten InvalidateHighlight guard to C++ shape

Closes Phase 10 of emview-rewrite-followups. The C++ guard checks
ActivePanel->Viewed and the highlight flags; Rust was using
self.active.is_some() as a weaker proxy. Caller signatures now thread
&PanelTree where needed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 11: Visit-stack removal — close `invariant_equilibrium_at_target` factor=1.0 gap

**Highest-risk phase.** Touches animator math and all golden tests. Delete the persistent `visit_stack`; derive `rel_x`/`rel_y` from `ViewedX`/`ViewedY` on every read (matches C++). Spec §2.3 Item 11a.

**Files:**
- Modify: `crates/emcore/src/emView.rs` (lines 35–44 — `VisitState`; line 282 — field; lines 448, 658, 663–668 — readers/writers; many usage sites — audit required)
- Modify: `crates/emcore/src/emViewAnimator.rs` (line 3320 — remove `KNOWN GAP` skip)
- Possibly modify: `crates/emcore/src/emPanelTree.rs` (if there's a derived rel-coord helper used by Layout)

### Task 11.1: Comprehensive visit-stack audit

- [ ] **Step 1: Find every visit-stack reference**

```bash
grep -rn 'visit_stack\|VisitState' crates/
```
Record every file:line with surrounding context. **Do not proceed** until you have written down the role of each — getter, setter, push, pop, init, read for animator math, etc.

- [ ] **Step 2: Identify which fields of `VisitState` are actually consumed**

The struct has `panel`, `rel_x`, `rel_y`, `rel_a`. Per the spec, `rel_x`/`rel_y` will be derived. What about `rel_a` (zoom factor) and `panel`? Read each usage site:

```bash
grep -rn 'rel_x\|rel_y\|rel_a' crates/
grep -rn '\.panel\b' crates/emcore/src/emView.rs  # too noisy — narrow to visit_stack contexts
```

Goal: classify each field as "derivable from ViewedX/Y/Width" or "needs persistent storage." For C++ parity:
- `rel_x = (View.HomeX + View.HomeWidth*0.5 - ViewedX) / ViewedWidth - 0.5` — derivable
- `rel_y = (View.HomeY + View.HomeHeight*0.5 - ViewedY) / ViewedHeight - 0.5` — derivable
- `rel_a = (View.HomeWidth*View.HomeHeight) / (ViewedWidth*ViewedHeight)` — derivable
- `panel` — the active panel? Or the panel-being-visited stack? Need to check semantics.

If `panel` is the supreme viewed panel or active panel, those already have dedicated fields (`active`, `supreme_viewed_panel`). If `panel` is genuinely a stack of visited ancestors, that's harder — but per the spec, the entire stack is being deleted because C++ doesn't have one.

### Task 11.2: Add derived-coord helpers

- [ ] **Step 1: Add `current_rel_xyz` to `emView`**

Edit `crates/emcore/src/emView.rs`. Add helpers next to `ViewedX`/`ViewedY`/`ViewedWidth`/`ViewedHeight`:
```rust
/// Port of C++ derivation in emView.cpp:1620-1622.
/// rel_x = (HomeX + HomeWidth*0.5 - ViewedX) / ViewedWidth - 0.5
pub fn current_rel_x(&self) -> f64 {
    (self.HomeX + self.HomeWidth * 0.5 - self.ViewedX) / self.ViewedWidth - 0.5
}

pub fn current_rel_y(&self) -> f64 {
    (self.HomeY + self.HomeHeight * 0.5 - self.ViewedY) / self.ViewedHeight - 0.5
}

pub fn current_rel_a(&self) -> f64 {
    (self.HomeWidth * self.HomeHeight) / (self.ViewedWidth * self.ViewedHeight)
}
```

(Verify the formula against `~/git/eaglemode-0.96.4/src/emCore/emPanel.cpp:608-617` quoted in the investigation report.)

### Task 11.3: Replace every visit-stack read with derived computation

- [ ] **Step 1: Walk the audit list from Task 11.1 Step 1**

For each read of `visit_stack[*].rel_x` (or `.rel_y`, `.rel_a`), replace with `view.current_rel_x()` (or y / a). For each read of `visit_stack[*].panel`, replace with `view.active` or `view.supreme_viewed_panel` based on context.

- [ ] **Step 2: Delete the `visit_stack` field and `VisitState` struct**

Edit `crates/emcore/src/emView.rs`:
- Delete `pub struct VisitState { ... }` at lines 35–44.
- Delete `visit_stack: Vec<VisitState>,` at line 282.
- Delete `visit_stack: vec![initial_visit],` in `new()` at line 448.
- Delete the getter at line 658 and the mutable getter at lines 663–668.

If a caller still uses the getter, replace with the derived helpers from Task 11.2.

- [ ] **Step 3: Verify no residual visit_stack**

```bash
grep -rn 'visit_stack\|VisitState' crates/
```
Expected: zero matches.

### Task 11.4: Re-enable `invariant_equilibrium_at_target` at factor=1.0

- [ ] **Step 1: Read the current test loop**

Run: `sed -n '3310,3340p' crates/emcore/src/emViewAnimator.rs`

- [ ] **Step 2: Add 1.0 to the factor list, delete the gap comment**

Replace:
```rust
    // factor=1.0 is excluded — known Rust-vs-C++ design gap, tracked below.
    //
    // KNOWN GAP (TODO phase 8): At factor=1.0, root-centering ...
    for &factor in &[2.0, 4.0, 16.0, 100.0] {
```
with:
```rust
    for &factor in &[1.0, 2.0, 4.0, 16.0, 100.0] {
```

(Delete the entire `KNOWN GAP` comment block.)

### Task 11.5: Run the full suite — golden churn analysis

- [ ] **Step 1: Run nextest**

```bash
cargo-nextest ntr 2>&1 | tail -40
```
Expected: 2406/2406 (no count regression; the re-added factor=1.0 makes `invariant_equilibrium_at_target` exercise more cases — same test count).

- [ ] **Step 2: Run golden suite, capture results**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -40 > /tmp/phase11-golden.log
grep -E 'test result|FAILED' /tmp/phase11-golden.log
```
Expected: total pass count ≥ 235. **Failures may shift** — that is OK per spec §3.2 #4. **Total may not regress.**

- [ ] **Step 3: If golden total regressed**

Compare against the prior phase's known failures. Use `scripts/divergence_report.py --diff` to see which tests changed status:
```bash
python3 scripts/divergence_report.py --diff
```

For each newly-failing golden test, run:
```bash
scripts/verify_golden.sh <name>
```
This produces an op-diff against C++. The visit-stack removal changes how rel coords are derived; if this breaks a test, it means a code path was depending on a stored rel_x that did not match the derived rel_x from `ViewedX/Y`.

**Do not patch the test.** Patch the code so the derived value matches what the previously-stored value was. Most likely cause: a code path that wrote to `visit_stack[*].rel_x` was setting it to a value that did NOT match `current_rel_x()`. That write was a bug being masked by the visit stack; the fix is to ensure `ViewedX/Y/Width/Height` are correct at the moment the rel coord is needed, not to restore the visit stack.

If a regression genuinely cannot be fixed within this phase's scope, **stop and revisit**. Do not commit a regression.

### Task 11.6: Phase gate + commit

- [ ] **Step 1: Final phase gate**

```bash
cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```
Expected: 2406/2406 nextest; golden total pass ≥235.

- [ ] **Step 2: Runtime smoke**

```bash
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
```
Expected: exit 124.

- [ ] **Step 3: Spec acceptance grep**

```bash
grep -rn 'ZuiWindow\|svp_update_count\|home_pixel_tallness\|tick_eoi\|PHASE-5-TODO\|backend-gap:' crates/
```
Expected: zero matches.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emViewAnimator.rs \
        crates/emcore/src/emPanelTree.rs
git commit -m "$(cat <<'EOF'
refactor(emView): remove visit_stack; derive rel coords on read (C++ parity)

Closes Phase 11 of emview-rewrite-followups. Deletes the Rust-only
visit_stack and VisitState. rel_x, rel_y, rel_a are now derived from
ViewedX/Y/Width/Height on every read (matching C++ emPanel.cpp:608-617).
The invariant_equilibrium_at_target test now asserts at factor=1.0; the
KNOWN GAP marker is gone.

This was the highest-risk phase. Closes the spec.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Spec close-out

After Phase 11 commits cleanly:

- [ ] **Step 1: Update the source note**

Edit `docs/superpowers/notes/2026-04-17-emview-rewrite-followups.md`. Append a closing block at the end:
```markdown
---

## CLOSED — 2026-04-17

All 11 follow-up items resolved.

- Spec: `docs/superpowers/specs/2026-04-17-emview-rewrite-followups-design.md` (commit `9f149ba`)
- Plan: `docs/superpowers/plans/2026-04-17-emview-rewrite-followups.md`
- Final commit: <fill in last commit SHA after Phase 11>

Acceptance: 2406/2406 nextest; golden ≥235; runtime smoke ≥15s ALIVE.
```

- [ ] **Step 2: Commit the close-out note**

```bash
git add docs/superpowers/notes/2026-04-17-emview-rewrite-followups.md
git commit -m "$(cat <<'EOF'
docs(notes): close emView-rewrite-followups note

All 11 items resolved across Phases 1-11. Spec and plan delivered.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 3: Final acceptance check**

```bash
cargo-nextest ntr && cargo test --test golden -- --test-threads=1
timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
git log --oneline | head -15
```

Expected: green suites, exit 124, ~13 commits visible (one per phase + close-out + a couple split commits).

The plan is complete.
