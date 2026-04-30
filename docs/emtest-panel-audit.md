# emTestPanel Compliance Task List

C++ ground truth: `~/Projects/eaglemode-0.96.4/src/emTest/emTestPanel.cpp`
Rust port: `crates/emtest/src/emTestPanel.rs` + `lib.rs`
Plans: `docs/superpowers/plans/2026-04-30-*.md`

---

## Onboarding Ritual

*Run these steps before touching any task. Do not skip.*

- [ ] Read `CLAUDE.md` — Port Ideology, authority order, code rules, annotation vocabulary.
- [ ] Read `docs/emtest-panel-audit.md` (this file) fully. Note which tasks are already checked.
- [ ] Skim the three plan files to understand task granularity before delegating:
  - `docs/superpowers/plans/2026-04-30-autoexpand-restructure.md`
  - `docs/superpowers/plans/2026-04-30-emtest-compliance-batch.md`
  - `docs/superpowers/plans/2026-04-30-polydrawpanel-port.md`
- [ ] Confirm clean baseline: `cargo-nextest ntr` — must be all green before starting.
- [ ] Confirm branch: `git status` — should be on `main`, clean working tree.

---

## Execution Guide

**You are the orchestrator.** You delegate work to subagents; you do not implement directly. Check off tasks in this file as subagents complete them. Commit the updated file at the end.

### Dependency order

```
Plan 1  ──►  Plan 2   (sequential within Plan 2; annotations batch first)
        └──►  Plan 3   (independent of Plan 2; can run in parallel with Plan 2)
                   └──►  Golden verification  (after Plans 2 and 3 both land)
```

### Approach per group

**Plan 1** (3 tasks): Use `superpowers:subagent-driven-development` on `2026-04-30-autoexpand-restructure.md`. **Must land before Plans 2 or 3 begin** — both depend on the AutoExpand restructure being in place.

**Plan 2 — Annotations batch** (M-1, M-3, M-4, M-5): These four are trivial annotation-only changes. Dispatch a **single subagent** with explicit instructions rather than running full SDD — no two-stage review needed for comment additions. Include `cargo xtask annotations` in the subagent's verification step.

**Plan 2 — Remainder** (all other Plan 2 tasks): Use `superpowers:subagent-driven-development` on `2026-04-30-emtest-compliance-batch.md`, starting at Task 2. Tasks within Plan 2 are sequential (Task 2 `view_context()` gates Tasks 3–10).

**Plan 3** (8 tasks): Use `superpowers:subagent-driven-development` on `2026-04-30-polydrawpanel-port.md`. Start after Plan 1 lands; **independent of Plan 2** — can run in parallel with Plan 2 if using worktrees.

**Golden verification** (M-2, M-7): After Plans 2 and 3 both land, run as a single subagent:
```bash
DUMP_DRAW_OPS=1 cargo test --test golden -- --test-threads=1
python3 scripts/diff_draw_ops.py test_panel --no-table
python3 scripts/diff_draw_ops.py polydrawpanel_default_render --no-table
```
Compare output against C++ baseline. If divergences found, file follow-up tasks.

### Test command (all subagents must pass this before committing)
```bash
cargo-nextest ntr
```

---

## Offboarding Ritual

*Run after all tasks are complete (or at a stopping point). Do not skip.*

- [ ] Run `cargo-nextest ntr` — confirm all tests pass.
- [ ] Run `cargo xtask annotations` — confirm no annotation violations.
- [ ] Run `cargo test --test golden -- --test-threads=1` — confirm no new pixel divergences.
- [ ] Check off all completed tasks in this file (the checkboxes below).
- [ ] Note any tasks not completed and why (blocker, deferred, needs follow-up).
- [ ] Commit the updated task list:
  ```bash
  git add docs/emtest-panel-audit.md
  git commit -m "docs(emtest-audit): mark completed tasks after implementation run"
  ```
- [ ] If any tasks were deferred or blocked, open a follow-up session with the specific blocker described.

---

## Plan 1 — AutoExpand/LayoutChildren Restructure
*Prerequisite for Plans 2 and 3.*

