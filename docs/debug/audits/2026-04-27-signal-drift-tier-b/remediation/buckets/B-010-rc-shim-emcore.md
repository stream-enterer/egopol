# B-010-rc-shim-emcore — P-004 — convert rc-shim consumers in emCoreConfigPanel to signal subscribe

**Pattern:** P-004-rc-shim-instead-of-signal
**Scope:** emcore
**Row count:** 15
**Mechanical-vs-judgement:** judgement-heavy
**Cited decisions:** D-002-rc-shim-policy (rule 1 convert across all 15 rows; no rule-2 candidates), D-006-subscribe-shape (canonical first-Cycle init wiring), D-009-polling-intermediary-replacement (origin: B-010 surfaced sightings 3 and 4 of the polling-intermediary pattern, triggering D-009's promotion).
**Prereq buckets:** none.

**Reconciliation amendments (2026-04-27, post-design 09f08710):**
- **All 15 rows uniform rule-1 convert.** Verified directly against C++: every site is widget-signal subscribe + `IsSignaled` in host `Cycle`, not post-finish/post-cycle member-field assignment. No rule-2 candidates; no reclassifications.
- **D-009-polling-intermediary-replacement promoted** by this brainstorm. Sighting 3 (`FsbEvents`, in scope) addressed by direct widget-state read in `IsSignaled` branches per D-006. Sighting 4 (`generation: Rc<Cell<u64>>` on emCoreConfigPanel, out of B-010 row scope) tracked for downstream resolution; row 80's reset reaction body keeps `generation.set(generation.get() + 1)` verbatim in this bucket.
- **FsbEvents disposition:** dropped per rule-1 convert. Each widget signal subscribed in FSB's first-Cycle init; Cycle reads widget state via `with_behavior_as::<T>(panel_id, |p| ...)` typed downcast (precedent: `emPanelTree.rs:1714`).
- **Open questions resolved:**
  - §2 (events-aggregator topology): no aggregator in C++; `emFileSelectionBox::Cycle` (cpp:385) is a flat list of `IsSignaled` branches in source order. Rule-1 convert applies.
  - §3 (generation-counter topology): C++ uses `emRecListener::OnRecChanged()` per group, no shared signal nor per-control generation counter. Rust generation counter is Rust-only divergence — D-009 sighting #4. Out of B-010 scope.
  - §4 (AutoplayFlags shape cross-check): not relevant to any B-010 row.
- **Bucket sketch row note "no Cycle override exists" disambiguation:** refers to Rust today (correct: zero `fn Cycle` impls in `emCoreConfigPanel.rs`). C++ has Cycle overrides on `emCoreConfigPanel`, `MouseMiscGroup`, `MaxMemGroup`, `PerformanceGroup`. Future audit re-runs should distinguish the two.
- **No new D-### entries beyond D-009.** No prereq edges. No row reassignments. No accessor-status revisions.

## Pattern description

Accessor is present on the upstream model/widget but the consumer routes around the signal by mutating `Rc<RefCell<>>` / `Rc<Cell<>>` shared state inside click/check/value closures, hiding the signal from any other observer. The shim observably changes timing (closures fire vs signals fire), so it is not a below-surface adaptation. In this bucket the shim takes two emcore shapes: emCoreConfigPanel widgets installing per-control `on_click`/`on_check`/`on_value` closures that mutate a config `Rc<RefCell<>>` plus a generation counter, and emFileSelectionBox using a `RefCell<Events>` aggregator drained by `Cycle`.

## Rows

| ID | C++ site | Rust site | Accessor status | Notes |
|---|---|---|---|---|
| emCoreConfigPanel-80 | src/emCore/emCoreConfigPanel.cpp:80 | crates/emcore/src/emCoreConfigPanel.rs:1539 | present | Reset button. on_click closure substitutes for AddWakeUpSignal+IsSignaled; mutates Rc<RefCell<config>>+Rc<Cell<u32>> gen. |
| emCoreConfigPanel-299 | src/emCore/emCoreConfigPanel.cpp:299 | crates/emcore/src/emCoreConfigPanel.rs:341 | present | StickBox. on_check closure substitutes for AddWakeUpSignal+IsSignaled; no Cycle override exists. |
| emCoreConfigPanel-300 | src/emCore/emCoreConfigPanel.cpp:300 | crates/emcore/src/emCoreConfigPanel.rs:363 | present | EmuBox. Same on_check closure pattern. |
| emCoreConfigPanel-301 | src/emCore/emCoreConfigPanel.cpp:301 | crates/emcore/src/emCoreConfigPanel.rs:378 | present | PanBox. Same on_check closure pattern. |
| emCoreConfigPanel-563 | src/emCore/emCoreConfigPanel.cpp:563 | crates/emcore/src/emCoreConfigPanel.rs:801 | present | MemField scalar (max megabytes per view). on_value closure pattern. |
| emCoreConfigPanel-746 | src/emCore/emCoreConfigPanel.cpp:746 | crates/emcore/src/emCoreConfigPanel.rs:1039 | present | MaxRenderThreads. on_value closure pattern. |
| emCoreConfigPanel-755 | src/emCore/emCoreConfigPanel.cpp:755 | crates/emcore/src/emCoreConfigPanel.rs:1066 | present | AllowSIMD. on_check closure pattern. |
| emCoreConfigPanel-773 | src/emCore/emCoreConfigPanel.cpp:773 | crates/emcore/src/emCoreConfigPanel.rs:1205 | present | DownscaleQuality. on_value closure pattern. |
| emCoreConfigPanel-791 | src/emCore/emCoreConfigPanel.cpp:791 | crates/emcore/src/emCoreConfigPanel.rs:1233 | present | UpscaleQuality. on_value closure pattern. |
| emFileSelectionBox-514 | src/emCore/emFileSelectionBox.cpp:514 | crates/emcore/src/emFileSelectionBox.rs:1101 | present | RefCell<Events> aggregator: closures push events; Cycle (1494) drains. Multi-event Rc<Cell<bool>>-style shim. |
| emFileSelectionBox-521 | src/emCore/emFileSelectionBox.cpp:521 | crates/emcore/src/emFileSelectionBox.rs:1120 | present | Same events-aggregator pattern; Cycle reacts at line 1520. |
| emFileSelectionBox-531 | src/emCore/emFileSelectionBox.cpp:531 | crates/emcore/src/emFileSelectionBox.rs:1494 | present | FilesLB selection_changed flag drained by Cycle at 1532; rc_cell_shim style instead of IsSignaled. |
| emFileSelectionBox-532 | src/emCore/emFileSelectionBox.cpp:532 | crates/emcore/src/emFileSelectionBox.rs:1547 | present | ItemTrigger drained by Cycle. Same events-aggregator pattern. |
| emFileSelectionBox-540 | src/emCore/emFileSelectionBox.cpp:540 | crates/emcore/src/emFileSelectionBox.rs:1205 | present | Cycle drains name_text_changed at line 1576. |
| emFileSelectionBox-550 | src/emCore/emFileSelectionBox.cpp:550 | crates/emcore/src/emFileSelectionBox.rs:1606 | present | FiltersLB filter_index_changed drained by Cycle. Same events-aggregator pattern. |

## C++ reference sites

- src/emCore/emCoreConfigPanel.cpp:80
- src/emCore/emCoreConfigPanel.cpp:299
- src/emCore/emCoreConfigPanel.cpp:300
- src/emCore/emCoreConfigPanel.cpp:301
- src/emCore/emCoreConfigPanel.cpp:563
- src/emCore/emCoreConfigPanel.cpp:746
- src/emCore/emCoreConfigPanel.cpp:755
- src/emCore/emCoreConfigPanel.cpp:773
- src/emCore/emCoreConfigPanel.cpp:791
- src/emCore/emFileSelectionBox.cpp:514
- src/emCore/emFileSelectionBox.cpp:521
- src/emCore/emFileSelectionBox.cpp:531
- src/emCore/emFileSelectionBox.cpp:532
- src/emCore/emFileSelectionBox.cpp:540
- src/emCore/emFileSelectionBox.cpp:550

## Open questions for the bucket-design brainstorm

- Per D-002 rule: confirm row-by-row that each cited C++ site uses a signal accessor + consumer subscribe (rule 1: convert) vs a post-finish/post-cycle member field (rule 2: keep). The notes assert "no Cycle override exists" for several emCoreConfigPanel rows — verify against C++ before defaulting to convert.
- emFileSelectionBox uses a `RefCell<Events>` aggregator drained by a single `Cycle`. Does converting each event to its own signal-subscribe preserve the C++ Cycle ordering (events drained in one pass, in insertion order), or does multi-signal subscribe reorder observable handler firing? May need an aggregator-preserving subscribe shape.
- For the emCoreConfigPanel `Rc<Cell<u32>>` generation counter that tracks any-config-mutated: does the C++ original have a single config-changed signal that all widgets fan into, or per-control signals? The fix shape (one signal vs many) depends on the C++ topology.
- D-002 flags the emAutoplay `AutoplayFlags { progress: Rc<Cell<f64>> }` adaptation question for the working-memory session. Out of scope for this emcore bucket but worth noting if any emcore row turns out to share that shape (none flagged in packet).

