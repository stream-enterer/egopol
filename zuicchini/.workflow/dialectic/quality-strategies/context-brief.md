# Situational Context Brief

This document is shared context for all Round 0 agents. It describes the current state of the project so that recommendations target what's actually needed, not what's already done.

## What the project is

zuicchini is a Rust port of Eagle Mode's emCore C++ UI framework. It's a zoomable UI toolkit with 20 widget types, a panel tree, a rendering pipeline, layout engines, input handling, and a scheduler. The C++ source (Eagle Mode 0.96.4) is available locally at `/home/ar/.local/git/eaglemode-0.96.4/`.

## What quality assurance has been completed

### Audit (complete)
- All 20 C++ widget classes compared against their Rust counterparts by LLM subagents
- Each subagent read both C++ and Rust source, produced a detailed report with C++ file:line references, Rust file:line references, severity ratings, confidence levels, and golden test coverage notes
- 170+ findings produced across 19 per-widget report files
- 6 cross-cutting concerns identified (CC-01 through CC-06): code duplication, silent setters, no disabled rendering, no min-extent guard, alignment defaults, hit-test geometry
- A separate pixel-fidelity audit confirmed all compositing pipeline math (Blinn div255, coverage, area sampling, bilinear interpolation, gradient sqrt table, Fixed12 arithmetic) is correct

### Fixes (3 sessions, 31 fixes applied)
- Session 1: 19 fixes (button-family hit tests, keyboard handling, modifier guards, alignment defaults, splitter geometry, colorfield text underlay, etc.)
- Session 2: 12 fixes (border substance_round_rect coefficient, label_space, best_label_tallness, MarginFilled, Dialog keyboard, Tunnel invalidation, etc.)
- Session 3: FileSelectionBox reactive layer (event system, FileItemPanel, directory navigation, name field sync, locale-aware sort)
- All 1144+ tests pass after every fix

### Current status (what remains)
- 83 items FIXED
- 30 items PENDING (mostly LOWs in FilePanel, FileDialog, CoreConfigPanel, Dialog, Look, ErrorPanel, Border)
- 2 items PARTIALLY FIXED (Button Click() API, RadioButton Drop re-index)
- 9 items INTENTIONAL DIVERGENCE (ScalarField f64 type, TextField undo/selection architecture, ListBox arrow keys, TkTest missing sections)
- 3 items CLOSED (investigated, no action needed)
- 16 items NOTE (observations, not bugs)

## What infrastructure exists

### Testing
- **1144+ tests** including golden pixel-comparison tests, behavioral tests, interaction tests, layout tests, scheduler tests, animator trajectory tests
- **Golden test generator**: C++ program (`tests/golden/gen/gen_golden.cpp`, 4251 lines) that links against `libemCore.so` and produces binary golden data
- **Comparison functions**: pixel (channel tolerance + max failure percentage), rect (f64 epsilon), behavioral/notice/input (exact match), trajectory (f64 tolerance)
- **Divergence measurement**: `MEASURE_DIVERGENCE=1` env var, `DUMP_GOLDEN=1` for diff images
- **Pre-commit hook**: runs `cargo fmt` → `clippy -D warnings` → `cargo-nextest ntr`

### Audit artifacts
- Per-widget `.md` files with every finding annotated: `### [SEVERITY] Description — **STATUS**`
- Status is greppable: `grep -rn 'PENDING\|PARTIALLY FIXED' results/*.md | grep '### '`
- Each finding has C++ file:line, Rust file:line, fidelity layer, confidence, coverage assessment
- Cross-cutting concerns tracked in `cross-cutting-concerns.md`
- TkTest and TestPanel composition divergences documented

### Orchestration
- Fix prompt (`~/Documents/prompt.md`) drives subagent dispatch with anti-patterns codified
- Run log tracks every fix with finding reference, change description, test results
- Single-source-of-truth annotation system (status lives in per-widget files, discovered by grep)

### C++ reference
- Full C++ source on disk (headers + implementation)
- C++ golden test generator compilable and runnable
- C++ binaries available (`emTestPanelStandalone`, `emTkTestPanelStandalone`)

## Quality bar (from CLAUDE.md)

- **Pixel arithmetic**: reproduce C++ integer formulas exactly. `(x*257+0x8073)>>16` not f64 division.
- **Geometry**: same algorithm and operation order on golden-tested paths.
- **State logic**: fully idiomatic Rust. Golden tests verify output, not structure.
- **When in doubt**: if the function's output feeds a golden test → port C++ formula exactly. If not → write idiomatic Rust.

## Project constraints

- Single-threaded UI (no Arc/Mutex — Rc/RefCell for shared state)
- No `#[allow(...)]` / `#[expect(...)]` — fix warnings instead
- No glob imports except `use super::*` in tests
- `f64` logical coords, `i32` pixel coords, `u32` image dims, `u8` color channels
- Color intermediate math in `i32` or wider, never truncate to `u8` mid-calculation

## Known failure modes from THIS workflow

These are actual problems encountered during the audit-fix cycle:

1. **LLM early completion**: agents fix 4 of 12 items and declare the session productive
2. **Lossy derivative documents**: manually-compiled checklists missed items from the source reports
3. **Over-fixing**: subagents refactored surrounding code while fixing a one-line bug
4. **Incomplete cross-cutting fixes**: CC-05 fixed in Label but not propagated to all widgets initially
5. **Silent skipping**: fix agents marked items ACCEPTED without justification to clear the checklist
6. **Stale metadata**: summary.md drifted out of sync with per-widget files
7. **Wrong function called**: TextField Ctrl+Left/Right called `word_boundary` instead of `word_index` — correct function existed but the wrong one was wired

## What the 8 proposed strategies are (being evaluated)

1. Adversarial re-audit of fix diffs against C++ reference
2. Cross-fix consistency check (CC-01 duplication, shared code paths)
3. Rationale adversarial review (challenge CLOSED/DEFERRED justifications)
4. Coverage gap analysis (map modified functions to golden test coverage)
5. Mutation testing on CLOSED items (write tests that demonstrate divergence)
6. Empirical pixel diffing (run both binaries, capture framebuffers, diff)
7. Commit-message-to-finding traceability audit
8. Forward-looking fragility analysis (maintenance guide for duplicated code)
