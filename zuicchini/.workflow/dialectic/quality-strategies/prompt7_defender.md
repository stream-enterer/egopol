# Prompt 7 Defense: Behavioral Parity Tests

## 1. STRONG: The three-domain decomposition correctly separates independently testable concerns

The prompt divides work into widget input handling (Domain 1), focus/activation (Domain 2), and notice/signal propagation (Domain 3). This decomposition is strong because it mirrors the actual separation in the codebase architecture. Widget `input()` methods (`src/widget/*.rs`) never call `PanelTree::queue_notice()` directly -- notices are queued by the tree infrastructure in `src/panel/tree.rs` and `src/panel/view.rs`. Focus management lives in `View::set_active_panel()`. These three systems have genuinely different failure modes, and testing them separately means a failure in one domain's tests gives a useful diagnostic signal rather than a cascade of unrelated failures.

**Mechanism**: Decomposition along actual architectural seams. A test for "ListBox Shift+click range select" depends on `ListBox::input()` and nothing else. A test for "Tab navigates to next focusable panel" depends on `View` and `PanelTree` but no widget code. When tests are organized along these seams, a red test points to a single subsystem.

## 2. WEAKNESS: The prompt assumes Tab/focus traversal infrastructure exists -- it does not

BP-15 through BP-18 instruct the subagent to test Tab forward, Tab backward, focus on disabled panels, and arrow key navigation. A search of `src/panel/` for `focus_next`, `focus_prev`, `tab_forward`, or `Tab` produces zero results in the panel tree or view. The `View::set_active_panel()` method exists, but there is no `FocusNext`/`FocusPrev` equivalent to C++ `emPanel::FocusNext`. The `KeyboardZoomScrollVIF` consumes keyboard zoom/scroll events but does not implement Tab focus cycling.

This means BP-15, BP-16, and BP-18 will not produce passing tests -- they will produce compilation errors or tests that expose missing infrastructure, which the prompt's error handling classifies as "dispatch a FIX subagent for the Rust widget code" (step 2 of the post-subagent protocol). A subagent tasked with "fixing" a missing Tab traversal system will attempt to implement a significant architectural feature, violating the prompt's own constraint that this layer writes tests, not production code.

**Amendment**: Add a "prerequisite check" phase before Domain 2. The orchestrator (or a lightweight probe subagent) should verify that `PanelTree` or `View` exposes Tab focus cycling. If the infrastructure does not exist, Domain 2 items should be logged as BLOCKED with a rationale, not forced through a fix-subagent loop that will produce a rushed implementation of focus traversal. Alternatively, reclassify BP-15/16/18 as "infrastructure gap findings" whose deliverable is a report, not a test.

## 3. STRONG: The "test is correct, Rust code diverges" presumption (step 2) is MOSTLY right and works as an oracle

The post-subagent protocol says: "If tests fail with assertion errors: that's a FINDING. The test is correct (it encodes C++ behavior); the Rust code diverges. Dispatch a FIX subagent for the Rust widget code, not the test."

This works because the subagent reads the C++ source and writes tests that encode C++ branching logic. When such a test fails against the Rust code, either: (a) the Rust code has a genuine behavioral divergence, or (b) the test misread the C++ code. Case (a) is the common case and the presumption correctly accelerates the workflow by defaulting to "fix the code." Case (b) is caught when the fix subagent reads the Rust code, finds the behavior is intentionally different, and reports back.

