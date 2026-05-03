# has_awake idle findings — 2026-05-03

Capture: `/tmp/em_instr.idle.log`
Branch: `instr/hang-2026-05-02` @ `instr-7-loop-chain` (`38637b08`)
Threshold: 80%

## Window
153 slices, 60.00s

## Per-engine-type aggregation

| engine_type | cycles | stay_awake_pct | ext_wakes | classification |
|---|---:|---:|---:|---|
| `emcore::emInputDispatchEngine::InputDispatchEngine` | 151 | 0.0% | 0 | episodic |
| `emcore::emWindowStateSaver::emWindowStateSaver` | 4 | 0.0% | 4 | externally-rewoken |
| `emcore::emView::UpdateEngineClass` | 144 | 0.0% | 144 | externally-rewoken |
| `emcore::emRecListener::ListenerEngine` | 0 | 0.0% | 151 | never-awake |

## Offenders
- `emcore::emWindowStateSaver::emWindowStateSaver` — externally-rewoken (cycles=4, stay_awake=0.0%, ext_wakes=4)
- `emcore::emView::UpdateEngineClass` — externally-rewoken (cycles=144, stay_awake=0.0%, ext_wakes=144)

## External-wake caller breakdown
### `emcore::emWindowStateSaver::emWindowStateSaver`
- `crates/emcore/src/emScheduler.rs:954` — count=4

### `emcore::emView::UpdateEngineClass`
- `crates/emcore/src/emView.rs:3421` — count=144

## Verdict

The post-fix idle baseline still has `has_awake==1` ≈66.7% of slices because
`emView::UpdateEngineClass` is woken ~2.4 Hz from `WakeUpUpdateEngine`
(`emView.rs:3421`) by callers further up the stack that this instrumentation
cannot see — `WakeUpUpdateEngine` itself was not in the Task 7 list of
wrappers tagged `#[track_caller]`, so the resolved caller bottoms out at the
`ctx.wake_up(id)` line inside its body. Each cycle's `Cycle()` returns
`stay_awake=false` (engine releases itself), but something in the per-frame
or per-tick path keeps re-poking it. `emWindowStateSaver` (4 wakes from the
signal-drain loop) and `emRecListener::ListenerEngine` (151 wakes recorded
but no observed cycles — analyzer quirk: cycles=0 because every WAKE in the
window happened before any STAYAWAKE record exists for it, suggesting the
engine is registered but its scope dispatch path doesn't go through
`DoTimeSlice`'s match arm for some reason) are secondary.

The interesting question — *who calls `WakeUpUpdateEngine`?* — is one
inspection step away: tag `WakeUpUpdateEngine` with `#[track_caller]` and
re-capture, or simply grep for its callers. The fix shape (if any) cannot
be decided from this capture alone; spec B1 should compare C++
`UpdateEngine->WakeUp()` call sites against the Rust ports.

## Next steps

- [ ] Grep `WakeUpUpdateEngine` callers and compare against
  `UpdateEngine->WakeUp()` sites in
  `~/Projects/eaglemode-0.96.4/src/emCore/emView.cpp`. If Rust calls this
  from places C++ does not, that is the bug.
- [ ] Investigate the `emRecListener::ListenerEngine` cycles=0/ext_wakes=151
  classification quirk — either an analyzer bug or a real instrumentation
  gap (e.g., engine is woken but its scope-dispatch arm in `DoTimeSlice` is
  missing the STAYAWAKE emission).
- [ ] Add `#[track_caller]` to `WakeUpUpdateEngine` and other domain-specific
  wake helpers (the Task 7 instrumentation only covered the generic
  scheduler/context wrappers).
- [ ] Defer fix decision until B1 is written; idle CPU at 0.4% remains
  acceptable post-blink-fix.
