# B-018-fileDialog-singleton — P-008 (RETIRED) — emFileDialog false-positive

**Pattern:** P-008-connect-with-poll-fallback (**RETIRED 2026-04-27** — category error in audit classification scheme; see `pattern-catalog.md`)
**Scope:** emcore
**Row count:** 1 (verified observable equivalence to C++; reclassified to `faithful`)
**Mechanical-vs-judgement:** judgement-heavy → no work needed
**Cited decisions:** none.
**Prereq buckets:** none.

**Reconciliation amendments (2026-04-27, post-design 04059bac):**
- **B-018 closes as a false positive.** Verified diagnosis: `AddWakeUpSignal + IsSignaled-in-Cycle` is the canonical emEngine subscription pattern (subscription arming + wakeup-cause check inside Cycle), not "hybrid drift." Rust rs:169/516/733 mirror C++ cpp:90/196 exactly; rs:516/733 split is idiom adaptation for Rust's outer-engine lifecycle (mutually exclusive per dialog spawn).
- **emFileDialog-196 reclassified `drifted → faithful`** in `inventory-enriched.json`.
- **P-008 pattern retired** in `pattern-catalog.md` with retirement-reason note. Audit trail preserved.
- **No code changes. No prereq edges. No new D-### entries.** B-018 closes as designed and immediately mergeable.
- **Latent gap noted (out of B-018 scope):** `CheckFinish` post-show else branch at `rs:532-543` parks OD via `pending_actions` but does not call `scheduler.connect(od.finish_signal, outer_engine_id)`. All current callers (rs:806/815/1129/1228/1332) are `#[cfg(test)]`; production post-show goes through `run_file_dialog_check_finish`. If a future non-test caller invokes `CheckFinish` post-show, this becomes drift. Recommend separate audit follow-up.

## Pattern description (historical, preserved for audit trail)

P-008 covered sites where a `scheduler.connect(...)` call coexists with a nearby `IsSignaled(...)` poll on the same signal — a hybrid wiring shape the audit categorized as "neither pure event-driven nor pure polling." The categorization was a misread: `IsSignaled` in emCore is a wakeup-cause probe (engine-state check), not an independent state poll, so connect + IsSignaled-in-Cycle are complementary, not redundant. The singleton instance was `emFileDialog`'s `od_finish_sig` / `od_sig` wiring across rs:169 / 516 / 733 — all three sites verified canonical.

## Rows

| ID | C++ site | Rust site | Accessor status | Notes |
|---|---|---|---|---|
| emFileDialog-196 | src/emCore/emFileDialog.cpp:196 | crates/emcore/src/emFileDialog.rs:516 | present | IsSignaled at rs:169; fallback `scheduler.connect(od_finish_sig, outer_engine_id)` at rs:733 |

## C++ reference sites

- src/emCore/emFileDialog.cpp:196

## Open questions for the bucket-design brainstorm

- Is the IsSignaled poll at `emFileDialog.rs:169` the actual drift, or is the connect_call at `:516` misplaced relative to the C++ shape at `emFileDialog.cpp:196`?
- Does the post-show `scheduler.connect(od_finish_sig, outer_engine_id)` at `:733` make either earlier wiring site redundant, or are all three required by C++ semantics?
- What does C++ `emFileDialog.cpp:196` actually do — connect, poll, or both — and which Rust site is the faithful mirror?
- Should remediation remove the poll, remove one of the connects, or restructure so the three sites collapse into a single wiring point?
- Does this singleton justify a P-008-wide ADR, or is `emFileDialog` idiosyncratic enough that the decision stays local to this bucket?
- Are there latent test signals (golden, behavioral) that would distinguish "poll fires first" vs "connect fires first" ordering, and do we need to author one before remediating?
