# B-013-dialog-cells-emstocks — P-004 — emstocks dialog-result trigger conversion (rule-1 half-convert)

**Pattern:** P-004-rc-shim-instead-of-signal (reclassified from P-005 by B-013 brainstorm — `emDialog.finish_signal` is present; audit accessor-status heuristic missed it)
**Scope:** emstocks
**Row count:** 4
**Mechanical-vs-judgement:** judgement-heavy — was prejudged as D-002 rule-2 keep-shim; brainstorm verified all 4 rows are rule-1 convert (trigger side). Implementation is mechanical once design is settled.
**Cited decisions:** D-002-rc-shim-policy (rule 1, trigger side — convert from `cell.take()` polling to `IsSignaled` subscribe), D-006-subscribe-shape (canonical wiring, applied per-dialog with first-Cycle init), D-004-stocks-application-strategy (mechanical application across 4 rows).
**Prereq buckets:** none.

**Reconciliation amendments (2026-04-27, post-design ec317565):**
- **Audit-data correction (third accessor-status heuristic gap, after B-006/B-007/B-008):** all 4 rows reclassified `pattern_id P-005 → P-004` and `accessor_status missing → present`. `emDialog.finish_signal` is a public field on `emDialog`; the audit's automated accessor-status heuristic missed it. Captured in each row's `reconciliation` field.
- **Rule applied: D-002 rule 1 (convert), trigger side only.** All 4 C++ sites use `AddWakeUpSignal(GetFinishSignal()) + IsSignaled + GetResult` — canonical signal-subscribe shape, not post-finish member-field assignment. Bucket sketch's original "rule-2 keep-shim" framing was prejudged wrong.
- **Half-convert design:** subscribe via per-dialog first-Cycle init (D-006); `IsSignaled(dialog.finish_signal)` replaces `cell.take().is_some()` as the trigger; the `Rc<Cell<Option<DialogResult>>>` and `set_on_finish` callback **stay** as the result-delivery buffer (idiom adaptation, NOT DIVERGED — observable behavior matches C++; cell is `pub(crate)` internal state below the user-visible surface).
- **Watch-list (not a decision):** emDialog's lack of sync post-show `GetResult` is an architectural gap that affects every dialog consumer in the codebase (emfileman, emmain, emFileDialog). A future bucket may close it via `App::inspect_dialog_by_id` + `emDialog::GetResult`; B-013 explicitly does not. Same shape as D-008's A3 watch-list note.
- **D-002 affects count amended:** P-004 +4 (29→33), P-005 −4 (6→2).

## Pattern description

Consumer polls `Rc<Cell<Option<DialogResult>>>` (filled by a `set_on_finish` callback) instead of subscribing to the dialog's `finish_signal` accessor that already exists on `emDialog`. C++ uses canonical `AddWakeUpSignal(GetFinishSignal()) + IsSignaled + GetResult` for all 4 sites — rule-1 convert (trigger side). The fix wires per-dialog first-Cycle subscribe to `dialog.finish_signal` and gates result-read on `IsSignaled`; the cell+callback stay as a delivery buffer (idiom adaptation, not divergence — observable behavior matches C++).

## Rows

| ID | C++ site | Rust site | Accessor status | Notes |
|---|---|---|---|---|
| emStocksListBox-189 | src/emStocks/emStocksListBox.cpp:189 | crates/emstocks/src/emStocksListBox.rs:54 | missing | Cut-confirmation dialog finish via Rc<Cell<Option<DialogResult>>> shim; observed in Cycle |
| emStocksListBox-287 | src/emStocks/emStocksListBox.cpp:287 | crates/emstocks/src/emStocksListBox.rs:55 | missing | Paste-confirmation dialog finish: rc_cell_shim. Same pattern as Cut/Delete/Interest |
| emStocksListBox-356 | src/emStocks/emStocksListBox.cpp:356 | crates/emstocks/src/emStocksListBox.rs:56 | missing | Delete-confirmation dialog finish: rc_cell_shim |
| emStocksListBox-443 | src/emStocks/emStocksListBox.cpp:443 | crates/emstocks/src/emStocksListBox.rs:57 | missing | Interest-change confirmation dialog finish: rc_cell_shim |

## C++ reference sites

- src/emStocks/emStocksListBox.cpp:189
- src/emStocks/emStocksListBox.cpp:287
- src/emStocks/emStocksListBox.cpp:356
- src/emStocks/emStocksListBox.cpp:443

## Open questions for the bucket-design brainstorm

- Confirm per D-002 rule 2 that each of the 4 rows' C++ original truly uses a post-finish member-field read (not a signal accessor + subscribe); spot-check the cited cpp:line sites before committing to keep-shim.
- Decide the exact `DIVERGED:` annotation category and citation text for the keep-shim outcome (language-forced vs preserved-design-intent framing) — the rc-shim shape mirrors C++ post-finish member-field semantics, so the annotation must explain why the shim *is* the C++ contract here.
- Whether all 4 dialog-result Cells share a single annotation block or each row carries its own — file is the same (`emStocksListBox.rs`), lines are adjacent (54-57), so a single block citing all four sites is plausible.
- Per D-004 operational consequence: confirm "mechanical application across all in-bucket rows" is the intent (i.e., the 4 rows get the identical keep-shim treatment, no per-row redesign).
- Whether the keep-shim outcome here sets precedent the working-memory session needs to back-propagate to the analogous emAutoplay flags-passing pattern flagged in D-002's open questions.

