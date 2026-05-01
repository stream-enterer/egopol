# C-24 / I-11 Item Panel Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `CustomItemBehavior` (and `DefaultItemPanelBehavior`) to the owning `emListBox` so mouse/keyboard input dispatches selection (C-24) and item-text changes propagate to child panel behaviors (I-11).

**Architecture:** Two routing directions. C-24 (child → parent): item behavior's `Input` calls `ctx.with_parent_behavior`, which temporarily takes the parent behavior out of the tree and invokes `dispatch_item_input` on it; `ListBoxPanel` overrides this to call `emListBox::process_item_input`. I-11 (parent → child): `emListBox::SetItemText` looks up `item.child_panel_id` and calls `on_item_text_changed` on the child behavior via `ctx.tree.with_behavior_dyn`. Both routing primitives live in `emPanelTree`/`emEngineCtx`.

**Tech Stack:** Rust, emcore crate (`crates/emcore/`), emtest crate (`crates/emtest/`), `cargo-nextest ntr`, `cargo clippy -- -D warnings`.

---

## File Map

| File | Change |
|------|--------|
| `crates/emcore/src/emPanelTree.rs` | Add `with_behavior_dyn`, focus-request queue + drain |
| `crates/emcore/src/emPanel.rs` | Add `ItemInputResult`, `dispatch_item_input`, `on_item_text_changed` to `PanelBehavior` |
| `crates/emcore/src/emEngineCtx.rs` | Add `PanelCtx::with_parent_behavior`, `PanelCtx::request_focus` |
| `crates/emcore/src/emListBox.rs` | `Item::child_panel_id`; `create_item_children` stores PanelId; `process_item_input` extracted; `SetItemText` ctx arg + child notify; `DefaultItemPanelBehavior` gains `item_index`, `Input`, `on_item_text_changed`; unit tests |
| `crates/emtest/src/emTestPanel.rs` | `ListBoxPanel::dispatch_item_input`; `CustomItemBehavior` gains `item_index`, `Input`, `on_item_text_changed` |
| `crates/emcore/src/emView.rs` | Drain focus-request queue and call `set_focus` |

---

## Task 1: `PanelTree` infrastructure primitives

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs`

### 1.1 Add `with_behavior_dyn`

Place immediately after `with_behavior_as` (around line 1728).

- [ ] Add the method:

```rust
/// Extract a panel's behavior, call a closure on `&mut dyn PanelBehavior`,
/// then put it back. No downcast required.
pub fn with_behavior_dyn<R>(
    &mut self,
    id: PanelId,
    f: impl FnOnce(&mut dyn PanelBehavior) -> R,
) -> Option<R> {
    let mut behavior = self.take_behavior(id)?;
    let result = f(behavior.as_mut());
    if self.panels.contains_key(id) {
        self.put_behavior(id, behavior);
    }
    Some(result)
}
```

### 1.2 Add focus-request queue to `PanelTree`

`PanelTree` already has `navigation_requests: Vec<PanelId>` (line ~313). Mirror the pattern.

- [ ] Add field to `PanelTree` struct (near `navigation_requests`):

```rust
focus_requests: Vec<PanelId>,
```

- [ ] Initialize in `PanelTree::new` (near `navigation_requests: Vec::new()`):

```rust
focus_requests: Vec::new(),
```

- [ ] Add methods after `drain_navigation_requests`:

```rust
pub(crate) fn request_focus(&mut self, id: PanelId) {
    self.focus_requests.push(id);
}

