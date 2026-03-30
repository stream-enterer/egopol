# emFileMan Design

Date: 2026-03-30

## Objective

Port the C++ emFileMan application module to Rust as the file browser that
closes the rendering loop: directories display files, files display content
via plugins, and the full Eagle Mode browsing experience works end-to-end.

## Rationale

The plugin system is implemented (Spec: plugin-manager-design) and emStocks
is a working dynamic plugin, but nothing invokes the plugin system except
tests. emStocksFilePanel is a data-model stub because there's no panel tree
integration. The missing pieces are:

1. emFilePanel completion (Spec: emFilePanel-completion-design) — base class
   infrastructure for all file-viewing panels
2. emFileMan — the application module that drives the file browser

emFileMan provides the loop closure: emDirPanel displays directory contents
as a grid of emDirEntryPanel instances, each of which calls
`emFpPluginList::CreateFilePanel()` to insert plugin panels into the tree.
Without emFileMan, plugins exist but nothing creates them in a browsable
context.

## Scope

- New crate: `crates/emfileman/` (cdylib + rlib)
- 14 C++ headers → 15 Rust files (one split) + 3 FpPlugin entry points
- Full C++ parity: directory browsing, file display, selections, commands,
  themes, sorting/filtering, keyboard navigation, IPC, link files
- 3 `.emFpPlugin` config files
- Prerequisite: emFilePanel completion (Spec 1)

---

## Section 1: Principles & Constraints

### C++ parity (governing principle)

Every type in `include/emFileMan/` gets a Rust equivalent covering the same
public API surface. The file browser works end-to-end: navigate directories,
view file content via plugins, select files, execute commands, switch themes.

### File and Name Correspondence (inherited)

Each C++ header in `include/emFileMan/` gets a `.rs` file in
`crates/emfileman/src/` with the same name. One split: `emFileManTheme.h`
contains both `emFileManTheme` and `emFileManThemeNames` — split into two
files per one-type-per-file rule.

### Plugin architecture (inherited)

emFileMan is a `cdylib` crate loaded at runtime. It exports 3 `#[no_mangle]`
entry points (emDirFpPlugin, emDirStatFpPlugin, emFileLinkFpPlugin).
The host binary discovers it via `.emFpPlugin` config files. emFileMan
depends on `emcore` at compile time but the host does NOT depend on
emFileMan at compile time.

### Linux only

C++ `#ifdef _WIN32` code (drive listing, Windows file attributes, device name
detection) is not ported. The Rust port targets Linux. This matches the C++
Linux build which also excludes Windows code.

---

## Section 2: Crate Structure

### Cargo configuration

```toml
[package]
name = "emfileman"
version = "0.1.0"
edition = "2021"

[lib]
name = "emFileMan"
crate-type = ["cdylib", "rlib"]

[dependencies]
emcore = { path = "../emcore" }
```

### File layout

```
crates/emfileman/src/
  lib.rs                    module declarations, re-exports
  emDirEntry.rs             filesystem metadata, COW shared data
  emDirModel.rs             directory loading state machine
  emDirPanel.rs             grid of directory entries
  emDirEntryPanel.rs        single file/directory display
  emDirEntryAltPanel.rs     alternative content views
  emDirStatPanel.rs         directory statistics
  emFileManModel.rs         selections, commands, IPC
  emFileManConfig.rs        global defaults
  emFileManTheme.rs         100+ layout/color parameters
  emFileManThemeNames.rs    theme catalog (SPLIT: from emFileManTheme.h)
  emFileManViewConfig.rs    per-view config bridge
  emFileLinkModel.rs        link file parser
  emFileLinkPanel.rs        link file display
  emFileManControlPanel.rs  sort/filter/theme UI
  emFileManSelInfoPanel.rs  selection statistics
  emDirFpPlugin.rs          #[no_mangle] entry point
  emDirStatFpPlugin.rs      #[no_mangle] entry point
  emFileLinkFpPlugin.rs     #[no_mangle] entry point
```

