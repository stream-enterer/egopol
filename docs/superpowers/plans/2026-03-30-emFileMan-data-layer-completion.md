# emFileMan Data Layer Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete 7 partially-implemented files in `crates/emfileman/` to full C++ API parity, fix 3 emcore infrastructure gaps (PanelParentArg, CreateFilePanel return type, emContext scheduler access), and add a synchronous image file loader — zero stubs, zero deferrals.

**Architecture:** Each existing "data" struct (e.g., `emDirModelData`) is preserved. A new "model" struct wraps it with the appropriate emcore model infrastructure (emFileModel, emRecFileModel, emConfigModel, or plain model with generation counter). All model types get `Acquire()` factories. Models that need scheduler access (emFileManModel for IPC server) get it from `emContext::GetScheduler()`, matching C++ `emContext(emScheduler &)`.

**Tech Stack:** Rust, emcore (emFileModel, emRecFileModel, emConfigModel, emContext, emMiniIpc, emProcess, emScheduler), libc (stat/readdir)

**Spec:** `docs/superpowers/specs/2026-03-30-emFileMan-data-layer-completion-design.md`

---

## Reference: Key Imports and Signatures

These are used across multiple tasks. Refer back here instead of re-reading files.

```rust
// crates/emcore/src/emContext.rs
impl emContext {
    pub fn acquire<T: 'static>(&self, name: &str, create: impl FnOnce() -> T) -> Rc<RefCell<T>>;
}

// crates/emcore/src/emFileModel.rs
pub trait FileModelOps {
    fn reset_data(&mut self);
    fn try_start_loading(&mut self) -> Result<(), String>;
    fn try_continue_loading(&mut self) -> Result<bool, String>;
    fn quit_loading(&mut self);
    fn try_start_saving(&mut self) -> Result<(), String>;
    fn try_continue_saving(&mut self) -> Result<bool, String>;
    fn quit_saving(&mut self);
    fn calc_memory_need(&self) -> u64;
    fn calc_file_progress(&self) -> f64;
}

pub struct emFileModel<T> {
    // fields: data: Option<T>, path: PathBuf, state: FileState, change_signal: SignalId, ...
    pub fn new(path: PathBuf, signal_id: SignalId, update_signal: SignalId) -> Self;
    pub fn GetMap(&self) -> Option<&T>;
    pub fn GetFilePath(&self) -> &Path;
    pub fn GetFileState(&self) -> &FileState;
}

// crates/emcore/src/emRecFileModel.rs
pub struct emRecFileModel<T: Record + Default> {
    pub fn new(path: PathBuf) -> Self;
    pub fn GetMap(&self) -> &T;
    pub fn GetFileState(&self) -> &FileState;
    pub fn TryLoad(&mut self) -> Result<(), ...>;
}

// crates/emcore/src/emConfigModel.rs
pub struct emConfigModel<T: Record> {
    pub fn new(value: T, path: PathBuf, signal_id: SignalId) -> Self;
    pub fn GetRec(&self) -> &T;
    pub fn Set(&mut self, new_value: T) -> bool;
    pub fn modify<F: FnOnce(&mut T)>(&mut self, f: F) -> bool;
    pub fn GetChangeSignal(&self) -> SignalId;
    pub fn IsUnsaved(&self) -> bool;
    pub fn TryLoad(&mut self) -> Result<(), RecError>;
    pub fn Save(&mut self) -> Result<(), RecError>;
    pub fn TryLoadOrInstall(&mut self) -> Result<(), RecError>;
}

// crates/emcore/src/emResTga.rs
pub fn load_tga(data: &[u8]) -> Result<emImage, TgaError>;

// crates/emcore/src/emProcess.rs
pub fn TryStartUnmanaged(
    args: &[&str],
    extra_env: &HashMap<String, String>,
    dir_path: Option<&Path>,
    flags: StartFlags,
) -> Result<(), ProcessError>;

// crates/emcore/src/emMiniIpc.rs
type MessageCallback = Box<dyn FnMut(&[String])>;
pub struct emMiniIpcServer {
    pub fn new(scheduler: &mut EngineScheduler, callback: MessageCallback) -> Self;
    pub fn StartServing(&mut self, scheduler: &mut EngineScheduler, name: Option<&str>) -> Result<(), MiniIpcError>;
    pub fn GetServerName(&self) -> String;
}
```

---

## Phase 1 — emcore Infrastructure

### Task 0: Add Scheduler Access to emContext

C++ `emContext` takes `emScheduler &` in its constructor — every model has scheduler access. The Rust `emContext` omitted this. Models like `emFileManModel` need it to create `emMiniIpcServer` during construction (the IPC server must be running by the time `Cycle()` is first called).

**Files:**
- Modify: `crates/emcore/src/emContext.rs`
- Modify: `crates/emcore/src/emGUIFramework.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emcore/src/emContext.rs`:

```rust
    #[test]
    fn scheduler_access_from_context() {
        use crate::emScheduler::EngineScheduler;
        let sched = Rc::new(RefCell::new(EngineScheduler::new()));
        let ctx = emContext::NewRootWithScheduler(Rc::clone(&sched));
        let retrieved = ctx.GetScheduler();
        assert!(retrieved.is_some());
    }

    #[test]
    fn child_inherits_scheduler() {
        use crate::emScheduler::EngineScheduler;
        let sched = Rc::new(RefCell::new(EngineScheduler::new()));
        let root = emContext::NewRootWithScheduler(Rc::clone(&sched));
        let child = emContext::NewChild(&root);
        assert!(child.GetScheduler().is_some());
    }

    #[test]
    fn new_root_without_scheduler() {
        let ctx = emContext::NewRoot();
        assert!(ctx.GetScheduler().is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emContext -- --test-threads=1`
Expected: FAIL — `NewRootWithScheduler` and `GetScheduler` not found

- [ ] **Step 3: Implement scheduler field on emContext**

In `crates/emcore/src/emContext.rs`, add import:

```rust
use crate::emScheduler::EngineScheduler;
```

Add field to `emContext` struct:

```rust
pub struct emContext {
    parent: Option<Weak<emContext>>,
    children: RefCell<Vec<Weak<emContext>>>,
    clipboard: RefCell<Option<Rc<RefCell<dyn emClipboard>>>>,
    registry: RefCell<HashMap<ModelKey, ModelEntry>>,
    /// Scheduler reference, matching C++ `emContext(emScheduler &)`.
    /// Root contexts may carry a scheduler; child contexts inherit via parent.
    scheduler: Option<Rc<RefCell<EngineScheduler>>>,
}
```

Update `NewRoot()` to set `scheduler: None`.

Add `NewRootWithScheduler`:

```rust
    /// Create a root context with scheduler access.
    /// Port of C++ `emRootContext(emScheduler &)`.
    pub fn NewRootWithScheduler(scheduler: Rc<RefCell<EngineScheduler>>) -> Rc<Self> {
        Rc::new(Self {
            parent: None,
            children: RefCell::new(Vec::new()),
            clipboard: RefCell::new(None),
            registry: RefCell::new(HashMap::new()),
            scheduler: Some(scheduler),
        })
    }
```

Update `NewChild()` to set `scheduler: None` (children walk the parent chain).

Add `GetScheduler`:

```rust
    /// Get the scheduler, walking the parent chain if needed.
    /// Port of C++ `emContext::GetScheduler()`.
    pub fn GetScheduler(&self) -> Option<Rc<RefCell<EngineScheduler>>> {
        if let Some(sched) = &self.scheduler {
            return Some(Rc::clone(sched));
        }
        self.GetParentContext()
            .and_then(|parent| parent.GetScheduler())
    }
```

- [ ] **Step 4: Update emGUIFramework to use NewRootWithScheduler**

In `crates/emcore/src/emGUIFramework.rs`, change `App::new()` to store the scheduler as `Rc<RefCell<EngineScheduler>>` and pass it to the context. The `App` struct currently creates `EngineScheduler::new()` as a direct field and `emContext::NewRoot()` separately. Change to:

```rust
pub struct App {
    // ... existing fields ...
    pub scheduler: Rc<RefCell<EngineScheduler>>,
    // ... remove the old `scheduler: EngineScheduler` field ...
}
```

In `App::new()`:
```rust
let scheduler = Rc::new(RefCell::new(EngineScheduler::new()));
let context = emContext::NewRootWithScheduler(Rc::clone(&scheduler));
```

All existing code that does `app.scheduler.foo()` changes to `app.scheduler.borrow_mut().foo()`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p emcore --lib emContext -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 6: Run full workspace check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass. The `Rc<RefCell<>>` wrapping may require updating callers in emGUIFramework and tests.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emContext.rs crates/emcore/src/emGUIFramework.rs
git commit -m "feat(emcore): add scheduler access to emContext, matching C++ emRootContext"
```

---

### Task 1: PanelParentArg Extension + CreateFilePanel Return Type

**Files:**
- Modify: `crates/emcore/src/emFpPlugin.rs`
- Modify: `crates/emstocks/src/emStocksFpPlugin.rs`
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`
- Modify: `crates/emfileman/src/emDirFpPlugin.rs`
- Modify: `crates/emfileman/src/emDirStatFpPlugin.rs`
- Modify: `crates/emfileman/src/emFileLinkFpPlugin.rs`

