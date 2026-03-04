# emCore Feature Contract: Rust + wgpu Reimplementation

> **Purpose:** Guide an LLM-driven extraction and reimplementation of Eagle Mode's `emCore` library from C++ into idiomatic Rust, targeting a wgpu rendering backend. The end product is a standalone, reusable **zoomable UI framework library** (not the Eagle Mode file manager or sample applications).

> **Date:** 2026-03-04

---

## 1. What emCore Is

emCore is Eagle Mode's **core UI framework library**. It is both a library and an API:

- **As a library**, it provides ~80 source files implementing foundations (strings, containers, threading), a software rendering pipeline, a recursive zoomable panel system, a widget toolkit, layout managers, a model/signal/scheduling system, and windowing abstractions.
- **As an API**, it defines the public interfaces that applications build against: creating panels, painting content, handling input, managing shared state through models, and running the event loop.

emCore is **not** a complete application. The file manager (`emFileMan`), fractal viewer (`emFractal`), chess game (`SilChess`), and other modules in the Eagle Mode repository are consumers of emCore. Our reimplementation targets emCore itself, scoped down from these sample applications.

**Our deliverable:** A Rust crate (`zuicchini`) that provides equivalent functionality to the C++ emCore, with rendering via wgpu instead of CPU-based scanline rasterization. Starts as a single crate with well-separated modules; may split into a workspace later if compile times warrant it.

---

## 2. Architectural Overview

emCore has five major subsystems that compose into a complete UI framework:

```
+------------------------------------------------------------------+
|                        Application Code                          |
|    (creates panels, handles input, paints content, uses models)  |
+------------------------------------------------------------------+
         |              |               |              |
+--------+--+  +--------+--+  +--------+--+  +--------+--+
| Widget    |  | Layout     |  | Model /   |  | Panel /   |
| Toolkit   |  | System     |  | Signal    |  | View /    |
| (emBorder,|  | (Linear,   |  | (emModel, |  | Window    |
|  emButton,|  |  Pack,     |  |  emSignal,|  | (emPanel, |
|  emText-  |  |  Raster)   |  |  emEngine)|  |  emView,  |
|  Field..)|  |            |  |           |  |  emWindow)|
+-----------+  +------------+  +-----------+  +-----------+
         |              |               |              |
+------------------------------------------------------------------+
|                     Scheduler / Event Loop                       |
|         (emScheduler, time slices, cooperative multitasking)     |
+------------------------------------------------------------------+
         |
+------------------------------------------------------------------+
|                     Rendering Backend                             |
|   C++: emPainter (CPU scanline rasterizer, AVX2 SIMD)           |
|   Rust: CPU rasterizer (port of emPainter) + wgpu tile compositor |
+------------------------------------------------------------------+
         |
+------------------------------------------------------------------+
|                     Platform Abstraction                          |
|   C++: emX11/emWnds (X11, Windows)                              |
|   Rust: winit + wgpu (cross-platform)                            |
+------------------------------------------------------------------+
```

---

## 3. Subsystem Contracts

Each subsystem below specifies **what must be reimplemented**, the behavioral contract, and Rust-idiomatic design notes.

---

### 3.1 Foundation Types

**Scope:** Core types, containers, error handling, utilities.

These provide the building blocks all other subsystems depend on. Most map directly to Rust standard library types or well-known crates, but the *behavioral contracts* matter for API compatibility.

#### 3.1.1 Numeric Types & Utilities

| C++ Type/Function | Rust Equivalent | Notes |
|---|---|---|
| `emInt8..emInt64`, `emUInt8..emUInt64`, `emByte` | `i8..i64`, `u8..u64`, `u8` | Direct mapping |
| `emException` | `Result<T, E>` with custom error types | Replace throw/catch with `?` propagation |
| `emLog`, `emWarning`, `emFatalError` | `log` crate (`info!`, `warn!`, `error!`) + `panic!` | Use `tracing` or `log` facade |
| `emGetClockMS`, `emSleepMS` | `std::time::Instant`, `std::thread::sleep` | Direct mapping |
| `emCalcAdler32`, `emCalcCRC32`, `emCalcCRC64`, `emCalcHashCode` | `crc32fast`, `std::hash::Hash` | Use crates for checksums |
| `emGetIntRandom`, `emGetDblRandom` | `rand` crate | Use `rand::Rng` trait |
| `emEncodeUtf8Char`, `emDecodeUtf8Char` | Native `char`, `String` | Rust strings are UTF-8 natively |
| `emTryLoadFile`, `emTrySaveFile`, `emTryLoadDir` | `std::fs` functions returning `Result` | Direct mapping |
| `emTryOpenLib`, `emTryResolveSymbolFromLib` | `libloading` crate | For plugin system if retained |

#### 3.1.2 String (`emString`)

**C++ behavior:** Copy-on-write reference-counted string with printf-style formatting, null-terminated, thread-unsafe sharing.

**Rust mapping:** Use `String` (owned) and `&str` (borrowed). No COW needed -- Rust's ownership model handles this idiomatically.

**Contract:**
- Must support efficient creation, concatenation, substring extraction
- Must support `format!()` equivalent to `emString::Format()`
- Must be UTF-8 (Rust default; C++ emString was locale-dependent)
- Interop with C strings (`CString`/`CStr`) where platform APIs require it

#### 3.1.3 Containers

| C++ Type | Rust Equivalent | Behavioral Contract |
|---|---|---|
| `emArray<T>` (COW dynamic array) | `Vec<T>` | Growable, sortable, binary-searchable |
| `emAvlTreeMap<K,V>` (sorted COW map) | `BTreeMap<K,V>` | Ordered by key, O(log n) lookup |
| `emAvlTreeSet<T>` (sorted COW set) | `BTreeSet<T>` | Ordered, set algebra (union, intersect, difference) |
| `emList` (intrusive linked list) | `VecDeque<T>` or custom intrusive list | Internal scheduler use only |
| `emOwnPtrArray<T>` (owned pointer array) | `Vec<Box<T>>` | Ownership transfer on insert, drop on remove |

**Key Rust adaptation:** Eliminate all copy-on-write. Rust's ownership + borrowing replaces COW with zero-cost moves and explicit cloning.

#### 3.1.4 Smart Pointers & References