19 files total. 14 map to C++ headers, 1 split, 3 map to C++ `.cpp`-only
files, 1 `lib.rs`.

### Config files

```
etc/emCore/FpPlugins/
  emDir.emFpPlugin          FileTypes = { "directory" }, Priority = 1.0
  emDirStat.emFpPlugin      FileTypes = { "directory" }, Priority = 0.1
  emFileLink.emFpPlugin     FileTypes = { ".emFileLink" }, Priority = 1.0
```

---

## Section 3: Internal Dependency Graph

```
Layer 0 — No emFileMan deps:
  emDirEntry          filesystem metadata, COW via Rc<SharedData>
  emFileManConfig     6 config fields, emConfigModel + Record
  emFileManTheme      ~100 layout/color params, emConfigModel + Record
  emFileManThemeNames theme catalog, scans theme directory
  emFileLinkModel     link file parser, emRecFileModel + Record

Layer 1 — Depends on Layer 0:
  emFileManViewConfig per-view config bridge (Config + Theme)
  emDirModel          3-phase directory loading (emFileModel + emDirEntry)
  emFileManModel      selections + commands + IPC (emDirEntry)

Layer 2 — Depends on Layers 0+1:
  emDirPanel          grid layout (emFilePanel + DirModel + FileManModel + ViewConfig)
  emDirEntryPanel     themed entry display (FileManModel + ViewConfig + CreateFilePanel)
  emDirEntryAltPanel  alternative content (same deps as DirEntryPanel)
  emDirStatPanel      entry counts (emFilePanel + DirModel + ViewConfig)
  emFileLinkPanel     link display (emFilePanel + FileLinkModel + ViewConfig)

Layer 3 — Depends on Layers 0+1+2:
  emFileManControlPanel  sort/filter/theme widgets (FileManModel + ViewConfig + ThemeNames)
  emFileManSelInfoPanel  selection stats scanner (FileManModel + DirEntry)

Layer 4 — Entry points:
  emDirFpPlugin          creates emDirPanel
  emDirStatFpPlugin      creates emDirStatPanel
  emFileLinkFpPlugin     creates emFileLinkPanel
```

---

## Section 4: Type Designs

### emDirEntry

Filesystem entry metadata with COW shared data. Port of C++
`emDirEntry` with `SharedData` reference counting.

```rust
struct SharedData {
    path: String,
    name: String,
    target_path: String,        // symlink target (empty if not symlink)
    owner: String,
    group: String,
    hidden: bool,
    stat: libc::stat,           // following symlinks
    lstat: Option<libc::stat>,  // not following symlinks (Some if symlink)
    stat_errno: i32,
    lstat_errno: i32,
    target_path_errno: i32,
}

pub struct emDirEntry {
    data: Rc<SharedData>,
}
```

COW via `Rc<SharedData>` with `Rc::make_mut` — same pattern as emArray,
emAvlTreeMap, emList.

Methods match C++ names: `GetPath`, `GetName`, `GetTargetPath`, `GetOwner`,
`GetGroup`, `IsSymbolicLink`, `IsDirectory`, `IsRegularFile`, `IsHidden`,
`GetStat`, `GetLStat`, `GetStatErrNo`, `GetLStatErrNo`,
`GetTargetPathErrNo`.

`Load(path)` and `Load(parent_path, name)` populate shared data via
`libc::lstat` / `libc::stat` / `libc::readlink` / `libc::getpwuid_r` /
`libc::getgrgid_r`.

### emDirModel

Directory loading state machine. Extends `emFileModel` via `FileModelOps`.

Three-phase incremental loading:
1. **Read**: `emTryReadDir` equivalent reads one name per
   `try_continue_loading()` call into `Vec<String>`
2. **Sort + deduplicate**: sort names, remove duplicates (seen in ISO/proc
   filesystems)
3. **Stat**: load one `emDirEntry` per call from sorted names

