# Phase 3 — Resume Handoff (checkpoint 1)

**Created:** 2026-04-20 after session context exhaustion mid-Phase-3.
**Checkpoint state:** `main` @ `23cfba47` (no-ff merge of `port-rewrite/phase-3` branch at `d558dd69`). Phase 3 is PARTIAL — Tasks 1, 2, and the Task 3+4 bundle (B3.1→B3.4d) are done; Tasks 5, 6, 7 + Closeout remain.

---

## Handoff prompt

Paste the section between the dashed rules below into the next session after `/clear`.

---

You are resuming the eaglemode-rs port-ownership-rewrite **Phase 3** at `/home/a0/git/eaglemode-rs`. Phase 3 is PARTIAL at `main` @ `23cfba47` — a checkpoint merge of the phase branch after Tasks 1, 2, and the Task 3+4 bundle (B3.1→B3.4d) landed green. Tasks 5, 6, 7 + Closeout C1–C11 remain. This is your job.

### Required reading before any dispatch (in order)

1. `CLAUDE.md` — Port Ideology, `#[allow]` whitelist, Do-NOT list. Binding.
2. `docs/superpowers/plans/2026-04-19-port-rewrite-phase-3-widget-signals.md` — read Tasks 5, 6, 7 and the Closeout header; skim Tasks 1–4 to understand what already landed.
3. `docs/superpowers/plans/2026-04-19-port-rewrite-bootstrap-ritual.md` — you are past Bootstrap; relevant sections are **Closeout C1–C11** plus `B11a` (not applicable, hook is active).
4. `docs/superpowers/specs/2026-04-19-port-ownership-rewrite-design.md` — §4 D4.9 (Task 5 emFpPlugin), §6 D6.1–D6.5 signal model context.
5. `docs/superpowers/notes/2026-04-19-phase-3-ledger.md` — the full mid-phase ledger with every commit's summary and rationale. Load this into context; the per-task entries are load-bearing for Closeout C5/C6.
6. `docs/superpowers/notes/2026-04-19-phase-3-baseline.md` — entry baseline (Phase-2 exit) for Closeout C3 delta computation.

### Do NOT re-run Bootstrap

Phase 3 Bootstrap (B1–B12) ran 2026-04-20. Baseline was captured. You are past it. Do NOT create a new branch — Tasks 5/6/7 land directly on `main` OR on a continuation branch `port-rewrite/phase-3-continue` cut from `main`. User preference (current session) is TBD — ask them on first user turn if they want a continuation branch.

### State summary at resume (verify before starting)

