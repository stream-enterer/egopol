# Agent 2: Port Quality Specialist — Verification Strategies for a C++-to-Rust Port with a Living Oracle

## The Fundamental Asymmetry of Port Verification

Port verification is not code review. It is not testing. It is a distinct discipline with a unique structural advantage: **the specification is executable**. The C++ codebase is not documentation that might be ambiguous or incomplete. It is a running program that answers any question you can formulate as an input. This changes everything about what strategies are effective, and what strategies are wasted effort.

General code review asks "does this code do the right thing?" and the reviewer must infer "the right thing" from naming, comments, tests, and domain knowledge. Port verification asks "does this code do the same thing as that code?" — a question that can, in principle, be resolved mechanically for any given input. The challenge is that the space of inputs is vast, the two languages have different semantics, and the port may have intentional divergences that must be distinguished from bugs.

The 8 proposed strategies vary enormously in how well they exploit this asymmetry. Some are highly port-specific and could not exist without the oracle. Others are generic quality practices dressed up in port-verification language. I will evaluate each, propose what is missing, and argue for a specific prioritization.

## Strategy-by-Strategy Analysis

### Strategy 1: Adversarial Re-audit of Fix Diffs Against C++ Reference

**Port-specificity: VERY HIGH. This is the single most important strategy.**

The initial audit compared pre-fix Rust to C++. Fixes were applied. Nobody has systematically verified that the fixes match C++. This is not a redundant check — it catches a distinct and dangerous class of error.

Consider what happens when an LLM fix agent reads an audit finding that says "C++ uses `(1-(264-14)/264)*r` for face inset" and applies a fix. The agent might:
- Get the formula right but apply it at the wrong point in the control flow
- Match the formula but use Rust's default f64 division instead of the integer arithmetic the C++ uses
- Fix the named function but miss that the same formula appears in a second code path (the CC-01 problem)
- Introduce a new divergence in surrounding code while fixing the target divergence (the over-fixing failure mode from the context brief)

The fix diff is a small, bounded artifact. Reading the diff against the C++ reference is fast. The question is narrow: "does this diff make the Rust code match the corresponding C++ code?" This is a perfect task for a focused verification pass.

**Methodology**: For each fix commit, extract the diff. For each changed function, open the corresponding C++ function. Verify line-by-line that the fix achieves parity. Check that nothing else in the diff diverges from C++. This is mechanical, high-signal, and directly exploits the oracle.

**What it catches that nothing else does**: Fixes that are "close but not right" — where the fix agent understood the intent of the finding but implemented it with a subtle semantic difference. The Ctrl+Left/Right bug (calling `word_boundary` instead of `word_index`) is exactly this pattern: the correct function existed, the wrong one was wired. A fix agent could easily introduce the same class of error while fixing a different finding.

### Strategy 2: Cross-Fix Consistency Check (CC-01 Duplication)

**Port-specificity: HIGH. The duplication exists because of a port design decision.**

C++ has `emButton` as a base class with `emCheckButton`, `emCheckBox`, `emRadioButton`, `emRadioBox` inheriting from it. Rust has 5 standalone widgets with duplicated logic. This is a defensible Rust design decision (composition over inheritance, avoiding trait object complexity), but it creates a verification obligation: every fix to shared logic in one widget must be mirrored in the others.

The audit found 6 cross-cutting concerns. CC-06 (hit_test face inset) was fixed across all 5 widgets. But was each fix identical? The context brief documents that CC-05 (alignment defaults) was initially fixed in Label but not propagated. The same pattern could recur with any subsequent fix.

**Methodology**: For each button-family widget, extract the input handling, hit testing, paint pipeline, and keyboard handling functions. Diff them pairwise. Every difference that isn't explained by the widget's specific behavior (checkbox toggle vs radio exclusion) is a suspect divergence. Cross-reference against the C++ base class to determine which widget has the correct version.

