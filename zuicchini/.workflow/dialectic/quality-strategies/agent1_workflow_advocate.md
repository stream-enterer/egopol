# Meta-Strategies for LLM-Dispatched Quality Assurance: A First-Principles Analysis

## Thesis

The optimal second-pass quality strategy for this project is dominated by a single asymmetry: **mechanically verifiable work produces reliable value; subjective judgment under token pressure produces unreliable value.** The 8 proposed strategies span a spectrum from fully empirical (pixel diffing) to fully analytical (rationale review). The project's current state -- 31 fixes applied, 1144 tests passing, 30 remaining LOWs mostly deferred with justification -- means the highest-value work is not "fix more things" but "verify the 31 fixes are actually correct and the 10 closures are actually benign." The strategies that accomplish this most reliably are those that produce binary pass/fail outputs from mechanical processes, not those that ask an agent to read code and render a judgment.

## The Five Structural Constraints

### 1. Context window economics favor decomposition into single-function units

The Border audit report illustrates the problem. border.rs is 2676 lines. The C++ emBorder is 1970 lines. A fix diff for one finding might be 20 lines. An agent tasked with "verify fix 20 (substance_round_rect coefficient) is correct" needs to read: the finding description (10 lines), the diff (20 lines), the surrounding Rust code (100 lines), and the C++ reference (50 lines). That is 180 lines -- comfortably within budget. An agent tasked with "verify all 7 Border fixes are correct" needs to hold 7 findings, 7 diffs, and cross-reference them against two 2000-line files. That is a qualitatively different task that will trigger lossy summarization failure modes.

The implication: **strategies that decompose into per-finding or per-function verification units are structurally superior to strategies that require holistic file-level reasoning.** Strategy 1 (adversarial re-audit of fix diffs) is excellent if dispatched per-finding. Strategy 2 (cross-fix consistency) is inherently holistic and therefore inherently unreliable when delegated to agents.

### 2. Verification asymmetry: reading diffs is harder than running tests

A fundamental confusion in LLM-assisted workflows is treating "agent reads a diff and says it's correct" as verification. It is not verification -- it is a second opinion. The failure mode is identical to the first opinion: the agent may misread the C++ reference, misunderstand the Rust semantics, or declare victory after superficial pattern matching.

True verification produces a bit: pass or fail. The project already has the infrastructure for this. Golden tests compare pixels. `MEASURE_DIVERGENCE=1` quantifies drift. The C++ golden test generator can be extended to cover specific scenarios. When an agent writes a new golden test case that exercises a fixed code path and the test passes, that is verification. When an agent reads a diff and writes "LGTM, the coefficient is now 0.023 matching C++ line 634," that is an echo of the fix itself.

The implication: **strategies that produce new test cases are more valuable than strategies that produce prose judgments.** Strategy 5 (mutation testing on CLOSED items) is underrated precisely because it produces tests. Strategy 3 (rationale adversarial review) is overrated precisely because it produces prose.

### 3. Diminishing returns favor validation over new fixes

The project's fix curve tells a clear story. Session 1 fixed 19 items including 3 HIGHs, systemic CC-06 hit-test bugs, and keyboard handling gaps. Session 2 fixed 12 items including the Border coefficient and geometry bugs. Session 3 resolved the remaining 30 items, with 8 code fixes, 14 deferred, and 10 closed. The severity gradient has flattened: what remains is LOWs, deferred infrastructure gaps (Cycle engine, FileItemPanel), and design-decision closures.

Fixing the remaining DEFERRED items (RadioButton Drop re-index, FileSelectionBox reactive layer) requires building infrastructure that does not exist yet. These are not bugs amenable to LLM point fixes -- they are feature implementations requiring hundreds of lines of new code, architectural decisions about Cycle/signal systems, and integration testing. An LLM subagent dispatched to "implement the FileSelectionBox reactive layer" will produce something, but the probability of it being correct, idiomatic, and compatible with the existing architecture is low without extensive iteration.

