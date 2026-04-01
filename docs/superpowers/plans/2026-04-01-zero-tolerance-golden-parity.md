# Zero-Tolerance Golden Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** All 241 golden tests pass at `compare_images(name, actual, expected, w, h, 0, 0.0)`.

**Architecture:** Fix the one confirmed regression (gradient hash formula), then run the full suite at zero tolerance and iteratively audit+fix each remaining failure against C++ source. Tolerances are already zeroed (commit `cceddbc`).

**Tech Stack:** Rust, C++ reference at `~/git/eaglemode-0.96.4/`

**Key files:**
- `crates/emcore/src/emPainter.rs` — paint methods, `sample_pixel_texture`, `blit_span_textured`
- `crates/emcore/src/emPainterInterpolation.rs` — `sample_linear_gradient`
- `crates/emcore/src/emColor.rs` — `GetBlended` (do NOT modify)
- `crates/eaglemode/tests/golden/` — all golden test files (tolerances already at 0)
- C++ reference: `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlPSInt.cpp` (hash formula)

---

### Task 1: Restore gradient hash formula in sample_linear_gradient

The C++ gradient rendering pipeline (`emPainter_ScTlPSInt.cpp:297`) uses the hash formula `((c1*a1 + c2*a2) * 257 + 0x8073) >> 16`, NOT `emColor::GetBlended`. The Rust `sample_linear_gradient` calls `GetBlended` which uses a different formula. This was confirmed as a regression from commit `b645fb3`.

**Files:**
- Modify: `crates/emcore/src/emPainterInterpolation.rs:1165-1180`

- [ ] **Step 1: Read the C++ gradient blending formula**

Read `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlPSInt.cpp` lines 280-300 and `emPainter_ScTlIntGra.cpp` lines 24-39. The C++ pipeline:
1. `InterpolateLinearGradient` produces a 1-byte `g` value (0-255) per pixel via integer stepping
2. The paint scanline uses `g` to compute per-channel weighted blend: `((c1R * (255-g) + c2R * g) * 257 + 0x8073) >> 16` (for the 1-channel, 2-color gradient case with full opacity)

Note: The full C++ formula is more complex (it accounts for per-color alpha and opacity), but for the gradient-with-full-opacity path that `sample_linear_gradient` serves, the core is the `(x * 257 + 0x8073) >> 16` rounding.

- [ ] **Step 2: Fix `sample_linear_gradient` to use the hash formula**

In `crates/emcore/src/emPainterInterpolation.rs`, replace the `GetBlended` call with inline hash formula blending:

```rust
pub(crate) fn sample_linear_gradient(
    start: (f64, f64),
    end: (f64, f64),
    c0: emColor,
    c1: emColor,
    point: (f64, f64),
) -> emColor {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 {
        return c0;
    }
    let t = ((point.0 - start.0) * dx + (point.1 - start.1) * dy) / len_sq;
    let g = (t.clamp(0.0, 1.0) * 255.0 + 0.5) as i32;
    let mix = |a: i32, b: i32| -> u8 {
        (((a * (255 - g) + b * g) * 257 + 0x8073) >> 16) as u8
    };
    emColor::rgba(
        mix(c0.GetRed() as i32, c1.GetRed() as i32),
        mix(c0.GetGreen() as i32, c1.GetGreen() as i32),
        mix(c0.GetBlue() as i32, c1.GetBlue() as i32),
        mix(c0.GetAlpha() as i32, c1.GetAlpha() as i32),
    )
}
```

- [ ] **Step 3: Apply same fix to `sample_pixel_texture` LinearGradient arm**

In `crates/emcore/src/emPainter.rs:5696-5710`, the `PixelTexture::LinearGradient` match arm also calls `GetBlended`. Apply the same hash formula:

