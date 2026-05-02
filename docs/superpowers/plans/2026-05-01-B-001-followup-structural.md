# B-001-followup — Structural pre-phase + deferred D-006 wiring (B-001 4.1/4.2/4.3 + B-017 row 1) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the structural scaffolding the original B-001 Phase 4 implementer flagged as missing — `PanelBehavior::Cycle` + parent-instantiation + `Rc<RefCell<>>` member references on `emStocksControlPanel` / `emStocksItemPanel` / `emStocksItemChart`, plus engine promotion for `emStocksPricesFetcher` — then re-execute the 67 deferred D-006 rows from B-001 Phase 4.1/4.2/4.3 and the 2 upstream-fetcher rows from B-017 row 1.

**Architecture:** Phased bottom-up. Each panel gets its structural pre-phase (Phase A / Phase C) followed immediately by its D-006 wiring (Phase B / Phase D), so the wiring task targets a real `Cycle` body it just touched. Fetcher engine promotion + B-017 fetcher-side subscribes ride alongside in Phase E (independent of the panel structural work). Phase F closes the loop with a no-`#[allow]` audit, work-order entry, and design-doc deferral-section closure for B-001 + B-017.

**Tech Stack:** Rust, `emcore::emEngineCtx::{SignalCtx, ConstructCtx, EngineCtx, PanelCtx, SignalId}`, `emcore::emPanel::PanelBehavior`, `cargo`, `cargo-nextest`. Reference: Eagle Mode 0.96.4 at `~/Projects/eaglemode-0.96.4/src/emStocks/` and `~/Projects/eaglemode-0.96.4/include/emStocks/`.

---

## Pre-flight (run before Phase A)

