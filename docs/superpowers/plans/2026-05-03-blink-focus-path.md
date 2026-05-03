# Blink Focus-Path Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore visible cursor blink on focused TextField panels by measuring which of four candidate mechanisms causes the chain break, then fixing the identified ordering issue with a regression test that locks the bug class out.

**Architecture:** Measure-then-fix. Phase 0 adds three structured log lines to the engine wake observability instrumentation, runs a manual GUI capture, and the extended analyzer emits one of four verdicts (O1/O2/O3/O4). Phase 1 dispatches to a small, scoped fix in `emView` or `emSubViewPanel` (≤30 LOC) for outcomes O1/O2, or stops and re-brainstorms for O3/O4. A new integration test (`BlinkProbe`) catches the bug class generically.

**Tech Stack:** Rust (`emcore` crate), Python 3 (`scripts/analyze_hang.py`), bash (capture launcher), `cargo`/`cargo-nextest`/`clippy`, `dlog!`/`emInstr` shared-FD logging, manual GUI session for capture.

**Spec:** `docs/superpowers/specs/2026-05-03-blink-focus-path-design.md`

**Predecessors:**
- `docs/scratch/2026-05-03-blink-findings.md` (A2 path-trace findings)
- `docs/scratch/2026-05-03-has-awake-findings.md` (A1 findings — separate, for B1)
- Tag `instr-7-loop-chain` (existing instrumentation branch tip)

---

## File Structure

### Phase 0 (lives on `instr/blink-trace-2026-05-03`, never merges to main)

| Path | Responsibility | Action |
|---|---|---|
| `crates/emcore/src/emView.rs` | Add 3 emission sites: NOTICE_FC_DECODE, SET_ACTIVE_RESULT, SET_FOCUSED_RESULT | Modify |
| `scripts/analyze_hang.py` | Parse new line types; emit Phase 0 verdict (O1-O4) | Modify |
| `scripts/test_analyze_hang.py` | Pytest cases for new parsers and verdict logic | Modify |

### Phase 0 documentation (lives on `main`)

| Path | Responsibility | Action |
|---|---|---|
| `docs/scratch/2026-05-03-blink-trace-results.md` | Phase 0 verdict + prediction calibration | Create |
| `docs/scratch/2026-05-03-set-active-panel-missing-wake.md` | D1 deferred-divergence note for B1 | Create |

### Phase 1 (lives on `fix/blink-focus-path-2026-05-03`, merges to main)

| Path | Responsibility | Action |
|---|---|---|
| `crates/emcore/src/emView.rs` and/or `crates/emcore/src/emSubViewPanel.rs` | Phase 1a or 1b ordering fix | Modify |
| `crates/emcore/tests/blink_focus_path.rs` | BlinkProbe regression test (and conditional sub-view variant for 1b) | Create |

---

## Branching discipline

```
main (c7fa120b)
 │
 ├─[cut Task 1]─→ instr/blink-trace-2026-05-03   (Phase 0; from tag instr-7-loop-chain)
 │                  │ commit per task: 2, 3, 4, 6, 7
 │                  │ capture happens at Task 9 (manual GUI; on instr branch)
 │                  │ at Task 10, RESULTS commit goes to main, NOT this branch
 │                  └─ tag instr-blink-2026-05-03 at end (Task 11), branch retained for archival
 │
 ├─ direct commits on main: findings doc + D1 note (Task 10, 11)
 │
 └─[cut Task 13]─→ fix/blink-focus-path-2026-05-03   (Phase 1; from main HEAD which now includes findings)
                    │ commit per task: 15, 17, 19
                    │ recapture at Task 20 uses instr-blink-2026-05-03 with fix cherry-picked
                    └─[merge Task 21]─→ main
```

**Hard rules:**
- Never merge instr branch to main.
- Never push `--force`.
- Always run pre-commit hook (do not pass `--no-verify`).
- Per `CLAUDE.md`: read C++ source (`~/Projects/eaglemode-0.96.4/`) before changing Rust to confirm correct ordering.

---

## Task 1: Set up Phase 0 instrumentation branch

**Files:**
- No code edits; branch creation only.

- [ ] **Step 1: Verify clean working tree on main**

```bash
git status
```

Expected: `nothing to commit, working tree clean` and `On branch main`.

- [ ] **Step 2: Cut instrumentation branch from instr-7-loop-chain tag**

```bash
git checkout -b instr/blink-trace-2026-05-03 instr-7-loop-chain
git log --oneline -3
```

Expected first line: a commit from the prior instrumentation work (around `141a030b instr: phase A 7-LOOP-CHAIN — engine wake observability ...`).

- [ ] **Step 3: Verify the build still works on this base**

```bash
cargo check
```

Expected: PASS (no errors). If it fails, investigate before proceeding — the instr base must be buildable.

---

## Task 2: Add NOTICE_FC_DECODE emission in `emView::handle_notice_one`

**Files:**
- Modify: `crates/emcore/src/emView.rs` (insertion just before `behavior.notice(flags, &state, &mut ctx);` at line ~4203 on the instr branch).

- [ ] **Step 1: Locate the insertion point**

```bash
grep -n "behavior.notice(flags, &state, &mut ctx);" crates/emcore/src/emView.rs
```

Expected: one hit (around line 4203). Note the exact line number.

- [ ] **Step 2: Insert the NOTICE_FC_DECODE emission**

Edit `crates/emcore/src/emView.rs`. Find the block:

```rust
            // Deliver notice (C++ emPanel.cpp:1419-1421).
            // No-behavior: treat as base Notice() no-op (C++ base is virtual no-op).
            if let Some(mut behavior) = tree.take_behavior(id) {
                let state = tree.build_panel_state(id, window_focused, pixel_tallness);
                let mut ctx = PanelCtx::with_sched_reach_optional_roots(
                    tree,
                    id,
                    pixel_tallness,
                    sched,
                    framework_actions,
                    root_context,
                    view_context,
                    framework_clipboard,
                    pending_actions,
                );
                behavior.notice(flags, &state, &mut ctx);
```

Insert these lines immediately BEFORE `behavior.notice(...)`:

```rust
                // Phase 0 (B2): NOTICE_FC_DECODE — emit per FOCUS_CHANGED notice
                // delivery so the analyzer can read in_active_path/window_focused
                // at the exact dispatch moment. Unconditional on FOCUS_CHANGED so
                // we capture both branches (would-fire-handler vs. would-skip).
                if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
                    let behavior_type = std::any::type_name_of_val(&*behavior);
                    let line = format!(
                        "NOTICE_FC_DECODE|wall_us={}|panel_id={:?}|behavior_type={}|in_active_path={}|window_focused={}|flags={:#x}\n",
                        crate::emInstr::wall_us(),
                        id,
                        behavior_type,
                        if state.in_active_path { "t" } else { "f" },
                        if state.window_focused { "t" } else { "f" },
                        flags.bits(),
                    );
                    crate::emInstr::write_line(&line);
                }
                behavior.notice(flags, &state, &mut ctx);
```

- [ ] **Step 3: Build to verify**

```bash
cargo check
```

Expected: PASS. If `type_name_of_val` is unstable on the toolchain, swap to `std::any::type_name::<dyn crate::emPanel::PanelBehavior>()` (which always emits `dyn PanelBehavior`) — the analyzer doesn't strictly need per-type discrimination since `panel_id` is the primary key. Document this fallback in a comment if used.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2) NOTICE_FC_DECODE per FOCUS_CHANGED notice delivery

Captures (in_active_path, window_focused, flags) at notice dispatch
time so the blink-trace analyzer can distinguish O1/O2/O3 outcomes per
the B2 spec.
EOF
)"
```

---

## Task 3: Add SET_ACTIVE_RESULT emission in `emView::set_active_panel`

**Files:**
- Modify: `crates/emcore/src/emView.rs` (insertion immediately AFTER the `for id in notice_ids { tree.queue_notice(id, flags, None); }` loop in `set_active_panel`).

- [ ] **Step 1: Locate the insertion point**

```bash
grep -n "for id in notice_ids" crates/emcore/src/emView.rs
```

Expected: one hit inside `set_active_panel` (around line ~1791 on the instr branch). Find the matching closing `}` of the for-loop — that's the insertion point.

- [ ] **Step 2: Insert the SET_ACTIVE_RESULT emission**

Edit `crates/emcore/src/emView.rs`. Find:

```rust
        for id in notice_ids {
            tree.queue_notice(id, flags, None);
        }
    }