Progress: 0-20% during Phase 1 (asymptotic via sqrt), 20-100% during
Phases 2-3 (linear by entry count).

Memory need: `entry_count * 8192` (heuristic, matches C++).

`GetEntryIndex(name)` uses binary search on sorted entries.

`IsOutOfDate()` always returns true (directories are always re-scanned).

### emFileManConfig

Global file manager defaults. 6 fields:

- `SortCriterion`: enum (ByName, ByEnding, ByClass, ByVersion, ByDate, BySize)
- `NameSortingStyle`: enum (PerLocale, CaseSensitive, CaseInsensitive)
- `SortDirectoriesFirst`: bool
- `ShowHiddenFiles`: bool
- `ThemeName`: String
- `Autosave`: bool

`emConfigModel` + `Record` trait. Loads from
`emGetConfigDirOverloadable("emFileMan", None)`.

### emFileManTheme

Visual theme configuration. ~100 fields organized as:

- 17 color fields (BackgroundColor, SourceSelectionColor, etc.)
- ~80 dimension fields (geometry for background, borders, name, path, info,
  file/dir/alt content areas, padding)
- 5 alignment fields
- 2 string fields (DisplayName, DisplayIcon)
- 4 ImageFileRec fields (OuterBorderImg, FileInnerBorderImg,
  DirInnerBorderImg, AltInnerBorderImg) with lazy image loading

All fields are `Record` fields loaded from `.emFileManTheme` config files.

ImageFileRec: a string field that lazily loads an `emImage` from the theme
directory on first access. Implemented as a struct with
`RefCell<Option<emImage>>` for lazy init.

### emFileManThemeNames

Theme catalog. Scans theme directory, groups themes by display name (style)
and aspect ratio.

Two-level structure: `Vec<ThemeStyle>` where each style contains
`Vec<ThemeAR>` (name, aspect ratio string, height).

`BTreeMap<String, (usize, usize)>` for name-to-index reverse lookup.

Methods: `GetThemeStyleCount`, `GetThemeAspectRatioCount`,
`GetThemeName`, `GetDefaultThemeName`, `IsExistingThemeName`.

### emFileManViewConfig

Per-view configuration bridging global defaults with view-local overrides.

Holds copies of all 6 config fields. Setters write back to global config
if autosave is enabled. Signals `ChangeSignal` on any mutation.

`CompareDirEntries(e1, e2)` — the sorting comparison function implementing
6 criteria:
- **ByName**: locale-aware, case-sensitive, or case-insensitive comparison
- **ByEnding**: file extension comparison with name fallback
- **ByClass**: right-to-left word-class comparison (alpha/digit/other)
- **ByVersion**: numeric-aware version comparison ("2.10" > "2.9")
- **ByDate**: `st_mtime` comparison with name fallback
- **BySize**: `st_size` comparison with name fallback

Directories-first option applies as a pre-filter.

`RevisitEngine`: saves view position before theme change, smoothly animates
back after layout recalculates. Uses `emEngine` trait.

### emFileManModel

Central application model. Three subsystems:

**Selections** — Two sorted `Vec<SelEntry>` (source and target) with hash-based
binary search:

```rust
struct SelEntry {
    hash_code: i32,
    path: String,
}
```

Binary search compares hash first, then string. Methods: `SelectAsSource`,
`SelectAsTarget`, `DeselectAsSource`, `DeselectAsTarget`,
`IsSelectedAsSource`, `IsSelectedAsTarget`, `ClearSourceSelection`,
`ClearTargetSelection`, `SwapSelection`, `UpdateSelection` (removes
nonexistent paths), `SelectionToClipboard`.

**Commands** — Tree of `CommandNode` loaded from filesystem:

```rust
pub struct CommandNode {
    pub cmd_path: String,
    pub command_type: CommandType,  // Command, Group, Separator
    pub order: f64,
    pub interpreter: String,
    pub dir: String,
    pub default_for: String,
    pub caption: String,
    pub description: String,
    pub icon: emImage,
    pub hotkey: emInputHotkey,
    pub border_scaling: f64,
    pub pref_child_tallness: f64,
    pub children: Vec<CommandNode>,
    pub dir_crc: u64,
}
```