```rust
PixelTexture::LinearGradient {
    color_a,
    color_b,
    start,
    end,
} => {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return *color_a;
    }
    let t = ((px - start.0) * dx + (py - start.1) * dy) / len_sq;
    let g = (t.clamp(0.0, 1.0) * 255.0 + 0.5) as i32;
    let mix = |a: i32, b: i32| -> u8 {
        (((a * (255 - g) + b * g) * 257 + 0x8073) >> 16) as u8
    };
    emColor::rgba(
        mix(color_a.GetRed() as i32, color_b.GetRed() as i32),
        mix(color_a.GetGreen() as i32, color_b.GetGreen() as i32),
        mix(color_a.GetBlue() as i32, color_b.GetBlue() as i32),
        mix(color_a.GetAlpha() as i32, color_b.GetAlpha() as i32),
    )
}
```

- [ ] **Step 4: Apply same fix to `sample_pixel_texture` RadialGradient arm**

In `crates/emcore/src/emPainter.rs:5711-5741`, the radial gradient also calls `GetBlended`. The `factor` variable is already 0-255 (matching C++ `g`). Replace the `GetBlended` call:

```rust
// factor is 0–255: 0=center (inner), 255=edge (outer).
let g = factor as i32;
let mix = |a: i32, b: i32| -> u8 {
    (((a * (255 - g) + b * g) * 257 + 0x8073) >> 16) as u8
};
emColor::rgba(
    mix(color_inner.GetRed() as i32, color_outer.GetRed() as i32),
    mix(color_inner.GetGreen() as i32, color_outer.GetGreen() as i32),
    mix(color_inner.GetBlue() as i32, color_outer.GetBlue() as i32),
    mix(color_inner.GetAlpha() as i32, color_outer.GetAlpha() as i32),
)
```

- [ ] **Step 5: Run full golden suite, record baseline**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | grep -E 'FAILED|test result:'`

Record the exact pass/fail count. This is the new baseline — it must only improve from here. Expected: eagle_logo, gradient_h, gradient_v, gradient_radial should now pass. Some other gradient-dependent tests may also improve.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emPainterInterpolation.rs crates/emcore/src/emPainter.rs
git commit -m "fix(gradient): restore C++ hash formula ((x*257+0x8073)>>16) for gradient blending

Commit b645fb3 replaced the correct C++ gradient hash formula with
emColor::GetBlended's formula, which uses different rounding. The C++
gradient pipeline (emPainter_ScTlPSInt.cpp:297) uses ((c1*a1+c2*a2)*257+0x8073)>>16,
not GetBlended. Restore this in sample_linear_gradient and sample_pixel_texture
for both linear and radial gradient paths."
```

---

### Task 2: Audit and fix remaining failures

After Task 1, some tests will still fail. This task is iterative — repeat the audit cycle for each remaining failure until the suite is green.

**Methodology per failure** (from the spec):

- [ ] **Step 1: Run the full golden suite and list all remaining failures**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | grep FAILED`

Record the list. This is the work queue.

- [ ] **Step 2: For each failing test, generate diff images**

Run: `DUMP_GOLDEN=1 cargo test --test golden <test_name> -- --test-threads=1`

This produces actual/expected/diff images. Examine the diff to classify:
- **Arithmetic**: Scattered noise, small max_diff (1-2). Look for formula differences.
- **Structural**: Contiguous wrong regions, large max_diff (>50). Look for logic differences.

- [ ] **Step 3: Trace the Rust rendering path**

Starting from the test's `Paint` call, follow the code to the pixel output. Identify which `emPainter` method produces the divergent pixels. Key methods to check:
- `paint_linear_gradient` / `paint_radial_gradient` → `sample_pixel_texture` → blend
- `blit_span_textured` → `sample_pixel_texture` → `blend_pixel_unchecked` / `blend_with_coverage_unchecked`
- `PaintBorderImage` → 9-slice section boundaries → image scaling → blend
- `PaintEllipse` → `ellipse_polygon` → `rasterize` → `blit_span`
- `PaintRoundRect` → `round_rect_polygon` → `rasterize` → `blit_span`

- [ ] **Step 4: Read the corresponding C++ source**

Find the same rendering path in `~/git/eaglemode-0.96.4/src/emCore/`. Key files:
- `emPainter.cpp` — PaintRect, PaintEllipse, PaintRoundRect, PaintBezier, PaintBorderImage
- `emPainter_ScTl.cpp` — scanline tool setup (TX, TDX, TDY, Color1, Color2)
- `emPainter_ScTlPSInt.cpp` — paint scanline with interpolation (hash formula blending)
- `emPainter_ScTlPSCol.cpp` — paint scanline with solid color (canvas color blending)
- `emPainter_ScTlIntGra.cpp` — gradient interpolation
- `emPainter_ScTlIntImg.cpp` — image interpolation

Do not trust Rust code comments about what C++ does. Read the actual C++ source.

- [ ] **Step 5: Compare formulas, identify the difference, and fix**

Port the exact C++ formula. Common differences to look for:
- Rounding: `>>16` vs `/256`, `+0.5` vs truncation, `0x8073` bias
- Precision: 8-bit vs 16-bit weight ranges
- Operation order: multiply-then-shift vs divide
- Missing `+0.5` pixel-center offset
- Different polygon vertex count or trig computation
- Different 9-slice boundary rounding (RoundX/RoundY)

- [ ] **Step 6: Run the FULL golden suite after each fix**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | grep -c FAILED`

