# Blink path-trace Phase 0 results — 2026-05-03

Capture: `/tmp/em_instr.blink-trace.log` (9.5 MB, 2 markers)
Branch: `instr/blink-trace-2026-05-03` @ `71260017`
Run: 2026-05-03 12:51 (open marker wall_us=19288047, close marker wall_us=87398740)

## Headline verdict (analyzer)

**O1** — `in_active_path` stale at notice time.

Decisive event picked by the analyzer's path-trace heuristic:
- `wall_us=35453391`, `panel_id=PanelId(19v5)`, `behavior_type=emTestPanel::emTestPanel::TextFieldPanel`
- `in_active_path=False`, `window_focused=False`, `flags=0x3ff`, `branch_fired=False`

Per the spec dispatch table, O1 → Phase 1a (fix in `set_active_panel` /
`build_panel_state`).

## Re-analysis (manual) — verdict is misleading

The analyzer picked PanelId(19v5) as the focus-transition target, but
that panel is part of a **layout-driven blanket FOCUS_CHANGED flush** at
wall_us≈35453000–35453500 affecting *every* panel in the test sub-view
with `iap=f, wf=f` regardless of behavior type. It is not the user's
click event.

The user's actual click landed at **wall_us=48655320** on
**PanelId(125v1)** — see `SET_ACTIVE_RESULT|target_panel_id=PanelId(125v1)|window_focused=t|notice_count=12`,
with the matching FOCUS_CHANGED notice for that panel at:

```
NOTICE_FC_DECODE|wall_us=48655524|panel_id=PanelId(125v1)|behavior_type=emTestPanel::emTestPanel::TextFieldPanel|in_active_path=t|window_focused=t|flags=0xf0
```

So at the *real* click target the focus-path bookkeeping is **correct
at notice dispatch time**: `iap=t`, `wf=t`, hence
`state.in_focused_path()=true`. The notice handler's `if self.is_focused`
branch *should* run.

**Yet zero `WAKE` entries from `emTestPanel.rs:255` (the
`ctx.wake_up_panel(id)` call in the focused branch) appear in the entire
log.** The only WAKEs from `emTestPanel.rs` come from lines 1555, 2370,
3200 — none of which is the cursor-blink wake site. This means the
focused-branch wake call is either not running, or it is running and
silently no-opping inside `wake_up_panel`.

`wake_up_panel` has two early-return guards (`emEngineCtx.rs:847`):

```rust
pub fn wake_up_panel(&mut self, id: PanelId) {
    let Some(panel) = self.tree.GetRec(id) else { return; };
    let Some(eid) = panel.engine_id else { return; };       // ← silent
    if let Some(sched) = self.scheduler.as_deref_mut() {    // ← silent
        sched.wake_up(eid);
    }
}
```

If `panel.engine_id` is `None` (the cursor-blink engine is not yet
registered for this panel) or `self.scheduler` is `None` (the
notice-dispatch `PanelCtx` was built without a scheduler), the wake
silently no-ops without producing a `WAKE` log line.

Reframing the actual data: `iap=t && wf=t && !branch_fired` matches the
truth-table row **O3-AMBIG**, not O1. Per the spec, O3-AMBIG → "STOP:
re-brainstorm B2.1 (impossible-row outcome; investigate)".

## Branch fired (WAKE within 100ms window)?

**For the analyzer's chosen target (PanelId(19v5)):** `f`
**For the actual click target (PanelId(125v1)):** `f`

Either way: `branch_fired=False`.

## Verdict

- **As reported by the analyzer:** O1
- **As supported by the data when retargeted to the user click:** O3-AMBIG

Per B2 spec dispatch:
- O1 → proceed to Phase 1a
- O2 → proceed to Phase 1b
- O3 / O3-AMBIG / O4 → STOP, re-brainstorm B2.1

The data fits O3-AMBIG. Proceeding to Phase 1a on the analyzer's stated
verdict would be acting on a wrong target panel and a stale-flush event
that is not the bug under investigation.

## Prediction calibration (advisor's check)

- Pre-measurement priors: 60% O1, 25% O2, 10% O3, 5% O4
- Headline (analyzer): O1 — within the modal prior; falsely calibrated.
- Real (after retargeting): O3-AMBIG — within the long tail (~10%).
- Retrospective: code reading suggested both `iap` and `wf` should be
  true at notice dispatch. The data confirms this for the actual click
  target. The bug is *downstream* of the focus-path system, in the
  notice handler → `wake_up_panel` → engine registration chain, not in
  the focus-path bookkeeping. The 60%-O1 prior was wrong. The original
  A2 hypothesis ("`state.in_focused_path()` returned false") is also
  wrong — it returns true; the bug is one layer further down.

## Analyzer-heuristic finding (B2 follow-up)

The analyzer's blink-command path-trace identifies the focus-transition
target by scanning ACTIVATE/transition events early in the post-marker
window. It locked onto a layout-driven FOCUS_CHANGED flush at
wall_us=35453000–35453500 instead of the user's click at
wall_us=48655320. This is a re-discovery of the same heuristic
limitation flagged in the A2 findings (PanelCycleEngine scope mapping)
and should be tracked for analyzer hardening, but it is not load-bearing
for the B2 investigation now that the data has been read manually.

## Full analyzer report

```
## Path-trace verdict (transition)

Focus-change identified at +16165.3ms (`PanelId(19v5)`, `emTestPanel::emTestPanel::TextFieldPanel`).

- ✓ **NOTICE FOCUS_CHANGED → TextFieldPanel** — `NOTICE|wall_us=35453390|recipient_panel_id=PanelId(19v5)|flags=0x3ff`
- ✗ **Engine REGISTER for PanelCycleEngine** — no REGISTER record matches target panel

## Identified break

First ✗: **Engine REGISTER for PanelCycleEngine**.

_Next step: spec B2 — investigate Engine REGISTER for PanelCycleEngine._


## Phase 0 verdict: O1
  decisive_event: wall_us=35453391, panel_id=PanelId(19v5), behavior_type=emTestPanel::emTestPanel::TextFieldPanel
    in_active_path=False, window_focused=False, flags=0x3ff, branch_fired=False

## Phase 0 dispatch
→ Phase 1a: in_active_path stale; fix in set_active_panel/build_panel_state
```

## Supporting raw events (manual reanalysis)

```
SET_ACTIVE_RESULT|wall_us=48655320|target_panel_id=PanelId(125v1)|window_focused=t|notice_count=12|sched_some=f
NOTICE|wall_us=48655523|recipient_panel_id=PanelId(125v1)|recipient_type=emTestPanel::emTestPanel::TextFieldPanel|flags=0xf0
NOTICE_FC_DECODE|wall_us=48655524|panel_id=PanelId(125v1)|behavior_type=emTestPanel::emTestPanel::TextFieldPanel|in_active_path=t|window_focused=t|flags=0xf0
[next 100ms: 8 WAKE entries; none from emTestPanel.rs:255]
```
