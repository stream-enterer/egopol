# Phase 4d — emRec Persistence IO — Ledger

**Started:** 2026-04-22
**Branch:** port-rewrite/phase-4d
**Baseline:** see 2026-04-19-phase-4d-baseline.md
**Spec sections:** §7 D7.1
**JSON entries to close:** none (E026 deferred to Phase 4e)

## Task log

- **Task 1 fixup** — reviewer feedback applied: `RecIoError` fields `pub → pub(crate)` (I1); removed untagged `RecIoError::new` constructor, `with_location` is canonical (I2); trimmed paraphrase doc comments on `TryReadDelimiter` / `TryReadDouble` / `TryReadQuoted` (writer methods were already signature-only) (M1); renamed `_assert_dyn_safe → assert_dyn_safe` in both files (M6); added WHY comment on `PeekResult::Delimiter(char)` noting ASCII-only lexer contract (M8). Gates: fmt clean, clippy `-D warnings` clean, nextest 2617/2617 (unchanged).
- **Task 1** — emRecReader / emRecWriter traits defined. Ports the per-element primitive API from `emRec.h:1569-1620` (reader) and `emRec.h:1691-1724` (writer). `ElementType` enum mirrors C++ `ET_*` names (prefix dropped); `PeekResult` folds C++'s `TryPeekNext(char *pDelimiter)` out-parameter into the `Delimiter(char)` variant (DIVERGED, documented at enum declarations). `TryReadIdentifier` / `TryReadQuoted` return owned `String` instead of `const char *` into an internal buffer (idiom adaptation — buffer lifetime invisible to callers). `TryReadInt` returns `i32` (C++ `int` width — not `i64` as an earlier plan sketch suggested). `ThrowElemError` / `ThrowSyntaxError` *return* the constructed error rather than throw (DIVERGED; helper, not a throw). Both traits dyn-compatible; compile-time dyn-safety + method-list assertions live in `#[cfg(test)]` `DummyReader` / `DummyWriter` impls. `RecIoError` (new, in `emRecReader.rs`) carries `source_name` / `line` / `message` with `Display` + `Error`; intentionally distinct from `emRecParser::RecError` (legacy text parser). State-machine driver methods (`TryStartReading`, `TryContinueReading`, `TryFinishReading`, `QuitReading`, `GetRootRec`, `GetSourceName`, protected `TryRead`/`TryClose`) deferred to Task 2+. Files: `crates/emcore/src/emRecReader.rs` (new), `crates/emcore/src/emRecWriter.rs` (new), `crates/emcore/src/lib.rs` (+2 `pub mod` lines). +4 tests (2613 → 2617); fmt clean, clippy `-D warnings` clean, nextest 2617/2617.
