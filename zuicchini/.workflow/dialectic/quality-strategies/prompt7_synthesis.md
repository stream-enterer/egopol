# Prompt 7 Synthesis: Concrete Edits

Synthesized from the adversarial critique and defender response for the behavioral parity testing prompt (`prompt-7-behavioral-parity.md`).

**Synthesis principle**: Prefer changes where both agents agree on the problem even if they disagree on severity. Flag areas where the defender's rebuttal relies on assumptions about future usage patterns that may not hold. Reject adversarial critiques that would make the prompt so cautious it produces nothing, and reject defender arguments that amount to "this is fine because it's hard to do better."

---

## Concrete Edits

### Edit 1: Add modifier-key harness instructions to Domain 1 subagent template

**What to change**: In the section "What to tell subagents / For Domain 1 subagents (BP-1 through BP-14)", add the following bullet after the `PipelineTestHarness` bullet:

> - "For modifier-dependent inputs (Shift+click, Ctrl+key, etc.), you MUST set the modifier state on the harness before dispatching the event. Call `harness.input_state.press(Key::Shift)` before the modified click, then `harness.input_state.release(Key::Shift)` after. If the harness does not expose a method for this, create a helper function in the test file that mutates `harness.input_state` directly (it is `pub`). Do NOT bypass the harness to call `widget.input()` directly. If modifier state cannot be set through any public API, log the item as BLOCKED with the reason 'harness lacks modifier state API' and move on."

**Which agent's concern this addresses**: Adversarial critique #1 (harness cannot express modifier-dependent input sequences). The adversary correctly identifies that 10 of 14 Domain 1 items require modifier keys and the prompt gives subagents no guidance on how to set them. The defender does not address this point at all (it is absent from the defense), which amounts to tacit agreement. The three failure modes the adversary identifies (modify harness, bypass harness, encode no-modifier behavior as correct) are all realistic. The fix is simple: tell the subagent how to use the existing `pub` field.

**Why the other agent's position doesn't fully apply**: N/A -- the defender did not contest this point.

---

### Edit 2: Add an intentional-divergence pre-filter to the post-subagent protocol

**What to change**: In the section "After each subagent returns", replace step 2:

Old:
> 2. If tests fail with assertion errors: that's a FINDING. Log it. The test is correct (it encodes C++ behavior); the Rust code diverges. Dispatch a FIX subagent for the Rust widget code, not the test. After fixing, re-run.

New:
> 2. If tests fail with assertion errors: first check whether the failing widget appears in the INTENTIONAL DIVERGENCE list (grep for `INTENTIONAL DIVERGENCE` in `.workflow/widget-comparison/results/*.md`). If it does, log the failure as "assertion encodes C++ behavior that was intentionally diverged -- verify the Rust behavior is the intended replacement" and move on without dispatching a fix subagent. If the widget is NOT in the intentional divergence list: that's a FINDING. Log it. The test is correct (it encodes C++ behavior); the Rust code diverges. Dispatch a FIX subagent for the Rust widget code, not the test. After fixing, re-run.

**Which agent's concern this addresses**: Both agents agree on this. Adversarial critique #2 and #6 argue that the orchestrator blindly assumes all assertion failures are Rust bugs. Defender point #3 concedes the same problem ("the 9 documented INTENTIONAL DIVERGENCE items will trigger false positives") and proposes the same amendment. When both agents independently identify the same fix, adopt it.

**Why the other agent's position doesn't fully apply**: The adversary's broader claim (#6) that the orchestrator "cannot verify" correctness of subagent C++ readings is true but unfixable within this prompt's architecture -- the orchestrator is intentionally a non-code-reading coordinator. The intentional-divergence filter addresses the highest-risk subset (known divergences) without requiring the orchestrator to become a code reader.

---

### Edit 3: Add timing control guidance for double-click and keywalk tests

**What to change**: In the section "What to tell subagents / For Domain 1 subagents (BP-1 through BP-14)", add the following bullet:

> - "For timing-dependent behaviors (double-click detection, keywalk prefix accumulation timeout): if the widget uses `Instant::now()` for timing, check whether it exposes a way to inject time or override the timeout. If it does, use it. If it does not, test the 'fast path' only (events within the timeout window) and log the 'timeout expiry' branch as UNTESTABLE with the reason 'no injectable clock'. Do NOT use `std::thread::sleep()` to simulate timeouts -- this makes the test suite fragile."