- [ ] **Confirm working tree clean.** `git status` reports clean (or only this plan file).
- [ ] **Confirm baseline green.** `cargo check --workspace` succeeds; `cargo clippy --workspace --all-targets -- -D warnings` succeeds; `cargo-nextest ntr` matches the recorded baseline (record the count seen at this gate; B-001 Phase 5 final-gate count is the floor).
- [ ] **Read the B-001 design doc deferral section** at `docs/superpowers/specs/2026-04-27-B-001-no-wire-emstocks-design.md` — specifically the §"Phase 4 Partial Merge — 2026-05-01" block (lines ~463-505 at time of writing) which enumerates the three blocking conditions.
- [ ] **Read the B-017 design doc** at `docs/superpowers/specs/2026-04-27-B-017-polling-no-acc-emstocks-design.md` §I-1 / §"Resolutions from Adversarial Review" item 4 / Amendment Log — specifically the upstream-subscribes coordination deferral.
- [ ] **Read the B-001 plan** at `docs/superpowers/plans/2026-05-01-B-001-no-wire-emstocks.md` for the canonical D-006/D-007/D-008 patterns this plan re-executes verbatim. The `&mut impl SignalCtx` mutator-bound and D-008 A1 combined-form accessor shapes are precedents — do not re-derive.
- [ ] **Read the latest work-order entry** at `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/work-order.md` (last `### 2026-05-01 — B-001 Phase 1-5 partial merge` entry) for the exact "67 of 71 rows deferred" framing.
- [ ] **Verify M-001 per-panel:** open every C++ `.cpp` in scope (`emStocksControlPanel.cpp`, `emStocksItemPanel.cpp`, `emStocksItemChart.cpp`, `emStocksPricesFetcher.cpp`) and `grep -nE 'IsSignaled|AddWakeUpSignal|Signal\('` to count branches and subscribes. Compare to design doc per-panel tables. Record any divergence as a design-doc errata before starting Phase B / D / E.
- [ ] **Verify the C++ factory sites:**
  - `emStocksFilePanel.cpp:237-245` — `emStocksFilePanel::CreateControlPanel` returns `new emStocksControlPanel(parent, name, *FileModel, *Config, *ListBox)` when VFS is good, else delegates to `emFilePanel::CreateControlPanel`.
  - `emStocksListBox.cpp:696-701` — `emStocksListBox::CreateItemPanel(name, itemIndex)` constructs `new emStocksItemPanel(*this, name, itemIndex, FileModel, Config)`.
  - `emStocksItemPanel.cpp:549` — `Chart=new emStocksItemChart(l1,"Chart",ListBox,Config);` (ItemChart is a child of ItemPanel's expanded layout, not ListBox direct).
- [ ] **Verify the C++ fetcher subscribes:** `emStocksPricesFetcher.cpp:38-39` —
  ```cpp
  AddWakeUpSignal(FileModel->GetChangeSignal());
  AddWakeUpSignal(FileModel->GetFileStateSignal());
  ```
  These are the two upstream subscribes B-017 row 1 fetcher-side must port.
- [ ] **Verify the C++ ItemPanel/ItemChart subscribes:** both ctors call (from `emStocksItemPanel.cpp:65-66` and `emStocksItemChart.cpp:64-65`):
  ```cpp
  AddWakeUpSignal(Config.GetChangeSignal());
  AddWakeUpSignal(ListBox.GetSelectedDateSignal());
  ```
  Plus an `emRecListener` parent base class subscribe. ItemPanel additionally subscribes its widgets in `AutoExpand` (per the B-001 design doc per-panel table).

LLM-failure-mode guards (CLAUDE.md "Plan Tool Rules"):

- **No `#[allow(...)]` / `#[expect(...)]`.** Fix warnings at the cause. Allowed exceptions: `non_snake_case` on the `emCore` module / `em`-prefixed types, `too_many_arguments`. Anything else is a bug.
- **No polling intermediaries (D-009).** Do not introduce a `Cell`/`RefCell` field set in site A and drained by site B's `Cycle`. Thread `ectx` to site A and fire synchronously per D-007.
- **D-007 ectx-threading discipline.** Mutator signatures use `&mut impl SignalCtx`, NOT `&mut EngineCtx<'_>`. Deferred-fire (`DropOnlySignalCtx`) only at Rust-`Drop`-language-forced sites.
- **File and Name Correspondence.** Method/type names match C++ verbatim. Forced renames carry `DIVERGED:` with category. New Rust-only fields carry `RUST_ONLY:` with charter.
- **TDD-first.** Each wiring task adds the behavioral test before the wiring (the test compiles and FAILs before the production code change).
- **Pre-commit hook is the source of truth per commit.** It runs `cargo fmt`, `cargo clippy -D warnings`, `cargo-nextest ntr`, and `cargo xtask annotations`. Per-task gate: `cargo check -p emstocks` + `cargo clippy -p emstocks --all-targets -- -D warnings` + `cargo nextest run -p emstocks`. Full nextest at Phase F.
- **M-001 per task:** every D-006 wiring task opens the matching C++ `Cycle` block and counts `IsSignaled` branches before writing the Rust block.

---

## Phase A — `emStocksControlPanel` structural

**Scope:** Add `PanelBehavior` impl with `Cycle`, add `file_model` / `config` / `list_box` `Rc<RefCell<>>` member-reference fields, add a `subscribed_init: bool` flag, change the constructor to accept those refs, wire `emStocksFilePanel::CreateControlPanel` to construct it. **No D-006 wiring yet** — Cycle body is a no-op + first-Cycle-init scaffold only.

**Files in scope:**
- Modify: `crates/emstocks/src/emStocksControlPanel.rs` (struct, ctor, `PanelBehavior` impl, `Cycle`, `Default`)
- Modify: `crates/emstocks/src/emStocksFilePanel.rs` (add `CreateControlPanel` override; wire to behavior trait)
- Modify: `crates/emstocks/src/emStocksFilePanel.rs` test fixtures (`emStocksFilePanel` ctor cascade)
- Modify: `crates/emstocks/src/emStocksControlPanel.rs` `#[cfg(test)] mod tests` (`make_panel` fixture cascade)

### Pre-checks

- [ ] **Verify Rust state.** `crates/emstocks/src/emStocksControlPanel.rs:467-480` shows the current `pub struct emStocksControlPanel { look, update_controls_needed, widgets }` and `new(look)` ctor. `Default` impl exists at line ~721.
- [ ] **Verify C++ ctor signature.** `emStocksControlPanel.h:33-36` and `emStocksControlPanel.cpp:25-80` — ctor takes `(ParentArg parent, const emString& name, emStocksFileModel& fileModel, emStocksConfig& config, emStocksListBox& listBox)`.
- [ ] **Verify the Rust `emStocksFilePanel` already holds the refs we need:** `crates/emstocks/src/emStocksFilePanel.rs:59` shows `pub(crate) config: Rc<RefCell<emStocksConfig>>`, the `list_box: Option<emStocksListBox>` at `:61`, and `model: Rc<RefCell<emStocksFileModel>>` (search the field block).
- [ ] **Verify `PanelBehavior::CreateControlPanel` trait hook** at `crates/emcore/src/emPanel.rs:404-411` accepts `(&mut self, &mut PanelCtx, &str, bool)` and returns `Option<PanelId>`. The default returns `None` (delegating up).
- [ ] **Verify `emStocksListBox` holds Rc refs** (post-B-001 Phase 3 at `ce7e85b4`). `crates/emstocks/src/emStocksListBox.rs:20-150` should show `Option<Rc<RefCell<emStocksFileModel>>>` + `Option<Rc<RefCell<emStocksConfig>>>` fields. We need to wrap the `emStocksListBox` itself in `Rc<RefCell<>>` on the FilePanel side so ControlPanel can borrow it.

### Task A.1 — wrap `list_box` in `Rc<RefCell<>>` on `emStocksFilePanel`

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs:61` — change `list_box: Option<emStocksListBox>` to `list_box: Option<Rc<RefCell<emStocksListBox>>>`.
- Modify: `crates/emstocks/src/emStocksFilePanel.rs` everywhere `self.list_box.as_ref()` / `self.list_box.is_none()` / `let mut lb = emStocksListBox::new()` / `self.list_box = Some(lb)` — locate via `rg -n 'list_box' crates/emstocks/src/emStocksFilePanel.rs`.

- [ ] **Step 1: Audit current touchpoints.** Run: `rg -n 'self\.list_box|list_box' crates/emstocks/src/emStocksFilePanel.rs`. Record every line.

- [ ] **Step 2: Change the field type.**

```rust
// crates/emstocks/src/emStocksFilePanel.rs
// (around line 61)
pub(crate) list_box: Option<Rc<RefCell<emStocksListBox>>>,
```

- [ ] **Step 3: Update construction site** in `Cycle`'s VFS-good branch where `let mut lb = emStocksListBox::new();` lives (around `:480-490`):

```rust
let mut lb = emStocksListBox::new();
// ... existing Phase 3 ref-handing into lb ...
self.list_box = Some(Rc::new(RefCell::new(lb)));
```

- [ ] **Step 4: Update reads.** Each `self.list_box.as_ref().map(|lb| ...)` becomes `self.list_box.as_ref().map(|lb| lb.borrow().<method>())`. Each `self.list_box.as_mut()` becomes `self.list_box.as_ref().map(|lb| lb.borrow_mut())`. Note: in `Cycle`, when ListBox's own `Cycle` runs (B-001 Phase 4.5), the existing pattern is `if let Some(ref mut lb) = self.list_box { lb.Cycle(...) }` — this becomes `if let Some(lb) = self.list_box.as_ref() { lb.borrow_mut().Cycle(...) }`. Watch for borrow conflicts when `self.config` is also borrowed in the same scope; if a conflict arises, scope each `borrow_mut()` to a `{ ... }` block.

- [ ] **Step 5: Update Paint** (around `:105`): `if let Some(ref list_box) = self.list_box { list_box.PaintEmptyMessage(...) }` → `if let Some(lb) = self.list_box.as_ref() { lb.borrow().PaintEmptyMessage(...) }`.

- [ ] **Step 6: `cargo check -p emstocks`.** Fix any callsites (tests included).

- [ ] **Step 7: `cargo clippy -p emstocks --all-targets -- -D warnings`.** Clean.

- [ ] **Step 8: `cargo nextest run -p emstocks`.** Existing tests pass.

- [ ] **Step 9: Commit.**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs
git commit -m "$(cat <<'EOF'
refactor(emstocks,B-001-followup A.1): wrap list_box in Rc<RefCell<>>

emStocksFilePanel.list_box becomes Option<Rc<RefCell<emStocksListBox>>>
so ControlPanel (Phase A.2) can hold a cross-Cycle reference mirroring
the C++ `emStocksListBox &` member. (a)-justified Rc<RefCell<>>: ListBox
will be co-borrowed by FilePanel::Cycle and ControlPanel::Cycle.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

### Task A.2 — extend `emStocksControlPanel` with member-reference fields

**Files:**
- Modify: `crates/emstocks/src/emStocksControlPanel.rs:467-480` (struct + ctor)
- Modify: `crates/emstocks/src/emStocksControlPanel.rs:721-...` (`Default` impl) — keep `Default` for tests but produce dummy refs, OR remove `Default` and audit tests.

- [ ] **Step 1: Write a failing struct-shape test** in `#[cfg(test)] mod tests`:

```rust
#[test]
fn control_panel_holds_member_refs() {
    let model = Rc::new(RefCell::new(emStocksFileModel::new(
        std::path::PathBuf::from("/tmp/fp_a2.emStocks"),
    )));
    let config = Rc::new(RefCell::new(emStocksConfig::default()));
    let list_box = Rc::new(RefCell::new(emStocksListBox::new()));
    let look = Rc::new(emLook::new());
    let panel = emStocksControlPanel::new(look, model.clone(), config.clone(), list_box.clone());
    // strong_count goes to 2 (held by panel + held by test scope)
    assert_eq!(Rc::strong_count(&model), 2);
    assert_eq!(Rc::strong_count(&config), 2);
    assert_eq!(Rc::strong_count(&list_box), 2);
}
```

- [ ] **Step 2: Run, expect compile failure** (`new` arity mismatch).

Run: `cargo test -p emstocks --lib emStocksControlPanel::tests::control_panel_holds_member_refs`
Expected: `error[E0061]: this function takes 1 argument but 4 arguments were supplied`.

- [ ] **Step 3: Edit the struct + ctor.**

```rust
pub struct emStocksControlPanel {
    pub(crate) look: Rc<emLook>,
    /// C++ `emStocksFileModel & FileModel` member reference. (a)-justified —
    /// borrowed across Cycle and held by FilePanel as well.
    pub(crate) file_model: Rc<RefCell<emStocksFileModel>>,
    /// C++ `emStocksConfig & Config` member reference.
    pub(crate) config: Rc<RefCell<emStocksConfig>>,
    /// C++ `emStocksListBox & ListBox` member reference.
    pub(crate) list_box: Rc<RefCell<emStocksListBox>>,
    pub(crate) update_controls_needed: bool,
    pub(crate) widgets: Option<ControlWidgets>,
    /// D-006 first-Cycle init flag; mirrors the B-001 ListBox pattern at
    /// emStocksListBox.rs (Phase 4.5 precedent).
    pub(crate) subscribed_init: bool,
}

impl emStocksControlPanel {
    pub fn new(
        look: Rc<emLook>,
        file_model: Rc<RefCell<emStocksFileModel>>,
        config: Rc<RefCell<emStocksConfig>>,
        list_box: Rc<RefCell<emStocksListBox>>,
    ) -> Self {
        Self {
            look,
            file_model,
            config,
            list_box,
            update_controls_needed: true,
            widgets: None,
            subscribed_init: false,
        }
    }
    // ... rest unchanged ...
}
```

- [ ] **Step 4: Remove `impl Default for emStocksControlPanel`** (around line 721) — `Default` cannot fabricate a meaningful `file_model` path. Replace with a `#[cfg(test)] pub(crate) fn for_test()` helper if any test needs it:

```rust
#[cfg(test)]
impl emStocksControlPanel {
    pub(crate) fn for_test() -> Self {
        Self::new(
            Rc::new(emLook::new()),
            Rc::new(RefCell::new(emStocksFileModel::new(
                std::path::PathBuf::from("/tmp/control_panel_test.emStocks"),
            ))),
            Rc::new(RefCell::new(emStocksConfig::default())),
            Rc::new(RefCell::new(emStocksListBox::new())),
        )
    }
}
```

- [ ] **Step 5: Update existing test fixture** at `:763-764` (`fn make_panel() -> emStocksControlPanel { emStocksControlPanel::new(emLook::new()) }`) → `emStocksControlPanel::for_test()`.

- [ ] **Step 6: Run new + existing tests.**

Run: `cargo test -p emstocks --lib emStocksControlPanel`
Expected: PASS for `control_panel_holds_member_refs` and existing `make_panel`-using tests.

- [ ] **Step 7: `cargo clippy -p emstocks --all-targets -- -D warnings`.** Clean.

- [ ] **Step 8: Commit.**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs
git commit -m "$(cat <<'EOF'
feat(emstocks,B-001-followup A.2): emStocksControlPanel member refs

Mirrors C++ ctor signature emStocksControlPanel(parent, name,
FileModel&, Config&, ListBox&) with (a)-justified Rc<RefCell<>>. Adds
subscribed_init flag for the upcoming D-006 first-Cycle wiring (Phase B).
Default impl replaced with #[cfg(test)] for_test() helper.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

### Task A.3 — `PanelBehavior::Cycle` no-op scaffold + first-Cycle-init slot

**Files:**
- Modify: `crates/emstocks/src/emStocksControlPanel.rs` — add `impl PanelBehavior for emStocksControlPanel` block.

- [ ] **Step 1: Failing test** — assert that calling `Cycle` once flips `subscribed_init` to true:

```rust
#[test]
fn control_panel_first_cycle_flips_subscribed_init() {
    use emcore::emEngineCtx::test_support::TestPanelHarness;
    let mut harness = TestPanelHarness::new();
    let mut panel = emStocksControlPanel::for_test();
    assert!(!panel.subscribed_init);
    harness.cycle_behavior(&mut panel);
    assert!(panel.subscribed_init);
}
```

(If `TestPanelHarness::cycle_behavior` does not exist, use the same fixture B-001 Phase 4.5 used to drive ListBox — search `rg -n 'fn cycle_behavior|drive_cycle' crates/emcore/src` and reuse.)

- [ ] **Step 2: Run, expect compile failure** (no `PanelBehavior` impl).

- [ ] **Step 3: Add the impl.**

```rust
impl emcore::emPanel::PanelBehavior for emStocksControlPanel {
    fn Cycle(
        &mut self,
        _ectx: &mut emcore::emEngineCtx::EngineCtx<'_>,
        _pctx: &mut emcore::emEngineCtx::PanelCtx,
    ) -> bool {
        // D-006 first-Cycle init slot. Phase B will populate this with the
        // 37 deferred row subscribes per the B-001 design doc per-panel table.
        if !self.subscribed_init {
            // Subscribes go here in Phase B.
            self.subscribed_init = true;
        }
        false
    }
}
```

- [ ] **Step 4: Run test, expect PASS.**

- [ ] **Step 5: `cargo check / clippy / nextest -p emstocks` clean.**

- [ ] **Step 6: Commit.**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs
git commit -m "$(cat <<'EOF'
feat(emstocks,B-001-followup A.3): PanelBehavior::Cycle scaffold

Adds the empty Cycle body + subscribed_init first-Cycle gate. No
subscribes wired yet — Phase B will populate. Behavioral test asserts
the first-Cycle latch flips.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

### Task A.4 — `emStocksFilePanel::CreateControlPanel` factory

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs` (`PanelBehavior` impl block at `:89`)

- [ ] **Step 1: Failing test** in `crates/emstocks/tests/typed_subscribe_b001.rs` (or a new `crates/emstocks/tests/control_panel_factory.rs` if the typed-subscribe file is too crowded):

```rust
#[test]
fn file_panel_create_control_panel_returns_some_when_vfs_good() {
    use emcore::emEngineCtx::test_support::PanelTreeFixture;
    let mut fixture = PanelTreeFixture::new();
    let fp_id = fixture.create_panel_with_behavior(
        "fp", emStocksFilePanel::new(/*...*/),
    );
    // Drive VFS to Loaded so Cycle creates the list_box.
    fixture.set_vfs_loaded(fp_id);
    fixture.cycle();
    // Now CreateControlPanel must return Some.
    let cp_id = fixture.create_control_panel(fp_id, "ctrl");
    assert!(cp_id.is_some(), "VFS-good FilePanel must yield a ControlPanel");
}
```

(If the test fixture API differs, mirror the existing pattern at `crates/emstocks/tests/typed_subscribe_b001.rs` for B-001 Phase 4.5; do NOT invent fixture APIs — reuse existing ones, scoped to a new helper if needed.)

- [ ] **Step 2: Run, expect FAIL** (`CreateControlPanel` returns `None` per default).

- [ ] **Step 3: Override `CreateControlPanel` in `impl PanelBehavior for emStocksFilePanel`.**

```rust
fn CreateControlPanel(
    &mut self,
    parent_ctx: &mut emcore::emEngineCtx::PanelCtx,
    name: &str,
    _self_is_active: bool,
) -> Option<emcore::emEngineCtx::PanelId> {
    // Mirrors C++ emStocksFilePanel.cpp:237-245 — VFS-good path constructs
    // emStocksControlPanel; otherwise delegates to emFilePanel default
    // (Rust trait default returns None, which the tree walker turns into
    // an upward delegation chain — equivalent to C++ delegation).
    if !self.file_panel.GetVirFileState().is_good() {
        return None;
    }
    let list_box = self.list_box.as_ref()?.clone();
    let cp = emStocksControlPanel::new(
        self.look.clone(),
        self.model.clone(),
        self.config.clone(),
        list_box,
    );
    parent_ctx.create_child_panel(name, Box::new(cp))
}
```

(`create_child_panel` is the canonical helper; if its name in `PanelCtx` differs, locate via `rg -n 'create_child|create_panel' crates/emcore/src/emEngineCtx.rs crates/emcore/src/emPanelTree.rs` and use the existing API. Do not invent.)

- [ ] **Step 4: Run test, expect PASS.**

- [ ] **Step 5: `cargo check / clippy / nextest -p emstocks` clean.**

- [ ] **Step 6: Verify with manual smoke** (skip if no GUI affordance — the test gate above is sufficient).

- [ ] **Step 7: Commit.**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs crates/emstocks/tests/
git commit -m "$(cat <<'EOF'
feat(emstocks,B-001-followup A.4): emStocksFilePanel.CreateControlPanel

Mirrors C++ emStocksFilePanel.cpp:237-245 — when VFS is good, construct
emStocksControlPanel with the FileModel/Config/ListBox refs; otherwise
delegate to the trait default (which the tree walker turns into upward
parent delegation, matching the C++ emFilePanel::CreateControlPanel
fallback).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

### Phase A verification gate

- [ ] `cargo check -p emstocks` clean.
- [ ] `cargo clippy -p emstocks --all-targets -- -D warnings` clean.
- [ ] `cargo nextest run -p emstocks` green.
- [ ] `cargo xtask annotations` clean (no new annotation regressions).
- [ ] No new `#[allow]` / `#[expect]` introduced (`rg -n '#\[allow|#\[expect' crates/emstocks/`).

---

## Phase B — `emStocksControlPanel` D-006 wiring (re-execute deferred Task 4.1)

**Scope:** Subscribe the 37 ControlPanel rows from the B-001 design doc per-panel table §"emStocksControlPanel (37 rows)" (`docs/superpowers/specs/2026-04-27-B-001-no-wire-emstocks-design.md:237-263`). For each subscribe, drop the `_` prefix from the matching `ControlWidgets` field added in B-001 Phase 2 (`65f45229`).

**Files:**
- Modify: `crates/emstocks/src/emStocksControlPanel.rs`
- Modify: `crates/emstocks/tests/typed_subscribe_b001.rs` (or sibling test file)

**Reference table (37 rows):** B-001 design doc §"emStocksControlPanel (37 rows)". Implementer reads that section row-by-row. The summary breakdown is:
- 1 × FileModel ChangeSignal subscribe (G1 consumer)
- 1 × Config ChangeSignal subscribe (G2 consumer)
- 1 × ListBox SelectedDate subscribe (G4 consumer)
- 1 × ListBox Selection subscribe (G5 consumer; deferred to second-Cycle if ListBox not yet attached — see B-001 design §Sequencing)
- 1 × ListBox ItemTrigger subscribe (G6 consumer)
- 32 × widget-instance subscribes (G7/G8 widget signals — the 21 underscore-prefixed `_xxx` fields plus 11 already-non-prefixed widgets)

### Task B.0 — Test scaffold

**Files:**
- Modify: `crates/emstocks/tests/typed_subscribe_b001.rs`

- [ ] **Step 1: Add a `mod control_panel_subscribes` block** with one `#[test]` per row category. Stub each test with the assertion shape from B-001 Phase 4.5 (e.g., the `test_listbox_selection_subscribe_*` precedents at `crates/emstocks/tests/typed_subscribe_b001.rs` from commit `7ce5f674`). One test per subscribe is over-budget; instead, one test per **category** (G1, G2, G4, G5, G6, then a sweep over the 32 widget rows) — mirrors the B-001 Phase 4.5 coverage compromise (representative click-throughs + construction-only sweeps).

- [ ] **Step 2: All tests fail to compile or fail at assertion** (no subscribe yet). Confirm.

- [ ] **Step 3: Commit (red).**

```bash
git add crates/emstocks/tests/typed_subscribe_b001.rs
git commit -m "test(emstocks,B-001-followup B.0): control_panel subscribe red bar"
```

### Task B.1 — G1/G2/G4 model+config+selected-date subscribes

- [ ] **Step 1: Inside `Cycle`'s first-Cycle-init block (Task A.3 slot), add three `ectx.connect` calls.**

```rust
if !self.subscribed_init {
    let eid = ectx.engine_id;

    // G1 consumer: FileModel ChangeSignal.
    let model_sig = self.file_model.borrow().GetChangeSignal();
    ectx.connect(model_sig, eid);
    self.model_change_sig = Some(model_sig);

    // G2 consumer: Config ChangeSignal (D-008 A1 — pass ectx).
    let cfg_sig = self.config.borrow().GetChangeSignal(ectx);
    ectx.connect(cfg_sig, eid);
    self.config_change_sig = Some(cfg_sig);

    // G4 consumer: ListBox SelectedDateSignal.
    let sd_sig = self.list_box.borrow().GetSelectedDateSignal();
    ectx.connect(sd_sig, eid);
    self.selected_date_sig = Some(sd_sig);

    self.subscribed_init = true;
}
```

(Add the three `Option<SignalId>` fields to the struct first — mirror B-001 Phase 4.5's `selected_date_sig` precedent on `emStocksFilePanel:86`.)

- [ ] **Step 2: After the first-Cycle gate, add the IsSignaled reaction block** mirroring C++ `emStocksControlPanel.cpp` Cycle body (open the C++ file, count branches per M-001 — should be 1 branch reading all three signals OR'd into `update_controls_needed = true`):

```rust
let mut needs_update = self.update_controls_needed;
if self.model_change_sig.map(|s| ectx.IsSignaled(s)).unwrap_or(false) {
    needs_update = true;
}
if self.config_change_sig.map(|s| ectx.IsSignaled(s)).unwrap_or(false) {
    needs_update = true;
}
if self.selected_date_sig.map(|s| ectx.IsSignaled(s)).unwrap_or(false) {
    needs_update = true;
}
if needs_update {
    self.update_controls_needed = true;
    // UpdateControls call requires PanelCtx + borrows — invoke per existing
    // pattern (the existing UpdateControls method takes &emStocksConfig +
    // &emStocksRec + &emStocksListBox + PanelCtx). Borrow the refs:
    let cfg = self.config.borrow();
    let model = self.file_model.borrow();
    let lb = self.list_box.borrow();
    self.UpdateControls(&cfg, model.GetRec(), &lb, _pctx);
}
```

(If `model.GetRec()` is not the right accessor, search the C++ — `emStocksControlPanel.cpp` `UpdateControls()` reads `*FileModel` directly; locate the Rust analogue via `rg -n 'GetRec|stocks_rec' crates/emstocks/src/emStocksFileModel.rs`.)

- [ ] **Step 3: Run G1/G2/G4 tests, expect PASS.**

- [ ] **Step 4: clippy + nextest clean.**

- [ ] **Step 5: Commit.**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs crates/emstocks/tests/typed_subscribe_b001.rs
git commit -m "feat(emstocks,B-001-followup B.1): ControlPanel G1/G2/G4 subscribes (3 rows)"
```

### Task B.2 — G5/G6 ListBox Selection / ItemTrigger subscribes

- [ ] **Step 1: Inside the first-Cycle-init block (after Task B.1's three subscribes), add the two delegated subscribes.** Note the early-return pattern from B-001 design §Sequencing:

```rust
// G5 consumer: ListBox SelectionSignal (delegating accessor — early-return
// if inner emListBox not yet attached). B-001 design §Sequencing: this
// matches the deferred-init pattern.
if let Some(sel_sig) = self.list_box.borrow().GetSelectionSignal() {
    ectx.connect(sel_sig, eid);
    self.selection_sig = Some(sel_sig);
}
// G6 consumer: ListBox ItemTriggerSignal (delegating accessor).
if let Some(trig_sig) = self.list_box.borrow().GetItemTriggerSignal() {
    ectx.connect(trig_sig, eid);
    self.item_trigger_sig = Some(trig_sig);
}
```

If either is `None` here, the `subscribed_init` latch should NOT flip — defer to next Cycle. Adjust the latch accordingly:

```rust
let g5_g6_ready = self.selection_sig.is_some() && self.item_trigger_sig.is_some();
if g1_g2_g4_done && g5_g6_ready {
    self.subscribed_init = true;
}
```

(Match the B-001 design §Sequencing two-tier `subscribed_init` + `subscribed_widgets` pattern. The original B-001 plan's Task 4.1 specced this — re-read it for the exact split.)

- [ ] **Step 2: Add IsSignaled branches** for selection + item-trigger per the C++ Cycle body (open the C++ and count).

- [ ] **Step 3: Tests pass; clippy + nextest clean.**

- [ ] **Step 4: Commit.**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs crates/emstocks/tests/
git commit -m "feat(emstocks,B-001-followup B.2): ControlPanel G5/G6 subscribes (2 rows)"
```

### Task B.3 — G7/G8 widget-instance subscribes (32 rows)

This task drops the underscore prefix from each of the 21 underscore-prefixed `ControlWidgets` fields and adds the corresponding click/value-change subscribe. Plus 11 already-non-prefixed widget subscribes. Total: 32 rows.

The B-001 design doc §"emStocksControlPanel (37 rows)" enumerates each row with its signal kind (`GetClickSignal`, `GetValueSignal`, `GetTextSignal`). **Do not paraphrase from memory** — re-read the design doc row-by-row.

- [ ] **Step 1: Subscribed-widgets latch.** Add a `subscribed_widgets: bool` field. Widget subscribes go in `AutoExpand` (because widgets only exist when expanded — see B-001 design §"AutoExpand-deferred widget subscribe", per-panel table column for ControlPanel).

```rust
pub fn AutoExpand<C: emcore::emEngineCtx::ConstructCtx>(&mut self, cc: &mut C) {
    let look = self.look.clone();
    self.widgets = Some(ControlWidgets::new(cc, look));
    self.update_controls_needed = true;
    self.subscribed_widgets = false; // re-subscribe on every expand
}
```

(In `Cycle`, after the first-Cycle init block, add a `subscribed_widgets` gate that runs once per AutoExpand.)

- [ ] **Step 2: For each of the 32 widget rows in the design doc table, drop the `_` prefix from the field, then in `Cycle`'s widgets-subscribe block call:**

```rust
if let Some(w) = self.widgets.as_ref() {
    if !self.subscribed_widgets {
        ectx.connect(w.fetch_share_prices.GetClickSignal(ectx), eid);
        ectx.connect(w.delete_share_prices.GetClickSignal(ectx), eid);
        // ... 30 more — ONE PER ROW IN THE DESIGN DOC ...
        self.subscribed_widgets = true;
    }
}
```

(`emButton::GetClickSignal`, `emCheckBox::GetCheckedSignal`, etc. — reuse the exact accessor names from B-001 Phase 1 G accessors and the widget-side `GetXxxSignal` D-008 A1 accessors. If a needed accessor does not exist, that is a B-001 prereq miss — escalate, do not invent.)

- [ ] **Step 3: Add the IsSignaled reaction branches.** Per C++ `emStocksControlPanel.cpp` Cycle, each click triggers a corresponding action (e.g., `Config.Sorting = ByName`, `FetchSharePrices()`). Mirror these. M-001: count C++ branches before writing.

- [ ] **Step 4: Run all 32 widget tests.**

- [ ] **Step 5: clippy + nextest clean.**

- [ ] **Step 6: Commit (may split into 2-3 sub-commits for diff reviewability).**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs crates/emstocks/tests/
git commit -m "feat(emstocks,B-001-followup B.3): ControlPanel G7/G8 widget subscribes (32 rows)"
```

### Phase B verification gate

- [ ] `cargo check -p emstocks` clean.
- [ ] `cargo clippy -p emstocks --all-targets -- -D warnings` clean.
- [ ] `cargo nextest run -p emstocks` green.
- [ ] `cargo xtask annotations` clean.
- [ ] `rg -n '#\[allow|#\[expect' crates/emstocks/` — no new entries.
- [ ] **Verify 37 rows wired:** `rg -nc 'ectx\.connect' crates/emstocks/src/emStocksControlPanel.rs` matches the design doc count.
- [ ] **Verify no underscore-prefixed `_xxx: emButton` or `_xxx: emTextField` fields remain** in `ControlWidgets`: `rg -n 'pub\(crate\) _\w+:' crates/emstocks/src/emStocksControlPanel.rs` is empty (or only the two `_min_visible_interest_buttons` / `_sorting_buttons` `Vec` storage that B-001 Phase 2 already left underscore-prefixed for separate reasons).

---

## Phase C — `emStocksItemPanel` + `emStocksItemChart` structural

**Scope:** Same shape as Phase A, but for ItemPanel and ItemChart. Add member-reference fields, `subscribed_init` latch, `PanelBehavior::Cycle` no-op scaffold, and the `emStocksListBox::CreateItemPanel` + ItemPanel-internal `Chart=new emStocksItemChart(...)` factory wiring.

**Files:**
- Modify: `crates/emstocks/src/emStocksItemPanel.rs`
- Modify: `crates/emstocks/src/emStocksItemChart.rs`
- Modify: `crates/emstocks/src/emStocksListBox.rs` (`CreateItemPanel` factory)

### Pre-checks

- [ ] **Verify C++ ItemPanel ctor** at `emStocksItemPanel.cpp:26-66` — takes `(emStocksListBox & parent, const emString & name, int itemIndex, emStocksFileModel & fileModel, emStocksConfig & config)`; ctor body subscribes `Config.GetChangeSignal()` + `ListBox.GetSelectedDateSignal()`.
- [ ] **Verify C++ ItemChart ctor** at `emStocksItemChart.cpp:25-66` — takes `(ParentArg parent, const emString & name, emStocksListBox & listBox, emStocksConfig & config)`; ctor body subscribes the same two signals.
- [ ] **Verify C++ ItemChart instantiation site** at `emStocksItemPanel.cpp:549` — `Chart=new emStocksItemChart(l1,"Chart",ListBox,Config);` inside `AutoExpand`. Note: ItemChart needs `ListBox` + `Config` only (not `FileModel`) — verify by re-reading the ctor.
- [ ] **Verify ListBox factory** at `emStocksListBox.cpp:696-701` — `CreateItemPanel(name, itemIndex)` constructs `new emStocksItemPanel(*this, name, itemIndex, FileModel, Config)`. The Rust analogue in `crates/emcore/src/emListBox.rs:1050-1065` is `pub fn CreateItemPanel(&mut self, index: usize)` with an `item_panel_factory` field at `:323` — the override mechanism.

### Task C.1 — `emStocksItemPanel` member-reference fields + ctor cascade

**Files:**
- Modify: `crates/emstocks/src/emStocksItemPanel.rs:185-222` (struct + `new`)

- [ ] **Step 1: Failing struct-shape test** (mirror Task A.2 shape).

- [ ] **Step 2: Edit struct.**

```rust
pub struct emStocksItemPanel {
    pub(crate) look: Rc<emLook>,
    /// C++ `emStocksFileModel & FileModel` member reference.
    pub(crate) file_model: Rc<RefCell<emStocksFileModel>>,
    /// C++ `emStocksConfig & Config` member reference.
    pub(crate) config: Rc<RefCell<emStocksConfig>>,
    /// C++ `emStocksListBox & ListBox` member reference.
    pub(crate) list_box: Rc<RefCell<emStocksListBox>>,
    pub(crate) stock_rec_index: Option<usize>,
    pub(crate) update_controls_needed: bool,
    pub country: CategoryPanel,
    pub sector: CategoryPanel,
    pub collection: CategoryPanel,
    pub(crate) widgets: Option<ItemWidgets>,
    pub(crate) chart: Option<emStocksItemChart>,
    pub(crate) subscribed_init: bool,
    pub(crate) subscribed_widgets: bool,
    // existing prev_* fields unchanged
    pub prev_own_shares: String,
    pub prev_purchase_price: String,
    pub prev_purchase_date: String,
    pub prev_sale_price: String,
    pub prev_sale_date: String,
}
```

- [ ] **Step 3: Update `new` to accept the three refs.**

```rust
pub fn new(
    look: Rc<emLook>,
    file_model: Rc<RefCell<emStocksFileModel>>,
    config: Rc<RefCell<emStocksConfig>>,
    list_box: Rc<RefCell<emStocksListBox>>,
) -> Self { ... }
```

- [ ] **Step 4: Update existing test fixtures** — search `rg -n 'emStocksItemPanel::new' crates/emstocks/` and cascade. Add a `#[cfg(test)] for_test()` helper.

- [ ] **Step 5: clippy + nextest clean.**

- [ ] **Step 6: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup C.1): emStocksItemPanel member refs"
```

### Task C.2 — `emStocksItemChart` member-reference fields + ctor cascade

**Files:**
- Modify: `crates/emstocks/src/emStocksItemChart.rs:38-...` (struct + `new`)

- [ ] **Step 1: Failing struct-shape test.**

- [ ] **Step 2: Edit struct.**

```rust
pub struct emStocksItemChart {
    /// C++ `emStocksListBox & ListBox` member reference.
    pub(crate) list_box: Rc<RefCell<emStocksListBox>>,
    /// C++ `emStocksConfig & Config` member reference.
    pub(crate) config: Rc<RefCell<emStocksConfig>>,
    pub(crate) subscribed_init: bool,
    // existing fields unchanged ...
}
```

- [ ] **Step 3: Update `new` + `UpdateData` callers** — `UpdateData(&mut self, stock_rec, config)` already takes `config` by reference; that signature is unchanged for callers, but the struct now owns its own `config` ref so internally we can prefer `self.config.borrow()`. Be explicit: keep the parameter to preserve call-site contract or refactor to a no-arg variant that reads from `self.config.borrow()` — pick whichever minimizes test churn. **Default: keep the parameter** (lower blast radius); add an internal `Cycle` method that reads from `self.config.borrow()` for the D-006 path.

- [ ] **Step 4: Cascade test fixtures** — `crates/emstocks/src/emStocksItemChart.rs` has many tests (`:1361-1617`). Add a `#[cfg(test)] for_test()` helper.

- [ ] **Step 5: clippy + nextest clean.**

- [ ] **Step 6: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup C.2): emStocksItemChart member refs"
```

### Task C.3 — `PanelBehavior::Cycle` scaffolds

**Files:**
- Modify: `crates/emstocks/src/emStocksItemPanel.rs`
- Modify: `crates/emstocks/src/emStocksItemChart.rs`

- [ ] **Step 1: Failing first-Cycle-init test for each.**

- [ ] **Step 2: Add `impl PanelBehavior` blocks with no-op `Cycle` flipping `subscribed_init`** — mirror Task A.3.

- [ ] **Step 3: Tests pass; clippy + nextest clean.**

- [ ] **Step 4: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup C.3): ItemPanel/ItemChart PanelBehavior::Cycle scaffold"
```

### Task C.4 — `emStocksListBox::CreateItemPanel` factory

**Files:**
- Modify: `crates/emstocks/src/emStocksListBox.rs`

- [ ] **Step 1: Failing test asserting that an attached ListBox creates an `emStocksItemPanel` for index N.**

- [ ] **Step 2: Wire the factory.** The Rust pattern is `item_panel_factory: Option<ItemPanelFactory>` on `emListBox` (`crates/emcore/src/emListBox.rs:323-324`). `emStocksListBox::new` should set this factory at construction:

```rust
pub fn new() -> Self {
    let mut me = Self { /* existing fields */ };
    // Factory closure constructing emStocksItemPanel with FileModel/Config refs.
    // Cannot construct here because we don't yet hold the refs — Phase 3 of
    // B-001 made FileModel/Config refs Optional on emStocksListBox; the
    // factory must capture them lazily via a Weak<> back-pointer or be
    // installed on `attach` (when refs are handed in). Match the existing
    // attach pattern:
    me
}

pub fn attach(&mut self,
    file_model: Rc<RefCell<emStocksFileModel>>,
    config: Rc<RefCell<emStocksConfig>>,
) {
    self.file_model = Some(file_model.clone());
    self.config = Some(config.clone());
    // Self-reference: the factory needs `*this` — use the inner emListBox's
    // attached ItemPanelFactory hook (which receives parent context per call).
    let look = self.look.clone();
    let fm = file_model.clone();
    let cfg = config.clone();
    // pseudo: assume self.inner.set_item_panel_factory(Box::new(move |...| { ... }))
    // Look up the actual factory hook signature in emListBox.rs:323 first.
}
```

(The factory closure receives the `emStocksListBox` reference indirectly via the parent context provided by the inner `emListBox` create-path. Locate the exact hook signature at `crates/emcore/src/emListBox.rs:323` + the call site at `:1050-1098` and match its API. Do not invent — if the hook cannot capture `Rc<RefCell<emStocksListBox>>` self-reference, that is a structural blocker and Phase C is incomplete; escalate.)

- [ ] **Step 3: Failing test passes.**

- [ ] **Step 4: clippy + nextest clean.**

- [ ] **Step 5: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup C.4): emStocksListBox.CreateItemPanel factory"
```

### Task C.5 — `emStocksItemPanel::AutoExpand` constructs `emStocksItemChart`

**Files:**
- Modify: `crates/emstocks/src/emStocksItemPanel.rs:245-258` (`AutoExpand` body)

- [ ] **Step 1: Failing test** — assert that calling `AutoExpand` populates `self.chart = Some(_)` mirroring C++ `emStocksItemPanel.cpp:549` (`Chart=new emStocksItemChart(l1,"Chart",ListBox,Config)`).

- [ ] **Step 2: Add chart construction in `AutoExpand`.**

```rust
pub fn AutoExpand<C: emcore::emEngineCtx::ConstructCtx>(&mut self, cc: &mut C) {
    if self.widgets.is_none() {
        self.widgets = Some(ItemWidgets::new(cc, self.look.clone()));
        // Mirrors C++ emStocksItemPanel.cpp:549 — Chart is a child of the
        // l1 layout (RUST_ONLY: layout flattening — the `l1` linear group
        // is not modeled separately; ItemChart is owned directly by
        // ItemPanel here. Document at the field declaration.).
        self.chart = Some(emStocksItemChart::new(
            self.list_box.clone(),
            self.config.clone(),
        ));
        self.update_controls_needed = true;
        self.subscribed_widgets = false;
    }
}
```

- [ ] **Step 3: Test passes; clippy + nextest clean.**

- [ ] **Step 4: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup C.5): ItemPanel.AutoExpand constructs ItemChart"
```