Meanwhile, the 31 fixes that were applied are assumed correct because the 1144 tests pass. But test passage is necessary, not sufficient. The golden tests cover specific scenarios. Fix 20 changed a coefficient from 0.006 to 0.023 -- but the substance_round_rect function is noted as "not directly golden-tested." Fix 21 changed label_space to use pre-HowTo width -- but coverage is noted as "likely uncovered for how_to + label combination." Fix 22 rewrote best_label_tallness -- but coverage for "icon + aux combinations" is noted as uncovered.

The implication: **the highest-value work is writing golden tests for the 31 fixed functions, not fixing more LOWs.** Every fix that lacks golden test coverage is a regression waiting to happen.

### 4. Agent failure modes are asymmetric across task types

The context brief documents seven failure modes from the actual workflow. Let me classify them by task type:

**Generation failures** (producing code):
- Over-fixing: refactored surrounding code while fixing a one-line bug
- Wrong function called: wired word_boundary instead of word_index

**Verification failures** (reading/judging):
- Early completion: fixed 4 of 12 items and declared the session productive
- Lossy derivative documents: manually-compiled checklists missed items
- Silent skipping: marked items ACCEPTED without justification
- Stale metadata: summary.md drifted out of sync

The pattern is revealing. Generation failures produce wrong code that tests can catch. Verification failures produce wrong metadata that nothing catches -- they accumulate silently. The "stale metadata" and "silent skipping" failures are particularly insidious because they corrupt the trust chain that all subsequent decisions depend on.

This means: **agents are more dangerous as auditors than as coders.** A coding agent that makes a mistake will likely be caught by the pre-commit hook (clippy, tests). An audit agent that declares a CLOSED item "correctly closed" when it is actually a real bug will never be caught unless someone re-examines it.

The implication: strategies that use agents as coders (write tests, write fixes) are safer than strategies that use agents as judges (review rationales, assess closures). Strategy 3 (rationale adversarial review) asks agents to do exactly the thing they are worst at: render a subjective judgment about whether a previous agent's justification is sound. Strategy 5 (mutation testing) asks agents to do what they are best at: write concrete code that the test framework evaluates mechanically.

### 5. Composition effects are real but testable

31 independently-correct fixes applied over 3 sessions. Each fix passed all tests in isolation. But consider: Fix 6 changed Label alignment defaults from Center to Left. Fix 22 rewrote best_label_tallness to include icon geometry. Fix 26 changed label_layout for description-only labels. These three fixes all modify the label rendering pipeline. Did anyone test them together? The golden tests exercise specific widget configurations. If no golden test has an icon + description-only label in a Border with HowTo, then three individually-correct fixes could interact to produce an incorrect composite result.

The TestPanel and TkTest composition layers are designed exactly for this -- they render all widgets together in a single tree. But the coverage notes say TestPanel has only 2 tests (root, expanded) and TkTest has "7/9 gaps closed" with missing sections for Tunnels, Test Dialog, and File Selection.

The implication: **full-scene rendering tests are the cheapest way to detect composition effects.** Not agent-by-agent re-review of each fix, but rendering the entire widget tree and comparing it against C++ output. Strategy 6 (empirical pixel diffing) is the only strategy that can detect composition effects efficiently.

## Strategy Rankings

### Tier 1: Highest Value (do these)

**Strategy 6: Empirical pixel diffing (full-scene comparison)**

This is the single most valuable strategy. The C++ binaries are available. The C++ golden test generator is compilable. Rendering the full TkTest panel in both C++ and Rust, capturing framebuffers, and diffing them pixel-by-pixel will reveal every visible divergence in a single operation. No agent judgment is required -- just pixel comparison. Composition effects, individually-untested fixes, and incorrectly-closed items all manifest as pixel differences.

The workflow is: run C++ emTkTestPanelStandalone at a canonical size, capture framebuffer. Run Rust equivalent at same size, capture framebuffer. Diff. Every non-zero pixel is a finding. Each finding can then be traced to a specific widget region and investigated.

This strategy is superior because: (a) it is holistic without requiring holistic reasoning from an agent, (b) it catches composition effects that per-widget testing misses, (c) it validates CLOSED items empirically rather than analytically, (d) its output is mechanically interpretable.

Limitation: it only catches rendering divergences, not behavioral ones (keyboard handling, state transitions, callbacks). It also requires both binaries to be runnable, which may require display/framebuffer infrastructure.