Also, in the BP-2 row of the checklist, change the "Branches to cover" column:

Old:
> Type single char -> prefix match, type multiple chars within 1000ms -> accumulated prefix, type `*` prefix -> substring search, no match -> beep, focus lost -> clear accumulator

New:
> Type single char -> prefix match, type multiple chars -> accumulated prefix, type `*` prefix -> substring search, no match -> search string cleared with no item selected, focus lost -> clear accumulator. Note: if the keywalk timeout cannot be tested without `sleep()`, log the timeout-expiry branch as UNTESTABLE.

**Which agent's concern this addresses**: Adversarial critique #3 (double-click timing relies on wall-clock time the harness does not control) and defender point #4 (beep is untestable, rephrase as observable outcome). Both agents agree timing-dependent and side-effect-only behaviors need special handling. The adversary is right that this is a structural limitation. The defender is right that the beep assertion should be rephrased.

**Why the other agent's position doesn't fully apply**: The adversary's proposal would effectively block testing of all timing-dependent items. The edit above allows testing the fast path (which is the common path and still valuable) while honestly logging what cannot be tested, rather than pretending the coverage exists or blocking the entire item.

---

### Edit 4: Reclassify BP-12 (ColorField) or rewrite its specification

**What to change**: In the Domain 1 checklist, replace the BP-12 row:

Old:
> | BP-12 | ColorField expansion | `emColorField.cpp` (Cycle, lines ~100-210) | RGB slider change -> color updates, HSV slider change -> color updates, text field hex input -> color updates, alpha slider when alpha enabled | PENDING |

New:
> | BP-12 | ColorField sub-widget wiring | `emColorField.cpp` (Cycle, lines ~100-210) | Expand ColorField to sub-widget zoom level. Verify: (1) clicking R/G/B sliders updates the color value, (2) entering hex text updates the color value, (3) HSV slider changes update RGB sliders and vice versa. This tests callback wiring between parent and child widgets, not ColorField's own Input() method. Use the `expand_to()` harness method to reach the sub-widget zoom level before dispatching input. | PENDING |

**Which agent's concern this addresses**: Both agents agree BP-12 is misclassified. Adversarial critique #7 correctly identifies that ColorField's `Input()` delegates to child panels and has minimal branching. Defender point #6 agrees and proposes essentially the same reframing. The edit preserves the test item but with correct instructions so the subagent knows what it is actually testing.

**Why the other agent's position doesn't fully apply**: The adversary suggests the misclassification will cause the subagent to produce garbage. The defender suggests it might already be covered by existing tests. Neither position justifies removing the item entirely -- ColorField callback wiring is worth testing, it just needs correct framing.

---

### Edit 5: Add a prerequisite-check phase before Domain 2

**What to change**: Add a new subsection before the Domain 2 table:

> ### Domain 2 Prerequisite Check
>
> Before dispatching any Domain 2 subagent, dispatch a lightweight probe subagent with the following instruction: "Search the Rust source (`src/panel/tree.rs`, `src/panel/view.rs`, `src/panel/behavior.rs`) for Tab focus cycling infrastructure: methods like `focus_next`, `focus_prev`, `cycle_focus`, or any handler for `Key::Tab`. Report what exists and what does not."
>
> If Tab focus cycling infrastructure does not exist: log BP-15, BP-16, and BP-18 as BLOCKED with the finding "Tab focus cycling not implemented -- infrastructure gap." Do not dispatch fix subagents to implement focus cycling; that is out of scope for this prompt. Proceed to BP-17 and BP-19, which test click-activation and arrow-key navigation respectively and may be testable with existing infrastructure.

**Which agent's concern this addresses**: Defender point #2 (the prompt assumes Tab/focus traversal exists -- it does not). The adversary does not raise this specific point, but adversarial critique #8 (the prompt has no mechanism to detect missing Rust API surface) covers the general case. The defender's observation that `focus_next`/`focus_prev` do not exist in the Rust code is a factual finding that prevents 3 of 5 Domain 2 items from succeeding.

**Why the other agent's position doesn't fully apply**: The adversary's general critique (#8) would suggest blocking many more items preemptively, which would make the prompt too cautious. The edit targets only the items the defender has factually verified as unimplementable, and logs them as infrastructure gaps rather than silently skipping them.