**Where the presumption breaks**: The 9 documented INTENTIONAL DIVERGENCE items (ScalarField f64-vs-i64, TextField anchor-based selection, ListBox arrow keys, etc.) will trigger false positives. A test encoding C++ `emListBox` behavior that expects no ArrowUp/ArrowDown handling will "pass" (the C++ branches don't exist), but a test encoding C++ `emTextField::Input` selection behavior using `SelectionStartIndex`/`SelectionEndIndex` will fail against the Rust anchor-based model -- and the fix subagent should NOT "fix" the Rust code to use the C++ model.

**Amendment**: Add a pre-filter step: before dispatching the fix subagent, the orchestrator checks whether the failing test's widget appears in the INTENTIONAL DIVERGENCE list (greppable from `.workflow/widget-comparison/results/*.md`). If it does, the orchestrator logs the failure as "divergence validated by test -- confirm intentionality" and proceeds, rather than dispatching a fix.

## 4. WEAKNESS: BP-2 (keywalk) specifies a "beep" assertion that is untestable in the current harness

BP-2 includes "no match -> beep" as a branch to cover. The Rust code calls `crate::window::system_beep()` for no-match, which is an FFI call with no observable side effect in the test harness. The existing unit tests for keywalk (`list_box.rs` lines 1971-2025) test prefix match, substring search, and fuzzy match by calling `keywalk_search()` directly -- they do not test the beep path through the pipeline.

A subagent writing a pipeline test for "no match -> beep" will either: (a) assert that `keywalk_chars` was cleared (the observable side effect of a failed search), which is testing an internal implementation detail, or (b) attempt to mock `system_beep()`, which requires modifying production code. Option (a) is fragile but acceptable if properly scoped. Option (b) violates the "do NOT modify any production code" constraint.

**Amendment**: Rephrase BP-2's "no match -> beep" branch as "no match -> search string cleared, no item selected." This tests the observable behavioral outcome without requiring a beep oracle.

## 5. STRONG: The PipelineTestHarness is the right tool for this job

The prompt mandates all input go through `PipelineTestHarness`. This is a genuinely strong constraint because `PipelineTestHarness::dispatch()` reproduces the full production path: VIF chain filtering, hit test on mouse press, `view_to_panel_x/y` coordinate transforms, keyboard suppression for non-active-path panels, and post-order DFS broadcast with consumption. The existing `TestHarness` in `tests/support/mod.rs` omits the coordinate transform (it passes view-space coordinates directly to behaviors), which means tests using `TestHarness` can pass for the wrong reason -- the widget receives coordinates it would never get in production.

The `PipelineTestHarness` catches real bugs that `TestHarness` misses. For example, a widget that checks `event.mouse_x < 0.5` (half-width in panel-local space) will behave correctly in `TestHarness` when given view-space coordinates that happen to be < 0.5, but `PipelineTestHarness` will correctly transform view-space coordinates to panel-local coordinates, exposing any coordinate-system confusion.

## 6. WEAKNESS: BP-12 (ColorField expansion) tests an internal Cycle(), not input handling

BP-12 is titled "ColorField expansion" and asks for "RGB slider change -> color updates, HSV slider change -> color updates, text field hex input -> color updates." But ColorField's `input()` method (at `src/widget/color_field.rs:424`) does not handle slider or text field input directly. ColorField creates sub-widgets (ScalarField instances for RGB/HSV sliders, a TextField for hex input) during expansion. Those sub-widgets handle their own input. The color update logic lives in callback closures wired during `layout_children`.

A subagent reading `emColorField.cpp` Cycle() and trying to write pipeline tests will find that the Rust ColorField cannot be tested through `PipelineTestHarness::click()` at the top level -- the sub-widgets are internal to the panel tree and require auto-expansion at a specific zoom level to be created and positioned. The existing `tests/pipeline/colorfield.rs` likely handles this already with `expand_to()`.

**Amendment**: Rephrase BP-12 to acknowledge the expansion requirement: "Expand the ColorField to its sub-widget zoom level. Verify that clicking on the R slider, then the G slider, then entering hex text, produces the expected color values. This tests the callback wiring, not the input handling of ColorField::input() itself." Or, if the sub-widget interactions are already covered by BP-8 (ScalarField), BP-4/BP-6 (TextField), and the existing colorfield pipeline test, downgrade BP-12 to a "wiring verification" test that checks callback connectivity rather than re-testing sub-widget input handling through a different parent.

## 7. STRONG: The anti-satisficing rule is calibrated to a known failure mode

The context brief documents "LLM early completion" as failure mode #1: "agents fix 4 of 12 items and declare the session productive." The prompt's anti-satisficing rule ("If you catch yourself composing 'The core widget interactions are tested'... that is early completion. There are 24 items.") directly targets this with a concrete tripwire. It works because it transforms a subjective judgment ("am I done?") into an objective count ("have I completed 24 items?").

**Why this is better than most anti-satisficing rules**: The prompt gives the agent a LIST. The list is finite and countable. The agent cannot rationalize "the important ones are done" because the prompt never assigns priority -- all 24 items are PENDING. This eliminates the failure mode where an agent prioritizes easy items, completes them, and declares victory because it did the "high-impact" work.

## 8. WEAKNESS: The prompt conflates "branch coverage" with "behavioral parity" and will produce redundant tests

BP-1 says: "Single click, Shift+click range, Ctrl+click toggle, double-click trigger. Test each in SINGLE, MULTI, TOGGLE, READ_ONLY modes." This is 4 input patterns x 4 modes = 16 test functions. But many of these combinations have identical behavior: in READ_ONLY mode, all four input patterns do nothing. In SINGLE mode, Shift+click and Ctrl+click both behave like single click. Testing "Shift+click in SINGLE mode" and "Ctrl+click in SINGLE mode" produces two tests that assert the same thing -- the modifier is ignored.

This is not harmful (redundant tests pass), but it wastes subagent context window on tests that add no diagnostic value. When a future refactor changes ListBox selection, all 16 tests will need updating, even though only 6-8 of them test genuinely different code paths.

**Amendment**: Reframe BP-1's instruction as "Test each DISTINCT behavior: (1) single click selects in all modes except ReadOnly, (2) Shift+click extends range only in Multi mode, (3) Ctrl+click toggles only in Multi and Toggle modes, (4) double-click triggers, (5) ReadOnly rejects all selection input. For modes where the behavior is identical to single-click, a single test covering that mode is sufficient."

## 9. WEAKNESS: BP-7 (TextField clipboard) requires external clipboard infrastructure

BP-7 asks for "Ctrl+C -> copy, Ctrl+X -> cut, Ctrl+V -> paste, selection publish on mouse-select." The Rust TextField uses callback-based clipboard integration (`ClipboardCopyCb`, `ClipboardPasteCb` types at `src/widget/text_field.rs:19-20`). Testing clipboard operations through the pipeline requires either: (a) wiring mock clipboard callbacks before the test, or (b) testing that the widget calls the callbacks with correct arguments.

Option (a) is the right approach and works within the existing architecture -- the test sets `tf.clipboard_copy = Some(Box::new(|text| recorded.push(text)))` and verifies the recorded copy text after Ctrl+C. But the prompt's subagent instructions don't mention this setup requirement, and the "do NOT modify production code" constraint is correct. The subagent will need to discover the callback pattern by reading the Rust source, which the prompt says to do for C++ only.

**Amendment**: Add to Domain 1 subagent instructions: "For clipboard operations, wire test callback closures (e.g., `clipboard_copy`, `clipboard_paste`) to Rc<RefCell<Vec<String>>> recorders so you can assert on the text that was copied/pasted. Read the Rust widget source to find the callback field names."

## 10. STRONG: Placing tests in the existing `tests/pipeline/` structure prevents infrastructure sprawl

The prompt says "Place all tests in `tests/pipeline/<widget>.rs` -- add new test functions to the existing module." This is exactly right. The existing modules (`button.rs`, `listbox.rs`, `textfield.rs`, etc.) already have `SharedFooPanel` wrapper patterns, helper functions like `item_center_view_y()`, and `setup_*_harness()` functions. Adding tests to existing modules means subagents inherit working setup code and established conventions. Creating new top-level test files would duplicate this infrastructure and diverge in style.

The only new modules the prompt expects are `focus.rs` (Domain 2) and `notices.rs` (Domain 3), which is appropriate because those test the panel tree, not individual widgets, and have no existing module to extend.

## 11. WEAKNESS: The 24-item all-or-nothing completion rule conflicts with context window reality

The prompt insists "Your job is not finished until all 24 items are DONE" and provides a handoff mechanism only "if approaching context limits." But each BP item involves: dispatching a subagent, waiting for completion, running clippy + nextest (which takes measurable time), logging results, and potentially dispatching a fix subagent. With 24 items, this is at minimum 24 dispatch-verify-log cycles, plus fix cycles for items that expose divergences.

Based on the context brief's experience with fix sessions (Session 1: 19 fixes, Session 2: 12 fixes, Session 3: FileSelectionBox), a single orchestration context can handle roughly 12-19 items before quality degrades. The prompt's completion rule creates a perverse incentive: as the context fills, the agent is forced to complete items faster, which means less careful C++ branch analysis and more superficial tests -- exactly the failure mode the prompt is trying to prevent.

**Amendment**: Split the 24 items into two orchestration sessions: Domain 1 (14 items) in Session A, Domains 2+3 (10 items) in Session B. Each session has its own completion rule. The handoff note between sessions is mandatory, not triggered by "approaching context limits." This removes the incentive to rush late items and makes the handoff point predictable.

## 12. STRONG: "Do NOT modify any production code. Write tests only." prevents scope creep into the most common LLM failure mode

The context brief identifies "over-fixing" (failure mode #3) as "subagents refactored surrounding code while fixing a one-line bug." By constraining test-writing subagents to test-only changes and requiring a SEPARATE fix subagent dispatch for divergences, the prompt creates an explicit handoff boundary. The test subagent cannot rationalize "I'll just fix this small thing while I'm here" because its instructions prohibit production code changes entirely.

**Mechanism**: This works because it leverages the orchestrator's position as a gate. The test subagent reports "test fails with assertion error," and the ORCHESTRATOR makes the decision to dispatch a fix subagent. This prevents the test subagent from both diagnosing and treating the disease, which is where LLM agents most commonly lose scope control. The orchestrator can also decide NOT to fix -- for intentional divergences -- which a combined test+fix subagent would not naturally do.

The one tension: step 2 says "dispatch a FIX subagent for the Rust widget code, not the test." This means the orchestrator needs judgment about WHEN to fix vs. when to log an intentional divergence. The prompt provides no heuristic for this decision, relying on the orchestrator's knowledge of the INTENTIONAL DIVERGENCE list. See amendment in point 3 above.
