# F018 Audit — Compositor Integration Contract Compliance

Audit findings for the contract spec at
`docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md`.
Each section below maps to one open question or contract rule and records the
current Rust port's compliance status, evidence, and notes for the remediation
plan.

## Status legend

- **COMPLIANT** — current code provably satisfies the rule under all scenarios in scope.
- **VIOLATION** — current code provably fails the rule in at least one observable scenario.
- **PARTIAL** — code satisfies some scenarios but provably fails others; both noted.
- **INCONCLUSIVE** — verification deferred to remediation phase (requires test harness, visual check, or downstream open question).

---

## Open Questions

### O.1 — OS-driver canvas color initial value

**Question:** What value does the C++ OS driver pass as `canvasColor` to the top-level `emView::Paint` call? (Spec rule II.1.)

**Investigation:**

**Finding:**

**Implication:**

### O.2 — Rust emPainter canvas-color carrier

**Question:** Does the Rust `emPainter` carry canvas color as a member, and if so where is it set/updated? (Spec rule II.5.)

**Investigation:**

**Finding:**

**Implication:**

### O.3 — Per-tile painter clip rect

**Question:** In the per-tile single-threaded path (`emWindow.rs:668-687`), does the painter's clip rect cover the tile bounds or the viewport bounds? (Spec rule I.3.)

**Investigation:**

**Finding:**

**Implication:**

### O.4 — Recording painter records `Clear`

**Question:** Does the recording painter in `render_parallel_inner` record `Clear` ops, or bypass them? (Spec rule IV.5.)

**Investigation:**

**Finding:**

**Implication:**

### O.5 — Compositor unallocated-tile behavior

**Question:** What does `WgpuCompositor::render_frame` do for tiles that are out of the active grid (resized smaller, or never allocated)? (Spec rules I.2, I.4.)

**Investigation:**

**Finding:**

**Implication:**

---

## Cluster I — Pixel Equivalence

### I.1 — Framebuffer pre-state must not be observable

**Status:**
**Evidence:**
**Notes:**

### I.2 — Tile backing-store init color is not observable

**Status:**
**Evidence:**
**Notes:**

### I.3 — Conditional framebuffer clear must mirror C++

**Status:**
**Evidence:**
**Notes:**

### I.4 — Compositor load-clear color must not be observable

**Status:**
**Evidence:**
**Notes:**

### I.5 — Runtime `view.background_color` changes propagate

**Status:**
**Evidence:**
**Notes:**

---

## Cluster II — Canvas-color Propagation

### II.1 — `view.Paint` receives the OS-driver canvas color

**Status:**
**Evidence:**
**Notes:**

### II.2 — SVP receives the conditionally-updated canvas color

**Status:**
**Evidence:**
**Notes:**

### II.3 — Children receive their own `CanvasColor`

**Status:**
**Evidence:**
**Notes:**

### II.4 — Tile boundaries do not perturb canvas color

**Status:**
**Evidence:**
**Notes:**

### II.5 — `emPainter` is not a canvas-color carrier

**Status:**
**Evidence:**
**Notes:**

---

## Cluster III — Non-opaque Composition

### III.1 — Non-opaque SVP reveals view background

**Status:**
**Evidence:**
**Notes:**

### III.2 — Non-opaque child reveals parent

**Status:**
**Evidence:**
**Notes:**

### III.3 — Opaque-panel skip-clear remains valid under tiles

**Status:**
**Evidence:**
**Notes:**

---

## Cluster IV — Dirty-region Soundness

### IV.1 — `InvalidatePainting` propagates to tile cache and compositor

**Status:**
**Evidence:**
**Notes:**

### IV.2 — Painted region shrinking invalidates the difference

**Status:**
**Evidence:**
**Notes:**

### IV.3 — `IsOpaque` change invalidates SVP-choice path

**Status:**
**Evidence:**
**Notes:**

### IV.4 — All three render strategies obey the dirty contract identically

**Status:**
**Evidence:**
**Notes:**

### IV.5 — Recording-painter ops must record the conditional clear

**Status:**
**Evidence:**
**Notes:**

---

## Cluster V — Acceptance Criteria

### V.1 — F018 repro: `VFS_WAITING`/`VFS_LOADING` background is grey

**Status:**
**Evidence:**
**Notes:**

### V.2 — Background-color change visibly propagates

**Status:**
**Evidence:**
**Notes:**

### V.3 — Strategy parity

**Status:**
**Evidence:**
**Notes:**

### V.4 — Painted-region shrink shows no ghost

**Status:**
**Evidence:**
**Notes:**

### V.5 — Opacity transition rebuilds framebuffer

**Status:**
**Evidence:**
**Notes:**

---

## Summary

(Filled in by Task 36: total compliant / violation / partial / inconclusive counts, ordered list of violations to address, and any newly-discovered acceptance criteria.)