```

Replace with:

```rust
        let notice_count = notice_ids.len();
        for id in notice_ids {
            tree.queue_notice(id, flags, None);
        }
        // Phase 0 (B2): SET_ACTIVE_RESULT — emit per set_active_panel call so
        // the analyzer can correlate the panel-tree write against the notice
        // dispatch time. `sched_some=f` reflects that queue_notice is called
        // with None for sched here (the missing-WakeUpUpdateEngine divergence
        // tracked under D1).
        let line = format!(
            "SET_ACTIVE_RESULT|wall_us={}|target_panel_id={:?}|window_focused={}|notice_count={}|sched_some=f\n",
            crate::emInstr::wall_us(),
            target,
            if self.window_focused { "t" } else { "f" },
            notice_count,
        );
        crate::emInstr::write_line(&line);
    }
```

- [ ] **Step 3: Build to verify**

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2) SET_ACTIVE_RESULT per set_active_panel exit

Captures (target_panel_id, window_focused, notice_count, sched_some)
after notices are queued, so the analyzer can correlate the path-update
moment against the notice-dispatch moment per the B2 spec.
EOF
)"
```

---

## Task 4: Add SET_FOCUSED_RESULT emission in `emView::SetFocused`

**Files:**
- Modify: `crates/emcore/src/emView.rs` (insertion AFTER the `for (id, flags) in notice_list { tree.queue_notice(id, flags, None); }` loop in `SetFocused`).

- [ ] **Step 1: Locate the insertion point**

```bash
awk '/pub fn SetFocused/,/^    \}/' crates/emcore/src/emView.rs | head -50
```

Verify the structure of `SetFocused` matches:

```rust
        for (id, flags) in notice_list {
            tree.queue_notice(id, flags, None);
        }
    }
```

- [ ] **Step 2: Determine view-kind detection strategy**

`SetFocused` is a method on `emView`, called for both outer view and sub-views. To distinguish, use the cached `update_engine_id`'s scope. Read what's available:

```bash
grep -n "update_engine_id\|update_engine_scope\|fn IsSubView" crates/emcore/src/emView.rs | head -10
```

If a `view_kind`-style accessor doesn't already exist, derive `view_kind` from the absence/presence of a parent scope. Simplest: emit the engine-id field as a debug string, and let the analyzer correlate.

For this plan, use the simplest unambiguous tag: emit the `update_engine_id` directly (analyzer maps it to scope via REGISTER lines):

```rust
        let panels_notified = notice_list.len();
        for (id, flags) in notice_list {
            tree.queue_notice(id, flags, None);
        }
        // Phase 0 (B2): SET_FOCUSED_RESULT — emit per SetFocused call so the
        // analyzer can detect double-toggles or stale window_focused at notice
        // dispatch. update_engine_id maps to view-kind via REGISTER entries.
        let line = format!(
            "SET_FOCUSED_RESULT|wall_us={}|update_engine_id={:?}|focused={}|panels_notified={}\n",
            crate::emInstr::wall_us(),
            self.update_engine_id,
            if self.window_focused { "t" } else { "f" },
            panels_notified,
        );
        crate::emInstr::write_line(&line);
    }
```

Insert this in place of the `for (id, flags) in notice_list { ... }` block in `SetFocused`.

- [ ] **Step 3: Build to verify**

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "$(cat <<'EOF'
instr: phase 0 (B2) SET_FOCUSED_RESULT per SetFocused exit

Captures (update_engine_id, focused, panels_notified) after SetFocused
queues notices, so the analyzer can detect window_focused double-toggles
or stale-at-dispatch races per the B2 spec. Engine id maps to view-kind
via existing REGISTER entries.
EOF
)"
```

---

## Task 5: Verify Phase 0 instrumentation does not break the build or unit tests

**Files:** none (verification only).

- [ ] **Step 1: Run clippy with warnings as errors**

```bash
cargo clippy -- -D warnings
```

Expected: PASS. If warnings, fix at the source (do not `#[allow]` per `CLAUDE.md`).

- [ ] **Step 2: Run the full nextest suite**

```bash
cargo-nextest ntr
```

Expected: PASS. If any test fails, the new emission lines may be interfering with test infrastructure (e.g., capturing logs in tests). Investigate before proceeding.

- [ ] **Step 3: Visual sanity check on instrumentation**

```bash
EM_INSTR_FD=9 cargo test --test fu005_file_state_signal -- --test-threads=1 9>/tmp/em_instr.smoke.log
grep -c "NOTICE_FC_DECODE\|SET_ACTIVE_RESULT\|SET_FOCUSED_RESULT" /tmp/em_instr.smoke.log
```

Expected: a non-negative integer (likely 0 if these tests don't exercise focus changes; that's fine — we just want the format to compile and be writable). If non-zero, inspect a sample line and confirm the format matches the spec.

---

## Task 6: Extend `analyze_hang.py` parser to recognize the three new line types

**Files:**
- Modify: `scripts/analyze_hang.py` (add three parse functions after the existing `parse_inval_drain` around line 97).

- [ ] **Step 1: Read the existing parser shape**

```bash
sed -n '40,110p' scripts/analyze_hang.py
```

Expected: a series of `parse_X(line)` functions following the pattern `f = _parse_kv_line(line, "X"); ... return {...}`.

- [ ] **Step 2: Add three new parse functions**

Append these functions immediately after `parse_inval_drain` (around line 105):

```python
def parse_notice_fc_decode(line):
    """B2 Phase 0: parse NOTICE_FC_DECODE."""
    f = _parse_kv_line(line, "NOTICE_FC_DECODE")
    if f is None:
        return None
    return {
        "kind": "NOTICE_FC_DECODE",
        "wall_us": int(f["wall_us"]),
        "panel_id": f["panel_id"],
        "behavior_type": f.get("behavior_type", ""),
        "in_active_path": f["in_active_path"] == "t",
        "window_focused": f["window_focused"] == "t",
        "flags": int(f["flags"], 16),
    }


def parse_set_active_result(line):
    """B2 Phase 0: parse SET_ACTIVE_RESULT."""
    f = _parse_kv_line(line, "SET_ACTIVE_RESULT")
    if f is None:
        return None
    return {
        "kind": "SET_ACTIVE_RESULT",
        "wall_us": int(f["wall_us"]),
        "target_panel_id": f["target_panel_id"],
        "window_focused": f["window_focused"] == "t",
        "notice_count": int(f["notice_count"]),
        "sched_some": f["sched_some"] == "t",
    }


def parse_set_focused_result(line):
    """B2 Phase 0: parse SET_FOCUSED_RESULT."""
    f = _parse_kv_line(line, "SET_FOCUSED_RESULT")
    if f is None:
        return None
    return {
        "kind": "SET_FOCUSED_RESULT",
        "wall_us": int(f["wall_us"]),
        "update_engine_id": f["update_engine_id"],
        "focused": f["focused"] == "t",
        "panels_notified": int(f["panels_notified"]),
    }
```

- [ ] **Step 3: Run the parser smoke test**

```bash
python3 -c "
from scripts.analyze_hang import parse_notice_fc_decode, parse_set_active_result, parse_set_focused_result
print(parse_notice_fc_decode('NOTICE_FC_DECODE|wall_us=12345|panel_id=PanelId(497v1)|behavior_type=TFP|in_active_path=t|window_focused=f|flags=0xf0'))
print(parse_set_active_result('SET_ACTIVE_RESULT|wall_us=12300|target_panel_id=PanelId(497v1)|window_focused=t|notice_count=3|sched_some=f'))
print(parse_set_focused_result('SET_FOCUSED_RESULT|wall_us=12100|update_engine_id=EngineId(7v1)|focused=t|panels_notified=42'))
"
```

Expected: three dicts printed, all fields populated correctly.

- [ ] **Step 4: Commit**

```bash
git add scripts/analyze_hang.py
git commit -m "$(cat <<'EOF'
analyzer: phase 0 (B2) parsers for new instrumentation lines

Adds parse_notice_fc_decode, parse_set_active_result, and
parse_set_focused_result to recognize the three new structured log
lines emitted by emView for the blink focus-path investigation.
EOF
)"
```

