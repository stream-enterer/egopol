# Phase 4c — Exit

**Captured:** 2026-04-22
**Branch:** port-rewrite/phase-4c
**Tip:** 27883ca2 (phase-4c: ledger — fill in Task 7 SHA)

## nextest

```
Summary [15.150s] 2613 tests run: 2613 passed, 9 skipped
```

## goldens

```
test result: FAILED. 237 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

Same 6 pre-existing failures carried forward from baseline:
- `composition::composition_tktest_1x`
- `composition::composition_tktest_2x`
- `notice::notice_window_resize`
- `test_panel::testpanel_expanded`
- `test_panel::testpanel_root`
- `widget::widget_file_selection_box`

## clippy

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
```

Exit 0. No warnings. (`--all-targets --all-features -D warnings`.)

## cargo fmt --check

Clean (exit 0, no output).

## rc_refcell_total

```
290
```

## try_borrow_total

```
0
```

## Delta

| Metric              | Baseline | Exit  | Delta      |
|---------------------|----------|-------|------------|
| nextest tests       | 2562     | 2613  | +51        |
| nextest failed      | 0        | 0     | 0          |
| nextest skipped     | 9        | 9     | 0          |
| golden passed       | 237      | 237   | 0          |
| golden failed       | 6        | 6     | 0          |
| clippy warnings     | 0        | 0     | 0          |
| cargo fmt           | clean    | clean | –          |
| rc_refcell_total    | 351      | 290   | −61        |
| try_borrow_total    | 0        | 0     | 0          |
| new unsafe blocks   | —        | 0     | 0          |

All deltas satisfy Closeout C3 constraints:
- nextest ≥ baseline (+51, all passing)
- goldens passed ≥ baseline (237 = 237)
- goldens failed ≤ baseline (6 = 6)
- rc_refcell_total did not increase (I4c-8) — decreased by 61
- try_borrow_total held at 0 (I4c-8)
- No new `unsafe` blocks introduced (I4c-8)

## JSON entries closed

None — Phase 4c closes no entries (E026/E027 land at Phase 4e; persistence at Phase 4d).