---

### Edit 6: Add clipboard callback setup to Domain 1 subagent instructions

**What to change**: In the section "What to tell subagents / For Domain 1 subagents (BP-1 through BP-14)", add:

> - "For BP-7 (clipboard operations): the Rust TextField uses callback-based clipboard integration (`ClipboardCopyCb`, `ClipboardPasteCb`). Before testing Ctrl+C/X/V, wire test callback closures to `Rc<RefCell<Vec<String>>>` recorders so you can assert on the text that was copied/pasted. Read the Rust widget source (`src/widget/text_field.rs`) to find the callback field names. This is an exception to the general rule of reading only C++ source -- the clipboard integration pattern is Rust-specific."

**Which agent's concern this addresses**: Defender point #9 (BP-7 requires clipboard callbacks the subagent won't know about without reading Rust source). The adversary raises the general clipboard concern in critique #8 but does not propose a fix. The defender provides the specific amendment. Since the test cannot work without this setup, and the information cannot be derived from C++ source alone, telling the subagent upfront is strictly better than letting it discover the problem mid-execution.

**Why the other agent's position doesn't fully apply**: The adversary's broader critique (#8) argues the prompt should have a general mechanism for API surface mismatch detection. That is a valid systemic concern but too expensive to operationalize for every item. The targeted fix for BP-7 handles the known case.

---

### Edit 7: Specify RecordingBehavior for ordering-sensitive Domain 3 tests

**What to change**: In the section "What to tell subagents / For Domain 3 subagents (BP-20 through BP-24)", replace:

Old:
> - "Build a panel tree, attach `NoticeBehavior` or `RecordingBehavior` (from `tests/support/mod.rs`) to capture notice flags"
> [...]
> - "Assert the accumulated `NoticeFlags` on each panel match what C++ would fire"

New:
> - "Build a panel tree, attach `RecordingBehavior` (from `tests/support/mod.rs`) to capture notice events with ordering information. Use `NoticeBehavior` only when you need simple flag-set assertions and ordering does not matter."
> [...]
> - "Assert TWO things: (1) the SET of `NoticeFlags` delivered to each panel matches what C++ would fire, AND (2) for BP-21 and BP-24 (where propagation order matters), assert that the SEQUENCE of notice deliveries matches C++ order using `RecordingBehavior`'s ordered log. For example, for focus change, assert that the old focused panel receives `FOCUS_CHANGED` before the new focused panel."

**Which agent's concern this addresses**: Adversarial critique #9 (notice ordering assertions conflate set membership with sequence). The adversary correctly identifies that `NoticeBehavior` accumulates flags via `insert()` and cannot observe delivery order. The defender does not address this point. Since the C++ code has specific ordering guarantees (old-panel-before-new-panel for focus changes, parent-before-children for layout), and ordering bugs cause stale-data cascades, testing order is not optional for this domain.

**Why the other agent's position doesn't fully apply**: The defender's silence suggests agreement that the current instructions default subagents to `NoticeBehavior`. The adversary's critique is well-founded. The edit does not require ordering assertions for all 5 Domain 3 items -- only the 2 where C++ explicitly documents propagation order (BP-21 focus changes, BP-24 active changes).

---

### Edit 8: Add a compilation-failure triage step to the post-subagent protocol

**What to change**: In the section "After each subagent returns", expand step 3:

Old:
> 3. If tests fail with compilation errors: dispatch a fix subagent for the test code.

New:
> 3. If tests fail with compilation errors: determine the cause. (a) If the error is a typo, wrong method name, or incorrect type in the test code: dispatch a fix subagent for the test code. (b) If the error is because the Rust widget does not expose a method or type that the C++ code has (e.g., no `keywalk_prefix()` accessor, no `DragMode` enum): log the item as a FINDING with category "API surface gap -- C++ feature `X` has no Rust equivalent" and mark the specific branch as UNTESTABLE. Do NOT dispatch a fix subagent to add the missing API -- that is production code work outside this prompt's scope. Write tests for the branches that ARE testable and mark the item PARTIAL if some branches could not be covered.

