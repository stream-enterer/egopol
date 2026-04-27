# B-018-fileDialog-singleton — Design (no-op + reclassify)

**Bucket:** B-018-fileDialog-singleton
**Pattern (audit):** P-008-connect-with-poll-fallback
**Row count:** 1 (`emFileDialog-196`)
**Disposition:** **False positive — reclassify drifted → faithful.** No code changes.
**Status flow:** B-018 closes as designed; no implementation PR.

## Summary

The audit flagged `emFileDialog-196` as drift because a `scheduler.connect(...)` call (`emFileDialog.rs:733`) coexists with an `IsSignaled(...)` poll (`emFileDialog.rs:169`) and a second wake-up wiring (`emFileDialog.rs:516`) on the same `od.finish_signal`. Reading C++ settles the question: this is the **canonical emEngine subscribe-and-poll-in-Cycle shape**, not a hybrid drift. All three Rust sites are faithful mirrors of C++. The row should be reclassified out of the drift inventory; pattern P-008 retires (singleton, category error in the audit's classification scheme).

## Diagnostic finding

### C++ shape (`src/emCore/emFileDialog.cpp`)

| Line | Code | Role |
|---|---|---|
| 90 | `if (OverwriteDialog && IsSignaled(OverwriteDialog->GetFinishSignal())) { ... }` (inside `Cycle()`) | Poll/consume — checks each Cycle whether OD finished, reads result, finishes outer |
| 196 | `AddWakeUpSignal(OverwriteDialog->GetFinishSignal());` (inside `CheckFinish()`, immediately after `OverwriteDialog=new emDialog(...)` at line 188) | Subscribe — arms the outer engine to wake when OD's finish signal fires |

These are **complementary, not redundant**. `AddWakeUpSignal` registers the engine subscription; `IsSignaled` inside `Cycle()` is the standard consumption probe. This is the canonical emEngine pattern — both are required.

### Rust three sites mapped to C++

| Rust site | Code | C++ mirror | Verdict |
|---|---|---|---|
| `emFileDialog.rs:169` | `if ctx.IsSignaled(od_sig) { … }` inside `on_cycle_ext` | `cpp:90` | ✅ Faithful — `EngineCtx::IsSignaled` (emEngineCtx.rs:180) is the documented "Rust equivalent of C++ `emEngine::IsSignaled()`" (engine-scoped, depends on subscription) |
| `emFileDialog.rs:516` | `self.dialog.add_pre_show_wake_up_signal(od.finish_signal)` inside `CheckFinish` (pre-show branch, `pending.is_some()`) | `cpp:196` | ✅ Faithful — pre-show wiring; outer private engine not yet constructed |
| `emFileDialog.rs:733` | `ctx.scheduler.connect(od_finish_sig, outer_engine_id)` inside `run_file_dialog_check_finish` (post-show standalone helper) | `cpp:196` | ✅ Faithful — post-show wiring; outer engine exists, direct subscribe |

The 516/733 split is **idiom adaptation forced by the Rust outer-engine lifecycle** (the private engine is constructed at `Dialog::show()`, not at `CheckFinish` entry). C++ doesn't face this because `emEngine` attachment timing differs. The split preserves observable wiring: a single OD spawn traverses exactly one of the two paths, and both end with the outer engine subscribed to OD's finish signal. No double-subscribe occurs in production.

### Verification (per user-requested checks before accepting "false positive")

**Check 1 — rs:169 is a canonical engine-state probe, not an independent poll.** ✅
`ctx.IsSignaled(od_sig)` resolves to `EngineCtx::IsSignaled` (emEngineCtx.rs:180), whose body is `self.scheduler.is_signaled_for_engine(signal, self.engine_id)`. Doc-comment (emEngineCtx.rs:176-179): *"Check whether a specific signal has been signaled since the last time this engine's `Cycle` was called. Rust equivalent of C++ `emEngine::IsSignaled()`."* This API depends on the engine being subscribed (woken via the connect). It is not a `signal.is_set()` / `od_sig.fired()` independent poll.

**Check 2 — rs:516 and rs:733 are mutually exclusive paths per OD spawn.** ✅
- `CheckFinish` method (rs:462) is documented at rs:457-461 as the **pre-show wrapper**: *"This wrapper only handles the pre-show tree-reach shape… Post-show calls go directly through `run_file_dialog_check_finish`."*
- The pre-show branch (rs:515) gates on `self.dialog.pending.is_some()` and wires via `add_pre_show_wake_up_signal` (rs:516).
- `run_file_dialog_check_finish` (rs:677) is the post-show production path, invoked from the `on_check_finish` closure installed at rs:239-263 (called at rs:252). It calls `ctx.scheduler.connect(...)` at rs:733 — direct subscribe, outer engine exists.
- A given OD spawn goes through exactly one of these surfaces; rs:516 and rs:733 cannot both fire for the same OD instance.

## Disposition

| Site | Action |
|---|---|
| `emFileDialog.rs:169` | **Keep as-is.** Canonical poll. |
| `emFileDialog.rs:516` | **Keep as-is.** Pre-show subscribe wiring; mirrors C++:196. |
| `emFileDialog.rs:733` | **Keep as-is.** Post-show subscribe wiring; mirrors C++:196. |

No additions. No removals. No restructuring. The "three wiring sites" surface is structurally correct; collapsing them would either break the pre/post-show lifecycle split (forced by Rust ownership) or remove the canonical poll-in-Cycle consumer.

No D-006 application: the bucket needs no wiring shape because nothing changes.

## D-### proposals

None. The diagnosis closes the bucket without invoking new global decisions. P-008 itself can retire as a singleton false positive (deferred to reconciliation; not a B-018 deliverable).

## Prereq edges

None. Specifically:

- **No edge to B-013-watch-list-graduation.** B-013's framework gap (sync post-show `GetResult`) is unrelated. emFileDialog reads `od.finalized_result` asynchronously via the `pending_actions` closure at rs:179-223 after the finish signal fires — this is the canonical emCore pattern (mirroring C++:91-105) and explicitly does *not* require a sync post-show read. The B-013 architectural concern does not generalize to this site.
- No edge to any other bucket.

## Audit-data corrections

1. **Reclassify `emFileDialog-196`:** drifted → faithful. The row's `connect_call` evidence is correctly observed but the verdict-flip rule misfired: the coexistence of `IsSignaled` with a `scheduler.connect` is the canonical emEngine shape, not drift. Update `inventory-enriched.json` (or equivalent spine artifact) accordingly.

2. **Retire pattern P-008-connect-with-poll-fallback** in `pattern-catalog.md`: singleton bucket whose only member reclassifies as faithful. Mark P-008 with a "retired — false positive" note rather than deleting (preserves audit trail).

3. **New observation (out of B-018 scope, surface for separate handling):** `emFileDialog::CheckFinish` post-show else branch at `emFileDialog.rs:532-543` parks the OD via `pending_actions` but does **not** call `scheduler.connect(od.finish_signal, outer_engine_id)`. Caller analysis (rs:806, 815, 1129, 1228, 1332) shows this branch is **only exercised by `#[cfg(test)]` callers** in the current codebase — production post-show flow routes through `run_file_dialog_check_finish` (rs:677). The branch is therefore a latent gap that doesn't affect production, but if a future caller invokes `CheckFinish` post-show outside tests, the OD's finish signal will not wake the outer engine. Recommend filing as a separate audit follow-up (not a B-018 deliverable, not necessarily even a drift item — closer to "test-only path missing parity wiring").

## Verification approach

No new tests. The existing emFileDialog test suite (callers at rs:806/815/1129/1228/1332 plus the integration paths exercised by `run_file_dialog_check_finish`) already covers the OD finish-signal flow end-to-end. Reclassification is a metadata change; no behavior changes, no test changes.

## Reconciliation handoff

Reconciler should:
1. Mark B-018 status `designed (no-op)` and `mergeable` in `work-order.md`.
2. Update `inventory-enriched.json`: `emFileDialog-196` verdict drifted → faithful, with reason "canonical subscribe + poll-in-Cycle pattern; B-018 design doc verified all three Rust sites mirror C++:90 and C++:196".
3. Update `pattern-catalog.md` P-008 entry: append "retired — singleton false positive, see B-018 design doc".
4. Add reconciliation log entry summarizing the false-positive finding and the latent CheckFinish.else gap as a separate audit follow-up.
5. Append to `decisions.md` only if reconciler determines a global "what counts as canonical subscribe+poll-in-Cycle" entry is worth recording — the design doc itself doesn't require one.
