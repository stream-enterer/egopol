# F010 investigation methodology — design spec

**Status:** draft
**Date:** 2026-04-26
**Scope:** one-off, F010 X+Z only (per brainstorming decision 2026-04-26)
**Approach:** rigid schema + cluster-first traversal + append-only log (Approach 4 per brainstorming sequence)

## Provenance

This spec is the output of a brainstorming sequence on 2026-04-26 that produced a hypothesis-category enumeration via three subagent passes (architectural grounding, differential constraint, adversarial pre-mortem) and a synthesis of those outputs (`synthesis-v2.md`). The methodology spec defines the *investigation protocol* — not a fix. The investigation's implementation phase is the running of the methodology; its deliverable feeds a separate fix-spec → fix-plan → fix-implementation cycle.

Authority order (higher wins on conflict):
1. CLAUDE.md (project instructions).
2. This spec.
3. `synthesis-v2.md` (the unified hypothesis-category list).
4. The two source artifacts (`architectural-grounding.md`, `differential-constraint.md`) and the pre-mortem (`premortem.md`).
5. The investigator's mid-flight judgement.

The spec's deliberate constraints (pre-registration, cluster-first traversal, append-only log, termination gate) are designed to defeat specific failure modes observed in prior investigation phases — narrative-fitting, rabbit-holing, acceptance-criterion drift, post-hoc hypothesis rewriting. Suspending a constraint requires an `escalate` log entry naming the constraint and the justification; silent suspension is a methodology violation.

---

## Section 1 — Scope and goal

**Goal:** produce converged evidence for F010 X+Z (panel interior renders solid black where C++ paints `DirContentColor` light grey; six-field info pane invisible) — either a single confirmed root cause backed by mechanical and manual evidence, or "all hypotheses falsified, escalate." That evidence feeds a separate fix-spec.

**Explicitly in scope:**
- F010 X (panel interior black) and F010 Z (info pane invisible). Y (border gradients) is not under investigation; its working state is evidence.
- Identifying which hypothesis (or combination) explains the X+Z-fail-but-Y-works pattern in the live GUI under the user's environment.
- Producing artifacts (pre-registration table, test harness, append-only log) sufficient for the fix-spec phase to act on without re-opening investigation questions.

**Explicitly out of scope:**
- Designing the fix. Even if a hypothesis is confirmed, the methodology's deliverable is converged evidence — not a code change.
- Generalizing the methodology for reuse on future bugs.
- Other F010 sub-symptoms (slow loading is F017; theme parse is closed).
- Reopening F018 closure or any other already-closed issue. Findings that contradict prior closures become *separate* outputs to ISSUES.json, not F010 redirects.

**Non-goal:** zero rabbit-holing. The methodology aims to make rabbit-holing expensive and visible via append-only logging, pre-registration, and cluster-first traversal — not to eliminate it.

---

## Section 2 — Hypothesis category checklist (seed)

The pre-registration table must contain at least one hypothesis per category in the unified checklist from `synthesis-v2.md`, OR an explicit "ruled out a priori, reason: …" entry for that category. New categories discovered during investigation are appended to the checklist with timestamps.

Seed categories (refer by ID; full descriptions in `docs/debug/investigations/F010-hypothesis-enumeration/synthesis-v2.md`):

- **Tier 1:** H1 (recording-mode dispatch hole — Clear silently dropped), H2 (tile pre-fill / background_color contract).
- **Tier 2:** H3 (render-strategy split), H4 (texture-sampling at replay — font-atlas), H5 (tile composite alpha re-blend), H6 (DrawList replay state-snapshot equivalence), P1 (GPU/atlas resource lifecycle), P2 (state-transition idempotency / hot-reload invalidation), P4 (async-prep ordering).
- **Tier 3:** H7 (Send-Sync soundness), H8 (GPU pipeline — sRGB, surface clear, present), H9 (SVP-boundary IsOpaque correctness), H10 (canvas_color snapshot/op-arg disagreement), H11 (debug_assert / push-pop pairing in release), P3 (vtable / trait-dispatch override missing), P5 (build-config-conditional code path), P7 (multi-target paint composition — tentative), P8 (coordinate-rounding to zero-area degenerate rects).
- **Joint blind spots:** B1 (theme/runtime-data correctness), B2 (panel state machine reaches VFS_LOADED), B3 (paint-not-reached for symptomatic panel), B4 (stale tile cache from prior frame), B5 (font cache initialization order), B6 (build-config / GPU-vendor / DPR env-only repros), B7 (recursive paint invocation safety), B8 (compositor dirty-tile invalidation timing).

**Total: 19 hypothesis categories + 8 blind spots = 27 entries minimum** in the pre-registration table.

Cluster memberships (referenced in pre-registration entries):
- `same-observable-with-H1`: H1, P8 — same observable (Clear is a no-op), different mechanisms.
- `invalidation-cluster`: P2, B1, B8 — right code, wrong pixels, via different invalidation paths.
- `dispatch-cluster`: P3, B2, B3 — Clear never called from the right place, via different dispatch failures.
- `order-config-cluster`: P4, B5, H11, P5 — environment- or order-dependent reproductions.