- [ ] **Step 1: Extend PanelParentArg with parent_panel field**

In `crates/emcore/src/emFpPlugin.rs`, replace the PanelParentArg struct (lines 39-55):

```rust
/// Parent argument for panel creation.
/// DIVERGED: C++ emPanel::ParentArg carries full parent panel reference with
/// layout constraint forwarding. This version carries parent panel ID for tree
/// integration but does not forward layout constraints. Full constraint
/// forwarding deferred to panel framework completion.
pub struct PanelParentArg {
    root_context: Rc<emContext>,
    parent_panel: Option<PanelId>,
}

impl PanelParentArg {
    pub fn new(root_context: Rc<emContext>) -> Self {
        Self {
            root_context,
            parent_panel: None,
        }
    }

    pub fn with_parent(root_context: Rc<emContext>, parent: PanelId) -> Self {
        Self {
            root_context,
            parent_panel: Some(parent),
        }
    }

    pub fn root_context(&self) -> &Rc<emContext> {
        &self.root_context
    }

    pub fn parent_panel(&self) -> Option<PanelId> {
        self.parent_panel
    }
}
```

Add import at top of file:

```rust
use crate::emPanelTree::PanelId;
```

- [ ] **Step 2: Change emFpPluginFunc return type to Box**

In `crates/emcore/src/emFpPlugin.rs`, replace the type alias (lines 18-26):

```rust
/// Type of the plugin function for creating a file panel.
/// Port of C++ `emFpPluginFunc`.
/// DIVERGED: Returns Box<dyn PanelBehavior> (not Rc<RefCell>) because panels
/// are owned by the panel tree, not shared.
pub type emFpPluginFunc = fn(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>>;
```

- [ ] **Step 3: Update TryCreateFilePanel return type**

In `crates/emcore/src/emFpPlugin.rs`, change `TryCreateFilePanel` (line 182-245) return type from `Result<Rc<RefCell<dyn PanelBehavior>>, FpPluginError>` to `Result<Box<dyn PanelBehavior>, FpPluginError>`. No other changes needed — the `func()` call already returns the right type after step 2.

- [ ] **Step 4: Update CreateFilePanel and CreateFilePanelWithStat return types**

In `crates/emcore/src/emFpPlugin.rs`:

Change `CreateFilePanel` (line 694-727) return type from `Rc<RefCell<dyn PanelBehavior>>` to `Box<dyn PanelBehavior>`. Replace every `Rc::new(RefCell::new(...))` with `Box::new(...)` in this method.

Change `CreateFilePanelWithStat` (line 731-766) return type from `Rc<RefCell<dyn PanelBehavior>>` to `Box<dyn PanelBehavior>`. Replace every `Rc::new(RefCell::new(...))` with `Box::new(...)` in this method.

- [ ] **Step 5: Update all 4 plugin entry points**

In `crates/emstocks/src/emStocksFpPlugin.rs`, change return type and wrapping:
```rust
#[no_mangle]
pub fn emStocksFpPluginFunc(
    _parent: &PanelParentArg,
    _name: &str,
    _path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emStocksFpPlugin: No properties allowed.".to_string();
        return None;
    }

    Some(Box::new(emStocksFilePanel::new()))
}
```

In `crates/emfileman/src/emDirFpPlugin.rs`, change return type:
```rust
#[no_mangle]
pub fn emDirFpPluginFunc(
    _parent: &PanelParentArg,
    _name: &str,
    _path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emDirFpPlugin: No properties allowed.".to_string();
        return None;
    }
    // TODO: return new emDirPanel when panel integration is complete
    *error_buf = "emDirFpPlugin: not yet implemented".to_string();
    None
}
```

Apply the same `Rc<RefCell<dyn PanelBehavior>>` → `Box<dyn PanelBehavior>` change to `emDirStatFpPlugin.rs` and `emFileLinkFpPlugin.rs` (same pattern — only the return type and error message string differ).

- [ ] **Step 6: Verify compilation**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS (no errors, no warnings)

- [ ] **Step 7: Run all tests**

Run: `cargo-nextest ntr`
Expected: All 89+ existing tests pass. No behavioral changes.

- [ ] **Step 8: Commit**

```bash
git add crates/emcore/src/emFpPlugin.rs crates/emstocks/src/emStocksFpPlugin.rs crates/emstocks/src/emStocksFilePanel.rs crates/emfileman/src/emDirFpPlugin.rs crates/emfileman/src/emDirStatFpPlugin.rs crates/emfileman/src/emFileLinkFpPlugin.rs
git commit -m "refactor(emcore): extend PanelParentArg, switch CreateFilePanel to Box return"
```

---

### Task 2: Synchronous Image File Loader

**Files:**
- Modify: `crates/emcore/src/emImageFile.rs`

- [ ] **Step 1: Write the failing test**

Add to end of `crates/emcore/src/emImageFile.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn load_nonexistent_file_returns_none() {
        assert!(load_image_from_file(Path::new("/nonexistent/path.tga")).is_none());
    }

    #[test]
    fn load_empty_file_returns_none() {
        let dir = std::env::temp_dir().join("emcore_test_img");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("empty.tga");
        std::fs::write(&path, b"").expect("write");
        assert!(load_image_from_file(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_valid_tga_returns_image() {
        // Minimal valid 1x1 RGBA TGA (type 2, 32bpp)
        let mut tga = vec![0u8; 18 + 4];
        tga[2] = 2;       // uncompressed true-color
        tga[12] = 1;      // width = 1
        tga[14] = 1;      // height = 1
        tga[16] = 32;     // 32 bpp
        tga[18] = 0xFF;   // B
        tga[19] = 0x00;   // G
        tga[20] = 0x00;   // R
        tga[21] = 0xFF;   // A

        let dir = std::env::temp_dir().join("emcore_test_img");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_1x1.tga");
        std::fs::write(&path, &tga).expect("write");

        let img = load_image_from_file(&path);
        assert!(img.is_some());
        let img = img.unwrap();
        assert_eq!(img.GetWidth(), 1);
        assert_eq!(img.GetHeight(), 1);

        let _ = std::fs::remove_file(&path);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emImageFile -- --test-threads=1`
Expected: FAIL — `load_image_from_file` not found

- [ ] **Step 3: Implement load_image_from_file**

Add to `crates/emcore/src/emImageFile.rs`, before the `#[cfg(test)]` block:

```rust
/// Load an image from a file path synchronously.
/// Supports TGA format. Returns None on any error (missing file, bad format).
///
/// DIVERGED: C++ uses the async emImageFileModel plugin system with format
/// dispatching to emTga/emBmp/emGif/etc. This synchronous loader handles TGA
/// only and serves small theme border images. Full async image loading will be
/// ported with the image app modules.
pub fn load_image_from_file(path: &Path) -> Option<emImage> {
    let data = std::fs::read(path).ok()?;
    crate::emResTga::load_tga(&data).ok()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emcore --lib emImageFile -- --test-threads=1`
Expected: All 3 new tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emImageFile.rs
git commit -m "feat(emcore): add synchronous TGA image file loader"
```

---

### Phase 1 Gate

- [ ] **Run full gate check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass. No regressions.

---

## Phase 2 — Layer 0 Models (no inter-emFileMan deps)

### Task 3: emFileManConfig Model Wrapper

**Files:**
- Modify: `crates/emfileman/src/emFileManConfig.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emFileManConfig.rs`:

```rust
    #[test]
    fn config_model_acquire_singleton() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let c1 = emFileManConfig::Acquire(&ctx);
        let c2 = emFileManConfig::Acquire(&ctx);
        assert!(Rc::ptr_eq(&c1, &c2));
    }

    #[test]
    fn config_model_getters_match_defaults() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let cfg = emFileManConfig::Acquire(&ctx);
        let cfg = cfg.borrow();
        assert_eq!(cfg.GetSortCriterion(), SortCriterion::ByName);
        assert_eq!(cfg.GetNameSortingStyle(), NameSortingStyle::PerLocale);
        assert!(!cfg.GetSortDirectoriesFirst());
        assert!(!cfg.GetShowHiddenFiles());
        assert_eq!(cfg.GetThemeName(), "");
        assert!(cfg.GetAutosave());
    }

    #[test]
    fn config_model_setters_round_trip() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let cfg = emFileManConfig::Acquire(&ctx);
        {
            let mut cfg = cfg.borrow_mut();
            cfg.SetSortCriterion(SortCriterion::BySize);
            cfg.SetNameSortingStyle(NameSortingStyle::CaseInsensitive);
            cfg.SetSortDirectoriesFirst(true);
            cfg.SetShowHiddenFiles(true);
            cfg.SetThemeName("Glass1");
            cfg.SetAutosave(false);
        }
        let cfg = cfg.borrow();
        assert_eq!(cfg.GetSortCriterion(), SortCriterion::BySize);
        assert_eq!(cfg.GetNameSortingStyle(), NameSortingStyle::CaseInsensitive);
        assert!(cfg.GetSortDirectoriesFirst());
        assert!(cfg.GetShowHiddenFiles());
        assert_eq!(cfg.GetThemeName(), "Glass1");
        assert!(!cfg.GetAutosave());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManConfig -- --test-threads=1`
Expected: FAIL — `emFileManConfig` type not found

- [ ] **Step 3: Implement emFileManConfig model wrapper**

Add the following to `crates/emfileman/src/emFileManConfig.rs`, after the `Record` impl for `emFileManConfigData` but before `#[cfg(test)]`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emConfigModel::emConfigModel;
use emcore::emSignal::SignalId;