pub(crate) fn drain_focus_requests(&mut self) -> Vec<PanelId> {
    std::mem::take(&mut self.focus_requests)
}
```

- [ ] Compile check:

```bash
cargo check -p emcore 2>&1 | head -30
```

Expected: no errors.

- [ ] Commit:

```bash
git add crates/emcore/src/emPanelTree.rs
git commit -m "feat(emPanelTree): add with_behavior_dyn and focus-request queue"
```

---

## Task 2: `PanelBehavior` + `PanelCtx` API layer

**Files:**
- Modify: `crates/emcore/src/emPanel.rs`
- Modify: `crates/emcore/src/emEngineCtx.rs`

### 2.1 Add `ItemInputResult` and new `PanelBehavior` methods

- [ ] In `emPanel.rs`, add `ItemInputResult` before the `PanelBehavior` trait. Place it near the top of the file after existing imports:

```rust
/// Return value of `PanelBehavior::dispatch_item_input`.
/// Tells the calling item behavior whether the event was consumed and
/// whether it should request keyboard focus.
#[derive(Default)]
pub struct ItemInputResult {
    pub consumed: bool,
    /// True on `MouseLeft` press — the item should request focus (C++ `panel->Focus()`).
    pub focus_self: bool,
}
```

- [ ] In the `PanelBehavior` trait, add two default-no-op methods. Place them near other input-related methods (after `Input`):

```rust
/// Called by child item panel behaviors to dispatch selection/trigger logic
/// to the owning listbox. Default no-op; override in container behaviors
/// (e.g. `ListBoxPanel`).
fn dispatch_item_input(
    &mut self,
    _item_index: usize,
    _event: &emInputEvent,
    _state: &PanelState,
    _ctx: &mut PanelCtx,
) -> ItemInputResult {
    ItemInputResult::default()
}

/// Called by the owning listbox when an item's display text changes.
/// Default no-op; override in item panel behaviors.
fn on_item_text_changed(&mut self, _text: &str) {}
```

- [ ] Re-export `ItemInputResult` from `emcore`'s public API. In `crates/emcore/src/lib.rs`, add after the existing `pub use emVarModel` line (around line 125):

```rust
pub use emPanel::ItemInputResult;
```

- [ ] Compile check:

```bash
cargo check -p emcore 2>&1 | head -30
```

Expected: no errors.

### 2.2 Add `PanelCtx::with_parent_behavior` and `PanelCtx::request_focus`

- [ ] In `emEngineCtx.rs`, add both methods to `impl PanelCtx`. Place near `wake_up_panel` (around line 680):

```rust
/// Take the parent panel's behavior out of the tree, call `f` with both
/// the parent behavior and this `PanelCtx` (giving `f` full context
/// access), then put the parent back. Returns `None` if this panel has
/// no parent or the parent has no behavior.
///
/// The parent is temporarily absent from the tree during `f`, so
/// `ctx.tree` is freely accessible inside `f`.
pub fn with_parent_behavior<R>(
    &mut self,
    f: impl FnOnce(&mut dyn PanelBehavior, &mut PanelCtx) -> R,
) -> Option<R> {
    let parent_id = self.tree.GetParentContext(self.id)?;
    let mut behavior = self.tree.take_behavior(parent_id)?;
    let result = f(behavior.as_mut(), self);
    if self.tree.panels.contains_key(parent_id) {
        self.tree.put_behavior(parent_id, behavior);
    }
    Some(result)
}