- [ ] **[known-1]** `TestPanel`, `TkTestGrpPanel`, `TkTestPanel`, `PolyDrawPanel`: move first-child creation from `LayoutChildren` to `AutoExpand`; leave `LayoutChildren` for positioning only. `handle_notice_one` gates `LayoutChildren` on `GetFirstChild(id).is_some()` — without this fix no children are ever created.
- [ ] **I-1** `TestPanel::AutoExpand`: call `SetAutoExpansionThreshold(ctx.id, 900.0, Area, ...)` on self. C++ sets it in the constructor (cpp:39); Rust lacks tree access there, so set it in AutoExpand. Currently root panel uses default 150.0 — wrong zoom-level expansion.
- [ ] **I-2** Remove `MAX_DEPTH = 10` constant and the `if self.depth < MAX_DEPTH` guard from `TestPanel::AutoExpand`. Remove `depth: u32` field and parameter from `TestPanel::new`. Rely on AE threshold like C++.

---

## Plan 2 — Compliance Batch

### Annotations
- [ ] **M-1** `make_star` (~line 2433): add `// RUST_ONLY: language-forced utility` comment. C++ inlines these vertices manually (cpp:372–413).
- [ ] **M-3** `emGetInsResImage("emTest", "icons/teddy.tga")` call (~line 486): add `// DIVERGED: (dependency-forced)` comment. C++ uses `emGetInsResImage(GetRootContext(), "icons", "teddy.tga")` → `res/icons/teddy.tga`; Rust cdylib uses `res/emTest/icons/teddy.tga`.
- [ ] **M-4** Remove stale "flat placeholder; Task 11 restructures" comment at ~line 1227. Replace with `// C++ name "PolyDraw" (emTestPanel.cpp:490)`.
- [ ] **M-5** Run `cargo xtask annotations`; fix any DIVERGED blocks missing a category tag (language-forced / dependency-forced / upstream-gap-forced / performance-forced).

### ConstructCtx `view_context()` — prerequisite for I-6 and I-9
- [ ] **I-6a** Add `view_context: Option<&'a Rc<emContext>>` field to `PanelCtx` and `view_context()` method to `ConstructCtx` trait. Implement for all four implementors (`EngineCtx`, `SchedCtx`, `PanelCtx`, `InitCtx`). `emView::Context` (the per-view child context created in `emView::new`) is the view-scoped context.
- [ ] **I-6b** Thread `view_context` through `HandleNotice` and `handle_notice_one` alongside the existing `root_context` parameter. Production call sites (`emView::Update`, `emSubViewPanel`) pass `Some(&self.Context)`; test call sites pass `None`.

### TestPanel Cycle / signal wiring
- [ ] **C-13** Add `Cycle` to `TestPanel`. Watch BgColorField color signal via `ectx.IsSignaled`; on signal: read color from `ColorFieldPanel` behavior via `as_any().downcast_ref`, update `bg_shared`. (cpp:62–71)
- [ ] **C-14** In `TestPanel::AutoExpand`: wire BgColorField's color signal via `ectx.connect(color_signal, ectx.engine_id)` instead of the `on_color` callback. Store signal ID in `TestPanel`. (cpp:495)
- [ ] **I-3** Remove the `on_color` callback and the `BgShared` polling intermediary pattern from BgColorField creation. `bg_shared` is still needed as the backing store read by `Paint` and `Drop`, but it is now written only by `Cycle` via `IsSignaled` — not by a callback.
- [ ] **I-4** In `TkTestPanel::Cycle`: remove the `signals_connected` deferred-connect pattern. Wire signals in `AutoExpand` (or immediately after child creation in `AutoExpand`) rather than deferring to the first `Cycle`. (cpp: signals wired at construction)
- [ ] **I-5** Fix `sf5↔sf6` Cell-based value pipe in `ScalarFieldWithDynamicMax::Cycle` (~lines 261–269). `sf6_max` Cell is written in sf5's `on_value` callback and drained by sf6's `Cycle` — prohibited polling intermediary per CLAUDE.md. Wire sf5's value signal directly to sf6's engine via `ectx.connect` and read the value synchronously in sf6's Cycle via `IsSignaled`. (cpp:638: `AddWakeUpSignal(SFLen->GetValueSignal())`)

