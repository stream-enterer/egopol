# Adversarial Critique of Prompt 7 (Behavioral Parity Testing)

## 1. The harness cannot express modifier-dependent input sequences

`PipelineTestHarness` has an `input_state: InputState` field that is `pub` but never mutated by any of the harness's own methods (`click`, `press_key`, `press_char`, `drag`). The production path in `ZuiApp::handle_input` calls `self.input_state.press(key)` / `.release(key)` on every event; the harness does not. When `dispatch()` calls `event.clone().with_modifiers(&self.input_state)`, the modifier fields are always `false` because no one pressed Shift or Ctrl into `input_state`.

This means BP-1 (Shift+click range, Ctrl+click toggle), BP-3 (Ctrl+A select all, Shift+Ctrl+A deselect all), BP-4 (Shift+Arrow extend selection, Ctrl+Arrow word skip), BP-5 (Ctrl+Backspace delete word, Shift+Ctrl+Delete delete to end), BP-6 (Shift+Ctrl+A deselect), and BP-7 (Ctrl+C/X/V clipboard) are all untestable through the existing harness API. A subagent will either (a) discover this and modify the harness, violating the "do NOT modify production code / existing tests" instruction, (b) bypass the harness and call `widget.input()` directly with hand-crafted events, making the "full pipeline dispatch" claim false, or (c) write tests that pass because the modifier flags are never set and the assertions encode the no-modifier behavior as correct.

**Compounding mechanism**: The more items use option (c), the more "branch coverage" the run log reports while systematically never testing any modifier-dependent path. The team sees "24/24 DONE" and believes modifier interactions are verified.

## 2. Assertion source is the subagent's reading of C++, not C++ execution

The prompt says subagents should "read the C++ file" and "write a Rust test that asserts the resulting widget state matches what the C++ code would produce." The expected values in assertions come from an LLM reading C++ source code and predicting what it would do. But the audit history already documents cases where LLM interpretation of C++ was wrong (context brief item 7: "Wrong function called: TextField Ctrl+Left/Right called `word_boundary` instead of `word_index`").

When a subagent reads `emListBox.cpp SelectByInput` lines 800-850 and writes `assert_eq!(selected_indices, vec![2, 3, 4, 5])` for a Shift+click range test, that assertion might be wrong. If the Rust code happens to have the same bug as the subagent's misreading, the test passes and the bug is invisible. If the Rust code is correct and the assertion is wrong, step 2 of "After each subagent returns" says "the test is correct; the Rust code diverges" and dispatches a FIX subagent to make the Rust code match the wrong assertion.

**Compounding mechanism**: Each incorrect assertion that triggers a "fix" makes the Rust code worse, and the fix makes the test pass, so there is no feedback signal that something went wrong. The run log records "divergence found + fix applied" as a success.

## 3. Double-click detection relies on real wall-clock timing that the harness does not control

TextField's multi-click detection uses `std::time::Instant::now()` with a 500ms window (`DOUBLE_CLICK_MS`). ListBox's keywalk uses `Instant::now()` with a 1000ms timeout (`KEYWALK_TIMEOUT_MS`). The harness dispatches events with zero delay between them — every press/release pair in `click()` happens within the same microsecond.

For BP-2 (keywalk: "type multiple chars within 1000ms -> accumulated prefix"), every test will accumulate because the events arrive in sub-microsecond intervals. The "type after 1000ms timeout -> clear accumulator" case is impossible to test without `std::thread::sleep(Duration::from_millis(1001))`, which the prompt never mentions and which would make the test suite take minutes.

For BP-6 (TextField double-click -> select word, triple-click -> select line), the harness's `click()` method dispatches press+release in rapid succession, so two consecutive `click()` calls will always register as a double-click (the time between them is ~0). There is no way to test "single click positions cursor" vs "double-click selects word" because the test cannot separate them in time. A subagent trying to test single-click behavior after having already tested double-click in the same test function will get triple-click behavior instead.

