# F018 Phase 3 Deferrals — Design Spec

**Date:** 2026-04-26
**Status:** Approved (brainstorm)
**Related:**
- `docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md` (parent)
- `docs/superpowers/specs/2026-04-25-F018-compositor-remediation-design.md` (Phase 1 + 2 plan)
- `docs/debug/F018-audit.md` (audit)
- `docs/debug/ISSUES.json` entry F018

## Background

Phase 1 (rules I.1, I.4, I.5) and Phase 2 (rule IV.3) of F018 landed in commits `f56f7fa2`, `988e799d`, `0e61e09f`. The user-visible loading-state black-overlay bug is closed pending manual visual verification.

Three follow-ups were deferred:

1. **Phase 3 cascade (rule II.5 partial completion).** The `PanelBehavior::Paint` trait surface gained a `canvas_color: emColor` parameter and every implementor was migrated, but **13 widget-side helpers** still read `painter.GetCanvasColor()` internally and need the parameter threaded down.
2. **Painter canvas-color carrier removal.** Once no production reader remains, `emPainter::canvas_color`, `Get/SetCanvasColor`, and `DrawOp::SetCanvasColor` should be deleted to retire the carrier entirely.
3. **`WakeUp` pairing for `SVPChoiceByOpacityInvalid` (Phase 2 reviewer Minor #1).** C++ `emPanel::InvalidatePainting` writes `View.SVPChoiceByOpacityInvalid=true` then calls `View.UpdateEngine->WakeUp()`. The Rust IV.3 fix at `emView.rs:3188`/`:3237` writes the flag but omits the wake.

This spec defines the fix shape for all three deferrals and orders them as one phased plan.

## Architecture and phasing

Three phases. Each phase ends in a green tree. Phases land sequentially.

- **Phase A — Cascade `canvas_color` through widget helpers** (deferral #1).
- **Phase B — Retire painter canvas-color carrier** (deferral #2). Coupled to A.
- **Phase C — Pair `WakeUpUpdateEngine` with `SVPChoiceByOpacityInvalid` writes** (deferral #3). Independent.

Order: A → B → C. C could land first, but A → B → C keeps each commit set focused on a single concern.

## Phase A — Cascade `canvas_color`

### Helper signature pattern

Every helper that currently reads `painter.GetCanvasColor()` gains a `canvas_color: emColor` parameter, placed immediately after `painter: &mut emPainter` (matching the `PanelBehavior::Paint` trait convention from `f56f7fa2`). Helper bodies replace `let canvas = painter.GetCanvasColor();` (or inline `painter.GetCanvasColor()` arguments) with the parameter.

### Layer 1 — widget-internal helpers (10 commits)

13 reads in 11 files. Each helper has exactly one caller (its own `Paint` impl), which already has `canvas_color: emColor` in scope post-`988e799d`.

Per-helper commit:

1. Add `canvas_color: emColor` parameter to the helper.
2. Replace `painter.GetCanvasColor()` read with the parameter.
3. Update the single caller in the same file to pass `canvas_color`.
4. `cargo check --workspace --tests` clean → commit.

Files (one helper each unless noted):

- `crates/emcore/src/emButton.rs:191`
- `crates/emcore/src/emCheckBox.rs:135`
- `crates/emcore/src/emCheckButton.rs:97`
- `crates/emcore/src/emColorField.rs:445`
- `crates/emcore/src/emListBox.rs:1221`
- `crates/emcore/src/emRadioBox.rs:165`
- `crates/emcore/src/emRadioButton.rs:352`
- `crates/emcore/src/emScalarField.rs:344`
- `crates/emcore/src/emSplitter.rs:140` (`PaintContent`)
- `crates/emcore/src/emTextField.rs:1234`

(`emTunnel` is already done — `paint_tunnel` derives `content_canvas_color` explicitly. Use that as the template.)

**Pre-commit invariant.** Before each Layer 1 commit, run `grep -rn '\.<helper_name>\b' crates/` to confirm the helper has exactly one caller. If more, escalate to Layer 2 (treat as a shared helper).

### Layer 2 — `emBorder` shared helpers (4 commits)

Four helpers, ~39 production callers across widgets, groups, dialogs, control panels:

- `paint_border` — read at `emBorder.rs:1668`, ~30 callers.
- `paint_label` — delegates to `paint_label_impl`, ~3 callers.
- `paint_label_colored` — delegates to `paint_label_impl`, ~5 callers.
- `paint_label_impl` — 2 reads at `emBorder.rs:1610` and `:1632`, called only by `paint_label` / `paint_label_colored`.

Per-helper commit: add `canvas_color: emColor` parameter, delete the read, update **all** callers in the same commit.

**Caller-side correctness.** Every caller is already inside a `Paint` trait impl that has `canvas_color` in scope. Callers must never invent a value or fall back to `painter.GetCanvasColor()`. If a caller doesn't have `canvas_color` in scope, **stop and re-design** — that's a fidelity question, not a coding question (see `emTunnel::paint_tunnel` precedent: derive explicitly from the panel's declared canvas color).

### Phase A exit criteria

- Zero `painter.GetCanvasColor()` reads in `crates/*/src/`. Test files exempt.
- `cargo check --workspace --tests` + `cargo clippy -- -D warnings` clean.
- `cargo-nextest ntr` full suite passes.
- `scripts/verify_golden.sh --report` shows no regressions vs. baseline at `988e799d`.

## Phase B — Retire painter canvas-color carrier

### What gets removed

1. **`emPainter::canvas_color` field** (the tracked `emColor` member).
2. **`emPainter::GetCanvasColor()` and `emPainter::SetCanvasColor(...)` methods.**
3. **`DrawOp::SetCanvasColor` dispatcher variant** + its handlers in:
   - `crates/emcore/src/render/software_compositor.rs`
   - `crates/emcore/src/render/wgpu_compositor.rs`
   - Any DrawOp encoder/decoder bridging the two.
4. **`painter.SetCanvasColor(...)` call sites in production code** — no-ops with no reader after Phase A.

### What stays

- **`emPanel::GetCanvasColor()` and `emEngineCtx::GetCanvasColor()`** — unrelated APIs returning the panel's *declared* canvas color (the `Look`-derived value the panel announces to its children). They are the *source* the trait `canvas_color` parameter is derived from.
- **`canvas_color` arguments to `PaintRect`, `PaintRoundRect`, `PaintTextBoxed`, etc.** — per-paint-op canvas color used for sub-pixel coverage blend math. Different semantic from the painter's tracked field.
- **Test files reading `painter.GetCanvasColor()` for assertions** — case-by-case. If the test asserts something that no longer exists, delete the assertion. Tests asserting panel-side semantics (e.g. `emFilePanel.rs:642`/`:656` — `panel.GetCanvasColor()`, not `painter`) stay.

### Commits

- **Commit B1:** remove `painter.SetCanvasColor` call sites + `DrawOp::SetCanvasColor` variant + compositor handlers. Field/methods stay temporarily so the workspace builds.
- **Commit B2:** remove the field, `GetCanvasColor`/`SetCanvasColor` methods, fix any test fallout.

### Phase B exit criteria

- `emPainter::canvas_color`, `Get/SetCanvasColor`, `DrawOp::SetCanvasColor` all gone.
- No `painter.SetCanvasColor` call sites in production.
- `cargo check --workspace --tests` + `cargo clippy -- -D warnings` clean.
- `cargo-nextest ntr` full suite passes.
- `scripts/verify_golden.sh --report` shows no regressions.

### Risk note — ops-dump tooling

If `DrawOp::SetCanvasColor` appears in `target/golden-divergence/*.jsonl` streams or `scripts/diff_draw_ops.py` filters, removal requires regenerating the C++ baseline via `scripts/verify_golden.sh --regen`. Acceptable — C++ has no equivalent variant, so its absence is a fidelity improvement.

## Phase C — `WakeUp` pairing

### Signature change

Both `emView::InvalidatePainting` (line 3181) and `emView::invalidate_painting_rect` (line 3196) gain `ctx: &mut SchedCtx<'_>` as the first parameter (matching existing convention, e.g. `WakeUpUpdateEngine(&mut self, ctx: &mut SchedCtx<'_>)`):

```rust
pub fn InvalidatePainting(
    &mut self,
    ctx: &mut SchedCtx<'_>,
    tree: &PanelTree,
    panel: PanelId,
)

pub fn invalidate_painting_rect(
    &mut self,
    ctx: &mut SchedCtx<'_>,
    tree: &PanelTree,
    panel: PanelId,
    x: f64, y: f64, w: f64, h: f64,
)
```

### Body change

Immediately after each `self.SVPChoiceByOpacityInvalid = true;` write (`emView.rs:3188` and `:3237`):

```rust
self.WakeUpUpdateEngine(ctx);
```

C++ ordering: dirty-rect push → `SVPChoiceByOpacityInvalid = true` → `WakeUp()`. Mirror exactly.

### Caller updates

5 production + 3+ test sites:

- `crates/emcore/src/emSubViewPanel.rs:463`
- `crates/emcore/src/emGUIFramework.rs:1394`
- `crates/emcore/src/emWindow.rs:1226`
- `crates/eaglemode/tests/support/pipeline.rs:431`
- `crates/eaglemode/tests/golden/widget_interaction.rs:829`
- `crates/emcore/tests/f018_iv3_svpchoice_invalidation.rs:35,52`
- `crates/emcore/src/emView.rs:5612, 5630, 5638` (in-file tests)

Each caller already runs inside an engine cycle / `SchedCtx` scope. If a caller doesn't have `ctx` in scope, **stop and investigate** — that's an architectural question about where the wake should fire, not a plumbing question.

### New test

Extend `crates/emcore/tests/f018_iv3_svpchoice_invalidation.rs` with one assertion: after `view.InvalidatePainting(ctx, &tree, panel_id)`, the scheduler reports the engine as woken (use whatever scheduler API exists — explore at implementation time).

### Phase C exit criteria

- Every `SVPChoiceByOpacityInvalid = true` write in `emView.rs` is immediately followed by `self.WakeUpUpdateEngine(ctx)`.
- New test assertion proves the pairing.
- `cargo check --workspace --tests` + `cargo clippy -- -D warnings` clean.
- `cargo-nextest ntr` full suite passes.

## Risks

1. **Hidden helper-to-helper calls in Phase A Layer 1.** Mitigation: pre-commit grep confirms exactly one caller; if more, escalate to Layer 2.
2. **Phase A Layer 2 callers without `canvas_color` in scope.** This was the previous subagent failure mode — cascading without bound. Mitigation: any such caller is a stop-and-redesign signal. Use the `emTunnel::paint_tunnel` precedent — derive explicitly, never read from painter.
3. **Phase B `DrawOp::SetCanvasColor` in ops-dump tooling.** Mitigation: regenerate C++ baseline via `--regen` if needed.
4. **Phase C `ctx` not in scope at a caller.** Mitigation: defensive grep before signature change. Investigate, don't paper over.

## Out of scope

- Manual visual verification of the loading-state grey background (separate task, requires `cargo run --release --bin eaglemode`).
- Any restructuring of `emPainter` beyond removing the canvas-color carrier.
- F010 cluster (gates on F018 manual verification, not on this spec).

## Closure

This spec **does not close F018 on its own**. It removes the deferrals. F018's `needs-manual-verification` status advances to `closed` only after the runtime visual confirmation (separate from this spec) is also signed off.
