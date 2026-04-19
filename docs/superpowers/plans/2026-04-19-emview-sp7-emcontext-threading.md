# SP7 — emContext Threading — Implementation Plan

**Spec:** `docs/superpowers/specs/2026-04-19-emview-sp7-emcontext-threading-design.md`
**Date:** 2026-04-19
**Branch:** `sp7-emcontext-threading` (created off current `main`).
**Estimated scope:** 6 phased commits. Phase 2 is the large mechanical call-site sweep (~180 test sites + ~5 production); other phases are small.

## Anti-patterns this plan hardens against

- **Silent observable drift.** Every phase is gated by nextest + golden at the same baseline (237/6). No phase ships if pixel or test count regresses outside documented expectations.
- **Drift from C++.** Exactly one *new* DIVERGED block is permitted (`emView::Context` composition). Subagents **must not** introduce other DIVERGED markers. If a subagent finds a second forced divergence, it must stop and ask before adding.
- **Split-brain state.** Phase 2 mutates `emView::new`'s signature; all call sites update in the **same commit**. Do not leave the tree half-migrated.
- **Re-introducing the SP3 bridge.** The `core_config: Rc<RefCell<emCoreConfig>>` parameter on `emView::new` is deleted in Phase 2 — no caller keeps constructing a local config.
- **Test-only helpers leaking to prod.** `emContext::NewRoot()` stays test-only in the view subsystem; production paths use `app.context` plumbed through the call chain.
- **Scope creep.** SP7 does **not** install a real clipboard; does **not** nest per-window contexts under the root; does **not** touch panel construction except the one `emSubViewPanel::new` signature change.

## Phase gates

Each phase closes with a gate. Gate = `cargo check && cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1`. If a gate fails, **fix the cause**; do not skip. No `--no-verify`.

Golden baseline: 237 passed / 6 failed (pre-existing: `composition_tktest_{1x,2x}`, `notice_window_resize`, `testpanel_{expanded,root}`, `widget_file_selection_box`). Any new failure is a regression.

Nextest baseline at branch start: **2443/2443**. Each phase records the post-phase count.

---

## Phase 0 — Branch + audit (read-only)

**Task 0.1.** Create branch:

```bash
git checkout -b sp7-emcontext-threading
```

**Task 0.2.** Capture pre-state counts for the sweep in Phase 2:

```bash
rg -c 'emView::new\b' crates/ --glob '*.rs'
rg -c 'emWindow::create\b|new_popup_pending\b' crates/ --glob '*.rs'
rg -n 'emCoreConfig::default\(\)|Rc::new\(RefCell::new\(emCoreConfig' crates/ --glob '*.rs'
```

Record expected hits in the phase-gate notes so Phase 2's "all sites touched" is verifiable.

**Task 0.3.** Grep-confirm there is exactly one production `emView::new` outside `emWindow.rs`/`emSubViewPanel.rs` (the `emViewInputFilter.rs:2537` test site) — expected, but verify.

**Gate 0:** counts captured; no code changes. Proceed.

---

## Phase 1 — emContext::GetRootContext + emCoreConfig::Acquire port

**Task 1.1.** In `crates/emcore/src/emContext.rs`, add:

```rust
/// Port of C++ `emContext::GetRootContext()` — walks up the parent
/// chain to the root. A root returns a clone of its own Rc.
pub fn GetRootContext(self: &Rc<emContext>) -> Rc<emContext> {
    match self.GetParentContext() {
        Some(parent) => parent.GetRootContext(),
        None => Rc::clone(self),
    }
}
```

Placement: immediately below `GetParentContext`. Keep name casing per File-and-Name Correspondence.

**Task 1.2.** In `crates/emcore/src/emCoreConfig.rs`, add:

```rust
impl emCoreConfig {
    /// Port of C++ `emCoreConfig::Acquire(emRootContext &)`. Returns the
    /// shared singleton, registered on the root of `ctx`'s chain under
    /// the empty name `""`. Subsequent calls from any context in the
    /// same tree return the same `Rc`.
    pub fn Acquire(ctx: &Rc<crate::emContext::emContext>) -> Rc<RefCell<Self>> {
        let root = ctx.GetRootContext();
        root.acquire::<emCoreConfig>("", emCoreConfig::default)
    }
}
```

Imports as needed (`std::rc::Rc`, `std::cell::RefCell`).

**Task 1.3.** Add two unit tests in `emContext.rs` (or `emCoreConfig.rs`, whichever is natural):

- `GetRootContext_returns_root_from_deep_child`: build root → child → grandchild, assert `grandchild.GetRootContext()` is `Rc::ptr_eq` to the root Rc.
- `emCoreConfig_is_singleton_across_sibling_contexts`: build root, two children; `Acquire` from each child; assert `Rc::ptr_eq` on the returned configs.