| C++ Type | Rust Equivalent | Contract |
|---|---|---|
| `emRef<T>` (intrusive refcount) | `Rc<T>` | Shared ownership of models (see Decision #5 below) |
| `emOwnPtr<T>` (unique ownership) | `Box<T>` | Exclusive ownership |
| `emCrossPtr<T>` (weak auto-null) | `Weak<T>` (from `Rc`/`Arc`) | Must handle upgrade failure via `Option` |
| `emAnything` (type-erased value) | `Box<dyn Any>` with `downcast_ref` | Runtime type checking |

**Decision #5: `Rc` everywhere, no `Arc`.**

The tension was: `Rc<T>` avoids atomic overhead but isn't `Send + Sync`, while wgpu requires `Send + Sync` for GPU resources. This resolved cleanly given our other decisions:

- **Panel tree** (Decision #4) uses arena + `PanelId` handles — no smart pointers at all.
- **Model/Context** (Decision #3) uses `Rc<T>` for typed singletons and `ResourceCache` entries — all single-threaded, never crosses to GPU.
- **Rendering** (Decision #2) hands off tile bitmaps as plain `Vec<u8>` to the wgpu compositor. No shared ownership across threads.

The boundary is: the entire panel/model/signal/scheduler domain is `Rc`-only (or handle-based). The GPU compositor owns its wgpu device/queue exclusively and receives tile data by value through a channel. `Arc` is not needed anywhere. If a future need arises (e.g., background file loading on a thread pool), only the specific data crossing the thread boundary needs `Arc`, and that would be a localized addition, not a global refactor.

#### 3.1.5 Threading & Concurrency

| C++ Type | Rust Equivalent | Contract |
|---|---|---|
| `emThread` | `std::thread::JoinHandle` | Spawn, join, get hardware thread count |
| `emThreadMiniMutex` (spinlock) | `std::sync::Mutex` or `parking_lot::Mutex` | Lock/unlock |
| `emThreadMutex` (readers-writer) | `RwLock<T>` | Concurrent reads, exclusive writes |
| `emThreadRecursiveMutex` | `parking_lot::ReentrantMutex` | Recursive locking (avoid if possible) |
| `emThreadEvent` (semaphore) | `std::sync::mpsc` channels or `tokio::sync::Semaphore` | Thread signaling |
| RAII lock guards | `MutexGuard`, `RwLockReadGuard` | Automatic via Rust's `Drop` |

**Design note:** The emCore scheduler is fundamentally single-threaded (cooperative). Threading primitives are included for completeness but the initial implementation is fully single-threaded — tile rasterization can be parallelized later if benchmarks warrant it.

#### 3.1.6 Process & I/O

| C++ Type | Rust Equivalent | Contract |
|---|---|---|
| `emProcess` | `std::process::Command` + `Child` | Spawn with piped stdin/stdout/stderr, wait, signal |
| `emFileStream` | `BufReader<File>` / `BufWriter<File>` | Buffered I/O with byte-order conversion |
| `emTmpFile` | `tempfile` crate | Auto-cleanup on drop |

---

### 3.2 Scheduler & Cooperative Multitasking

This is the **heart of emCore's execution model**. Everything flows through the scheduler.

#### 3.2.1 Architecture: winit Event Loop + Engine Scheduler

The C++ emCore scheduler was two separable layers:
1. **Outer loop** (`emStandardScheduler::Run`) -- sleeps to ~10ms cadence, calls `DoTimeSlice()`, checks termination. This is a trivial sleep loop with zero OS event awareness. Platform event handling (X11, Windows) ran *inside* `DoTimeSlice()` as an engine.
2. **Inner task executor** (`emScheduler::DoTimeSlice`) -- processes pending signals, wakes engines, executes engines by priority. Self-contained ~70 lines of pointer manipulation with no OS dependencies.

**Decision: We do not reimplement the outer loop.** winit owns the event loop. The inner task executor becomes an `EngineScheduler` struct called from winit's `AboutToWait` callback.

This is a pure simplification because:
- The C++ platform backend was already an engine *inside* `DoTimeSlice()`. With winit, the direction reverses (winit delivers events, we run engines in response), but the engine/signal system is unaffected.
- `IsTimeSliceAtEnd()` is just a wall-clock deadline check -- works identically regardless of who calls `do_time_slice()`.
- Signal instant-chaining is entirely internal to `do_time_slice()`.
- On macOS, iOS, and web, winit *must* own the event loop. A custom outer loop would fight the platform.
- The C++ `emScheduler` was already designed for this separation (`DoTimeSlice` is protected, `Run` is virtual).

**Behavioral contract for `EngineScheduler`:**
- Executes one **time slice** per call to `do_time_slice()`
- Each time slice has two phases:
  1. **Signal phase:** Process pending signals, wake connected engines
  2. **Engine phase:** Execute awake engines by priority (5 levels: VERY_LOW to VERY_HIGH)
- Engines within the same priority use **FIFO ordering with alternating time-slice fairness** (prevents starvation)
- Provides `is_time_slice_at_end()` for engines to yield cooperatively (wall-clock deadline, ~50ms max)
- Termination handled by winit's `event_loop.exit()` rather than a custom `InitiateTermination()`

**Rust design:**
```
pub struct EngineScheduler {
    // Pending signals list
    // 10 engine wake queues (5 priorities x 2 time-slice parities)
    // Time slice counter, clock
    // Deadline time for current slice
}

impl EngineScheduler {
    pub fn do_time_slice(&mut self);           // Called from winit AboutToWait
    pub fn is_time_slice_at_end(&self) -> bool; // Wall-clock deadline check
}
```

**Integration with winit:**
```
impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.scheduler.do_time_slice();
        // Request redraw if needed
    }
    fn window_event(&mut self, ...) {
        // Translate winit events to emCore input events
    }
}
```

**Critical invariant:** The engine scheduler is single-threaded. All engine `Cycle()` calls happen on the main thread. This eliminates the need for locks on shared state within the scheduler's domain.

#### 3.2.2 Engine (`emEngine`)

**Behavioral contract:**
- An engine is a unit of cooperative work tied to a scheduler
- Starts sleeping; wakes via `WakeUp()` or signal connection
- `Cycle()` called when awake; returns `true` to stay awake next slice, `false` to sleep
- Can connect to multiple signals via `AddWakeUpSignal()` (reference-counted connections)
- `IsSignaled(signal)` checks if a specific signal fired this cycle
- Priority affects execution order within a time slice

**Three execution patterns must be supported:**
1. **Polling:** `Cycle()` returns `true` to run every slice
2. **Event-driven:** Connected to signals, wakes only when signaled
3. **Long-running job:** Checks `IsTimeSliceAtEnd()` to yield mid-work

**Rust design:**
```
pub trait Engine {
    fn cycle(&mut self) -> bool;  // Return true to stay awake
}
```

Engine registration, wake-up queues, and signal connections managed by the Scheduler.

#### 3.2.3 Signal (`emSignal`)

**Behavioral contract:**
- Binary event (fired or not-fired per time slice)
- `Signal()` adds to scheduler's pending list
- Connected engines wake when signal is processed
- `IsPending()` returns true until processed
- `Abort()` cancels a pending signal
- One signal can wake multiple engines
- Signaling within a `Cycle()` wakes the target engine within the same time slice (instant chaining)

**Rust design:**
```
pub struct Signal { /* internal id, pending state, connection list */ }

impl Signal {
    pub fn fire(&self, scheduler: &mut Scheduler);
    pub fn is_pending(&self) -> bool;
    pub fn abort(&mut self);
}
```

#### 3.2.4 Timer (`emTimer`)

**Behavioral contract:**
- Arms a signal to fire after N milliseconds
- Supports one-shot and periodic modes
- Periodic mode maintains average rate (bounded by time-slice frequency)
- Multiple timers share a single `TimerCentral` engine per scheduler

---

### 3.3 Model / Context System

The C++ emCore used a **runtime service locator**: `Acquire(context, typeid, name)` looked up or created shared models in an AVL tree keyed by `(TypeId, name)`, with `LookupInherited()` walking up a context tree. This was idiomatic pre-2010 C++ (same pattern as COM's `QueryInterface` or Android's `getSystemService`), but in Rust it means `dyn Any` downcasting everywhere with no compile-time type safety.

#### 3.3.1 Decision: Hybrid — Typed Singletons + Resource Cache

Analysis of real C++ usage shows the service locator served two distinct roles:

1. **Known singleton services** (finite, fixed set): `emClipboard`, `emScreen`, `emCoreConfig`, `emSigModel("emFileModel::UpdateSignal")`, `emVarModel<TkResources>`. These are always the same types, always common, often with empty names. There are roughly 5-10 of these.

2. **Dynamic named resources** (open-ended): `emTextFileModel::Acquire(ctx, "/path/to/file")`, `emImageFileModel::Acquire(ctx, "/some/image.png")`. The key is a runtime string (file path), and the value is cached for sharing across panels.

**We split these into two mechanisms:**

**Typed singleton services** get concrete accessors on the context — no string keys, no `dyn Any`, full compile-time type safety:

```
pub struct Context {
    parent: Option<Weak<Context>>,
    children: Vec<Rc<Context>>,

    // Typed singletons — known at compile time
    clipboard: Option<Rc<Clipboard>>,
    screen: Option<Rc<Screen>>,
    core_config: Option<Rc<CoreConfig>>,
    // ... (small, finite set)
}

impl Context {
    /// Walk up parent chain until found, like C++ LookupInherited
    pub fn clipboard(&self) -> Option<&Rc<Clipboard>> {
        self.clipboard.as_ref().or_else(|| self.parent()?.clipboard())
    }
}
```

**Dynamic named resources** use a typed `ResourceCache<K, V>`, itself stored as a typed singleton in the context:

```
pub struct ResourceCache<V> {
    entries: HashMap<String, Rc<V>>,
}

impl<V> ResourceCache<V> {
    /// Find-or-create, equivalent to Acquire(ctx, name, common=true)
    pub fn get_or_insert_with(&mut self, name: &str, f: impl FnOnce() -> V) -> Rc<V>;
}
```

File models become: `ctx.file_cache().get_or_insert_with("/path", || TextFileModel::new("/path"))`.

**Why this preserves the C++ system's strengths:**
- **Hierarchical scoping:** Typed accessors walk the parent chain, same as `LookupInherited()`. A view-level clipboard override shadows the root one.
- **Find-or-create caching:** `ResourceCache` replicates the `Acquire` pattern for dynamic resources. Two panels requesting the same file path get the same `Rc<T>`.
- **Lazy creation:** Resources created on first access, same as C++.
- **Decoupling:** Panels access services through context, not constructor injection. No threading dependencies through 20 layers of panel nesting.
- **Automatic cleanup:** `Rc`/`Weak` replaces the GC timer. When all user refs drop, the `Rc` strong count reaches 1 (only the cache). The cache can optionally apply LRU eviction, but the default is immediate cleanup — simpler than the C++ `MinCommonLifetime` timer, and equivalent for the common case (`MinCommonLifetime=0`). If profiling reveals cache thrashing, LRU/TTL policies can be added to `ResourceCache` without changing the API.

**What we intentionally drop:**
- **Runtime `typeid` + string lookup for known services.** Compile-time typed accessors are strictly better — a typo is a compile error, not a silent new model creation.
- **Dynamic plugin registration of new service types.** In C++, any code could register arbitrary model types at any context level. This requires `dyn Any` and stringly-typed lookup. Since we control all service types (game UI, not a plugin framework), this flexibility is unnecessary.
- **Separate "common" vs "private" model distinction.** Private models were just `Rc<T>` without registration — in Rust, that's simply owning an `Rc<T>` locally. No framework support needed.

#### 3.3.2 Concrete Model Types

The C++ model types map as follows under the hybrid approach:

| C++ Model Type | Rust Equivalent | Mechanism |
|---|---|---|
| `emCoreConfig` | `CoreConfig` struct | Typed singleton on root context |
| `emClipboard` | `Clipboard` wrapper | Typed singleton, inherited lookup |
| `emScreen` | Removed (winit handles this) | N/A |
| `emSigModel` | Named signal field on relevant struct | Typed singleton or direct field |
| `emVarModel<T>` | `Rc<RefCell<T>>` or typed singleton | Depends on usage — most become typed fields |
| `emVarSigModel<T>` | `Rc<WatchedVar<T>>` (value + change signal) | Typed singleton or direct field |
| `emConfigModel` | `ConfigModel<T: Record>` | Typed singleton, file-backed, KDL serialization |
| `emFileModel` | `FileModel<T>` | `ResourceCache<FileModel<T>>` keyed by path |

#### 3.3.3 Record System (`emRec`) → KDL Serialization

**Decision #6: KDL replaces emRec.**

The C++ emRec was a custom text format (`key = value`, `{ }` nesting, `#` comments, `#%rec:TypeName%#` headers). We have no backward compatibility requirement — we never read Eagle Mode config files. Rather than porting the custom parser or using RON/TOML/JSON, we use **KDL** (KDL Document Language) via the `kdl` crate.

KDL's node-based syntax maps naturally to emRec's patterns:

```kdl
// emRec: AlarmHour = 7
// KDL:
alarm-hour 7

// emRec: ClockBorderColor = {187 170 102 255}
// KDL:
clock-border-color 187 170 102 255

// emRec: colors = { { color = "black" fade = 20 } { color = "blue" fade = 0 } }
// KDL:
colors {
    color "black" fade=20
    color "blue" fade=0
}
```

**Behavioral contract (preserved from emRec):**
- Hierarchical serializable data structures with change notification
- Leaf types: `bool`, `int`, `f64`, `String`, `Color`
- Container types: structs (KDL nodes with properties), arrays (KDL children)
- Human-readable, hand-editable, supports comments
- Change listeners propagate modifications up the tree via Signal system

**Rust design:** Use `kdl` crate for parsing/serialization. Config structs derive or implement a KDL mapping. Change notification via the Signal system.

```
pub trait Record {
    fn from_kdl(node: &kdl::KdlNode) -> Result<Self, ConfigError> where Self: Sized;
    fn to_kdl(&self) -> kdl::KdlNode;
    fn set_to_default(&mut self);
    fn is_default(&self) -> bool;
    fn change_signal(&self) -> &Signal;
}
```

#### 3.3.4 Priority-Scheduled File Loading

**Behavioral contract:**
- Only one file model loads at a time (serialized via `emPriSchedAgent`)
- Clients specify memory limits and priorities
- File loading is incremental (`TryContinueLoading()` returns chunks)
- State machine: `FS_WAITING -> FS_LOADING -> FS_LOADED` (or `FS_LOAD_ERROR`, `FS_TOO_COSTLY`)
- Progress reporting via `GetFileProgress()` (0-100%)

---

### 3.4 Panel / View / Window System

This is the **defining feature** of emCore: the recursive, infinitely-zoomable panel hierarchy.

#### 3.4.1 Decision: Arena + Handles with PanelCtx

The C++ panel tree uses parent-owns-children via raw pointers, with mutable backpointers, self-deletion (`delete this` in `Notice()`), and tree mutation during traversal (guarded by `RestartInputRecursion` flags). This is the classic Rust borrow-checker fight: mutable parent + mutable children + backpointers + self-deletion.

**We use arena allocation with handle-based references and a borrowed PanelCtx for callbacks.**

Panels are stored in a flat `SlotMap`. All parent/child/sibling links are `PanelId` values (generational indices), not references. Tree mutation is inserting/removing from the arena. Panel behavior is a trait object extracted from the arena during callbacks to avoid aliased `&mut`.

```
pub struct PanelId(slotmap::DefaultKey);  // Generational index

pub struct PanelData {
    // Tree links (all by ID, no references)
    parent: Option<PanelId>,
    first_child: Option<PanelId>,
    last_child: Option<PanelId>,
    prev_sibling: Option<PanelId>,
    next_sibling: Option<PanelId>,

    // Name lookup (replaces C++ AVL tree)
    name: String,

    // Behavior (extracted during callbacks)
    behavior: Option<Box<dyn PanelBehavior>>,

    // Layout, viewing state, flags...
    layout_rect: (f64, f64, f64, f64),
    canvas_color: Color,
    // ...
}

pub struct PanelTree {
    arena: SlotMap<PanelId, PanelData>,
    root: Option<PanelId>,
    // Child name lookup: HashMap<(PanelId, String), PanelId>
    name_index: HashMap<(PanelId, String), PanelId>,
}
```

**Callback pattern — extract, call, reinsert:**

During `paint()`, `input()`, `notice()`, etc., the behavior is temporarily taken out of the arena so the trait method can receive `&mut PanelCtx` (which holds `&mut PanelTree`) without aliased borrows:

```
// Inside the framework (not user code):
let mut behavior = tree.arena[id].behavior.take().unwrap();
let mut ctx = PanelCtx { id, tree: &mut tree };
behavior.notice(&mut ctx, flags);
ctx.tree.arena[id].behavior = Some(behavior);
```

**PanelCtx provides safe tree operations to panel implementations:**

```
pub struct PanelCtx<'a> {
    id: PanelId,
    tree: &'a mut PanelTree,
}

impl PanelCtx<'_> {
    // Tree mutation
    fn create_child(&mut self, name: &str, behavior: Box<dyn PanelBehavior>) -> PanelId;
    fn delete_child(&mut self, id: PanelId);
    fn delete_self(&mut self);  // Sets flag; actual removal after callback returns

    // Layout
    fn layout_child(&mut self, child: PanelId, x: f64, y: f64, w: f64, h: f64, canvas_color: Color);

    // Tree queries
    fn parent(&self) -> Option<PanelId>;
    fn children(&self) -> ChildIter;
    fn view_condition(&self) -> f64;
    fn name(&self) -> &str;

    // Context access (Decision #3)
    fn context(&self) -> &Context;
}
```

**Why this works:**
- **No borrow checker fights.** `PanelId` is `Copy` — passing IDs around never borrows anything.
- **Self-deletion** becomes `ctx.delete_self()` setting a flag. The caller (framework notice loop) handles removal after the callback returns, same as C++ removing from the notice ring before calling `HandleNotice()`.
- **Tree mutation during input** uses the same restart pattern as C++: collect panel IDs to visit, check a `restart_input` flag after each callback.
- **Cache-friendly.** Panels packed in contiguous arena memory, not scattered heap allocations.
- **Generational keys** catch use-after-remove at runtime (panic on stale ID), replacing the C++ `emCrossPtr` weak pointer pattern.
- **No `Rc`, no `RefCell`, no `unsafe`** in the panel tree.

**What we intentionally change from C++:**
- `HashMap<(PanelId, String), PanelId>` replaces the per-panel AVL tree for child name lookup. Same O(1) amortized lookup, simpler implementation.
- `behavior: Option<Box<dyn PanelBehavior>>` with take/replace replaces C++ virtual dispatch on the panel node itself. Panel data (layout, viewing state) is separate from panel behavior (paint, input, notice).

#### 3.4.2 Panel Behavioral Contract

- Panels form a **tree**. Each panel has zero or more children.
- Every panel has its own **coordinate system**: width is always `1.0`, height is the panel's **tallness** (aspect ratio).
- A panel's position within its parent is set by the parent calling `Layout(x, y, w, h, canvasColor)`.
- Panels are the unit of **painting**, **input handling**, and **focus**.

**Lifecycle:**
1. Construction: becomes child of parent panel or root of a view
2. Layout: parent calls `Layout()` to position this panel
3. Child layout: `LayoutChildren()` called when layout or child list changes
4. Auto-expansion: when zoom level crosses threshold, `AutoExpand()` creates children dynamically
5. Painting: `Paint(painter, canvasColor)` called if visible
6. Auto-shrink: when zoom level drops, `AutoShrink()` destroys children
7. Destruction: removes from arena and recursively removes all children

**Key properties:**
- `name: String` -- unique among siblings
- `identity: String` -- path from root (e.g., `"root:child1:leaf"`)
- `layout_rect: (x, y, w, h)` -- position in parent coordinates
- `canvas_color: Color` -- background color hint
- `focusable: bool` -- can receive focus
- `enable_switch: bool` -- enabled state (ANDed with ancestors)

**Viewing state (valid only when panel is visible in a view):**
- `is_viewed: bool` -- currently being painted
- `is_in_viewed_path: bool` -- self or descendant is viewed
- `viewed_rect: (x, y, w, h)` -- position in screen pixels
- `clip_rect: (x1, y1, x2, y2)` -- clipping bounds in view coordinates
- `view_condition: f64` -- size metric that increases as user zooms in

**Coordinate transforms:**
```
panel_to_view_x(x) = x * viewed_width + viewed_x
panel_to_view_y(y) = y * viewed_width / pixel_tallness + viewed_y
```

**PanelBehavior trait:**
```
pub trait PanelBehavior {
    fn paint(&self, ctx: &PanelCtx, painter: &mut Painter, canvas_color: Color);
    fn input(&mut self, ctx: &mut PanelCtx, event: &InputEvent, state: &InputState, mx: f64, my: f64);
    fn get_cursor(&self, ctx: &PanelCtx) -> Cursor;
    fn is_opaque(&self, ctx: &PanelCtx) -> bool;
    fn layout_children(&mut self, ctx: &mut PanelCtx);
    fn notice(&mut self, ctx: &mut PanelCtx, flags: NoticeFlags);
    fn auto_expand(&mut self, ctx: &mut PanelCtx);
    fn auto_shrink(&mut self, ctx: &mut PanelCtx);
    fn cycle(&mut self, ctx: &mut PanelCtx) -> bool;
}
```

**Notice flags that must be supported:**
- `CHILD_LIST_CHANGED` -- children added/removed
- `LAYOUT_CHANGED` -- position/size changed
- `VIEWING_CHANGED` -- visibility or viewed rect changed
- `ENABLE_CHANGED` -- enabled state changed
- `ACTIVE_CHANGED` -- active state changed
- `FOCUS_CHANGED` -- focus state changed
- `VIEW_FOCUS_CHANGED` -- view gained/lost OS focus
- `UPDATE_PRIORITY_CHANGED` -- priority changed
- `MEMORY_LIMIT_CHANGED` -- memory limit changed

#### 3.4.3 View (`emView`)

**Behavioral contract:**

A view is a **viewport** into a panel tree. It manages navigation, animation, focus, and rendering.

**Core state:**
- Root panel (the tree being viewed)
- **Supreme viewed panel** -- the highest panel whose parent is NOT visible (determines what to render)
- Active panel -- the panel the user is "at" for navigation purposes
- Focused panel -- the panel receiving keyboard input
- Visit state -- which panel the camera is anchored to, with relative offset and zoom

**Navigation model (the infinite zoom):**
- `Visit(panel, rel_x, rel_y, rel_a, adherent)` -- smoothly animate camera to show `panel` at relative position and zoom level
- `rel_x, rel_y`: offset from panel center (in panel-widths/heights)
- `rel_a`: view area relative to panel area (1.0 = panel fills view)
- `VisitFullsized(panel)` -- zoom to fit panel exactly
- `VisitNext/Prev/In/Out/Left/Right/Up/Down()` -- directional navigation
- `Zoom(fix_x, fix_y, factor)` -- zoom around a point
- `Scroll(dx, dy)` -- pan the view

**View flags:**
- `POPUP_ZOOM` -- zoom creates popup window at cursor
- `ROOT_SAME_TALLNESS` -- root panel matches view aspect ratio
- `NO_ZOOM` -- disable zooming
- `NO_USER_NAVIGATION` -- disable all user navigation
- `NO_FOCUS_HIGHLIGHT` -- suppress focus visual
- `NO_ACTIVE_HIGHLIGHT` -- suppress active visual
- `EGO_MODE` -- first-person navigation mode

**View input filter chain:**
The view processes input through a chain of filters before delivering to panels:
1. `DefaultTouchVIF` -- convert multi-touch gestures to zoom/pan/mouse events
2. `CheatVIF` -- debug/cheat code handling
3. `KeyboardZoomScrollVIF` -- arrow keys, Page Up/Down for navigation
4. `MouseZoomScrollVIF` -- mouse wheel zoom, middle-button pan

Each filter can consume events or pass them through.

**Input delivery to panels:**
After VIF chain, events propagate **bottom-up** from the topmost panel at the cursor position up through ancestors. Any panel can consume (eat) the event.

**Rendering flow:**
1. Determine supreme viewed panel
2. Recursively paint from root down through viewed path
3. Each panel painted with appropriate coordinate transform and clip rect
4. Siblings painted in stacking order (first child = back, last child = front)

#### 3.4.4 View Animators

**Behavioral contract:**

Animators provide smooth, physically-modeled camera movement.

| Animator | Behavior |
|---|---|
| `KineticViewAnimator` | Velocity-based with friction (deceleration). Base for others. | **Essential** |
| `SpeedingViewAnimator` | Accelerates toward a target velocity (for keyboard navigation). | **Essential** |
| `VisitingViewAnimator` | Smooth animation for `Visit()` calls. Curved pathfinding through panel tree. Handles seeking non-existent panels. | **Essential** |
| `SwipingViewAnimator` | Touch-drag with spring physics and momentum. | Deferred (touch support) |
| `MagneticViewAnimator` | Snaps view to "best" panel alignment automatically. | Deferred (polish) |

Animators have master/slave relationships and can overlay each other. Each produces velocity deltas that the view integrates per frame. Implement the three essential animators first; Swiping and Magnetic are deferred until touch support and UI polish phases respectively.

#### 3.4.5 Window (`emWindow`)

**Behavioral contract:**
- Extends View with OS window management
- Flags: `MODAL`, `UNDECORATED`, `POPUP`, `MAXIMIZED`, `FULLSCREEN`, `AUTO_DELETE`
- Position/size management (with border awareness)
- Window icon
- Close signal
- Transient window relationships (dialogs parented to owner)
- `WindowStateSaver` -- persists geometry to config file

**Rust mapping:** Use `winit` for window creation. The emWindow abstraction wraps a winit `Window` + a wgpu `Surface`.

#### 3.4.6 Screen (`emScreen`)

**Behavioral contract:**
- Desktop geometry: virtual desktop bounds, per-monitor rects
- DPI query
- Mouse pointer control (move, warp)
- Screensaver inhibition
- Window creation factory

**Rust mapping:** Use `winit` monitor enumeration and window building.

#### 3.4.7 Input System

**Behavioral contract:**

Input events carry:
- `key: InputKey` -- mouse buttons, touch, keyboard keys, modifiers
- `chars: String` -- UTF-8 text for keyboard events
- `repeat: u32` -- 1=single, 2=double-click
- `variant: u32` -- 0=left/main, 1=right/numpad

Input state tracks:
- Mouse position `(x, y)`
- Active touches with IDs and positions
- All key states as a bitfield
- Modifier state helpers: `shift()`, `ctrl()`, `alt()`, `meta()`

Hotkey type: combination of modifiers + key, parseable from strings like `"Ctrl+C"`.

Cursor types: `Normal`, `Invisible`, `Wait`, `Crosshair`, `Text`, `Hand`, `LeftRightArrow`, `UpDownArrow`, `LeftRightUpDownArrow`.

#### 3.4.8 Clipboard

**Behavioral contract:**
- `put_text(text, selection)` -- put text to clipboard or X11 selection
- `get_text(selection) -> String` -- retrieve text
- `clear(selection)` -- clear

**Rust mapping:** Use `arboard` or `clipboard` crate, or winit clipboard support.

---

### 3.5 Rendering System

**Architecture: CPU rasterization + GPU compositing (tile-based).**

The Painter is reimplemented as a CPU software rasterizer in Rust, closely following the C++ original. Panels paint into bitmap tile buffers. wgpu composites the tiles to screen. This hybrid approach preserves the original's rendering advantages (canvas color blending, f64 precision, sub-pixel anti-aliasing) while using the GPU for efficient display and scaling.

```
Panel::Paint() calls
    |
    v
Painter (CPU software rasterizer)
    |-- Immediate-mode: paint calls write pixels directly to tile buffer
    |-- f64 coordinates throughout, 4096x sub-pixel AA grid
    |-- Canvas color blending applied per-pixel natively
    |
    v
Tile Cache (NEW)
    |-- Each visible panel region maps to one or more bitmap tiles
    |-- Dirty tracking: only re-rasterize tiles whose panels changed
    |-- Tiles are CPU bitmaps (RGBA, fixed size e.g. 256x256)
    |
    v
wgpu Compositor
    |-- Upload dirty tiles as GPU textures
    |-- Render textured quads with affine transforms
    |-- Simple shader: sample tile texture, output to surface
    |-- Handles window resize, DPI scaling, vsync
```

**Why this approach:**
- Canvas color blending (`target += (source - canvas) * alpha`) works natively -- no custom shaders needed
- f64 precision throughout the coordinate chain -- no f32 jitter at deep zoom
- Sub-pixel anti-aliasing at 4096x resolution -- matches the original exactly
- No tessellation complexity (no lyon, no bezier-to-triangles conversion)
- No custom GPU shaders for gradients, strokes, or blending
- Tile caching means static panels don't re-rasterize -- potentially faster than the C++ version which repaints everything every frame
- wgpu's role is minimal and well-understood (textured quads)

#### 3.5.1 Painter API (`emPainter`)

The Painter is the rendering interface that all panels use. It is an **immediate-mode CPU rasterizer** that writes pixels directly to bitmap tile buffers. The API is a direct port of the C++ original.

**Coordinate system:**
- User space: f64 floating-point coordinates set by origin + scale
- Transform: `x_pixels = x_user * scale_x + origin_x`
- Clipping: axis-aligned rectangle in pixel coordinates (fractional for sub-pixel clipping)
- The View creates painters with narrowed clip rects for each panel; panels receive a pre-clipped painter

**Drawing primitives -- all must be supported:**

**Filled areas:**
- `paint_rect(x, y, w, h, texture, canvas_color)`
- `paint_polygon(points, texture, canvas_color)` -- convex/concave, with holes
- `paint_ellipse(x, y, w, h, texture, canvas_color)`
- `paint_ellipse_sector(x, y, w, h, start_angle, range_angle, texture, canvas_color)`
- `paint_bezier(points, texture, canvas_color)` -- closed cubic bezier path
- `paint_round_rect(x, y, w, h, rx, ry, texture, canvas_color)`

**Stroked lines:**
- `paint_line(x1, y1, x2, y2, thickness, stroke, start_end, end_end, canvas_color)`
- `paint_polyline(points, thickness, stroke, start_end, end_end, canvas_color)`
- `paint_bezier_line(points, thickness, stroke, start_end, end_end, canvas_color)`
- `paint_ellipse_arc(x, y, w, h, start, range, thickness, stroke, start_end, end_end, canvas_color)`

**Outlined shapes (closed stroked paths):**
- `paint_rect_outline`, `paint_polygon_outline`, `paint_bezier_outline`, `paint_ellipse_outline`, `paint_ellipse_sector_outline`, `paint_round_rect_outline`

**Images:**
- `paint_image(x, y, w, h, image, alpha, canvas_color, extension)` -- with optional source rect
- `paint_image_colored(x, y, w, h, image, color1, color2, canvas_color, extension)` -- color gradient mapping
- `paint_border_image(...)` -- 9-patch scaling for borders

**Text:**
- `paint_text(x, y, text, char_height, width_scale, color, canvas_color)`
- `paint_text_boxed(x, y, w, h, text, max_char_height, color, canvas_color, alignment, ...)` -- fitted text
- `get_text_size(text, char_height, ...) -> (width, height)`

**What changes from the C++ Painter:**
- No `UserSpaceMutex` / `LeaveUserSpace` / `EnterUserSpace` -- single-threaded, no mutex coordination needed
- No raw bitmap constructor with arbitrary pixel format masks -- tiles are always RGBA8
- No `emRenderThreadPool` -- tile rasterization is single-threaded initially (can add parallelism later by rasterizing independent tiles on separate threads)
- Sub-painter copying (used by the View to set clip+transform per panel) becomes push/pop state on the painter, avoiding the C++ shallow-copy-with-shared-buffer pattern

#### 3.5.2 Texture System

Textures define how filled areas and strokes are colored:

| Type | Description |
|---|---|
| `Color` | Solid RGBA color |
| `Image` | Bitmap with interpolation |
| `ImageColored` | Bitmap with two-color gradient mapping |
| `LinearGradient` | Two-point linear gradient |
| `RadialGradient` | Elliptical radial gradient |

**Image extension modes:** `Tiled`, `Edge` (clamp), `Zero` (transparent outside bounds)

**Image quality levels:**
- Downscale: Nearest, 2x2 through 6x6 area sampling
- Upscale: Nearest, AreaSampling, Bilinear, Bicubic, Lanczos, Adaptive

All implemented in the CPU rasterizer, matching the C++ original's quality.

#### 3.5.3 Stroke System

Line styling:
- Dash types: `Solid`, `Dashed`, `Dotted`, `DashDotted`
- Configurable dash/gap length factors
- Rounded or angular joins/caps

Line end decorations (16 types, API designed for all, implementation prioritized):

**Essential (implement first):** `Butt`, `Cap`, `Round` (HalfCircle), `Arrow`

**Deferred (stub until needed):** `ContourArrow`, `LineArrow`, `Triangle`, `ContourTriangle`, `Square`, `ContourSquare`, `HalfSquare`, `Circle`, `ContourCircle`, `Diamond`, `ContourDiamond`, `HalfDiamond`, `Stroke`

Each end has configurable inner color, width factor, and length factor. The `StrokeEnd` enum includes all 16 variants from the start so the API is stable; deferred variants return a fallback (e.g., `Butt`) until implemented.

#### 3.5.4 Canvas Color Blending

emCore uses a non-standard blending formula for overlapping objects that share edges:

```
target_new = target_old + (source - canvas_color) * alpha
```

This prevents color bleeding at shared edges. Standard alpha blending:
```
target_new = target_old * (1 - alpha) + source * alpha
```

The canvas color formula is **essential for correct rendering** of the bordered panel system. Because the Painter is a CPU rasterizer, this formula is applied per-pixel natively -- no custom GPU shaders required.

#### 3.5.5 Color System

`Color` is a 32-bit RGBA value:
- Bit layout: `R[31:24] G[23:16] B[15:8] A[7:0]`
- Alpha: 255 = opaque, 0 = transparent
- Named color constants, HSV conversion, blending operations

#### 3.5.6 Image Type

```
pub struct Image {
    width: u32,
    height: u32,
    channel_count: u8,  // 1 (grey), 2 (grey+alpha), 3 (RGB), 4 (RGBA)
    data: Vec<u8>,      // Row-major: (y*width + x) * channel_count + c
}
```

Must support: creation, resizing, pixel access, copy, fill, channel conversion.

#### 3.5.7 Font Cache & Text Rendering

**Behavioral contract:**
- LRU glyph cache with configurable memory limit
- Per-character metrics (dimensions, positioning)
- Lazy loading (glyphs loaded on demand)
- Text painted as colored images into tile buffers by the CPU rasterizer

Font rendering stays CPU-based, matching the C++ original. Glyphs are rasterized to the glyph cache and painted as images via `paint_image_colored()`. No GPU text rendering crates needed.

#### 3.5.8 Tile Cache (NEW)

The tile cache is new infrastructure that does not exist in the C++ version. The C++ version repaints the entire visible area every frame. The tile cache enables caching of rasterized panel content.

**Behavioral contract:**
- The visible viewport is divided into fixed-size tiles (e.g., 256x256 RGBA8 bitmaps)
- Each tile tracks which panels contribute to it
- When a panel signals a repaint (via the existing Notice/Signal system), tiles overlapping that panel are marked dirty
- Only dirty tiles are re-rasterized each frame
- Clean tiles are re-uploaded to GPU only if not already resident
- Zoom/pan navigation invalidates tiles (new viewport region needs rasterization)
- Tile memory is bounded -- off-screen tiles are evicted LRU

**Dirty tracking: tile-level, full repaint.**

When any panel overlapping a tile signals a change, the entire tile is re-rasterized from scratch -- all panels touching that tile are repainted in order. Panel-level partial updates are not supported because canvas color blending depends on paint order; selectively repainting one panel would require replaying subsequent panels in sequence, effectively building a retained scene graph for marginal gain.

Tile-level dirty tracking is simple, correct by construction, and strictly better than the C++ version (which repaints the entire visible area every frame).

**Design considerations:**
- Tile size: 256x256 RGBA8 is a reasonable starting point (256KB per tile). Tune via benchmarks.
- During fast zoom/pan animation, all tiles are dirty -- falls back to full rasterization (same as C++ original). Caching helps when the view is stable or slowly navigating.
- Off-screen tile eviction: LRU with a configurable memory cap.
- Future optimization: rasterize independent dirty tiles on separate threads (embarrassingly parallel since tiles don't share buffers).
- Tile size, eviction policy, and parallelism are benchmark-driven decisions that do not need to be resolved upfront.

#### 3.5.9 wgpu Compositor

wgpu's role is minimal: display pre-rasterized tile bitmaps on screen.

**Responsibilities:**
- Maintain a pool of GPU textures for tile bitmaps
- Upload dirty tiles from CPU to GPU (`queue.write_texture()`)
- Each frame: render textured quads (one per visible tile) to the surface
- Handle window resize, DPI scaling, vsync
- Single render pipeline: vertex shader (quad positioning) + fragment shader (texture sample, no blending logic)

**What wgpu does NOT do:**
- No geometry tessellation
- No custom blend modes or canvas color shaders
- No gradient computation
- No text rendering
- No anti-aliasing (handled by CPU rasterizer)

**Shader complexity:** One pipeline, one vertex shader, one fragment shader. Textured quads only.

---

### 3.6 Widget Toolkit

All widgets inherit from `emBorder`, which inherits from `emPanel`. The widget toolkit provides ready-made UI components.

#### 3.6.1 Border (`emBorder`) -- Base Widget

**Behavioral contract:**
- Adds border chrome, labels (caption + description + icon), and content area to a panel
- 10 outer border types: `None`, `Filled`, `Margin`, `MarginFilled`, `Rect`, `RoundRect`, `Group`, `Instrument`, `InstrumentMoreRound`, `PopupRoot`
- 4 inner border types: `None`, `Group`, `InputField`, `OutputField`, `CustomRect`
- Auxiliary area support (expandable config panels)
- Look/theme system (`emLook`) with recursive application
- Content area computed from border type + label size
- Shared toolkit resources (pre-loaded border/button images)

**Key virtual methods:**
- `PaintContent(painter, x, y, w, h, canvas_color)` -- override for custom content
- `GetContentRect() -> (x, y, w, h)` -- query content area
- `HasHowTo() / GetHowTo()` -- tooltip system

#### 3.6.2 Widgets to Reimplement

**Essential (reimplement fully):**

| Widget | Purpose | Key Signals/State |
|---|---|---|
| `emButton` | Clickable button | `ClickSignal`, `PressStateSignal`, pressed state |
| `emCheckButton` | Toggle button | `CheckSignal`, checked state |
| `emCheckBox` | Small checkbox variant | Same as CheckButton, different visuals |
| `emRadioButton` | Mutual exclusion button | `Mechanism` for group coordination |
| `emRadioBox` | Small radio variant | Same as RadioButton, different visuals |
| `emLabel` | Non-focusable text display | Caption, description, icon |
| `emTextField` | Text input (single/multi-line) | `TextSignal`, `SelectionSignal`, undo/redo, clipboard, cursor, validation, password mode |
| `emScalarField` | Numeric input with scale | `ValueSignal`, min/max, scale marks, keyboard interval |
| `emColorField` | RGBA/HSV color editor | `ColorSignal`, expandable with slider children |
| `emSplitter` | Resizable two-panel divider | `PosSignal`, min/max position, orientation |
| `emListBox` | Selectable item list | `SelectionSignal`, `ItemTriggerSignal`, selection modes (read-only, single, multi, toggle), sorting |
| `emDialog` | Modal dialog window | `FinishSignal`, result code, OK/Cancel/custom buttons |

**Lower priority (reimplement if needed for game UI):**

| Widget | Purpose | Notes |
|---|---|---|
| `emFileSelectionBox` | File browser | Only if game needs file open/save |
| `emFileDialog` | File open/save dialog | Wraps FileSelectionBox in dialog |
| `emCoreConfigPanel` | Core settings editor | Only for debug/preferences |
| `emErrorPanel` | Error display | Simple text display |

#### 3.6.3 Look/Theme System (`emLook`)

**Behavioral contract:**
- Defines visual properties: background color, foreground color, button colors, etc.
- Applied recursively to widget trees
- Widgets query look for painting
- Must be extensible for custom themes

---

### 3.7 Layout System

Layout classes automatically position child panels. Each provides a different algorithm.

#### 3.7.1 Linear Layout (`emLinearLayout`)

**Behavioral contract:**
- Arranges children in a single row or column
- Orientation: fixed horizontal/vertical, or **adaptive** based on panel tallness vs threshold
- Per-child weight (proportion of space)
- Per-child min/max tallness constraints
- Configurable spacing: inner (between children) and outer (margins)
- Alignment within available space
- Minimum cell count (for empty padding)

#### 3.7.2 Raster Layout (`emRasterLayout`)

**Behavioral contract:**
- Grid layout with uniform cell sizing
- Row-by-row or column-by-column ordering
- Fixed or automatic column/row count
- Preferred/min/max child tallness
- Strict mode (fill container vs maximize panel size)
- Same spacing/alignment system as Linear

#### 3.7.3 Pack Layout (`emPackLayout`)

**Behavioral contract:**
- Recursive binary space partitioning
- Evaluates multiple split positions and orientations
- Minimizes deviation from preferred aspect ratios
- Per-child weight and preferred tallness
- Optimized for ~7 or fewer children
- Produces visually balanced, irregular layouts

#### 3.7.4 Group Variants

Each layout has a `Group` variant that adds:
- Group border (`OBT_GROUP`)
- Focusable by default
- Otherwise identical layout algorithm

---

## 4. Out of Scope

The following emCore-adjacent modules are **NOT** part of this reimplementation:

- **Platform backends**: `emX11`, `emWnds` (replaced by winit + wgpu compositor)
- **emRenderThreadPool**: CPU thread pool for parallel scanline rendering (not needed; tile rasterization is single-threaded initially, parallelizable later without the mutex dance)
- **Sample applications**: `emFileMan`, `emFractal`, `emMines`, `emNetwalk`, `emClock`, `SilChess`, etc.
- **File format codecs**: `emBmp`, `emGif`, `emJpeg`, `emPng`, `emTiff`, `emSvg`, etc. (use Rust `image` crate)
- **emFpPlugin**: Dynamic plugin loading for file panels (not needed for game UI)
- **emMiniIpc**: Inter-process communication (not needed unless multi-process architecture desired)
- **emInstallInfo**: Installation path resolution (replaced by Rust project structure)
- **emTiling**: Deprecated layout (replaced by Linear/Raster/Pack)

---

## 5. Dependency Map for Implementation Order

Implementation should proceed bottom-up through the dependency graph:

```
Phase 1: Foundation
    Types, Error handling, Logging
    String (just use std String)
    Containers (just use std collections)

Phase 2: Engine Scheduler
    Signal
    EngineScheduler (task executor, called from winit)
    Engine (cooperative tasks)
    Timer

Phase 3: Model System
    Context (typed singletons + ResourceCache, inherited lookup)
    WatchedVar, named signals
    Record trait (KDL serialization)
    ConfigModel, FileModel

Phase 4: Rendering
    Color, Image
    Texture, Stroke, StrokeEnd types
    Painter (CPU software rasterizer, port of emPainter)
    Font cache / text rendering
    Tile cache (dirty tracking, tile management)
    wgpu compositor (tile upload, textured quad display)

Phase 5: Panel System
    Panel (core abstraction)
    View (viewport + navigation)
    View Animators (kinetic, speeding, visiting; swiping+magnetic deferred)
    View Input Filters (mouse zoom, keyboard nav, touch)
    Input system (events, state, hotkeys)
    Cursor, Clipboard

Phase 6: Windowing
    Screen abstraction (via winit)
    Window (via winit + wgpu surface)
    WindowStateSaver

Phase 7: Widget Toolkit
    Border (base widget)
    Look/theme system
    Label, Button, CheckButton/Box, RadioButton/Box
    TextField, ScalarField, ColorField
    Splitter, ListBox
    Dialog

Phase 8: Layout System
    LinearLayout / LinearGroup
    RasterLayout / RasterGroup
    PackLayout / PackGroup
```

---

## 6. Rust Crate Structure (Suggested)

```
zuicchini/
  src/
    lib.rs
    foundation/       # Types, utilities, error handling
    scheduler/        # EngineScheduler, Engine, Signal, Timer
    model/            # Context, Model, VarModel, Record, FileModel
    render/
      painter/        # CPU software rasterizer (port of emPainter)
      color.rs        # Color type, HSV conversion
      image.rs        # Image type (CPU bitmap)
      texture.rs      # Texture fill types
      stroke.rs       # Stroke and StrokeEnd types
      font_cache.rs   # Glyph cache, text metrics
      tile_cache.rs   # Tile management, dirty tracking
    compositor/       # wgpu tile compositor
      shaders/        # WGSL shader sources (minimal: textured quads)
    panel/            # Panel, View, ViewAnimator, InputFilter
    input/            # InputEvent, InputState, InputKey, Cursor, Clipboard
    window/           # Window, Screen, WindowStateSaver
    widgets/          # Border, Button, TextField, etc.
    layout/           # LinearLayout, RasterLayout, PackLayout
```

**Key Rust crate dependencies:**
- `wgpu` -- GPU compositing (tile display, not rasterization)
- `winit` -- window creation and event loop
- `kdl` -- KDL serialization for Record/config system
- `log` + `tracing` -- logging
- `rand` -- random numbers
- `arboard` -- clipboard access

---

## 7. Key Behavioral Invariants

These invariants must hold across the entire reimplementation:

1. **Single-threaded engine scheduler:** All engine `Cycle()` calls happen on the main thread, driven by winit's `AboutToWait` callback. No locks needed for model/signal/panel state.

2. **Cooperative yielding:** Long operations must check `is_time_slice_at_end()` and yield. No blocking calls in engine code. The deadline is a wall-clock check (~50ms max per slice), independent of the winit event loop.

3. **Panel coordinate invariant:** A panel's width is always `1.0`. Height equals tallness. Children are positioned in parent coordinates via `Layout()`.

4. **Canvas color blending:** The formula `target += (source - canvas) * alpha` must be used wherever canvas color is specified. This is not standard alpha blending. Because the Painter is a CPU rasterizer, this is applied per-pixel natively.

5. **Signal instant chaining:** A signal fired during `Cycle()` wakes the connected engine within the same time slice (not deferred to next slice).

6. **Resource identity:** Two `ResourceCache::get_or_insert_with()` calls with the same key must return the same `Rc<T>` instance. Typed singleton accessors always return the same instance per context level.

7. **Focus follows zoom:** As the user zooms into a panel, the active panel updates to reflect the current navigation position.

8. **View condition monotonicity:** `get_view_condition()` increases monotonically as the user zooms into a panel. Auto-expansion triggers at threshold.

9. **Input bottom-up propagation:** Input events start at the deepest panel under the cursor and propagate upward. Any panel can consume the event.

10. **Child stacking order:** First child is drawn first (back), last child drawn last (front). Same order for input (front panel gets first chance).