/// Request that this panel receive keyboard focus.
/// Queued in `PanelTree`; drained by `emView` each frame.
/// Matches C++ `emPanel::Focus()`.
pub fn request_focus(&mut self) {
    let id = self.id;
    self.tree.request_focus(id);
}
```

- [ ] Compile check:

```bash
cargo check -p emcore 2>&1 | head -30
```

Expected: no errors.

- [ ] Commit:

```bash
git add crates/emcore/src/emPanel.rs crates/emcore/src/emEngineCtx.rs
git commit -m "feat(emPanel,emEngineCtx): ItemInputResult, dispatch_item_input, on_item_text_changed, with_parent_behavior, request_focus"
```

---

## Task 3: `emListBox` core changes + C-24 unit tests

**Files:**
- Modify: `crates/emcore/src/emListBox.rs`

### 3.1 Add `child_panel_id` to `Item`

The `Item` struct is around line 243. It currently has `name`, `text`, `data`, `selected`, `interface`.

- [ ] Add the field:

```rust
/// PanelId of the child panel created by `item_behavior_factory`.
/// Set by `create_item_children`; `None` when using `item_panel_factory`
/// or before `AutoExpand`.
child_panel_id: Option<PanelId>,
```

- [ ] Initialize in `AddItem` and `InsertItem` wherever `Item { ... }` is constructed. Add `child_panel_id: None` to each struct literal.

- [ ] Compile check:

```bash
cargo check -p emcore 2>&1 | head -30
```

### 3.2 `create_item_children` stores child `PanelId`

`create_item_children` (around line 1080) currently calls `ctx.create_child(&item.name)` and then `ctx.tree.set_behavior(child, behavior)`. It does not store the child PanelId.

- [ ] Change the loop body. The current loop iterates `for (i, item) in self.items.iter().enumerate()`. After the loop body creates the child and sets the behavior, store the PanelId. Since `items` is borrowed via `iter()`, use an index loop instead to allow mutation:

```rust
for i in 0..self.items.len() {
    let child = ctx.create_child(&self.items[i].name);
    let behavior: Box<dyn PanelBehavior> =
        if let Some(factory) = &self.item_behavior_factory {
            factory(
                i,
                &self.items[i].text,
                self.items[i].selected,
                look.clone(),
                sel_mode,
                enabled,
            )
        } else {
            Box::new(DefaultItemPanelBehavior::new(
                i,
                self.items[i].text.clone(),
                self.items[i].selected,
                look.clone(),
                sel_mode,
                enabled,
            ))
        };
    ctx.tree.set_behavior(child, behavior);
    self.items[i].child_panel_id = Some(child);
}
```

Note: `DefaultItemPanelBehavior::new` gains an `index: usize` first argument (added in step 3.3).

### 3.3 Add `item_index` to `DefaultItemPanelBehavior` and its two new methods

`DefaultItemPanelBehavior` struct (around line 130) has `text`, `selected`, `look`, `selection_mode`, `enabled`.

- [ ] Add the field:

```rust
item_index: usize,
```

- [ ] Update `DefaultItemPanelBehavior::new` to accept and store `index`:

```rust
pub fn new(
    index: usize,
    text: String,
    selected: bool,
    look: Rc<emLook>,
    selection_mode: SelectionMode,
    enabled: bool,
) -> Self {
    Self {
        item_index: index,
        text,
        selected,
        look,
        selection_mode,
        enabled,
    }
}
```

- [ ] Update the two call sites for `DefaultItemPanelBehavior::new` (one in `create_item_children` above; search for any others with `grep -n "DefaultItemPanelBehavior::new"` in emListBox.rs) to pass the index as first argument.

- [ ] Add `Input` and `on_item_text_changed` to `impl PanelBehavior for DefaultItemPanelBehavior`:

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    state: &PanelState,
    _is: &emInputState,
    ctx: &mut PanelCtx,
) -> bool {
    let idx = self.item_index;
    let result = ctx
        .with_parent_behavior(|parent, ctx| parent.dispatch_item_input(idx, event, state, ctx))
        .unwrap_or_default();
    if result.focus_self {
        ctx.request_focus();
    }
    result.consumed
}

fn on_item_text_changed(&mut self, text: &str) {
    self.text = text.to_string();
}
```

### 3.4 Extract `process_item_input` from `input_impl`

`input_impl` (around line 1344) handles `MouseLeft`, `Space`, and `Enter` in a match on `event.key`. These three arms currently call `self.select_by_input(...)` directly.

- [ ] Add `ItemInputResult` to the imports at the top of `emListBox.rs` (or use the full path `crate::emPanel::ItemInputResult`).

- [ ] Add the new public method before `input_impl`:

```rust
/// Dispatch selection/trigger for an item panel's input event.
/// Called by item panel behaviors via `dispatch_item_input`.
/// Port of C++ `emListBox::ProcessItemInput`.
pub fn process_item_input(
    &mut self,
    item_index: usize,
    event: &emInputEvent,
    state: &PanelState,
    ctx: &mut PanelCtx,
) -> ItemInputResult {
    if !self.enabled {
        return ItemInputResult::default();
    }
    let mut result = ItemInputResult::default();
    match event.key {
        InputKey::MouseLeft if event.variant == InputVariant::Press => {
            if !event.alt && !event.meta {
                let trigger = event.is_repeat();
                self.select_by_input(item_index, event.shift, event.ctrl, trigger);
                result.consumed = true;
                result.focus_self = true;
            }
        }
        InputKey::Space if event.variant == InputVariant::Press => {
            if !event.alt && !event.meta {
                self.select_by_input(item_index, event.shift, event.ctrl, false);
                result.consumed = true;
            }
        }
        InputKey::Enter if event.variant == InputVariant::Press => {
            if !event.alt && !event.meta {
                self.select_by_input(item_index, event.shift, event.ctrl, true);
                result.consumed = true;
            }
        }
        _ => {}
    }
    if result.consumed {
        self.drain_pending_fires(ctx);
    }
    result
}
```

- [ ] In `input_impl`, replace the `MouseLeft`, `Space`, and `Enter` arms. These currently construct the click index from mouse coordinates and call `select_by_input` themselves. They are on the listbox widget itself (not an item child), so keep the coordinate math but delegate to `process_item_input` for the selection and drain:

For `MouseLeft` (around line 1397):
```rust
InputKey::MouseLeft if event.variant == InputVariant::Press => {
    if !self.hit_test(event.mouse_x, event.mouse_y, state.pixel_tallness) {
        return false;
    }
    let tallness = if self.last_w > 0.0 {
        self.last_h / self.last_w * state.pixel_tallness
    } else {
        1.0
    };
    let cr = self.border.GetContentRectUnobscured(1.0, tallness, &self.look);
    let row_h = if self.items.is_empty() {
        cr.h
    } else {
        cr.h / self.items.len() as f64
    };
    let rel_y = event.mouse_y - cr.y + self.scroll_y;
    let clicked_idx = (rel_y / row_h) as usize;
    if clicked_idx < self.items.len() && !event.alt && !event.meta {
        self.focus_index = clicked_idx;
        // NOTE: drain_pending_fires is called inside process_item_input.
        self.process_item_input(clicked_idx, event, state, ctx);
    }
    true
}
```

For `Space` (the listbox-direct path, around line 1430):
```rust
InputKey::Space if event.variant == InputVariant::Press => {
    if !event.alt && !event.meta {
        self.process_item_input(self.focus_index, event, state, ctx);
    }
    true
}
```

For `Enter`:
```rust
InputKey::Enter if event.variant == InputVariant::Press => {
    if !event.alt && !event.meta {
        self.process_item_input(self.focus_index, event, state, ctx);
    }
    true
}
```

Remove the now-duplicated `self.drain_pending_fires(ctx)` calls that were in the original `Input` method after `input_impl` returns — **wait:** `drain_pending_fires` is still called at the end of `Input` for keyboard-nav events (arrows, etc.) that don't go through `process_item_input`. Keep the outer `drain_pending_fires` call in `Input`; just ensure it is not called twice for the `process_item_input` path. Check the existing `Input` method structure to confirm this is safe (adding a second drain is harmless — it drains an empty queue).

- [ ] Compile check:

```bash
cargo check -p emcore 2>&1 | head -30
```

### 3.5 Write C-24 unit tests

Add these tests to the existing `#[cfg(test)]` block in `emListBox.rs`. Use the existing `TestInit`, `test_tree`, `default_panel_state`, `default_input_state` helpers. You need a test-only container that implements `PanelBehavior` and overrides `dispatch_item_input`:

- [ ] Add test helper struct inside `#[cfg(test)]`:

```rust
/// Test-only container: wraps emListBox as a PanelBehavior, implements
/// dispatch_item_input so item children can route input to it.
struct TestListBoxContainer {
    widget: emListBox,
}

impl PanelBehavior for TestListBoxContainer {
    fn Paint(&mut self, _p: &mut emPainter, _cc: emColor, _w: f64, _h: f64, _s: &PanelState) {}
    fn IsOpaque(&self) -> bool { false }

    fn dispatch_item_input(
        &mut self,
        item_index: usize,
        event: &emInputEvent,
        state: &PanelState,
        ctx: &mut PanelCtx,
    ) -> ItemInputResult {
        self.widget.process_item_input(item_index, event, state, ctx)
    }
}
```

- [ ] Write the C-24 mouse-left test. This sets up a tree, creates children, synthesizes a click on a child, and asserts selection:

```rust
#[test]
fn item_mouse_left_selects_via_dispatch() {
    let mut __init = TestInit::new();
    let look = emLook::new();
    let mut lb = emListBox::new(&mut __init.ctx(), look.clone());
    lb.set_items(make_items(&["A", "B", "C"]));
    lb.SetSelectionType(SelectionMode::Multi);

    // Create tree with container as root behavior.
    let (mut tree, root_id) = test_tree();
    let mut container = TestListBoxContainer { widget: lb };

    // Expand: create item children as children of root_id.
    {
        let mut ctx = PanelCtx::new(&mut tree, root_id, 1.0);
        container.widget.create_item_children(&mut ctx);
    }
    tree.set_behavior(root_id, Box::new(container));

    // Children created; get the PanelId of item index 1 ("B").
    let child_b = tree.children(root_id).nth(1).expect("item B panel missing");

    // Take item B's behavior, call Input with child_b as ctx.id.
    let mut behavior = tree.take_behavior(child_b).unwrap();
    let ps = default_panel_state();
    let is = default_input_state();
    {
        let mut ctx = PanelCtx::new(&mut tree, child_b, 1.0);
        behavior.Input(&emInputEvent::press(InputKey::MouseLeft), &ps, &is, &mut ctx);
    }
    tree.put_behavior(child_b, behavior);

    // Assert item 1 is now selected.
    tree.with_behavior_as::<TestListBoxContainer, _>(root_id, |c| {
        assert!(c.widget.IsSelected(1), "item B should be selected after mouse click");
    });
}
```

- [ ] Write the C-24 Space-key test:

```rust
#[test]
fn item_space_selects_via_dispatch() {
    let mut __init = TestInit::new();
    let look = emLook::new();
    let mut lb = emListBox::new(&mut __init.ctx(), look.clone());
    lb.set_items(make_items(&["A", "B", "C"]));
    lb.SetSelectionType(SelectionMode::Multi);

    let (mut tree, root_id) = test_tree();
    let mut container = TestListBoxContainer { widget: lb };
    {
        let mut ctx = PanelCtx::new(&mut tree, root_id, 1.0);
        container.widget.create_item_children(&mut ctx);
    }
    tree.set_behavior(root_id, Box::new(container));

    let child_c = tree.children(root_id).nth(2).expect("item C panel missing");
    let mut behavior = tree.take_behavior(child_c).unwrap();
    {
        let mut ctx = PanelCtx::new(&mut tree, child_c, 1.0);
        behavior.Input(&emInputEvent::press(InputKey::Space), &default_panel_state(), &default_input_state(), &mut ctx);
    }
    tree.put_behavior(child_c, behavior);

    tree.with_behavior_as::<TestListBoxContainer, _>(root_id, |c| {
        assert!(c.widget.IsSelected(2), "item C should be selected after Space");
    });
}
```

- [ ] Write the C-24 Enter-trigger test. This needs a scheduler so `drain_pending_fires` can fire the callback — use `PanelCtx::with_sched_reach` (same pattern as the existing `trigger_item_fires_callback` test around line 2415):