**Gate 1:** `cargo check && cargo clippy -D warnings && cargo-nextest ntr`. Expect 2445/2445.

**Commit:** `sp7(1/N): emContext::GetRootContext + emCoreConfig::Acquire port`.

---

## Phase 2 — emView::new takes parent context; mechanical call-site sweep

**Task 2.1.** In `crates/emcore/src/emView.rs`:

- Add field `pub(crate) Context: Rc<crate::emContext::emContext>` to `emView`. Place next to `CoreConfig`. Add a `DIVERGED:` block on this field:

  ```
  // DIVERGED: C++ `class emView : public emContext` — Rust has no
  // inheritance; store the context by composition. `GetContext` and
  // `GetRootContext` accessors below delegate to this field, giving
  // callers the same observable access the C++ inheritance grants.
  ```

- Change `emView::new`'s signature from
  `pub fn new(root: PanelId, viewport_width: f64, viewport_height: f64, core_config: Rc<RefCell<emCoreConfig>>) -> Self`
  to
  `pub fn new(parent_context: Rc<crate::emContext::emContext>, root: PanelId, viewport_width: f64, viewport_height: f64) -> Self`.

- In the body, replace the `core_config` parameter use with:

  ```rust
  let ctx = crate::emContext::emContext::NewChild(&parent_context);
  let core_config = crate::emCoreConfig::emCoreConfig::Acquire(&ctx);
  ```

- Initialize `Context: ctx` and `CoreConfig: core_config` in the `Self { ... }` literal.

- Add accessors:

  ```rust
  pub fn GetContext(&self) -> &Rc<crate::emContext::emContext> { &self.Context }
  pub fn GetRootContext(&self) -> Rc<crate::emContext::emContext> {
      self.Context.GetRootContext()
  }
  ```

  Place next to the other view-level accessors.

- Remove the SP3 bridge comment on `CoreConfig` ("SP7 will source this via ..."). Replace with a one-line note citing `emView.cpp:35`.

**Task 2.2.** Sweep every `emView::new(` call site in the tree. Two classes:

- **Production (known set):** `crates/emcore/src/emWindow.rs:211` and `:336`, `crates/emcore/src/emSubViewPanel.rs:76`. These take `parent_context` from their own new arguments (Phase 3/4) — for now, temporarily pass a placeholder that **will be replaced in the next phase**. To avoid a broken intermediate, Phase 2 also performs Phases 3 and 4's signature changes on `emWindow::create` / `emWindow::new_popup_pending` / `emSubViewPanel::new`, then threads the parent context through.

  **Rationale:** the call graph is `emWindow::create → emView::new` and `emSubViewPanel::new → emView::new`. Breaking mid-graph leaves the tree non-compiling; the only safe atomic unit is "all three functions change signature together." Treat Phases 2–4 as **one commit**.

- **Tests (mechanical):** every `emView::new(root, w, h, core_config)` becomes `emView::new(emContext::NewRoot(), root, w, h)`. Wherever the test currently does `let core_config = Rc::new(RefCell::new(emCoreConfig::default())); ... emView::new(..., core_config)`, delete the `core_config` local and inline the `NewRoot()` as the first argument.

  Tests that explicitly want to share a config across views: replace the shared `core_config` with a shared `ctx = emContext::NewRoot()` and pass that same Rc into each `emView::new` — `Acquire` will return the same singleton for sibling views of the same root.

**Task 2.3.** Update `emWindow::create`:

- Signature: add `parent_context: Rc<emContext>` as a new first parameter (after `event_loop`, `gpu`). Choose position that keeps call-site diffs minimal.
- Delete the `let core_config = Rc::new(RefCell::new(emCoreConfig::default()));` line; call `emView::new(parent_context, root_panel, w as f64, h as f64)`.

**Task 2.4.** Update `emWindow::new_popup_pending`:

- Signature: add `parent_context: Rc<emContext>` as a new first parameter.
- Delete the local `core_config`; thread `parent_context` to `emView::new`.

**Task 2.5.** Update `emSubViewPanel::new`:

- Signature: `pub fn new(parent_context: Rc<emContext>) -> Self`.
- Thread to the inner `emView::new` call. The current emSubViewPanel call chain will need the parent context supplied at panel-construction time — check callers and forward.

**Task 2.6.** Sweep `emWindow::create` / `emWindow::new_popup_pending` / `emSubViewPanel::new` call sites:

- `emMainWindow` / `emVirtualCosmos` / `emMainContentPanel` — already hold `Rc<emContext>` (as `app.context`); pass it.
- Tests constructing windows: pass `emContext::NewRoot()`.

**Task 2.7.** `emView::update` cached field probe / test-support helpers: audit for stale references to the old 4-arg `emView::new`. Update `emView::new_for_test` (if present) to match.