---

## Task 7: Implement Phase 0 verdict logic in `analyze_hang.py blink` command

**Files:**
- Modify: `scripts/analyze_hang.py` (extend `blink_command_text` at line ~285 to emit a Phase 0 verdict section).

- [ ] **Step 1: Read the existing blink command structure**

```bash
sed -n '285,400p' scripts/analyze_hang.py
```

Expected: a function that walks lines between markers, accumulates events, and prints a verdict text.

- [ ] **Step 2: Add the verdict-emission helper**

Append this helper just BEFORE `blink_command_text`:

```python
def _phase0_verdict(notice_fc_events, wake_events, panel_id_filter,
                    fire_caller_lines):
    """B2 Phase 0: compute O1/O2/O3/O4 verdict from events.

    notice_fc_events: list of NOTICE_FC_DECODE dicts (post-marker, filtered
        to panel_id_filter).
    wake_events: list of WAKE dicts (post-marker, all).
    panel_id_filter: panel_id string for the focus-target TextField.
    fire_caller_lines: iterable of (file_substr, line_min, line_max) tuples
        identifying the wake_up_panel call site inside the notice handler.

    Returns: dict {"outcome": "O1"|"O2"|"O3"|"O4"|"O3-AMBIG", "events": [...]}.
    """
    if not notice_fc_events:
        return {"outcome": "O4", "events": []}

    # Decisive event = last NOTICE_FC_DECODE for the target panel.
    decisive = notice_fc_events[-1]

    # Did a wake fire from the handler within 100ms after the notice?
    fired = False
    for wake in wake_events:
        if wake["wall_us"] < decisive["wall_us"]:
            continue
        if wake["wall_us"] - decisive["wall_us"] > 100_000:  # 100ms
            break
        caller = wake.get("caller", "")
        for substr, lmin, lmax in fire_caller_lines:
            if substr in caller:
                # caller format: "path/to/file.rs:NNN"
                try:
                    line_no = int(caller.rsplit(":", 1)[1])
                except (ValueError, IndexError):
                    continue
                if lmin <= line_no <= lmax:
                    fired = True
                    break
        if fired:
            break

    iap = decisive["in_active_path"]
    wf = decisive["window_focused"]

    if not iap:
        outcome = "O1"  # in_active_path stale at notice time
    elif iap and not wf:
        outcome = "O2"  # window_focused stale at notice time
    elif iap and wf and fired:
        outcome = "O3"  # handler ran; bug elsewhere
    elif iap and wf and not fired:
        outcome = "O3-AMBIG"  # impossible by code; flag and treat as O3
    else:
        outcome = "O3-AMBIG"

    return {
        "outcome": outcome,
        "events": [
            {
                "wall_us": decisive["wall_us"],
                "panel_id": decisive["panel_id"],
                "behavior_type": decisive["behavior_type"],
                "in_active_path": iap,
                "window_focused": wf,
                "flags": decisive["flags"],
                "branch_fired": fired,
            }
        ],
    }
```

- [ ] **Step 3: Wire the verdict into `blink_command_text`**

In `blink_command_text`, after the existing per-event accumulation but before the final return:

1. Find the loop that accumulates events. Add accumulators for the three new line types alongside the existing ones:

```python
    notice_fc_events_for_target = []
    set_active_events = []
    set_focused_events = []
```

2. Inside the line loop, add:

```python
            elif ln.startswith("NOTICE_FC_DECODE|"):
                ev = parse_notice_fc_decode(ln)
                if ev and ev["panel_id"] == focus["recipient_panel_id"]:
                    notice_fc_events_for_target.append(ev)
            elif ln.startswith("SET_ACTIVE_RESULT|"):
                ev = parse_set_active_result(ln)
                if ev:
                    set_active_events.append(ev)
            elif ln.startswith("SET_FOCUSED_RESULT|"):
                ev = parse_set_focused_result(ln)
                if ev:
                    set_focused_events.append(ev)
```

3. After the loop, before the final `chain.append(...)` calls, compute and append the verdict:

```python
    fire_caller_lines = [
        ("crates/emtest/src/emTestPanel.rs", 240, 245),
        ("crates/emcore/src/emColorFieldFieldPanel.rs", 144, 148),
    ]
    verdict = _phase0_verdict(
        notice_fc_events_for_target, wake_events,
        focus["recipient_panel_id"], fire_caller_lines,
    )
    out_lines.append("")
    out_lines.append(f"## Phase 0 verdict: {verdict['outcome']}")
    for ev in verdict["events"]:
        out_lines.append(
            f"  decisive_event: wall_us={ev['wall_us']}, panel_id={ev['panel_id']}, "
            f"behavior_type={ev['behavior_type']}"
        )
        out_lines.append(
            f"    in_active_path={ev['in_active_path']}, "
            f"window_focused={ev['window_focused']}, "
            f"flags={ev['flags']:#x}, branch_fired={ev['branch_fired']}"
        )
    out_lines.append("")
    out_lines.append("## Phase 0 dispatch")
    dispatch = {
        "O1": "→ Phase 1a: in_active_path stale; fix in set_active_panel/build_panel_state",
        "O2": "→ Phase 1b: window_focused stale; fix in SubViewPanel::Input/SetFocused",
        "O3": "→ STOP: re-brainstorm B2.1 (bug not in focus-path system)",
        "O3-AMBIG": "→ STOP: re-brainstorm B2.1 (impossible-row outcome; investigate)",
        "O4": "→ STOP: re-brainstorm B2.1 (notice not delivered to TextField)",
    }
    out_lines.append(dispatch.get(verdict["outcome"], "→ unknown outcome"))
```

You will need to ensure `wake_events` is collected — it likely already is. If not, add an accumulator for `WAKE|` lines parallel to the others. Verify by:

```bash
grep -n "WAKE|\|wake_events" scripts/analyze_hang.py | head -10
```

- [ ] **Step 4: Commit**

```bash
git add scripts/analyze_hang.py
git commit -m "$(cat <<'EOF'
analyzer: phase 0 (B2) verdict emission for blink command

Computes O1/O2/O3/O4 outcome from NOTICE_FC_DECODE,
SET_ACTIVE_RESULT, SET_FOCUSED_RESULT, and WAKE events per the B2
spec truth table. Prints verdict + dispatch instructions to the
report.
EOF
)"
```

---

## Task 8: Add unit tests for analyzer changes

**Files:**
- Modify: `scripts/test_analyze_hang.py` (add test cases for parsers + verdict).

- [ ] **Step 1: Read the existing test file structure**

```bash
ls scripts/test_analyze_hang.py 2>/dev/null && head -40 scripts/test_analyze_hang.py
```

If file does not exist on the instr branch, that is acceptable — create it. Otherwise extend it.

- [ ] **Step 2: Add test cases**

Append these tests (or create the file with them):

```python
import sys
import os
sys.path.insert(0, os.path.dirname(__file__))
from analyze_hang import (
    parse_notice_fc_decode, parse_set_active_result, parse_set_focused_result,
    _phase0_verdict,
)


def test_parse_notice_fc_decode_full():
    line = ("NOTICE_FC_DECODE|wall_us=77680728|panel_id=PanelId(497v1)|"
            "behavior_type=TextFieldPanel|in_active_path=t|window_focused=f|flags=0xf0")
    ev = parse_notice_fc_decode(line)
    assert ev["wall_us"] == 77680728
    assert ev["panel_id"] == "PanelId(497v1)"
    assert ev["behavior_type"] == "TextFieldPanel"
    assert ev["in_active_path"] is True
    assert ev["window_focused"] is False
    assert ev["flags"] == 0xf0


def test_phase0_verdict_o1_iap_false():
    notice = [{
        "wall_us": 1000, "panel_id": "P", "behavior_type": "T",
        "in_active_path": False, "window_focused": True, "flags": 0xf0,
    }]
    v = _phase0_verdict(notice, [], "P", [("emTestPanel.rs", 240, 245)])
    assert v["outcome"] == "O1"


def test_phase0_verdict_o2_wf_false():
    notice = [{
        "wall_us": 1000, "panel_id": "P", "behavior_type": "T",
        "in_active_path": True, "window_focused": False, "flags": 0xf0,
    }]
    v = _phase0_verdict(notice, [], "P", [("emTestPanel.rs", 240, 245)])
    assert v["outcome"] == "O2"


def test_phase0_verdict_o3_branch_fires():
    notice = [{
        "wall_us": 1000, "panel_id": "P", "behavior_type": "T",
        "in_active_path": True, "window_focused": True, "flags": 0xf0,
    }]
    wake = [{
        "wall_us": 1050, "caller": "crates/emtest/src/emTestPanel.rs:242",
    }]
    v = _phase0_verdict(notice, wake, "P", [("emTestPanel.rs", 240, 245)])
    assert v["outcome"] == "O3"


def test_phase0_verdict_o3_ambig_branch_does_not_fire():
    notice = [{
        "wall_us": 1000, "panel_id": "P", "behavior_type": "T",
        "in_active_path": True, "window_focused": True, "flags": 0xf0,
    }]
    v = _phase0_verdict(notice, [], "P", [("emTestPanel.rs", 240, 245)])
    assert v["outcome"] == "O3-AMBIG"


def test_phase0_verdict_o4_no_notice():
    v = _phase0_verdict([], [], "P", [("emTestPanel.rs", 240, 245)])
    assert v["outcome"] == "O4"


if __name__ == "__main__":
    import sys
    failed = 0
    for name, fn in list(globals().items()):
        if name.startswith("test_") and callable(fn):
            try:
                fn()
                print(f"PASS {name}")
            except AssertionError as e:
                print(f"FAIL {name}: {e}")
                failed += 1
    sys.exit(1 if failed else 0)
```

