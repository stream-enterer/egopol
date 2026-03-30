# emFilePanel Completion Design

Date: 2026-03-30

## Objective

Wire emFilePanel to emFileModel so file panels actively track model state,
drive the loading lifecycle via the priority scheduler, and forward
memory/priority from the panel tree to the model. This is emCore
infrastructure — without it, no plugin panel renders.

## Rationale

emFilePanel currently paints loading/error status screens but has no connection
to an actual file model. It stores state passively (callers must manually set
`file_state`, `memory_need`, etc.) and does not participate in Cycle/Notice.
Every file-viewing panel (emDirPanel, emDirStatPanel, emFileLinkPanel,
emStocksFilePanel) inherits from emFilePanel and needs this integration to
function. The priority scheduler (emPriSchedAgent) is already ported and ready
to drive model loading.

## Scope

- FileModelClient trait and client list in emFileModel
- FileModelState trait for type erasure (emFilePanel holds any model)
- Scheduler integration in emFileModel (PSAgent lifecycle)
- emFilePanel: SetFileModel, Cycle, Notice, IsContentReady
- Tests for all new behavior

---

## Section 1: Principles & Constraints

### C++ parity (governing principle)

The integration matches C++ architecture: emFilePanel holds a model reference,
registers as a client, monitors state changes in Cycle(), forwards
memory/priority in Notice(). The model participates in the priority scheduler
independently of panels.

### File and Name Correspondence (inherited)

`emFilePanel.h` maps to `emFilePanel.rs`. `emFileModel.h` maps to
`emFileModel.rs`. Method names match C++ (`SetFileModel`, `GetVirFileState`,
`IsContentReady`, `UpdateMemoryLimit`, `UpdatePriority`). The
`FileModelClient` trait matches the C++ abstract base class
`emFileModelClient` by name correspondence at the method level.

### No new files

All changes are to existing files: `emFilePanel.rs` and `emFileModel.rs`.

---

## Section 2: FileModelClient and Client List

### FileModelClient trait

Port of C++ `emFileModelClient`. Panels implement this to participate in
model memory/priority decisions.

```rust
pub(crate) trait FileModelClient {
    fn get_memory_limit(&self) -> u64;
    fn get_priority(&self) -> f64;
    fn is_reload_annoying(&self) -> bool;
}
```

### Client list in emFileModel

Add to `emFileModel<T>`:

```rust
clients: Vec<Weak<RefCell<dyn FileModelClient>>>,
memory_limit_invalid: bool,
priority_invalid: bool,
```

Methods:

- `AddClient(client: &Rc<RefCell<dyn FileModelClient>>)` — push weak ref,
  invalidate limits and priority, wake model
- `RemoveClient(client: &Rc<RefCell<dyn FileModelClient>>)` — remove matching
  weak ref by pointer identity, invalidate limits and priority, wake model
- `UpdateMemoryLimit()` — iterate clients (cleaning dead weak refs), compute
  max limit. If limit changes: may trigger TooCostly → Waiting or
  Waiting → TooCostly transitions
- `UpdatePriority()` — iterate clients, compute max priority. Update PSAgent
  priority via `PriSchedModel::SetAccessPriority()`
- `IsAnyClientReloadAnnoying() -> bool` — iterate clients, return true if any
  client returns true

The client list is typically 1-2 entries (one panel per model, occasionally
two when a panel is being replaced). Same behavioral semantics as C++ intrusive
linked list.

---

## Section 3: FileModelState Trait (Type Erasure)

emFilePanel needs to hold a reference to its model without knowing the data
type `T`. C++ solves this via the non-generic `emFileModel` base class. Rust
needs a trait for type erasure.

```rust
/// Read-only view of file model state, erasing the data type T.
/// DIVERGED: C++ emFileModel base class — Rust uses trait for type erasure
/// since emFileModel<T> is generic.
pub(crate) trait FileModelState {
    fn GetFileState(&self) -> FileState;
    fn GetFileProgress(&self) -> f64;
    fn GetErrorText(&self) -> &str;
    fn get_memory_need(&self) -> u64;
    fn GetFileStateSignal(&self) -> SignalId;
}
```

`emFileModel<T>` implements `FileModelState` for all `T`. emFilePanel holds
`Option<Rc<RefCell<dyn FileModelState>>>`.

---

## Section 4: Scheduler Integration in emFileModel

Port of C++ `emFileModel::Cycle()` and PSAgent lifecycle.

### New fields in emFileModel<T>

```rust
ps_agent: Option<PriSchedAgentId>,
ps_model: Option<Rc<RefCell<PriSchedModel>>>,
```

### PSAgent lifecycle

- `StartPSAgent(scheduler)` — register agent with `PriSchedModel`, request
  access at current priority
- `EndPSAgent(scheduler)` — release access, remove agent

### GotAccess callback

When the scheduler grants access, the model runs loading/saving in a
time-sliced loop:

```
loop {
    if time_slice_ended { break }
    match state {
        Waiting | Loading => step_loading(ops)
        Unsaved | Saving  => step_saving(ops)
        _ => { EndPSAgent(); break }
    }
}
```

### Cycle method

Port of C++ `emFileModel::Cycle()`. Called by the model's engine registration
(not by panels):

1. `UpdateMemoryLimit()` if invalidated
2. `UpdatePriority()` if invalidated
3. If state is Waiting and memory permits → `StartPSAgent()`
4. If Loaded → check `IsOutOfDate()`, if yes and not annoying → reset and
   reload
5. Signal `FileStateSignal` on any state change

### What stays the same

`step_loading()`, `step_saving()`, `FileModelOps` trait,
`emAbsoluteFileModelClient<T>` — all unchanged.