### TestPanel Notice + Input
- [ ] **C-12** Add `notice()` to `TestPanel::PanelBehavior`. Body: `ctx.UpdateControlPanel()` (no-op in Rust — omit) + repaint is automatic. The C++ impl (cpp:74–78) calls both; in Rust the view repaints every frame, so the impl body can be empty, but the method must exist for correctness. (cpp:74–78)
- [ ] **C-15** Fix `TestPanel::Input` (~line 1145): (a) add `STATE: pressed=k1,k2,...` by scanning `input_state` over all `InputKey` variants; (b) remove stale `_input_state` discard; (c) add forwarding comment at end matching C++ `emPanel::Input` call. (cpp:88–105)

### emVarModel scope + count
- [ ] **I-6c** In `TestPanel::AutoExpand` and `Drop`: switch emVarModel calls from `&self.root_ctx` to `ctx.view_context().unwrap_or(&self.root_ctx)`. Store `view_ctx: Option<Rc<emContext>>` in `TestPanel`, set it in `AutoExpand`. (cpp:32–48)
- [ ] **I-19** Add `count: usize` parameter to `emVarModel::Set` in `emVarModel.rs`. Pass `10` at the `TestPanel::Drop` call site. (cpp:47)

### TestPanel structural
- [ ] **I-10** At `TestPanel::Paint` (~line 1091) where `state.window_focused` is used as `IsViewFocused()`: add `// DIVERGED: (language-forced) C++ IsViewFocused() is per-view; Rust PanelState has window_focused (per-window). Observable only with multiple views per window.`

### TestPanel Paint
- [ ] **C-18** Fix linear gradient: replace collapsed `paint_linear_gradient` call with `PaintRect(0.2, 0.94, 0.02, 0.01, emLinearGradientTexture(0.207, 0.944, 0x00000080, 0.213, 0.946, 0x80808080))`. (cpp:415–419)
- [ ] **C-19** Fix radial gradient: replace with `PaintRect(0.221, 0.94, 0.008, 0.01, emRadialGradientTexture(0.223, 0.941, 0.004, 0.008, 0xFF8800FF, 0x005500FF))`. (cpp:420–423)
- [ ] **C-22** Fix gradient ellipse: replace solid `PaintEllipse` with `PaintEllipse(0.23, 0.94, 0.02, 0.01, emRadialGradientTexture(0.23, 0.94, 0.02, 0.01, 0, 0x00cc88FF))`. (cpp:425–428)
- [ ] **C-20** Fix image tile: replace `paint_image_scaled` with `PaintRect(0.26, 0.94, 0.02, 0.01, emImageTexture(0.26, 0.94, 0.001, 0.001*ratio, TestImage))` where texture width is 0.001, not 0.02. (cpp:430–435)
- [ ] **C-16** Add `emImageColoredTexture` rect after image tile: `PaintRect(0.2625, 0.942, 0.02, 0.01, emImageColoredTexture(1.0005, 0.942, 0.001, 0.001*ratio, TestImage, 0x00FFFFFF, 0xFF0000FF))`. (cpp:441–451)
- [ ] **C-17** Add three extend-mode rects at y=0.907/0.910/0.913: `PaintRect(0.275, y, 0.002, 0.002, emImageTexture(0.2755, y+0.0005, 0.001, 0.001, TestImage, 50, 10, 110, 110, 255, EXTEND_TILED/EDGE/ZERO))`. (cpp:453–478)
- [ ] **C-21** Fix caption text alignment: inner horizontal `AlignmentH::Center` (not `Left`); `formatTallness` 0.2 (not 0.5). (cpp:134–141)
- [ ] **C-25** Add annotation at `push_state/SetClipping` block (~line 574): `// DIVERGED: (language-forced) C++ creates sub-painter with restricted origin/scale (cpp:225–231); second &mut emImage borrow forbidden by borrow checker. Workaround clips correctly but does not shift origin — add to golden verification.`
- [ ] **M-2** Golden verification: confirm Rust `painter.scale(w, w)` coordinate space matches C++ `[0,1]×[0,h]` space. Run `cargo test --test golden test_panel` after paint fixes; compare with C++ baseline using `scripts/diff_draw_ops.py`.
- [ ] **M-7** Golden verification: confirm `paint_polygon_even_odd` produces the same pixel output as C++ `PaintPolygon` with even-odd winding. Check in the same golden run.