- [ ] **Step 3: Run the tests**

```bash
python3 scripts/test_analyze_hang.py
```

Expected: All `PASS`. Exit 0. Fix any failures before proceeding — they indicate the parser or verdict logic is wrong, and Phase 0 will produce wrong outcomes.

- [ ] **Step 4: Commit**

```bash
git add scripts/test_analyze_hang.py
git commit -m "$(cat <<'EOF'
analyzer: phase 0 (B2) unit tests for new parsers and verdict logic

Covers each row of the truth table (O1/O2/O3/O3-AMBIG/O4) so
verdict miscompute can be caught at test time before depending on it
to interpret a real capture.
EOF
)"
```

---

## Task 9: Manual GUI capture for Phase 0

**Files:** none (capture only). Produces `/tmp/em_instr.blink-trace.log`.

**Note for the implementer:** This task requires the user's hands. Cannot be fully automated.

- [ ] **Step 1: Build release**

```bash
cargo build -p eaglemode --release
```

Expected: PASS. The release binary lives at `target/release/eaglemode`.

- [ ] **Step 2: Verify the capture launcher exists**

```bash
ls scripts/run_blink_capture.sh
```

Expected: exists (inherited from `instr-7-loop-chain`). If missing, recreate it from the spec's capture procedure.

- [ ] **Step 3: Launch the GUI with structured logging**

In one terminal:

```bash
EM_INSTR_FD=9 cargo run -p eaglemode --release 9>/tmp/em_instr.blink-trace.log
```

The GUI window opens after a few seconds. **DO NOT close it manually.**

- [ ] **Step 4: Send "open" SIGUSR1 marker once the window is up**

In a second terminal:

```bash
PID=$(pgrep -f "target/release/eaglemode")
echo "PID=$PID"
kill -USR1 $PID
```

Expected: prints a single PID. Verify there are no orphan PIDs (`pgrep` returns just one). If multiple, kill the orphans first.

- [ ] **Step 5: Click into a TextField and hold focus**

ASK THE USER: "Please click into one of the test-panel TextFields (e.g., the 'tf1' or 'tf2' fields). Hold focus there for ~30 seconds without clicking elsewhere or moving the mouse out of the TextField. Reply 'done' when 30 seconds have passed."

Wait for user reply.

- [ ] **Step 6: Send "close" SIGUSR1 marker**

```bash
kill -USR1 $PID
```

Then wait ~1 second for buffer flush.

- [ ] **Step 7: Close the GUI cleanly**

ASK THE USER: "Please close the GUI window now (Alt+F4 or click the close button). Reply 'closed' when the process has exited."

Wait for user reply.

- [ ] **Step 8: Verify the capture log is non-empty and contains markers**

```bash
ls -la /tmp/em_instr.blink-trace.log
grep -c "MARKER\|NOTICE_FC_DECODE\|SET_ACTIVE_RESULT\|SET_FOCUSED_RESULT\|WAKE" /tmp/em_instr.blink-trace.log
```

Expected: file size > 100KB; grep returns a positive integer (likely thousands). MARKER count should be 2 (open + close).

If MARKER count is 0 or 1, the SIGUSR1 was sent to the wrong PID — start over from Step 3 with a clean log path.

---

## Task 10: Run the analyzer; record outcome; write findings + D1 deferral on main

**Files:**
- Create on main: `docs/scratch/2026-05-03-blink-trace-results.md`
- Create on main: `docs/scratch/2026-05-03-set-active-panel-missing-wake.md`

- [ ] **Step 1: Run the analyzer's blink command**

```bash
python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-trace.log > /tmp/blink-trace-report.txt
cat /tmp/blink-trace-report.txt
```

Expected: a report ending with a "Phase 0 verdict: OX" line (X ∈ {1, 2, 3, 3-AMBIG, 4}) and a dispatch instruction.

- [ ] **Step 2: Note the outcome**

Extract the verdict:

```bash
grep "^## Phase 0 verdict:" /tmp/blink-trace-report.txt
```

Expected: `## Phase 0 verdict: OX`. Record this for use in subsequent tasks.

- [ ] **Step 3: Switch to main and write the findings doc**

```bash
git stash --include-untracked
git checkout main
git pull origin main  # ensure up-to-date; if anything fetched, review before continuing
```

Create `docs/scratch/2026-05-03-blink-trace-results.md` with this content (substitute `<...>` placeholders):

```markdown
# Blink path-trace Phase 0 results — 2026-05-03

Capture: `/tmp/em_instr.blink-trace.log`
Branch: `instr/blink-trace-2026-05-03` @ `<commit-sha>`
Run: <YYYY-MM-DD HH:MM>

## Decisive NOTICE_FC_DECODE event

- `wall_us`: <copy from analyzer report>
- `panel_id`: <copy>
- `behavior_type`: <copy>
- `in_active_path`: <t|f>
- `window_focused`: <t|f>
- `flags`: <0x...>

## Branch fired (WAKE within 100ms window?)

<t|f>

## Verdict

**<O1|O2|O3|O3-AMBIG|O4>**

Per B2 spec dispatch:
- O1 → proceed to Phase 1a
- O2 → proceed to Phase 1b
- O3 / O3-AMBIG / O4 → STOP, re-brainstorm B2.1

## Prediction calibration (advisor's check)

- Pre-measurement priors: 60% O1, 25% O2, 10% O3, 5% O4
- Actual: <OX>
- Retrospective: <one-line note: was the prior well-calibrated against
  reality? did code-reading vs. data instinct turn out to be right?>

## Full analyzer report

<paste contents of /tmp/blink-trace-report.txt below this line>
```

Fill in the `<...>` placeholders from the analyzer report and capture metadata.

- [ ] **Step 4: Write the D1 deferral note**

Create `docs/scratch/2026-05-03-set-active-panel-missing-wake.md`:

```markdown
# `emView::set_active_panel` missing `WakeUpUpdateEngine` — divergence note for B1

Found during B2 spec exploration on 2026-05-03 at main `637d8bf1`.

## Divergence

C++ `emView::SetActivePanel` (`~/Projects/eaglemode-0.96.4/src/emCore/emView.cpp:307`):
```cpp
ActivePanel = panel;
ActivationAdherent = adherent;
InvalidateHighlight();
TitleInvalid = true;
UpdateEngine->WakeUp();      // <-- explicit wake
Signal(ControlPanelSignal);
```

Rust `emView::set_active_panel` (`crates/emcore/src/emView.rs:1791-1793` on
main `637d8bf1`):
```rust
for id in notice_ids {
    tree.queue_notice(id, flags, None);  // None for sched
}
```

`tree.queue_notice(..., None)` calls `add_to_notice_list(..., None)`, which
takes the wake-up branch only when `sched` is `Some`. So the Rust port
silently relies on incidental wakes from elsewhere (e.g., subsequent
`InvalidateHighlight`, input-dispatch side-effects) to flush the queued
FOCUS_CHANGED notice.

## Why this is deferred from B2

Adding the wake-up call increases UpdateEngine wake cadence, which is the
exact concern B1 (the `has_awake==1` 66.7%-of-slices work) is trying to
reduce. B1 should make the call about whether to add this wake source.

## Why this is real

In the A2 capture, the FOCUS_CHANGED notice DID dispatch (the NOTICE log
entry was present at the textfield), so the missing wake-up was not
load-bearing for the blink symptom. But in any scenario where no other
engine wakes the UpdateEngine between the click and the next idle frame,
the focus notice would be silently delayed by up to one timeslice
(~50 ms). This is observable as occasional "click did not feel responsive"
without any error trace.

## Recommended action for B1

Decide whether to:
1. Add the explicit `view.WakeUpUpdateEngine(ctx)` call after the
   `for id in notice_ids` loop in `set_active_panel` (mirrors C++).
2. Leave it as-is and document the latent fragility (matches current
   behavior; saves wake cadence).
3. Pass `Some(sched)` to `queue_notice` so the wake propagates without an
   additional call (cleanest if `set_active_panel`'s caller already has
   sched access — which it does, via the `ctx` parameter).
```

- [ ] **Step 5: Commit findings + D1 note on main**

```bash
git add docs/scratch/2026-05-03-blink-trace-results.md docs/scratch/2026-05-03-set-active-panel-missing-wake.md
git commit -m "$(cat <<'EOF'
scratch: B2 Phase 0 blink-trace results + B1 D1 deferral note

Phase 0 capture verdict: <OX>. Records prediction-vs-actual for
calibration and the analyzer's full report. Companion D1 note
captures the set_active_panel missing-WakeUpUpdateEngine divergence
that came up during B2 exploration but is deferred to B1.
EOF
)"
git push origin main
```

- [ ] **Step 6: Restore instr branch state**

```bash
git checkout instr/blink-trace-2026-05-03
git stash pop || true   # may be empty; that's fine
```

---

## Task 11: Tag instr branch for archival; finalize Phase 0

**Files:** none (tagging only).

- [ ] **Step 1: Tag the instr branch tip**

```bash
git checkout instr/blink-trace-2026-05-03
git tag instr-blink-2026-05-03
git push origin instr/blink-trace-2026-05-03 instr-blink-2026-05-03
```

- [ ] **Step 2: Decision point — read the verdict and decide path**

Read the verdict from `docs/scratch/2026-05-03-blink-trace-results.md`:

```bash
git checkout main
grep -E "^\*\*<.*\*\*$|^\*\*O[0-9]" docs/scratch/2026-05-03-blink-trace-results.md
```

Based on the verdict, dispatch:
- **O1**: Continue to Task 12 (Phase 1a). Skip Task 13.
- **O2**: Skip Task 12. Continue to Task 13 (Phase 1b).
- **O3 / O3-AMBIG / O4**: Skip Tasks 12-19. Go to Task 20 (re-brainstorm handoff).

DOCUMENT THE DISPATCH DECISION IN A COMMIT:

```bash
# Edit blink-trace-results.md to add a final line:
#   "**Next phase dispatched:** Phase 1a / Phase 1b / B2.1 re-brainstorm"
git add docs/scratch/2026-05-03-blink-trace-results.md
git commit -m "scratch: B2 Phase 0 dispatch decision recorded — <next phase>"
git push origin main
```

---

## Task 12: Phase 1a — fix for outcome O1 (`in_active_path` stale at notice time)

**ONLY EXECUTE IF Task 11 verdict was O1.**

**Files:**
- Modify: `crates/emcore/src/emView.rs` and/or `crates/emcore/src/emPanelTree.rs`
- Possibly modify: `crates/emcore/src/emSubViewPanel.rs`
- Create: `crates/emcore/tests/blink_focus_path.rs`

- [ ] **Step 1: Cut the fix branch from main HEAD**

```bash
git checkout main
git pull origin main
git checkout -b fix/blink-focus-path-2026-05-03
```

- [ ] **Step 2: Run the diagnostic procedure (B2 spec § Phase 1a)**

Open the capture log and the analyzer report for the Phase 0 results:

```bash
less docs/scratch/2026-05-03-blink-trace-results.md
less /tmp/em_instr.blink-trace.log
```

Find the offending `NOTICE_FC_DECODE` line (the one with `in_active_path=f`). Note its `wall_us`. Then find:

```bash
WALL=<copy from blink-trace-results>
TARGET=<copy panel_id>
# Most recent SET_ACTIVE_RESULT for the target panel before the notice:
awk -v wall=$WALL -v target="$TARGET" 'BEGIN{best=""} \
  /SET_ACTIVE_RESULT/{ if (match($0, /wall_us=([0-9]+)/, m) && m[1] < wall && index($0, "target_panel_id="target)) best = $0 } \
  END{print best}' /tmp/em_instr.blink-trace.log

# Most recent SET_FOCUSED_RESULT before the notice:
awk -v wall=$WALL 'BEGIN{best=""} \
  /SET_FOCUSED_RESULT/{ if (match($0, /wall_us=([0-9]+)/, m) && m[1] < wall) best = $0 } \
  END{print best}' /tmp/em_instr.blink-trace.log
```

Identify which call queued the FOCUS_CHANGED that was delivered:

- If the latest `SET_ACTIVE_RESULT` for the target precedes the notice and reports `notice_count > 0` AND its target_panel_id matches the notice's panel_id, then `set_active_panel` queued the FOCUS_CHANGED.
- If `SET_FOCUSED_RESULT` precedes the notice with `panels_notified > 0` AND no `SET_ACTIVE_RESULT` is closer in time, then `SetFocused` queued it.

Read C++ ground truth for the suspect site:
- `~/Projects/eaglemode-0.96.4/src/emCore/emView.cpp` `SetActivePanel` (line 273-314)
- `~/Projects/eaglemode-0.96.4/src/emCore/emView.cpp` `SetFocused` (search for `void emView::SetFocused`)
- `~/Projects/eaglemode-0.96.4/src/emCore/emPanel.cpp` notice flush logic

- [ ] **Step 3: Identify the divergence and write a one-paragraph diagnosis**

Open `docs/scratch/2026-05-03-blink-trace-results.md` and append a `## Diagnosis (Phase 1a)` section with one paragraph naming:
1. Which call queued the FOCUS_CHANGED with stale `in_active_path`.
2. The C++ ordering at that site.
3. The Rust ordering at that site.
4. The proposed fix (one sentence).

Commit before applying:

```bash
git add docs/scratch/2026-05-03-blink-trace-results.md
git commit -m "scratch: B2 Phase 1a diagnosis — <one-line summary>"
```

- [ ] **Step 4: Apply the fix**

The exact change is data-dependent. Likely shapes per spec:
- Reorder operations at one call site so the path-update precedes the notice queueing.
- Change a `queue_notice(id, flags, None)` to be placed at a different point in the sequence.
- In `SetFocused`, re-evaluate `in_active_path` at notice flush time rather than queue time.
- In `SubViewPanel::Input`, swap or remove a SetFocused call.

**Hard constraint:** the fix MUST be ≤30 LOC across at most 2 files. If diagnosis requires more, STOP and escalate per spec ("If diagnosis reveals the fix is larger, escalate to re-brainstorm B2.1").

- [ ] **Step 5: Verify the build is clean**

```bash
cargo check
cargo clippy -- -D warnings
```

Expected: PASS. Fix any warnings at the source.

- [ ] **Step 6: Commit the fix (separate commit from the test)**

```bash
git add <changed files>
git commit -m "$(cat <<'EOF'
fix(blink): <one-line: what ordering was wrong>

Phase 0 capture (docs/scratch/2026-05-03-blink-trace-results.md)
identified that <which call> queued FOCUS_CHANGED with
in_active_path=false. C++ at <emView.cpp:NNN> orders <X> before <Y>;
the Rust port had them <reversed/redundant/etc>. Fixes by <one-line
mechanic>.

Refs: docs/superpowers/specs/2026-05-03-blink-focus-path-design.md
EOF
)"
```

---

## Task 13: Phase 1b — fix for outcome O2 (`window_focused` stale at notice time)

**ONLY EXECUTE IF Task 11 verdict was O2.**