### Phase C verification gate

- [ ] `cargo check -p emstocks` / `cargo clippy -p emstocks --all-targets -- -D warnings` / `cargo nextest run -p emstocks` clean.
- [ ] `cargo xtask annotations` clean.
- [ ] No new `#[allow]` / `#[expect]`.

---

## Phase D — `emStocksItemPanel` + `emStocksItemChart` D-006 wiring

**Scope:** 29 rows in ItemPanel + 2 rows in ItemChart = 31 rows. References:
- B-001 design doc §"emStocksItemPanel (25 rows + 1 inner CategoryPanel)" (lines ~265-281)
- B-001 design doc §"emStocksItemChart (2 rows: -64, -65)" (lines ~276-285)

(The 29-vs-25 discrepancy: design doc §"Phase 4 Partial Merge" at line 477 says ItemPanel-29; design doc §"emStocksItemPanel (25 rows + 1 inner CategoryPanel)" at line 265 says 25 + the embedded CategoryPanel rows. Re-read both sections to reconcile the count before starting — this is a M-001-class verification step.)

### Task D.1 — ItemPanel G2 (Config) + G4 (SelectedDate) ctor-shape subscribes

These are the two C++ ctor-body subscribes at `emStocksItemPanel.cpp:65-66`. They are part of the 29-row count.

