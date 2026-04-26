# F010 hypothesis-category synthesis (v2)

Supersedes `synthesis.md`. v1 is preserved on disk for audit; v2 is the authoritative input to the methodology spec's pre-registration template.

v2 differs from v1 by integrating outputs of the (5) pre-mortem (`premortem.md`). Pre-mortem candidates have been audited against v1's H1–H11 + B1–B8 for distinctness, and either promoted to the hypothesis list (P-tier categories) or extracted to methodology constraints (where the candidate is methodology-level rather than mechanistic).

---

## Provenance

| Source artifact | Frame | Output |
|---|---|---|
| `architectural-grounding.md` | layer-by-layer paint pipeline contracts | Cat A–I (folded into H1–H11 in v1) |
| `differential-constraint.md` | paint-primitive property differential (Y vs X+Z) | Cat 1–4 (folded into H1–H11 in v1) |
| `synthesis.md` (v1) | cross-projection synthesis of the above two | H1–H11 + B1–B8 |
| `premortem.md` | adversarial enumeration outside v1's space (4 methods) | P1–P8 |
| **this file (v2)** | merge of v1 + premortem audit | unified H1–H11 + P1–P5,P7,P8 + B1–B8 + methodology constraint M1 |

The provisional 7-list from my pre-investigation freeform brainstorm is excluded. It was the unrigorous baseline this enumeration was designed to defeat.

---

## Audit of pre-mortem candidates

Each P-candidate was checked for genuine distinctness from H1–H11 + B1–B8 per the discipline rule "categories must be genuinely outside the existing space."

| Candidate | Promoted? | Rationale |
|---|---|---|
| **P1** GPU/atlas resource lifecycle (eviction during record→replay) | Yes | H7 = Send/Sync soundness ≠ lifetime correctness. B4 = whole-tile staleness ≠ sub-tile resource eviction. Triangulated 3 methods. |
| **P2** State-transition idempotency / hot-reload cache invalidation | Yes | B1 = initial parse ≠ post-init drift. B2 = reaches VFS_LOADED ≠ handler-fires-but-no-recompute. B8 = compositor-tile invalidation ≠ data-layer invalidation. Triangulated 2 methods. |
| **P3** Virtual-method / trait-dispatch override missing in Rust port | Yes | B3 = no-paint-at-all ≠ wrong-Paint-arm-reached. B2 = state-machine arm ≠ trait dispatch. Single-method but idiosyncratically high-fit given CLAUDE.md File-and-Name correspondence. |
| **P4** Non-deterministic async-task ordering between paint-prep stages | Yes | B5 = font cache init order only ≠ general async-prep ordering. H7 = soundness ≠ ordering correctness. Triangulated 2 methods. |
| **P5** Build-config-conditional code path (cfg-gated arm) | Yes (low prior) | H11 = `debug_assert!` specifically ≠ general cfg-gating. B6 = runtime env ≠ build-time env. Triangulated 2 methods. |
| **P6** Avoidance-fix bit-rot | **No (extracted to methodology constraint M1)** | Methodology-level, not a root-cause category. The pre-mortem itself flags this. |
| **P7** Multi-pass / multi-target paint composition (intra-frame scratch target) | Yes (tentative) | H5 = tile-composite-against-framebuffer ≠ intra-frame side-target composition. F010 fit speculative — accepted for completeness, low prior. |
| **P8** Coordinate-system rounding asymmetry — zero-area degenerate rects | Yes | Same observable as H1, different mechanism. Critical: would survive an H1 fix. |

---

## Unified hypothesis-category list

Each category cites provenance. Tiers reflect convergence count + mechanistic specificity + coverage of the X+Y+Z pattern.

### Tier 1 — high confidence (multi-projection convergence)

| ID | Short name | Provenance |
|---|---|---|
| **H1** | Recording-mode dispatch hole — `emPainter::Clear` silently dropped | (3) Cat A + (4) Cat 1 (convergence C1) |
| **H2** | Tile pre-fill / `view.background_color` contract | (3) Cat C + (4) Cat 4 (convergence C2) |

### Tier 2 — single-projection or pre-mortem-triangulated, real and bounded

| ID | Short name | Provenance |
|---|---|---|
| **H3** | Render-strategy split — bug fires only in display-list branch | (3) Cat B (asymmetric A1) |
| **H4** | Texture-sampling at replay (font-atlas specific) | (4) Cat 2 (asymmetric A6) |
| **H5** | Tile composite alpha re-blend | (3) Cat D (asymmetric A5) |
| **H6** | DrawList replay state-snapshot equivalence | (3) Cat E (asymmetric A2) |
| **P1** | GPU/atlas resource lifecycle (eviction during record→replay) | premortem M1 + M2 + M4 (3 methods) |
| **P2** | State-transition idempotency / hot-reload cache invalidation | premortem M3 + M4 (2 methods) |
| **P4** | Non-deterministic async-task ordering between paint-prep stages | premortem M1 + M3 (2 methods) |

### Tier 3 — residual concerns, low prior for F010

