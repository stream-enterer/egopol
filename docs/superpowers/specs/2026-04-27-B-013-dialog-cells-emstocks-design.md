# B-013-dialog-cells-emstocks — Design

**Bucket:** B-013-dialog-cells-emstocks
**Pattern (post-correction):** P-004-rc-shim-accessor-present (was P-005-rc-shim-no-accessor; corrected by this brainstorm — see Audit-data corrections)
**Scope:** `emstocks` (4 rows, all in `crates/emstocks/src/emStocksListBox.rs`)
**Cited decisions:** D-002-rc-shim-policy (rule 1, convert — trigger side), D-006-subscribe-shape (per-dialog first-Cycle init), D-004-stocks-application-strategy (mechanical application across all in-bucket rows)
**Prereq buckets:** none
**New global decisions:** none

---

## 1. Audit-data corrections

The bucket sketch's framing was wrong. The C++ source uses signal-subscribe + sync-result-read (`AddWakeUpSignal(Dialog->GetFinishSignal())` + `IsSignaled(...) + Dialog->GetResult()`), not post-finish member-field assignment. The Rust port's `finish_signal: SignalId` accessor on `emDialog` (`crates/emcore/src/emDialog.rs:55`) is already present, so accessor-status is not missing.

Per-row triage:

| ID | C++ pattern (verified) | Rust today | D-002 rule | Disposition |
|---|---|---|---|---|
| emStocksListBox-189 | `AddWakeUpSignal(CutStocksDialog->GetFinishSignal())` (cpp:189) + Cycle: `IsSignaled(...) + GetResult()` (cpp:656-657) | `set_on_finish` writes `cut_stocks_result: Rc<Cell<Option<DialogResult>>>`; Cycle polls `cell.take()` as trigger | **Rule 1 — convert (trigger side)** | Subscribe to `dialog.finish_signal`; replace `cell.take()` polling with `IsSignaled(dialog.finish_signal)` as the trigger; cell stays as delivery buffer |
| emStocksListBox-287 | Same shape, Paste (cpp:287, 662-663) | Same shape, `paste_stocks_result` | **Rule 1 — convert (trigger side)** | Same |
| emStocksListBox-356 | Same shape, Delete (cpp:356, 668-669) | Same shape, `delete_stocks_result` | **Rule 1 — convert (trigger side)** | Same |
| emStocksListBox-443 | Same shape, Interest (cpp:443, 674-676) | Same shape, `interest_result` | **Rule 1 — convert (trigger side)** | Same |

Audit-data updates (working-memory session):
- All 4 rows: `pattern_id` P-005 → P-004; `accessor_status` missing → present.
- D-002 "Affects" line: P-004 +4, P-005 -4.
- B-013 reconciliation log entry: third audit-data accessor-status correction (cf. B-006/B-007/B-008 gap-blocked fixes). The audit's automated heuristic missed the inherited/composed `finish_signal` accessor on `emDialog`. Pattern is established but not promoted to a decision — known audit-data-quality issue, not a design choice.

## 2. Pattern split: trigger vs delivery

The drift in B-013 separates into two halves:

**Trigger half (rule-1, converted by this design).** C++ uses `IsSignaled(GetFinishSignal())` to wake the consumer when the dialog finishes. Rust today uses `cell.take().is_some()` polling — observable drift. Fix: subscribe to `dialog.finish_signal` and use `IsSignaled` as the trigger.

**Delivery half (idiom adaptation, no annotation).** C++ reads the result via the synchronous `Dialog->GetResult()` member-field accessor. Rust's `emDialog` does not expose a synchronous post-show result reader (the post-show state lives in App's dialog registry; only `App::mutate_dialog_by_id`-style deferred mutation is currently exposed). The Rust port delivers the result via the `set_on_finish` callback writing into a `pub(crate) Rc<Cell<Option<DialogResult>>>` field on `emStocksListBox`. The cell is below the observable surface — its role is to bridge `DialogPrivateEngine::Cycle`'s scope (where `DlgPanel.finalized_result` is in scope) to the consumer panel's scope. The observable timing is identical to C++:

| Step | C++ | Rust (this design) |
|---|---|---|
| Dialog finalizes result | `DlgPanel.FinishState=2; Result=...` | `dlg.finalized_result = Some(...)` |
| Result observable | `Dialog->GetResult()` | (cell filled by on_finish in same Cycle body) |
| Subscriber notified | `Signal(FinishSignal)` → `IsSignaled(...)` true on next slice | `sched.fire(finish_signal)` → consumer's `IsSignaled(dialog.finish_signal)` true on next slice |
| Subscriber reads result | `Dialog->GetResult()` | `cell.take()` |

`on_finish` runs in the same `DialogPrivateEngine::Cycle` body that calls `fire(finish_signal)` (cf. `emDialog.rs:976-986` and the comment chain at line 456: "fire(finish_signal) → invoke on_finish sequence"). The cell is therefore guaranteed to be populated by the time any subscriber observes `IsSignaled` true on the following slice.