Commands loaded from shell scripts with `#[[BEGIN PROPERTIES]]` blocks.
Properties: Type, Order, Interpreter, Directory, DefaultFor, Caption,
Description, Icon, BgColor, FgColor, ButtonBgColor, ButtonFgColor, Hotkey,
BorderScaling, PrefChildTallness.

`SearchDefaultCommandFor(path)` — depth-first search, priority = longest
matching extension + 1.

`RunCommand(cmd, view)` — builds args
(`[interpreter] cmd_path src_count tgt_count src_paths... tgt_paths...`),
sets env vars (`EM_FM_SERVER_NAME`, `EM_COMMAND_RUN_ID`, `EM_X/Y/WIDTH/HEIGHT`),
spawns via `emProcess::TryStartUnmanaged`.

**IPC** — `emMiniIpc` server receiving selection commands from spawned
processes. Messages: `update`, `select <id> <paths...>`,
`selectks <id> <paths...>`, `selectcs <id> <paths...>`. Command run ID
validation prevents stale process interference.

`Cycle()`: triggered by file update signal, calls `UpdateSelection()` and
`UpdateCommands()` (CRC-based change detection for hot reload).

### emDirPanel

Grid of directory entries. Extends emFilePanel.

**Lazy model acquisition**: `notice()` handles `NF_VIEWING_CHANGED` —
acquires `emDirModel` when viewed, releases when not. This is the demand-
loading mechanism that prevents loading every directory on the filesystem.

**LayoutChildren()**: Grid layout algorithm. When content complete: calculates
optimal rows/cols for aspect ratio, applies theme padding, lays out
column-major. When incomplete: uses existing layout hints with bounds clamping.

**UpdateChildren()**: Creates/updates/deletes `emDirEntryPanel` children based
on model entries. Filters hidden files via config. Sorts children via
`CompareDirEntries()`. Handles active panel deletion with focus transfer.

**KeyWalk**: Type-ahead search. Accumulates typed characters, prefix-matches
against entry names (case-insensitive). `*` prefix enables substring search.
1-second timeout via `emTimer`.

### emDirEntryPanel

Themed display of a single file or directory entry. The rendering workhorse
(995 lines C++).

**Paint()**: Draws themed background (round rect), outer border image, name
(colored by file type: executable/regular/directory/fifo/block/char/socket),
info text (permissions, owner, size, timestamp), inner border, content area
background.

**UpdateContentPanel()**: The loop closure. When the panel is viewed and
content area exceeds `MinContentVW` threshold, calls
`emFpPluginList::CreateFilePanel(self, ContentName, path, stat_errno, mode)`.
Destroys content panel when panel leaves active path and isn't viewed.

**UpdateBgColor()**: Reads selection state from `emFileManModel`. Source
selection → SourceSelectionColor. Target selection → TargetSelectionColor.
Both → 50% blend.

**Recursive call guard**: `GetIconFileName()` delegates to content panel's
icon, with a boolean guard to prevent infinite recursion.

### emDirEntryAltPanel

Alternative content view. Like emDirEntryPanel but creates content via
`CreateFilePanel(..., alternative)` with incrementing alternative index.
Recursively nests: each alt panel can contain another alt panel for the
next alternative.

### emDirStatPanel

Simple statistics panel extending emFilePanel. Counts directory entries by
type (files, subdirectories, other, hidden). Renders formatted text.
Listens to VirFileState and Config change signals.

### emFileLinkModel

Link file parser. Extends emRecFileModel. Record fields:
- `BasePathType`: enum (None, Bin, Include, Lib, HtmlDoc, PdfDoc, PsDoc,
  UserConfig, HostConfig, Tmp, Res, Home)
- `BasePathProject`: String
- `Path`: String
- `HaveDirEntry`: bool