- [ ] **Step 1: Failing test.**

- [ ] **Step 2: First-Cycle init in `ItemPanel::Cycle` adds `Config.GetChangeSignal()` + `ListBox.GetSelectedDateSignal()` connects.** Mirror Task B.1 pattern.

- [ ] **Step 3: IsSignaled reaction branch triggers `update_controls_needed`** plus the OwningShares toggle logic that already lives in the existing `Cycle` body at `emStocksItemPanel.rs:259-...`.

- [ ] **Step 4: Tests pass; clippy + nextest clean.**

- [ ] **Step 5: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup D.1): ItemPanel ctor-shape subscribes (2 rows)"
```

### Task D.2 — ItemPanel widget-subscribe sweep (~27 rows)

`emStocksItemPanel`'s `ItemWidgets` block contains buttons / text fields / checkboxes mirroring the C++ widget pointer block at `emStocksItemPanel.cpp:30-65` (Name, Symbol, WKN, ISIN, Country/Sector/Collection sub-CategoryPanels, OwningShares, OwnShares, TradePrice, TradeDate, UpdateTradeDate, Price/PriceDate, FetchSharePrice, DesiredPrice, ExpectedDividend, InquiryDate, UpdateInquiryDate, Interest, ShowAllWebPages, Comment, plus 4×WebPage/ShowWebPage). For each, drop any underscore prefix and add the click/text subscribe.

- [ ] **Step 1: Subscribed-widgets latch** — reuse `subscribed_widgets` field added in Phase C.

- [ ] **Step 2: For each row in design doc §"emStocksItemPanel" table, add the connect.** Re-read the design doc table; do not paraphrase.

- [ ] **Step 3: IsSignaled reaction branches** mirror C++ Cycle — open the C++ and count.

- [ ] **Step 4: Tests pass; clippy + nextest clean.**

- [ ] **Step 5: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup D.2): ItemPanel widget subscribes (~27 rows)"
```