---

## Section 3 — Pre-registration template (rigid schema)

One YAML entry per hypothesis. Each entry is a separate file:

```
docs/debug/investigations/F010-investigation/hypotheses/<id>.yaml
```

Schema:

```yaml
id: H1                       # from category checklist
short_name: "Recording-mode dispatch hole — Clear silently dropped"
hypothesis_statement: |
  Precise claim about a mechanism that, if true, explains the
  X+Z-fail-but-Y-works pattern.
falsification_criterion: |
  An observation that, if seen, kills this hypothesis. Must be a single
  unambiguous observation. "Evidence consistent with X" is not allowed —
  Popperian falsification only.
experimental_design: |
  The specific experiment that produces the observation. Must be runnable
  from the harness (Section 4). Names which production code path is
  exercised.
evidence_shape: |
  What artifact the experiment produces — test pass/fail, op-stream JSON,
  image diff, etc.
falsification_action: |
  What to do if this hypothesis is falsified: which cluster mate to
  discriminate next, or "no successor — cluster boundary reached."
cluster_membership: ["same-observable-with-H1"]
```

**Lock rule.** The pre-registration table is *drafted* in phase 1 (before the harness exists) and *locked* at end of phase 2 (after harness construction validates each entry's `experimental_design` against what the harness can actually run). Phase-2 revisions to draft entries are permitted because the harness may reveal that a planned experiment is not realizable as written; once the harness is locked, the pre-registration is locked alongside it.

After lock: edits to existing entries are forbidden. Corrections are *new* entries with `supersedes: <old_id>` and an explanatory body. New hypotheses discovered mid-investigation are *appended* with their own IDs and timestamps; they cannot retroactively explain prior evidence.

---

## Section 4 — Test harness design rules

The harness is the minimal code needed to run every pre-registered experiment. F010-specific; not a general-purpose emPainter fixture.

Rules:

1. **Production-path fidelity.** The harness drives the same code path the live GUI uses for `emDirPanel::Paint`. Specifically: construct a recording painter, drive a real_stack panel tree through `emView::Paint`, replay the recorded DrawList into a target image. Direct-mode-only fixtures are insufficient — H1 only fires in recording mode.
2. **Op-level visibility.** The harness exposes the recorded `DrawOp` stream as a structured artifact (JSON via `DUMP_DRAW_OPS=1` or equivalent). Falsification criteria for H1, H4, H6, P8 require op-stream inspection.
3. **C++ comparison wired.** The harness supports diffing recorded ops against C++ ops via `scripts/diff_draw_ops.py` (already in tree).
4. **Cluster-discrimination support.** The harness can run the same scenario in *both* recording and direct mode for the same panel tree. Required for H1 vs P8 discrimination and any other same-observable cluster discrimination.
5. **YAGNI.** No infrastructure for hypotheses not pre-registered. If no entry needs a GPU-vendor matrix or a font-eviction simulator, none is built.

The harness is built in phase 2 (after pre-registration is complete) and locked at end of phase 2. Modifications during execution require an `observe` log entry explaining why.

---

## Section 5 — Cluster-first execution protocol

Investigation proceeds cluster by cluster. Per cluster:

1. **Pick a representative.** The cluster representative is the hypothesis with the simplest mechanical falsification test. Selection is logged as a `decide` entry.
2. **Run the representative's falsification experiment.** Falsified → mark `falsify`; advance to next cluster mate. Confirmed → proceed to step 3.
3. **Run each cluster mate's pre-registered falsification experiment.** Each mate's experiment serves dual duty: (a) falsifying the mate, and (b) discriminating the mate's mechanism from the representative's. A mate is correctly discriminated when its experiment produces a `falsify`-tier outcome (its falsification criterion is met) while the representative remains confirmed. If a mate's experiment *also* confirms — i.e. both the representative's falsification criterion and the mate's are not met — the cluster cannot be discriminated by the pre-registered experiments. The cluster is suspended via an `escalate` entry; investigation cannot advance until a new discrimination experiment is designed and the harness is extended.
4. **Declare cluster resolved** only when one hypothesis is confirmed AND every mate is `falsify`-marked. Logged as `confirm` entry citing both the representative's confirming `observe` entries and each mate's falsifying `observe` entries.
5. **Advance to next cluster.** Cluster ordering: cheapest expected experiment first, ranking fixed at end of phase 2 when the harness is locked. Deviations require `decide` log entries explaining the reordering.

Forbidden:
- Confirming a hypothesis without discrimination tests against all cluster mates.
- Declaring a cluster resolved when a mate is not discriminated.
- Skipping a cluster because "it probably isn't the cause."

---

## Section 6 — Evidence-recording conventions

Append-only investigation log:

```
docs/debug/investigations/F010-investigation/log/NNNN-<short>.md
```

Monotonic 4-digit counter, one file per entry.

Entry types:

- `observe` — experimental result with raw evidence (op stream, image diff, etc.).
- `decide` — methodology decision (cluster representative pick, ordering, etc.).
- `confirm` — hypothesis confirmation, citing supporting `observe` entries.
- `falsify` — hypothesis falsification, citing supporting `observe` entries.
- `revise` — correction to a prior entry; must specify `supersedes: NNNN` and explain.
- `escalate` — investigation cannot proceed; names the blocker.

Frontmatter:

```yaml
id: 0042
type: observe
timestamp: 2026-04-26T14:30:00Z
hypothesis_ids: [H1, P8]
supersedes: null
artifacts: [target/F010/h1-op-stream.json]
```

**Append-only enforcement** is by file-per-entry + git history. Edits and deletes are visible in `git log -p`. Silent rewriting is a methodology violation visible at review time.

---

## Section 7 — Termination gate

Two independent confirmation channels, both required:

1. **Mechanical channel.** A controlled test in the harness that drives the production code path the GUI uses, reproduces the symptom (or its closest mechanical analog with op-level visibility), and *flips from fail to pass when a candidate fix is applied*. Test added to the suite; passing in CI.
2. **Manual channel.** The user launches the live GUI under conditions that previously produced the symptom and confirms it is gone. This is the only required user touchpoint per full-autonomy decision. Logged as the terminating `confirm` entry.

Termination requires **all of**:
- One hypothesis is `confirm`-marked.
- Every cluster mate of that hypothesis has a `falsify` or fully-discriminated entry.
- All other clusters are resolved.
- Mechanical channel green.
- Manual channel positive.

Forbidden: declaring termination on the mechanical channel alone, or on a proxy signal. The F018 acceptance miss (where a non-user-visible signal substituted for user-visible verification) is the cautionary tale.

---

## Section 8 — Forbidden fix-shapes

The methodology hands off a forbidden-fix-shapes list to the fix-spec phase. M1 is the seed; new entries may be added during investigation if evidence reveals additional avoidance failure modes.

**M1 — Avoidance-fix bit-rot.** Forbidden: any "fix" that resolves the symptom by routing around the broken code path rather than fixing it. Concrete F010 examples explicitly forbidden:

- Forcing the per-tile direct branch always (bypasses the display-list path).
- Disabling the recording-painter entirely.
- Feature-flag-gated bypass of the suspected mechanism.

Test for avoidance-fix shape: *"If the bypassed path is re-enabled in 3 weeks, will the symptom return?"* If yes, fix is avoidance and forbidden.

The fix-spec phase must explicitly enumerate which forbidden shapes apply and demonstrate how its proposed fix avoids them.

---

## Section 9 — Stop conditions

The methodology halts in three modes:

1. **Success.** Termination gate satisfied per Section 7. Hand off to fix-spec.
2. **All-falsified.** Every pre-registered hypothesis falsified; every blind-spot attempted and falsified; no surviving candidate. Halt with `escalate` entry. User decides between (a) re-running pre-registration against an expanded checklist (new methodology cycle), or (b) accepting that investigation cannot proceed without new evidence.
3. **Blocked.** A cluster cannot be discriminated, or an unforeseen experiment cannot run. Halt with `escalate` entry naming the blocker. User decides between extending the harness (return to phase 2) or accepting blockage.

**Forbidden halt mode:** declaring success without termination-gate satisfaction. Specifically: "the mechanical test passes, ship it" without manual channel confirmation is a methodology violation.

---

## Glossary

- **Hypothesis** — a precise mechanism that, if true, would explain X+Z-fail-but-Y-works.
- **Falsification criterion** — a single observation that, if seen, kills the hypothesis.
- **Cluster** — a set of hypotheses that share an observable but differ in mechanism. Listed in Section 2.
- **Cluster representative** — the hypothesis with the simplest mechanical falsification test, selected per cluster as the first to test.
- **Cluster discrimination test** — an experiment whose evidence distinguishes one cluster mate's mechanism from another's.
- **Termination gate** — the two-channel confirmation required before the methodology can declare success.
- **Avoidance fix** — a fix that resolves the symptom by routing around the broken path rather than fixing it. Forbidden per M1.
- **Append-only log** — investigation log where prior entries are never edited or deleted; corrections are new entries with `supersedes:` references.

## Open questions for the writing-plans phase

These are deferred to the plan, not the spec:

1. **Cluster-discrimination cross-falsification.** When the falsification test for hypothesis A also incidentally falsifies cluster mate B, the methodology requires a *separate* discrimination test to verify B was not pseudo-falsified. The plan must specify how each cluster's discrimination test is constructed; the spec only requires that one exists per cluster mate.
2. **M1 operationalization.** The "would the symptom return if the bypassed path were re-enabled?" test is conceptual. The plan may need to operationalize it as a concrete checklist for the fix-spec phase.
3. **Harness construction order.** Phase 2 builds the harness based on registered hypotheses. The plan must order harness components so that the cheapest cluster's harness exists before any falsification runs (otherwise we'd build heavier infra than needed if the first cluster resolves the bug).
