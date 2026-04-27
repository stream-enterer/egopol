# B-001-no-wire-emstocks — Design

**Date:** 2026-04-27
**Status:** Approved (brainstorm)
**Bucket:** `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/buckets/B-001-no-wire-emstocks.md`
**Pattern:** P-001-no-subscribe-no-accessor
**Scope:** emstocks, 71 rows
**Mechanical-vs-judgement:** balanced — accessor groups are the judgement axis (5 model accessors + N widget signals); per-panel wiring is mechanical once accessors land.

## Goal and scope

Wire the missing P-001 sites across emstocks: both halves of the wire (model-side accessor + consumer-side subscribe). The bucket contains 71 rows clustered around three categories of accessor:

1. **Model/data accessors** — `emStocksFileModel::GetChangeSignal`, `emStocksConfig::GetChangeSignal`, `emStocksPricesFetcher::GetChangeSignal`, `emStocksListBox::GetSelectedDateSignal`, `emListBox::GetSelectionSignal` (already ported), `emListBox::GetItemTriggerSignal` (already ported).
2. **Widget signal accessors** — every widget kind used here (`emTextField::text_signal`, `emCheckBox::check_signal`, `emScalarField::value_signal`, `emButton::click_signal`, `emRadioButton::check_signal`) is already ported and accessible. The "missing accessor" tag in the audit row is, in these cases, an artifact of "the *widget instance* doesn't exist on the panel, so the signal isn't reachable." The fix is to add the widget field, not to add a new accessor.
3. **Cross-panel reaction wires** — ControlPanel/ItemPanel/ItemChart/FilePanel reacting to ListBox/FileModel/Config signals.

All 71 rows are wired in this bucket using **D-006-subscribe-shape** (first-Cycle init + IsSignaled top-of-Cycle). Three accessor-side gap-blocked rows are filled in scope per **D-003-gap-blocked-fill-vs-stub**. **D-004-stocks-application-strategy** — confirmed: design once, apply mechanically.

## Cited decisions

- **D-006-subscribe-shape** — canonical wiring pattern (subscribed_init flag + ectx.connect in first Cycle + IsSignaled at top).
- **D-003-gap-blocked-fill-vs-stub** — fill the three accessor-side gap rows (FileModel/Config/PricesFetcher) in this bucket; both halves live in emstocks scope.
- **D-004-stocks-application-strategy** — operationally a non-decision; this design is the canonical "design once" artifact.

## Audit-data anomalies (corrections)

The following audit rows are stale or mis-tagged. They remain in this bucket but the design records the correction so the working-memory session can patch `inventory-enriched.json`:

1. **emStocksFileModel-accessor-model-change** — tagged "accessor missing." The Rust `emStocksFileModel` composes `emRecFileModel<emStocksRec>` (line 21), which transitively contains `emFileModel::change_signal` with the `GetChangeSignal()` accessor at `emcore/src/emFileModel.rs:64`. The accessor *exists* on the embedded base; what's missing is a *delegating* accessor on `emStocksFileModel`. Fix is a one-line forward, not a new SignalId.

2. **emStocksConfig-accessor-config-change** — tagged "accessor missing." This is genuine: `emStocksConfig` is currently a plain data `struct` (not composed with `emConfigModel`). The C++ `emStocksConfig : public emConfigModel` inherits `GetChangeSignal`. Rust must either (a) compose with `emConfigModel`, or (b) add a `change_signal: SignalId` field directly. See decision below.

3. **emStocksPricesFetcher-accessor-model-change** — tagged "accessor missing." Genuine: the Rust struct has no SignalId field. Add a `change_signal: SignalId` field and `GetChangeSignal()` accessor.

4. **emStocksListBox-53** — `GetItemTriggerSignal` is *inherited*. The Rust `emStocksListBox` holds an `Option<emListBox>` (`list_box: Option<emListBox>`), and `emListBox.item_trigger_signal` is already a public `SignalId` (`emcore/src/emListBox.rs:312`). No new accessor needed — only a consumer subscribe.

