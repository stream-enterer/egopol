# Phase 1 — Scheduler Event-Loop Threading — Ledger

**Started:** 2026-04-19 (resume session)
**Branch:** port-rewrite/phase-1
**Baseline:** see 2026-04-19-phase-1-baseline.md
**Spec sections:** §3.1, §3.1.1, §3.3, §3.7 (framework_actions only), §4 D4.1–D4.11
**JSON entries to close:** E001, E002, E003, E004, E005, E007, E008, E009, E010, E011, E036

## Task log

- Task 1 done @ 0bb61f0. register_engine argument-order decision: adapters expose target order `(behavior, priority)`; bodies call legacy `scheduler.register_engine(pri, b)`. Task 3 will flip the scheduler side.
- Task 1 note: name collision — `emEngine::EngineCtx` (existing, cycle-context) coexists with new `emEngineCtx::EngineCtx`. Both `pub`; different module paths. No rename performed. May warrant reconciliation in later phase.
- Task 1 note: `SignalId`/`EngineId`/`Priority` are re-exports only internally in `emScheduler`; imports sourced from `emSignal` and `emEngine` modules directly.
  - Task 1 quality-review fixes @ 152460b
  - Task 1 allow(dead_code) removed; scaffolding exercised via tests @ fe3b7a6

## Task 2
- Task 2 done @ fad907f. `App.scheduler` now a plain `EngineScheduler` value; `framework_actions: Vec<DeferredAction>` and `pending_inputs: Vec<(WindowId, emInputEvent)>` added. New test `framework_scheduler_is_plain_value` passes.
- Scope deviation: `windows: HashMap<WindowId, Rc<RefCell<emWindow>>>` left wrapped (not narrowed to plain value). Narrowing cascades into many call sites in emWindow / materialize_popup_surface / view wiring; out of scope for Task 2. Flag for Phase 1 revisit or dedicated follow-up.
- Scope deviation: `emContext::NewRootWithScheduler(Rc<RefCell<EngineScheduler>>)` constructor call in `App::new` replaced with `NewRoot()` + TODO marker. Task 8 must wire the scheduler through ConstructCtx when emContext is ported.
- Compile state: clean compile, clippy `dead_code` warning on `framework_actions`/`pending_inputs` (fields unused until Task 3-5 wire-up). Committed with `--no-verify` per plan line 302 (intermediate-red allowed on long-running phase branches). Expected closure: Task 3 consumes `framework_actions`; Task 4/5 drain `pending_inputs`.
- Breakages elsewhere: none. Changes isolated to `emGUIFramework.rs`. DoTimeSlice legacy signature still used (Task 3 will change signature; no ripple now).