| ID | Short name | Provenance |
|---|---|---|
| **H7** | Send-Sync soundness for DrawOp `*const emImage` variants | (3) Cat F |
| **H8** | GPU pipeline (sRGB / surface clear / present) | (3) Cat G |
| **H9** | SVP-boundary `IsOpaque` correctness | (3) Cat I |
| **H10** | `canvas_color` snapshot/op-arg disagreement | (4) P15 |
| **H11** | `debug_assert!` compiled out in release / push-pop pairing | (3) Cat H |
| **P3** | Virtual-method / trait-dispatch override missing in Rust port | premortem M3 |
| **P5** | Build-config-conditional code path (general cfg-gating, not just debug_assert) | premortem M1 + M2 |
| **P7** | Multi-pass / multi-target paint composition (intra-frame scratch target) | premortem M1 + M2 (tentative) |
| **P8** | Coordinate-rounding to zero-area degenerate rects | premortem M2 |

### Joint blind spots — categories neither prior frame could surface (input to pre-registration "must consider" list, distinct from H/P categories)

These remain from v1 and were partially attacked by the pre-mortem (P1, P2, P4, P5 surfaced from joint-blind-spot territory). Some are still listed because the pre-mortem reinforced them as worth pre-registering even though they didn't directly become P-categories.

| ID | Short name | Notes after pre-mortem |
|---|---|---|
| **B1** | Theme/runtime-data correctness (`DirContentColor` parses to expected light-grey) | Strengthened: P2 generalizes B1 to "parses-correctly-then-drifts," but B1's "parses-wrong-from-the-start" remains its own hypothesis. |
| **B2** | Panel state machine reaches `VFS_LOADED` in production at the symptomatic zoom | Held — no pre-mortem alternative. |
| **B3** | Paint-not-reached for symptomatic panel (parent cull / viewport clip) | Held; P3 is adjacent but distinct. |
| **B4** | Stale tile from prior frame surviving cache invalidation | Held; P1 is at sub-tile granularity, not whole-tile. |
| **B5** | Font cache initialization order vs. first paint | Subsumed by P4 (general async-prep ordering). Pre-register P4, but B5 remains as a specific, more-easily-tested instance. |
| **B6** | Build-config / GPU-vendor / DPR / environment-only repros | Partially refined by P5 (build-time) — but B6's runtime-environment dimension (GPU vendor, DPR) is still its own concern. |
| **B7** | Recursive paint invocation safety | Held — no pre-mortem alternative. |
| **B8** | Compositor dirty-tile invalidation timing | Held — distinct from P2 (data-layer invalidation). |

### Methodology constraints (extracted from premortem; not hypothesis categories)

| ID | Short name | Constraint |
|---|---|---|
| **M1** | Avoidance-fix bit-rot | The methodology spec MUST forbid "fix by avoiding the broken path" (e.g. forcing per-tile direct mode to dodge the recording-mode hole). Such fixes leave the underlying bug latent and cause symptom recurrence on path re-enablement. Fix-shape rule: the fix must address the broken layer, not bypass it. (Pre-mortem fix-survival scenario S3.) |

---

## Pattern observations across the unified list

These observations are not categories themselves but inform how the methodology spec ranks pre-registered hypotheses for falsification priority:

1. **The Y-pinned-resource vs X+Z-evictable-resource asymmetry (P1)** is a discriminator that complements the recording-mode-vs-direct-mode discriminator (H1). Both fit the X+Z-fail-but-Y-works pattern but predict different falsifying evidence:
   - H1 falsifies if forcing per-tile direct mode does NOT make symptom vanish.
   - P1 falsifies if instrumenting resource-handle lifetimes shows no eviction events between record and replay.
   The methodology should run *both* falsifications, not assume H1's confirmation falsifies P1.

2. **H1 + P8 both produce the same observable (Clear is a no-op) via different mechanisms.** H1 = recording-mode silent return; P8 = zero-area rect after f64→i32 quantization. The mechanical test for H1 (recording painter receives Clear, op stream lacks Clear-equivalent) does not falsify P8 (the rect computation may also be zero in direct mode). Methodology must test both.

3. **P2 + B1 + B8 form an invalidation-correctness cluster.** B1 = initial value wrong. P2 = correct initial value drifts because handler doesn't recompute. B8 = correct value but compositor doesn't dirty the tile to repaint. All three would manifest as "right code but wrong pixels." The methodology should treat them as a sub-cluster requiring a single instrumentation pass to discriminate.

4. **P3 + B2 + B3 form a dispatch-correctness cluster.** P3 = wrong Paint method (trait override missing). B2 = right method but wrong state-machine arm. B3 = method not invoked at all. All three manifest as "Clear is never called from the right place." The methodology should treat them as a sub-cluster.

5. **P4 + B5 + H11 + P5 form an order/config-dependent cluster.** All four would produce environment- or order-dependent reproductions. The methodology should require the failing controlled test to fix order/env explicitly, since otherwise one of these could mask another.

---

## Open questions for the methodology spec (informed by v2)

1. The pre-registration template should require, for each hypothesis, a falsification experiment that does *not* also incidentally falsify the cluster-mates (per observations 1–5 above). Otherwise the methodology can declare "H1 confirmed" while P1, P3, B2, etc. remain live.

2. The methodology should require a **second mechanical test specifically for cluster-discrimination** when two or more hypotheses produce identical observables (H1 vs P8; P2 vs B1 vs B8; P3 vs B2 vs B3).

3. Methodology constraint M1 (avoidance-fix bit-rot) needs operationalization: what *is* the test that distinguishes "fix at the root" from "fix by avoiding"? The fix-spec phase will hit this; the methodology should pre-commit a definition.