5. **emStocksListBox-51 / -52** — tagged "ListBox holds no FileModel/Config ref." Confirmed by reading source: `emStocksListBox` does *not* hold `FileModel`/`Config` references; the parent `emStocksFilePanel` passes `rec` and `config` per-Cycle into `emStocksListBox::Cycle(ectx, rec, config)`. C++ `emStocksListBox` holds these as members and subscribes in its own Cycle. Rust ListBox cannot subscribe directly without holding refs. Two options: (a) add the refs and mirror C++, (b) move the subscription up to `emStocksFilePanel::Cycle` and have *it* react on ListBox's behalf. The design picks (a) below — see §"emStocksListBox" — because the C++ contract is "ListBox reacts to model/config changes by re-sorting visible items"; that reaction logically belongs in ListBox, and moving it changes structure.

6. **Subscriptions for action buttons that don't exist as Rust fields** — `emStocksControlPanel-650/-658/-666/.../-772` (NewStock, CutStocks, CopyStocks, PasteStocks, DeleteStocks, SelectAll, ClearSelection, SetHighInterest, SetMediumInterest, SetLowInterest, ShowFirstWebPages, ShowAllWebPages, FindSelected, FindNext, FindPrevious — 15 click rows) reference C++ `emButton` instances that do *not* currently exist as fields on Rust `ControlWidgets`. The audit's "missing subscribe" is downstream of "missing widget." Adding the subscribe requires first adding the `emButton` field. This is in-scope for the bucket — both halves are within the same panel — but the implementation step is "add widget field + subscribe," not just "subscribe."

These corrections do not move any rows out of B-001.

## Accessor groups

Group rows by the C++ signal they target. For each group: which model exposes the signal, what its Rust state is today, what fix the accessor needs, and which rows depend on it.

### G1 — `emStocksFileModel.GetChangeSignal()` (FileModel→change broadcast)

**C++ source.** Inherited from `emRecFileModel` / `emFileModel`. Fired when `emFileModel::Signal()` runs (i.e., on every `emFileModel::Save`/`Load`/explicit `Signal()` call).

**Rust state today.** Underlying SignalId exists at `emcore/src/emFileModel.rs:117` (`change_signal: SignalId`). Accessor `emFileModel::GetChangeSignal()` returns `SignalId` (line 64). `emStocksFileModel` composes `emRecFileModel<emStocksRec>` but exposes no delegating accessor.

**Fix.** Add a one-line delegating accessor on `emStocksFileModel`:

```rust
/// Port of inherited C++ emFileModel::GetChangeSignal.
pub fn GetChangeSignal(&self) -> SignalId {
    self.file_model.GetChangeSignal()
}
```

**Rows depending on G1 (consumer subscribes):**
- `emStocksControlPanel-74` (outer ControlPanel.Cycle)
- `emStocksControlPanel-1144` (inner CategoryPanel.Cycle subscribes outer FileModel)
- `emStocksItemPanel-831` (inner CategoryPanel.Cycle)
- `emStocksListBox-51` (ListBox.Cycle — but see §"emStocksListBox" — needs FileModel ref added)

### G2 — `emStocksConfig.GetChangeSignal()` (Config→change broadcast)

**C++ source.** Inherited from `emConfigModel`. Fired on `emConfigModel::Signal()` calls (e.g., when config setters mutate state).

**Rust state today.** `emcore::emConfigModel::GetChangeSignal()` returns `SignalId` (line 68). `emStocksConfig` is a plain struct — *does not* compose `emConfigModel`. There is no SignalId on it.

**Fix.** Two choices — design picks **(B)**:

- (A) Compose `emStocksConfig` with `emcore::emConfigModel::emConfigModel`. Mirrors C++ `class emStocksConfig : public emConfigModel`. Larger blast radius — the existing plain struct is read/written across the codebase as a value type with `Default` and `Clone`. Compositional ownership of an `emConfigModel` (which holds a SignalId, scheduler-bound) breaks the value-type usage.
- (B) Add a `change_signal: SignalId` field directly to `emStocksConfig`, plus a `Signal(&mut self, ectx: &mut EngineCtx)` mutator that fires it. Add `GetChangeSignal(&self) -> SignalId` accessor. Skips the full `emConfigModel` port (consistent with current Rust: `emStocksConfig` is config data, not a configmodel singleton). Mark with `DIVERGED:` *only if* the divergence is observable at any golden site; otherwise the accessor's contract is identical and this is below-surface adaptation.

