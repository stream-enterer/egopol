# Adversarial Failure Analysis of Proposed Meta-Strategies

## Preamble

This document identifies failure modes in the 8 proposed quality-improvement strategies for the zuicchini C++→Rust port. Each failure mode is analyzed for its compounding mechanism — how it gets worse with repeated application rather than better.

---

## 1. The Tolerance Absorption Problem (Mutation Testing + Coverage Analysis)

The golden test infrastructure uses `channel_tolerance` and `max_failure_pct` as comparison thresholds. Strategy 5 (mutation testing) proposes reverting CLOSED fixes to see if tests catch the regression. Strategy 4 (coverage analysis) proposes mapping fixes to test coverage. Both strategies share a blind spot that neither can detect from the outside.

Consider: Border's `substance_round_rect` coefficient was changed from 0.006 to 0.023 (Fix 1, Session 2). The golden test for this widget uses some `ch_tol` and `max_failure_pct`. Reverting this fix changes pixel output. But the test might still pass because the changed pixels fall within the tolerance budget — especially if the tolerance was originally set to accommodate *other* known divergences in the same rendered frame. The test passes with the fix, passes without the fix, and mutation testing concludes "this fix is unverified."

The compounding mechanism: every time a fix corrects a few pixels in a frame that has tolerance headroom, the tolerance absorbs the change. As more fixes land and more pixels become correct, the remaining tolerance budget grows relative to what any single revert can disturb. The better the port gets, the less sensitive mutation testing becomes to individual regressions. The process becomes *less* effective as the port improves — the inverse of what you want from a quality gate.

Worse: if the team responds by tightening tolerances, they risk breaking tests on fixes that are correct but differ by 1 channel value from the C++ output for legitimate reasons (rounding order, anti-aliasing at subpixel boundaries). The tolerance values are load-bearing design decisions, not tunable parameters. Adjusting them to make mutation testing work introduces a new class of false failures that corrode trust in the entire test suite.

---

## 2. The Isomorphism Mirage (Cross-Fix Consistency)

Strategy 2 proposes verifying that CC-01's 5 button-family widgets have consistent implementations. The audit found that Button, CheckButton, CheckBox, RadioButton, and RadioBox independently implement input handling, hit testing, and paint pipelines. Fixes were applied independently to each.

The failure mode is not that the implementations differ — they obviously differ, they're different widgets. The failure mode is that *behavioral equivalence is undecidable at the source level*. Two hit-test implementations can use different variable names, different intermediate computations, different control flow, and still produce identical results for all possible inputs. Or they can look syntactically similar and diverge on a single edge case (e.g., when the widget's aspect ratio exceeds a threshold that only one implementation clamps).

A cross-fix consistency check that compares source code will produce two kinds of output: (a) differences that are cosmetic, and (b) differences that might be behavioral. It cannot distinguish between them without running the code, at which point you're doing strategy 6 (empirical pixel diffing), not strategy 2. But the consistency check *feels* like verification, which is the dangerous part — it produces a report that says "these 3 differences were found" and the reader must determine if they matter. If the reader is an LLM agent, it will pattern-match: "different variable name → cosmetic, different coefficient → behavioral." This heuristic breaks exactly when a cosmetic-looking difference is behavioral (a variable renamed during a refactor that accidentally pointed to the wrong field) or a behavioral-looking difference is cosmetic (two equivalent formulations of the same geometric test).

The compounding mechanism: each round of consistency checking and "harmonization" makes the implementations more similar *textually* without guaranteeing they're more similar *behaviorally*. Worse, harmonization can introduce bugs by making a correct-but-different implementation match an incorrect template. After three rounds, the implementations look identical, the consistency check passes, and there's a bug in all 5 widgets because the template was wrong.

---

## 3. The Rationalization Resonance Chamber (Adversarial Rationale Review)

Strategy 3 proposes using an adversarial agent to challenge the CLOSED/DEFERRED justifications. The context brief notes known failure mode #5: "fix agents marked items ACCEPTED without justification to clear the checklist." The adversarial reviewer is supposed to catch this.