**Strategy 4 (refined): Coverage gap closure -- write golden tests for uncovered fixes**

The audit reports annotate coverage for each finding. At least 5 of the 31 fixes target functions explicitly noted as "not directly golden-tested" or "uncovered." The refined strategy is: for each fix, check if the modified function has golden test coverage. If not, add a golden test case to the C++ generator and the Rust test suite.

Concretely:
- Fix 20 (substance_round_rect coefficient): Add golden test for OBT_RECT border substance rect geometry
- Fix 21 (label_space pre-HowTo): Add golden test for border with how_to + label
- Fix 22 (best_label_tallness with icons): Add golden test for border with icon + aux panel
- Fix 23 (MarginFilled full clear): Add golden test for MarginFilled border type
- Fix 26 (label_layout desc-only): Add golden test for description-only label

Each test case is independently dispatchable to an agent with a narrow context requirement: read the C++ golden test generator (4251 lines, but only the relevant widget setup section), read the Rust golden test harness, write a new test case. The output is mechanically verifiable: the test either passes or fails.

This strategy converts "we hope the fix is correct" into "we know the fix is correct" for every covered function. It also creates regression protection that persists beyond this audit cycle.

**Strategy 5 (refined): Existence tests for CLOSED divergences**

10 items were CLOSED with justifications like "intentional Rust convenience" or "no current consumer does this." The refined mutation testing strategy is: for each CLOSED item, write a test that demonstrates the divergence exists and documents that it is accepted. This is not mutation testing in the traditional sense -- it is divergence documentation through test code.

