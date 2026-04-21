# Phase 3.5 — emDialog as emWindow — Ledger

**Started:** 2026-04-22
**Branch:** port-rewrite/phase-3-5-emdialog-as-emwindow
**Baseline:** see docs/superpowers/notes/2026-04-19-phase-3-closeout.md (nextest 2476/0/9; goldens 237/6; rc_refcell_total 256).
**Plan:** docs/superpowers/plans/2026-04-21-port-rewrite-phase-3-5-emdialog-as-emwindow.md
**Source brainstorm:** docs/superpowers/plans/2026-04-21-port-rewrite-phase-3-5-emdialog-as-emwindow-plan.md
**JSON entries:** E024 remains open (closed in Phase 3.6, not here).

## Bootstrap decisions

See plan §"Bootstrap decisions" (B3.5a–B3.5e).

## Task log

- **Task 1 — Audit:** COMPLETE.
  - 1a: ViewFlags::POPUP_ZOOM and ROOT_SAME_TALLNESS present at emView.rs:22 bitflags block (POPUP_ZOOM line 23, ROOT_SAME_TALLNESS line 28). PASS (no gap-fill).
  - 1b: DeferredAction::CloseWindow(winit::window::WindowId) present at emEngineCtx.rs:37. PASS.
  - 1c: `on_finished: Option<WidgetCallbackRef<DialogResult>>` per D1 (brainstorm decision). Matches Rust port's existing virtual-to-callback rendition (emButton.on_click, emDialog.on_check_finish). No audit gap. Task 5 (emDialog reshape) adds the field.
  - 1d: close_signal firing confirmed on WindowEvent::CloseRequested in emGUIFramework.rs:399-420. Modal (WF_MODAL) top-level windows traverse the same `self.windows` branch (lines 406-411), so close_signal fires identically for modal and non-modal on user-requested close. PASS.
