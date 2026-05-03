# `emView::set_active_panel` missing `WakeUpUpdateEngine` — divergence note for B1

Found during B2 spec exploration on 2026-05-03 at main `3bb7bd37`.

## Divergence

C++ `emView::SetActivePanel` (`~/Projects/eaglemode-0.96.4/src/emCore/emView.cpp:307`):

```cpp
ActivePanel = panel;
ActivationAdherent = adherent;
InvalidateHighlight();
TitleInvalid = true;
UpdateEngine->WakeUp();      // <-- explicit wake
Signal(ControlPanelSignal);
```

Rust `emView::set_active_panel` (`crates/emcore/src/emView.rs:1791-1793` on
main `3bb7bd37`):

```rust
for id in notice_ids {
    tree.queue_notice(id, flags, None);  // None for sched
}
```

`tree.queue_notice(..., None)` calls `add_to_notice_list(..., None)`,
which takes the wake-up branch only when `sched` is `Some`. So the Rust
port silently relies on incidental wakes from elsewhere (e.g.,
subsequent `InvalidateHighlight`, input-dispatch side-effects) to flush
the queued FOCUS_CHANGED notice.

## Why this is deferred from B2

Adding the wake-up call increases `UpdateEngine` wake cadence, which is
the exact concern B1 (the `has_awake==1` 66.7%-of-slices work) is trying
to reduce. B1 should make the call about whether to add this wake source.

## Why this is real

In the B2 Phase 0 capture (`/tmp/em_instr.blink-trace.log`,
2026-05-03), the FOCUS_CHANGED notice DID dispatch (the
`NOTICE_FC_DECODE` log entry was present at PanelId(125v1) post-click —
see `docs/scratch/2026-05-03-blink-trace-results.md`), so the missing
wake-up was not load-bearing for the blink symptom in this run. But in
any scenario where no other engine wakes the `UpdateEngine` between the
click and the next idle frame, the focus notice would be silently
delayed by up to one timeslice (~50 ms). This is observable as
occasional "click did not feel responsive" without any error trace.

## Recommended action for B1

Decide whether to:

1. Add the explicit `view.WakeUpUpdateEngine(ctx)` call after the
   `for id in notice_ids` loop in `set_active_panel` (mirrors C++).
2. Leave it as-is and document the latent fragility (matches current
   behavior; saves wake cadence).
3. Pass `Some(sched)` to `queue_notice` so the wake propagates without
   an additional call (cleanest if `set_active_panel`'s caller already
   has sched access — which it does, via the `ctx` parameter).
