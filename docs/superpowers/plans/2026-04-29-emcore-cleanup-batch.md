# emcore cleanup batch — implementation plan

**Date:** 2026-04-29
**Scope:** emcore crate only. Three independent mechanical tasks batched into one PR.
**Base:** `main` @ `3a8143ac`
**Test baseline:** 2895/2895

---

## Context

Three accumulated cleanup items from the F010 Signal-Drift Tier-B session, all emcore-affecting, all mechanical:

1. **`*_for_test()` idiom consistency** — B-010 added 5 test-surface accessors to 4 widget files without `#[doc(hidden)]`; all other `*_for_test()` methods in emcore carry `#[doc(hidden)]`. Fix: add the attribute to the 5 inconsistent sites.
2. **D10 pseudo-DIVERGED cleanup** — 8 doc-comment blocks in `emDialog.rs` (×7) and `emFileDialog.rs` (×1) use `DIVERGED (Phase 3.6...)` parenthetical form. The annotation linter scans for `DIVERGED:` (colon form) and silently misses these. All 8 are language-forced (same root: `'static + FnMut` closure can't capture `&mut emFileDialog`). Fix: reclassify to `DIVERGED: (language-forced)` + tighten linter.
3. **Methodology note** — Document the pre-merge verification lesson from B-015: "verify C++ Cycle branch structure directly; do not rely on design-doc 'no branching' claims." Record in the spine decisions doc for future bucket designers.

All three tasks are independent. No new tests required. No inventory rows to flip.

---

## Task 1 — Add `#[doc(hidden)]` to B-010 widget test accessors

**Files:** `crates/emcore/src/{emCheckBox,emScalarField,emListBox,emTextField}.rs`

### Steps

1. `crates/emcore/src/emCheckBox.rs` — find `pub fn set_checked_for_test`. Add `#[doc(hidden)]` on the line immediately above it.

2. `crates/emcore/src/emScalarField.rs` — find `pub fn set_value_for_test`. Add `#[doc(hidden)]` immediately above it.

3. `crates/emcore/src/emListBox.rs` — two methods:
   - Find `pub fn set_selected_indices_for_test`. Add `#[doc(hidden)]` immediately above it.
   - Find `pub fn set_triggered_item_index_for_test`. Add `#[doc(hidden)]` immediately above it.

4. `crates/emcore/src/emTextField.rs` — find `pub fn set_text_for_test`. Add `#[doc(hidden)]` immediately above it.

**Verification:** `rg -n 'for_test' crates/emcore/src/emCheckBox.rs crates/emcore/src/emScalarField.rs crates/emcore/src/emListBox.rs crates/emcore/src/emTextField.rs` — every hit should have `#[doc(hidden)]` on the preceding non-blank line.

---

## Task 2 — Reclassify pseudo-DIVERGED blocks + tighten linter

### 2a. Reclassify 8 doc-comment blocks

All 8 are **language-forced**: Rust's `'static + FnMut` closure lifetime requires fields that C++ reaches via `this` (virtual `CheckFinish`) to be mirrored onto `DlgPanel`, since the closure cannot capture `&mut emFileDialog` across the struct boundary.

For each block below, replace the opening `DIVERGED (Phase …)` tag with `DIVERGED: (language-forced)`. Keep all prose after the tag unchanged.

| File | Line | Current tag | New tag |
|------|------|-------------|---------|
| `crates/emcore/src/emDialog.rs` | 540 | `DIVERGED (Phase 3.6 Task 3)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emDialog.rs` | 555 | `DIVERGED (Phase 3.6 Task 3)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emDialog.rs` | 563 | `DIVERGED (Phase 3.6 Task 3)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emDialog.rs` | 568 | `DIVERGED (Phase 3.6 Task 3 fix)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emDialog.rs` | 576 | `DIVERGED (Phase 3.6.1 Task 2)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emDialog.rs` | 588 | `DIVERGED (Phase 3.6.1 Task 2)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emDialog.rs` | 593 | `DIVERGED (Phase 3.6.1 Task 2)` | `DIVERGED: (language-forced)` |
| `crates/emcore/src/emFileDialog.rs` | 655 | `DIVERGED (Phase 3.6.1 Task 2)` | `DIVERGED: (language-forced)` |

**Note:** Line numbers above reflect `main @ 3a8143ac`. Verify with `rg -n 'DIVERGED (Phase' crates/emcore/` before editing.

### 2b. Tighten the annotation linter

File: `crates/xtask/src/annotations.rs`

Add a third scan pass inside `run()` that flags any line containing `DIVERGED (` (without a colon) as malformed:

```rust
pub fn run(_args: impl Iterator<Item = String>) -> std::process::ExitCode {
    let mut failures = Vec::new();
    for hit in scan("DIVERGED:", DIVERGED_CATEGORIES) {
        failures.push(hit);
    }
    for hit in scan("RUST_ONLY:", RUST_ONLY_CATEGORIES) {
        failures.push(hit);
    }
    for hit in scan_malformed("DIVERGED (") {
        failures.push(hit);
    }
    // ... rest unchanged
}
```

Add the helper at the bottom of the file:

```rust
fn scan_malformed(tag: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let walker = walkdir::WalkDir::new("crates")
        .into_iter()
        .filter_entry(|e| !e.path().starts_with("crates/xtask"))
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"));
    for entry in walker {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        for (i, line) in text.lines().enumerate() {
            if line.contains(tag) {
                failures.push(format!(
                    "{}:{} malformed annotation '{}' — use 'DIVERGED: (category)' form",
                    entry.path().display(),
                    i + 1,
                    tag.trim(),
                ));
            }
        }
    }
    failures
}
```

**Verification:** `cargo xtask annotations` must pass with zero failures after Task 2a is complete and Task 2b is added. If it reports failures, Task 2a missed a site.

---

## Task 3 — Methodology note in decisions.md

File: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/decisions.md`

Append a new section at the end of the file:

```markdown
---

## Methodology Notes

These are not decisions (no D-### ID) — they are process lessons captured for future bucket designers.

### M-001 — Verify C++ Cycle branch structure directly

**Lesson (from B-015 pre-merge):** Design-doc claims about C++ `Cycle()` branching behavior must not be trusted without reading the actual `.cpp` source. The B-015 design doc stated "C++ does not switch per signal in `emColorField::Cycle`." Direct read of `emColorField.cpp:116-187` showed 8 per-signal `IsSignaled` branches with distinct cascade flags (`rgbaChanged`, `hsvChanged`, `textChanged`). The naive `any() + sync_from_children` Rust shape would have caused observable cross-channel feedback (writing back to a slider the user is actively editing).

**Rule:** Before writing the Rust `Cycle` body for any P-006/P-007/P-001 consumer, open the C++ `.cpp` file, count the `IsSignaled` calls, and confirm the per-branch logic. Do not rely on brainstorm summaries, design-doc prose, or LLM recall about branching. One `grep -n 'IsSignaled' emFoo.cpp` is the verification.
```

---

## End-of-batch gate

Run all of the following in order. Do not commit until all pass.

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo-nextest ntr
cargo xtask annotations
```

Expected: 2895/2895 tests pass (no new tests added). Annotation lint: zero failures. Clippy: zero warnings.

Then commit all changes in a single commit:

```
chore(emcore): cleanup batch — doc(hidden) on test accessors, D10 DIVERGED reclassification, M-001 methodology note
```

No inventory rows to flip. No work-order status changes. PM reconciliation (work-order note + memory update) is handled by the PM session.