- **Branch:** `main` @ `23cfba47` (merged partial checkpoint). Working tree clean except for untracked `.claude/` (harness scratch; ignore).
- **Predecessor tag:** `port-rewrite-phase-3-partial-checkpoint-1` (created post-handoff — verify `git tag -l 'port-rewrite-phase-3*'`).
- **Gate at merge:** nextest **2472 passed / 0 failed / 9 skipped**; goldens **237 passed / 6 failed** (baseline preserved); clippy clean under active pre-commit hook.
- **Metric snapshot at checkpoint:**
  - `rc_refcell_total` = **257** (Phase entry baseline was 262; Task 2 drove −6; B3.4b added +1 around a test fixture that isn't load-bearing).
  - `diverged_total` = **172** (Phase entry 176; B3.4d deleted 4 obsolete DIVERGED blocks for `GetCheckSignal`/`CheckChanged`/`Clicked` on emCheckButton + emCheckBox).
  - `rust_only_total` = **18** (Phase entry 17; Task 1 added `emInputDispatchEngine.rust_only`).
  - `DIVERGED-B3.*` transient markers = **0** (all resolved by B3.4d).
  - `phase3_ignored_tests` = **0** (all 11 B3.3/B3.4a/b-era ignores removed by end of B3.4d; test count at keystone was 2472/0/9 with the 9 skipped being pre-existing).
- **Pre-commit hook:** **ACTIVE**. Every commit from B3.4d keystone onward runs fmt/clippy/nextest. Do NOT disable unless a new stage-only cascade emerges (would need user consent + B11a-style ledger entry).

### JSON entries remaining

- **E024** (emFileDialog polling pattern). Current `status: "open"` in `docs/superpowers/notes/2026-04-19-port-divergence-raw-material.json` (line 894). **Task 6 closes it.**
- **E025** (`GetCheckSignal` → `Option<Box<dyn FnMut(bool)>>` callback on checkbutton/checkbox/radiobutton). Current `status: "open"` (line 924). **The Task 3+4 bundle architecturally resolved this** (widgets now have real `SignalId` fields that fire per C++, matching the C++ emSignal semantics). Close E025 in Closeout C5/C6 with evidence citing commit `33d25c72` (B3.4d keystone) — the DIVERGED blocks that E025 named are the ones B3.4d deleted.

### Invariants already satisfied (verified at checkpoint)

- **I3a** (every widget has `_signal: SignalId` field per C++ `GetXxxSignal`): PASS. emCheckButton/emCheckBox (via forced-divergence mirror)/emButton/emRadioButton/emTextField/emColorField/emScalarField/emFileSelectionBox/emListBox/emSplitter/emDialog all have the field(s). emCheckBox's field is a Rust-only mirror of emCheckButton's (C++ inherits; Rust can't). Documented as forced at field definition.
- **I3b** (widget `Box<dyn FnMut(` types all go through `WidgetCallback<Args>` or `WidgetCallbackRef<T>` alias): PASS for widget callbacks. Non-widget `Box<dyn FnMut(` remaining in `crates/emcore/src/` are `emMiniIpc::MessageCallback`, `emPainter::op_log_fn`, `emPriSchedAgent::callback`/`got_access`, plus `ValidateCb`/`DialogCheckFinishCb` (return-value divergence — bool returns, not fit for the alias). All are annotated with rationale comments. **Task 7 will re-assert I3b and either tighten the invariant wording to "widget callbacks only" or accept the documented exclusions — the ledger notes the expected relaxation.**
- **I3c** (clipboard on emGUIFramework not emContext): PASS (Task 2).
- **I3e** (InputDispatchEngine registered as framework-owned top-priority): PASS (Task 1, at `Priority::VeryHigh`).

### Invariant still to satisfy

- **I3d** (emFpPlugin methods take `&mut impl ConstructCtx` / `&mut dyn ConstructCtxObj`): **Task 5's job.** `ConstructCtxObj` object-safe shim will live in `emEngineCtx.rs`. All plugin implementors migrate.

### Pending work — Task 5, 6, 7, Closeout

1. **Task 5 — emFpPlugin API migration.** Plan file Task 5 section is authoritative. Summary:
   - Add `ConstructCtxObj` object-safe shim to `emEngineCtx.rs` (blanket `impl<T: ConstructCtx> ConstructCtxObj for T`).
   - Extend `emFpPlugin::CreateFilePanel`/`TryCreateFilePanel`/`CreateFilePanelWithStat`/`SearchPlugin` to take `&mut dyn ConstructCtxObj`.
   - Migrate every `impl emFpPlugin for` implementor.
   - Migrate test fixtures (`fp_plugin.rs`, `plugin_invocation.rs`, `dynamic_plugins.rs`).
   - Gate: nextest ≥ 2472, goldens 237/6. One commit; hook active; no cascade.
2. **Task 6 — emFileDialog polling → signal-based (closes E024).** Plan Task 6 is authoritative. Summary:
   - Allocate `result_signal: SignalId` at dialog construction (via `ConstructCtx` — this ties in naturally with Task 5's plumbing landing).
   - Replace polling methods with signal-based observer wiring.
   - Commit message closes E024.
3. **Task 7 — Full gate + invariants.** Re-run all gates + I3a/I3b/I3c/I3d/I3e. Document I3b exclusions in a ledger note (or tighten the wording). If any invariant fails, fix and re-gate.
4. **Closeout C1–C11.** Per ritual. Key points:
   - **C2/C3:** write `2026-04-19-phase-3-exit.md` with exit metrics vs baseline.
   - **C5/C6:** mark E024 + E025 `resolved-phase-3` in the raw-material JSON, add `resolution_commit` SHAs. Dedicated commit per C6.
   - **C7:** write `2026-04-19-phase-3-closeout.md`.
   - **C9:** merge `port-rewrite/phase-3-continue` (if used) into `main` with `--no-ff`. **Explicit user confirmation required** (precedent: Phase 1.75/1.76/2 all required). Note that `main` already contains the partial checkpoint — C9 merge will be the final-Task-5-6-7 branch; the merge commit history will show the phase in two steps.
   - **C10:** tag `port-rewrite-phase-3-complete`.
   - **C11:** announce.
5. **Final status line expected:** `COMPLETE — all C1–C11 invariants SAT. E024/E025 resolved.`

### Discipline reminders (echo into subagent prompts)

1. "Clean up divergences, workarounds, hacks; not create new ones." — CLAUDE.md.
2. No new `#[allow(...)]` outside whitelist (too-many-arguments, non_snake_case on emCore module, non_camel_case_types on em-prefixed types).
3. No new `Rc<RefCell<>>`, `Weak`, `Any`/downcast, `Arc`/`Mutex`, `Cow`, throwaway schedulers.
4. No new `unsafe` without destructuring attempt first (see `feedback_destructure_before_unsafe.md`).
5. C++ fidelity — read each `.cpp` before writing signal-fire / plugin code.
6. Review every task: dispatch spec + code-quality reviewer subagents on Tasks 5, 6 (see `feedback_review_every_task.md`).
7. Post-commit: run the plan's self-review grep assertions; report counts.

### Carryover traps (from earlier phases + mid-phase-3)

1. **`PanelCtx: ConstructCtx`** impl panics on `create_signal`/`register_engine` when scheduler absent. Production paths always supply one; layout-only unit-test `PanelCtx::new` paths don't and will panic if they construct widgets. B3.4b subagent flagged this as a risk. Task 5 should not trip it (plugin construction happens at framework-init via `InitCtx` OR during Cycle via `EngineCtx` — both have schedulers). But if a test fixture constructs a plugin via raw `PanelCtx::new`, that will panic — rewrite the fixture.
2. **emCheckBox `check_signal`** is a forced-divergence mirror of emCheckButton (C++ inherits; Rust can't). Task 5/6 don't touch it. If the Closeout reviewer flags it, cite the forced-divergence DIVERGED comment at the field.
3. **Task 2 Issue-1-style asymmetric None-handling** — avoid. When adding new `Option<&mut X>` fields, either require all sites to supply or use fallback pattern uniformly. Don't mix panic and fallback.
4. **Test-fixture `view_rc` holdouts** still scheduled for Phase 5 removal — do NOT touch in Phase 3 (noted in Phase 2 closeout).
5. **`WidgetCallbackRef<T>`** alias exists for lifetime-parametric payloads (`&str`, `&DialogResult`, `&[usize]`). Task 6's `emFileDialog::result_signal` may want a matching callback surface — if so, `WidgetCallback<DialogResult>` (by-value) or `WidgetCallbackRef<DialogResult>` (borrow) as fits. Match C++.
6. **emCoreConfigPanel** has `#[allow(clippy::too_many_arguments)]` on `make_factor_field` (added in B3.4b). Whitelist-approved; leave alone.
7. **Pre-commit hook is active.** Every commit runs fmt/clippy/nextest. Don't `--no-verify`. If a commit can't pass the hook, fix the root cause or split into smaller commits.

### Subagent model guidance

- Tasks 5 and 6 are moderate scope — opus is justified given the C++ fidelity requirement on Task 6.
- Task 7 is a mechanical gate + invariant sweep — sonnet is fine.
- Closeout C1–C11 is ceremony + JSON edits + ledger writing — mix of manual + opus-for-closeout-note.

### If a subagent halts or escalates

- BLOCKED on context: provide more, re-dispatch.
- BLOCKED on reasoning: escalate to more capable model.
- BLOCKED on plan gap: STOP, write `docs/superpowers/notes/2026-04-19-phase-3-<task>-blocked.md`, ask the user.
- Do NOT reach for new `Rc<RefCell<>>`, `Any`, `Weak`, or `unsafe` to make blockage go away.

### Begin with

1. Read the required docs in order.
2. `git status`, `git log --oneline -10`, `git tag -l 'port-rewrite-phase-3*'` to confirm the checkpoint state.
3. Ask the user: "Continuation branch or direct-on-main for Tasks 5–7 + Closeout?" Their previous preference (across the series) has been `port-rewrite/phase-<N>` branches merged `--no-ff` into main at Closeout. Default to cutting `port-rewrite/phase-3-continue` from `main` unless they say otherwise.
4. Dispatch Task 5 via `superpowers:subagent-driven-development`. Plan file Task 5 is the brief.

---

End of handoff prompt.

## Checkpoint audit (for humans, not the resume prompt)

**What this session accomplished (Phase 3 Tasks 1–4 bundle):**

| Task | Commit(s) | Summary |
|---|---|---|
| Bootstrap | `5d88d776` | Baseline captured, ledger opened, branch cut. |
| Task 1 | `c7ca9971` + `766ed77c` | InputDispatchEngine + pending_inputs wiring. +1 rust_only marker. nextest 2458→2459. |
| Task 2 | `8fb275f4` + `e167d895` | Clipboard relocation to emGUIFramework (chartered §3.6(a)). rc_refcell 262→256 (−6). |
| Task 3+4 bundle | `59d5daf0` (B3.1) → `33d25c72` (B3.4d keystone) | 10 commits across 4 sub-phases: PanelCtx extension, widget Input sig, WidgetCallback alias, signal allocation + fires + DIVERGED deletions. nextest 2459→2472 (+13), diverged 176→172 (−4 obsolete blocks). |

**Sub-phase split rationale (Task 3+4 bundle):** Two full-bundle dispatches returned BLOCKED — the subagents correctly assessed the bundle as too large for single-session gate-green verification. Split into B3.1–B3.4d, each individually gate-green. User approved Option C (bundle) and subsequently the 4-sub-phase split. Pre-commit hook disabled for the cascade; B3.4d re-enabled and made its commit under the active hook.

**Not done in this session:**
- Task 5 (emFpPlugin API) — I3d still to satisfy.
- Task 6 (emFileDialog signals, closes E024).
- Task 7 (full gate + invariants).
- Closeout C1–C11. E025 is architecturally resolved but the JSON entry hasn't been marked `resolved-phase-3` yet (C6's job).

**Branch state:** `port-rewrite/phase-3` still exists at `d558dd69`. `main` is at `23cfba47` (the merge). The branch can be deleted after resume (Task 5–7 land on `port-rewrite/phase-3-continue` or direct-on-main per user pref), OR kept around until Closeout for git-blame clarity.