### CanvasPanel interaction
- [ ] **C-8** In `CanvasPanel::Input`: on any left-press, call `event.eat()` (lowercase) unconditionally before the vertex search. Omit the C++ `Focus()` call — Rust focus-on-click is handled by the window dispatch loop before `Input` fires. (cpp:1315–1317)
- [ ] **C-7** Replace three hardcoded `1.0` y-bounds with `panel_h` (= `state.layout_rect.h`): (a) drag clamp `raw_y.clamp(0.0, 1.0)` → `.clamp(0.0, panel_h)`; (b) ShowHandles check `(0.0..1.0).contains(&my)` → `my >= 0.0 && my < panel_h`. (cpp:1341, 1354)
- [ ] **C-9** Add change-guard before vertex write: `if self.vertices[idx] != (x, y) { self.vertices[idx] = (x, y); }`. Remove the current unconditional overwrite. (cpp:1344–1347). `InvalidatePainting` calls are no-ops in Rust (view repaints every frame) — omit them.
- [ ] **C-10** Fix handle radius: `let r = state.ViewToPanelDeltaX(12.0).min(0.05)`. Check `PanelState` for `ViewToPanelDeltaX`; if absent, compute `12.0 / (state.viewed_rect.w / w)`. (cpp:1464)
- [ ] **C-11** Fix help-text: `p.PaintTextBoxed(0.0, h - 0.03, 1.0, 0.03, ..., 0.03)`. Remove the `* h` multiplications. (cpp:1485–1490)
- [ ] **C-26** At end of `CanvasPanel::Input`: add `// C++ cpp:1361: emPanel::Input(event,state,mx,my) — base handles cursor; Rust base Input is a no-op; call preserved for fidelity.`

### CustomListBox
- [ ] **C-23** Add `AutoExpand` to `CustomItemBehavior`: create `LabelPanel { emLabel("This is a custom list\nbox item panel (it is\nrecursive...)") }` with listbox's look, then `ListBoxPanel { emListBox }` with `SelectionMode::Multi`, items "1"–"7", index 0 selected. (cpp:941–956)
- [ ] **C-24** Add `Input` to `CustomItemBehavior`: call `self.process_item_input(event, state, ctx)` then `self.group.Input(event, state, input_state, ctx)`. (cpp:932–938)
- [ ] **I-11** Add `item_text_changed` override to `CustomItemBehavior`: `self.group.SetCaption(new_text)`. (cpp:959–962)
- [ ] **I-12** On the `emListBox` in `TkTestPanel::create_all_categories` (the CustomListBox lb7 ~line 1854): add `lb.SetChildTallness(0.4)`, `lb.SetAlignment(AlignmentH::Left, AlignmentV::Top)`, `lb.SetStrictRaster()`. (cpp:992–994)
- [ ] **I-13** Fix look capture in `CustomItemBehavior` factory: use the listbox's live look (`lb.GetLook()` after `lb` is constructed) rather than the outer `lb7_look` captured at factory creation. (cpp:980)

### Dialog + structural
- [ ] **I-7** Fix dialog construction order in `TkTestPanel`: move `set_view_window_flags` before `AddNegativeButton`, matching C++ flag-before-buttons order. (cpp:799–803)
- [ ] **I-9** After I-6 lands: un-DIVERGE `CbTopLev` — use `ctx.view_context()` vs `ctx.root_context()` to select dialog parent. If `emDialog::new` doesn't expose parent-context selection, update the existing DIVERGED block to `dependency-forced` and keep it. (cpp:790)

---

## Plan 3 — PolyDrawPanel Full Port

