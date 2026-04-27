# B-012-rc-shim-mainctrl — P-004 — convert rc-shim consumers in emMainControlPanel

**Pattern:** P-004-rc-shim-instead-of-signal
**Scope:** emmain:emMainControlPanel
**Row count:** 7
**Mechanical-vs-judgement:** judgement-heavy
**Cited decisions:** D-002-rc-shim-policy (rule 1 convert across all 7 rows; no rule-2 candidates), D-006-subscribe-shape (canonical first-Cycle init, shared block with B-006), D-007-mutator-fire-shape (`ReloadFiles` restructure for row 224).
**Prereq buckets:** B-019-stale-annotations (hard, all 7 rows — camouflage removal), B-006-typed-subscribe-mainctrl (soft, all 7 rows — shared first-Cycle init block; second-to-land merges connect calls into the first's block).

**Reconciliation amendments (2026-04-27, post-design bf6e9bd5):**
- **All 7 rows are uniform rule-1 convert.** Audit's accessor-status verdict held — `emButton.click_signal` exists at `emButton.rs:40`. No reclassifications.
- **Two-hop relay design (row 224, mw.to_reload):** unwound via `mw.ReloadFiles(&self, ectx)` synchronous fire of `file_update_signal`. Deletes `mw.to_reload` field and `MainWindowEngine::Cycle` polling block (rs:382-390). F5 hotkey at rs:269 inlines to direct `app.scheduler.fire(app.file_update_signal)` (input-path bifurcation: ectx unavailable in input handler; resolved by inlining the 1-line branch rather than carrying a parallel `ReloadFilesFromInput(app)` shim). `MainWindowEngine` survives — still handles close/title/startup_done/to_close.
- **Hard prereq edges encoded** in `inventory-enriched.json`: all 7 rows depend on `cleanup-emMainControlPanel-35` (ClickFlags shim removal); rows 221 and 224 also depend on their specific cleanup items.
- **Residual drift note (out of scope):** rows 221 (fullscreen) and 226 (quit) keep stubbed log-only reaction bodies — App access from Cycle is a separate axis not captured by B-012's row scope. Subscription drift fixed; reaction-body drift remains for follow-up audit pass.
- **Watch-list note (now formalised):** `mw.to_reload` was the **2nd sighting** of the polling-intermediary pattern; **promoted to D-009-polling-intermediary-replacement** by B-010 brainstorm `09f08710` after the 3-sighting threshold was reached. The `mw.ReloadFiles(&self, ectx)` restructure is the canonical D-009 fix recipe for this row.

**Inbound notes from prior reconciliations:**
- B-019 (`e7129430`) maps three cleanup items here: `cleanup-emMainControlPanel-35` (ClickFlags shim), `cleanup-emMainControlPanel-303` (row `emMainControlPanel-221`, rs:301), `cleanup-emMainControlPanel-320` (row `emMainControlPanel-224`, rs:319). B-019 lands first to remove camouflage; this bucket's design should not preserve any framing from those removed annotations.
- **Two-hop relay flagged for design:** the `cleanup-emMainControlPanel-320` site involves a `mw.to_reload` chain through `emMainWindow` → `MainWindowEngine` → `file_update_signal`. B-012's design needs to address this second hop, not just the immediate click-handler shim.

## Pattern description

Accessor is present but the Rust consumer routes around the signal by sharing `Rc<RefCell<>>` / `Rc<Cell<>>` state into click-handler closures, hiding the signal from any other observer. This observably changes timing (closures fire vs signals fire), so it is a drift, not a below-surface adaptation. In this bucket the 7 sites are all `widget-click` handlers in `emMainControlPanel` — each must be triaged against the C++ original to decide convert (rule 1) vs keep (rule 2).

## Rows

| ID | C++ site | Rust site | Accessor status | Notes |
|---|---|---|---|---|
| emMainControlPanel-220 | src/emMain/emMainControlPanel.cpp:220 | crates/emmain/src/emMainControlPanel.rs:296 | present | widget-click rc_cell_shim |
| emMainControlPanel-221 | src/emMain/emMainControlPanel.cpp:221 | crates/emmain/src/emMainControlPanel.rs:301 | present | widget-click rc_cell_shim |
| emMainControlPanel-222 | src/emMain/emMainControlPanel.cpp:222 | crates/emmain/src/emMainControlPanel.rs:311 | present | widget-click rc_cell_shim |
| emMainControlPanel-223 | src/emMain/emMainControlPanel.cpp:223 | crates/emmain/src/emMainControlPanel.rs:315 | present | widget-click rc_cell_shim |
| emMainControlPanel-224 | src/emMain/emMainControlPanel.cpp:224 | crates/emmain/src/emMainControlPanel.rs:319 | present | widget-click rc_cell_shim |
| emMainControlPanel-225 | src/emMain/emMainControlPanel.cpp:225 | crates/emmain/src/emMainControlPanel.rs:328 | present | widget-click rc_cell_shim |
| emMainControlPanel-226 | src/emMain/emMainControlPanel.cpp:226 | crates/emmain/src/emMainControlPanel.rs:334 | present | widget-click rc_cell_shim |

## C++ reference sites

- src/emMain/emMainControlPanel.cpp:220
- src/emMain/emMainControlPanel.cpp:221
- src/emMain/emMainControlPanel.cpp:222
- src/emMain/emMainControlPanel.cpp:223
- src/emMain/emMainControlPanel.cpp:224
- src/emMain/emMainControlPanel.cpp:225
- src/emMain/emMainControlPanel.cpp:226

## Open questions for the bucket-design brainstorm

- For each of the 7 rows, does the C++ original use a signal accessor + subscribe at the consumer site (rule 1, convert) or a member field assigned post-finish/post-cycle (rule 2, keep)? Confirm row-by-row before bucketing into convert-set vs keep-set.
- Are any rows ambiguous enough to require escalation to the working-memory session per D-002's rule 3?
- Do the 7 click handlers share a common observer (e.g., a sibling control panel or the main view) such that conversion can reuse a single signal subscription, or does each need its own?