```rust
#[test]
fn item_enter_triggers_via_dispatch() {
    let mut __init = TestInit::new();
    let look = emLook::new();
    let mut lb = emListBox::new(&mut __init.ctx(), look.clone());
    lb.set_items(make_items(&["A", "B"]));

    let triggered: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let trig_clone = triggered.clone();
    lb.on_trigger = Some(Box::new(
        move |idx: usize, _sched: &mut crate::emEngineCtx::SchedCtx<'_>| {
            trig_clone.borrow_mut().push(idx);
        },
    ));

    let (mut tree, root_id) = test_tree();
    let mut container = TestListBoxContainer { widget: lb };
    {
        let fw_cb: RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>> = RefCell::new(None);
        let mut ctx = PanelCtx::with_sched_reach(
            &mut tree, root_id, 1.0,
            &mut __init.sched, &mut __init.fw, &__init.root, &fw_cb, &__init.pa,
        );
        container.widget.create_item_children(&mut ctx);
    }
    tree.set_behavior(root_id, Box::new(container));

    let child_a = tree.children(root_id).nth(0).expect("item A missing");
    let mut behavior = tree.take_behavior(child_a).unwrap();
    {
        let fw_cb: RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>> = RefCell::new(None);
        let mut ctx = PanelCtx::with_sched_reach(
            &mut tree, child_a, 1.0,
            &mut __init.sched, &mut __init.fw, &__init.root, &fw_cb, &__init.pa,
        );
        behavior.Input(&emInputEvent::press(InputKey::Enter), &default_panel_state(), &default_input_state(), &mut ctx);
    }
    tree.put_behavior(child_a, behavior);

    assert_eq!(*triggered.borrow(), vec![0usize], "Enter should trigger item A");
}
```

- [ ] Run the new tests:

```bash
cargo test -p emcore item_mouse_left_selects_via_dispatch item_space_selects_via_dispatch item_enter_triggers_via_dispatch -- --test-threads=1 2>&1 | tail -20
```

Expected: all three pass.

- [ ] Run full emcore test suite to confirm no regressions:

```bash
cargo test -p emcore -- --test-threads=1 2>&1 | tail -20
```

Expected: all existing tests pass.

- [ ] Commit:

```bash
git add crates/emcore/src/emListBox.rs
git commit -m "feat(emListBox): C-24 — item_index, process_item_input, dispatch routing, unit tests"
```

---

## Task 4: `emListBox::SetItemText` + I-11 unit tests

**Files:**
- Modify: `crates/emcore/src/emListBox.rs`

### 4.1 Change `SetItemText` signature and add child notification

- [ ] Change the signature of `SetItemText` from:

```rust
pub fn SetItemText(&mut self, index: usize, text: String)
```

to:

```rust
pub fn SetItemText(&mut self, index: usize, text: String, ctx: Option<&mut PanelCtx>)
```

- [ ] Update the body. After the existing `iface.item_text_changed(&text)` call, add child notification:

```rust
pub fn SetItemText(&mut self, index: usize, text: String, ctx: Option<&mut PanelCtx>) {
    if let Some(item) = self.items.get_mut(index) {
        if item.text != text {
            item.text = text.clone();
            if let Some(iface) = &mut item.interface {
                iface.item_text_changed(&text);
            }
            if let (Some(child_id), Some(ctx)) = (item.child_panel_id, ctx.as_deref_mut()) {
                ctx.tree.with_behavior_dyn(child_id, |child| {
                    child.on_item_text_changed(&text);
                });
            }
            self.keywalk_chars.clear();
        }
    }
}
```

- [ ] Find all call sites of `SetItemText` in the codebase and add `None` as the third argument (construction-time calls that have no ctx):

```bash
grep -rn "SetItemText(" crates/ --include="*.rs"
```

Update each call site that does not have a `PanelCtx` to pass `None`. Call sites inside a `PanelBehavior` method that has `ctx: &mut PanelCtx` should pass `Some(ctx)`.

- [ ] Compile check:

```bash
cargo check 2>&1 | head -30
```

Expected: no errors.

### 4.2 Write I-11 unit tests

- [ ] Write the I-11 propagation test (inside `#[cfg(test)]` in `emListBox.rs`):

