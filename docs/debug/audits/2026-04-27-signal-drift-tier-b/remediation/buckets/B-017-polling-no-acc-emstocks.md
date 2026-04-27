# B-017-polling-no-acc-emstocks — P-007 — emstocks polling consumers + missing accessors

**Pattern:** P-007-polling-accessor-missing
**Scope:** emstocks
**Row count:** 3
**Mechanical-vs-judgement:** balanced
**Cited decisions:** D-004-stocks-application-strategy, D-005-poll-replacement-shape, D-006-subscribe-shape.

**Prereq buckets:** B-004-no-wire-misc (hard, row 2 only — `emFilePanel::GetVirFileStateSignal` from G1), B-001-no-wire-emstocks (hard, row 1 only — `emStocksPricesFetcher::GetChangeSignal` from G3).

**Reconciliation amendments (2026-04-27, post-design a27d2faa):**
- **Bucket sketch's "emTimer::TimerCentral unported" claim is stale.** TimerCentral IS ported at `crates/emcore/src/emTimer.rs` and exposed via `Scheduler::create_timer/start_timer/is_running`. Active consumers in `emMiniIpc.rs` and integration tests. Strike from bucket framing.
- **2 hard cross-bucket prereqs encoded** in `inventory-enriched.json`:
  - row 2 (`emStocksFilePanel-34`) → `emFilePanel-accessor-vir-file-state` (B-004 G1)
  - row 1 (`emStocksFetchPricesDialog-62`) → `emStocksPricesFetcher-accessor-model-change` (B-001 G3)
- **B-001 G3 reconciliation flag resolved.** B-001's open question — "G3 ported but no in-bucket consumer; B-001 amendment candidate" — answered by B-017 row 1. The accessor port stays in B-001 (no row reassignment); the consumer wiring stays in B-017. Edge encoded.
- **3 accessor groups:** G-ext1 (B-004's `GetVirFileStateSignal`, consumed by row 2), G-ext2 (B-001's `emStocksPricesFetcher::GetChangeSignal`, consumed by row 1), G-int1 (in-row `save_timer_signal` on `emStocksFileModel` allocated via `Scheduler::create_timer`; row 3, no external accessor).
- **Recommended PR staging:** B-004 G1 + B-001 G3 land first; B-017 lands as one PR after both. Row 3 is natural pilot if review pressure forces staging (first emstocks model engine registration).

## Pattern description

Polling consumer paired with a missing accessor: the Rust site re-reads source state each tick (or compares before/after) without ever calling `IsSignaled` on a corresponding signal, and the producing model also never exposed/allocated the signal accessor. Fix requires both adding the missing accessor on the producer and rewriting the consumer per D-005 (direct subscribe). In this bucket all three sites live in emstocks across a fetcher dialog, the file panel, and the file model itself (where one row is upstream-gap-adjacent because `emTimer::TimerCentral` is unported).

## Rows

| ID | C++ site | Rust site | Accessor status | Notes |
|---|---|---|---|---|
| emStocksFetchPricesDialog-62 | src/emStocks/emStocksFetchPricesDialog.cpp:62 | crates/emstocks/src/emStocksFetchPricesDialog.rs:91 | missing | Cycle polls fetcher.HasFinished() and unconditionally calls UpdateControls; no IsSignaled / connect on ChangeSignal |
| emStocksFilePanel-34 | src/emStocks/emStocksFilePanel.cpp:34 | crates/emstocks/src/emStocksFilePanel.rs:354 | missing | Cycle polls vir-file-state via before/after compare; same drift as emFileLinkPanel (F010 root cause) |
| emStocksFileModel-41 | src/emStocks/emStocksFileModel.cpp:41 | crates/emstocks/src/emStocksFileModel.rs:62 | missing | Save timer simulated via Instant; emTimer::TimerCentral unported, no SignalId allocated |

## C++ reference sites

- src/emStocks/emStocksFetchPricesDialog.cpp:62
- src/emStocks/emStocksFileModel.cpp:41
- src/emStocks/emStocksFilePanel.cpp:34

## Open questions for the bucket-design brainstorm

- PR-staging: should one of the three rows (e.g., emStocksFilePanel-34, which mirrors the F010 root-cause shape) pilot the pattern fix and merge before the other two land? (D-004 deferred item.)
- For each consumer that polls multiple sources, confirm whether the C++ original subscribes individually or to an aggregated signal; default mirror C++. (D-005 deferred item.)
- emStocksFileModel-41 depends on `emTimer::TimerCentral`, which is unported — does this row require a TimerCentral port (or a chartered RUST_ONLY substitute) as a prereq, or can the save-timer signal be allocated independently of TimerCentral?
- For each missing accessor, confirm the producer-side allocation (SignalId on the model) lands in the same change as the consumer rewrite, vs. split into a separate prep commit.
- Confirm Phase-3 clustering did not split other emstocks P-007 rows into adjacent buckets that should be merged here.