**Files:**
- Modify: `crates/emcore/src/emSubViewPanel.rs` (primary suspect per spec)
- Possibly modify: `crates/emcore/src/emView.rs` (if a `SetFocused` guard is required)
- Create: `crates/emcore/tests/blink_focus_path.rs`

- [ ] **Step 1: Cut the fix branch from main HEAD**

```bash
git checkout main
git pull origin main
git checkout -b fix/blink-focus-path-2026-05-03
```

- [ ] **Step 2: Run the diagnostic procedure (B2 spec § Phase 1b)**

Open the capture log and search for `SET_FOCUSED_RESULT` events for the sub-view's `update_engine_id` (cross-reference with `REGISTER` lines for SubView scope to identify the right engine id):

```bash
WALL=<copy from blink-trace-results>
# All SET_FOCUSED_RESULT events before the notice, in chronological order:
awk -v wall=$WALL '/SET_FOCUSED_RESULT/{ if (match($0, /wall_us=([0-9]+)/, m) && m[1] < wall) print $0 }' \
  /tmp/em_instr.blink-trace.log
```

Look for:
- A `SET_FOCUSED_RESULT|... |focused=t|...` followed by a `SET_FOCUSED_RESULT|... |focused=f|...` for the SAME `update_engine_id`. This is a double-toggle race.
- An ABSENCE of any `focused=t` SET_FOCUSED_RESULT for the sub-view's engine id before the notice. This is a propagation gap.

Read C++ ground truth:
- `~/Projects/eaglemode-0.96.4/src/emCore/emSubViewPanel.cpp` `Input` and `Notice`
- `~/Projects/eaglemode-0.96.4/src/emCore/emView.cpp` `SetFocused`

Walk the input-dispatch path in Rust to confirm the offending site:
- `crates/emcore/src/emWindow.rs::dispatch_input`
- `crates/emcore/src/emSubViewPanel.rs::Input` (lines 296-298 SetFocused, 348 set_active_panel)
- `crates/emcore/src/emSubViewPanel.rs::notice` (lines 547-549 sub_view.SetFocused)

- [ ] **Step 3: Identify the divergence and write a one-paragraph diagnosis**

Same pattern as Task 12 Step 3 but for `## Diagnosis (Phase 1b)`.

```bash
git add docs/scratch/2026-05-03-blink-trace-results.md
git commit -m "scratch: B2 Phase 1b diagnosis — <one-line summary>"
```

- [ ] **Step 4: Apply the fix**

Likely shapes per spec:
- Remove a redundant SetFocused call (one of `SubViewPanel::Input` line 296-299 or `SubViewPanel::notice` lines 547-549 — they currently race).
- Reorder so the authoritative SetFocused happens last.
- Add a guard in `view.SetFocused` to skip if state is already correct (already exists at lines 803-805; verify it's not creating the race itself).

**Hard constraint:** ≤30 LOC across at most 2 files. Escalate to re-brainstorm if larger.

- [ ] **Step 5: Verify the build is clean**

```bash
cargo check
cargo clippy -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit the fix**

```bash
git add <changed files>
git commit -m "$(cat <<'EOF'
fix(blink): <one-line: what sub-view focus ordering was wrong>

Phase 0 capture (docs/scratch/2026-05-03-blink-trace-results.md)
identified <focused-toggle race / propagation gap> in the sub-view
focus path. C++ at <emSubViewPanel.cpp:NNN> handles this by <X>; the
Rust port had <Y>. Fixes by <one-line mechanic>.

Refs: docs/superpowers/specs/2026-05-03-blink-focus-path-design.md
EOF
)"
```

---

## Task 14: Add the `BlinkProbe` regression test (common to Phase 1a and 1b)

**ONLY EXECUTE if Task 12 or Task 13 ran (i.e., not for O3/O4).**

**Files:**
- Create: `crates/emcore/tests/blink_focus_path.rs`

- [ ] **Step 1: Write the failing test (TDD: this should fail before fix, pass after)**

Note: at this point the fix from Task 12 or 13 is already applied, so the test should PASS. This task captures the regression-guard discipline: even though the test wasn't written first, it must demonstrably fail when the fix is reverted (Step 5 verifies this).

The test asserts on **focus-path bookkeeping at notice-dispatch time** — the *exact* mechanism of the bug. Driving the full engine-cycle chain from an integration test would require constructing a full `emWindow` + `HashMap<WindowId, emWindow>` for `DoTimeSlice`, which is heavier scaffolding than this regression guard needs.

Create `crates/emcore/tests/blink_focus_path.rs`:

```rust
//! B2 regression test: focus-path bookkeeping at notice-dispatch time.
//!
//! Locks the bug class identified in the engine-wake observability
//! investigation (A2): when a panel becomes the active focused panel,
//! its notice handler MUST observe `state.in_focused_path() == true`.
//! If false, the focus-dependent branch of the handler (e.g., the
//! TextField cursor-blink wake-up) silently never fires.
//!
//! Generic over widget — uses an instrumented BlinkProbe behavior with
//! the same notice shape as production TextFieldPanel.

use std::cell::Cell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emEngineCtx::{EngineCtx, PanelCtx};
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emPainter::emPainter;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelTree::PanelId;
use emcore::test_view_harness::TestViewHarness;

#[derive(Default)]
struct ProbeShared {
    notice_fc_count: Cell<u32>,
    notice_saw_in_focused_path: Cell<bool>,
    notice_saw_in_active_path: Cell<bool>,
    notice_saw_window_focused: Cell<bool>,
    wake_called: Cell<bool>,
}

struct BlinkProbe {
    shared: Rc<ProbeShared>,
}

impl BlinkProbe {
    fn new(shared: Rc<ProbeShared>) -> Self { Self { shared } }
}

impl PanelBehavior for BlinkProbe {
    fn Paint(
        &mut self, _p: &mut emPainter, _c: emColor, _w: f64, _h: f64, _s: &PanelState,
    ) {}

    fn Input(
        &mut self, _e: &emInputEvent, _s: &PanelState, _i: &emInputState, _c: &mut PanelCtx,
    ) -> bool { false }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState, ctx: &mut PanelCtx) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.shared.notice_fc_count.set(self.shared.notice_fc_count.get() + 1);
            self.shared.notice_saw_in_active_path.set(state.in_active_path);
            self.shared.notice_saw_window_focused.set(state.window_focused);
            let in_fp = state.in_focused_path();
            self.shared.notice_saw_in_focused_path.set(in_fp);
            if in_fp {
                let id = ctx.id;
                ctx.wake_up_panel(id);
                self.shared.wake_called.set(true);
            }
        }
    }

    fn Cycle(&mut self, _e: &mut EngineCtx, _p: &mut PanelCtx) -> bool { false }
}

#[test]
fn focused_panel_notice_sees_in_focused_path_true() {
    use emcore::emView::emView;

    let mut h = TestViewHarness::new();

    // Build tree: root → probe (both focusable).
    let root = h.tree.create_root_deferred_view("root");
    h.tree.get_mut(root).unwrap().focusable = true;
    h.tree.Layout(root, 0.0, 0.0, 1.0, 1.0, 1.0, None);

    let probe_id = h.tree.create_child(root, "probe", None);
    h.tree.get_mut(probe_id).unwrap().focusable = true;
    h.tree.Layout(probe_id, 0.0, 0.0, 1.0, 1.0, 1.0, None);

    let shared = Rc::new(ProbeShared::default());
    h.tree.set_behavior(probe_id, Box::new(BlinkProbe::new(shared.clone())));

    // Build the view; pump one Update to settle initial notices.
    let mut view = emView::new(emcore::emContext::emContext::NewRoot(), root, 640.0, 480.0);
    {
        let mut sc = h.sched_ctx();
        view.Update(&mut h.tree, &mut sc);
    }

    // Reset counters — we only care about post-set_active_panel notices.
    shared.notice_fc_count.set(0);
    shared.wake_called.set(false);

    // Window focus + active-panel transition (mimics user clicking probe).
    view.SetFocused(&mut h.tree, true);
    {
        let mut sc = h.sched_ctx();
        view.set_active_panel(&mut h.tree, probe_id, false, &mut sc);
    }

    // Pump Update to flush notices.
    {
        let mut sc = h.sched_ctx();
        view.Update(&mut h.tree, &mut sc);
    }

    // Assertions: the FOCUS_CHANGED notice must have been delivered to
    // the probe with both in_active_path and window_focused true.
    assert!(
        shared.notice_fc_count.get() > 0,
        "FOCUS_CHANGED notice was never delivered to the probe after \
         set_active_panel + Update; notice flush is broken"
    );
    assert!(
        shared.notice_saw_in_active_path.get(),
        "in_active_path was false at FOCUS_CHANGED dispatch — set_active_panel \
         did not propagate the path-update before the notice queued. \
         (in_active_path=false, window_focused={})",
        shared.notice_saw_window_focused.get()
    );
    assert!(
        shared.notice_saw_window_focused.get(),
        "window_focused was false at FOCUS_CHANGED dispatch — SetFocused \
         did not propagate before the notice queued. \
         (in_active_path={}, window_focused=false)",
        shared.notice_saw_in_active_path.get()
    );
    assert!(
        shared.notice_saw_in_focused_path.get(),
        "in_focused_path() returned false despite both in_active_path and \
         window_focused being true; PanelState::in_focused_path is broken"
    );
    assert!(
        shared.wake_called.get(),
        "wake_up_panel was not called from notice handler — the \
         focus-dependent branch of the handler is unreachable, so any \
         downstream blink/animation will silently freeze"
    );
}
```

- [ ] **Step 2: Verify API names and signatures match**

```bash
grep -n "pub fn create_root_deferred_view\|pub fn set_behavior\|pub fn Layout" \
  crates/emcore/src/emPanelTree.rs | head