### Task D.3 — ItemChart 2-row subscribe (G2 + G4, rows -64 / -65)

- [ ] **Step 1: Failing test.**

- [ ] **Step 2: First-Cycle init in `ItemChart::Cycle` adds `Config.GetChangeSignal()` + `ListBox.GetSelectedDateSignal()` connects.** Mirrors C++ `emStocksItemChart.cpp:64-65`.

- [ ] **Step 3: IsSignaled reaction branch sets `data_up_to_date = false` and calls `UpdateData` per C++ `emStocksItemChart::Cycle`** (read the C++ Cycle body — it likely sets `DataUpToDate=false` then calls `InvalidatePainting`).

- [ ] **Step 4: Tests pass; clippy + nextest clean.**

- [ ] **Step 5: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup D.3): ItemChart ctor-shape subscribes (2 rows)"
```

### Phase D verification gate

- [ ] `cargo check -p emstocks` / `cargo clippy -p emstocks --all-targets -- -D warnings` / `cargo nextest run -p emstocks` clean.
- [ ] **Verify 31 rows wired:** `rg -nc 'ectx\.connect' crates/emstocks/src/emStocksItemPanel.rs crates/emstocks/src/emStocksItemChart.rs` matches design doc total.
- [ ] `cargo xtask annotations` clean.

---

## Phase E — `emStocksPricesFetcher` engine promotion + B-017 row 1 fetcher-side

**Scope:** Promote `emStocksPricesFetcher` from a passive struct to a panel-as-proxy-engine (or a free engine) so it can subscribe to its own `FileModel.GetChangeSignal()` + `GetFileStateSignal()` per `emStocksPricesFetcher.cpp:38-39`. This is the I-1 finding — B-017 row 1 is undertested without it.

**Files:**
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs`
- Modify: any caller that constructs the fetcher (`rg -n 'emStocksPricesFetcher::new' crates/emstocks/`)
- Modify: `crates/emstocks/src/emStocksFetchPricesDialog.rs` (consumer of the fetcher's ChangeSignal — already wired by B-017 row 1 consumer-side; verify the wiring still holds after engine promotion)

### Pre-checks

- [ ] **Verify C++ fetcher ctor at `emStocksPricesFetcher.cpp:25-40`** — confirms `AddWakeUpSignal(FileModel->GetChangeSignal())` and `AddWakeUpSignal(FileModel->GetFileStateSignal())`.
- [ ] **Verify C++ fetcher class hierarchy** — `class emStocksPricesFetcher : public emEngine, public emFileModelClient` (search the header at `~/Projects/eaglemode-0.96.4/include/emStocks/emStocksPricesFetcher.h`). The `emEngine` inheritance is what lets it subscribe — Rust must mirror engine identity.
- [ ] **Verify C++ Cycle body at `emStocksPricesFetcher.cpp:102-...`** — counts IsSignaled branches.
- [ ] **Verify Rust fetcher state.** `crates/emstocks/src/emStocksPricesFetcher.rs:19-...` shows the current passive struct. B-001 Phase 1 G3 added `change_signal: Cell<SignalId>` + `GetChangeSignal(&self, &mut impl SignalCtx)` accessor + 4 internal mutator fires at `cpp:70/134/264/272`. Phase E builds on this.

### Task E.1 — Engine identity for `emStocksPricesFetcher`

**Files:**
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs`

The engine identity question: the fetcher is owned by the `emStocksFetchPricesDialog`, which is itself a panel. Two options:
- **Option A (proxy-engine via dialog).** The dialog hosts the fetcher's engine — the dialog's `Cycle` invokes `self.fetcher.cycle(ectx, eid)`. Mirrors B-017's earlier emStocksFileModel-on-FilePanel proxy pattern (`crates/emstocks/src/emStocksFilePanel.rs:435-...`).
- **Option B (independent engine).** The fetcher allocates its own engine via `ectx.create_engine()` at first-Cycle and registers itself as a self-driving engine. Closer to C++ semantics but heavier refactor.

**Default: Option A (proxy-engine via dialog).** Rationale: matches the B-017 emStocksFileModel-on-FilePanel precedent at `:454`-`:469` and avoids a new engine-lifecycle pattern. Document the choice with `RUST_ONLY: language-forced — emEngine identity is tied to PanelBehavior in this codebase; fetcher proxies through the dialog's engine` at the field declaration.

- [ ] **Step 1: Failing test** — assert that after dialog's first Cycle, fetcher subscribes to FileModel signals (observe via `Rc::strong_count` on the FileModel or by firing `FileModel.Signal()` and asserting fetcher's Cycle ran). Use the shape from B-017 row 2 / row 3 tests as precedent.