**Why (B).** `emStocksConfig` is currently used as a plain Rust value passed by reference into Cycle methods (e.g., `lb.Cycle(ectx, rec, config)`). The C++ multi-inheritance of `emConfigModel` is itself a Rust language-forced divergence per the existing `emStocksFileModel` precedent (composition vs. MI). Going the full `emConfigModel`-composition path would force a re-architecting of every emstocks call-site that holds `emStocksConfig` by value or `&`. Option (B) is the smallest viable shape — adds the SignalId field plus accessor, keeps the value-type flow.

**Caveat.** Option (B) requires mutator sites (every `config = new_config` write in `ReadFromWidgets` / config-load code) to *also* call `config.Signal(ectx)`. The implementer must enumerate those sites — primarily in `emStocksControlPanel::ReadFromWidgets` and the file-load path — and add the fire. Without this, G2 subscribers will never wake.

**Rows depending on G2:**
- `emStocksControlPanel-75`, `-1014` (CategoryPanel inner Cycle subscribes Config)
- `emStocksItemPanel-74`, `-832`
- `emStocksItemChart-64`
- `emStocksListBox-52`

### G3 — `emStocksPricesFetcher.GetChangeSignal()`

**C++ source.** Owned: `emSignal ChangeSignal;` (`emStocksPricesFetcher.h:103`); accessor at line 66. Fired when fetch progresses or completes.

**Rust state today.** `emStocksPricesFetcher` (struct at `emStocksPricesFetcher.rs:18`) has no SignalId field, no accessor.

**Fix.** Add `change_signal: SignalId` to the struct, allocate in `new(...)` via `ctx.create_signal()` (so `new` must take `&mut C: ConstructCtx` or equivalent — currently `new` takes none, see header). Add `GetChangeSignal()` accessor. Add a private `signal_change(&mut self, ectx: &mut EngineCtx)` and call it from every internal state transition that the C++ original signals (consult `emStocksPricesFetcher.cpp` for `Signal(ChangeSignal)` call sites).