```

If `Layout`'s signature differs from what the test expects (`Layout(id, x, y, w, h, ?, sched)`), check the canonical setup in `setup_children_on` at `crates/emcore/src/emView.rs:5520` and copy the exact pattern.

If `create_root_deferred_view` is not visible to the integration test (compile error), add the `test-support` feature to emcore's dev-dependencies. Check `crates/emcore/Cargo.toml`'s `[dev-dependencies]` block; if missing, add:

```toml
[dev-dependencies]
emcore = { path = ".", features = ["test-support"] }
```

(Note: this is a self-referential dev-dep, which Cargo allows specifically for enabling test-only features in integration tests.)

- [ ] **Step 3: Run the new test**

```bash
cargo test --test blink_focus_path
```

Expected: PASS (because the fix from Task 12/13 is in place). If FAIL, the fix is incomplete or the test setup is wrong — investigate before proceeding.

- [ ] **Step 4: Run the full test suite**

```bash
cargo-nextest ntr
```

Expected: PASS — no regressions introduced by the fix.

- [ ] **Step 5: Verify the test would fail without the fix (regression-guard sanity check)**

This is critical: the test must demonstrably catch the bug. Verify by temporarily reverting the fix:

```bash
git stash  # stash the fix
cargo test --test blink_focus_path
```

Expected: FAIL with one of the assertion messages showing the offending field (e.g., `in_active_path was false at FOCUS_CHANGED dispatch ...` for O1, or `window_focused was false ...` for O2).

If the test still PASSES without the fix, the test setup is not exercising the bug — investigate before proceeding (the test is worthless as a regression guard). Possible causes:
- Tree setup doesn't reproduce the offending ordering (e.g., the bug requires a sub-view tree which the simple integration test doesn't model).
- The `view.Update` flush isn't reaching the probe.

If the bug is sub-view-specific (Phase 1b), the simpler test may not reproduce — in that case, the regression guard is the sub-view variant added in Task 15.

Restore the fix:

```bash
git stash pop
cargo test --test blink_focus_path
```

Expected: PASS again.

- [ ] **Step 6: Commit the test**

```bash
git add crates/emcore/tests/blink_focus_path.rs
git commit -m "$(cat <<'EOF'
test(emcore): B2 regression test — focused panel engine wakes and cycles

Locks out the focus-path → engine-wake → Cycle chain bug class
identified in the A2 engine wake observability investigation. Uses an
instrumented BlinkProbe with the same notice/Cycle shape as production
TextFieldPanel, so any structural ordering bug in the focus-path system
is caught regardless of widget. Verified to FAIL when the Phase 1
fix is reverted.

Refs: docs/superpowers/specs/2026-05-03-blink-focus-path-design.md
EOF
)"
```

---

## Task 15: (Phase 1b only) Add sub-view variant test

**ONLY EXECUTE IF Task 11 verdict was O2.**

**Files:**
- Modify: `crates/emcore/tests/blink_focus_path.rs` (append a second test).

- [ ] **Step 1: Append the sub-view variant test**

Append to `crates/emcore/tests/blink_focus_path.rs`:

```rust
#[test]
fn focused_panel_in_subview_engine_wakes_and_cycles() {
    // Sub-view variant: probe lives inside a SubViewPanel, exercising the
    // outer-set_active_panel → SVP::notice → sub_view.SetFocused →
    // sub_view.set_active_panel → sub-view notice flush chain that
    // Phase 1b targeted.
    //
    // Setup:
    //  outer-root (focusable)
    //    └── SubViewPanel ("svp", focusable)
    //         └── sub-root (focusable)
    //              └── probe (focusable)
    //
    // Expected: setting outer-active to SVP and sub-active to probe
    // results in probe.cycle_count_focused > 0 within budget.

    use emcore::emView::emView;

    let mut h = TestViewHarness::new();
    let outer_root = h.tree.create_root_deferred_view("outer-root");
    h.tree.get_mut(outer_root).unwrap().focusable = true;
    h.tree.Layout(outer_root, 0.0, 0.0, 1.0, 1.0, 1.0, None);

    // Construct an emSubViewPanel on outer-root.
    // This requires the actual SubViewPanel constructor; see
    // crates/emcore/src/emSubViewPanel.rs::new for the canonical pattern.
    // <Implementer note: fill this in with the real ctor signature, which
    // requires emWindow context plumbing. If the ctor is too heavy to
    // mock, mark this test #[ignore] with a comment naming the API gap
    // and open a follow-up to expose a lighter-weight test ctor.>

    // ... (SubViewPanel + inner probe setup) ...

    // <If construction is feasible:>
    //   - Create SubViewPanel as outer-root child
    //   - Inside the sub-tree, create a focusable probe
    //   - SetFocused on outer view = true
    //   - set_active_panel(outer_view, svp_id) on outer
    //   - sub_view.SetFocused(true) (or rely on SVP::notice propagation)
    //   - sub_view.set_active_panel(probe_id) on sub
    //   - Update both views; tick scheduler; assert probe focused-cycles > 0.

    // For this test to be useful even if SubViewPanel ctor is heavy:
    // mark #[ignore] and document the API gap; CI will skip but the test
    // is on file as a future regression guard.

    eprintln!("focused_panel_in_subview_engine_wakes_and_cycles: \
               implementer must fill in SubViewPanel construction; see \
               emSubViewPanel.rs::new for ctor signature. If too heavy \
               for this test scope, leave #[ignore] in place.");
}
```

**Implementer guidance:** if SubViewPanel construction in this test is non-trivial (requires wgpu/winit handles, full window setup, etc.), mark the test `#[ignore = "SubViewPanel ctor needs lighter test scaffolding; see follow-up"]` and document the gap in `docs/scratch/2026-05-03-blink-trace-results.md` as a `## Sub-view test scaffolding` section. Phase 1a/1b's primary regression guard is `focused_panel_engine_wakes_and_cycles`, which already covers the bug class generically.

- [ ] **Step 2: Run tests**

```bash
cargo test --test blink_focus_path
```

Expected: both tests PASS, OR the first PASSes and the second is `ignored`. No FAILURES.

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/tests/blink_focus_path.rs
git commit -m "$(cat <<'EOF'
test(emcore): B2 sub-view regression test (Phase 1b)

Adds focused_panel_in_subview_engine_wakes_and_cycles, exercising the
outer-set_active_panel → SVP::notice → sub_view.SetFocused →
sub_view.set_active_panel → sub-view notice flush chain that Phase 1b
targeted. May be #[ignore]'d if SubViewPanel construction is heavier
than this test scope can carry.

