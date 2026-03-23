# Implement emTestPanelStandalone — Interactive Visual Test Application

## What this is

Zuicchini is a Rust port of Eagle Mode's emCore UI toolkit. The port is complete — 84 capabilities verified, 199 golden tests, 1096+ total tests, parallel rendering. But it has only ever run headlessly in test harnesses. This task creates the first interactive windowed application: a standalone launcher that opens a window and displays the TestPanel.

The C++ equivalent is `emTestPanelStandalone.cpp` — a ~15 line program that creates a framework, opens a window, puts a TestPanel in it, and runs the event loop. The TestPanel exercises every widget, layout, painter operation, and interaction in emCore. If this runs and looks correct, the port works.

## What you need to know about the codebase

- **Project root:** current working directory (Rust crate, builds with `cargo`)
- **C++ reference:** `~/.local/git/eaglemode-0.96.4/` (read-only)
- **Window system:** zuicchini uses `winit` for windowing and `wgpu` for GPU rendering. The main application struct is `App` in `src/window/app.rs` — it implements winit's `ApplicationHandler`.
- **Window:** `ZuiWindow` in `src/window/zui_window.rs` manages a wgpu surface, tile cache, view, and input dispatch. It has `render_parallel()` for multi-threaded tile rendering.
- **Panel tree:** `PanelTree` in `src/panel/tree.rs` holds the widget hierarchy. Panels implement `PanelBehavior` (trait in `src/panel/behavior.rs`).
- **TestPanel:** Already ported at `src/test_panel.rs` (or find it with `grep -rn 'struct TestPanel' src/`). Used by the golden test infrastructure. It creates a complex widget tree exercising borders, buttons, labels, scalar fields, text fields, color fields, checkboxes, radio buttons, list boxes, splitters, layouts, and custom painting.
- **Scheduler:** `EngineScheduler` in `src/scheduler/` drives the event loop — engines cycle, signals fire, timers tick.
- **Port fidelity:** Use idiomatic Rust throughout. This is application-level glue code, not pixel arithmetic. Use the existing types and patterns in the codebase.

## C++ reference

Read these before implementing:
- `~/.local/git/eaglemode-0.96.4/src/emTest/emTestPanelStandalone.cpp` — the target (very short)
- `~/.local/git/eaglemode-0.96.4/src/emTest/emTestPanel.cpp` — to understand what TestPanel does (you have the Rust port, but the C++ shows the intent)

## What to implement

A binary target that:
1. Initializes the zuicchini toolkit (scheduler, context, resource cache)
2. Creates a window via `ZuiWindow` (or `App`)
3. Creates a `TestPanel` as the root panel in the window's view
4. Runs the event loop
5. The user can zoom, pan, click widgets, and interact with the full TestPanel widget tree

## Implementation steps

1. **Find the existing application entry point.** Search `src/` for `fn main`, `ApplicationHandler`, `App::new`, or `bin/`. Understand how zuicchini currently launches (if it does). If there's already a binary target, extend it. If not, create one.

2. **Read `src/window/app.rs`** fully. Understand how `App` creates windows, handles events, and drives the scheduler. This is the winit integration layer.

3. **Read `src/window/zui_window.rs`** fully. Understand how windows are created, how the view is set up, and how the panel tree gets attached.

4. **Read the TestPanel Rust source.** Understand its constructor — what arguments it takes, what it creates in auto_expand.

5. **Create the binary.** Options:
   - A new file at `src/bin/test_panel.rs` (cargo auto-discovers binaries in `src/bin/`)
   - Or an example at `examples/test_panel.rs` (run with `cargo run --example test_panel`)

   The binary should be minimal — initialize, create window, create TestPanel, run. Match the simplicity of the C++ standalone (~15 lines of actual logic).

6. **Run it:** `cargo run --bin test_panel` (or `--example`). The window should open and display the TestPanel.

7. **Verify interactivity:** You can't automate this, but confirm the binary compiles, runs without panic, and opens a window. If it crashes, read the error and fix it.

## Potential issues to watch for

- **TestPanel may need a Context and Scheduler** to be set up before construction. Read its constructor to see what it requires.
- **The TestPanel loads a test image** (`teddy.tga`). It uses `include_bytes!` or loads from a resource path. Check that the image is accessible from the binary's working directory, or that the path is embedded.
- **The window's view needs a root panel.** Check how `ZuiWindow` or `App` attaches a panel tree to the view. The TestPanel should be the root.
- **The scheduler must be running** for engines to cycle, signals to fire, and animations to work. Make sure the event loop drives the scheduler.
- **wgpu initialization** may need specific configuration (power preference, backend selection). Follow whatever `App` or `ZuiWindow` already does.

## Rules

- Keep the binary minimal. This is glue code, not a new feature.
- Do not modify the TestPanel implementation. Use it as-is.
- Do not modify the window/app infrastructure. Use it as-is.
- If something is missing (e.g., no public constructor for App, or no way to set the root panel), that's the gap to fix — add the minimum public API needed, don't restructure.
- `cargo check` after every change. `cargo test --workspace` before committing to verify no regressions.
- Commit: `feat: add TestPanel standalone binary`

## After implementation

Report:
1. Does it compile?
2. Does it run without panicking?
3. What command launches it?
4. Any issues encountered (missing APIs, resource loading failures, etc.)
