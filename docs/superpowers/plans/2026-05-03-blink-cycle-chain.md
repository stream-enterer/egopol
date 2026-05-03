# Blink-Cycle Chain Implementation Plan (B2.1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Identify the specific link in the post-handler chain (notice handler → wake → engine pickup → Cycle → InvalidatePainting) where the cursor-blink path breaks for focused TextFieldPanels, then land a small (≤30 LOC) verdict-dispatched fix and a regression test that catches the bug class.

**Architecture:** Measure-then-fix, mirrors B2. Phase 0 adds three structured log lines (`HANDLER_ENTRY`, `WUP_RESULT`, `CYCLE_ENTRY`) on `instr/blink-cycle-2026-05-03` (cut from `instr-blink-2026-05-03` tag), revises the analyzer's click-target heuristic, and emits a 9-bin verdict per spec § Phase 0 → outcome dispatch. Phase 1 dispatches a verdict-keyed fix in 1-3 emcore files (≤30 LOC) plus a regression test. Ambiguous or out-of-budget verdicts escalate to B2.2.

**Tech Stack:** Rust (`emcore`, `emtest`), Python 3 (`scripts/analyze_hang.py`), `dlog!`/`emInstr` shared-FD logging, manual GUI session for capture.

**Spec:** `docs/superpowers/specs/2026-05-03-blink-cycle-chain-design.md`

**Predecessors:**
- `docs/scratch/2026-05-03-blink-trace-results.md` — B2 Phase 0 findings + handoff
- `docs/scratch/2026-05-03-set-active-panel-missing-wake.md` — B1 D1 deferral (out of scope)
- Tag `instr-blink-2026-05-03` — B2's archived instr-branch tip (base for B2.1's instr branch)

---

## File Structure

### Phase 0 (lives on `instr/blink-cycle-2026-05-03`, never merges to main)

| Path | Responsibility | Action |
|---|---|---|
| `crates/emtest/src/emTestPanel.rs` | Add HANDLER_ENTRY in test TextFieldPanel::notice (line ~246) | Modify |
| `crates/emcore/src/emColorFieldFieldPanel.rs` | Add HANDLER_ENTRY in production TextFieldPanel::notice (line ~148) | Modify |
| `crates/emcore/src/emEngineCtx.rs` | Restructure wake_up_panel to single-exit + emit WUP_RESULT (line ~847) | Modify |
| `crates/emcore/src/emPanelCycleEngine.rs` | Add CYCLE_ENTRY at both behavior.Cycle sites (lines ~122, ~237) | Modify |
| `scripts/analyze_hang.py` | Parse 3 new lines; revise click-target heuristic; emit B2.1 9-bin verdict | Modify |
| `scripts/test_analyze_hang.py` | Coverage tests for parsers + verdict bins | Modify |

### Phase 0 documentation (lives on `main`)

| Path | Responsibility | Action |
|---|---|---|
| `docs/scratch/2026-05-03-blink-cycle-results.md` | B2.1 verdict + chain-trace + prediction calibration | Create |

### Phase 1 (lives on `fix/blink-cycle-chain-2026-05-03`, merges to main)

| Path | Responsibility | Action |
|---|---|---|
| 1-3 files in `crates/emcore/src/` (verdict-dispatched per spec) | Phase 1 fix per verdict | Modify |
| `crates/emcore/tests/blink_focus_path.rs` | `focused_text_field_engine_wakes_and_cycles` regression test | Create |

---

## Branching discipline

```
main (01aa60de)
 │
 ├─[cut Task 1]─→ instr/blink-cycle-2026-05-03    (Phase 0; from tag instr-blink-2026-05-03)
 │                  │ commit per task: 2, 3, 4, 5, 6, 8, 9, 11
 │                  │ capture happens at Task 12 (manual GUI; on instr branch)
 │                  │ at Task 13, RESULTS commit goes to main, NOT this branch
 │                  └─ tag instr-blink-cycle-2026-05-03 at end (Task 14), branch retained
 │
 ├─ direct commits on main: findings doc + dispatch decision (Tasks 13, 14)
 │
 └─[cut Tasks 15-20]─→ fix/blink-cycle-chain-2026-05-03   (Phase 1; from main)
                         │ commit: verdict-dispatched fix (1-2 commits)
                         │ commit: regression test
                         │ recapture at Task 23 uses instr-blink-cycle-2026-05-03 + fix cherry-picked
                         └─[merge Task 24]─→ main
```

**Hard rules:**
- Never merge instr branch to main.
- Never push `--force`.
- Always run pre-commit hook (do not pass `--no-verify`).
- Per `CLAUDE.md`: read C++ source (`~/Projects/eaglemode-0.96.4/`) before changing Rust to confirm correct ordering. Phase 1 fix sketches in the spec name the relevant C++ functions per outcome.
- Skip per-task `cargo-nextest ntr` in subagent loops; pre-commit hook is source of truth per commit (per memory `feedback_skip_nextest_per_task.md`).

---

## Task 1: Set up Phase 0 instrumentation branch

**Files:** No code edits; branch creation only.

- [ ] **Step 1: Verify clean working tree on main**

```bash
git status
```

Expected: `On branch main`. Untracked files unrelated to this plan (`docs/debug/investigations/dir-panel-cycle-busy-loop.md`, `docs/superpowers/plans/2026-05-02-hang-instrumentation-plan.md`, `scripts/__pycache__/`) are acceptable — they don't conflict with branch operations.

- [ ] **Step 2: Cut instrumentation branch from instr-blink-2026-05-03 tag**

```bash
git checkout -b instr/blink-cycle-2026-05-03 instr-blink-2026-05-03
git log --oneline -3
```

Expected first line: a commit from B2's Phase 0 instrumentation work (`71260017 analyzer: phase 0 (B2) unit tests for new parsers and verdict logic` or similar).

- [ ] **Step 3: Verify the build still works on this base**

```bash
cargo check
```

Expected: PASS. If it fails, investigate before proceeding — the base must be buildable.

---

## Task 2: Add HANDLER_ENTRY in test panel `TextFieldPanel::notice`

**Files:**
- Modify: `crates/emtest/src/emTestPanel.rs` (the `TextFieldPanel::notice` method around line 246).

- [ ] **Step 1: Locate the notice handler**

```bash
grep -n "fn notice" crates/emtest/src/emTestPanel.rs | head -5
```

Expected: a hit at line ~246 inside `impl PanelBehavior for TextFieldPanel`. Also other notice impls (different widgets); the target is the TextFieldPanel one.

- [ ] **Step 2: Read the current notice handler body**

```bash
sed -n '246,260p' crates/emtest/src/emTestPanel.rs
```

Expected (instr-blink-2026-05-03 base):

```rust
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState, ctx: &mut PanelCtx) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.is_focused = state.in_focused_path();
            self.widget.on_focus_changed(self.is_focused);
            // Mirrors C++ emTextField::Notice (emTextField.cpp:343-350):
            // RestartCursorBlinking + WakeUp guarded by IsInFocusedPath().
            if self.is_focused {
                self.widget.RestartCursorBlinking();
                let id = ctx.id;
                ctx.wake_up_panel(id);
            }
        }
    }
```

- [ ] **Step 3: Replace the body to add HANDLER_ENTRY emission at the end of the FC block**

Replace the `if flags.intersects(...) { ... }` block with:

```rust
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            let in_focused_path = state.in_focused_path();
            self.is_focused = in_focused_path;
            self.widget.on_focus_changed(self.is_focused);
            // Mirrors C++ emTextField::Notice (emTextField.cpp:343-350):
            // RestartCursorBlinking + WakeUp guarded by IsInFocusedPath().
            if self.is_focused {
                self.widget.RestartCursorBlinking();
                let id = ctx.id;
                ctx.wake_up_panel(id);
            }
            // Phase 0 (B2.1): HANDLER_ENTRY — emit at end of FOCUS_CHANGED
            // block so the analyzer can detect whether the body ran and
            // whether the focus-dependent branch was taken.
            let line = format!(
                "HANDLER_ENTRY|wall_us={}|panel_id={:?}|impl=emTestPanel::TextFieldPanel|flags={:#x}|is_focused_path={}|branch_taken={}\n",
                emcore::emInstr::wall_us(),
                ctx.id,
                flags.bits(),
                if in_focused_path { "t" } else { "f" },
                if self.is_focused { "t" } else { "f" },
            );
            emcore::emInstr::write_line(&line);
        }
```

The change hoists `state.in_focused_path()` into a local `in_focused_path` so it can be logged alongside `self.is_focused` (they should always be equal; logged for sanity).

- [ ] **Step 4: Build to verify**

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 5: Commit (allow pre-commit hook to run)**

```bash
git add crates/emtest/src/emTestPanel.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2.1) HANDLER_ENTRY in test TextFieldPanel notice

Captures (panel_id, impl, flags, is_focused_path, branch_taken) at end
of the FOCUS_CHANGED block so the analyzer can detect whether the
notice body ran and whether the focus-dependent branch entered.
EOF
)"
```

---

## Task 3: Add HANDLER_ENTRY in production `TextFieldPanel::notice`

**Files:**
- Modify: `crates/emcore/src/emColorFieldFieldPanel.rs` (the `TextFieldPanel::notice` method around line 148).

- [ ] **Step 1: Locate the notice handler**

```bash
grep -n "fn notice" crates/emcore/src/emColorFieldFieldPanel.rs | head -5
```

Expected: a hit at line ~148 inside `impl PanelBehavior for TextFieldPanel`.

- [ ] **Step 2: Read the current notice handler body**

```bash
sed -n '148,162p' crates/emcore/src/emColorFieldFieldPanel.rs
```

Expected:

```rust
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState, ctx: &mut PanelCtx) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.is_focused = state.in_focused_path();
            self.text_field.on_focus_changed(self.is_focused);
            // Mirrors C++ emTextField::Notice (emTextField.cpp:343-350):
            // RestartCursorBlinking() + WakeUp() guarded by IsInFocusedPath()
            // so they fire on focus-gain only, not focus-loss.
            if self.is_focused {
                self.text_field.RestartCursorBlinking();
                let id = ctx.id;
                ctx.wake_up_panel(id);
            }
        }
    }
```

- [ ] **Step 3: Replace the body to add HANDLER_ENTRY emission**

Replace with:

```rust
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState, ctx: &mut PanelCtx) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            let in_focused_path = state.in_focused_path();
            self.is_focused = in_focused_path;
            self.text_field.on_focus_changed(self.is_focused);
            // Mirrors C++ emTextField::Notice (emTextField.cpp:343-350):
            // RestartCursorBlinking() + WakeUp() guarded by IsInFocusedPath()
            // so they fire on focus-gain only, not focus-loss.
            if self.is_focused {
                self.text_field.RestartCursorBlinking();
                let id = ctx.id;
                ctx.wake_up_panel(id);
            }
            // Phase 0 (B2.1): HANDLER_ENTRY — emit at end of FOCUS_CHANGED
            // block so the analyzer can detect whether the body ran and
            // whether the focus-dependent branch was taken.
            let line = format!(
                "HANDLER_ENTRY|wall_us={}|panel_id={:?}|impl=emColorFieldFieldPanel::TextFieldPanel|flags={:#x}|is_focused_path={}|branch_taken={}\n",
                crate::emInstr::wall_us(),
                ctx.id,
                flags.bits(),
                if in_focused_path { "t" } else { "f" },
                if self.is_focused { "t" } else { "f" },
            );
            crate::emInstr::write_line(&line);
        }
    }
```

The `impl=` field discriminates this from the test-panel HANDLER_ENTRY. Note `crate::emInstr::` here (vs `emcore::emInstr::` in emtest) — emcore-internal access.

- [ ] **Step 4: Build to verify**

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emColorFieldFieldPanel.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2.1) HANDLER_ENTRY in production TextFieldPanel notice

Mirrors the test-panel HANDLER_ENTRY emission added in Task 2. impl=
field distinguishes the two sites in the analyzer.
EOF
)"
```

---

## Task 4: Restructure `wake_up_panel` and add WUP_RESULT

**Files:**
- Modify: `crates/emcore/src/emEngineCtx.rs` (`wake_up_panel` at line ~847).

- [ ] **Step 1: Locate wake_up_panel**

```bash
grep -n "fn wake_up_panel" crates/emcore/src/emEngineCtx.rs
```

Expected: one hit at line ~847.

- [ ] **Step 2: Read the current implementation**

```bash
sed -n '847,860p' crates/emcore/src/emEngineCtx.rs
```

Expected:

```rust
    pub fn wake_up_panel(&mut self, id: PanelId) {
        let Some(panel) = self.tree.GetRec(id) else {
            return;
        };
        let Some(eid) = panel.engine_id else {
            return;
        };
        if let Some(sched) = self.scheduler.as_deref_mut() {
            sched.wake_up(eid);
        }
    }
```

- [ ] **Step 3: Restructure to single-exit form with WUP_RESULT emission**

Replace the function body with:

```rust
    #[track_caller]
    pub fn wake_up_panel(&mut self, id: PanelId) {
        let caller = std::panic::Location::caller();

        // Compute panel state without holding the borrow into the dispatch.
        // engine_id is Copy (small), so we can extract it and drop the
        // panel reference before borrowing scheduler.
        let (panel_found, engine_id) = match self.tree.GetRec(id) {
            Some(panel) => (true, panel.engine_id),
            None => (false, None),
        };

        let scheduler_some = self.scheduler.is_some();

        let wake_dispatched = match (engine_id, self.scheduler.as_deref_mut()) {
            (Some(eid), Some(sched)) => {
                sched.wake_up(eid);
                true
            }
            _ => false,
        };

        // Phase 0 (B2.1): WUP_RESULT — single-exit emit so all guard
        // outcomes can be distinguished by the analyzer. wake_dispatched=t
        // iff sched.wake_up was actually invoked (which itself emits a
        // WAKE log line unconditionally).
        let line = format!(
            "WUP_RESULT|wall_us={}|panel_id={:?}|caller={}:{}|panel_found={}|engine_id={:?}|scheduler_some={}|wake_dispatched={}\n",
            crate::emInstr::wall_us(),
            id,
            caller.file(),
            caller.line(),
            if panel_found { "t" } else { "f" },
            engine_id,
            if scheduler_some { "t" } else { "f" },
            if wake_dispatched { "t" } else { "f" },
        );
        crate::emInstr::write_line(&line);
    }
```

The `#[track_caller]` attribute lets the emit capture the call site. The borrow checker accepts the structure because `engine_id: Option<EngineId>` is `Copy`, so the `panel` reference into `self.tree` is dropped before `self.scheduler.as_deref_mut()` is invoked.

- [ ] **Step 4: Build to verify**

```bash
cargo check
```

Expected: PASS. If borrow checker complains, the most likely cause is `self.tree.GetRec(id)` returning a borrow that outlives the match — verify EngineId is `Copy` via `grep "Copy" crates/emcore/src/emEngine.rs | head` and adjust if needed (e.g., extract via `.copied()`).

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emEngineCtx.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2.1) WUP_RESULT in wake_up_panel single-exit form

Restructures wake_up_panel to compute (panel_found, engine_id,
scheduler_some, wake_dispatched) up front, then emit WUP_RESULT once at
exit. Adds #[track_caller] so the call-site file:line is captured. The
analyzer can now distinguish OB1 (panel disappeared), OB2 (engine_id
None), OB3 (scheduler None), and OB4-reached (wake_dispatched=t) cases
per the B2.1 spec.
EOF
)"
```

---

## Task 5: Add CYCLE_ENTRY at both PanelCycleEngine.Cycle sites

**Files:**
- Modify: `crates/emcore/src/emPanelCycleEngine.rs` (both `behavior.Cycle(...)` invocations at lines ~122 and ~237).

- [ ] **Step 1: Locate both invocation sites**

```bash
grep -n "behavior\.Cycle\|let busy = behavior" crates/emcore/src/emPanelCycleEngine.rs
```

Expected: two hits, around lines 122 and 237. They look identical syntactically (`let busy = behavior.Cycle(&mut ectx, &mut pctx);`).

- [ ] **Step 2: Read both sites**

```bash
sed -n '118,128p' crates/emcore/src/emPanelCycleEngine.rs
echo "---"
sed -n '233,243p' crates/emcore/src/emPanelCycleEngine.rs
```

Expected: each shows the same pattern — a `pctx` constructed just above, then `let busy = behavior.Cycle(&mut ectx, &mut pctx);` followed by INVAL_DRAIN-related code.

- [ ] **Step 3: Insert CYCLE_ENTRY before the first behavior.Cycle (Toplevel path, line ~122)**

Find:

```rust
                    let busy = behavior.Cycle(&mut ectx, &mut pctx);
```

(first occurrence around line 122)

Replace with:

```rust
                    // Phase 0 (B2.1): CYCLE_ENTRY (Toplevel path) — emit
                    // before behavior.Cycle so the analyzer can detect
                    // whether DoTimeSlice picked up the woken engine and
                    // PanelCycleEngine routed to a behavior.
                    {
                        let line = format!(
                            "CYCLE_ENTRY|wall_us={}|engine_id={:?}|panel_id={:?}|behavior_type={}\n",
                            crate::emInstr::wall_us(),
                            ctx.engine_id,
                            self.panel_id,
                            std::any::type_name_of_val(&*behavior),
                        );
                        crate::emInstr::write_line(&line);
                    }
                    let busy = behavior.Cycle(&mut ectx, &mut pctx);
```

- [ ] **Step 4: Insert CYCLE_ENTRY before the second behavior.Cycle (SubView path, line ~237)**

Apply the same replacement to the second occurrence (around line 237). Since the surrounding code differs in whitespace/indentation, copy the exact `let busy = behavior.Cycle(&mut ectx, &mut pctx);` line as `old_string` for the second `Edit` and add the CYCLE_ENTRY block right above. The CYCLE_ENTRY block content is identical to the Toplevel one.

The `Edit` tool with `replace_all=false` requires unique `old_string`. To target each site individually, include enough surrounding context to uniquify. Use the comment line above each behavior.Cycle call:
- Toplevel path: precede with the relevant context line (e.g., the `pctx_tree` usage from line ~108).
- SubView path: precede with the `sub_tree_mut()` usage line (~226).

Alternative: use `replace_all=true` with the simple `let busy = behavior.Cycle(&mut ectx, &mut pctx);` line, since both sites need the same insertion.

- [ ] **Step 5: Build to verify**

```bash
cargo check
```

Expected: PASS. If `type_name_of_val` is unstable on the toolchain, fall back to `std::any::type_name::<dyn crate::emPanel::PanelBehavior>()` (which always returns the trait object name) and document the fallback in the comment.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emPanelCycleEngine.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2.1) CYCLE_ENTRY at both behavior.Cycle sites

Emits CYCLE_ENTRY immediately before each behavior.Cycle invocation in
PanelCycleEngine (Toplevel + SubView paths). Combined with the existing
unconditional BLINK_CYCLE log inside TextFieldPanel.Cycle, this lets the
analyzer distinguish OC-NOPICKUP (CYCLE_ENTRY absent) from OC-DISPATCH
(CYCLE_ENTRY present, BLINK_CYCLE absent) per the B2.1 spec.
EOF
)"
```

---

## Task 6: Verify Phase 0 instrumentation does not break the build or unit tests

**Files:** none (verification only).

- [ ] **Step 1: Run clippy with warnings as errors**

```bash
cargo clippy -- -D warnings
```

Expected: PASS. If warnings, fix at the source (do not `#[allow]` per `CLAUDE.md`).

- [ ] **Step 2: Visual sanity check on instrumentation**

```bash
EM_INSTR_FD=9 cargo test --test fu005_file_state_signal -- --test-threads=1 9>/tmp/em_instr.smoke.log
grep -c "HANDLER_ENTRY\|WUP_RESULT\|CYCLE_ENTRY" /tmp/em_instr.smoke.log
```