Per Port Ideology, the cell is idiom adaptation forced by emDialog's callback-only post-show API — but the absence of `emDialog::GetResult()` is itself a project-internal architectural choice (App owns dialogs in a registry; handle-side state goes pending-only). Idiom adaptation forced by a project-internal ownership choice is NOT a valid forced-category framing for `DIVERGED:`. The cell field is annotated only by a one-line informational `//` comment explaining its role; no `DIVERGED:` block.

## 3. Implementation

### 3.1 `emStocksListBox` field additions

Add 4 single-byte flags for per-dialog first-Cycle-init:

```rust
pub(crate) cut_subscribed: bool,
pub(crate) paste_subscribed: bool,
pub(crate) delete_subscribed: bool,
pub(crate) interest_subscribed: bool,
```

Initialize to `false` in `new()`.

The 4 existing `Rc<Cell<Option<DialogResult>>>` fields (`cut_stocks_result`, `paste_stocks_result`, `delete_stocks_result`, `interest_result`) **stay**. Add a single one-line comment above the cluster (lines 53-57) noting their role as delivery buffers from `set_on_finish` callbacks.

### 3.2 Mutator-creation sites — minimal changes

**Signatures unchanged.** `CutStocks`, `PasteStocks`, `DeleteStocks`, `SetInterest` keep their `<C: ConstructCtx>` generic signature. No connect call here — the connect moves to `Cycle` per D-006 first-Cycle-init.

