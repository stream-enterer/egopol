# Phase 4c — Baseline

**Captured:** 2026-04-21 (bootstrap)
**Branch:** port-rewrite/phase-4c
**Parent:** main @ 3afcdaaa (phase-4c prep: handoff prompt for fresh session)

## nextest

```
Summary [15.332s] 2562 tests run: 2562 passed, 9 skipped
```

## goldens

```
test result: FAILED. 237 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

(6 failures are the longstanding pre-existing goldens unchanged since Phase 4a baseline.)

## clippy

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.78s
```

Exit 0. No warnings.

## rc_refcell_total

```
351
```

## diverged_total

```
182
```

## rust_only_total

```
18
```

## idiom_total

```
0
```

## try_borrow_total

```
0
```

## Notes

All seven metrics match Phase 4b.1's exit exactly — expected, since no work has
landed on main since 4b.1 closed. B8 green (nextest 0 failed, goldens match
baseline, clippy clean).