- [ ] **Step 2: Add fields to `emStocksPricesFetcher`.**

```rust
pub struct emStocksPricesFetcher {
    // existing fields ...
    /// First-Cycle init latch.
    subscribed_init: bool,
    /// Captured upstream signals (cached from FileModel for IsSignaled gates).
    file_model_change_sig: Option<SignalId>,
    file_model_state_sig: Option<SignalId>,
    /// FileModel reference — was already in scope via existing `model` field;
    /// verify field name and reuse. Add Rc<RefCell<>> wrapping if the
    /// existing field is by-value.
}
```

- [ ] **Step 3: Add a `cycle(&mut self, ectx, eid)` method** that:
  1. On first call (`!subscribed_init`): allocates the FileModel signals via `self.model.borrow().GetChangeSignal()` + `self.model.borrow().GetFileStateSignal()`, calls `ectx.connect(sig, eid)` for each, captures into `Option<SignalId>` fields, sets `subscribed_init = true`.
  2. On every call: checks `IsSignaled` for each captured signal and runs the C++ Cycle reaction body (the switch on `FileModel->GetFileState()` at `cpp:104-...`).

- [ ] **Step 4: Verify `emStocksFileModel::GetFileStateSignal` exists.** If not, this is a B-001 Phase 1 prereq miss — escalate. Search: `rg -n 'GetFileStateSignal' crates/emstocks/src/emStocksFileModel.rs crates/emcore/src/emFileModel.rs`.