The failure count must be <= the previous count. If any previously-passing test regresses, the fix is wrong. Back it out immediately and investigate why.

- [ ] **Step 7: Commit each fix individually**

One commit per root cause fixed. The commit message must name the C++ file and line that was matched. Example:
```
fix(painter): match C++ PaintEllipse vertex count formula (emPainter.cpp:1234)
```

- [ ] **Step 8: Repeat steps 2-7 until 0 failures**

The pass count must increase monotonically. When `grep -c FAILED` returns 0, this task is done.

---

### Task 3: Clean up and verify

- [ ] **Step 1: Remove stale tolerance comments**

Search for comments referencing `ch_tol`, `max_diff`, "known divergence", "design difference", or similar in the golden test files. Remove them — they document a state that no longer exists.

Run: `grep -rn 'ch_tol\|max_diff\|known.*diverge\|design difference\|LSB\|tolerance' crates/eaglemode/tests/golden/*.rs`

Remove or update each match.

- [ ] **Step 2: Run full CI checks**

Run:
```bash
cargo clippy -- -D warnings
cargo-nextest ntr
cargo test --test golden -- --test-threads=1
```

All three must pass clean.

- [ ] **Step 3: Verify divergence log is clean**

Run: `cat target/golden-divergence/divergence.jsonl | grep -v '"max_diff":0' | grep -v '"pass":true' | grep -v '"mismatches":0'`

Expected: no output (every pixel test has max_diff=0).

- [ ] **Step 4: Verify no non-zero tolerances remain**

Run: `grep -n 'compare_images' crates/eaglemode/tests/golden/*.rs | grep -v 'pub fn\|fn compare' | grep -v '0, 0.0'`

Expected: no output.

- [ ] **Step 5: Commit cleanup**

```bash
git add crates/eaglemode/tests/golden/
git commit -m "chore(golden): remove stale tolerance comments, all tests pass at tol=0"
```

---

## Critical Rules for Task 2

These rules override normal development instincts. They exist because of the documented DIV-008/b645fb3 antipattern where a "fix" in one session undid a correct fix from another session.

1. **Full suite after every change.** Not "after every task" — after every change to production code. A fix that passes its target test but regresses another test is not a fix.

2. **Never modify `GetBlended`.** It correctly matches `emColor::GetBlended` in C++. The gradient, image, and compositing paths use different formulas in C++ — do not assume they all go through `GetBlended`.

3. **Read the actual C++ source.** Not Rust comments. Not memory files. Not prior session documentation. The C++ source at `~/git/eaglemode-0.96.4/` is the single source of truth.

4. **Monotonic progress.** Track the failure count. It must never increase. If it does, stop, back out the change, and investigate before proceeding.

5. **One root cause per commit.** If a single formula change fixes 5 tests, that's one commit. If 5 tests need 5 different fixes, that's 5 commits.