**Task 2.8.** Audit DIVERGED markers before commit: exactly one new (`emView::Context` field). Any other DIVERGED the subagent considers must be flagged up, not silently added.

**Gate 2:** `cargo check` (compiles), `cargo clippy -- -D warnings`, `cargo-nextest ntr` (expect ≥ 2445 — may gain 1–2 from shared-singleton test added in this phase if natural), `cargo test --test golden -- --test-threads=1` (237/6 baseline, no new failures).

**Commit:** `sp7(2/N): thread emContext through emView/emWindow/emSubViewPanel` (single commit; covers tasks 2.1–2.8).

---

## Phase 3 — emSubViewPanel cleanup; SP3 bridge-comment removal

**Task 3.1.** In `crates/emcore/src/emSubViewPanel.rs`, remove the SP3-era `DIVERGED:` comment about the default-constructed `emCoreConfig`. The threading now goes through the parent context, so there is no divergence to mark.

**Task 3.2.** Review every `core_config` mention in `crates/emcore/src/` — any remaining `Rc::new(RefCell::new(emCoreConfig::default()))` in the view subsystem (outside unit tests that build standalone configs on purpose) is a leftover and should be deleted.

**Task 3.3.** Confirm `emView::new_for_test` (if it still exists per SP1's `test-support` gating) either matches the new signature or is deleted if unused.

**Task 3.4.** Add one behavioural test to `crates/eaglemode/tests/behavioral/core_config.rs` (file exists per the grep):

```rust
#[test]
fn sp7_sibling_views_share_core_config_singleton() {
    // Two views built with the same parent context see the same
    // emCoreConfig Rc — per C++ Acquire semantics.
    let root = emContext::NewRoot();
    let tree1 = PanelTree::new(); let p1 = /* ... */;
    let tree2 = PanelTree::new(); let p2 = /* ... */;
    let v1 = emView::new(Rc::clone(&root), p1, 100.0, 100.0);
    let v2 = emView::new(Rc::clone(&root), p2, 100.0, 100.0);
    assert!(Rc::ptr_eq(&v1.CoreConfig, &v2.CoreConfig));
}
```

(Concrete test harness adapts to existing helpers; assertion shape as above.)

**Gate 3:** full gate as Phase 2. Expect 2446/2446 (one new behavioural test).

**Commit:** `sp7(3/N): SP3 bridge cleanup + CoreConfig singleton behavioural test`.

---

## Phase 4 — Closeout-doc update

**Task 4.1.** Edit `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md`:

- §1 status table: drop SP7 from "Known Rust-port incompletenesses remaining". Update the "Tests" and "Golden" rows to reflect post-SP7 counts.
- §6 markers table: `DIVERGED:` count bumps by 1 (the `emView::Context` composition block). Note its justification.
- §8.0 sub-project table: SP7 row flips to **Complete YYYY-MM-DD** with commit SHAs and artifacts (spec + plan).
- §8.1 item 15: flip to closed with SHAs; keep the original text below a `**CLOSED 2026-04-19 by SP7**` line, per the doc's existing convention.
- Add a new residual if appropriate: real `emClipboard` implementation. Wiring point (`emContext::set_clipboard`) exists; no backend installed. Not blocking. Mark as separate sub-project candidate.

**Gate 4:** doc-only commit; pre-commit hook skips clippy/tests.

**Commit:** `sp7(4/N): closeout — SP7 complete`.

---

## Phase 5 — Merge

**Task 5.1.** Run full gate on branch tip one last time.
**Task 5.2.** Merge to main (no-ff) to preserve branch topology consistent with prior sub-projects.

```bash
git checkout main
git merge --no-ff sp7-emcontext-threading
```

**Gate 5:** main post-merge passes full gate.

**Commit:** auto-generated merge commit.

---

## Exit criteria

- [ ] Branch `sp7-emcontext-threading` merged to `main`.
- [ ] Nextest: **≥ 2445/2445** (2443 baseline + 2 Phase-1 units + 1 Phase-3 behavioural = 2446 expected).
- [ ] Golden: **237/6** (no new failures; same six pre-existing).
- [ ] Smoke: `timeout 20 cargo run --release --bin eaglemode` exits 124 or 143.
- [ ] Exactly one new `DIVERGED:` block (`emView::Context` composition).
- [ ] `emView::new`, `emWindow::create`, `emWindow::new_popup_pending`, `emSubViewPanel::new` all take parent context.
- [ ] No `emCoreConfig::default()` calls remain in the view-subsystem production paths.
- [ ] `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md` updated.

## Rollback

If Phase 2 (the load-bearing mechanical sweep) fails golden and cannot be fixed quickly: revert the branch tip to Phase 1, reassess. Phase 1 is a pure addition and safe to keep; everything downstream is clean revert.