**Compounding mechanism**: Tests for timing-dependent behaviors either (a) always take the fast path, giving false coverage of the slow path, or (b) use `sleep()` which makes the suite fragile to CI load. Either way, the boundary conditions (exactly at the timeout threshold) are never tested.

## 4. The "do NOT modify production code" constraint prevents fixing the actual gap this layer should find

The prompt's purpose is to find behavioral divergences between C++ and Rust. When a divergence is found, the orchestrator dispatches a "FIX subagent for the Rust widget code." But the instruction to subagents says "Do NOT modify any production code. Write tests only." The fix subagent gets different instructions, but the sequencing creates a structural problem: the test-writing subagent writes assertions based on C++ reading, and if the Rust API does not expose the state needed for those assertions, the subagent either writes untestable assertions or invents an approximation.

For example, BP-2 requires asserting that keywalk accumulated a prefix string and matched against item names. But `ListBox` may not expose `keywalk_prefix()` or `keywalk_accumulator()` as a public accessor. The subagent cannot add one (no production code changes) and cannot assert on internal state. The test ends up asserting on `selected_index()` after typing, which tests that SOME selection happened but not that the prefix-matching algorithm works correctly. An off-by-one in prefix matching could select item "Bar" instead of item "Baz" and the test would pass as long as it asserted "some item was selected."

**Compounding mechanism**: Each widget where the Rust API does not expose intermediate state creates a test that asserts on the final outcome rather than the mechanism. These tests pass for all bugs that produce a plausible-looking final outcome, which is the majority of behavioral divergences.

## 5. The checklist assigns one subagent per widget, preventing cross-widget interaction testing

BP-9 (Button state machine), BP-10 (CheckButton toggle), and BP-13 (RadioButton exclusion) are assigned to separate subagents. But in C++, RadioButton exclusion depends on the button group mechanism in `emRadioButton.cpp`/`emRadioBox.cpp`, where clicking one radio button deselects siblings via the parent container. A subagent testing BP-13 in isolation will create a RadioButton, click it, and assert it is checked — which tests nothing about exclusion. To test exclusion, you need multiple RadioButtons in a RadioBox, which means the subagent for BP-13 needs to understand the RadioBox container, which is not in its instructions.

Similarly, BP-9 tests Button in isolation but some Button behaviors interact with the panel tree's focus system (Enter key only fires if the button is in the active path — tested by the harness's keyboard suppression, but only if the panel is set up correctly). If the subagent creates a Button panel that happens to be the active panel (because it is the only panel), the keyboard suppression path is never tested.

**Compounding mechanism**: As more widgets are added to the real application, the interactions between them become the primary source of bugs. Tests written against isolated widgets in a single-panel harness never encounter those interactions. The coverage number grows but the defect-finding power does not.

## 6. The orchestrator "does not read source code" but must make correctness judgments about divergences

When a subagent's test fails with an assertion error, step 2 says: "that's a FINDING. Log it. The test is correct (it encodes C++ behavior); the Rust code diverges." This presupposes that the subagent's reading of C++ was correct and its assertion accurately encodes C++ behavior. But the orchestrator "does not read source code" — it cannot verify this assumption. It will dispatch a fix subagent to make the Rust code match a potentially wrong assertion.

The orchestrator also cannot distinguish between three types of assertion failure: (a) genuine Rust divergence from C++ behavior, (b) subagent misread C++ and wrote a wrong assertion, (c) the Rust API returns values in a different format (e.g., 0-indexed vs 1-indexed) that the subagent did not account for. The prompt treats all three identically as case (a).

**Compounding mechanism**: The more items processed, the more "divergences found and fixed" appears in the run log, which looks like progress. But some fraction of those fixes are regressions introduced to match incorrect assertions. The run log structure does not distinguish genuine fixes from regression-causing fixes, so the team has no signal for when this happens.