**What it catches**: Inconsistencies introduced by the fix process itself. Fix session 1 applied modifier guards to all 5 widgets — but did it apply the exact same guard? Or did one widget get `!ctrl && !alt && !meta` while another got `!alt && !meta`? These are the errors that pass all existing tests (because the tests don't exercise modifier combinations) but create behavioral divergence.

### Strategy 3: Rationale Adversarial Review (Challenge CLOSED/DEFERRED/INTENTIONAL)

**Port-specificity: MEDIUM. The oracle enables definitive resolution.**

The context brief documents 9 items marked INTENTIONAL DIVERGENCE. Some are clearly defensible (f64 vs i64 for ScalarField — the Rust port genuinely needs float support). Others deserve scrutiny.

The most important question for each intentional divergence is not "is this defensible?" but "what user-visible behavior differs?" The audit reports for TextField's selection model and undo architecture both claim "behavioral parity achieved" with caveats. The selection model caveat is specific: "ModifySelection closest-endpoint logic for shift-click on existing selections" — shift-clicking within an existing selection to switch which endpoint extends. This is rare but it IS a behavioral difference.

**Methodology**: For each INTENTIONAL DIVERGENCE, enumerate the specific user-visible behavioral differences (not architectural differences). For each behavioral difference, determine: (a) can it be triggered through normal UI interaction? (b) does any consuming code depend on the C++ behavior? (c) would matching the C++ behavior require changing the architecture, or just adding a special case?

The ListBox arrow keys divergence is the most suspicious. It is marked INTENTIONAL but the description says "Arrow keys added — not in C++." This is a Rust-only feature addition, not a divergence from C++ behavior. The question is whether this addition interferes with any other input handling that C++ does differently. Does C++ use arrow keys for something else in this context? Does the Rust arrow key handler consume events that should propagate to a parent?

**What it catches**: Premature acceptance. The "silent skipping" failure mode from the context brief — fix agents marking items ACCEPTED without justification to clear the checklist. An adversarial review forces each acceptance to be justified against the oracle.

### Strategy 4: Coverage Gap Analysis

**Port-specificity: HIGH. The golden test infrastructure is the port's primary validation mechanism.**

The coverage map reveals significant gaps: RadioBox, Dialog, FileDialog, CoreConfigPanel have no golden tests at all. CheckButton has interaction tests but no render golden. Button has only one render state tested.

But the coverage gap analysis is not just "which widgets lack tests." The more important question is: **which of the fixed functions lack test coverage?** The audit reports annotate many findings with "Coverage: uncovered" — meaning the fix was applied to code that no golden test exercises. These are the highest-risk fixes because they have never been validated against the oracle empirically.

**Methodology**: For every finding marked FIXED, check the "Coverage" annotation. For each "uncovered" fix, determine: (a) can a golden test be written for this code path? (b) does the golden test generator already exercise this path but the Rust test just doesn't exist? (c) is the code path only exercisable through interaction (requiring the C++ binary for comparison)?

The coverage map shows that Label alignment (CC-05) was "masked by golden tests that use width-constrained or single-line text." This is a critical insight: the golden tests didn't just fail to test alignment — they actively masked the bug by using inputs that happened to produce the same output regardless of alignment. The coverage gap analysis should specifically look for other tests that pass by accident rather than by correctness.

### Strategy 5: Mutation Testing on CLOSED Items

**Port-specificity: MEDIUM. Standard mutation testing, but targeted at the port boundary.**

The idea of writing tests that demonstrate divergence is sound in principle, but the targeting is wrong. Mutation testing on CLOSED items is low-value because CLOSED items were investigated and determined to need no fix. The higher-value target is FIXED items with no coverage — write tests that would fail if the fix were reverted.

**Methodology**: For each FIXED finding with "Coverage: uncovered," write a test that specifically exercises the fixed behavior. Run it. If it passes, the fix is working. Revert the fix temporarily and verify the test fails. If the test still passes with the fix reverted, the test is not actually covering the fix.

This is standard regression testing methodology, but the port context makes it more valuable because the C++ golden test generator can produce the reference output for the new test cases. You are not inventing expected outputs — you are computing them from the oracle.

### Strategy 6: Empirical Pixel Diffing

**Port-specificity: VERY HIGH. This is the oracle's greatest power, underutilized.**

Both binaries exist. Both can render to pixel buffers. The golden test infrastructure already does framebuffer comparison. The strategy of running both binaries and diffing their output is the most direct exploitation of the oracle.

But the brief's framing — "run both binaries, capture framebuffers, diff" — undersells the approach. The real power is **exploratory differential testing**: render the same widget tree with systematically varied parameters (different sizes, zoom levels, text content, enabled/disabled states, font sizes) and diff the outputs. This finds bugs that targeted tests miss because it explores the input space without human bias about what's worth testing.

**What can only be found empirically**:
- Rounding differences that compound across nested layouts (a 1px error in a parent layout shifts all children)
- Font rendering differences at specific sizes (text measurement functions may diverge only at certain string lengths)
- Interaction between fixes (fix A is correct in isolation, fix B is correct in isolation, but together they produce a visible artifact)
- Performance-related rendering differences (if Rust takes a fast path that C++ doesn't, or vice versa)

**What can NOT be found empirically**: Logic bugs in code paths that don't affect rendering (signal firing, callback ordering, clipboard operations, focus management). These require code reading or behavioral testing.

**Methodology**: Extend the golden test generator to produce a grid of widget states: every widget type x every enabled/disabled state x 3 zoom levels x 3 content variants. Render in both C++ and Rust. Diff with a tolerance-aware pixel comparator. Any diff above the noise floor is a finding.

### Strategy 7: Commit-Message-to-Finding Traceability Audit

**Port-specificity: LOW. This is generic process auditing.**

Verifying that every fix commit references a finding and every finding has a fix commit is useful for process hygiene but low-value for finding bugs. The traceability audit answers "did we attempt to fix everything?" not "did we fix everything correctly?" Strategy 1 (fix diff re-audit) answers the harder question.

That said, this strategy has one port-specific use: detecting **orphaned fixes** — commits that change widget behavior without referencing a finding. These could be over-fixes, where a fix agent refactored surrounding code. They could also be independent improvements that accidentally diverge from C++.

**Methodology**: Quick pass. Map commit messages to finding IDs. Flag any commits that touch widget code without a finding reference. Flag any findings that lack a commit reference. Takes an hour, produces a checklist for Strategy 1.

### Strategy 8: Forward-Looking Fragility Analysis

**Port-specificity: MEDIUM. Useful but not urgent.**

Identifying duplicated code that needs coordinated maintenance (CC-01) and documenting it is good engineering practice. But it does not find bugs — it prevents future bugs. Given that the project has 30 pending LOWs and active development, a fragility analysis is premature. The code is still changing. Write the maintenance guide after the code stabilizes.

## What Is Missing from the 8 Strategies

### Missing Strategy A: Systematic Semantic Equivalence Verification

The audit found a signed/unsigned mismatch in alpha dimming. This is one instance of a pervasive class of port bug: **C++ implicit semantics that Rust makes explicit, where the explicit Rust choice is wrong**.

Specific categories:
- **Integer promotion**: C++ promotes `u8 * u8` to `int` (32-bit signed). Rust keeps it as `u8` and panics on overflow. The port must use explicit widening. Where does it widen to `u16` vs `i32` vs `u32`?
- **Signed right shift**: C++ right shift of signed integers is implementation-defined (arithmetic on all target platforms). Rust guarantees arithmetic shift for signed types. But if the C++ code uses unsigned types and the Rust port uses signed, or vice versa, the shift behavior differs for negative values.
- **Float-to-int conversion**: C++ truncates toward zero. Rust saturates (since 1.45). For negative coordinates, `(-0.7 as i32)` is 0 in Rust but -0 in C++ (both 0, but `(-1.7 as i32)` is -1 in both — actually this is consistent). The real danger is overflow: C++ is UB, Rust saturates.
- **Enum discriminant ranges**: C++ enums can hold any value in their underlying type. Rust enums must hold a valid discriminant. If C++ code uses enum values as bit flags or arithmetic operands, the Rust port must use a different representation.
- **Operator precedence**: Mostly identical, but C++ `&` and `|` have lower precedence than comparison operators, which has caused bugs in C++ that might be "fixed" in the Rust port (where the C++ bug was intentional behavior).

**Methodology**: For every function in the pixel arithmetic and geometry layers, list every integer operation and verify that the Rust type matches the C++ promoted type. This is tedious but mechanical. Focus on code that mixes signed and unsigned values, code that shifts, and code that converts between float and integer.

### Missing Strategy B: Boundary Value Differential Testing

The audit found that golden tests masked the alignment bug by using width-constrained text. This is a specific instance of a general problem: golden tests use "happy path" inputs that avoid boundary conditions.

**Methodology**: For each widget, identify the boundary conditions:
- Minimum and maximum sizes (VCT_MIN_EXT thresholds, panel sizes approaching zero or infinity)
- Empty and maximum-length content (empty text, text longer than the widget, zero-item and maximum-item lists)
- Boundary color values (fully transparent, fully opaque, max-channel values that might overflow in blend math)
- Edge coordinates (clicks exactly on the hit-test boundary, at the first and last pixel of a widget)

For each boundary condition, render in both C++ and Rust and diff. This is where Strategy 6 (empirical diffing) becomes most powerful: you are not randomly exploring the input space, you are systematically probing the boundaries where port bugs are most likely to hide.

### Missing Strategy C: Control Flow Graph Comparison

The audit compared code line-by-line, but some divergences are structural rather than local. A function might have the same formulas but different branching:

```cpp
// C++: early return
if (x < 0) return;
if (y < 0) return;
doWork(x, y);

// Rust: nested
if x >= 0 {
    if y >= 0 {
        do_work(x, y);
    }
}
```

These are equivalent. But:

```cpp
// C++: early return with side effect
if (x < 0) { count++; return; }

// Rust: forgot the side effect
if x < 0 { return; }
```

A control flow comparison would catch this. For each function pair, compare the number of return points, the conditions under which each return point is reached, and the side effects (mutations, signals, invalidations) on each path.

**Methodology**: This does not require tooling. For each function in the audit scope, draw the decision tree in C++ and Rust. Verify that every leaf (return/exit point) has the same side effects. Focus on functions with many branches — the paint functions and input handlers are the highest-risk.

## Prioritized Ranking

Given the current project state (83 items fixed, 30 LOWs pending, 9 intentional divergences, all tests passing), here is the priority ordering:

1. **Strategy 1 (Fix diff re-audit)** — Highest ROI. Bounded scope (31 fix commits). Directly exploits the oracle. Catches the most dangerous class of remaining bug (fixes that are close but wrong). Must be done before any other strategy because it validates the foundation.

2. **Strategy 6 (Empirical pixel diffing)** — Second highest ROI. Most direct use of the oracle. Finds bugs that code reading cannot find (rounding accumulation, interaction between fixes). The golden test generator infrastructure already exists. Extending it to cover more states is incremental effort with high payoff.

3. **Missing Strategy A (Semantic equivalence)** — Third priority. Catches a class of bug that both code review and testing frequently miss. The alpha dimming bug proves this class exists in the codebase. Systematic sweep prevents future discoveries of the same class.

4. **Strategy 2 (CC-01 consistency)** — Fourth priority. Bounded scope (5 widgets). Mechanical comparison. Catches inconsistencies introduced by the fix process.

5. **Strategy 4 (Coverage gap analysis)** — Fifth priority. Identifies where to focus future golden test development. The "Coverage: uncovered" annotations in audit reports are already a roadmap.

6. **Missing Strategy B (Boundary value differential)** — Sixth priority. The alignment bug proves that boundary inputs find bugs that typical inputs miss. Systematic boundary testing is high-value but high-effort.

7. **Strategy 3 (Rationale adversarial review)** — Seventh priority. Important for the 9 intentional divergences, but lower priority than strategies that find actual bugs. The intentional divergences are already documented with justifications.

8. **Strategy 5 (Mutation testing)** — Eighth priority. Valuable for regression safety but does not find new bugs.

9. **Strategy 7 (Traceability audit)** — Ninth priority. Quick to execute, low signal. Use as a preprocessing step for Strategy 1.

10. **Strategy 8 (Fragility analysis)** — Last priority. Prevents future bugs, doesn't find current ones. Do after code stabilizes.

## The Meta-Strategy: Oracle-First Verification

The unifying principle is: **use the oracle whenever you can, use code reading when you must**.

Code reading is expensive, error-prone, and biased by the reader's assumptions. The oracle (C++ code + C++ binary + golden test generator) is cheap, deterministic, and comprehensive within its exercised input space. Every strategy should be evaluated by how effectively it leverages the oracle.

- Fix diff re-audit: reads the oracle (C++ code) against a small Rust diff. High oracle utilization.
- Empirical pixel diffing: runs the oracle (C++ binary) and compares outputs. Maximum oracle utilization.
- Semantic equivalence: reads the oracle (C++ type system) against Rust types. Medium oracle utilization.
- CC-01 consistency: reads 5 Rust files and the oracle (C++ base class). Medium oracle utilization.
- Rationale review: reads the oracle to challenge justifications. Medium oracle utilization.
- Coverage gap analysis: uses the oracle's test generator to identify missing coverage. Medium oracle utilization.
- Traceability audit: does not use the oracle. Low port-specificity.
- Fragility analysis: does not use the oracle. Low port-specificity.

The strategies that most effectively exploit the unique structure of a C++-to-Rust port — two complete codebases implementing the same specification — are those that treat the C++ code as a queryable specification and the C++ binary as a reference implementation. Read the oracle when verifying formulas. Run the oracle when verifying outputs. Compare against the oracle at every opportunity.

When the oracle cannot be queried (behavioral contracts like "signal fires before callback," ordering invariants, error handling paths), fall back to structural comparison: control flow graphs, side-effect enumeration, and boundary condition analysis. But always prefer empirical verification over analytical verification when both are available.

The port is not done when all tests pass. The port is done when every function's output matches the oracle's output for every input the function can receive. The strategies above move systematically toward that goal.