---

## Section 5: emFilePanel Changes

### Struct fields

Replace the boolean stub with an active model connection:

```rust
pub struct emFilePanel {
    // Keep:
    custom_error: Option<String>,

    // Replace has_model/file_state/error_text/memory_need/memory_limit:
    model: Option<Rc<RefCell<dyn FileModelState>>>,
    last_vir_file_state: VirtualFileState,

    // Cached panel state for FileModelClient:
    cached_memory_limit: u64,
    cached_priority: f64,
    cached_in_active_path: bool,
}
```

### SetFileModel()

Port of C++ `emFilePanel::SetFileModel`:

1. If old model exists: unregister as client
2. Store new model reference
3. If new model exists: register as client
4. Compute and cache VirtualFileState
5. Invalidate painting if state changed

### FileModelClient implementation

emFilePanel implements `FileModelClient`. Methods read from cached panel state:

- `get_memory_limit()` → `self.cached_memory_limit`
- `get_priority()` → `self.cached_priority`
- `is_reload_annoying()` → `self.cached_in_active_path`

### Cycle()

Port of C++ `emFilePanel::Cycle`:

```rust
fn Cycle(&mut self) -> bool {
    if let Some(ref model) = self.model {
        let model = model.borrow();
        let new_state = self.compute_vir_file_state(&*model);
        if new_state != self.last_vir_file_state {
            self.last_vir_file_state = new_state;
            return true;  // invalidate painting
        }
    }
    false
}
```

### notice()

Port of C++ `emFilePanel::Notice`:

```rust
fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
    if flags.contains(NoticeFlags::MEMORY_LIMIT_CHANGED) {
        self.cached_memory_limit = state.memory_limit;
        // invalidate model's memory limit for re-aggregation
    }
    if flags.contains(NoticeFlags::UPDATE_PRIORITY_CHANGED) {
        self.cached_priority = state.priority;
        // invalidate model's priority for re-aggregation
    }
}
```

### IsContentReady()

Port of C++ `emFilePanel::IsContentReady`. Added to PanelBehavior:

- Waiting/Loading → `(false, readying: true)`
- Error/TooCostly/NoModel → `(false, readying: false)`
- Loaded/Unsaved → delegate to tree's default

### paint_status() unchanged

The existing status painting code is preserved. Derived panels check
`GetVirFileState().is_good()` and render content instead of calling
`paint_status()`.

---

## Section 6: Testing Strategy

### Unit tests (emFilePanel.rs)

- Existing 11 tests: keep — they test paint_status() and VirtualFileState
- Cycle() detects model state change and returns true
- Cycle() returns false when state unchanged
- notice() with MEMORY_LIMIT_CHANGED updates cached limit
- notice() with UPDATE_PRIORITY_CHANGED updates cached priority
- SetFileModel(Some(...)) registers client, SetFileModel(None) unregisters
- IsContentReady() returns correct readying flag for each VirtualFileState

### Unit tests (emFileModel.rs)

- AddClient / RemoveClient lifecycle
- UpdateMemoryLimit aggregates max across 2 clients
- UpdateMemoryLimit with dead weak ref (dropped client) is cleaned up
- UpdatePriority aggregates max across 2 clients
- IsAnyClientReloadAnnoying returns true if any client says yes
- Memory limit change triggers TooCostly → Waiting transition
- Memory limit decrease triggers Loaded → TooCostly (if need > new limit)

### Behavioral tests (scheduler integration)

- Model with PSAgent: loading driven by scheduler, not manual step_loading
- Two models competing: higher-priority loads first
- Client connects → model starts loading; last client disconnects → model stops
- Panel Cycle() + scheduler DoTimeSlice() → full loading lifecycle end-to-end

---

## Section 7: Scope Summary

Files changed:
- `crates/emcore/src/emFileModel.rs` — client list, scheduler integration,
  FileModelState trait, Cycle method
- `crates/emcore/src/emFilePanel.rs` — replace stub with active model
  connection, Cycle/Notice/IsContentReady

Files unchanged:
- `emFpPlugin.rs`, `emPanel.rs`, `emView.rs`, `emPanelTree.rs`,
  `emPriSchedAgent.rs`
- `emStocksFilePanel.rs` (remains a stub — Spec 2 work)

Estimated size: ~200-300 lines new/changed code, ~200 lines tests.

---

## Section 8: Phase Structure

### Phase 1 — emFileModel client list and scheduler

**Goal:** emFileModel supports client registration and drives loading via
the priority scheduler.

Work items:
1. FileModelClient trait definition
2. Client list (Vec<Weak<RefCell<dyn FileModelClient>>>)
3. AddClient / RemoveClient methods
4. UpdateMemoryLimit / UpdatePriority aggregation
5. IsAnyClientReloadAnnoying
6. FileModelState trait + blanket impl
7. PSAgent fields and StartPSAgent / EndPSAgent
8. GotAccess callback with time-sliced loading loop
9. Cycle method (engine-driven)
10. Unit tests for all of the above

Gate: Model loads a file when a client connects with sufficient memory
limit. Two models compete via scheduler priority.

### Phase 2 — emFilePanel integration

**Goal:** emFilePanel actively tracks model state and participates in
panel tree lifecycle.

Work items:
1. Replace struct fields (model ref, cached state)
2. SetFileModel() with client registration
3. FileModelClient impl (reads cached panel state)
4. Cycle() — detect state changes
5. notice() — forward memory/priority
6. IsContentReady() — VFS-based readiness
7. Update existing tests, add new integration tests

Gate: emFilePanel attached to a model shows loading progress, transitions
to Loaded state, reports content ready. Memory limit changes propagate
from panel tree through client to model.