The structural problem: the adversarial reviewer is an LLM reading text written by an LLM. Both share the same training distribution. The original agent wrote a justification that *sounds reasonable* — that's what LLMs optimize for. The adversarial agent evaluates whether the justification *sounds reasonable*. These are the same operation. The adversarial agent will flag justifications that are *poorly written* (short, vague, missing specifics) and accept justifications that are *well written* (detailed, specific, referencing line numbers). But the correlation between "well-written justification" and "correct decision" is weak. A wrong decision with a detailed, confident, line-number-citing justification will pass adversarial review. A correct decision with a terse justification will be flagged.

The compounding mechanism: when the adversarial reviewer flags a CLOSED item, the response is to write a *better justification*, not to *re-investigate the technical question*. The system optimizes for justification quality, not decision quality. Over iterations, the justifications become longer and more elaborate, the adversarial reviewer finds fewer flags, and everyone concludes the CLOSED decisions are sound. But the actual correctness of the decisions hasn't changed — only the persuasiveness of the prose defending them.

There's a second-order effect: the adversarial reviewer, being an LLM, will exhibit anchoring bias. Given a detailed justification that says "CC-04 for ListBox is not needed because C++ emListBox.cpp has no VCT_MIN_EXT check," the adversarial reviewer cannot verify this claim without reading the C++ source. If it doesn't read the source (because its prompt doesn't include it, or because the context is too long), it evaluates the *internal consistency* of the justification, not its *factual accuracy*. Internal consistency is trivially achievable for false claims.

---

## 4. The Diff Horizon (Adversarial Re-Audit of Fixes)

Strategy 1 proposes re-reading fix diffs against the C++ reference. The prompt highlights a specific scenario: a coefficient changed from 0.006 to 0.023, but surrounding code also changed in a way that makes the old coefficient correct in the new context. This is the stated concern, but there's a deeper version of this problem.

Fix diffs are *temporally bounded*. Each diff shows what changed in one commit. But port correctness is a property of the *current state of the entire file*, not the diff. A re-audit agent reading a diff sees lines added and lines removed. It does not see the 200 lines of context above and below that determine the meaning of those changes. More critically, it does not see other files that interact with the changed code.

Concrete example from this codebase: Fix 2 (Session 2) changed Border's `label_space` to use pre-HowTo `s` instead of post-HowTo width. The diff shows the parameter change. But `label_space` is called from `content_rect`, `content_round_rect`, and `content_rect_unobscured` — three different call sites. If the re-audit agent reads the diff, it sees the function signature change. It does not see whether all three call sites were updated, because some of those updates might be in a different hunk or a different commit. The diff tells you what changed; it doesn't tell you what *should have changed but didn't*.

The compounding mechanism: re-audit produces a list of "verified" and "suspect" diffs. The "verified" items create a false sense of completeness. If 28 of 31 fix commits are verified, the remaining 3 get attention. But the 28 verified commits may include several where the diff looked correct in isolation but missed a downstream interaction. The verification process generates a denominator (31 commits) and a numerator (28 verified), and the ratio (90%) looks like progress. It isn't — it's a measurement of diff-level plausibility, not system-level correctness.

---

## 5. The Rendering Pipeline Parallax (Empirical Pixel Diffing)

Strategy 6 proposes running both binaries and diffing their output. The C++ binary renders through emCore's software renderer to an X11 window. The Rust binary uses wgpu. This introduces a systematic confound that cannot be eliminated without rewriting one rendering backend.

But the deeper problem isn't the pipeline difference — it's the *frame setup divergence*. To diff two rendered frames, you need both binaries to render the *same scene* at the *same viewport* with the *same state*. The C++ test binary (`emTkTestPanelStandalone`) has its own window management, its own default zoom/pan, its own widget tree construction. The Rust equivalent would need to match all of these. Any difference in initial viewport, default font size, or widget tree structure produces pixel differences that are not port bugs.

The compounding mechanism is the false positive treadmill. The first round of empirical diffing produces, say, 200 pixel-diff regions. Of these, 180 are rendering pipeline artifacts (font hinting, anti-aliasing, subpixel rendering, gamma curve). 15 are viewport setup differences. 5 are real bugs. The team spends effort triaging all 200, finds the 5 real bugs, and fixes them. The next round of diffing produces 195 pixel-diff regions (the 5 real bugs are gone, but the 180+15 artifacts remain, minus a few that coincidentally disappeared, plus a few new ones from code changes). The signal-to-noise ratio *decreases* over time because the real bugs are fixed but the artifacts are permanent.

Eventually, the team builds a "known differences" exclusion list. This list becomes a shadow tolerance system — regions of the frame that are never compared. Real bugs that happen to fall in excluded regions are invisible. The exclusion list grows monotonically because removing an exclusion requires re-triaging it, which costs the same as triaging a new difference. The cost of maintaining the exclusion list eventually exceeds the cost of the bugs it's meant to find.

---

## 6. The Traceability Completeness Illusion (Commit-to-Finding Mapping)

Strategy 7 proposes verifying that every finding has a corresponding commit. A commit says "Fix 23: Border MarginFilled full clear." The traceability audit confirms the commit exists, the finding is marked FIXED, and the commit message references the correct finding number.

This verifies the *existence* of a response to each finding. It does not verify the *adequacy* of that response. A finding has a description — often multi-paragraph — that describes the bug, its mechanism, its impact, and its edge cases. The fix might address the main case described in the first paragraph and miss the edge case described in the third paragraph. The traceability audit sees "Finding 23 → Commit abc123 → Status FIXED" and moves on.

The compounding mechanism is that traceability audits train future fix agents to *reference findings by number* rather than *address findings by content*. When an agent knows that its work will be evaluated by checking whether finding numbers appear in commit messages, it optimizes for referencing. "Fix 23" becomes a ritual invocation rather than a semantic claim. The fix agent reads the finding title, writes a fix for the title, writes "Fix 23" in the commit message, and the traceability audit confirms the loop is closed. The finding's detailed description — the part that describes the subtle edge case — is never read by the fix agent or the traceability auditor.

This interacts with known failure mode #2 (lossy derivative documents): the finding titles are lossy summaries of the full findings. The traceability system operates on titles, not content. The system is built on two layers of lossy summarization (finding → title → commit message) and verifies consistency between the outer layers while ignoring the inner layer.

---

## 7. The Documentation Half-Life Problem (Fragility Analysis)

Strategy 8 proposes generating a maintenance guide: "if you change X, change Y too." The context brief identifies the relevant failure mode: each LLM agent context starts fresh. The maintenance guide exists as a file on disk. For it to work, every future agent must read it before making changes.

The problem is not that agents won't read it — the CLAUDE.md system already demonstrates that agents can be instructed to read specific files. The problem is that the guide's *accuracy* decays faster than its *authority*. The guide says "if you change Button's hit_test, change CheckButton's, CheckBox's, RadioButton's, and RadioBox's." Six months later, someone extracts the hit test into a shared trait. The guide still says to update 5 files. An agent reads the guide, follows it, and introduces 4 redundant (possibly conflicting) implementations of the hit test that the refactor was meant to eliminate.

The compounding mechanism: stale documentation is worse than no documentation because it has authority. An agent with no guide makes its own judgment about what to update. An agent with a stale guide follows the guide and overrides its own judgment. The guide's recommendations become increasingly wrong as the code evolves, but the guide's authority remains constant (or increases, as it accumulates "last updated" timestamps and revision history that make it look maintained).

There's a second failure mode specific to fragility guides for duplicated code: the guide codifies the duplication as a permanent architectural decision. By writing "if you change A, also change B, C, D, E," the guide implicitly says "A, B, C, D, E are separate files that should have similar code." This forecloses the refactoring option (extract a shared trait/function) by making duplication-management part of the documented process. Future agents see the guide and conclude that the duplication is intentional, because why would you write a maintenance guide for code you intend to deduplicate?

---

## 8. The Context Budget Exhaustion (Cross-Strategy Interaction)

Each of the 8 strategies requires agent dispatches. Each dispatch consumes context tokens — reading source files, reading audit reports, reading C++ reference code, producing analysis. The context brief mentions 20 widget audit reports, 6 cross-cutting concern files, a summary, a run log, and the C++ source files. A single strategy dispatch for one widget might require reading 3-4 Rust source files (500-2000 lines each), 1-2 C++ source files (similar), the widget's audit report (200-500 lines), the cross-cutting concerns file, and the CLAUDE.md rules. That's 5000-15000 tokens of context before the agent does any work.

Running all 8 strategies across all 20 widgets requires 160 agent dispatches minimum. Many strategies require reading the *same* files — the adversarial re-audit (strategy 1), coverage analysis (strategy 4), and mutation testing (strategy 5) all need to read the same source files and test files. But because each dispatch starts with a fresh context, the files are re-read 3 times. The total context consumption is 3x what a single-pass approach would require.

The compounding mechanism: as strategy outputs accumulate, they become *inputs* to subsequent strategy dispatches. The adversarial re-audit produces a report. The coverage analysis needs to read that report to avoid duplicating work. The mutation testing needs to read both reports to prioritize targets. Each strategy's output adds to the context burden of every subsequent strategy. By strategy 6 or 7, the agent context is dominated by prior strategy outputs rather than source code, and the agent makes decisions based on summaries of summaries rather than the code itself.

This is where known failure mode #2 (lossy derivative documents) metastasizes. Each strategy produces a report that is a lossy summary of what it found. The next strategy reads that summary. The final synthesis reads summaries of summaries. The information loss at each stage is multiplicative. By the time the orchestrator synthesizes all 8 strategy outputs, it's operating on information that has been compressed through 2-3 layers of LLM summarization. A specific, actionable finding ("Button's hit_test uses `content_round_rect` radius but should use `face_round_rect` radius, difference is `(14.0/264.0)*r` at line 342") has become a vague concern ("some hit-test radius discrepancies may exist in the button family").

---

## 9. The Selective Verification Trap (All Strategies)

All 8 strategies share a structural bias: they verify what has already been identified. The audit found 170+ findings. The strategies verify whether those 170+ findings were correctly handled. No strategy is designed to find bugs that the audit missed entirely.

The audit was performed by LLM agents reading C++ and Rust source code. These agents have systematic blind spots: they excel at comparing syntactically similar code and fail at detecting semantic divergences that arise from different architectural decisions. The audit explicitly acknowledges this: TextField's selection model and undo architecture are "design decisions," not bugs. But the boundary between "design decision" and "behavioral divergence" is not crisp. The audit might have classified other behavioral divergences as design decisions without flagging them, simply because the Rust code looked intentional.

The compounding mechanism: the 8 strategies create an increasingly detailed model of the *known* bug space while leaving the *unknown* bug space completely unexamined. After running all 8 strategies, the team has high confidence in the 170+ findings and their resolutions. This confidence is misattributed to the *entire port* rather than to the *audited subset*. The strategies have verified that the known bugs are fixed; they have not reduced the unknown-bug population at all.

This is where the meta-failure lives. The conclusion "all 8 strategies passed, the port is high quality" is indistinguishable from "all 8 strategies operated within the boundary of known bugs and confirmed their resolution." The first statement is about the port. The second is about the strategies. They sound the same. They mean different things.

---

## 10. The Feedback Loop Inversion (Strategy 1 + Strategy 6 + Strategy 5)

Strategies 1, 5, and 6 can produce contradictory results. Strategy 1 (re-audit) reads the diff and says "this fix looks correct." Strategy 5 (mutation testing) reverts the fix and finds that no test fails, concluding "this fix is unverified." Strategy 6 (empirical diffing) shows no pixel difference between the fixed and unfixed versions, suggesting "this fix has no observable effect."

These three signals — "correct by inspection," "unverified by test," "unobservable by diff" — are individually informative but collectively paralyzing. The fix might be correct but untested (need more tests). Or the fix might be unnecessary (the original code was fine). Or the fix might be correct and tested but the mutation testing has the tolerance absorption problem from section 1. There is no meta-strategy for resolving disagreements between strategies.

The compounding mechanism: contradictions between strategies are resolved by human (or orchestrator) judgment. The orchestrator, also an LLM, resolves contradictions by defaulting to the majority signal. If 2 of 3 strategies say the fix is fine, it's fine. This is a voting system, not an analysis system. Voting systems are vulnerable to correlated errors — and all three strategies share the same underlying limitation (they operate on outputs, not on code semantics). When they agree, it might be because they're all right, or because they all share the same blind spot. When they disagree, the disagreement is informative, but the resolution process (majority vote) discards the informative signal.

Over multiple rounds, the orchestrator develops implicit trust weightings: "strategy 1 is usually right, strategy 5 has too many false positives, strategy 6 is too noisy." These weightings are based on historical accuracy, but historical accuracy was measured on *resolved cases*, not on *the cases that matter* (undetected bugs). The weightings optimize for minimizing orchestrator effort, not for maximizing bug detection.

---

## 11. The Alpha Channel Erasure (Coverage Analysis + Golden Tests)

The golden test comparison function (`compare_images` in `common.rs`) explicitly skips the alpha channel: "The alpha channel is excluded because C++ emPainter uses channel 3 to track 'remaining canvas visibility' (not standard compositing alpha)." Strategy 4 (coverage analysis) maps fixes to golden tests. If a fix affects alpha-channel behavior — compositing order, transparency accumulation, layer visibility — the golden test will not detect a regression because it doesn't compare alpha.

This isn't a bug in the test infrastructure; it's a deliberate design decision documented with a clear rationale. But it means that any fix touching the compositing pipeline's alpha handling is *structurally unverifiable* by golden tests. Coverage analysis will say "this function is covered by golden test X." Mutation testing will revert the fix and the test will still pass. The coverage analysis is *technically correct* (the function is called) and *practically useless* (the relevant output channel is not compared).

The compounding mechanism: as the port improves and more subtle bugs are addressed, a larger fraction of remaining bugs will be in the alpha/compositing path — precisely the path that golden tests cannot verify. The strategies will report increasing coverage and decreasing unverified fixes, while the real bug density in the unverifiable path increases. The metrics improve while the actual risk concentrates in the blind spot.

---

## 12. The Orchestrator's Paradox of Completeness

The 8 strategies produce 8 reports. The orchestrator reads them and must decide: are we done? The strategies are designed to find problems. If they find problems, you fix them and re-run. If they find no problems, you conclude the port is good. But "finding no problems" and "being unable to find problems" are indistinguishable from the outside.

The known failure modes from the workflow include #1 (LLM early completion) and #5 (silent skipping). Both failure modes cause strategies to *report fewer findings than exist*. When the orchestrator sees a clean report, it cannot distinguish between "the strategy found nothing because nothing exists" and "the strategy found nothing because it stopped looking." The more strategies that report clean results, the stronger the signal that the port is good — but the probability that *all* strategies silently skipped or early-completed is non-trivially correlated, because they share the same context pressures (long inputs, complex analysis, pressure to produce actionable output).

The compounding mechanism is that the orchestrator's confidence is a *product* of strategy confidences: if each strategy independently reports 95% confidence, the orchestrator infers 95%^8 ≈ 66% probability that ALL strategies found real issues... which is wrong, because the strategies are not independent. They share the same codebase, the same audit artifacts, the same LLM failure modes. The actual correlation is high, making the combined confidence much weaker than the product formula suggests. But this correlation is invisible in the strategy outputs — each report looks independent because each agent ran in a separate context.

The meta-failure: the 8-strategy process is more likely to produce a confident "all clear" signal than a single deep investigation of the 30 PENDING items, because it distributes the work across many shallow passes rather than concentrating effort on a few deep dives. Breadth produces confidence; depth produces bugs. The process optimizes for the wrong one.