## 7. BP-12 (ColorField expansion) is not an input-handling parity test

The checklist places BP-12 in "Domain 1: Widget Input Handling Parity" but describes it as: "RGB slider change -> color updates, HSV slider change -> color updates, text field hex input -> color updates, alpha slider when alpha enabled." These are not conditional branches in an `Input()` method — they are sub-widget composition behaviors. ColorField contains child ScalarField sliders and a child TextField. The "branches to cover" describe the data flow between child widgets and the parent ColorField state, not input dispatch logic.

A subagent receiving BP-12 with the instruction "read the C++ Input() method, identify every conditional branch" will find that `emColorField::Input()` delegates to child panels and has very little branching of its own. The subagent will either (a) correctly report that there are no interesting branches and write trivial tests, (b) misidentify the Cycle() update logic as Input() branches and write tests for the wrong method, or (c) attempt to test child widget interactions which requires the auto-expansion pipeline (child panels are only created when zoomed in far enough).

**Compounding mechanism**: The misclassification teaches the run log that "ColorField input parity" is DONE when actually the input parity question for ColorField is about whether child panel creation, layout, and input delegation match C++ — a question the Domain 1 template cannot ask.

## 8. The prompt has no mechanism to detect that the Rust API surface differs from C++

Several items reference C++ methods or concepts that may not exist in Rust at all. BP-14 (TextField drag-move with `DM_MOVE`) references a C++ drag mode that the Rust TextField may not implement. BP-7 (clipboard operations) references system clipboard integration that may not exist in the headless test environment. BP-2 (keywalk beep) references a feedback mechanism that Rust may implement differently or not at all.

When a subagent tries to write a test for a feature that does not exist in Rust, the test will fail to compile. Step 3 of "After each subagent returns" says "dispatch a fix subagent for the test code," but the right action is to recognize that a C++ feature is missing from the port — which is a finding of a different kind than a behavioral divergence. The prompt conflates "test won't compile because the API is different" with "test won't compile because the test code has a bug."

**Compounding mechanism**: Compilation failures from missing features get "fixed" by removing the test or weakening the assertion, which means the missing feature is never tracked as a gap. Over time, the set of "things we verified" and the set of "things that exist in Rust" converge, but the set of "things that exist in C++ but not Rust" becomes invisible.

## 9. Notice ordering assertions (Domain 3) conflate set membership with sequence

BP-20 through BP-24 use `NoticeBehavior` which accumulates notice flags into a shared `Rc<RefCell<NoticeFlags>>` via `insert()`. This is set-union accumulation — it records WHICH flags were ever set, but not the ORDER in which they were set, not HOW MANY TIMES each was set, and not WHICH PANEL received each flag.

The prompt says to "assert the accumulated NoticeFlags on each panel match what C++ would fire." For BP-21 (focus change notices), C++ fires `FOCUS_CHANGED` on the old focused panel, then on the new focused panel, then `VIEW_FOCUS_CHANGED` on ancestors. If Rust fires them in the wrong order (new before old, or ancestors before descendants), the accumulated flag set is identical. A bug in notification ordering — which can cause widgets to read stale state during their `notice()` handler — passes every test because the test infrastructure cannot observe ordering.

`RecordingBehavior` does record ordering (it pushes strings to a `Vec`), but the Domain 3 instructions say to use `NoticeBehavior` or `RecordingBehavior`, and then say to "assert accumulated NoticeFlags." A subagent will use `NoticeBehavior` because the assertion template matches it directly, never discovering that `RecordingBehavior` could reveal ordering bugs.

**Compounding mechanism**: As the panel tree grows more complex, notice ordering bugs compound: a handler that runs before its dependency has updated produces a stale-data cascade. The tests say the right flags were delivered, so the team believes notice propagation is correct.

## 10. The anti-satisficing rule targets the wrong failure mode

