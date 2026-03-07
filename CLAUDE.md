# EGOPOL

Cargo workspace with two crates:
- `zuicchini/` — UI framework library (reimplementation of Eagle Mode's emCore in Rust)
- `egopol/` — game binary, depends on zuicchini via path

## Commands

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo run -p egopol
```

## Pre-commit hook

Runs `cargo fmt` (auto-applied) then `clippy -D warnings` then `cargo test`.
Do not skip with `--no-verify`. If a commit fails, fix the cause and retry.

## Code Rules

- **Types & coordinates**: `f64` logical, `i32` pixel, `u32` image dims, `u8` color channels.
- **Color**: `Color` (packed u32 RGBA) for storage. Intermediate blend math in `i32` or wider.
- **Ownership**: `Rc`/`RefCell` shared state, `Weak` parent refs.
- **Strings**: `String` owned, `&str` params. Convert with `.to_string()`.
- **Errors**: Per-module `Result` with custom error enums (`Display` + `Error`). `assert!` only for logic-error invariants.
- **Imports**: std → external → `crate::`. Explicit names. `use super::*` only in `#[cfg(test)]`.
- **Construction**: `new()` primary, builder `with_*(self) -> Self` for optional config.
- **Modules**: One primary type per file. Private `mod` + public `use` re-exports in `mod.rs`.
- **Visibility**: `pub(crate)` default. `pub` only for library API consumed by `egopol`.
- **Unwrap**: `expect("reason")` unless invariant is obvious from context. Bare `unwrap()` fine in tests and same-line proofs.
- **Warnings**: Fix the cause (remove dead code, prefix `_`, apply clippy fix). Suppress only genuine false positives with a comment.

## Do NOT

- `#[allow(...)]` / `#[expect(...)]` — fix the warning instead
- `Arc` / `Mutex` — single-threaded UI tree
- `Cow` — use `String` / `&str`
- Glob imports (`use foo::*`) — except `use super::*` in tests
- Truncate color math to `u8` mid-calculation
- `assert!` for recoverable errors
- `--no-verify` on commits