**Which agent's concern this addresses**: Adversarial critique #8 (no mechanism to detect Rust API surface differences from C++) and critique #4 (the "no production code" constraint prevents testing internal state). Both agents acknowledge the tension: the adversary says missing features get silently erased, the defender (point #12) says the test/production separation prevents scope creep. The edit resolves the tension by keeping the "no production code" constraint but adding an explicit triage path for API gaps, so they are tracked as findings rather than either silently dropped or scope-creeped into production changes.

**Why the other agent's position doesn't fully apply**: The adversary's concern that missing features become invisible is valid, but the proposed remedy (allowing production code changes) would reintroduce the scope creep the defender rightly warns against. The edit tracks gaps explicitly without allowing the subagent to "fix" them.

---

### Edit 9: Reframe BP-1 to test distinct behaviors rather than the full mode-x-input cross product

**What to change**: In the BP-1 row, replace the "Branches to cover" column:

Old:
> Single click, Shift+click range, Ctrl+click toggle, double-click trigger. Test each in SINGLE, MULTI, TOGGLE, READ_ONLY modes.

New:
> Test each DISTINCT behavior: (1) single click selects in SINGLE/MULTI/TOGGLE modes, (2) Shift+click extends range -- only meaningful in MULTI mode, (3) Ctrl+click toggles -- only meaningful in MULTI and TOGGLE modes, (4) double-click triggers in all modes, (5) READ_ONLY mode rejects all selection mutations. Do not write redundant tests for modifier combinations that reduce to single-click behavior in a given mode.

**Which agent's concern this addresses**: Defender point #8 (the full cross product produces redundant tests that waste context). The adversary's critique #11 (branch vs. behavioral path) pulls in the opposite direction, arguing the cross product is too SMALL. However, the adversary is talking about interaction effects between branches (e.g., Shift+Ctrl in Multi mode), not the basic mode-x-input matrix. The edit reduces the matrix to distinct behaviors while adding room for the interaction cases the adversary cares about.

**Why the other agent's position doesn't fully apply**: The adversary's argument (#11) that "behavioral paths" are 2^N is theoretically correct but operationally paralyzing -- no prompt can enumerate 256 paths for a single widget. The edit focuses on the cases where mode genuinely changes behavior, which covers the highest-risk divergences.

---

### Edit 10: Split the 24-item workload into two sessions with a mandatory handoff

**What to change**: In the "Completion protocol" section, replace the current text with:

> **Session structure**: This checklist is divided into two sessions.
>
> - **Session A**: Domain 1 (BP-1 through BP-14). Your job in Session A is not finished until all 14 items are DONE or BLOCKED.
> - **Session B**: Domain 2 (BP-15 through BP-19) + Domain 3 (BP-20 through BP-24). Run in a fresh context.
>
> At the end of Session A, write a mandatory handoff note in `.workflow/widget-comparison/run-log.md` with: items completed, items BLOCKED with reasons, divergences found, fixes applied, and any observations about harness limitations that Session B should know.
>
> **Anti-satisficing rule (per session)**: If you catch yourself composing "The core widget interactions are tested" or "The highest-risk behaviors are covered" -- that is early completion. Session A has 14 items. Completing 10 is not done. Session B has 10 items. Completing 7 is not done.
>
> **If approaching context limits within a session**: Write a handoff note with remaining PENDING item IDs and tell the user "N items remain."