- [ ] **I-15** `PolyDrawPanel::new`: add caption "Poly Draw Test" and description via `group.SetCaption` / `group.SetDescription`. (cpp:1005–1009)
- [ ] **I-14** `PolyDrawPanel::new`: call `group.set_orientation_threshold_tallness(1.0)` if `emLinearGroup` supports it; else add `// DIVERGED: upstream-gap-forced` comment. (cpp:1011)
- [ ] **C-1** `PolyDrawPanel::AutoExpand`: build full 22-widget control tree — Controls raster layout, four sub-groups (general/stroke/strokeStart/strokeEnd), 16-radio Method group, VertexCount, FillColor, WithCanvasColor, StrokeWidth, StrokeColor, StrokeRounded, StrokeDashType (4 radios), DashLengthFactor, GapLengthFactor, StrokeStartType (17 radios), StrokeStartInnerColor, StrokeStartWidthFactor, StrokeStartLengthFactor, StrokeEndType (17 radios), StrokeEndInnerColor, StrokeEndWidthFactor, StrokeEndLengthFactor. Store `Rc<RefCell<RadioGroupData>>` handles for four radio groups; store panel IDs for scalar/color widgets. (cpp:1071–1261)
- [ ] **C-2** `PolyDrawPanel::Cycle`: watch 18 signals via `ectx.IsSignaled`; on any: read all control values, build `emStroke` and two `emStrokeEnd` values, call `canvas.setup(...)`. Note: `emStroke` has no `rounded` field in the Rust port — add `// DIVERGED: upstream-gap-forced` at that site. (cpp:1015–1068)
- [ ] **C-6** `CanvasPanel::setup`: resize vertex array; for new vertices use `cos/sin * 0.4 + 0.5` for x and `GetHeight() * (sin * 0.4 + 0.5)` for y (panel height from layout rect). (cpp:1284–1298)
- [ ] **C-5** `CanvasPanel::Paint`: add `WithCanvasColor` branch — if true: `Clear(emColor(96,128,160), canvas_color)`; if false: `Clear(emLinearGradientTexture(0,0,rgb(80,80,160),0,h,rgb(160,160,80)), canvas_color)` and zero canvas color. (cpp:1372–1386)
- [ ] **C-3** `CanvasPanel::Paint`: replace unconditional `PaintPolygon` with 16-way switch on `render_type` (0=PaintPolygon … 15=PaintRoundRectOutline). Derive `x1,y1,x2,y2,x,y,w,h,sa,ra` from vertex array per cpp:1388–1403. (cpp:1405–1461)
- [ ] **C-4** `CanvasPanel::Paint` handles loop: color depends on `render_type` and vertex index — yellow (`rgba(255,255,0,128)`) for non-anchor bezier points (types 3–5, `i % 3 != 0`); gray (`rgba(128,128,128,128)`) for vertices beyond active count `m`; green otherwise. Blend with white at 75% for drag vertex. (cpp:1463–1483)

---

## Deferred — DIVERGED (no plan)

- **[known-2]** `TkTestGrpPanel` emSplitter hierarchy — C++ builds `sp → sp1/sp2 → TkTest`; Rust uses hardcoded 2×2 grid. DIVERGED annotation present. Blocked on emSplitter port.

---

## Closed

| ID | Reason |
|----|--------|
| I-8 | `EnableAutoDeletion(true)` ≡ C++ no-arg — not a bug |
| I-16 | `viewed_rect.w` is view pixels ≡ `GetViewCondition(VCT_WIDTH)` — not a bug |
| I-17 | Paint guard placement verified correct — not a bug |
| I-18 | `memory_limit` is `u64`, formats correctly — not a bug |
| M-6 | Same finding as I-7 (dialog construction order) |
| M-8 | `emColor::rgba(187,255,255,255)` ≡ 3-arg — equivalent |
| M-9 | `emCrossPtr` → name-lookup — idiom adaptation, no action |
| M-10 | `TkTest` border scaling / child tallness — confirmed correct |
| M-11 | RadioGroup shared across r1–r6 — confirmed correct |