/// Model wrapper for emFileManConfig.
/// Port of C++ `emFileManConfig` (extends emConfigModel).
pub struct emFileManConfig {
    config_model: emConfigModel<emFileManConfigData>,
}

impl emFileManConfig {
    /// Acquire the singleton config model.
    /// Port of C++ `emFileManConfig::Acquire`.
    pub fn Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>("", || {
            // DIVERGED: C++ resolves path via emGetConfigDirOverloadable.
            // We use a default-initialized config since file I/O is handled
            // by emConfigModel::TryLoadOrInstall which panels call after acquire.
            let signal_id = SignalId::default();
            let path = std::path::PathBuf::from("");
            Self {
                config_model: emConfigModel::new(
                    emFileManConfigData::default(),
                    path,
                    signal_id,
                ),
            }
        })
    }

    pub fn GetFormatName(&self) -> &str {
        "emFileManConfig"
    }

    pub fn GetChangeSignal(&self) -> SignalId {
        self.config_model.GetChangeSignal()
    }

    pub fn GetSortCriterion(&self) -> SortCriterion {
        self.config_model.GetRec().sort_criterion
    }

    pub fn SetSortCriterion(&mut self, sc: SortCriterion) {
        self.config_model.modify(|d| d.sort_criterion = sc);
    }

    pub fn GetNameSortingStyle(&self) -> NameSortingStyle {
        self.config_model.GetRec().name_sorting_style
    }

    pub fn SetNameSortingStyle(&mut self, nss: NameSortingStyle) {
        self.config_model.modify(|d| d.name_sorting_style = nss);
    }

    pub fn GetSortDirectoriesFirst(&self) -> bool {
        self.config_model.GetRec().sort_directories_first
    }

    pub fn SetSortDirectoriesFirst(&mut self, b: bool) {
        self.config_model.modify(|d| d.sort_directories_first = b);
    }

    pub fn GetShowHiddenFiles(&self) -> bool {
        self.config_model.GetRec().show_hidden_files
    }

    pub fn SetShowHiddenFiles(&mut self, b: bool) {
        self.config_model.modify(|d| d.show_hidden_files = b);
    }

    pub fn GetThemeName(&self) -> &str {
        &self.config_model.GetRec().theme_name
    }

    pub fn SetThemeName(&mut self, name: &str) {
        self.config_model
            .modify(|d| d.theme_name = name.to_string());
    }

    pub fn GetAutosave(&self) -> bool {
        self.config_model.GetRec().autosave
    }

    pub fn SetAutosave(&mut self, b: bool) {
        self.config_model.modify(|d| d.autosave = b);
    }

    pub fn IsUnsaved(&self) -> bool {
        self.config_model.IsUnsaved()
    }

    /// Access the inner config model for advanced operations (load/save).
    pub(crate) fn config_model(&self) -> &emConfigModel<emFileManConfigData> {
        &self.config_model
    }

    pub(crate) fn config_model_mut(&mut self) -> &mut emConfigModel<emFileManConfigData> {
        &mut self.config_model
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManConfig -- --test-threads=1`
Expected: All 6 tests pass (3 existing + 3 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManConfig.rs
git commit -m "feat(emFileMan): add emFileManConfig model wrapper with Acquire"
```

---

### Task 4: emFileManTheme Model Wrapper + ImageFileRec

**Files:**
- Modify: `crates/emfileman/src/emFileManTheme.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emFileManTheme.rs`:

```rust
    #[test]
    fn image_file_rec_empty_path_returns_fallback() {
        let rec = ImageFileRec::new("".to_string(), PathBuf::new());
        let img = rec.GetImage();
        // Fallback is 1x1 transparent
        assert_eq!(img.GetWidth(), 1);
        assert_eq!(img.GetHeight(), 1);
    }

    #[test]
    fn image_file_rec_caches_result() {
        let rec = ImageFileRec::new("".to_string(), PathBuf::new());
        let _img1 = rec.GetImage();
        let _img2 = rec.GetImage(); // should not panic or re-load
    }

    #[test]
    fn theme_model_acquire_same_name_returns_same_instance() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let t1 = emFileManTheme::Acquire(&ctx, "test_theme");
        let t2 = emFileManTheme::Acquire(&ctx, "test_theme");
        assert!(Rc::ptr_eq(&t1, &t2));
    }

    #[test]
    fn theme_model_field_access() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let theme = emFileManTheme::Acquire(&ctx, "default");
        let theme = theme.borrow();
        // Default Height is 0.0
        assert_eq!(theme.GetRec().Height, 0.0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManTheme -- --test-threads=1`
Expected: FAIL — `ImageFileRec` and `emFileManTheme` types not found

- [ ] **Step 3: Implement ImageFileRec and emFileManTheme**

Add the following to `crates/emfileman/src/emFileManTheme.rs`, after the `Record` impl for `emFileManThemeData` but before `#[cfg(test)]`:

```rust
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emConfigModel::emConfigModel;
use emcore::emImage::emImage;
use emcore::emImageFile::load_image_from_file;
use emcore::emSignal::SignalId;

/// Lazy-loading image record. Stores a path string and caches the loaded image.
///
/// DIVERGED: C++ `ImageFileRec` is a nested class extending emStringRec +
/// emRecListener with async emImageFileModel loading. This version loads
/// synchronously via load_image_from_file (TGA only). Sufficient for small
/// theme border images.
pub struct ImageFileRec {
    path: String,
    theme_dir: PathBuf,
    cached: RefCell<Option<emImage>>,
}

impl ImageFileRec {
    pub fn new(path: String, theme_dir: PathBuf) -> Self {
        Self {
            path,
            theme_dir,
            cached: RefCell::new(None),
        }
    }

    /// Get the loaded image. Loads on first call, caches thereafter.
    /// Returns a 1x1 transparent fallback if path is empty or load fails.
    pub fn GetImage(&self) -> Ref<'_, emImage> {
        {
            let mut cached = self.cached.borrow_mut();
            if cached.is_none() {
                let image = if self.path.is_empty() {
                    None
                } else {
                    let full_path = self.theme_dir.join(&self.path);
                    load_image_from_file(&full_path)
                };
                *cached = Some(image.unwrap_or_else(|| emImage::new(1, 1, 4)));
            }
        }
        Ref::map(self.cached.borrow(), |opt| opt.as_ref().expect("just set"))
    }

    pub fn GetPath(&self) -> &str {
        &self.path
    }
}

/// Theme model wrapper.
/// Port of C++ `emFileManTheme` (extends emConfigModel).
pub struct emFileManTheme {
    config_model: emConfigModel<emFileManThemeData>,
    outer_border_img: ImageFileRec,
    file_inner_border_img: ImageFileRec,
    dir_inner_border_img: ImageFileRec,
    alt_inner_border_img: ImageFileRec,
}

impl emFileManTheme {
    /// Acquire a theme by name.
    /// Port of C++ `emFileManTheme::Acquire`.
    pub fn Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>(name, || {
            let signal_id = SignalId::default();
            let theme_dir = GetThemesDirPath().unwrap_or_default();
            let path = theme_dir
                .join(format!("{}{}", name, THEME_FILE_ENDING));
            let data = emFileManThemeData::default();
            let outer_border_img =
                ImageFileRec::new(data.OuterBorderImg.clone(), theme_dir.clone());
            let file_inner_border_img =
                ImageFileRec::new(data.FileInnerBorderImg.clone(), theme_dir.clone());
            let dir_inner_border_img =
                ImageFileRec::new(data.DirInnerBorderImg.clone(), theme_dir.clone());
            let alt_inner_border_img =
                ImageFileRec::new(data.AltInnerBorderImg.clone(), theme_dir);
            Self {
                config_model: emConfigModel::new(data, path, signal_id),
                outer_border_img,
                file_inner_border_img,
                dir_inner_border_img,
                alt_inner_border_img,
            }
        })
    }

    pub fn GetFormatName(&self) -> &str {
        "emFileManTheme"
    }

    pub fn GetRec(&self) -> &emFileManThemeData {
        self.config_model.GetRec()
    }

    pub fn GetChangeSignal(&self) -> SignalId {
        self.config_model.GetChangeSignal()
    }

    pub fn GetOuterBorderImage(&self) -> Ref<'_, emImage> {
        self.outer_border_img.GetImage()
    }

    pub fn GetFileInnerBorderImage(&self) -> Ref<'_, emImage> {
        self.file_inner_border_img.GetImage()
    }

    pub fn GetDirInnerBorderImage(&self) -> Ref<'_, emImage> {
        self.dir_inner_border_img.GetImage()
    }

    pub fn GetAltInnerBorderImage(&self) -> Ref<'_, emImage> {
        self.alt_inner_border_img.GetImage()
    }

    /// Reinitialize image records after config data changes (e.g., after load).
    pub(crate) fn refresh_image_recs(&mut self) {
        let theme_dir = self
            .config_model
            .GetInstallPath()
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        let data = self.config_model.GetRec();
        self.outer_border_img =
            ImageFileRec::new(data.OuterBorderImg.clone(), theme_dir.clone());
        self.file_inner_border_img =
            ImageFileRec::new(data.FileInnerBorderImg.clone(), theme_dir.clone());
        self.dir_inner_border_img =
            ImageFileRec::new(data.DirInnerBorderImg.clone(), theme_dir.clone());
        self.alt_inner_border_img =
            ImageFileRec::new(data.AltInnerBorderImg.clone(), theme_dir);
    }

    pub(crate) fn config_model_mut(&mut self) -> &mut emConfigModel<emFileManThemeData> {
        &mut self.config_model
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManTheme -- --test-threads=1`
Expected: All 10 tests pass (6 existing + 4 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManTheme.rs
git commit -m "feat(emFileMan): add emFileManTheme model wrapper with ImageFileRec"
```

---

### Task 5: emFileManThemeNames Filesystem Discovery + Acquire

**Files:**
- Modify: `crates/emfileman/src/emFileManThemeNames.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emFileManThemeNames.rs`:

```rust
    #[test]
    fn acquire_singleton() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let t1 = emFileManThemeNames::Acquire(&ctx);
        let t2 = emFileManThemeNames::Acquire(&ctx);
        assert!(Rc::ptr_eq(&t1, &t2));
    }

    #[test]
    fn discover_from_directory() {
        // Create temp theme files
        let dir = std::env::temp_dir().join("emcore_test_themes_disc");
        let _ = std::fs::create_dir_all(&dir);

        // Minimal theme rec file with DisplayName, DisplayIcon, Height
        let content = "\
emFileManTheme\n\
DisplayName = \"TestStyle\"\n\
DisplayIcon = \"icon.tga\"\n\
Height = 0.6\n";
        std::fs::write(
            dir.join("Test1.emFileManTheme"),
            content,
        )
        .expect("write");

        let names = discover_themes_from_dir(&dir);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert!(names.IsExistingThemeName("Test1"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn change_generation_starts_at_zero() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let names = emFileManThemeNames::Acquire(&ctx);
        let names = names.borrow();
        assert_eq!(names.GetChangeGeneration(), 0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManThemeNames -- --test-threads=1`
Expected: FAIL — `Acquire`, `discover_themes_from_dir`, `GetChangeGeneration` not found

- [ ] **Step 3: Implement discovery and Acquire**

Add these imports at the top of `crates/emfileman/src/emFileManThemeNames.rs`:

```rust
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use crate::emFileManTheme::{GetThemesDirPath, THEME_FILE_ENDING};
```

Add after the existing `impl emFileManThemeNames` block but before `#[cfg(test)]`:

```rust
/// Scan a directory for .emFileManTheme files and extract DisplayName,
/// DisplayIcon, Height from each to build a theme catalog.
pub fn discover_themes_from_dir(dir: &Path) -> emFileManThemeNames {
    let mut entries: Vec<(&str, &str, &str, f64)> = Vec::new();
    // We need owned strings, so collect first then build refs
    let mut owned: Vec<(String, String, String, f64)> = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if !name_str.ends_with(THEME_FILE_ENDING) {
                continue;
            }
            let theme_name = name_str
                .strip_suffix(THEME_FILE_ENDING)
                .unwrap_or(&name_str)
                .to_string();

            // Parse just the 3 fields we need
            let Ok(content) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let mut display_name = String::new();
            let mut display_icon = String::new();
            let mut height: f64 = 0.0;

            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(val) = trimmed.strip_prefix("DisplayName") {
                    if let Some(val) = val.trim_start().strip_prefix('=') {
                        let val = val.trim().trim_matches('"');
                        display_name = val.to_string();
                    }
                } else if let Some(val) = trimmed.strip_prefix("DisplayIcon") {
                    if let Some(val) = val.trim_start().strip_prefix('=') {
                        let val = val.trim().trim_matches('"');
                        display_icon = val.to_string();
                    }
                } else if let Some(val) = trimmed.strip_prefix("Height") {
                    if let Some(val) = val.trim_start().strip_prefix('=') {
                        if let Ok(h) = val.trim().parse::<f64>() {
                            height = h;
                        }
                    }
                }
            }

            owned.push((theme_name, display_name, display_icon, height));
        }
    }

    let refs: Vec<(&str, &str, &str, f64)> = owned
        .iter()
        .map(|(n, dn, di, h)| (n.as_str(), dn.as_str(), di.as_str(), *h))
        .collect();
    emFileManThemeNames::from_themes(&refs)
}

impl emFileManThemeNames {
    /// Acquire the singleton theme names catalog.
    /// Port of C++ `emFileManThemeNames::Acquire`.
    pub fn Acquire(ctx: &Rc<emcore::emContext::emContext>) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>("", || {
            let theme_dir = GetThemesDirPath().unwrap_or_default();
            let mut catalog = discover_themes_from_dir(&theme_dir);
            catalog.change_generation = Rc::new(Cell::new(0));
            catalog.theme_dir_mtime = dir_mtime(&theme_dir);
            catalog.theme_dir = theme_dir;
            catalog
        })
    }

    pub fn GetChangeGeneration(&self) -> u64 {
        self.change_generation.get()
    }

    /// Check if theme directory changed and rescan if so.
    /// Port of C++ `emFileManThemeNames::Cycle`.
    pub fn Cycle(&mut self) -> bool {
        let current_mtime = dir_mtime(&self.theme_dir);
        if current_mtime != self.theme_dir_mtime {
            self.theme_dir_mtime = current_mtime;
            let new_catalog = discover_themes_from_dir(&self.theme_dir);
            self.styles = new_catalog.styles;
            self.name_to_packed_index = new_catalog.name_to_packed_index;
            self.change_generation
                .set(self.change_generation.get() + 1);
            return true;
        }
        false
    }
}

fn dir_mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "time error"))
        })
        .unwrap_or(0)
}
```

Also add these fields to the `emFileManThemeNames` struct:

```rust
pub struct emFileManThemeNames {
    styles: Vec<ThemeStyle>,
    name_to_packed_index: BTreeMap<String, (usize, usize)>,
    change_generation: Rc<Cell<u64>>,
    theme_dir: std::path::PathBuf,
    theme_dir_mtime: u64,
}
```

Update `from_themes()` to initialize the new fields:

```rust
pub fn from_themes(entries: &[(&str, &str, &str, f64)]) -> Self {
    // ... existing logic unchanged ...
    Self {
        styles,
        name_to_packed_index,
        change_generation: Rc::new(Cell::new(0)),
        theme_dir: std::path::PathBuf::new(),
        theme_dir_mtime: 0,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManThemeNames -- --test-threads=1`
Expected: All 10 tests pass (7 existing + 3 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManThemeNames.rs
git commit -m "feat(emFileMan): add emFileManThemeNames filesystem discovery and Acquire"
```

---

### Task 6: emFileLinkModel Wrapper

**Files:**
- Modify: `crates/emfileman/src/emFileLinkModel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emFileLinkModel.rs`:

```rust
    #[test]
    fn model_acquire_returns_same_instance() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let m1 = emFileLinkModel::Acquire(&ctx, "/tmp/test.emFileLink", true);
        let m2 = emFileLinkModel::Acquire(&ctx, "/tmp/test.emFileLink", true);
        assert!(Rc::ptr_eq(&m1, &m2));
    }

    #[test]
    fn model_acquire_different_names_different_instances() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let m1 = emFileLinkModel::Acquire(&ctx, "/tmp/a.emFileLink", true);
        let m2 = emFileLinkModel::Acquire(&ctx, "/tmp/b.emFileLink", true);
        assert!(!Rc::ptr_eq(&m1, &m2));
    }

    #[test]
    fn model_get_format_name() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let m = emFileLinkModel::Acquire(&ctx, "/tmp/test.emFileLink", true);
        assert_eq!(m.borrow().GetFormatName(), "emFileLink");
    }

    #[test]
    fn model_delegates_to_data() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let m = emFileLinkModel::Acquire(&ctx, "/tmp/test.emFileLink", true);
        let m = m.borrow();
        assert_eq!(m.GetBasePathType(), BasePathType::None);
        assert_eq!(m.GetBasePathProject(), "");
        assert_eq!(m.GetPath(), "");
        assert!(!m.GetHaveDirEntry());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileLinkModel -- --test-threads=1`
Expected: FAIL — `emFileLinkModel` type not found

- [ ] **Step 3: Implement emFileLinkModel**

Add to `crates/emfileman/src/emFileLinkModel.rs`, after the `Record` impl for `emFileLinkData` but before `#[cfg(test)]`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emFileModel::FileState;
use emcore::emRecFileModel::emRecFileModel;

/// Model wrapper for .emFileLink files.
/// Port of C++ `emFileLinkModel` (extends emRecFileModel).
pub struct emFileLinkModel {
    rec_model: emRecFileModel<emFileLinkData>,
}

impl emFileLinkModel {
    /// Acquire a link model by file path.
    /// Port of C++ `emFileLinkModel::Acquire`.
    pub fn Acquire(
        ctx: &Rc<emContext>,
        name: &str,
        _common: bool,
    ) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>(name, || Self {
            rec_model: emRecFileModel::new(PathBuf::from(name)),
        })
    }

    pub fn GetFormatName(&self) -> &str {
        "emFileLink"
    }

    pub fn GetFileState(&self) -> &FileState {
        self.rec_model.GetFileState()
    }

    /// Get the resolved full path of the link target.
    pub fn GetFullPath(&self) -> String {
        let data = self.rec_model.GetMap();
        let file_path = self.rec_model.path().to_string_lossy();
        data.GetFullPath(&file_path)
    }

    pub fn GetBasePathType(&self) -> BasePathType {
        self.rec_model.GetMap().base_path_type
    }

    pub fn GetBasePathProject(&self) -> &str {
        &self.rec_model.GetMap().base_path_project
    }

    pub fn GetPath(&self) -> &str {
        &self.rec_model.GetMap().path
    }

    pub fn GetHaveDirEntry(&self) -> bool {
        self.rec_model.GetMap().have_dir_entry
    }

    pub(crate) fn rec_model_mut(&mut self) -> &mut emRecFileModel<emFileLinkData> {
        &mut self.rec_model
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileLinkModel -- --test-threads=1`
Expected: All 8 tests pass (4 existing + 4 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileLinkModel.rs
git commit -m "feat(emFileMan): add emFileLinkModel wrapper with Acquire"
```

---

### Phase 2 Gate

- [ ] **Run full gate check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass.

---

## Phase 3 — Layer 1 Models (depend on Layer 0)

### Task 7: emDirModel Wrapper

**Files:**
- Modify: `crates/emfileman/src/emDirModel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emDirModel.rs`:

```rust
    #[test]
    fn model_acquire_same_path_returns_same() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let m1 = emDirModel::Acquire(&ctx, "/tmp");
        let m2 = emDirModel::Acquire(&ctx, "/tmp");
        assert!(Rc::ptr_eq(&m1, &m2));
    }

    #[test]
    fn model_delegates_entry_accessors() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let model = emDirModel::Acquire(&ctx, "/tmp");
        let model = model.borrow();
        // Initial state: no entries
        assert_eq!(model.GetEntryCount(), 0);
        assert!(model.GetEntryIndex("anything").is_none());
    }

    #[test]
    fn model_file_model_ops_wiring() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let model = emDirModel::Acquire(&ctx, "/tmp");
        let mut model = model.borrow_mut();
        // FileModelOps: reset should not panic
        model.reset_data();
        assert_eq!(model.GetEntryCount(), 0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emDirModel -- --test-threads=1`
Expected: FAIL — `emDirModel` type not found

- [ ] **Step 3: Implement emDirModel**

Add to `crates/emfileman/src/emDirModel.rs`, after the `Default` impl for `emDirModelData` but before `#[cfg(test)]`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emFileModel::FileModelOps;

/// Directory model wrapper.
/// Port of C++ `emDirModel` (extends emFileModel).
///
/// DIVERGED: Does not compose emFileModel<T> because emFileModel requires
/// SignalId and update_signal from the scheduler, which is not available via
/// emContext::acquire(). Instead, wraps emDirModelData directly and implements
/// FileModelOps. The panel layer will drive the loading state machine by calling
/// these methods in its Cycle.
pub struct emDirModel {
    data: emDirModelData,
    path: String,
}

impl emDirModel {
    /// Acquire a directory model by path.
    /// Port of C++ `emDirModel::Acquire`.
    pub fn Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>(name, || Self {
            data: emDirModelData::new(),
            path: name.to_string(),
        })
    }

    pub fn GetEntryCount(&self) -> usize {
        self.data.GetEntryCount()
    }

    pub fn GetEntry(&self, index: usize) -> &emDirEntry {
        self.data.GetEntry(index)
    }

    pub fn GetEntryIndex(&self, name: &str) -> Option<usize> {
        self.data.GetEntryIndex(name)
    }

    pub fn IsOutOfDate(&self) -> bool {
        self.data.IsOutOfDate()
    }

    pub fn name_count(&self) -> usize {
        self.data.name_count()
    }

    /// Get the directory path this model loads.
    pub fn GetFilePath(&self) -> &str {
        &self.path
    }

    // --- FileModelOps delegation ---

    pub fn reset_data(&mut self) {
        self.data.reset_data();
    }

    pub fn try_start_loading(&mut self) -> Result<(), String> {
        self.data.try_start_loading_from(&self.path)
    }

    pub fn try_continue_loading(&mut self) -> Result<bool, String> {
        self.data.try_continue_loading()
    }

    pub fn quit_loading(&mut self) {
        self.data.quit_loading();
    }

    pub fn calc_memory_need(&self) -> u64 {
        self.data.calc_memory_need()
    }

    pub fn calc_file_progress(&self) -> f64 {
        self.data.calc_file_progress()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emDirModel -- --test-threads=1`
Expected: All 11 tests pass (8 existing + 3 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emDirModel.rs
git commit -m "feat(emFileMan): add emDirModel wrapper with Acquire and FileModelOps"
```

---

### Task 8: emFileManViewConfig Model

**Files:**
- Modify: `crates/emfileman/src/emFileManViewConfig.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emFileManViewConfig.rs`:

```rust
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn view_config_acquire_returns_same() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let v1 = emFileManViewConfig::Acquire(&ctx);
        let v2 = emFileManViewConfig::Acquire(&ctx);
        assert!(Rc::ptr_eq(&v1, &v2));
    }

    #[test]
    fn view_config_setters_bump_generation() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let vc = emFileManViewConfig::Acquire(&ctx);
        let gen0 = vc.borrow().GetChangeSignal();
        vc.borrow_mut().SetSortCriterion(SortCriterion::BySize);
        let gen1 = vc.borrow().GetChangeSignal();
        assert!(gen1 > gen0);
    }

    #[test]
    fn view_config_compare_dir_entries_method() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let vc = emFileManViewConfig::Acquire(&ctx);
        let vc = vc.borrow();

        let e1 = emDirEntry::Load("/tmp", "aaa").unwrap();
        let e2 = emDirEntry::Load("/tmp", "zzz").unwrap();
        assert!(vc.CompareDirEntries(&e1, &e2) < 0);
    }

    #[test]
    fn view_config_is_unsaved_after_change() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let vc = emFileManViewConfig::Acquire(&ctx);
        assert!(!vc.borrow().IsUnsaved());
        vc.borrow_mut().SetShowHiddenFiles(true);
        assert!(vc.borrow().IsUnsaved());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManViewConfig -- --test-threads=1`
Expected: FAIL — `emFileManViewConfig` type not found

- [ ] **Step 3: Implement emFileManViewConfig**

Add imports at top of `crates/emfileman/src/emFileManViewConfig.rs`:

```rust
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::emFileManConfig::{emFileManConfig, emFileManConfigData};
use crate::emFileManTheme::emFileManTheme;
use crate::emFileManThemeNames::emFileManThemeNames;
```

Add after the existing free functions and before `#[cfg(test)]`:

```rust
/// Per-view configuration bridge.
/// Port of C++ `emFileManViewConfig` (extends emModel).
///
/// Holds local copies of config fields. Setters bump change_generation
/// and optionally write back to the global emFileManConfig if autosave is on.
pub struct emFileManViewConfig {
    ctx: Rc<emcore::emContext::emContext>,
    config: Rc<RefCell<emFileManConfig>>,
    theme: Rc<RefCell<emFileManTheme>>,
    theme_names: Rc<RefCell<emFileManThemeNames>>,
    sort_criterion: SortCriterion,
    name_sorting_style: NameSortingStyle,
    sort_directories_first: bool,
    show_hidden_files: bool,
    theme_name: String,
    autosave: bool,
    change_generation: Rc<Cell<u64>>,
    initial_sort_criterion: SortCriterion,
    initial_name_sorting_style: NameSortingStyle,
    initial_sort_directories_first: bool,
    initial_show_hidden_files: bool,
    initial_theme_name: String,
    initial_autosave: bool,
}

impl emFileManViewConfig {
    /// Acquire per-view config.
    /// Port of C++ `emFileManViewConfig::Acquire`.
    pub fn Acquire(ctx: &Rc<emcore::emContext::emContext>) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>("", || {
            let config = emFileManConfig::Acquire(ctx);
            let theme_names = emFileManThemeNames::Acquire(ctx);

            let (sc, nss, sdf, shf, tn, auto) = {
                let c = config.borrow();
                (
                    c.GetSortCriterion(),
                    c.GetNameSortingStyle(),
                    c.GetSortDirectoriesFirst(),
                    c.GetShowHiddenFiles(),
                    c.GetThemeName().to_string(),
                    c.GetAutosave(),
                )
            };

            let theme = emFileManTheme::Acquire(ctx, if tn.is_empty() { "default" } else { &tn });

            Self {
                ctx: Rc::clone(ctx),
                config,
                theme,
                theme_names,
                sort_criterion: sc,
                name_sorting_style: nss,
                sort_directories_first: sdf,
                show_hidden_files: shf,
                theme_name: tn.clone(),
                autosave: auto,
                change_generation: Rc::new(Cell::new(0)),
                initial_sort_criterion: sc,
                initial_name_sorting_style: nss,
                initial_sort_directories_first: sdf,
                initial_show_hidden_files: shf,
                initial_theme_name: tn,
                initial_autosave: auto,
            }
        })
    }

    fn bump_generation(&self) {
        self.change_generation
            .set(self.change_generation.get() + 1);
    }

    fn write_back_if_autosave(&self) {
        if self.autosave {
            let mut cfg = self.config.borrow_mut();
            cfg.SetSortCriterion(self.sort_criterion);
            cfg.SetNameSortingStyle(self.name_sorting_style);
            cfg.SetSortDirectoriesFirst(self.sort_directories_first);
            cfg.SetShowHiddenFiles(self.show_hidden_files);
            cfg.SetThemeName(&self.theme_name);
            cfg.SetAutosave(self.autosave);
        }
    }

    // --- Getters ---

    pub fn GetChangeSignal(&self) -> u64 {
        self.change_generation.get()
    }

    pub fn GetSortCriterion(&self) -> SortCriterion {
        self.sort_criterion
    }

    pub fn GetNameSortingStyle(&self) -> NameSortingStyle {
        self.name_sorting_style
    }

    pub fn GetSortDirectoriesFirst(&self) -> bool {
        self.sort_directories_first
    }

    pub fn GetShowHiddenFiles(&self) -> bool {
        self.show_hidden_files
    }

    pub fn GetThemeName(&self) -> &str {
        &self.theme_name
    }

    pub fn GetAutosave(&self) -> bool {
        self.autosave
    }

    pub fn GetTheme(&self) -> std::cell::Ref<'_, emFileManTheme> {
        self.theme.borrow()
    }

    // --- Setters ---

    pub fn SetSortCriterion(&mut self, sc: SortCriterion) {
        if self.sort_criterion != sc {
            self.sort_criterion = sc;
            self.bump_generation();
            self.write_back_if_autosave();
        }
    }

    pub fn SetNameSortingStyle(&mut self, nss: NameSortingStyle) {
        if self.name_sorting_style != nss {
            self.name_sorting_style = nss;
            self.bump_generation();
            self.write_back_if_autosave();
        }
    }

    pub fn SetSortDirectoriesFirst(&mut self, b: bool) {
        if self.sort_directories_first != b {
            self.sort_directories_first = b;
            self.bump_generation();
            self.write_back_if_autosave();
        }
    }

    pub fn SetShowHiddenFiles(&mut self, b: bool) {
        if self.show_hidden_files != b {
            self.show_hidden_files = b;
            self.bump_generation();
            self.write_back_if_autosave();
        }
    }

    pub fn SetThemeName(&mut self, name: &str) {
        if self.theme_name != name {
            self.theme_name = name.to_string();
            self.theme = emFileManTheme::Acquire(&self.ctx, name);
            self.bump_generation();
            self.write_back_if_autosave();
        }
    }

    pub fn SetAutosave(&mut self, b: bool) {
        if self.autosave != b {
            self.autosave = b;
            self.bump_generation();
            self.write_back_if_autosave();
        }
    }

    // --- Comparison ---

    pub fn CompareDirEntries(&self, e1: &emDirEntry, e2: &emDirEntry) -> i32 {
        let cfg = SortConfig {
            sort_criterion: self.sort_criterion,
            name_sorting_style: self.name_sorting_style,
            sort_directories_first: self.sort_directories_first,
        };
        CompareDirEntries(e1, e2, &cfg)
    }

    // --- Save state ---

    pub fn IsUnsaved(&self) -> bool {
        self.sort_criterion != self.initial_sort_criterion
            || self.name_sorting_style != self.initial_name_sorting_style
            || self.sort_directories_first != self.initial_sort_directories_first
            || self.show_hidden_files != self.initial_show_hidden_files
            || self.theme_name != self.initial_theme_name
            || self.autosave != self.initial_autosave
    }

    pub fn SaveAsDefault(&mut self) {
        let mut cfg = self.config.borrow_mut();
        cfg.SetSortCriterion(self.sort_criterion);
        cfg.SetNameSortingStyle(self.name_sorting_style);
        cfg.SetSortDirectoriesFirst(self.sort_directories_first);
        cfg.SetShowHiddenFiles(self.show_hidden_files);
        cfg.SetThemeName(&self.theme_name);
        cfg.SetAutosave(self.autosave);
        // Update initial state to match
        self.initial_sort_criterion = self.sort_criterion;
        self.initial_name_sorting_style = self.name_sorting_style;
        self.initial_sort_directories_first = self.sort_directories_first;
        self.initial_show_hidden_files = self.show_hidden_files;
        self.initial_theme_name = self.theme_name.clone();
        self.initial_autosave = self.autosave;
    }
}
```

**Important:** The `SetThemeName` method has a design issue — it can't re-acquire from emContext because the ViewConfig doesn't store the context. Fix this by storing the context:

Add `ctx: Rc<emcore::emContext::emContext>` field to the struct, set it in `Acquire`, and use it in `SetThemeName`:

```rust
pub fn SetThemeName(&mut self, name: &str) {
    if self.theme_name != name {
        self.theme_name = name.to_string();
        self.theme = emFileManTheme::Acquire(&self.ctx, name);
        self.bump_generation();
        self.write_back_if_autosave();
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManViewConfig -- --test-threads=1`
Expected: All 12 tests pass (8 existing + 4 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManViewConfig.rs
git commit -m "feat(emFileMan): add emFileManViewConfig model with generation counter"
```

---

### Task 9: emFileManModel — Full Model Wrapper

This is the largest task. It adds the model struct, command tree loading, RunCommand, IPC wiring, hotkey search, and sorted selection helpers.

**Files:**
- Modify: `crates/emfileman/src/emFileManModel.rs`

- [ ] **Step 1: Write tests for model wrapper basics**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emFileManModel.rs`:

```rust
    fn make_test_ctx() -> Rc<emcore::emContext::emContext> {
        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        emcore::emContext::emContext::NewRootWithScheduler(sched)
    }

    #[test]
    fn model_acquire_singleton() {
        let ctx = make_test_ctx();
        let m1 = emFileManModel::Acquire(&ctx);
        let m2 = emFileManModel::Acquire(&ctx);
        assert!(Rc::ptr_eq(&m1, &m2));
    }

    #[test]
    fn model_ipc_server_name_is_set() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        let name = model.borrow().GetMiniIpcServerName().to_string();
        assert!(name.starts_with("eaglemode-rs-fm-"));
        assert!(!name.is_empty());
    }

    #[test]
    fn model_selection_bumps_generation() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        let gen0 = model.borrow().GetSelectionSignal();
        model.borrow_mut().SelectAsSource("/tmp/a");
        let gen1 = model.borrow().GetSelectionSignal();
        assert!(gen1 > gen0);
    }

    #[test]
    fn model_shift_tgt_sel_path() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        assert_eq!(model.borrow().GetShiftTgtSelPath(), "");
        model
            .borrow_mut()
            .SetShiftTgtSelPath("/home/user/docs");
        assert_eq!(model.borrow().GetShiftTgtSelPath(), "/home/user/docs");
    }

    #[test]
    fn model_get_command_root_initially_none() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        assert!(model.borrow().GetCommandRoot().is_none());
    }

    #[test]
    fn model_get_command_by_path() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        let mut model = model.borrow_mut();

        // Set up a command tree manually
        let mut root = CommandNode::default();
        root.command_type = CommandType::Group;
        let mut child = CommandNode::default();
        child.cmd_path = "/cmds/test.sh".to_string();
        child.command_type = CommandType::Command;
        root.children.push(child);
        model.set_command_root(root);

        assert!(model.GetCommand("/cmds/test.sh").is_some());
        assert!(model.GetCommand("/cmds/nonexistent.sh").is_none());
    }

    #[test]
    fn model_selection_to_clipboard() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        {
            let mut m = model.borrow_mut();
            m.SelectAsSource("/home/user/a.txt");
            m.SelectAsSource("/home/user/b.txt");
        }
        let m = model.borrow();
        let clip = m.SelectionToClipboard(true, false);
        assert!(clip.contains("/home/user/a.txt"));
        assert!(clip.contains("/home/user/b.txt"));

        let clip_names = m.SelectionToClipboard(true, true);
        assert!(clip_names.contains("a.txt"));
        assert!(!clip_names.contains("/home/user/"));
    }

    #[test]
    fn model_search_hotkey_command() {
        let ctx = make_test_ctx();
        let model = emFileManModel::Acquire(&ctx);
        let mut model = model.borrow_mut();

        let mut root = CommandNode::default();
        root.command_type = CommandType::Group;
        let mut child = CommandNode::default();
        child.cmd_path = "/cmds/open.sh".to_string();
        child.command_type = CommandType::Command;
        child.hotkey = "Ctrl+O".to_string();
        root.children.push(child);
        model.set_command_root(root);

        assert!(model.SearchHotkeyCommand("Ctrl+O").is_some());
        assert!(model.SearchHotkeyCommand("Ctrl+X").is_none());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emfileman --lib emFileManModel -- --test-threads=1`
Expected: FAIL — `emFileManModel`, `GetSelectionSignal`, etc. not found

- [ ] **Step 3: Implement emFileManModel**

Add imports at top of `crates/emfileman/src/emFileManModel.rs`:

```rust
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emProcess;
```

Add after the `Default` impl for `SelectionManager` but before `#[cfg(test)]`:

```rust
/// Central file manager model.
/// Port of C++ `emFileManModel` (extends emModel).
///
/// Holds selections, command tree, and IPC server. The IPC server is created
/// and started during Acquire, matching C++ which constructs it in the model
/// constructor.
pub struct emFileManModel {
    selection: SelectionManager,
    command_root: Option<CommandNode>,
    shift_tgt_sel_path: String,
    command_run_id: u64,
    selection_generation: Rc<Cell<u64>>,
    commands_generation: Rc<Cell<u64>>,
    ipc_server_name: String,
    /// IPC server. None only if context had no scheduler (test contexts).
    ipc_server: Option<emcore::emMiniIpc::emMiniIpcServer>,
    /// Scheduler reference for IPC server lifecycle management.
    scheduler: Option<Rc<RefCell<emcore::emScheduler::EngineScheduler>>>,
}

impl emFileManModel {
    /// Acquire the singleton file manager model.
    /// Port of C++ `emFileManModel::Acquire`.
    ///
    /// If the context has a scheduler (production), creates and starts the
    /// IPC server immediately. If no scheduler (tests), IPC server is None.
    pub fn Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>> {
        let scheduler = ctx.GetScheduler();
        ctx.acquire::<Self>("", || {
            let ipc_server_name = format!("eaglemode-rs-fm-{}", std::process::id());

            // Create and start IPC server if scheduler available
            let ipc_server = scheduler.as_ref().map(|sched| {
                let mut server = emcore::emMiniIpc::emMiniIpcServer::new(
                    &mut sched.borrow_mut(),
                    Box::new(|_args: &[String]| {
                        // IPC callback wired in post-construction via set_ipc_callback
                    }),
                );
                let _ = server.StartServing(
                    &mut sched.borrow_mut(),
                    Some(&ipc_server_name),
                );
                server
            });

            Self {
                selection: SelectionManager::new(),
                command_root: None,
                shift_tgt_sel_path: String::new(),
                command_run_id: 0,
                selection_generation: Rc::new(Cell::new(0)),
                commands_generation: Rc::new(Cell::new(0)),
                ipc_server_name,
                ipc_server,
                scheduler: scheduler.clone(),
            }
        })
    }

    fn bump_selection_generation(&self) {
        self.selection_generation
            .set(self.selection_generation.get() + 1);
    }

    // --- Signals ---

    pub fn GetSelectionSignal(&self) -> u64 {
        self.selection_generation.get()
    }

    pub fn GetCommandsSignal(&self) -> u64 {
        self.commands_generation.get()
    }

    // --- Selection delegation ---

    pub fn GetSourceSelectionCount(&self) -> usize {
        self.selection.GetSourceSelectionCount()
    }

    pub fn GetSourceSelection(&self, index: usize) -> &str {
        self.selection.GetSourceSelection(index)
    }

    pub fn IsSelectedAsSource(&self, path: &str) -> bool {
        self.selection.IsSelectedAsSource(path)
    }

    pub fn SelectAsSource(&mut self, path: &str) {
        self.selection.SelectAsSource(path);
        self.bump_selection_generation();
    }

    pub fn DeselectAsSource(&mut self, path: &str) {
        self.selection.DeselectAsSource(path);
        self.bump_selection_generation();
    }

    pub fn ClearSourceSelection(&mut self) {
        self.selection.ClearSourceSelection();
        self.bump_selection_generation();
    }

    pub fn GetTargetSelectionCount(&self) -> usize {
        self.selection.GetTargetSelectionCount()
    }

    pub fn GetTargetSelection(&self, index: usize) -> &str {
        self.selection.GetTargetSelection(index)
    }

    pub fn IsSelectedAsTarget(&self, path: &str) -> bool {
        self.selection.IsSelectedAsTarget(path)
    }

    pub fn SelectAsTarget(&mut self, path: &str) {
        self.selection.SelectAsTarget(path);
        self.bump_selection_generation();
    }

    pub fn DeselectAsTarget(&mut self, path: &str) {
        self.selection.DeselectAsTarget(path);
        self.bump_selection_generation();
    }

    pub fn ClearTargetSelection(&mut self) {
        self.selection.ClearTargetSelection();
        self.bump_selection_generation();
    }

    pub fn SwapSelection(&mut self) {
        self.selection.SwapSelection();
        self.bump_selection_generation();
    }

    pub fn IsAnySelectionInDirTree(&self, dir_path: &str) -> bool {
        self.selection.IsAnySelectionInDirTree(dir_path)
    }

    pub fn UpdateSelection(&mut self) {
        let src_before = self.selection.GetSourceSelectionCount();
        let tgt_before = self.selection.GetTargetSelectionCount();
        self.selection.UpdateSelection();
        let src_after = self.selection.GetSourceSelectionCount();
        let tgt_after = self.selection.GetTargetSelectionCount();
        if src_after != src_before || tgt_after != tgt_before {
            self.bump_selection_generation();
        }
    }

    // --- Shift target ---

    pub fn GetShiftTgtSelPath(&self) -> &str {
        &self.shift_tgt_sel_path
    }

    pub fn SetShiftTgtSelPath(&mut self, path: &str) {
        self.shift_tgt_sel_path = path.to_string();
    }

    // --- IPC ---

    pub fn GetMiniIpcServerName(&self) -> &str {
        &self.ipc_server_name
    }

    pub fn GetCommandRunId(&self) -> String {
        self.selection.GetCommandRunId()
    }

    /// Handle an IPC message from a spawned command process.
    pub fn HandleIpcMessage(&mut self, args: &[&str]) {
        self.selection.handle_ipc_message(args);
        self.bump_selection_generation();
    }

    // --- Clipboard ---

    /// Format selections as newline-separated text for clipboard.
    pub fn SelectionToClipboard(&self, source: bool, names_only: bool) -> String {
        let count = if source {
            self.selection.GetSourceSelectionCount()
        } else {
            self.selection.GetTargetSelectionCount()
        };

        let mut lines = Vec::with_capacity(count);
        for i in 0..count {
            let path = if source {
                self.selection.GetSourceSelection(i)
            } else {
                self.selection.GetTargetSelection(i)
            };
            if names_only {
                let name = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                lines.push(name);
            } else {
                lines.push(path.to_string());
            }
        }
        lines.join("\n")
    }

    // --- Command tree ---

    pub fn GetCommandRoot(&self) -> Option<&CommandNode> {
        self.command_root.as_ref()
    }

    /// Look up a command by its cmd_path via DFS.
    pub fn GetCommand(&self, cmd_path: &str) -> Option<&CommandNode> {
        self.command_root
            .as_ref()
            .and_then(|root| find_command_by_path(root, cmd_path))
    }

    /// Search for the best default command for a file path.
    pub fn SearchDefaultCommandFor(&self, file_path: &str) -> Option<&CommandNode> {
        self.command_root
            .as_ref()
            .and_then(|root| super::emFileManModel::SearchDefaultCommandFor(root, file_path))
    }

    /// Search for a command matching a hotkey string.
    pub fn SearchHotkeyCommand(&self, hotkey: &str) -> Option<&CommandNode> {
        self.command_root
            .as_ref()
            .and_then(|root| find_command_by_hotkey(root, hotkey))
    }

    /// Set the command tree root (used by UpdateCommands or tests).
    pub fn set_command_root(&mut self, root: CommandNode) {
        self.command_root = Some(root);
        self.commands_generation
            .set(self.commands_generation.get() + 1);
    }

    /// Run a command by spawning a process.
    /// Port of C++ `emFileManModel::RunCommand`.
    pub fn RunCommand(
        &mut self,
        cmd: &CommandNode,
        extra_env: &HashMap<String, String>,
    ) -> Result<(), String> {
        self.command_run_id = self.command_run_id.wrapping_add(1);

        let src_count = self.selection.GetSourceSelectionCount();
        let tgt_count = self.selection.GetTargetSelectionCount();

        let mut args: Vec<String> = Vec::new();

        if !cmd.interpreter.is_empty() {
            args.push(cmd.interpreter.clone());
        }
        args.push(cmd.cmd_path.clone());
        args.push(src_count.to_string());
        args.push(tgt_count.to_string());
        for i in 0..src_count {
            args.push(self.selection.GetSourceSelection(i).to_string());
        }
        for i in 0..tgt_count {
            args.push(self.selection.GetTargetSelection(i).to_string());
        }

        let mut env = extra_env.clone();
        env.insert(
            "EM_FM_SERVER_NAME".to_string(),
            self.ipc_server_name.clone(),
        );
        env.insert(
            "EM_COMMAND_RUN_ID".to_string(),
            self.selection.GetCommandRunId(),
        );

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let dir_path = if cmd.dir.is_empty() {
            None
        } else {
            Some(Path::new(&cmd.dir))
        };

        emProcess::emProcess::TryStartUnmanaged(
            &arg_refs,
            &env,
            dir_path,
            emProcess::StartFlags::empty(),
        )
        .map_err(|e| format!("Failed to start command: {e}"))
    }
}

fn find_command_by_path<'a>(node: &'a CommandNode, cmd_path: &str) -> Option<&'a CommandNode> {
    if node.cmd_path == cmd_path {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_command_by_path(child, cmd_path) {
            return Some(found);
        }
    }
    None
}

fn find_command_by_hotkey<'a>(node: &'a CommandNode, hotkey: &str) -> Option<&'a CommandNode> {
    if node.command_type == CommandType::Command && !node.hotkey.is_empty() && node.hotkey == hotkey
    {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_command_by_hotkey(child, hotkey) {
            return Some(found);
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManModel -- --test-threads=1`
Expected: All tests pass (existing + 7 new)

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManModel.rs
git commit -m "feat(emFileMan): add emFileManModel with Acquire, commands, IPC, RunCommand"
```

---

### Task 10: Add Icon and Look Fields to CommandNode

**Files:**
- Modify: `crates/emfileman/src/emFileManModel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn command_node_has_icon_and_look_fields() {
        let node = CommandNode::default();
        assert!(node.icon.is_none());
        assert_eq!(node.look.GetBgColor(), emcore::emColor::emColor::default());
    }

    #[test]
    fn parse_command_properties_with_colors() {
        let content = "\
#!/bin/bash
# [[BEGIN PROPERTIES]]
# Type = Command
# Caption = Test
# BgColor = #FF0000FF
# FgColor = #00FF00FF
# [[END PROPERTIES]]
";
        let node = parse_command_properties(content, "/test.sh").unwrap();
        assert_eq!(node.look.GetBgColor().Get(), 0xFF0000FF);
        assert_eq!(node.look.GetFgColor().Get(), 0x00FF00FF);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManModel::tests::command_node_has_icon_and_look -- --test-threads=1`
Expected: FAIL — `icon` and `look` fields don't exist on CommandNode

- [ ] **Step 3: Add icon and look fields**

In `crates/emfileman/src/emFileManModel.rs`, add imports:

```rust
use emcore::emColor::emColor;
use emcore::emImage::emImage;
use emcore::emLook::emLook;
```

Add fields to `CommandNode` struct:

```rust
pub struct CommandNode {
    pub cmd_path: String,
    pub command_type: CommandType,
    pub order: f64,
    pub interpreter: String,
    pub dir: String,
    pub default_for: String,
    pub caption: String,
    pub description: String,
    pub icon: Option<emImage>,
    pub look: emLook,
    pub hotkey: String,
    pub border_scaling: f64,
    pub pref_child_tallness: f64,
    pub children: Vec<CommandNode>,
    pub dir_crc: u64,
}
```

Update `Default` for CommandNode:

```rust
impl Default for CommandNode {
    fn default() -> Self {
        Self {
            cmd_path: String::new(),
            command_type: CommandType::Command,
            order: 0.0,
            interpreter: String::new(),
            dir: String::new(),
            default_for: String::new(),
            caption: String::new(),
            description: String::new(),
            icon: None,
            look: emLook::default(),
            hotkey: String::new(),
            border_scaling: 0.0,
            pref_child_tallness: 0.0,
            children: Vec::new(),
            dir_crc: 0,
        }
    }
}
```

In `parse_command_properties`, add handling for the color and icon properties:

```rust
            "BgColor" => {
                if let Some(color) = parse_color_value(value) {
                    node.look.SetBgColor(color);
                }
            }
            "FgColor" => {
                if let Some(color) = parse_color_value(value) {
                    node.look.SetFgColor(color);
                }
            }
            "ButtonBgColor" => {
                if let Some(color) = parse_color_value(value) {
                    node.look.SetButtonBgColor(color);
                }
            }
            "ButtonFgColor" => {
                if let Some(color) = parse_color_value(value) {
                    node.look.SetButtonFgColor(color);
                }
            }
            "Icon" => {
                let icon_path = if Path::new(value).is_absolute() {
                    PathBuf::from(value)
                } else {
                    Path::new(cmd_path)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(value)
                };
                node.icon = emcore::emImageFile::load_image_from_file(&icon_path);
            }
```

Remove the old catch-all that ignored these keys:
```rust
            // Delete this line:
            "BgColor" | "FgColor" | "ButtonBgColor" | "ButtonFgColor" | "Icon" => {}
```

Add the color parsing helper:

```rust
fn parse_color_value(s: &str) -> Option<emColor> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 8 {
        u32::from_str_radix(s, 16).ok().map(emColor::new)
    } else if s.len() == 6 {
        u32::from_str_radix(s, 16)
            .ok()
            .map(|rgb| emColor::new((rgb << 8) | 0xFF))
    } else {
        None
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManModel -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManModel.rs
git commit -m "feat(emFileMan): add Icon and Look fields to CommandNode with color parsing"
```

---

### Phase 3 Gate

- [ ] **Run full gate check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass. No regressions. All 89+ existing tests still pass, plus ~30 new tests.

---

## Final Verification

- [ ] **Verify all C++ API methods are covered**

For each file, check the spec's "C++ methods accounted for" list against the implementation:

| File | Methods to verify exist |
|------|------------------------|
| emDirModel | Acquire, GetEntryCount, GetEntry, GetEntryIndex, IsOutOfDate, reset_data, try_start_loading, try_continue_loading, quit_loading, calc_memory_need, calc_file_progress |
| emFileLinkModel | Acquire, GetFormatName, GetFullPath, GetBasePathType, GetBasePathProject, GetPath, GetHaveDirEntry |
| emFileManConfig | Acquire, GetFormatName, Get/Set for all 6 fields, GetChangeSignal |
| emFileManTheme | Acquire, GetFormatName, GetRec, ImageFileRec::GetImage (x4), GetChangeSignal |
| emFileManThemeNames | Acquire, GetChangeGeneration, Cycle, discover_themes_from_dir |
| emFileManViewConfig | Acquire, Get/Set for all 6 fields, GetTheme, CompareDirEntries, IsUnsaved, SaveAsDefault, GetChangeSignal |
| emFileManModel | Acquire, all selection methods with generation, GetCommandRoot, GetCommand, SearchDefaultCommandFor, SearchHotkeyCommand, RunCommand, SelectionToClipboard, GetMiniIpcServerName, GetShiftTgtSelPath/Set |

- [ ] **Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore(emFileMan): final cleanup for data layer completion"
```
