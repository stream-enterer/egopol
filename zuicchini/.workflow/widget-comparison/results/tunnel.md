# Tunnel Audit Report

**Date**: 2026-03-18 (Session 2)
**C++ files**: emTunnel.cpp (192 LOC) + emTunnel.h (114 LOC) = 306 LOC
**Rust file**: tunnel.rs (332 LOC)

## Findings: 2 bugs, rest MATCH

### [MEDIUM] Missing invalidation on set_child_tallness and set_depth — **FIXED**
- C++ calls `InvalidatePainting()` + `InvalidateChildrenLayout()` on both setters
- Rust just stores the value — no invalidation
- Harmless if only set at construction. Breaks if changed at runtime.
- **Confidence**: high | **Coverage**: uncovered

### [LOW-MEDIUM] Child canvas color hardcoded to look.bg_color — **FIXED**
- C++ computes canvas color through full border paint pipeline
- Rust uses `self.look.bg_color` — may differ for non-default border types
- Only affects child panel compositing, not tunnel's own render
- **Confidence**: medium | **Coverage**: uncovered

### All other areas: MATCH
- Depth calculation, child rect geometry, tessellation quality, quad strip loop, angle computation, quadrant selection, image pixel sampling — all verified identical formula-by-formula.

## Summary: HIGH FIDELITY port. The tunnel's core rendering is pixel-identical. Only setter invalidation and child canvas color are gaps.
