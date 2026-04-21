# Phase 4a — emRec Trait + Primitive Concrete Types — Ledger

**Started:** 2026-04-21
**Branch:** port-rewrite/phase-4a
**Baseline:** see `2026-04-19-phase-4a-baseline.md`
**Spec sections:** §7 D7.1, §7 D7.3, §7 D7.4
**JSON entries to close:** none (E026 / E027 remain open until Phase 4d)

## Drift-note decision (plan line 21)

**Chosen:** option (a) — move existing `crates/emcore/src/emRec.rs` contents (905 lines: `RecStruct`, `RecValue`, `parse_rec`, `write_rec`) to a new file `crates/emcore/src/emRecParser.rs` with a `SPLIT:` comment citing `emCore/emRec.h` (C++ `emRecReader` / `emRecWriter`). `emRec.rs` is then free for the `pub trait emRec<T>` per I4a-1.

**Rationale:**
- CLAUDE.md File and Name Correspondence: "primary file keeps the C++ name". `class emRec` is the header's primary class; the trait belongs in `emRec.rs`.
- Existing `RecStruct`/`RecValue`/parser content does not correspond to `class emRec`; closer to `emRecReader` / `emRecWriter` (emRec.h lines 32–33). Splitting it out preserves correspondence.
- Applied as Task 2 pre-step, before the trait is introduced.

## B11a pre-commit hook

Phase 4a plan has no stage-only tasks — every task has its own Step 5 commit boundary. Hook left in place.

## Task log

- **Task 1 — emRecNode base trait** — COMPLETE. Commits `24839ea3` (initial), `3027e4af` (fixup: expanded doc, DIVERGED annotation on `parent()` citing `UpperNode` field + no-public-`GetParent`-on-emRecNode in C++). Spec review ✅; code-quality review flagged 3 Important items; 2 addressed in fixup, 1 (non_camel_case_types) already handled by crate-level attribute in `lib.rs`. Invariant I4a-1 / I4a-2 progress: `emRecNode.rs` exists with trait.