The prompt includes an "anti-satisficing rule": complete all 24 items, don't stop at 18 or 23. This targets the failure mode of an LLM declaring early victory. But the actual failure mode of this prompt is not "too few items completed" — it is "all 24 items completed with tests that cannot detect the bugs they are supposed to detect." The anti-satisficing rule pushes the orchestrator to produce MORE tests of the same structurally limited kind rather than BETTER tests of fewer items.

An orchestrator that completes 12 items with genuinely probing tests (modifier combinations, timing boundaries, cross-widget interactions) would be more valuable than one that completes 24 items where every test uses `click()` at the center of the viewport with no modifiers. The anti-satisficing rule makes the second outcome more likely because it penalizes spending time on any single item.

**Compounding mechanism**: The run log's completion metric (24/24) becomes the quality signal. Future prompts will reference "behavioral parity testing complete — 24 items verified" without examining what each item actually tested. The 24/24 number is load-bearing in quality arguments despite being structurally hollow.

## 11. The prompt conflates "conditional branch" with "behavioral path"

The instruction "identify every conditional branch" in a C++ `Input()` method is syntactic, not semantic. A C++ method with 8 `if` statements has 8 branches, but the number of behavioral paths through those branches is potentially 2^8 = 256 (minus infeasible combinations). The prompt asks subagents to "write a test for each branch," which means each branch is tested in exactly one context — typically the simplest one. The interaction effects between branches are untested.

For BP-1, `select_by_input` has branches for `shift`, `ctrl`, `trigger`, and `selection_mode`. A branch-level test plan covers: Shift path, Ctrl path, trigger path, each selection mode. But the actual behavioral paths include Shift+Ctrl in Multi mode (toggle-range), Shift in Single mode (which should have no effect beyond regular selection), Ctrl in ReadOnly mode (which should do nothing), double-click in Toggle mode (which should trigger). These cross-product cases are where the C++ and Rust behaviors are most likely to diverge because they test the interaction semantics, not just the individual branch predicates.

**Compounding mechanism**: As widgets gain more modifier combinations and mode flags, the gap between "branches covered" and "behavioral paths tested" grows combinatorially. A widget with 4 modes and 6 modifier combinations has 24 interaction points but the branch-coverage approach tests at most 10. The untested 14 interactions are where porting bugs live because they are the cases where the developer had to reason about multiple interacting design decisions simultaneously.

## 12. No validation that the PanelBehavior wrappers in tests match production wiring

Every pipeline test creates a custom `PanelBehavior` wrapper struct (e.g., `ButtonPanel`, `SharedListBoxPanel`, `SharedTextFieldPanel`). These wrappers decide which methods to delegate and which to ignore. `SharedListBoxPanel` forwards `notice()` for `FOCUS_CHANGED` and `ENABLE_CHANGED` but not for other notice types. `ButtonPanel` does not forward `notice()` at all. `SharedTextFieldPanel` forwards `FOCUS_CHANGED` but not `ENABLE_CHANGED`.

If the production wiring (wherever these widgets are actually used) forwards a different set of notices, the tests are testing an incorrect integration. When BP-9 tests Button's keyboard Enter handling, the `ButtonPanel` wrapper does not handle focus notices, so the button may never know it gained focus. In production, the hosting panel might forward focus notices which change the button's internal state in ways the test never exercises.

The prompt tells subagents to use `PipelineTestHarness` and create wrappers but provides no specification for what the wrapper must forward. Each subagent invents its own wrapper, creating N slightly different integration surfaces that diverge from each other and from production.

**Compounding mechanism**: As more wrapper variants are created across 14 widget tests, the implicit assumption "the wrapper correctly simulates production wiring" becomes harder to verify. Any bug in wrapper construction is invisible because the test only exercises the widget through its own wrapper. Fixing a notice-forwarding bug in production has no effect on the tests, and adding notice-forwarding to a wrapper has no effect on production — the two drift independently.