**Which agent's concern this addresses**: Defender point #11 (24-item all-or-nothing rule conflicts with context window reality) and adversarial critique #10 (anti-satisficing rule targets the wrong failure mode). The defender provides evidence from the project's own history that single sessions handle 12-19 items before quality degrades. The adversary argues that 24 items of structurally limited tests are worse than fewer well-crafted tests. Both concerns are partially addressed: the session split prevents late-context quality degradation (defender's concern), and the per-session anti-satisficing rule still prevents early abandonment (the legitimate target of the original rule).

**Why the other agent's position doesn't fully apply**: The adversary's argument (#10) that the anti-satisficing rule should be replaced with a depth-over-breadth approach would fundamentally change the prompt's architecture from a checklist executor to a judgment-heavy analyzer. That may be better in theory, but it reintroduces the subjective "am I done?" question that the rule was designed to eliminate. The session split is a more conservative fix that preserves the checklist structure.

---

### Edit 11: Add a note about test wrapper fidelity to Domain 1 subagent instructions

**What to change**: In the section "What to tell subagents / For Domain 1 subagents (BP-1 through BP-14)", add:

> - "If the existing pipeline test module for this widget already has a `SharedFooPanel` wrapper and `setup_*_harness()` function, reuse them. Do NOT create a new wrapper unless the existing one is missing. If you must create a new wrapper, model it on the existing ones in the same directory and forward the same set of notice types. If you are unsure which notices to forward, forward all notice types (`NoticeFlags::all()`) to avoid silently masking a propagation bug."

**Which agent's concern this addresses**: Adversarial critique #12 (no validation that PanelBehavior wrappers match production wiring). The adversary is right that N different wrappers forwarding different notice subsets creates N implicit integration contracts that may diverge from production. The defender does not address this point (point #10 praises the module structure but not the wrapper consistency). The edit mitigates the risk by mandating reuse of existing wrappers and defaulting new wrappers to forwarding all notices.

**Why the other agent's position doesn't fully apply**: The adversary's concern about wrapper drift is valid, but the proposed scope (validating every wrapper against production wiring) would require a separate audit and is out of scope for this prompt. The practical mitigation is to avoid creating new wrappers unnecessarily and to default new ones to maximum notice forwarding.

---

## Unresolved Tensions

These are areas where the adversary and defender are both right, and the synthesis cannot resolve the tension without a value judgment from the prompt author.

### Tension A: LLM-read C++ as oracle vs. ground truth

The adversary (critiques #2 and #6) argues that subagent-interpreted C++ is an unreliable oracle, pointing to the documented `word_boundary` vs `word_index` misreading. The defender (point #3) argues the presumption is "mostly right" and that the fix subagent will catch misreadings when it examines the Rust code. Both are correct: the C++ reading is the best available oracle that does not require running C++ binaries, but it has a nonzero error rate.

**The unresolved question**: Should the prompt add a "second-reader" verification step where a separate subagent re-reads the C++ code for any assertion failure before dispatching a fix? This would catch LLM misreadings but double the cost of every divergence finding. The prompt author must decide whether the misreading risk (documented as real but infrequent) justifies the cost.

### Tension B: Branch coverage vs. behavioral path coverage

The adversary (#11) argues that testing one context per branch is structurally insufficient because interaction effects between branches (Shift+Ctrl in Multi mode, double-click in Toggle mode) are where porting bugs live. The defender (#8) implicitly agrees by noting redundant tests in the cross product but does not address the adversary's deeper point about untested interactions.

**The unresolved question**: Should the prompt instruct subagents to test modifier COMBINATIONS (not just individual modifiers), and if so, which combinations? Exhaustive cross-product testing is infeasible. The prompt author must decide whether to add a heuristic (e.g., "for each pair of modifiers that the C++ code checks independently, write one test with both modifiers active") or accept that individual-modifier testing is sufficient for this layer.

### Tension C: Cross-widget interaction testing vs. single-widget isolation

The adversary (#5) argues that testing widgets in isolation misses interaction bugs (RadioButton exclusion requires multiple RadioButtons in a RadioBox). The defender does not address this. The adversary is right that the current structure cannot test exclusion, but adding cross-widget tests would require a different subagent architecture (one subagent gets multiple widgets) and different harness setup (multiple widgets in a panel tree).

**The unresolved question**: Should BP-13 (RadioButton exclusion) be rewritten to explicitly require a multi-widget test setup? This is a design decision: does "behavioral parity" mean each widget in isolation, or does it mean the widget system as composed? The prompt author must decide the scope boundary. A minimal resolution would be to add a note to BP-13: "This test requires multiple RadioButton instances in a shared RadioBox/RadioButton::Group to test exclusion behavior."

### Tension D: "No production code" purity vs. testability of internal state

The adversary (#4) argues that many assertions need internal state accessors that the Rust API does not expose, so tests degrade to final-outcome assertions. The defender (#12) argues the test/production boundary prevents scope creep, which is a documented and serious failure mode. Both are right.

**The unresolved question**: Should the prompt allow subagents to add `#[cfg(test)] pub(crate)` accessors to production code for internal state that tests need? This is common Rust practice and would not affect the public API. But it opens the door to production code changes, which the defender correctly identifies as the primary scope creep vector. The prompt author must decide whether the testability gain outweighs the scope creep risk.