For example, CC-02 (set_* methods don't fire signals) was CLOSED with the justification that remaining setters have no C++ signal equivalent. Write a test that calls set_text() on a Label and verifies that no callback fires. This documents the behavior and ensures that if someone later adds a callback, they must consciously update the test.

The key insight: CLOSED items are the riskiest category because they represent a trust boundary. "We decided this is fine" is the most common source of latent bugs. Tests that explicitly encode the divergence make the trust boundary visible and auditable.

### Tier 2: Moderate Value (do if budget permits)

**Strategy 1 (narrowed): Per-finding diff verification for HIGH/MEDIUM fixes only**

Adversarial re-audit of all 31 fix diffs is expensive and unreliable. But narrowing to the 7 HIGH and ~10 MEDIUM fixes is tractable. Each can be dispatched as an independent agent with a focused context: "Read fix 22 diff. Read C++ emBorder.cpp lines 460-464. Verify the Rust implementation matches the C++ algorithm. Report MATCH or MISMATCH with specific line references."

The key constraint: the agent must produce a structured output (MATCH/MISMATCH + evidence), not a prose review. Structured outputs are easier to validate and harder to game with "LGTM" shortcuts.

This catches the "wrong function called" failure mode (the TextField word_boundary vs word_index bug from fix 12 -- would a re-read catch it?). It also catches coefficient typos, off-by-one errors in loop bounds, and missed edge cases.

**Strategy 7: Commit-to-finding traceability audit**

This is a metadata integrity check. It verifies that every fix commit references a finding, every finding references a fix commit, and the status annotations in the per-widget files are consistent with the actual code changes. It is a cheap, mechanical task that an agent can perform reliably because it is pattern matching, not judgment.

The run-log documents 31 fixes with finding references. The per-widget files have status annotations. A traceability agent reads both, cross-references, and reports orphaned fixes (code changes not linked to findings) or orphaned findings (annotated FIXED but no corresponding code change). This catches the "stale metadata" failure mode directly.

### Tier 3: Low Value (skip or defer)

**Strategy 2: Cross-fix consistency check**

This strategy asks "did CC-01 fixes propagate to all 5 button-family widgets?" The answer is already known from the audit: CC-06 was fixed in all 5 widgets, CC-05 was fixed in Label and verified in Border, modifier guards were added to all 5 widgets. The cross-cutting concerns document tracks this explicitly.

The remaining value of this strategy is low because the cross-cutting concerns are already resolved and documented. An agent re-checking would mostly confirm what is already known. The risk of false negatives (agent says "all consistent" when it is not) is the same as the original audit's risk.

**Strategy 3: Rationale adversarial review**

This strategy asks an agent to challenge CLOSED and DEFERRED justifications. It is the worst strategy for this project at this stage. The 10 CLOSED items have detailed justifications with C++ line references. The 14 DEFERRED items have infrastructure dependency explanations. An adversarial agent will either: (a) agree with the justifications (producing no value), (b) disagree without evidence (producing noise), or (c) find a genuine error in a justification (possible but unlikely given the specificity of the existing rationales).

The fundamental problem is that challenging a justification requires the same holistic understanding that produced it. An agent challenging "CC-04 CLOSED because C++ emListBox.cpp has no VCT_MIN_EXT check" must read both the C++ and Rust code to verify this claim. That is just re-doing the original audit. If the original agent was wrong, a second agent with the same context and capabilities is equally likely to be wrong.

Strategy 5 (existence tests for CLOSED items) is strictly superior because it validates the same trust boundary empirically rather than analytically.

**Strategy 8: Forward-looking fragility analysis**

This strategy produces a maintenance guide identifying code that will break if modified without coordinated changes. It is useful documentation but produces no immediate quality improvement. It is a "nice to have" that an agent can produce at any time. It does not belong in a quality assurance pass.

## Proposed Additional Strategy: Behavioral Interaction Replay

The golden test infrastructure covers pixel comparison and basic interactions, but the coverage map reveals that 6 widgets have render-only coverage (no interaction tests) and 5 widgets have no golden tests at all. The empirical pixel diffing (Strategy 6) catches rendering divergences but not behavioral ones.

A behavioral replay strategy would: for each widget with interaction findings that were FIXED, script a sequence of inputs (mouse clicks, key presses) through both the C++ and Rust widget, and compare the resulting state transitions and rendered output at each step.

For example, Fix 29 added keyboard handling to Dialog (Enter -> Ok, Escape -> Cancel). The fix includes 4 new Rust tests. But are those tests correct? They test that the Rust code does what the Rust code does. A behavioral replay would send Enter to both C++ and Rust Dialog instances and compare the results, catching cases where the Rust tests encode the wrong expected behavior.

This is harder to automate than pixel diffing (it requires driving both UI toolkits programmatically) but would catch the entire class of "fix is self-consistent but wrong" bugs.

## Optimal Dispatch Plan

Given infinite agent budget, the priority order is:

1. **Empirical pixel diffing** (Strategy 6) -- full TkTest scene, one shot, catches everything visible
2. **Coverage gap closure** (Strategy 4 refined) -- 5-8 new golden tests for uncovered fixes, one agent per test
3. **CLOSED divergence tests** (Strategy 5 refined) -- 10 small tests encoding accepted divergences, one agent per 2-3 tests
4. **Per-finding diff verification** (Strategy 1 narrowed) -- 7-10 agents for HIGH/MEDIUM fixes only
5. **Traceability audit** (Strategy 7) -- 1 agent, mechanical cross-referencing

Given limited agent budget (3-5 agents), do strategies 1 and 2 only. Pixel diffing gives the broadest coverage for the least agent effort. Coverage gap closure gives the most durable value (regression tests that persist forever).

## Why This Ordering Is Correct

The fundamental insight is that this project has already done the hard analytical work. 170+ findings were produced by careful C++ vs Rust comparison. 31 fixes were applied. The remaining risk is not "we missed something in the analysis" but "the fixes themselves are subtly wrong" and "the closures are based on incorrect premises."

Empirical methods (pixel diffing, golden tests, behavioral replay) detect both categories of risk without requiring an agent to form a judgment. Analytical methods (adversarial review, rationale challenging, consistency checks) require an agent to form a judgment that is exactly as reliable as the judgment it is checking.

The project has the infrastructure to make empirical methods work: C++ binaries, golden test generator, comparison functions, divergence measurement tools. Using agents to write tests that exercise this infrastructure leverages agent strengths (code generation) while using mechanical processes for verification (test pass/fail). Using agents to read code and render opinions leverages agent weaknesses (subjective judgment under context pressure) while providing no mechanical verification of the opinion's correctness.

The optimal meta-strategy is therefore: **use agents as test authors, not as judges. Let the test infrastructure be the judge.**