```rust
#[test]
fn set_item_text_propagates_to_child_behavior() {
    let mut __init = TestInit::new();
    let look = emLook::new();
    let mut lb = emListBox::new(&mut __init.ctx(), look.clone());
    lb.set_items(make_items(&["Alpha", "Beta"]));

    let (mut tree, root_id) = test_tree();
    let mut container = TestListBoxContainer { widget: lb };
    {
        let mut ctx = PanelCtx::new(&mut tree, root_id, 1.0);
        container.widget.create_item_children(&mut ctx);
    }
    tree.set_behavior(root_id, Box::new(container));

    // Change item 0's text with ctx — should propagate to child behavior.
    {
        let mut ctx = PanelCtx::new(&mut tree, root_id, 1.0);
        let mut beh = tree.take_behavior(root_id).unwrap();
        let c = beh.as_any_mut().downcast_mut::<TestListBoxContainer>().unwrap();
        c.widget.SetItemText(0, "AlphaNew".to_string(), Some(&mut ctx));
        tree.put_behavior(root_id, beh);
    }

    // Get child 0's behavior and check its text field.
    let child_a = tree.children(root_id).nth(0).unwrap();
    tree.with_behavior_as::<DefaultItemPanelBehavior, _>(child_a, |beh| {
        assert_eq!(beh.text, "AlphaNew");
    });
}
```

- [ ] Write the I-11 no-ctx test (construction-time call, no child exists yet):

```rust
#[test]
fn set_item_text_before_expand_no_panic() {
    let mut __init = TestInit::new();
    let look = emLook::new();
    let mut lb = emListBox::new(&mut __init.ctx(), look);
    lb.set_items(make_items(&["X"]));
    // ctx = None: no child panels yet, should not panic.
    lb.SetItemText(0, "Y".to_string(), None);
    assert_eq!(lb.GetItemText(0), "Y");
}
```

- [ ] Run the new tests:

```bash
cargo test -p emcore set_item_text_propagates_to_child_behavior set_item_text_before_expand_no_panic -- --test-threads=1 2>&1 | tail -20
```

Expected: both pass.

- [ ] Run full emcore suite:

```bash
cargo test -p emcore -- --test-threads=1 2>&1 | tail -20
```

Expected: all pass.

- [ ] Commit:

```bash
git add crates/emcore/src/emListBox.rs
git commit -m "feat(emListBox): I-11 — SetItemText propagates to child behavior, unit tests"
```

---

## Task 5: `emTestPanel` wiring

**Files:**
- Modify: `crates/emtest/src/emTestPanel.rs`

### 5.1 `ListBoxPanel::dispatch_item_input`

`ListBoxPanel` is around line 380. It currently has `Paint`, `Input`, `GetCursor`, `IsOpaque`, `auto_expand`, `AutoExpand`, `LayoutChildren`, `notice`.

- [ ] Add `dispatch_item_input` to `impl PanelBehavior for ListBoxPanel`. Import `ItemInputResult` at the top of the file if not already present:

```rust
use emcore::emPanel::ItemInputResult;
```

```rust
fn dispatch_item_input(
    &mut self,
    item_index: usize,
    event: &emInputEvent,
    state: &PanelState,
    ctx: &mut PanelCtx,
) -> ItemInputResult {
    self.widget.process_item_input(item_index, event, state, ctx)
}
```

### 5.2 `CustomItemBehavior` — add `item_index`, `Input`, `on_item_text_changed`

`CustomItemBehavior` struct (around line 436) currently has `text: String`, `selected: bool`, `look: Rc<emLook>`.

- [ ] Add the field:

```rust
item_index: usize,
```

- [ ] Find the factory closure that creates `CustomItemBehavior` (the `set_item_behavior_factory` call, around line 2155). It currently builds:

```rust
Box::new(CustomItemBehavior {
    text: text.to_string(),
    selected,
    look,
})
```

Change to:

```rust
Box::new(CustomItemBehavior {
    item_index: _index,
    text: text.to_string(),
    selected,
    look,
})
```

Note: the factory closure signature is `|_index, text, selected, _look, _sel_mode, _enabled|`. Rename `_index` to `index` and use it:

```rust
lb7.set_item_behavior_factory(
    move |index, text, selected, look, _sel_mode, _enabled| {
        Box::new(CustomItemBehavior {
            item_index: index,
            text: text.to_string(),
            selected,
            look,
        })
    },
);
```

There is also a recursive `CustomItemBehavior` in `AutoExpand` (around line 510). Update that closure too with `item_index: _idx` (or `0` if no index is meaningful there):

```rust
lb.set_item_behavior_factory(move |idx, text, selected, _look, _sel_mode, _enabled| {
    Box::new(CustomItemBehavior {
        item_index: idx,
        text: text.to_string(),
        selected,
        look: look.clone(),
    })
});
```

- [ ] Add `Input` and `on_item_text_changed` to `impl PanelBehavior for CustomItemBehavior`. Place after `IsOpaque`:

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    state: &PanelState,
    _is: &emInputState,
    ctx: &mut PanelCtx,
) -> bool {
    let idx = self.item_index;
    let result = ctx
        .with_parent_behavior(|parent, ctx| parent.dispatch_item_input(idx, event, state, ctx))
        .unwrap_or_default();
    if result.focus_self {
        ctx.request_focus();
    }
    result.consumed
}

fn on_item_text_changed(&mut self, text: &str) {
    self.text = text.to_string();
}
```

- [ ] Compile check:

```bash
cargo check -p emtest 2>&1 | head -30
```

Expected: no errors.

- [ ] Run pre-commit suite:

```bash
cargo-nextest ntr 2>&1 | tail -20
```

Expected: same tests pass as before (no new failures; skipped tests remain skipped).

- [ ] Commit:

```bash
git add crates/emtest/src/emTestPanel.rs
git commit -m "feat(emTestPanel): C-24/I-11 — ListBoxPanel::dispatch_item_input, CustomItemBehavior item_index + Input + on_item_text_changed"
```

---

## Task 6: `emView` drains focus requests

**Files:**
- Modify: `crates/emcore/src/emView.rs`

This task wires `PanelTree::focus_requests` into the view's update cycle so `ctx.request_focus()` actually grants keyboard focus at runtime.

### 6.1 Find the drain site

`emView` already drains `navigation_requests` (search for `drain_navigation_requests` in emView.rs). Drain focus requests in the same location.

- [ ] Find the drain call:

```bash
grep -n "drain_navigation_requests" crates/emcore/src/emView.rs
```

### 6.2 Add focus drain

- [ ] Immediately after `drain_navigation_requests`, add:

```rust
for panel_id in self.tree.drain_focus_requests() {
    self.set_focus(Some(panel_id));
}
```

- [ ] Compile check:

```bash
cargo check -p emcore 2>&1 | head -30
```

Expected: no errors.

- [ ] Run full suite:

```bash
cargo-nextest ntr 2>&1 | tail -20
```

Expected: all previously-passing tests still pass.

- [ ] Commit:

```bash
git add crates/emcore/src/emView.rs
git commit -m "feat(emView): drain focus_requests each frame for ctx.request_focus() support"
```

---

## Self-Review Notes

- `TestListBoxContainer` in emListBox tests is a test-only analog of `ListBoxPanel`; it must live inside `#[cfg(test)]` so it doesn't affect the production API.
- `DefaultItemPanelBehavior::new` signature change (adding `index: usize` as first param) — verify there are no other call sites besides `create_item_children`. Run `grep -n "DefaultItemPanelBehavior::new"` in the codebase.
- `drain_pending_fires` is called inside `process_item_input` only when consumed. The outer `Input` method still calls it for keyboard-nav events. A second call to `drain_pending_fires` when the queue is empty is safe (it drains nothing).
- `SetItemText` with `ctx: Option<&mut PanelCtx>` — all construction-time callers (emTestPanel `AutoExpand` and similar) pass `None`. Any dynamic text change from a running behavior passes `Some(ctx)`.
- The two skipped golden tests (`polydrawpanel_default_render`, `testpanel_expanded`) remain in `.config/nextest.toml` and are unaffected by this work.
