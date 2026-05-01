# SlotMap Panic — Proven Facts

Facts confirmed by instrumentation runs. No hypotheses, no speculation.

## The Crash

- Panic: `invalid SlotMap key used` at `emPanelTree.rs:756:45` (`PanelTree::remove`)
- Call site: `ControlPanelBridge::Cycle` in `emMainWindow.rs:880`
- Trigger: zoom-out after zoom-in in the file manager

## Storage-Time Assertion

An assertion was added at `emMainWindow.rs` immediately before `self.content_ctrl_panel = Some(ccp_id)`:

```rust
assert!(ctrl_svp.sub_tree().contains(ccp_id), ...);
```

**This assertion fired.** The garbage PanelId is produced at creation time (inside `create_control_panel_in`), not removed between cycles.

## The Walk

When the crash occurs, `create_control_panel_in` walks the content tree's parent chain starting from the active panel. An audit assert was added:

```rust
assert!(in_target, "CreateControlPanel at {:?} returned ID {:?} not in target_tree ...", cur, rid, ...);
```

**This assert fired with `cur = PanelId(18v1)` returning `Some(PanelId(2519536797v3046123504))`.**

- `PanelId(18v1)` fields at the time of the call (read before `take_behavior`):
  - `has_behavior = true`
  - `is_active = false`
  - `parent = Some(PanelId(8v1))`
- Returned ID `2519536797v3046123504` was not in `target_tree` and not in `self` (content tree).

## Identity of Behavior at PanelId(18v1)

`eprintln!("[CCP_IMPL] ...")` was added at the top of `CreateControlPanel` for:
- `emDirPanel`
- `emDirEntryPanel`
- `emTestPanel`

**None of these fired during the crash runs.** The behavior at `PanelId(18v1)` is not any of these three types.

## `type_name()` Dispatch

Calling `behavior.type_name()` (vtable dispatch) on behaviors in the content tree produced a **SIGSEGV** for PanelIds `13v1`/`12v1` during one audit run.

Adding `eprintln!("[BEHAVIOR_REG] {:?} = {}", id, behavior.type_name())` inside `set_behavior` produced a **segfault at startup** (before any user interaction).

## `is_active` Desync

Two code paths write `emView::active`:

| Path | Updates `self.active` | Updates `PanelData::is_active` |
|------|-----------------------|-------------------------------|
| `RawVisit` (`emView.rs:966`) | Yes | **No** |
| `set_active_panel` (`emView.rs:1724`) | Yes | Yes |
| `SetActivePanel` (`emView.rs:782`, tests only) | Yes | **No** |

`RawVisit` is the path taken during zoom transitions. It writes `self.active = Some(panel)` without touching `PanelData::is_active`. `create_control_panel_in` (before the aborted fix) read `PanelData::is_active` to compute `self_is_active`. These two can diverge.

## Phase 0 Repro Confirmation (2026-05-01)

Repro reproduces deterministically at `09ca6e98` via `./scripts/repro_isactive_panic.sh` with zoom-in/zoom-out on a directory entry. Backtrace matches: `PanelTree::remove` (emPanelTree.rs:756) ← `ControlPanelBridge::Cycle` (emMainWindow.rs:880).

**New observation**: the run terminates with `free(): invalid pointer` printed by libc *after* the Rust panic message. This is glibc's heap-allocator detecting corruption in its own structures during shutdown/unwind. It is independent of the Rust `assert!` and is direct evidence of memory corruption (not just a logic-layer key-mismatch). Supports H1 (trait-object / heap corruption); cannot be explained by an `is_active` desync alone.

Baseline saved to `target/repro-baseline.log`.

## ASan Run — Blocked (Tooling)

ASan build of the eaglemode binary succeeds (`RUSTFLAGS="-Zsanitizer=address -Cforce-frame-pointers=yes"` on `nightly`, `cargo +nightly run --target x86_64-unknown-linux-gnu -p eaglemode`). Cosmos frames render, but plugin cdylibs (libemFileMan, libemStocks, libemTestPanel) fail at dlopen — the cosmos interior is empty. Re-link with `-Clink-arg=-Wl,--export-dynamic` to expose ASan's runtime symbols to dlopened libs did not change the result.

ASan is not viable for reproducing this panic without further tooling work (likely requires linking the ASan runtime as a dso, or rebuilding the plugin loader path). Task 1A is **blocked**; pivoting to Task 1B (non-vtable behavior type tag) for the same triangulation goal from a different direction.

## What Is Not Known

- The concrete type of the behavior at `PanelId(18v1)`.
- Why `behavior.type_name()` vtable dispatch crashes (both at runtime and at startup via `set_behavior`).
- Whether the garbage returned ID `2519536797v3046123504` is a stale/generationally-invalidated key, a corrupted value, or something else.
- Whether fixing the `is_active` desync alone resolves the crash, or whether the behavior at `PanelId(18v1)` has a separate defect.
