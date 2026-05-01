# CreateControlPanel `self_is_active` Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `self_is_active: bool` to `PanelBehavior::CreateControlPanel` and use it to restore the missing `IsActive()` guard in `emDirPanel` and `emDirEntryPanel`, fixing the `invalid SlotMap key used` panic on zoom-out.

**Architecture:** Single-task change across 6 locations — trait default, two call sites in `emPanelTree`, two real implementations in `emfileman`, and two test stubs. No walk logic changes; the guard is applied inside each behavior, matching C++ structure.

**Tech Stack:** Rust, `slotmap`, `cargo-nextest`

---

### Task 1: Add `self_is_active` parameter and restore `IsActive()` guards

**Files:**
- Modify: `crates/emcore/src/emPanel.rs:399`
- Modify: `crates/emcore/src/emPanelTree.rs:2155` (non-cross-tree call site)
- Modify: `crates/emcore/src/emPanelTree.rs:2207` (cross-tree call site in `create_control_panel_in`)
- Modify: `crates/emcore/src/emPanelTree.rs:3138` (test stub `ControlCreator`)
- Modify: `crates/emcore/src/emPanelTree.rs:3170` (test stub `SchedReachProbe`)
- Modify: `crates/emfileman/src/emDirPanel.rs:530`
- Modify: `crates/emfileman/src/emDirEntryPanel.rs:1201`

- [ ] **Step 1: Update the trait default in `emPanel.rs`**

In `crates/emcore/src/emPanel.rs`, replace the `CreateControlPanel` default at line 399:

```rust
fn CreateControlPanel(&mut self, _parent_ctx: &mut PanelCtx, _name: &str, _self_is_active: bool) -> Option<PanelId> {
    None
}
```

- [ ] **Step 2: Update the non-cross-tree call site in `PanelTree::CreateControlPanel`**

In `crates/emcore/src/emPanelTree.rs`, inside `PanelTree::CreateControlPanel` (line ~2155), replace:

```rust
let result = behavior.CreateControlPanel(&mut ctx, name);
```

with:

```rust
let self_is_active = self.panels.get(cur).map(|p| p.is_active).unwrap_or(false);
let result = behavior.CreateControlPanel(&mut ctx, name, self_is_active);
```

- [ ] **Step 3: Update the cross-tree call site in `create_control_panel_in`**

In `crates/emcore/src/emPanelTree.rs`, inside `create_control_panel_in` (line ~2207), replace:

```rust
let result = behavior.CreateControlPanel(&mut ctx, name);
```

with:

```rust
let self_is_active = self.panels.get(cur).map(|p| p.is_active).unwrap_or(false);
let result = behavior.CreateControlPanel(&mut ctx, name, self_is_active);
```

- [ ] **Step 4: Update the two test stubs in `emPanelTree.rs`**

`ControlCreator` (line ~3138):
```rust
fn CreateControlPanel(&mut self, ctx: &mut PanelCtx, name: &str, _self_is_active: bool) -> Option<PanelId> {
    Some(ctx.create_child(name))
}
```

`SchedReachProbe` (line ~3170):
```rust
fn CreateControlPanel(&mut self, ctx: &mut PanelCtx, name: &str, _self_is_active: bool) -> Option<PanelId> {
    self.0.set(ctx.as_sched_ctx().is_some());
    Some(ctx.create_child(name))
}
```

- [ ] **Step 5: Verify `cargo check` passes**

```bash
cargo check
```

Expected: no errors. This catches all missing call-site or impl updates before touching the guards.

- [ ] **Step 6: Add `IsActive()` guard to `emDirPanel::CreateControlPanel`**

In `crates/emfileman/src/emDirPanel.rs`, replace the signature and add the guard at line 530:

```rust
fn CreateControlPanel(&mut self, parent_ctx: &mut PanelCtx, name: &str, self_is_active: bool) -> Option<PanelId> {
    if !self_is_active { return None; }  // C++: if (IsActive())
    let panel = {
        let mut sched = parent_ctx
            .as_sched_ctx()
            .expect("CreateControlPanel requires scheduler-reach PanelCtx");
        crate::emFileManControlPanel::emFileManControlPanel::new(
            &mut sched,
            Rc::clone(&self.ctx),
        )
        .with_dir_path(&self.path)
    };
    Some(parent_ctx.create_child_with(name, Box::new(panel)))
}
```

- [ ] **Step 7: Add `IsActive()` guard to `emDirEntryPanel::CreateControlPanel`**

In `crates/emfileman/src/emDirEntryPanel.rs`, replace the signature and add the guard at line 1201:

```rust
fn CreateControlPanel(&mut self, parent_ctx: &mut PanelCtx, name: &str, self_is_active: bool) -> Option<PanelId> {
    if !self_is_active { return None; }  // C++: if (IsActive())
    let parent_dir = std::path::Path::new(self.dir_entry.GetPath())
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let mut panel = {
        let mut sched = parent_ctx
            .as_sched_ctx()
            .expect("CreateControlPanel requires scheduler-reach PanelCtx");
        crate::emFileManControlPanel::emFileManControlPanel::new(
            &mut sched,
            Rc::clone(&self.ctx),
        )
    };
    if !parent_dir.is_empty() {
        panel = panel.with_dir_path(parent_dir);
    }
    Some(parent_ctx.create_child_with(name, Box::new(panel)))
}
```

- [ ] **Step 8: Run tests**

```bash
cargo-nextest ntr
```

Expected: all passing. The two `create_control_panel` tests in `emPanelTree.rs` (both pass `self_is_active` from a tree where `is_active` is `false` by default for non-view-activated panels) should still pass because `ControlCreator` and `SchedReachProbe` ignore the parameter.

- [ ] **Step 9: Commit**

```bash
git add crates/emcore/src/emPanel.rs \
        crates/emcore/src/emPanelTree.rs \
        crates/emfileman/src/emDirPanel.rs \
        crates/emfileman/src/emDirEntryPanel.rs
git commit -m "fix(emDirPanel,emDirEntryPanel): restore IsActive() guard in CreateControlPanel

Missing guard allowed non-active ancestor panels to create a CCP in the
content sub-tree; the resulting PanelId was then passed to ctrl sub-tree's
remove(), panicking with 'invalid SlotMap key used'.

Adds self_is_active: bool to PanelBehavior::CreateControlPanel so the
framework can pass is_active from PanelData without a second source of
truth. Matches C++: emDirPanel.cpp:253, emDirEntryPanel.cpp:475."
```
