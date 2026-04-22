# Phase 4d — emRec Persistence IO — Ledger

**Started:** 2026-04-22
**Branch:** port-rewrite/phase-4d
**Baseline:** see 2026-04-19-phase-4d-baseline.md
**Spec sections:** §7 D7.1
**JSON entries to close:** none (E026 deferred to Phase 4e)

## Task log

- **Task 1** — emRecReader / emRecWriter traits defined. Ports the per-element primitive API from `emRec.h:1569-1620` (reader) and `emRec.h:1691-1724` (writer). `ElementType` enum mirrors C++ `ET_*` names (prefix dropped); `PeekResult` folds C++'s `TryPeekNext(char *pDelimiter)` out-parameter into the `Delimiter(char)` variant (DIVERGED, documented at enum declarations). `TryReadIdentifier` / `TryReadQuoted` return owned `String` instead of `const char *` into an internal buffer (idiom adaptation — buffer lifetime invisible to callers). `TryReadInt` returns `i32` (C++ `int` width — not `i64` as an earlier plan sketch suggested). `ThrowElemError` / `ThrowSyntaxError` *return* the constructed error rather than throw (DIVERGED; helper, not a throw). Both traits dyn-compatible; compile-time dyn-safety + method-list assertions live in `#[cfg(test)]` `DummyReader` / `DummyWriter` impls. `RecIoError` (new, in `emRecReader.rs`) carries `source_name` / `line` / `message` with `Display` + `Error`; intentionally distinct from `emRecParser::RecError` (legacy text parser). State-machine driver methods (`TryStartReading`, `TryContinueReading`, `TryFinishReading`, `QuitReading`, `GetRootRec`, `GetSourceName`, protected `TryRead`/`TryClose`) deferred to Task 2+. Files: `crates/emcore/src/emRecReader.rs` (new), `crates/emcore/src/emRecWriter.rs` (new), `crates/emcore/src/lib.rs` (+2 `pub mod` lines). +4 tests (2613 → 2617); fmt clean, clippy `-D warnings` clean, nextest 2617/2617.