- [ ] **Step 5: Commit (red bar test compiles and asserts the new shape).**

```bash
git commit -m "feat(emstocks,B-001-followup E.1): PricesFetcher engine fields + cycle()"
```

### Task E.2 — Proxy-engine drive from `emStocksFetchPricesDialog`

**Files:**
- Modify: `crates/emstocks/src/emStocksFetchPricesDialog.rs` (`PanelBehavior::Cycle`)

- [ ] **Step 1: Failing test** — assert that firing `FileModel.Signal(ectx)` causes the fetcher's Cycle reaction to run via the dialog's Cycle (observable via the fetcher's `change_signal` re-firing or via UpdateControls being invoked).

- [ ] **Step 2: In dialog's `Cycle`, after the existing B-017 row-1 dialog consumer block, invoke the fetcher cycle:**

```rust
// B-017 row 1 fetcher-side: drive the fetcher's proxy engine through the
// dialog's engine. Mirrors C++ emStocksPricesFetcher inheriting emEngine
// (cpp:38-39 upstream subscribes); Rust language-forced proxy because the
// fetcher is owned by the dialog and emEngine identity is panel-bound.
self.fetcher.cycle(ectx, ectx.engine_id);
```

- [ ] **Step 3: Test passes; clippy + nextest clean.**

- [ ] **Step 4: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup E.2): dialog proxies fetcher engine cycle"
```

### Task E.3 — ectx threading audit at fetcher mutation sites

The 4 internal `Signal(ChangeSignal)` fires at `cpp:70/134/264/272` were already wired by B-001 Phase 1 G3 with `&mut impl SignalCtx` parameters. Verify that the new upstream `cycle()` method also fires the fetcher's own `change_signal` when state actually changed — without this, the dialog's UpdateControls won't run on FileModel transitions.

- [ ] **Step 1: Audit the C++ `Cycle` body** — what state changes does it report via `Signal(ChangeSignal)`? Read `emStocksPricesFetcher.cpp:102-300` and enumerate.

- [ ] **Step 2: Mirror in Rust `cycle`.** Each state-change branch that C++ calls `Signal(ChangeSignal)` from must call `self.Signal(ectx)` in Rust.

- [ ] **Step 3: Add a regression test** — fire `FileModel.Signal(ectx)`, drive a dialog `Cycle`, observe that the fetcher's `change_signal` fired (subscribe a probe and assert IsSignaled).

- [ ] **Step 4: Commit.**

```bash
git commit -m "feat(emstocks,B-001-followup E.3): fetcher upstream-driven Signal fires"
```

### Phase E verification gate

- [ ] `cargo check -p emstocks` / `cargo clippy -p emstocks --all-targets -- -D warnings` / `cargo nextest run -p emstocks` clean.
- [ ] `cargo xtask annotations` clean.
- [ ] **Verify B-017 row 1 silent-undertest is closed** — the new regression test from E.3 demonstrates that FileModel transitions drive dialog UpdateControls.

---

## Phase F — Final gate + reconciliation

### Task F.1 — Full workspace gate

- [ ] `cargo fmt --all -- --check` clean.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo-nextest ntr` green; record final count vs. baseline (B-001 Phase 5 final-gate count as floor; F-1 should be ≥ baseline + new tests added across Phases A-E).
- [ ] `cargo xtask annotations` clean.
- [ ] `cargo test --test golden -- --test-threads=1` green (no golden regressions).

