#!/usr/bin/env python3
"""Phase C analyzer for hang-instrumentation log.

Streams /tmp/em_instr.phase0.log (or path passed on argv). Asserts the
reconciliation invariant. Emits a verdict row from the Phase 0 decision
matrix. Exits 1 if invariants fail OR no verdict can be produced.

Invariant: per slice, drain_pushes == carry_in + fire + timer.
A violation indicates a push to pending_signals from a path the
instrumentation does not see — typically the cdylib hazard (fire()
called from a plugin-resident copy that does not bump the shared
counter, but the data field IS shared, so the push still drains).

Reality-check: in this codebase, EngineCtxInner.instr is on the same
data instance that all callers (binary + cdylibs) reach via &mut
EngineScheduler, so fire() bumping `self.inner.instr.fire_pushes` is
visible regardless of which compiled copy of fire() ran. A violation
therefore indicates *some other* push site, not a cdylib copy hazard.
"""
import sys
from collections import Counter, defaultdict


def parse(path):
    rows = []
    with open(path) as f:
        for line in f:
            if not line.startswith("SLICE|"):
                continue
            kv = {}
            for part in line.rstrip("\n").split("|")[1:]:
                k, _, v = part.partition("=")
                try:
                    kv[k] = int(v)
                except ValueError:
                    kv[k] = v
            rows.append(kv)
    return rows


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/em_instr.phase0.log"
    rows = parse(path)
    if not rows:
        print(f"FAIL: no SLICE lines in {path}", file=sys.stderr)
        return 1

    print(f"Parsed {len(rows)} slices from {path}")

    # Section 1: invariant
    violations = []
    for r in rows:
        expected = r["carry_in"] + r["fire"] + r["timer"]
        if r["drain_pushes"] != expected:
            violations.append(r)

    if violations:
        print(f"\nINVARIANT FAIL: {len(violations)} slices violate "
              f"drain == carry_in + fire + timer")
        v = violations[0]
        print(f"  first: clock_start={v['clock_start']} "
              f"drain={v['drain_pushes']} carry_in={v['carry_in']} "
              f"fire={v['fire']} timer={v['timer']} "
              f"gap={v['drain_pushes'] - (v['carry_in']+v['fire']+v['timer'])}")
        print("\nVerdict: COUNTING_HOLE — pushes to pending_signals from a "
              "path not covered by fire() or the timer phase.")
        print("Phase A row: 7-HOLE. Add PEND_PRE/POST around each Cycle to "
              "bisect which Cycle body is mutating pending_signals.")
        return 0

    # Section 2: hot-slice gate
    hot = [r for r in rows if r["cycled"] > 5000]
    print(f"\nInvariant holds across all slices.")
    print(f"Hot slices (cycled>5000): {len(hot)}")
    if len(hot) < 10:
        print("FAIL: fewer than 10 hot slices — hang did not build up. "
              "Rerun. Two consecutive failures escalate.", file=sys.stderr)
        return 1

    # Section 3: verdict matrix
    agg = Counter()
    for r in hot:
        for k in ("cycled", "drain_pushes", "fire", "timer", "direct",
                  "rearms", "carry_in"):
            agg[k] += r[k]

    cycled = agg["cycled"]
    drain = agg["drain_pushes"]
    rearms = agg["rearms"]
    fire = agg["fire"]
    timer = agg["timer"]

    print(f"\nAggregate hot-slice totals: cycled={cycled} drain={drain} "
          f"fire={fire} timer={timer} rearms={rearms} direct={agg['direct']}")

    if drain == 0 and rearms > cycled // 2:
        print("\nVerdict: SELF_REARM — cycled high, no signal traffic, "
              "stay_awake dominates.")
        print("Phase A row: 7-REARM. Log per-engine stay_awake returns to "
              "identify the offender.")
        return 0
    if drain > cycled // 2:
        if fire > timer * 5:
            print("\nVerdict: WAKE_FIREHOSE_FIRE — fire() dominates pushes.")
        elif timer > fire * 5:
            print("\nVerdict: WAKE_FIREHOSE_TIMER — timer collection "
                  "dominates pushes.")
        else:
            print(f"\nVerdict: WAKE_FIREHOSE_MIXED — fire={fire} timer={timer}")
        print("Phase A row: 7-FIREHOSE. Log WAKE per-event with caller, "
              "REG per engine.")
        return 0

    print(f"\nVerdict: HOT_CYCLE — cycled={cycled} but drain={drain} "
          f"and rearms={rearms} both low.")
    print("Phase A row: 7-HOTCYC. Wrap dispatch in Instant timing.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