`GetFullPath()` resolves base path via `emGetInstallPath` then joins with
relative path.

### emFileLinkPanel

Link file display. Extends emFilePanel. Resolves target path via
`emFileLinkModel::GetFullPath()`. Creates child panel: either
`emDirEntryPanel` (if target is a file entry) or via `CreateFilePanel`
(if target is a plain file). Optional border rendering depending on parent
panel type. Lazy content creation at viewport threshold 60.0.

### emFileManControlPanel

Sort/filter/theme UI. Extends emLinearLayout. Contains:
- Sort criterion radio buttons (6 options)
- Name sorting style radio buttons (3 options)
- Directories-first checkbox
- Show hidden files checkbox
- Theme style/aspect ratio radio buttons
- Autosave checkbox
- Nested command group buttons

All widgets read/write through `emFileManViewConfig`. Command buttons
generated from `emFileManModel::GetCommandRoot()` tree.

### emFileManSelInfoPanel

Selection statistics with async directory scanning. State machine:
COSTLY → WAIT → SCANNING → SUCCESS / ERROR.

Two parallel stat computations:
- **Direct**: counts and sizes of selected entries
- **Recursive**: walks subdirectories with time-sliced Cycle()

Maintains directory stack for recursive traversal. Time-sliced to avoid
blocking UI.

### FpPlugin Entry Points

Three `#[no_mangle]` functions matching `emFpPluginFunc` signature:

- `emDirFpPluginFunc` — validates no properties, creates `emDirPanel`
- `emDirStatFpPluginFunc` — validates no properties, acquires `emDirModel`,
  creates `emDirStatPanel` with `updateFileModel=false`
- `emFileLinkFpPluginFunc` — validates no properties, creates
  `emFileLinkPanel`

---

## Section 5: Testing Strategy

### Unit tests

| Component | Tests |
|-----------|-------|
| emDirEntry | COW clone semantics, equality, hidden detection, symlink handling, stat accessors, Load from real filesystem |
| emFileManConfig | Record round-trip for all 6 fields |
| emFileManTheme | Field parsing, ImageFileRec lazy load |
| emFileManThemeNames | Catalog building from directory scan, index lookup, default theme |
| emFileManViewConfig | All 6 sort criteria × 3 naming styles, directories-first, config bridge sync |
| emFileManModel | Selection binary search (insert/find/delete), command property parsing, command search priority, hotkey lookup |
| emDirStatPanel | Entry counting by type |

### Behavioral tests

| Component | Tests |
|-----------|-------|
| emDirModel | 3-phase loading lifecycle, deduplication, progress calculation, memory need |
| emDirPanel | Lazy model acquisition (notice NF_VIEWING_CHANGED), UpdateChildren create/delete, KeyWalk prefix/substring match |
| emDirEntryPanel | Content panel lifecycle (create on view, destroy on leave), recursive guard, background color blending |
| emFileManModel | IPC message handling (select/selectks/selectcs with command run ID), CRC-based command hot reload |
| emFileLinkModel | Path resolution for each BasePathType |
| emFileLinkPanel | Child panel creation from resolved path |
| emFileManSelInfoPanel | State machine transitions, time-sliced recursive scan |

### Integration tests

| Component | Tests |
|-----------|-------|
| emDirModel | Load real directory, verify entry count and sorted order |
| emFileManTheme | Load actual theme file from C++ source tree |
| emFileManModel | Full selection lifecycle: select → swap → clear → update |
| FpPlugin entry points | dlopen → resolve symbol → create panel for directory/link |
| End-to-end | Load directory → emDirPanel creates entries → emDirEntryPanel calls CreateFilePanel → plugin panel renders |

---

## Section 6: Phase Structure

### Phase 1 — Data Foundation

**Goal:** emDirEntry and emFileManConfig ported with full tests.

Work items:
1. emDirEntry with COW shared data, Load via libc stat/lstat/readlink
2. emFileManConfig with Record trait, 6 fields, load/save
3. Unit tests for both