### Task F.2 — No-`#[allow]` audit

- [ ] `rg -n '#\[allow|#\[expect' crates/emstocks/` — confirm no new entries beyond the pre-flight baseline. Allowed exceptions: `non_snake_case` on `emCore` module / `em`-prefixed types (none expected here), `clippy::too_many_arguments`. Anything else is a bug — fix at the cause, not at the suppression.

### Task F.3 — Work-order log entry

- [ ] **Append a new section** to `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/work-order.md` modeled on the existing `### 2026-05-01 — B-001 Phase 1-5 partial merge` entry:

```markdown
### 2026-05-?? — B-001-followup merged (structural pre-phase + 67 deferred rows + B-017 row 1 fetcher-side)

- **Phase A (ControlPanel structural) → merged at <SHA>.** PanelBehavior::Cycle, member-reference Rc<RefCell<>> fields, emStocksFilePanel.CreateControlPanel factory.
- **Phase B (ControlPanel D-006 wiring) → merged at <SHA>.** 37 rows (G1/G2/G4/G5/G6 + G7/G8 widgets).
- **Phase C (ItemPanel + ItemChart structural) → merged at <SHA>.** PanelBehavior::Cycle on both, member refs, emStocksListBox.CreateItemPanel factory, AutoExpand chart construction.
- **Phase D (ItemPanel + ItemChart D-006 wiring) → merged at <SHA>.** 31 rows.
- **Phase E (Fetcher engine promotion + B-017 row 1 fetcher-side) → merged at <SHA>.** Proxy-engine via emStocksFetchPricesDialog; upstream FileModel subscribes (cpp:38-39) wired; I-1 silent-undertest closed.
- **Aggregate:** B-001 67/67 deferred rows wired (4 + 67 = 71/71 final). B-017 row 1 fully merged (consumer-side previously, fetcher-side now). Test suite delta: +<N>.
- **B-001 Phase 4 Partial Merge section in design doc closed** — see Task F.4.
- **B-017 I-1 deferral closed** — see Task F.5.
```

### Task F.4 — Close B-001 design-doc deferral section

- [ ] **Edit** `docs/superpowers/specs/2026-04-27-B-001-no-wire-emstocks-design.md` §"Phase 4 Partial Merge — 2026-05-01" — append a "## Resolution — 2026-05-??" subsection citing the merge SHA(s) for Phases A-D and noting the 67 deferred rows are now wired.

### Task F.5 — Close B-017 design-doc I-1 deferral

- [ ] **Edit** `docs/superpowers/specs/2026-04-27-B-017-polling-no-acc-emstocks-design.md` §"Resolutions from Adversarial Review (2026-05-01)" item 4 (and §"Adversarial Review §I-1") — append a resolution note citing Phase E's merge SHA and the regression test from Task E.3.

### Task F.6 — Update CLAUDE.md memory or docs/ index if needed

- [ ] Check whether any global memory entry or `docs/` index references the "67 of 71 deferred" status — if so, update.
- [ ] If `~/.claude/projects/-home-alex-Projects-eaglemode-rs/memory/MEMORY.md` has a B-001 entry, update via the appropriate skill or surface as a follow-up.

### Task F.7 — Final commit + branch finishing

- [ ] If on a feature branch, follow `superpowers:finishing-a-development-branch` to merge / PR / cleanup.

---

## Cross-cutting hardening guards (re-stated for executor visibility)

For every task in every phase:

1. **TDD-first.** Write the failing test, run it to confirm failure, then write production code to flip it green. No exceptions.
2. **M-001 verify-C++-branch-structure.** Before writing any `IsSignaled` reaction body, open the matching C++ `Cycle` and count branches. Document any divergence as a design-doc errata before writing code.
3. **D-007 ectx-threading.** Mutator signatures take `&mut impl SignalCtx`. Never use `EngineCtx<'_>` for a mutator. `DropOnlySignalCtx` only at language-forced Drop sites — none expected in this plan.
4. **D-009 no polling intermediaries.** No `Cell<bool>` set in site A and drained by site B's `Cycle`. Thread `ectx` to A; fire synchronously.
5. **No `#[allow]` / `#[expect]`.** Fix the cause. Allowed exceptions enumerated in Pre-flight.
6. **File and Name Correspondence.** Method/type names match C++. `DIVERGED:` / `RUST_ONLY:` annotations carry forced-category tags.
7. **Pre-commit hook source of truth.** Every commit goes through `cargo fmt` + `cargo clippy -D warnings` + `cargo-nextest ntr` + `cargo xtask annotations`. Per-task selective gate is `-p emstocks` for speed; full nextest at Phase F.
8. **No silent rename.** If a C++ name truly cannot be preserved, mark it `DIVERGED:` with the C++ name + forced category at the Rust definition site.

---

## Coverage check

| Bucket source | Rows / items | Phase |
|---|---|---|
| B-001 Task 4.1 (ControlPanel-37) | 37 rows | Phase B |
| B-001 Task 4.2 (ItemPanel-29) | 29 rows | Phase D (D.1 + D.2) |
| B-001 Task 4.3 (ItemChart-2) | 2 rows | Phase D.3 |
| B-001 structural pre-phase (PanelBehavior + factories + Rc refs) | 3 panels | Phases A + C |
| B-017 row 1 fetcher-side (cpp:38-39) | 2 upstream subscribes | Phase E |
| Reconciliation (work-order, design docs, audits) | — | Phase F |

**Total deferred-row coverage: 67 + 2 = 69 row-level wirings + 3 structural panels + 2 factory hooks.** All accounted for.