Refs: docs/superpowers/specs/2026-05-03-blink-focus-path-design.md
EOF
)"
```

---

## Task 16: Manual recapture to verify BLINK_CYCLE entries appear post-fix

**ONLY EXECUTE IF Task 12 or Task 13 ran.**

**Files:** none (capture only).

- [ ] **Step 1: Build the fix branch**

```bash
git checkout fix/blink-focus-path-2026-05-03
cargo build -p eaglemode --release
```

- [ ] **Step 2: Cherry-pick the fix onto a recapture branch from instr-blink-2026-05-03**

```bash
git checkout -b recapture/blink-fix-2026-05-03 instr-blink-2026-05-03
git cherry-pick <fix-commit-sha>
```

If cherry-pick conflicts (likely if the fix touches code also touched by the instr branch), resolve manually — the instr branch's edits should be preserved alongside the fix.

```bash
cargo build -p eaglemode --release
```

- [ ] **Step 3: Run the same capture procedure as Task 9 with the new log path**

```bash
EM_INSTR_FD=9 cargo run -p eaglemode --release 9>/tmp/em_instr.blink-recap.log
```

ASK USER: open marker → click TextField → wait 30s → close marker → close GUI. Same as Task 9 Steps 4-7.

- [ ] **Step 4: Verify BLINK_CYCLE entries appear**

```bash
grep -c "BLINK_CYCLE" /tmp/em_instr.blink-recap.log
grep "BLINK_CYCLE" /tmp/em_instr.blink-recap.log | head -5
```

Expected: count > 0 (probably tens-to-hundreds for a 30-second focused session). At least one entry should have `focused=t`.

If count is still 0:
- The fix did not actually resolve the chain. Re-run analyzer:
  ```bash
  python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-recap.log
  ```
- If the analyzer reports a different verdict than Task 11, the fix is wrong (or partial). Roll back and re-diagnose.
- If the analyzer reports `O3` now, the chain is correct but Cycle isn't producing BLINK_CYCLE entries — investigate `cycle_blink` / `RestartCursorBlinking` / `InvalidatePainting`. This is a re-scope to B2.1.

- [ ] **Step 5: Run the analyzer and confirm verdict is now O3**

```bash
python3 scripts/analyze_hang.py blink /tmp/em_instr.blink-recap.log | grep "Phase 0 verdict:"
```

Expected: `## Phase 0 verdict: O3` (handler now fires). If the verdict is still O1 or O2, the fix is incomplete.

- [ ] **Step 6: Update the findings doc on main**

```bash
git checkout main
```

Append to `docs/scratch/2026-05-03-blink-trace-results.md`:

```markdown
## Recapture verification (post-fix)

- Capture: `/tmp/em_instr.blink-recap.log`
- Recapture branch: `recapture/blink-fix-2026-05-03`
- BLINK_CYCLE count in window: <N>
- First focused BLINK_CYCLE: +<X> ms after focus
- New verdict: O3 (handler fires correctly)

Fix is verified.
```

```bash
git add docs/scratch/2026-05-03-blink-trace-results.md
git commit -m "scratch: B2 Phase 1 recapture verifies fix; verdict now O3"
git push origin main
```

- [ ] **Step 7: Clean up the recapture branch**

```bash
git branch -D recapture/blink-fix-2026-05-03  # or keep if you want archival
```

---

## Task 17: Merge fix branch to main using finishing-a-development-branch

**ONLY EXECUTE IF Task 12 or Task 13 ran AND Task 16 verified the fix.**

- [ ] **Step 1: Verify all tests pass on the fix branch**

```bash
git checkout fix/blink-focus-path-2026-05-03
cargo-nextest ntr
```

Expected: PASS.

- [ ] **Step 2: Invoke `superpowers:finishing-a-development-branch`**

This skill verifies tests, presents merge options, and executes the chosen path. Run:

```
[invoke superpowers:finishing-a-development-branch]
```

Choose option **1 (Merge locally)** or option **2 (Push and create PR)** based on the user's preference.

- [ ] **Step 3: After merge, push main and verify**

```bash
git checkout main
git pull origin main
git log --oneline -5
```

Expected: top of log shows the merge commit (or fix commits if fast-forward). Push hooks should have run cleanly.

---

## Task 18: Re-brainstorm path (O3 / O3-AMBIG / O4)

**ONLY EXECUTE IF Task 11 verdict was O3, O3-AMBIG, or O4. Skip if any of Tasks 12-17 ran.**

**Files:**
- Modify on main: `docs/scratch/2026-05-03-blink-trace-results.md` (append handoff section).

- [ ] **Step 1: Append handoff section to the findings doc**

```bash
git checkout main
```

Append to `docs/scratch/2026-05-03-blink-trace-results.md`:

```markdown
## Handoff to B2.1 brainstorm

Phase 0 verdict was <O3 / O3-AMBIG / O4>, which means the focus-path
system is NOT the bug. B2 closes here without a fix landed.

### What we learned

- Phase 0 successfully ruled out:
  <list of ruled-out mechanisms based on actual data>

### What remains unknown

- <list of next-investigation areas based on outcome:>
  - O3: bug is in Cycle, cycle_blink, RestartCursorBlinking, or
    InvalidatePainting forwarding. Re-investigate at the Cycle level.
  - O3-AMBIG: impossible-row outcome. Either a #[track_caller]
    instrumentation gap (notice handler fires but WAKE log is missed)
    OR the panel_id filter in the analyzer is wrong. Re-verify
    instrumentation correctness before re-investigating.
  - O4: notice flush is not reaching the textfield. Investigate the
    notice ring linkage and HandleNotice loop.

### Next step

Invoke `superpowers:brainstorming` for B2.1 with this findings doc and
the capture log as input. The brainstorm starts from a much smaller
candidate space than B2 did.
```

- [ ] **Step 2: Commit handoff**

```bash
git add docs/scratch/2026-05-03-blink-trace-results.md
git commit -m "$(cat <<'EOF'
scratch: B2 closes without fix; verdict <OX> redirects to B2.1

Phase 0 successfully ruled out the focus-path mechanism. The remaining
candidate space is smaller and well-articulated for the B2.1
brainstorm.
EOF
)"
git push origin main
```

- [ ] **Step 3: Notify the user**

Tell the user:

> Phase 0 verdict was <OX>. B2 closes without a fix. The findings doc
> has been updated with the handoff. Ready to invoke
> `superpowers:brainstorming` for B2.1 when you are.

---

## Self-review (run AFTER plan is complete)

> The agent executing this plan should run this self-check at the end of execution.

**Spec coverage check:**
- ✓ Phase 0 instrumentation (3 lines) — Tasks 2, 3, 4
- ✓ Phase 0 capture procedure — Task 9
- ✓ Phase 0 verdict (4 outcomes) — Tasks 7, 8
- ✓ O1 fix path — Task 12
- ✓ O2 fix path — Task 13
- ✓ Regression test — Task 14
- ✓ Sub-view variant test (Phase 1b) — Task 15
- ✓ Manual recapture verification — Task 16
- ✓ Merge to main — Task 17
- ✓ O3/O4 re-brainstorm handoff — Task 18
- ✓ D1 deferral note — Task 10
- ✓ Findings doc with calibration — Task 10
- ✓ Branch lifecycle (instr never merges; fix merges; docs to main) — Tasks 1, 11, 17

**Branch / file consistency check:**
- ✓ Phase 0 edits live on `instr/blink-trace-2026-05-03`
- ✓ Phase 1 edits live on `fix/blink-focus-path-2026-05-03`
- ✓ Documentation commits on `main`
- ✓ Recapture happens on a temporary cherry-pick branch (Task 16) without polluting main or fix branch

**Hard rule check (per CLAUDE.md):**
- ✓ Tasks 12 and 13 explicitly require reading C++ source before changing Rust
- ✓ No `--no-verify` on git commits
- ✓ No `#[allow(...)]` for warnings — fix at source
- ✓ Phase 0 instrumentation never merges to main

---

## Plan complete

The plan is self-contained. To execute:

1. **Subagent-Driven (recommended):** Use `superpowers:subagent-driven-development` to dispatch a fresh subagent per task with two-stage review.
2. **Inline Execution:** Use `superpowers:executing-plans` to run tasks in this session with checkpoints.

Tasks 12 and 13 are mutually exclusive (based on Phase 0 verdict). Tasks 14-17 only run if 12 OR 13 ran. Task 18 runs only if neither 12 nor 13 ran. The conditional dispatch is recorded in Task 11 Step 2.