**Per mutator, in the cancel-old-dialog branch:** add `self.<dlg>_subscribed = false;` next to the existing `self.<dlg>_result.set(None);`. (The new dialog will re-subscribe on its first Cycle observation; the old dialog's subscription is dropped when the dialog is closed via `pending_actions` and its `finish_signal` no longer fires.)

**Per mutator, after `self.<dlg>_dialog = Some(dialog);`:** add `self.<dlg>_subscribed = false;` (defensive — covers the no-prior-dialog case where the cancel-old branch did not run).

The `set_on_finish` closure that writes the cell **stays unchanged**. The `dialog.show(cc)` call stays.

### 3.3 `Cycle` — signature and per-dialog block

**Signature change:** `pub fn Cycle<C: emcore::emEngineCtx::ConstructCtx>(&mut self, cc: &mut C, ...)` → `pub fn Cycle(&mut self, ectx: &mut emcore::emEngineCtx::EngineCtx<'_>, ...)`. Required because `IsSignaled` and `connect`/`disconnect` are not on `ConstructCtx`. The single production caller (`emStocksFilePanel.rs:380`) already passes `ectx: &mut EngineCtx<'_>` — no caller change needed. Tests at `emStocksListBox.rs:1171,1185,1201,1318` only invoke the four mutators (with `ask=false`, no Cycle invocation under dialog state); they remain unchanged.

**Per-dialog block** (Cut shown; Paste/Delete/Interest are mechanical clones):

```rust
// Poll cut dialog.
if let Some(dialog) = self.cut_stocks_dialog.as_ref() {
    if !self.cut_subscribed {
        ectx.connect(dialog.finish_signal, ectx.id());
        self.cut_subscribed = true;
    }
    if ectx.IsSignaled(dialog.finish_signal) {
        let confirmed =
            self.cut_stocks_result.take() == Some(DialogResult::Ok);
        ectx.disconnect(dialog.finish_signal, ectx.id());
        self.cut_stocks_dialog = None;
        self.cut_subscribed = false;
        if confirmed {
            self.CutStocks(ectx, rec, false); // ectx is ConstructCtx; OK
        }
    } else {
        busy = true;
    }
}
```

Notes:
- `ectx.connect(sig, ectx.id())` mirrors C++ `AddWakeUpSignal(GetFinishSignal())`. The connect is deferred from the C++ "at dialog creation" site to the parent's first Cycle observation of `Some(dialog)` per D-006 wiring shape (justified by `DialogPrivateEngine::Cycle` taking ≥2 of its own cycles to reach `fire(finish_signal)` — cf. `emDialog.rs:850-860` — guaranteeing the parent gets at least one Cycle to land the connect before any fire could be observed).
- `ectx.IsSignaled(dialog.finish_signal)` mirrors C++ `IsSignaled(Dialog->GetFinishSignal())`.
- `self.cut_stocks_result.take()` reads the result delivered by the `on_finish` callback (which ran inside `DialogPrivateEngine::Cycle` at the same time that `fire(finish_signal)` was issued). The synchronous read at this point is sound by the dialog-cycle ordering described in §2.
- `ectx.disconnect(...)` cleans up the subscription before the dialog handle drops. Hygiene; the `SignalId` belongs to a dialog being torn down.
- `self.cut_subscribed = false;` resets the per-dialog flag so the next dialog (if any) starts fresh.
- The recursive `self.CutStocks(ectx, rec, false)` works because `EngineCtx` implements `ConstructCtx` (`emEngineCtx.rs:280`); no signature change to `CutStocks`.

Apply identically to Paste (`paste_stocks_*`), Delete (`delete_stocks_*`), and Interest (`interest_*`) blocks. The Interest block additionally reads `self.interest_to_set.take()` inside the `confirmed` branch as today.

The `else if self.<dlg>_dialog.is_some() { busy = true; }` branches in today's code collapse into the `else { busy = true; }` of the new shape.

### 3.4 No emcore changes

`emDialog` is not modified. No `result()` accessor added. No App-registry inspection path added. The trigger drift is closed; the delivery-buffer pattern is preserved as idiom adaptation.

## 4. Cited decisions (with rule applied per row)

- **D-002 rule 1** (convert): all 4 rows. Trigger side only — see §2 for the trigger/delivery split.
- **D-006**: per-dialog first-Cycle init for the `connect` call, adapted to lazily-created subscribables (4 per-dialog `subscribed: bool` flags rather than one panel-wide flag).
- **D-004**: mechanical application of the design across all 4 in-bucket rows.

D-001 not cited — `emDialog.finish_signal` is already `SignalId`; no flip needed.
D-007 not cited — no model accessor flips; mutators are synchronous already and `emDialog`'s own `fire(finish_signal)` is implemented.
D-008 not cited — no new model `SignalId` is allocated; the dialog's signal is allocated inside `emDialog::new` via the existing path.

## 5. Watch-list (not a decision)

**Candidate framework lift: synchronous post-show `emDialog::GetResult()`.** emDialog's port lacks a synchronous post-show result reader equivalent to C++ `Dialog->GetResult()`. The result is delivered exclusively via the `set_on_finish` callback, which forces every dialog consumer (this bucket plus emfileman, emmain, emFileDialog, etc.) to provide some delivery channel — typically an `Rc<Cell<...>>` or equivalent shared state — to bridge the dialog-engine scope to the consumer scope. Closing this gap would add `App::inspect_dialog_by_id<R>(id, |dlg| ...) -> R` (mirror of the existing `App::mutate_dialog_by_id`) plus a public `emDialog::result(&self) -> Option<DialogResult>` reader that consults App for post-show dialogs.

If this lift lands as a future bucket, B-013's residual cells become drop candidates: trigger-side already converted; delivery-side could be replaced with the new sync reader. Same shape as D-008's A3 watch-list note — promote when enough consumers accumulate the shim, not now.

## 6. Implementer's checklist

1. `crates/emstocks/src/emStocksListBox.rs`:
   - Add 4 `pub(crate) <dlg>_subscribed: bool` fields to the struct.
   - Initialize them to `false` in `new()`.
   - Add a one-line `//` comment above the existing `*_result` cell cluster noting their role as delivery buffers from `on_finish`.
   - In each of the 4 ask=true mutator branches: set `self.<dlg>_subscribed = false;` in the cancel-old-dialog block (next to `result.set(None)`) and after `self.<dlg>_dialog = Some(dialog);`.
   - Change `Cycle` signature to `(&mut self, ectx: &mut emcore::emEngineCtx::EngineCtx<'_>, rec: &mut emStocksRec, config: &emStocksConfig) -> bool`.
   - Replace each of the 4 per-dialog blocks in `Cycle` with the connect+IsSignaled+disconnect shape from §3.3.
   - Confirm recursive `self.<Mutator>(ectx, ...)` calls still type-check (EngineCtx: ConstructCtx).

2. `crates/emstocks/src/emStocksFilePanel.rs:380` already passes `ectx`. No change.

3. Tests at `emStocksListBox.rs:1171,1185,1201,1318` use `ask=false` and never invoke `Cycle` under dialog state. No test changes required.

4. Verify with `cargo check` then `cargo clippy -- -D warnings` then `cargo-nextest ntr`.

5. Pre-commit hook expected to pass cleanly.

## 7. Reconciliation summary (for working-memory session)

Working-memory session reconciliation tasks on design return:
- `inventory-enriched.json`: rows 189/287/356/443 → `pattern_id` P-004, `accessor_status` present.
- `decisions.md` D-002 "Affects" line: P-004 +4, P-005 -4.
- B-013 bucket sketch: pattern P-005 → P-004; reframe description as rule-1 convert (trigger side) with cell-as-delivery-buffer note (idiom adaptation, not divergence); cite D-006 alongside D-002 and D-004.
- B-013 reconciliation log: third audit-data accessor-status correction (cf. B-006/B-007/B-008). Heuristic gap noted; not promoted.
- Watch-list note in `decisions.md` D-008 neighborhood (or new watch-list section): "emDialog post-show synchronous `GetResult()` candidate framework lift" — first sighting B-013, affects all dialog consumers.

No prereq edges introduced. No new D-### entries. B-013 status: pending → designed.
