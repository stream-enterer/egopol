# Combined Reviewer Template — F010 Signal-Drift Tier-B

A single subagent dispatch that runs both spec-compliance and code-quality checks against a bucket implementation. Replaces the legacy two-subagent flow (separate spec-reviewer + code-reviewer dispatches), which re-tokenized overlapping context. Partitioning the *checklist* instead of the *subagent* preserves quality at ~30–40% of the legacy review-pass token cost.

## How to use

1. Pick a remaining bucket from `work-order.md`. Identify its design doc, bucket sketch, base SHA, head SHA, baseline test count, and worktree path.
2. Fill in every `{PLACEHOLDER}` in the prompt template below.
3. Dispatch as a single foreground `general-purpose` subagent.
4. If the report is `OVERALL: FIXUP REQUIRED`, dispatch a fixup subagent with the report content as input. After fixup, run this combined reviewer again on the new HEAD.
5. If `OVERALL: APPROVED FOR MERGE`, proceed with the standard merge workflow.

The template is self-contained — a fresh subagent with zero conversation context can execute it correctly given only the placeholder fills.

## Prompt template

```
You are reviewing a completed F010 Signal-Drift Tier-B bucket implementation. Run BOTH spec-compliance and code-quality checklists in one pass — do not split into two reviews.

## Bucket context

- Bucket: {BUCKET_ID} ({BUCKET_TITLE})
- Worktree: {WORKTREE_PATH}
- Branch: {BRANCH_NAME}
- Base: main HEAD ({BASE_SHA})
- Head: {HEAD_SHA}
- Baseline test count entering this bucket: {BASELINE_TEST_COUNT}

## Inputs to read (each exactly once)

- Design doc: docs/superpowers/specs/2026-04-27-{BUCKET_ID}-{SLUG}-design.md
- Bucket sketch: docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/buckets/{BUCKET_ID}-{SLUG}.md
- Cited D-### entries in: docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/decisions.md (read only the entries the bucket cites)
- Diff: `git diff {BASE_SHA}..{HEAD_SHA}` (run from the worktree)
- Touched files: only the files surfaced by the diff

## Spec-compliance checklist

- Every row in the design doc's row table is observably resolved in the diff. Map row → diff site explicitly.
- Every cited D-### decision is honored. Notable cluster conventions to verify:
  - **D-006** first-Cycle init shape (subscribe in first Cycle, gated on `subscribed_init: bool`).
  - **D-007** mutator-fire pattern. The ectx parameter on mutators MUST use the trait bound `&mut impl SignalCtx`, not the literal `&mut EngineCtx<'_>`. Both `EngineCtx` and `SchedCtx` impl `SignalCtx`. Any literal `&mut EngineCtx<'_>` in a new mutator signature is a deviation.
  - **D-008 A1** combined-form accessor. The accessor signature MUST be `GetXxxSignal(&self, ectx: &mut impl SignalCtx) -> SignalId`. Any split form (`Ensure*Signal` + `Get*Signal`) is a deviation.
  - **D-009** polling-intermediary-replacement (if relevant). Removed intermediary, fired synchronously.
- Click-through tests exist for every signal-subscribe wire (non-negotiable per cluster convention). One construction-only test does not satisfy this.
- Audit-data corrections noted in the design doc are applied (e.g., reclassifications, accessor-status fixes).
- Any row absorbed by a prior merge (e.g., absorbed by B-003's R-A drop) is annotated in the diff or design doc.
- Forced divergences are annotated correctly: `DIVERGED:` blocks carry both a forced-category tag (language-forced / dependency-forced / upstream-gap-forced / performance-forced) AND a test-result cite. Blocks without a category are fidelity bugs, not forced divergences.

## Code-quality checklist

- CLAUDE.md ownership rules: every `Rc<RefCell<T>>` carries a justification comment citing one of (a) cross-closure reference held by winit/wgpu callbacks, or (b) context-registry typed singleton. `Weak<RefCell<T>>` is acceptable only as the pair of an (a)-justified `Rc<RefCell<T>>`.
- CLAUDE.md naming: File and Name Correspondence preserved. Any rename carries a `DIVERGED:` annotation with the C++ name and reason.
- No `#[allow(...)]` / `#[expect(...)]` suppressions of fixable warnings (exceptions: too-many-arguments, `non_snake_case` on the `emCore` module, `non_camel_case_types` on `em`-prefixed types).
- No `Arc`, no `Mutex`, no `Cow`, no glob imports outside `#[cfg(test)] use super::*`.
- Test scaffolding is reasonable. If the diff has >30 lines of repeated stub-engine setup across multiple test functions, flag as "extract helper" debt — not a hard block, but a debt item.
- No dead code, no half-finished implementations, no leftover scaffolding (e.g., commented-out blocks, debug prints, unused stubs).
- Test count must be ≥ {BASELINE_TEST_COUNT}. Verify by running the test suite or by counting nextest-listed tests in the diff.
- Pre-commit hook clean: cargo fmt, clippy -D warnings, nextest. If any of these fail, that is a hard block — do not approve.
- `cargo xtask annotations` clean (validates DIVERGED/RUST_ONLY annotation hygiene).

## Report format

Return a single message with the structure below. Be terse — one line per finding. File:line citations are mandatory for findings.

```
SPEC COMPLIANCE: [APPROVED | CHANGES_REQUESTED]
- {file:line} — {issue} ({severity: critical | important | minor})

CODE QUALITY: [APPROVED | CHANGES_REQUESTED]
- {file:line} — {issue} ({severity: critical | important | minor})

DEVIATIONS FROM DESIGN (if any):
- {short description} — disposition: {forced | cluster-convention | open}

NEW PATTERNS (if any — promotion candidates if 3+ sightings):
- {short description}

DEBT ITEMS (if any — for PM session inventory):
- {file:line} — {description}

OVERALL: [APPROVED FOR MERGE | FIXUP REQUIRED | BLOCKED]
```

## Constraints

- Do not commit.
- Do not modify the worktree.
- Do not write the report to disk — return it as your message text.
- Do not invent findings. If a check passes, list nothing under that section.
- Do not split the review into multiple subagent dispatches; this template is the one pass.
```

## Notes for PM session

- "Cluster convention" disposition covers the documented post-merge supersessions (D-008 A1 combined form, D-007 trait bound). When the design doc was written before these amendments and the implementation correctly applied the post-amendment shape, that is `cluster-convention` — not a real deviation.
- The "DEBT ITEMS" section feeds the work-order reconciliation log. Test-scaffold extract-helper notes belong here.
- The "NEW PATTERNS" section feeds D-### promotion bookkeeping. Cross-check sighting counts against the relevant D-### watch-list paragraph in `decisions.md`.