Expected: a non-negative integer (likely 0 if these tests don't exercise focus changes). Format must compile and be writable. If non-zero, inspect a sample line and confirm the format matches the spec.

(Skip a full `cargo-nextest ntr` here per memory `feedback_skip_nextest_per_task.md`; the pre-commit hook ran nextest on each commit already.)

---

## Task 7: Extend `analyze_hang.py` parsers for the three new line types

**Files:**
- Modify: `scripts/analyze_hang.py` (add three parse functions after the existing B2 parsers).

- [ ] **Step 1: Read the existing B2 parser shape**

```bash
grep -n "def parse_notice_fc_decode\|def parse_set_active_result\|def parse_set_focused_result" scripts/analyze_hang.py
```

Expected: three hits around lines 107, 124, 141 (B2 added these).

- [ ] **Step 2: Append three new parse functions immediately after the B2 parsers**

```python
def parse_handler_entry(line):
    """B2.1 Phase 0: parse HANDLER_ENTRY."""
    try:
        f = _parse_kv_line(line, "HANDLER_ENTRY")
    except ValueError:
        return None
    return {
        "kind": "HANDLER_ENTRY",
        "wall_us": int(f["wall_us"]),
        "panel_id": f["panel_id"],
        "impl": f.get("impl", ""),
        "flags": int(f["flags"], 16),
        "is_focused_path": f["is_focused_path"] == "t",
        "branch_taken": f["branch_taken"] == "t",
    }


def parse_wup_result(line):
    """B2.1 Phase 0: parse WUP_RESULT."""
    try:
        f = _parse_kv_line(line, "WUP_RESULT")
    except ValueError:
        return None
    return {
        "kind": "WUP_RESULT",
        "wall_us": int(f["wall_us"]),
        "panel_id": f["panel_id"],
        "caller": f.get("caller", ""),
        "panel_found": f["panel_found"] == "t",
        "engine_id": f["engine_id"],
        "scheduler_some": f["scheduler_some"] == "t",
        "wake_dispatched": f["wake_dispatched"] == "t",
    }


def parse_cycle_entry(line):
    """B2.1 Phase 0: parse CYCLE_ENTRY."""
    try:
        f = _parse_kv_line(line, "CYCLE_ENTRY")
    except ValueError:
        return None
    return {
        "kind": "CYCLE_ENTRY",
        "wall_us": int(f["wall_us"]),
        "engine_id": f["engine_id"],
        "panel_id": f["panel_id"],
        "behavior_type": f["behavior_type"],
    }
```

- [ ] **Step 3: Smoke test the parsers**

```bash
python3 -c "
import sys; sys.path.insert(0, 'scripts')
from analyze_hang import parse_handler_entry, parse_wup_result, parse_cycle_entry
print(parse_handler_entry('HANDLER_ENTRY|wall_us=100|panel_id=PanelId(125v1)|impl=emTestPanel::TextFieldPanel|flags=0xf0|is_focused_path=t|branch_taken=t'))
print(parse_wup_result('WUP_RESULT|wall_us=200|panel_id=PanelId(125v1)|caller=crates/emtest/src/emTestPanel.rs:255|panel_found=t|engine_id=Some(EngineId(7v1))|scheduler_some=t|wake_dispatched=t'))
print(parse_cycle_entry('CYCLE_ENTRY|wall_us=300|engine_id=EngineId(7v1)|panel_id=PanelId(125v1)|behavior_type=emTestPanel::TextFieldPanel'))
"
```

Expected: three populated dicts.

- [ ] **Step 4: Commit**

```bash
git add scripts/analyze_hang.py
git commit -m "$(cat <<'EOF'
analyzer: phase 0 (B2.1) parsers for HANDLER_ENTRY / WUP_RESULT / CYCLE_ENTRY

Adds parse_handler_entry, parse_wup_result, parse_cycle_entry — the
three new structured log lines emitted by the B2.1 instrumentation
across emTestPanel, emColorFieldFieldPanel, emEngineCtx, and
emPanelCycleEngine.
EOF
)"
```

---

## Task 8: Revise click-target heuristic and emit B2.1 verdict

**Files:**
- Modify: `scripts/analyze_hang.py` (revise the blink command's target-picking; add 9-bin verdict logic).

- [ ] **Step 1: Locate the current target-picking and verdict-emission code**

```bash
grep -n "def blink_command_text\|def _phase0_verdict\|recipient_panel_id\|target_panel_id" scripts/analyze_hang.py | head -20
```

Expected: B2's `_phase0_verdict` at ~line 333; `blink_command_text` later. The current target picker uses focus-transition events.

- [ ] **Step 2: Add a helper to pick the click target from SET_ACTIVE_RESULT**

Append this helper near `_phase0_verdict`:

```python
def _pick_click_target(set_active_events, marker_open_us, marker_close_us):
    """B2.1: pick the click target as the latest SET_ACTIVE_RESULT
    with window_focused=t between the open and close markers."""
    candidates = [
        ev for ev in set_active_events
        if marker_open_us <= ev["wall_us"] <= marker_close_us
        and ev["window_focused"]
    ]
    if not candidates:
        return None
    return max(candidates, key=lambda ev: ev["wall_us"])
```

- [ ] **Step 3: Add the B2.1 verdict helper**

Append this helper just before `blink_command_text`:

```python
def _b21_verdict(target_panel_id, decisive_wall_us, events_by_kind):
    """B2.1: read the chain forward from the decisive NOTICE_FC_DECODE
    and name the bin per the 9-bin truth table.

    events_by_kind: dict mapping kind names to lists of events (already
    filtered to post-decisive, target-related rows).

    Returns: dict with "bin", "evidence" (list of strings), "dispatch".
    """
    if not events_by_kind.get("HANDLER_ENTRY"):
        return {"bin": "OA1", "evidence": ["NOTICE_FC_DECODE present, HANDLER_ENTRY absent"],
                "dispatch": "Re-brainstorm B2.2 (handler not invoked: panic / vtable / wrong impl)"}

    handler = events_by_kind["HANDLER_ENTRY"][0]
    if not events_by_kind.get("WUP_RESULT"):
        if handler["branch_taken"]:
            return {"bin": "OA1-PARTIAL", "evidence": ["HANDLER_ENTRY branch_taken=t but WUP_RESULT absent"],
                    "dispatch": "Re-brainstorm B2.2 (body crashed between RestartCursorBlinking and wake_up_panel)"}
        # Branch wasn't taken; that's the bug if iap+wf were both t at NOTICE_FC_DECODE.
        return {"bin": "OA1-BRANCH", "evidence": ["HANDLER_ENTRY branch_taken=f despite is_focused_path=" + ("t" if handler["is_focused_path"] else "f")],
                "dispatch": "Re-brainstorm B2.2 (focused branch not taken)"}

    wup = events_by_kind["WUP_RESULT"][0]
    if not wup["panel_found"]:
        return {"bin": "OB1", "evidence": ["WUP_RESULT panel_found=f"],
                "dispatch": "Re-brainstorm B2.2 (panel disappeared mid-handler; impossible by code; 🚨)"}
    if "Some(" not in wup["engine_id"]:
        return {"bin": "OB2", "evidence": ["WUP_RESULT engine_id=None"],
                "dispatch": "Phase 1 fix: engine binding lifecycle (see spec Phase 1 OB2)"}
    if not wup["scheduler_some"]:
        return {"bin": "OB3", "evidence": ["WUP_RESULT scheduler_some=f"],
                "dispatch": "Phase 1 fix: PanelCtx scheduler propagation (see spec Phase 1 OB3)"}
    # wake_dispatched should be t at this point.

    if not events_by_kind.get("CYCLE_ENTRY"):
        # OC-NOPICKUP — refine via WAKE.engine_type from the wake events list.
        wakes = events_by_kind.get("WAKE", [])
        engine_type = wakes[0].get("engine_type", "") if wakes else "<no WAKE>"
        if "<unregistered>" in engine_type:
            return {"bin": "OC-NOPICKUP-STALE", "evidence": ["WAKE engine_type=<unregistered>; CYCLE_ENTRY absent"],
                    "dispatch": "Phase 1 fix: stale engine_id binding; clear on deregister"}
        if "PanelCycleEngine" not in engine_type:
            return {"bin": "OC-NOPICKUP-WRONGTYPE", "evidence": [f"WAKE engine_type={engine_type}; CYCLE_ENTRY absent"],
                    "dispatch": "Phase 1 fix: wrong engine type bound to panel"}
        return {"bin": "OC-NOPICKUP-DOTIMESLICE", "evidence": ["WAKE engine_type=PanelCycleEngine; CYCLE_ENTRY absent"],
                "dispatch": "Re-brainstorm B2.2 (DoTimeSlice scheduler internals; out of budget)"}

    if not events_by_kind.get("BLINK_CYCLE"):
        return {"bin": "OC-DISPATCH", "evidence": ["CYCLE_ENTRY present, BLINK_CYCLE absent"],
                "dispatch": "Phase 1 fix: PanelCycleEngine routing (see spec Phase 1 OC-DISPATCH)"}

    blink_cycles = events_by_kind["BLINK_CYCLE"]
    if not any(bc.get("flipped", False) for bc in blink_cycles):
        return {"bin": "OD2", "evidence": [f"BLINK_CYCLE present ({len(blink_cycles)}), flipped=t never observed"],
                "dispatch": "Phase 1 fix: cycle_blink timer logic (see spec Phase 1 OD2)"}

    # BLINK_CYCLE flipped=t exists; check INVAL_DRAIN.
    inval_drains = events_by_kind.get("INVAL_DRAIN", [])
    target_drains = [d for d in inval_drains if d.get("panel_id") == target_panel_id]
    if target_drains and not any(d.get("drained", False) for d in target_drains):
        return {"bin": "OD3", "evidence": [f"INVAL_DRAIN drained=f ({len(target_drains)}) for click target post-flip"],
                "dispatch": "Phase 1 fix or escalate: paint pipeline (see spec Phase 1 OD3)"}

    return {"bin": "OD-OK", "evidence": ["full chain present, no visible blink — non-structural"],
            "dispatch": "Re-brainstorm B2.2 (paint content / capture procedure / perception)"}
```

- [ ] **Step 4: Wire into `blink_command_text`**

In `blink_command_text`, locate the existing line-loop (where B2's `notice_fc_events_for_target`, `set_active_events`, `set_focused_events` accumulators live). Add three new accumulators:

```python
    handler_entry_events = []
    wup_result_events = []
    cycle_entry_events = []
```

Inside the line dispatch (the if/elif chain that calls `parse_X` per prefix), add:

```python
            elif ln.startswith("HANDLER_ENTRY|"):
                ev = parse_handler_entry(ln)
                if ev:
                    handler_entry_events.append(ev)
            elif ln.startswith("WUP_RESULT|"):
                ev = parse_wup_result(ln)
                if ev:
                    wup_result_events.append(ev)
            elif ln.startswith("CYCLE_ENTRY|"):
                ev = parse_cycle_entry(ln)
                if ev:
                    cycle_entry_events.append(ev)
```

After the loop, replace (or extend) the B2 verdict-emission block with B2.1 verdict logic. Find the existing `## Phase 0 verdict:` emission and add a new section after it:

```python
    # B2.1 click-target picking + verdict.
    click_target_ev = _pick_click_target(set_active_events, marker_open_us, marker_close_us)
    if click_target_ev is None:
        out_lines.append("")
        out_lines.append("## B2.1 verdict: SKIPPED (no SET_ACTIVE_RESULT|window_focused=t between markers)")
    else:
        target_panel_id = click_target_ev["target_panel_id"]
        # Decisive event = first NOTICE_FC_DECODE for target with iap=t && wf=t after the SET_ACTIVE_RESULT.
        decisive = None
        for ev in notice_fc_events_for_target if False else []:  # collected differently per existing code
            pass
        # Pull all NOTICE_FC_DECODE events for the target panel (re-scan if needed).
        notice_fc_for_target = [
            ev for ev in (handler_entry_events + [])  # placeholder; actual NOTICE_FC_DECODE is parsed earlier
        ]
        # Simpler: re-extract from full notice_fc list. The existing B2 code holds a list,
        # named likely `notice_fc_events_for_target`. Use that:
        decisive_candidates = [
            ev for ev in (notice_fc_events_for_target if 'notice_fc_events_for_target' in dir() else [])
            if ev.get("panel_id") == target_panel_id
            and ev.get("in_active_path")
            and ev.get("window_focused")
            and ev.get("wall_us", 0) >= click_target_ev["wall_us"]
        ]
        decisive = decisive_candidates[0] if decisive_candidates else None

        if decisive is None:
            out_lines.append("")
            out_lines.append(f"## B2.1 verdict: O4-RETARGETED (no NOTICE_FC_DECODE for click target {target_panel_id} with iap=t,wf=t)")
            out_lines.append("→ Re-brainstorm B2.2 (notice did not reach the click target)")
        else:
            # Bucket post-decisive events by kind, filtering to target panel where applicable.
            window_us = decisive["wall_us"], decisive["wall_us"] + 5_000_000  # 5-second window
            def _post(events, panel_filter=True):
                return [
                    ev for ev in events
                    if ev.get("wall_us", 0) >= decisive["wall_us"]
                    and ev.get("wall_us", 0) <= decisive["wall_us"] + 5_000_000
                    and ((not panel_filter) or ev.get("panel_id") == target_panel_id)
                ]
            events_by_kind = {
                "HANDLER_ENTRY": _post(handler_entry_events),
                "WUP_RESULT":    _post(wup_result_events),
                "WAKE":          _post(wake_events, panel_filter=False),  # WAKE is engine-keyed, filter in helper
                "CYCLE_ENTRY":   _post(cycle_entry_events),
                "BLINK_CYCLE":   _post(blink_cycle_events) if 'blink_cycle_events' in dir() else [],
                "INVAL_DRAIN":   _post(inval_drain_events, panel_filter=False) if 'inval_drain_events' in dir() else [],
            }
            verdict = _b21_verdict(target_panel_id, decisive["wall_us"], events_by_kind)
            out_lines.append("")
            out_lines.append(f"## B2.1 verdict: {verdict['bin']}")
            for line in verdict["evidence"]:
                out_lines.append(f"  evidence: {line}")
            out_lines.append("")
            out_lines.append(f"## B2.1 dispatch: {verdict['dispatch']}")
```

NOTE TO IMPLEMENTER: The exact wiring depends on the variable names used in B2's `blink_command_text`. The implementer must:

1. Read the current `blink_command_text` body in full.
2. Identify the variable names B2 used for:
   - `marker_open_us`, `marker_close_us` (or similar — the wall_us of the open/close markers).
   - The list of NOTICE_FC_DECODE events (likely `notice_fc_events_for_target` per B2 Task 7).
   - The list of WAKE events.
   - The list of BLINK_CYCLE events (these are NOT covered by B2; may need to add an accumulator + parser).
   - The list of INVAL_DRAIN events (likely also new accumulator).
3. Adapt the variable references in the snippet above to match.

If `parse_blink_cycle` and `parse_inval_drain` don't already exist in the file, add minimal versions:

```python
def parse_blink_cycle(line):
    try:
        f = _parse_kv_line(line, "BLINK_CYCLE")
    except ValueError:
        return None
    return {
        "kind": "BLINK_CYCLE",
        "wall_us": int(f["wall_us"]),
        "engine_id": f["engine_id"],
        "panel_id": f["panel_id"],
        "focused": f["focused"] == "t",
        "flipped": f["flipped"] == "t",
        "busy": f["busy"] == "t",
    }
```

(The existing `parse_inval_drain` should already cover INVAL_DRAIN; verify.)

- [ ] **Step 5: Sanity-check by running blink against the existing B2 capture log if available**

```bash
ls -la /tmp/em_instr.blink-trace.log 2>/dev/null
# if present:
python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-trace.log 2>&1 | tail -10
```

Expected: the existing capture lacks the new line types (HANDLER_ENTRY/WUP_RESULT/CYCLE_ENTRY), so verdict should be `OA1` (HANDLER_ENTRY absent). That's the expected pre-recapture behavior — confirms the analyzer logic runs without crashing.

- [ ] **Step 6: Commit**

```bash
git add scripts/analyze_hang.py
git commit -m "$(cat <<'EOF'
analyzer: phase 0 (B2.1) click-target heuristic + 9-bin verdict

Replaces the B2 path-trace transition heuristic for picking the click
target with: latest SET_ACTIVE_RESULT|window_focused=t between markers
(addresses the B2 Phase 0 mis-targeting on PanelId(19v5)).

Adds _b21_verdict that reads the post-decisive chain
HANDLER_ENTRY → WUP_RESULT → WAKE → CYCLE_ENTRY → BLINK_CYCLE →
INVAL_DRAIN and emits one of the 9 bins (OA1/OB1/OB2/OB3/OC-NOPICKUP-*/
OC-DISPATCH/OD2/OD3/OD-OK) per the spec truth table, with a per-bin
dispatch instruction.
EOF
)"
```

---

## Task 9: Add unit tests for B2.1 analyzer changes

**Files:**
- Modify: `scripts/test_analyze_hang.py` (add test cases for new parsers + each verdict bin).

- [ ] **Step 1: Append parser tests**

Append these tests:

```python
from analyze_hang import (
    parse_handler_entry, parse_wup_result, parse_cycle_entry,
    _pick_click_target, _b21_verdict,
)


def test_parse_handler_entry_full():
    line = ("HANDLER_ENTRY|wall_us=100|panel_id=PanelId(125v1)|"
            "impl=emTestPanel::TextFieldPanel|flags=0xf0|"
            "is_focused_path=t|branch_taken=t")
    ev = parse_handler_entry(line)
    assert ev["wall_us"] == 100
    assert ev["panel_id"] == "PanelId(125v1)"
    assert ev["impl"] == "emTestPanel::TextFieldPanel"
    assert ev["flags"] == 0xf0
    assert ev["is_focused_path"] is True
    assert ev["branch_taken"] is True


def test_parse_wup_result_engine_id_none():
    line = ("WUP_RESULT|wall_us=200|panel_id=PanelId(125v1)|"
            "caller=crates/emtest/src/emTestPanel.rs:255|panel_found=t|"
            "engine_id=None|scheduler_some=t|wake_dispatched=f")
    ev = parse_wup_result(line)
    assert ev["panel_found"] is True
    assert ev["engine_id"] == "None"
    assert ev["scheduler_some"] is True
    assert ev["wake_dispatched"] is False


def test_parse_cycle_entry():
    line = ("CYCLE_ENTRY|wall_us=300|engine_id=EngineId(7v1)|"
            "panel_id=PanelId(125v1)|behavior_type=emTestPanel::TextFieldPanel")
    ev = parse_cycle_entry(line)
    assert ev["engine_id"] == "EngineId(7v1)"
    assert ev["behavior_type"] == "emTestPanel::TextFieldPanel"
```

- [ ] **Step 2: Append click-target picker tests**

```python
def test_pick_click_target_picks_latest_post_marker():
    events = [
        {"target_panel_id": "P_A", "window_focused": True, "wall_us": 1000},
        {"target_panel_id": "P_B", "window_focused": True, "wall_us": 2000},
        {"target_panel_id": "P_C", "window_focused": False, "wall_us": 2500},  # not focused
    ]
    ev = _pick_click_target(events, 500, 3000)
    assert ev["target_panel_id"] == "P_B"


def test_pick_click_target_none_when_outside_markers():
    events = [
        {"target_panel_id": "P_A", "window_focused": True, "wall_us": 100},
    ]
    assert _pick_click_target(events, 500, 3000) is None
```

- [ ] **Step 3: Append verdict-bin tests (one per bin)**

```python
def _decisive(wall_us=1000):
    return {"wall_us": wall_us, "panel_id": "P", "in_active_path": True, "window_focused": True, "flags": 0xf0}


def test_b21_verdict_oa1_no_handler_entry():
    v = _b21_verdict("P", 1000, {})
    assert v["bin"] == "OA1"


def test_b21_verdict_ob1_panel_not_found():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": False, "engine_id": "None", "scheduler_some": True, "wake_dispatched": False}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OB1"


def test_b21_verdict_ob2_engine_id_none():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "None", "scheduler_some": True, "wake_dispatched": False}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OB2"


def test_b21_verdict_ob3_scheduler_none():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "Some(EngineId(7v1))", "scheduler_some": False, "wake_dispatched": False}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OB3"


def test_b21_verdict_oc_nopickup_stale():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "Some(EngineId(7v1))", "scheduler_some": True, "wake_dispatched": True}],
        "WAKE": [{"engine_type": "<unregistered>"}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OC-NOPICKUP-STALE"


def test_b21_verdict_oc_dispatch():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "Some(EngineId(7v1))", "scheduler_some": True, "wake_dispatched": True}],
        "WAKE": [{"engine_type": "emcore::emPanelCycleEngine::PanelCycleEngine"}],
        "CYCLE_ENTRY": [{}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OC-DISPATCH"


def test_b21_verdict_od2_no_flip():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "Some(EngineId(7v1))", "scheduler_some": True, "wake_dispatched": True}],
        "WAKE": [{"engine_type": "PanelCycleEngine"}],
        "CYCLE_ENTRY": [{}],
        "BLINK_CYCLE": [{"flipped": False}, {"flipped": False}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OD2"


def test_b21_verdict_od3_drain_false():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "Some(EngineId(7v1))", "scheduler_some": True, "wake_dispatched": True}],
        "WAKE": [{"engine_type": "PanelCycleEngine"}],
        "CYCLE_ENTRY": [{}],
        "BLINK_CYCLE": [{"flipped": True}],
        "INVAL_DRAIN": [{"panel_id": "P", "drained": False}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OD3"


def test_b21_verdict_od_ok():
    eb = {
        "HANDLER_ENTRY": [{"branch_taken": True, "is_focused_path": True}],
        "WUP_RESULT": [{"panel_found": True, "engine_id": "Some(EngineId(7v1))", "scheduler_some": True, "wake_dispatched": True}],
        "WAKE": [{"engine_type": "PanelCycleEngine"}],
        "CYCLE_ENTRY": [{}],
        "BLINK_CYCLE": [{"flipped": True}],
        "INVAL_DRAIN": [{"panel_id": "P", "drained": True}],
    }
    v = _b21_verdict("P", 1000, eb)
    assert v["bin"] == "OD-OK"
```

- [ ] **Step 4: Run the tests**

```bash
python3 scripts/test_analyze_hang.py
```

Expected: all PASS, exit 0. If any fail, fix the verdict logic — don't loosen the assertion. The 9-bin truth table is the spec; tests are the executable contract.

- [ ] **Step 5: Commit**

```bash
git add scripts/test_analyze_hang.py
git commit -m "$(cat <<'EOF'
analyzer: phase 0 (B2.1) unit tests for parsers + 9-bin verdict logic

Covers each row of the B2.1 truth table (OA1/OB1/OB2/OB3/
OC-NOPICKUP-STALE/OC-DISPATCH/OD2/OD3/OD-OK) plus the new parsers and
the click-target picker, so verdict miscompute can be caught at test
time.
EOF
)"
```

---

## Task 10: Manual GUI capture for Phase 0

**Files:** none (capture only). Produces `/tmp/em_instr.blink-cycle.log`.

**Note for the implementer:** This task requires the user's hands at the keyboard. Cannot be fully automated. The orchestrator handles this directly; do NOT dispatch a subagent.

- [ ] **Step 1: Build release**

```bash
cargo build -p eaglemode --release
```

Expected: PASS. Build the eaglemode binary fresh on the instr branch so the new instrumentation is compiled in.

- [ ] **Step 2: Clear stale capture log**

```bash
rm -f /tmp/em_instr.blink-cycle.log
```

- [ ] **Step 3: Launch the GUI with structured logging via cargo run**

In a background-friendly form (the orchestrator should use `run_in_background=true`):

```bash
EM_INSTR_FD=9 cargo run -p eaglemode --release 9>/tmp/em_instr.blink-cycle.log
```

**IMPORTANT:** Use `cargo run` (not the bare binary) so plugins load — per memory `project_isactive_bug.md`, the bare binary doesn't trigger the plugin cdylib build/load chain.

Wait ~5-10 seconds for the GUI to fully open and plugins to register.

- [ ] **Step 4: Verify the GUI process is running**

```bash
EMPID=$(pgrep -f "target/release/eaglemode" | head -1)
echo "EMPID=$EMPID"
ls -la /tmp/em_instr.blink-cycle.log
```

Expected: a single PID, log file growing (>10 KB after a few seconds).

- [ ] **Step 5: Send "open" SIGUSR1 marker**

```bash
kill -USR1 $EMPID
sleep 0.5
grep -c "MARKER" /tmp/em_instr.blink-cycle.log
```

Expected: count = 1.

- [ ] **Step 6: Ask the user to interact**

ASK THE USER: "Please click into one of the test-panel TextFields (e.g., the 'tf1' or 'tf2' fields). Hold focus there for ~30 seconds without clicking elsewhere. Reply 'done' when 30 seconds have passed."

WAIT FOR USER REPLY.

- [ ] **Step 7: Send "close" SIGUSR1 marker**

```bash
kill -USR1 $EMPID
sleep 1
grep -c "MARKER" /tmp/em_instr.blink-cycle.log
```

Expected: count = 2.

- [ ] **Step 8: Ask the user to close the GUI**

ASK THE USER: "Please close the GUI window now (Alt+F4 or click the close button). Reply 'closed' when the process has exited."

WAIT FOR USER REPLY.

- [ ] **Step 9: Verify capture integrity**

```bash
pgrep -f "target/release/eaglemode" || echo "GUI exited cleanly"
ls -la /tmp/em_instr.blink-cycle.log
echo "--- Line type counts ---"
for k in MARKER HANDLER_ENTRY WUP_RESULT CYCLE_ENTRY NOTICE_FC_DECODE SET_ACTIVE_RESULT WAKE BLINK_CYCLE INVAL_DRAIN; do
  printf "%-20s %s\n" "$k" "$(grep -c "^$k|" /tmp/em_instr.blink-cycle.log)"
done
```

Expected: MARKER=2; HANDLER_ENTRY > 0 (at least one TextField focused = at least one HANDLER_ENTRY); WUP_RESULT > 0 (every wake_up_panel call logs once); CYCLE_ENTRY may or may not be > 0 (depends on whether the bug allowed any Cycle to run); NOTICE_FC_DECODE / SET_ACTIVE_RESULT / WAKE all > 0.

If MARKER ≠ 2: SIGUSR1 sent to wrong PID; restart from Step 2.
If HANDLER_ENTRY = 0: capture procedure was wrong (user didn't click into a TextField, or the FC notice didn't reach the handler at all); restart from Step 2.

---

## Task 11: Run the analyzer; record verdict + dispatch on main

**Files:**
- Create on main: `docs/scratch/2026-05-03-blink-cycle-results.md`

- [ ] **Step 1: Run the analyzer's blink command**

```bash
python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-cycle.log > /tmp/blink-cycle-report.txt 2>&1
echo "exit=$?"
tail -30 /tmp/blink-cycle-report.txt
```

Expected: a report ending with both a B2 `## Phase 0 verdict: OX` (legacy from B2 instrumentation) AND a `## B2.1 verdict: BIN` line + dispatch instruction.

- [ ] **Step 2: Note the B2.1 verdict**

```bash
grep "^## B2.1 verdict:\|^## B2.1 dispatch:" /tmp/blink-cycle-report.txt
```

Expected: two lines: `## B2.1 verdict: BIN` and `## B2.1 dispatch: ...`. Record both.

- [ ] **Step 3: Switch to main and write findings**

```bash
git checkout main
git pull origin main
```

If there are uncommitted instr-branch edits in the working tree (shouldn't be — Task 10 produced no edits), stash before switching:

```bash
git status --short
# only if dirty:
git stash --include-untracked
```

Create `docs/scratch/2026-05-03-blink-cycle-results.md` with this content (substitute `<...>` placeholders from the analyzer report):

```markdown
# Blink-cycle chain Phase 0 results — 2026-05-03 (B2.1)

Capture: `/tmp/em_instr.blink-cycle.log`
Branch: `instr/blink-cycle-2026-05-03` @ `<commit-sha>`
Run: <YYYY-MM-DD HH:MM>

## Click target

- `target_panel_id`: <copy from analyzer report>
- `wall_us` of SET_ACTIVE_RESULT: <copy>

## Decisive NOTICE_FC_DECODE event

- `wall_us`: <copy>
- `panel_id`: <copy>
- `in_active_path`: <t|f>
- `window_focused`: <t|f>
- `flags`: <0x...>

## Chain trace post-decisive

- HANDLER_ENTRY: <present|absent> (if present: `branch_taken=<t|f>`, `is_focused_path=<t|f>`)
- WUP_RESULT: <present|absent> (if present: `panel_found=<t|f>`, `engine_id=<Some|None>`, `scheduler_some=<t|f>`, `wake_dispatched=<t|f>`, `caller=<file:line>`)
- WAKE (cursor-blink, caller from emTestPanel.rs:255 or emColorFieldFieldPanel.rs:158): <present|absent> (if present: `engine_type=<...>`)
- CYCLE_ENTRY: <present|absent> (if present: `behavior_type=<...>`)
- BLINK_CYCLE: <present|absent> (count: <N>; `flipped=t` ever observed: <yes|no>)
- INVAL_DRAIN for click target: <present|absent> (if present: `drained=<t|f>` count: <X t / Y f>)

## Verdict

**<BIN>**

## Dispatch

<copy from analyzer report>

## Prediction calibration (advisor's check)

- Pre-measurement priors: 50% OB2, 25% OB3, 10% OC-NOPICKUP, 5% OC-DISPATCH, 5% OD2, 3% OD3, 2% other.
- Actual: <BIN>
- Retrospective: <one-line note: was the prior well-calibrated? what does the actual verdict imply about the next investigation?>

## Full analyzer report

<paste contents of /tmp/blink-cycle-report.txt below this line>
```

- [ ] **Step 4: Commit findings on main**

```bash
git add docs/scratch/2026-05-03-blink-cycle-results.md
git commit -m "$(cat <<'EOF'
scratch: B2.1 Phase 0 blink-cycle results

Phase 0 capture verdict: <BIN>. Records prediction-vs-actual for
calibration and the analyzer's full report. Dispatches to <Phase 1
fix path | B2.2 re-brainstorm> per the spec truth table.
EOF
)"
git push origin main
```

- [ ] **Step 5: Restore instr branch state**

```bash
git checkout instr/blink-cycle-2026-05-03
git stash pop || true   # may be empty; that's fine
```

---

## Task 12: Tag instr branch; finalize Phase 0; dispatch decision

**Files:** Modify `docs/scratch/2026-05-03-blink-cycle-results.md` on main (append dispatch decision).

- [ ] **Step 1: Tag the instr branch tip**

```bash
git checkout instr/blink-cycle-2026-05-03
git tag instr-blink-cycle-2026-05-03
git push origin instr/blink-cycle-2026-05-03 instr-blink-cycle-2026-05-03
```

- [ ] **Step 2: Decision point — dispatch path based on verdict**

Read the verdict from the findings doc:

```bash
git checkout main
grep "^\*\*<\|^\*\*O" docs/scratch/2026-05-03-blink-cycle-results.md | head -1
```

Based on the verdict, dispatch:

| Verdict | Next task | Notes |
|---|---|---|
| **OB2** | Task 13 | Phase 1 fix; engine binding lifecycle |
| **OB3** | Task 14 | Phase 1 fix; PanelCtx scheduler |
| **OC-DISPATCH** | Task 15 | Phase 1 fix; PanelCycleEngine routing |
| **OD2** | Task 16 | Phase 1 fix; cycle_blink timer |
| **OC-NOPICKUP-STALE** or **OC-NOPICKUP-WRONGTYPE** | Task 17 | Phase 1 fix; engine binding |
| **OD3** | Task 18 | Phase 1 fix or escalate; paint pipeline |
| **OA1**, **OB1**, **OD-OK**, **OC-NOPICKUP-DOTIMESLICE**, **OD3-deep** | Task 22 | Re-brainstorm B2.2 |

- [ ] **Step 3: Document the dispatch decision**

Append to `docs/scratch/2026-05-03-blink-cycle-results.md`:

```markdown
**Next phase dispatched:** <Task NN — Phase 1 <fix-name> | Task 22 — B2.2 re-brainstorm>
```

```bash
git add docs/scratch/2026-05-03-blink-cycle-results.md
git commit -m "scratch: B2.1 Phase 0 dispatch decision recorded — <next phase>"
git push origin main
```

---

## Task 13: Phase 1 fix for OB2 — engine binding lifecycle

**ONLY EXECUTE IF Task 12 verdict was OB2.**

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs` (and possibly `emEngineCtx.rs` or `emView.rs` depending on chosen site C1/C2/C3)
- Create: `crates/emcore/tests/blink_focus_path.rs` (regression test added in Task 19)

- [ ] **Step 1: Cut fix branch from main**

```bash
git checkout main
git pull origin main
git checkout -b fix/blink-cycle-chain-2026-05-03
```

- [ ] **Step 2: Run the diagnostic procedure (spec § Phase 1 OB2)**

Read the WUP_RESULT entry from the capture:

```bash
WALL=<from blink-cycle-results.md>
TARGET=<panel_id from blink-cycle-results.md>
grep "WUP_RESULT" /tmp/em_instr.blink-cycle.log | grep "panel_id=$TARGET" | head -3
```

Confirm `engine_id=None`. Note the `caller` field — confirms the call originates from `emTestPanel.rs:255` or `emColorFieldFieldPanel.rs:158`.

Read C++ ground truth:

```bash
sed -n '343,355p' ~/Projects/eaglemode-0.96.4/src/emCore/emTextField.cpp
sed -n '1315,1325p' ~/Projects/eaglemode-0.96.4/src/emCore/emTextField.cpp
```

Read the Rust `init_panel_view` and its callers:

```bash
grep -n "init_panel_view\|fn init_panel_view" crates/emcore/src/emPanelTree.rs | head -10
sed -n '670,710p' crates/emcore/src/emPanelTree.rs
```

Trace which path created the click target panel and identify why `init_panel_view` was deferred without a re-call.

- [ ] **Step 3: Pick the candidate site (C1/C2/C3/C4) and write the diagnosis**

Per spec § Phase 1 OB2 Candidate fix sites table, pick:

- **C1** if a specific missing-call site is identifiable (preferred — surgical).
- **C2** if multiple paths can leave panels unbound (general safety net at activation).
- **C3** as last resort (lazy-bind in wake_up_panel).
- **C4** is per-widget; only as final fallback.

Append to `docs/scratch/2026-05-03-blink-cycle-results.md`:

```markdown
## Diagnosis (Phase 1 OB2)

<one-paragraph explanation of: which path created the panel; why init_panel_view was deferred; whether a re-call site exists; chosen candidate (C1/C2/C3/C4) with reasoning>
```

```bash
git checkout main
git add docs/scratch/2026-05-03-blink-cycle-results.md
git commit -m "scratch: B2.1 Phase 1 OB2 diagnosis — <chosen candidate>"
git push origin main
git checkout fix/blink-cycle-chain-2026-05-03
git merge main  # bring the diagnosis into the fix branch
```

- [ ] **Step 4: Apply the fix (≤30 LOC)**

The exact change is data-dependent. Likely shapes per the chosen candidate:

- **C1** (find missing re-call): add a single `init_panel_view(id, Some(sched))` call at the identified site (a few LOC).
- **C2** (eager re-call from set_active_panel): add an idempotency-guarded `init_panel_view` invocation in `set_active_panel`, gated on `panel.engine_id.is_none()`.
- **C3** (lazy in wake_up_panel): add a self-healing branch in `wake_up_panel` when `engine_id.is_none()`.
- **C4** (per-widget): add the call in `TextFieldPanel::notice` before `wake_up_panel`.

Hard constraint: ≤30 LOC across at most 2 files. If diagnosis requires more, STOP and escalate to re-brainstorm B2.2 (per spec).

- [ ] **Step 5: Verify build is clean**

```bash
cargo check
cargo clippy -- -D warnings
```

Expected: PASS. No `#[allow]` for warnings.

- [ ] **Step 6: Commit the fix (separate commit from the test)**

```bash
git add <changed files>
git commit -m "$(cat <<'EOF'
fix(blink-cycle): bind cursor-blink engine before notice dispatch

Phase 0 capture (docs/scratch/2026-05-03-blink-cycle-results.md)
identified that PanelId(<...>) reached FOCUS_CHANGED notice with
panel.engine_id=None. The init_panel_view call site at <path:line>
deferred binding when sched=None and the deferred re-call never
fired. Fix: <one-line mechanic per chosen candidate>.

Refs: docs/superpowers/specs/2026-05-03-blink-cycle-chain-design.md
(Phase 1 OB2)
EOF
)"
```

Continue to Task 19 (regression test).

---

## Task 14: Phase 1 fix for OB3 — PanelCtx scheduler propagation

**ONLY EXECUTE IF Task 12 verdict was OB3.**

**Files:**
- Modify: `crates/emcore/src/emView.rs` (`handle_notice_one` and PanelCtx construction site at line ~4198)

- [ ] **Step 1: Cut fix branch**

```bash
git checkout main
git pull origin main
git checkout -b fix/blink-cycle-chain-2026-05-03
```

- [ ] **Step 2: Diagnostic procedure (spec § Phase 1 OB3)**

Confirm WUP_RESULT shows `scheduler_some=f`, and the WAKE log absence pattern matches:

```bash
grep "WUP_RESULT" /tmp/em_instr.blink-cycle.log | grep "scheduler_some=f" | head -3
```

Trace the notice-dispatch PanelCtx construction:

```bash
grep -n "with_sched_reach_optional_roots\|PanelCtx::with_sched" crates/emcore/src/emView.rs crates/emcore/src/emEngineCtx.rs | head -10
sed -n '4180,4215p' crates/emcore/src/emView.rs
```

Identify why `_optional_roots` was passed `None` for sched. Read C++ `emPanel::HandleNotice` for the dispatch shape:

```bash
grep -n "HandleNotice" ~/Projects/eaglemode-0.96.4/src/emCore/emPanel.cpp | head
```

- [ ] **Step 3: Write diagnosis paragraph**

Append `## Diagnosis (Phase 1 OB3)` to the findings doc, commit on main, merge into fix branch (same pattern as Task 13 Step 3).

- [ ] **Step 4: Apply the fix**

Replace the `with_sched_reach_optional_roots` call with one that accepts a non-optional scheduler, threading it from the caller. If the caller chain doesn't have a scheduler in scope at this point, trace upstream to find where it was dropped, and restore propagation.

Hard constraint: ≤15 LOC.

- [ ] **Step 5: Verify build**

```bash
cargo check && cargo clippy -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add <changed files>
git commit -m "$(cat <<'EOF'
fix(blink-cycle): plumb scheduler into notice dispatch PanelCtx

Phase 0 (docs/scratch/2026-05-03-blink-cycle-results.md) showed
WUP_RESULT.scheduler_some=f at the cursor-blink wake_up_panel call
site, meaning handle_notice_one's PanelCtx was built without a
scheduler. Fix: <one-line mechanic — replaced with_sched_reach_optional_roots
with the non-optional variant; threaded scheduler from caller>.

Refs: docs/superpowers/specs/2026-05-03-blink-cycle-chain-design.md
(Phase 1 OB3)
EOF
)"
```

Continue to Task 19.

---

## Task 15: Phase 1 fix for OC-DISPATCH — PanelCycleEngine routing

**ONLY EXECUTE IF Task 12 verdict was OC-DISPATCH.**

**Files:**
- Modify: `crates/emcore/src/emPanelCycleEngine.rs` (the panel/behavior lookup at lines ~110-122 or ~225-237)

- [ ] **Step 1: Cut fix branch**

```bash
git checkout main && git pull origin main
git checkout -b fix/blink-cycle-chain-2026-05-03
```

- [ ] **Step 2: Diagnostic procedure**

Read CYCLE_ENTRY entries and check `behavior_type`:

```bash
grep "CYCLE_ENTRY" /tmp/em_instr.blink-cycle.log | head -10
```

If `behavior_type` doesn't match the click target, the routing inside PanelCycleEngine resolved the wrong panel.

Read the PanelCycleEngine.Cycle implementation around both behavior.Cycle sites (lines ~118-130 and ~225-240) and identify the panel/behavior lookup logic.

- [ ] **Step 3: Diagnosis paragraph**

Append to findings doc, commit on main, merge into fix branch.

- [ ] **Step 4: Apply the fix (≤15 LOC)**

Fix the panel/behavior lookup so it resolves to the right target. Likely a `panel_id` vs `engine_id` mix-up, or a scope-mismatch.

- [ ] **Step 5: Verify build**

```bash
cargo check && cargo clippy -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emPanelCycleEngine.rs
git commit -m "fix(blink-cycle): PanelCycleEngine routes to the right behavior — <mechanic>"
```

Continue to Task 19.

---

## Task 16: Phase 1 fix for OD2 — cycle_blink timer logic

**ONLY EXECUTE IF Task 12 verdict was OD2.**

**Files:**
- Modify: `crates/emcore/src/emTextFieldWidget.rs` (the `cycle_blink` method, around line 2360-2390)

- [ ] **Step 1: Cut fix branch**

```bash
git checkout main && git pull origin main
git checkout -b fix/blink-cycle-chain-2026-05-03
```

- [ ] **Step 2: Read C++ ground truth**

```bash
sed -n '306,340p' ~/Projects/eaglemode-0.96.4/src/emCore/emTextField.cpp
```

C++ uses `emUInt64 clk = emGetClockMS()` and compares `clk >= CursorBlinkTime + 1000` and `clk >= CursorBlinkTime + 500`.

- [ ] **Step 3: Read Rust cycle_blink**

```bash
grep -n "fn cycle_blink\|cursor_blink_time\|cursor_blink_on" crates/emcore/src/emTextFieldWidget.rs | head -10
sed -n '2350,2400p' crates/emcore/src/emTextFieldWidget.rs
```

- [ ] **Step 4: Diagnosis paragraph + fix**

Identify the divergence (clock source, threshold, conditional structure). Align the Rust port with C++. Fix shape ≤15 LOC.

Append diagnosis to findings doc, commit on main, merge into fix branch.

- [ ] **Step 5: Verify build**

```bash
cargo check && cargo clippy -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emTextFieldWidget.rs
git commit -m "fix(blink-cycle): align cycle_blink timer with C++ — <mechanic>"
```

Continue to Task 19.

---

## Task 17: Phase 1 fix for OC-NOPICKUP (STALE / WRONGTYPE)

**ONLY EXECUTE IF Task 12 verdict was OC-NOPICKUP-STALE or OC-NOPICKUP-WRONGTYPE.**

**Files (per sub-bin):**
- OC-NOPICKUP-STALE: `crates/emcore/src/emPanelTree.rs` (engine deregistration path)
- OC-NOPICKUP-WRONGTYPE: `crates/emcore/src/emPanelCycleEngine.rs` or `emPanelTree.rs` (engine type at registration)

- [ ] **Step 1: Cut fix branch**

```bash
git checkout main && git pull origin main
git checkout -b fix/blink-cycle-chain-2026-05-03
```

- [ ] **Step 2: Diagnostic procedure**

Confirm WAKE entry's `engine_type`:

```bash
grep "WAKE" /tmp/em_instr.blink-cycle.log | grep "EngineId(<eid from WUP_RESULT>)" | head -3
```

For STALE: search for engine deregistration sites:

```bash
grep -rn "deregister_engine\|remove_engine" crates/emcore/src --include="*.rs" | head -10
```

For WRONGTYPE: search for engine registration sites that may bind a non-PCE engine:

```bash
grep -rn "panels\[.*\]\.engine_id\s*=" crates/emcore/src --include="*.rs"
```

- [ ] **Step 3: Diagnosis + fix**

Apply per sub-bin, ≤30 LOC. Append diagnosis to findings doc, commit on main, merge.

- [ ] **Step 4: Verify build**

```bash
cargo check && cargo clippy -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add <changed files>
git commit -m "fix(blink-cycle): clear stale engine_id on deregister | bind correct engine type — <mechanic>"
```

Continue to Task 19.

---

## Task 18: Phase 1 fix for OD3 — paint pipeline drop

**ONLY EXECUTE IF Task 12 verdict was OD3 AND dirty-tile evidence is clear (per spec dispatch table). If evidence is unclear, escalate to Task 22.**

**Files:**
- Modify: `crates/emcore/src/emPanelCycleEngine.rs` (drain logic) and/or `crates/emcore/src/emWindow.rs` (paint pipeline dirty-tile tracking)

- [ ] **Step 1: Cut fix branch**

```bash
git checkout main && git pull origin main
git checkout -b fix/blink-cycle-chain-2026-05-03
```

- [ ] **Step 2: Diagnostic procedure**

Confirm BLINK_CYCLE has flipped=t but corresponding INVAL_DRAIN has drained=f for the panel:

```bash
grep "BLINK_CYCLE\|INVAL_DRAIN" /tmp/em_instr.blink-cycle.log | grep "PanelId(<target>)" | head -10
```

Trace from PanelCycleEngine drain (`take_invalidate_self_request`) into the window's redraw cycle. If trace points into deep wgpu/winit internals, escalate to Task 22 instead of attempting Phase 1.

- [ ] **Step 3: Diagnosis + fix**

Apply ≤30 LOC. Likely in emPanelCycleEngine.rs (drain logic) or emWindow.rs (dirty-tile tracking).

- [ ] **Step 4: Verify build**

```bash
cargo check && cargo clippy -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add <changed files>
git commit -m "fix(blink-cycle): paint pipeline picks up cursor-blink invalidation — <mechanic>"
```

Continue to Task 19.

---

## Task 19: Add `focused_text_field_engine_wakes_and_cycles` regression test

**ONLY EXECUTE IF any of Tasks 13-18 ran (i.e., a Phase 1 fix landed).**

**Files:**
- Create: `crates/emcore/tests/blink_focus_path.rs`

- [ ] **Step 1: Verify TestViewHarness exists and find its API**

```bash
ls crates/emcore/src/test_view_harness.rs
grep -n "pub fn new\|pub fn sched_ctx\|pub fn run_time_slices\|pub.*tree" crates/emcore/src/test_view_harness.rs | head -10
```

If `run_time_slices` doesn't exist, add it to TestViewHarness as part of this task (small helper that loops `view.DoTimeSlice` for N iterations or until a condition). If it does exist, use it.

- [ ] **Step 2: Verify test-support feature flag if needed**

```bash
grep -n "test-support" crates/emcore/Cargo.toml
```

If `[dev-dependencies]` lacks `emcore = { path = ".", features = ["test-support"] }`, add it (per B2 plan Task 14 Step 2 guidance):

```toml
[dev-dependencies]
emcore = { path = ".", features = ["test-support"] }
```

- [ ] **Step 3: Write the regression test**

Create `crates/emcore/tests/blink_focus_path.rs`:

```rust
//! B2.1 regression test: focused TextFieldPanel produces BLINK_CYCLE
//! entries with at least one `flipped=t` event within budget.
//!
//! Locks out the bug class identified in B2.1 Phase 0 (engine binding,
//! cycle dispatch, paint invalidation between handler and Cycle). The
//! test exercises the *real* TextFieldPanel — not a probe behavior —
//! because the bug class lives below the behavior layer.

use emcore::emColorFieldFieldPanel::TextFieldPanel;
use emcore::emPanel::{NoticeFlags, PanelBehavior};
use emcore::emTextFieldWidget::emTextField;
use emcore::test_view_harness::TestViewHarness;

#[test]
fn focused_text_field_engine_wakes_and_cycles() {
    use emcore::emView::emView;

    let mut h = TestViewHarness::new();

    // Build tree: root → real TextFieldPanel.
    let root = h.tree.create_root_deferred_view("root");
    h.tree.get_mut(root).unwrap().focusable = true;
    h.tree.Layout(root, 0.0, 0.0, 1.0, 1.0, 1.0, None);

    let tf_id = h.tree.create_child(root, "tf", None);
    h.tree.get_mut(tf_id).unwrap().focusable = true;
    h.tree.Layout(tf_id, 0.0, 0.0, 1.0, 1.0, 1.0, None);

    // Construct real TextFieldPanel with a fresh emTextField widget.
    let widget = emTextField::new(/* args per ctor signature */);
    let behavior = TextFieldPanel::new(widget);
    h.tree.set_behavior(tf_id, Box::new(behavior));

    let mut view = emView::new(emcore::emContext::emContext::NewRoot(), root, 640.0, 480.0);
    {
        let mut sc = h.sched_ctx();
        view.Update(&mut h.tree, &mut sc);
    }

    // Window focus + active-panel transition (mimics user click).
    view.SetFocused(&mut h.tree, true);
    {
        let mut sc = h.sched_ctx();
        view.set_active_panel(&mut h.tree, tf_id, false, &mut sc);
    }
    {
        let mut sc = h.sched_ctx();
        view.Update(&mut h.tree, &mut sc);
    }

    // Pump the scheduler enough times for the cursor-blink Cycle to flip
    // (C++ uses 500ms/1000ms thresholds; Rust port uses Instant::now()).
    // Run until at least one flipped=t observed, with budget.
    let captured = h.run_time_slices_with_capture(/* iterations */ 100);

    let blink_cycles: Vec<_> = captured.iter()
        .filter(|line| line.starts_with("BLINK_CYCLE|"))
        .filter(|line| line.contains(&format!("panel_id={:?}", tf_id)))
        .collect();

    assert!(
        !blink_cycles.is_empty(),
        "no BLINK_CYCLE entry for the focused TextField — engine never cycled. \
         Bug class B2.1 (engine binding / cycle dispatch / paint invalidation) \
         is regressing. Captured lines: {:?}",
        captured.iter().take(20).collect::<Vec<_>>()
    );

    let any_flipped = blink_cycles.iter().any(|line| line.contains("flipped=t"));
    assert!(
        any_flipped,
        "BLINK_CYCLE entries appeared but flipped=t never observed within \
         budget — cycle_blink timer logic broken (OD2 regressing). Entries: {:?}",
        blink_cycles
    );
}
```

NOTE TO IMPLEMENTER:

- The exact `emTextField::new` signature must be looked up (`grep -n "impl emTextField" crates/emcore/src/emTextFieldWidget.rs | head`). If it requires args this test can't easily provide (e.g., a parent widget reference), consider exposing a test-helper constructor on emTextField gated by `#[cfg(any(test, feature = "test-support"))]`.

- `run_time_slices_with_capture` is a hypothetical helper. If TestViewHarness lacks a "capture emInstr lines" hook, add one — gate via a thread-local or per-harness buffer in `emInstr.rs` accessible only via `#[cfg(test)]`. Estimated ≤15 LOC of test-support surface.

- `TextFieldPanel::new` may or may not exist publicly. If not, adapt construction or expose a test-only constructor.

If any of these scaffolding requirements exceed ~30 LOC of test-support surface, document the gap in the findings doc as a follow-up and consider scope-reducing the test (e.g., asserting only "BLINK_CYCLE with panel_id=tf_id appears" without the flipped=t check, marking flipped=t as `#[ignore]`).

- [ ] **Step 4: Run the new test**

```bash
cargo test --test blink_focus_path
```

Expected: PASS (because the fix from Tasks 13-18 is in place). If FAIL, the fix is incomplete or the test setup is wrong; investigate.

- [ ] **Step 5: Verify the test would fail without the fix (regression-guard sanity check)**

This is critical. Stash the fix and confirm the test FAILS:

```bash
git stash       # stash the fix
cargo test --test blink_focus_path
```

Expected: FAIL with one of the assertion messages. The failure must indicate the bug class — e.g., "no BLINK_CYCLE entry" for OB2/OB3/OC-NOPICKUP/OC-DISPATCH, or "flipped=t never observed" for OD2.

If it PASSES without the fix: the test isn't exercising the bug class. Investigate before proceeding (the test is worthless as a regression guard). Common causes: tree setup doesn't reproduce the offending ordering; the emInstr capture hook isn't wired correctly.

Restore the fix:

```bash
git stash pop
cargo test --test blink_focus_path
```

Expected: PASS again.

- [ ] **Step 6: Commit the test**

```bash
git add crates/emcore/tests/blink_focus_path.rs crates/emcore/Cargo.toml crates/emcore/src/test_view_harness.rs
git commit -m "$(cat <<'EOF'
test(emcore): B2.1 regression — focused TextField produces BLINK_CYCLE

Asserts (a) at least one BLINK_CYCLE entry for the focused TextField,
(b) at least one with flipped=t within budget. Catches OB2, OB3,
OC-NOPICKUP, OC-DISPATCH, OD2 as regressions. OD3 (paint pipeline
drop) requires a separate pixel-chain test, deferred per spec.

Verified to FAIL when the Phase 1 fix is reverted.

Refs: docs/superpowers/specs/2026-05-03-blink-cycle-chain-design.md
(Regression test design)
EOF
)"
```

---

## Task 20: Manual recapture to verify the fix

**ONLY EXECUTE IF Tasks 13-18 ran AND Task 19 passed.**

**Files:** none (capture only).

**Note for the implementer:** Interactive — orchestrator drives, asks user for input.

- [ ] **Step 1: Cherry-pick the fix onto a recapture branch from instr-blink-cycle-2026-05-03**

```bash
git checkout -b recapture/blink-cycle-fix-2026-05-03 instr-blink-cycle-2026-05-03
git log fix/blink-cycle-chain-2026-05-03 --oneline | head -5  # find fix commit shas
git cherry-pick <fix-commit-sha-1> [<fix-commit-sha-2>]
```

If cherry-pick conflicts, resolve manually — instr-branch edits should be preserved alongside the fix.

```bash
cargo build -p eaglemode --release
```

- [ ] **Step 2: Capture (same procedure as Task 10, new log path)**

```bash
rm -f /tmp/em_instr.blink-cycle-recap.log
EM_INSTR_FD=9 cargo run -p eaglemode --release 9>/tmp/em_instr.blink-cycle-recap.log
```

(Run in background.) Wait ~5-10s for GUI to open.

```bash
EMPID=$(pgrep -f "target/release/eaglemode" | head -1)
kill -USR1 $EMPID  # open marker
```

ASK USER: "Click into a test-panel TextField and hold focus for 30s. Reply 'done' when complete."

```bash
kill -USR1 $EMPID  # close marker
sleep 1
```

ASK USER: "Close the GUI window. Reply 'closed' when exited."

- [ ] **Step 3: Verify BLINK_CYCLE entries appear**

```bash
grep -c "BLINK_CYCLE" /tmp/em_instr.blink-cycle-recap.log
grep "BLINK_CYCLE" /tmp/em_instr.blink-cycle-recap.log | grep "flipped=t" | head -5
```

Expected: BLINK_CYCLE count > 0 (probably tens-to-hundreds for 30s focused). At least some entries with `flipped=t`.

If count is still 0:
- Run analyzer: `python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-cycle-recap.log | tail -20`
- If new verdict differs from Task 12, the fix is wrong/partial — roll back and re-diagnose.
- If new verdict is OD-OK, the chain works in logs but not visually — that's a different investigation, not B2.1's territory.

- [ ] **Step 4: Run the analyzer; confirm verdict is now OD-OK**

```bash
python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-cycle-recap.log | grep "B2.1 verdict:"
```

Expected: `## B2.1 verdict: OD-OK` (full chain works in logs). If not, the fix is incomplete.

- [ ] **Step 5: Update findings doc on main**

```bash
git checkout main
```

Append to `docs/scratch/2026-05-03-blink-cycle-results.md`:

```markdown
## Recapture verification (post-fix)

- Capture: `/tmp/em_instr.blink-cycle-recap.log`
- Recapture branch: `recapture/blink-cycle-fix-2026-05-03`
- BLINK_CYCLE count in window: <N>
- First flipped=t at: +<X>ms after focus
- New B2.1 verdict: OD-OK (chain works post-fix)

Fix is verified.
```

```bash
git add docs/scratch/2026-05-03-blink-cycle-results.md
git commit -m "scratch: B2.1 Phase 1 recapture verifies fix; verdict now OD-OK"
git push origin main
```

- [ ] **Step 6: Clean up the recapture branch**

```bash
git branch -D recapture/blink-cycle-fix-2026-05-03
```

---

## Task 21: Merge fix branch via finishing-a-development-branch

**ONLY EXECUTE IF Task 20 verified the fix.**

**Note for the implementer:** Interactive — invoke the skill, let the user pick merge vs. PR.

- [ ] **Step 1: Verify all tests pass on the fix branch**

```bash
git checkout fix/blink-cycle-chain-2026-05-03
cargo-nextest ntr
```

Expected: PASS.

- [ ] **Step 2: Invoke superpowers:finishing-a-development-branch**

Run the skill. Let the user choose option 1 (merge locally) or option 2 (push and create PR) per their preference. Do not pick autonomously.

- [ ] **Step 3: Verify post-merge state**

```bash
git checkout main
git pull origin main
git log --oneline -5
```

Expected: top of log shows the merge commit (or fix commits if fast-forward).

---

## Task 22: Re-brainstorm handoff for non-fix verdicts

**ONLY EXECUTE IF Task 12 verdict was in {OA1, OB1, OD-OK, OC-NOPICKUP-DOTIMESLICE, or OD3-deep}. Skip if any of Tasks 13-21 ran.**

**Files:**
- Modify on main: `docs/scratch/2026-05-03-blink-cycle-results.md` (append handoff section).

- [ ] **Step 1: Append handoff section to findings doc**

```bash
git checkout main
```

Append:

```markdown
## Handoff to B2.2 brainstorm

Phase 0 verdict was <BIN>, which means <one-line interpretation: e.g., "the
chain breaks at DoTimeSlice scheduler internals (deep) — out of B2.1 budget">.
B2.1 closes here without a fix landed.

### What we learned

- Phase 0 instrumentation distinguished all 9 truth-table bins; the
  capture cleanly identified the failure point.
- Specifically ruled out: <list of bins ruled out by the chain trace>.

### What remains unknown

<bin-specific list:>
- OA1 → handler not invoked: investigate panic / vtable / cdylib trap
  modality.
- OB1 → panel disappeared mid-handler (impossible by code 🚨):
  investigate panel-tree corruption / threading.
- OC-NOPICKUP-DOTIMESLICE → DoTimeSlice scheduler internals: investigate
  wake queue management, parity guards, current_awake_idx scan.
- OD3-deep → paint pipeline depth: investigate dirty-tile tracking
  through wgpu/winit.
- OD-OK → non-structural: visual debugging (DUMP_GOLDEN frame
  capture) and capture-procedure verification.

### Next step

Invoke `superpowers:brainstorming` for B2.2 with this findings doc and
the capture log as input. The candidate space for B2.2 is much smaller
than B2.1's — only one bin remains active.
```

- [ ] **Step 2: Commit handoff**

```bash
git add docs/scratch/2026-05-03-blink-cycle-results.md
git commit -m "$(cat <<'EOF'
scratch: B2.1 closes without fix; verdict <BIN> redirects to B2.2

Phase 0 instrumentation cleanly identified the failure point but it
falls outside the ≤30 LOC fix budget for B2.1 (or is non-structural).
The remaining candidate space is smaller and well-articulated for the
B2.2 brainstorm.
EOF
)"
git push origin main
```

- [ ] **Step 3: Notify the user**

Tell the user:

> Phase 0 verdict was <BIN>. B2.1 closes without a fix. The findings doc
> has been updated with the handoff. Ready to invoke
> `superpowers:brainstorming` for B2.2 when you are.

---

## Self-review (run AFTER plan is complete)

> The agent executing this plan should run this self-check at the end of execution.

**Spec coverage check:**
- ✓ Phase 0 instrumentation (3 lines) — Tasks 2, 3, 4, 5
- ✓ Analyzer extension (parsers, click-target heuristic, verdict) — Tasks 7, 8
- ✓ Analyzer unit tests — Task 9
- ✓ Capture procedure — Task 10
- ✓ Findings doc + dispatch decision — Tasks 11, 12
- ✓ Phase 1 fix paths per bin — Tasks 13 (OB2), 14 (OB3), 15 (OC-DISPATCH), 16 (OD2), 17 (OC-NOPICKUP-STALE/WRONGTYPE), 18 (OD3)
- ✓ Regression test — Task 19
- ✓ Manual recapture verification — Task 20
- ✓ Merge to main — Task 21
- ✓ Re-brainstorm handoff — Task 22
- ✓ Branch lifecycle (instr never merges; fix merges; docs to main) — Tasks 1, 12, 21

**Branch / file consistency check:**
- ✓ Phase 0 edits live on `instr/blink-cycle-2026-05-03`
- ✓ Phase 1 edits live on `fix/blink-cycle-chain-2026-05-03`
- ✓ Documentation commits on `main`
- ✓ Recapture happens on a temporary cherry-pick branch (Task 20) without polluting main or fix branch

**Hard rule check (per CLAUDE.md):**
- ✓ Tasks 13-18 explicitly require reading C++ source before changing Rust
- ✓ No `--no-verify` on git commits
- ✓ No `#[allow(...)]` for warnings — fix at source
- ✓ Phase 0 instrumentation never merges to main
- ✓ Skip per-task `cargo-nextest ntr` in subagent loops (per memory)

---

## Plan complete

The plan is self-contained. To execute:

1. **Subagent-Driven (recommended):** Use `superpowers:subagent-driven-development` to dispatch a fresh subagent per task with two-stage review.
2. **Inline Execution:** Use `superpowers:executing-plans` to run tasks in this session with checkpoints.

**Conditional task structure:** Tasks 13-18 are mutually exclusive (based on Phase 0 verdict). Task 19 only runs if 13-18 ran. Task 20-21 only run if 19 passed. Task 22 runs only if 13-18 didn't. The conditional dispatch is recorded in Task 12 Step 2.

**Critical interactive checkpoints requiring orchestrator (NOT subagent):**
- Task 10: Manual GUI capture (orchestrator + user dance with SIGUSR1 markers).
- Task 12: Verdict-dependent dispatch decision.
- Task 19 Step 5: Regression-guard sanity check (test must FAIL without fix).
- Task 20: Manual recapture verification.
- Task 21: Merge choice (skill invocation).