Gate: emDirEntry loads real filesystem entries. Config round-trips correctly.

### Phase 2 — Theme System

**Goal:** Themes load and the catalog builds.

Work items:
1. emFileManTheme with ~100 fields, Record trait, ImageFileRec lazy loading
2. emFileManThemeNames catalog scanner
3. Tests against actual C++ theme files

Gate: Theme file from C++ source loads and all fields parse. Catalog finds
all themes.

### Phase 3 — Application State

**Goal:** Selections and commands work.

Work items:
1. emFileManModel: selection subsystem (hash binary search, all operations)
2. emFileManModel: command tree (filesystem scan, property parsing, search)
3. emFileManModel: IPC server, RunCommand, HotkeyInput
4. emFileManModel: Cycle with UpdateSelection + UpdateCommands
5. Full test coverage

Gate: Selection operations are correct. Commands load from C++ command tree.
IPC messages update selections.

### Phase 4 — Config Bridge and Models

**Goal:** Per-view config, directory model, and link model work.

Work items:
1. emFileManViewConfig with CompareDirEntries (6 sort criteria × 3 styles)
2. emFileManViewConfig RevisitEngine
3. emDirModel with 3-phase incremental loading
4. emFileLinkModel with path resolution
5. Tests for sorting, loading, path resolution

Gate: Directories load incrementally. Sorting produces correct order for
all criteria. Link paths resolve correctly.

### Phase 5 — Panel Layer

**Goal:** All panels render and the loop closes.

Work items:
1. emDirPanel (lazy acquisition, grid layout, UpdateChildren, KeyWalk)
2. emDirEntryPanel (themed Paint, UpdateContentPanel with CreateFilePanel,
   selection highlighting, recursive guard)
3. emDirEntryAltPanel (alternative content)
4. emDirStatPanel (entry counts)
5. emFileLinkPanel (child panel from resolved path)
6. Tests for panel lifecycle, content creation, layout

Gate: Directory panel creates entry panels. Entry panels call
CreateFilePanel. Plugin panels appear in the tree.

### Phase 6 — Control UI

**Goal:** Control panel and selection info work.

Work items:
1. emFileManControlPanel (sort/filter/theme widgets, command buttons)
2. emFileManSelInfoPanel (async scanning state machine)
3. Tests for widget state sync and scan lifecycle

Gate: Sort/filter changes propagate through ViewConfig to panels.
Selection statistics compute correctly.

### Phase 7 — Plugin Integration

**Goal:** End-to-end with dynamic loading.

Work items:
1. emDirFpPlugin, emDirStatFpPlugin, emFileLinkFpPlugin entry points
2. .emFpPlugin config files in etc/emCore/FpPlugins/
3. Integration tests: dlopen → create panel → verify panel type
4. End-to-end test: navigate directory tree via plugin system

Gate: `cargo build --workspace` produces `libemFileMan.so`. Plugin loads
via dlopen. Directory browsing works end-to-end.

---

## Section 7: Relationship to Other Specs

### Prerequisite: emFilePanel completion (Spec 1)

emDirPanel, emDirStatPanel, and emFileLinkPanel all extend emFilePanel.
They require:
- `SetFileModel()` that registers as a client
- `Cycle()` that monitors model state
- `notice()` that forwards memory/priority
- `IsContentReady()` based on VirtualFileState

### Prerequisite: Plugin manager (already implemented)

emDirEntryPanel calls `emFpPluginList::CreateFilePanel()`. emFileMan is
itself a cdylib loaded via the plugin system. Both are complete.

### Enables: emStocksFilePanel rendering

Once emFilePanel completion is done, emStocksFilePanel can be upgraded from
a stub to a real panel that tracks its model and renders content.

### Enables: Future app module ports

emBmp, emTga, emGif, emPdf, emAudioPlayer — all export `emFpPluginFunc`
and return panels that extend emFilePanel. Once emFileMan is ported, these
plugins render inside the directory browser.