**Rows depending on G3.** None of the 71 rows directly subscribe to G3 in this bucket (the audit's "PricesFetcher accessor missing" row stands alone). Per D-003, fill the accessor anyway — its absence blocks any future consumer (e.g., a fetch-progress UI). The design ports the accessor and leaves the consumer side to a future bucket if needed. Flag for working-memory session: confirm there is no consumer in B-001 that needs G3; if there is, add it here.

### G4 — `emStocksListBox.GetSelectedDateSignal()`

**C++ source.** Owned: `emSignal SelectedDateSignal;` (`emStocksListBox.h:89`); accessor at line 42. Fired when the selected-date cursor changes.

**Rust state today.** `emStocksListBox` has no `selected_date_signal` field; mutating `selected_date: String` is unsignalled. The setter path needs auditing — search for writes to `self.selected_date`.

**Fix.** Add `selected_date_signal: SignalId` to `emStocksListBox`. Allocate in `new` (currently takes no args; either change `new` to `new(cc: &mut C)` or thread a `SignalId` from caller). Add `GetSelectedDateSignal()` accessor. Add a `signal_selected_date(&mut self, ectx: &mut EngineCtx)` helper. Wire it at every `selected_date` mutation site.

**Rows depending on G4 (consumer subscribes):**
- `emStocksControlPanel-77`
- `emStocksFilePanel-255`
- `emStocksItemChart-65`
- `emStocksItemPanel-75`

### G5 — `emListBox::GetSelectionSignal()` (inherited via `emStocksListBox`)

**Rust state today.** `selection_signal: SignalId` exists on `emListBox` (line 310). `emStocksListBox` exposes the inner `Option<emListBox>` as `pub(crate) list_box`.

**Fix.** Add a delegating accessor on `emStocksListBox`:

```rust
pub fn GetSelectionSignal(&self) -> Option<SignalId> {
    self.list_box.as_ref().map(|lb| lb.selection_signal)
}
```

The `Option` wrapper is necessary because the inner emListBox is lazy-attached. Consumer subscribers must early-return if `None`, or the panel must defer subscribe until ListBox is attached (see §"Sequencing" below).

**Rows depending on G5:**
- `emStocksControlPanel-76`, `-1072` (FileSelectionBox-selection inside FileFieldPanel popup — different scope, see anomaly below), `-1143`
- `emStocksItemPanel-922`

### G6 — `emListBox::GetItemTriggerSignal()` (inherited via `emStocksListBox`)

Already accessible at `emListBox.item_trigger_signal`. Add delegating accessor analogous to G5. One row: `emStocksListBox-53`. Consumer is the parent `emStocksFilePanel` (or whatever houses the ListBox); the C++ call `AddWakeUpSignal(GetItemTriggerSignal())` is *self-subscribe* (ListBox subscribes to its own item-trigger, then in Cycle reacts e.g. by activating on Enter). Needs a Cycle on `emStocksListBox` that subscribes to its own signal and reacts; today `emStocksListBox::Cycle` exists but doesn't subscribe.

### G7 — Widget signals on widgets that *do* exist as panel fields

These rows subscribe to already-existing widget SignalIds. No accessor work; pure consumer wiring.

**ControlPanel (existing widgets):**
- `-413` `widgets.api_key.text_signal`
- `-427` `widgets.auto_update_dates.check_signal`
- `-435` `widgets.triggering_opens_web_page.check_signal`
- `-448` `widgets.chart_period.value_signal`
- `-466` per-button `widgets._min_visible_interest_buttons[i].check_signal` (3 buttons)
- `-557` per-button `widgets._sorting_buttons[i].check_signal` (11 buttons)
- `-566` `widgets.owned_shares_first.check_signal` (NB: C++ uses ClickSignal here on a checkbox; verify against C++)
- `-626` `widgets.selected_date` — but Rust has `selected_date: String` not a TextField; needs widget add
- `-756` `widgets.search_text.text_signal`

**ItemPanel (existing widgets):**
- `-342` `name.text_signal`, `-357` `symbol`, `-364` `wkn`, `-371` `isin`, `-395` `comment`
- `-432` `owning_shares.check_signal`
- `-441` `own_shares.text_signal`, `-446` `trade_price`, `-451` `trade_date`
- `-454` `update_trade_date.click_signal`
- `-490` per-button `_interest_buttons[i].check_signal` (3 buttons)
- `-504` `expected_dividend.text_signal`, `-509` `desired_price`, `-518` `inquiry_date`
- `-527` `_update_inquiry_date.click_signal`
- `-408` per-WebPage `web_pages[i].text_signal` (loop over NUM_WEB_PAGES)
- `-415` per-button `_show_web_page[i].click_signal`
- `-421` `_show_all_web_pages.click_signal`
- `-467` `_fetch_share_price.click_signal`
- `-914` inner CategoryPanel TextField — needs widget audit on `CategoryPanel`
- `-922` inner CategoryPanel ListBox-selection — covered by G5 if the inner ListBox is `emStocksListBox`-based, otherwise needs separate accessor (audit reading suggests it's the embedded `emListBox`)

### G8 — Widget signals on widgets that **do not exist as Rust fields today**

These rows look like consumer-only wiring in the audit, but in fact the widget itself is missing. The fix has two halves: (a) add the `emButton`/`emTextField` field to `ControlWidgets`, instantiate it in `ControlWidgets::new`; (b) subscribe to its `click_signal`/`text_signal` in Cycle and react.

**ControlPanel rows:**
- `-586` `FetchSharePrices` (button)
- `-600` `DeleteSharePrices`
- `-609` `GoBackInHistory`
- `-618` `GoForwardInHistory`
- `-626` `SelectedDate` (TextField — currently a `String`)
- `-650` `NewStock`
- `-658` `CutStocks`
- `-666` `CopyStocks`
- `-674` `PasteStocks`
- `-682` `DeleteStocks`
- `-690` `SelectAll`
- `-698` `ClearSelection`
- `-706` `SetHighInterest`
- `-714` `SetMediumInterest`
- `-722` `SetLowInterest`
- `-730` `ShowFirstWebPages`
- `-738` `ShowAllWebPages`
- `-749` `FindSelected`
- `-764` `FindNext`
- `-772` `FindPrevious`

This is 20 widget-add operations. Each is a small mechanical edit (add field, instantiate in `ControlWidgets::new`, subscribe in Cycle, react). The reaction targets a method that already exists on `emStocksListBox` (e.g., `CutStocks`, `PasteStocks`, `SelectAll`) or `emStocksControlPanel` itself.

### G9 — Inner `FileFieldPanel` widget signals

- `emStocksControlPanel-1064` (TextField text-changed inside FileFieldPanel popup)
- `emStocksControlPanel-1072` (FileSelectionBox-selection inside FileFieldPanel popup)

`FileFieldPanel` is a Rust-side helper struct in emstocks (verify location). Whether it exposes the inner widgets must be confirmed by reading. If the inner widgets are private/`_`-prefixed, this group needs widget exposure plus subscribe. Flag for the implementer.

## Per-panel consumer wiring

For every panel below, follow the D-006 shape exactly:

```rust
pub struct PanelXYZ {
    // ... existing fields
    /// First-Cycle init flag for D-006-subscribe-shape.
    subscribed_init: bool,
}

fn Cycle(&mut self, ectx: &mut EngineCtx, pctx: &mut PanelCtx) -> bool {
    if !self.subscribed_init {
        let eid = ectx.id();
        // ectx.connect(...) for every reactive signal — see per-panel list below.
        self.subscribed_init = true;
    }
    // IsSignaled checks at top, in C++ source order.
    // ...
    false // or whatever the existing Cycle returned
}
```

### emStocksControlPanel (37 rows)

Outer panel. Subscribes to G1, G2, G5, G7 (existing widgets), G8 (added widgets). The Cycle body mirrors C++ `emStocksControlPanel.cpp:97–`.

Connect-list (in C++ source order):
1. `self.file_model.borrow().GetChangeSignal()` — G1 row -74
2. `self.config.borrow().GetChangeSignal()` — G2 row -75
3. `self.list_box.GetSelectionSignal().expect(...)` — G5 row -76 (defer to second-Cycle if ListBox not yet attached; see §Sequencing)
4. `self.list_box.GetSelectedDateSignal()` — G4 row -77
5. (in `AutoExpand` path, after widgets exist) all G7/G8 widget signals enumerated above

IsSignaled branches (in C++ order, at top of Cycle):
- `if ectx.IsSignaled(file_model.GetChangeSignal()) { update_controls_needed = true; }`
- `if ectx.IsSignaled(config.GetChangeSignal()) { update_controls_needed = true; }`
- `if ectx.IsSignaled(list_box.selected_date_signal) { update_controls_needed = true; /* update SelectedDate display */ }`
- For each widget signal: corresponding mutator on Config or ListBox (mirror C++ `emStocksControlPanel::Cycle`).

Note the existing `update_controls_needed` flag matches C++'s `UpdateControlsNeeded` — reuse it.

### emStocksItemPanel (25 rows + 1 inner CategoryPanel)

Same shape. Connect-list:
1. `config.GetChangeSignal()` — row -74
2. `list_box.GetSelectedDateSignal()` — row -75
3. After AutoExpand — every widget signal in G7 list (12 rows of TextField/CheckBox/Button/RadioButton)

The inner `CategoryPanel` (rows -831, -832, -914, -922) is a sub-panel with its own Cycle; treat as a separate panel applying the same D-006 shape. Currently the Rust `CategoryPanel` is a plain struct `pub struct CategoryPanel` (line 32) with no Cycle. Either (a) give it its own Cycle and a back-reference to outer FileModel/Config (mirror C++), or (b) move the subscribe up to `emStocksItemPanel::Cycle` and dispatch into the CategoryPanel's reactor. Design picks (a) for fidelity to C++ structure; flag for the implementer if back-reference plumbing is a structural problem.

### emStocksItemChart (2 rows: -64, -65)

`emStocksItemChart::new` currently takes no args and has no Cycle. Add:
- `subscribed_init: bool`
- A `Cycle(&mut self, ectx, ..., config: &emStocksConfig, list_box: &emStocksListBox)` method.

C++ Cycle body (cpp:93–94) is a straight `IsSignaled` OR-check that triggers `UpdateData()`. Mirror exactly.

### emStocksFilePanel (2 rows: -255 and the existing Cycle integration)

Existing `Cycle` (line 349). Add `subscribed_init` field; connect G4 (`list_box.GetSelectedDateSignal()` — row -255) when ListBox is attached. Note the panel already calls `lb.Cycle(...)` — keep that as the engine-driven cycle; the new top-of-Cycle `IsSignaled` branch reacts to the date change (e.g., trigger ItemChart UpdateData).

### emStocksListBox (3 rows: -51, -52, -53)

To subscribe to G1/G2 in its own Cycle, ListBox must hold refs to FileModel and Config. Today the parent passes them per-call. The design adds:
```rust
pub struct emStocksListBox {
    // ... existing
    file_model_ref: Option<Rc<RefCell<emStocksFileModel>>>, // (a) cross-Cycle reference, justified per CLAUDE.md §Ownership
    config_ref: Option<Rc<RefCell<emStocksConfig>>>,
    subscribed_init: bool,
}
```

The parent `emStocksFilePanel` sets these refs at attach time (alongside `attach_list_box`). After attach, `emStocksListBox::Cycle` performs the D-006 init/check pattern itself.

If this Rc<RefCell<>> addition pushes against the project's ownership defaults: the C++ original holds `FileModel` and `Config` as references in `emStocksListBox`; the Rust port currently routes around that by passing per-call. Since the C++ shape is observable (the ListBox subscribes from its own scope), preserving the C++ shape is design-intent per CLAUDE.md §"Port Ideology." Add the Rc<RefCell<>> with a `// (a) cross-Cycle reference per CLAUDE.md §Ownership` justification comment.

For row -53 (self-subscribe to `item_trigger_signal`), connect `self.list_box.as_ref().unwrap().item_trigger_signal` and react.

### emStocksFetchPricesDialog (1 row)

Single row: subscribes to PricesFetcher's G3. Implementer confirms target signal (likely `fetcher.GetChangeSignal()`) and reaction.

### Other tail rows

- `emStocksConfig-accessor` — accessor add only (G2 fix).
- `emStocksFileModel-accessor` — accessor add only (G1 fix).
- `emStocksPricesFetcher-accessor` — accessor add only (G3 fix).

## Sequencing

**Within the bucket:**

1. **Land accessor adds first** (G1, G2, G3, G4, G5/G6 delegating accessors). These are leaf changes — no consumers yet, no cycle changes. Safe to land independently. Tests: a unit-level "accessor returns same SignalId across calls" sanity check per accessor.
2. **Land widget adds (G8) inside ControlWidgets/ItemWidgets** — instantiate the missing buttons/textfields, no Cycle wiring yet. Pre-condition for the consumer wiring stage.
3. **Land per-panel Cycle wiring** — one PR per panel (ControlPanel, ItemPanel, ItemChart, FilePanel, ListBox, FetchPricesDialog). Each PR is the D-006 init block plus the IsSignaled reactions for that panel. Rows in the same panel can't be split without leaving an inconsistent intermediate state.
4. **Inner CategoryPanel wiring** lands in the same PR as its outer panel.

**Lazy-attached widgets / ListBox.** ControlPanel and ItemPanel use lazy AutoExpand: widgets are `None` until first expand. The first-Cycle init can't connect a `None` widget. Use one of two shapes:
- (Preferred) Move widget-signal connects into a separate `subscribed_widgets: bool` flag; reset to `false` on AutoShrink, run on the first Cycle after AutoExpand. Two-tier init: model-level signals on first Cycle (always), widget-level signals on first Cycle-after-AutoExpand.
- (Alternative) Always force AutoExpand at panel construction (eager). Larger memory footprint; unlikely acceptable.

The ListBox attach in `emStocksFilePanel` follows the same pattern: a `list_box_subscribed: bool` separate from `subscribed_init` allows attach-deferred subscribe.

**Cross-bucket prereqs.** None. P-001 in emstocks does not consume any P-003 type-mismatch accessor; G1–G6 are all `SignalId`-typed (or trivially adaptable). The bucket can land without waiting on any other bucket.

## Verification strategy

**Behavioral tests** (per D-006 / B-005 precedent), one new file per panel:
- `crates/emstocks/tests/typed_subscribe_b001.rs` — fires each subscribed signal, runs Cycle, asserts the documented reaction (config setter ran, ListBox sort flag flipped, ItemChart `UpdateData` invoked, etc.).

Per-row pattern:
```rust
let mut h = Harness::new();
let panel = h.create_control_panel();
h.fire(panel.widgets.as_ref().unwrap().auto_update_dates.check_signal);
h.run_cycle();
assert!(panel.update_controls_needed);
```

For accessor rows (G1/G2/G3/G4): assert that firing `model.signal_change()` propagates to a subscriber's Cycle.

**No new pixel-level golden tests.** The drift surface is signal flow, not paint. Existing emstocks goldens remain the regression backstop for paint output.

**Annotation checks.** Where G2 picks Option (B), if any DIVERGED-tagged code is added, run `cargo xtask annotations`.

## Open items deferred to working-memory session

1. **Reconcile audit-data corrections** into `inventory-enriched.json`:
   - Tag the 3 accessor-side rows (FileModel/Config/PricesFetcher) with the corrected nuance (FileModel is a delegating-accessor add, not a SignalId add).
   - Tag the 20 G8 rows with "missing-widget + missing-subscribe" rather than just "missing-subscribe" — the accessor *exists* on the widget type; the widget instance is what's missing.
   - `emStocksListBox-53` accessor exists (inherited); only consumer-side wiring needed.
2. **G3 consumer absence.** B-001 has no consumer of `PricesFetcher::GetChangeSignal`. Confirm with cpp grep of `AddWakeUpSignal(.*PricesFetcher`. If a consumer exists in C++ that the audit missed, escalate as a B-001 amendment.
3. **G2 design choice** — confirm Option (B) (add SignalId field directly to plain struct) is acceptable; the alternative (compose with `emConfigModel`) has wider blast radius beyond B-001. No new D-### needed; this is a per-bucket within-D-006 detail.
4. **emStocksListBox Rc<RefCell<>> additions** for FileModel/Config — flag in case the working-memory session wants to escalate to a global decision. The justification chain: C++ holds these as members; preserving observable signal-flow requires Rust to do the same.
5. **AutoExpand-deferred widget subscribe** is a local pattern (two-tier init flag). If multiple buckets rediscover it, may warrant promotion to D-007. Not proposing that here — single occurrence.

## Success criteria

- All 71 rows have a `connect(...)` call in their panel's first-Cycle init block (or a deferred-init equivalent for AutoExpand-gated widgets).
- All 71 rows have a corresponding `IsSignaled(...)` branch in the panel's Cycle body, in C++ source order.
- Three accessor-side rows have a Rust accessor matching the C++ contract (or its delegating equivalent for inherited signals).
- 20 G8 widget instances exist as fields on `ControlWidgets`.
- `cargo clippy -D warnings` and `cargo-nextest ntr` pass.
- New `tests/typed_subscribe_b001.rs` covers all 71 rows.
- B-001 status in `work-order.md` flips `pending → designed`.
