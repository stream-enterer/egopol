# Phase 4d — emRec Persistence — Baseline

**Captured:** 2026-04-22
**Branch basis:** main @ 9378be7786d6e67da537eb32c4765f38e835c004

## nextest

2613 tests run: 2613 passed, 9 skipped (0 failed).

## goldens

237 passed; 6 failed; 0 ignored — matches 2026-04-18 emview closeout baseline.

## clippy

`cargo clippy --all-targets --all-features` — exit 0, no warnings.

## rc_refcell_total

421

## diverged_total

224

## rust_only_total

18

## idiom_total

0

## try_borrow_total

0

---

## Notes

Handoff asserted `rc_refcell_total: 290` at Phase 4c exit; this Bootstrap's
`rg -c 'Rc<RefCell<' crates/` reports 421 on the same tree. Same regex,
possibly different file-scope filtering in the handoff capture. Recording the
authoritative count here as the Phase 4d baseline — closeout deltas are
computed against this number, not the handoff figure.

Phase-4c exit metrics preserved for reference:
- nextest 2613 (match), goldens 237/6 (match), try_borrow_total 0 (match).
