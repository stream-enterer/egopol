# FFI Harness Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the FFI harness from layer 7 to layers 8-10, mechanically diagnosing root causes for 36 remaining golden test divergences.

**Architecture:** Export Rust rendering functions via C FFI (cdylib), call them from C++ test binaries alongside C++ reference code, compare outputs byte-for-byte. For each layer, export an "intermediates" function (returns computed values) and a "pipeline" function (renders to framebuffer).

**Tech Stack:** Rust cdylib (em-harness crate), C++ test binaries linked against libem_harness.so + libemCore.so, g++ compilation.

**Spec:** `docs/superpowers/specs/2026-04-10-ffi-harness-extension-design.md`

**Working directory:** `.worktrees/harness-prototype/` (branch: `harness-prototype`)

---

## Phase 1: Layer 8 — PaintBorderImage 9-Slice Boundaries

Covers 19+ of 36 failures. The key diagnostic question: does Rust compute the same 9-slice boundary coordinates as C++?

### Task 1: Extract boundary computation into a standalone public function

The boundary computation is currently inline in `PaintBorderImage` (lines 2287-2342 of `crates/emcore/src/emPainter.rs`). Extract it into a testable function that returns the 9 target rects without painting.

**Files:**
- Modify: `crates/emcore/src/emPainter.rs`

- [ ] **Step 1: Define the `BorderImageSlices` struct**

Add this struct above the `PaintBorderImage` method (around line 2230):

```rust
/// The 9 target rectangles computed by PaintBorderImage's boundary logic.
/// Each rect is (x, y, w, h) in logical coordinates, plus corresponding
/// source rect (src_x, src_y, src_w, src_h) in image pixels.
/// Order: UL(0), U(1), UR(2), L(3), C(4), R(5), LL(6), B(7), LR(8).
#[derive(Clone, Debug)]
pub struct BorderImageSlices {
    /// Adjusted insets after RoundX/RoundY pixel-rounding.
    pub adj_l: f64,
    pub adj_t: f64,
    pub adj_r: f64,
    pub adj_b: f64,
    /// 9 target rects: (x, y, w, h) in logical coordinates.
    pub target_rects: [(f64, f64, f64, f64); 9],
    /// 9 source rects: (src_x, src_y, src_w, src_h) in image pixels.
    pub source_rects: [(i32, i32, i32, i32); 9],
}
```

- [ ] **Step 2: Add `compute_border_image_slices` method**

Add this public method on `emPainter`:

```rust
/// Compute 9-slice boundary rects without painting.
/// Returns None if the border image would be skipped (zero alpha, non-positive dimensions).
pub fn compute_border_image_slices(
    &self,
    x: f64, y: f64, w: f64, h: f64,
    mut l: f64, mut t: f64, mut r: f64, mut b: f64,
    img_w: i32, img_h: i32,
    src_l: i32, src_t: i32, src_r: i32, src_b: i32,
    canvas_color: emColor,
) -> Option<BorderImageSlices> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }

    // C++ lines 1903-1908: pixel-round inset boundaries when not opaque.
    if !canvas_color.IsOpaque() {
        let f = self.RoundX(x + l) - x;
        if f > 0.0 && f < w - r { l = f; }
        let f = x + w - self.RoundX(x + w - r);
        if f > 0.0 && f < w - l { r = f; }
        let f = self.RoundY(y + t) - y;
        if f > 0.0 && f < h - b { t = f; }
        let f = y + h - self.RoundY(y + h - b);
        if f > 0.0 && f < h - t { b = f; }
    }

    let src_cx = img_w - src_l - src_r;
    let src_cy = img_h - src_t - src_b;
    let dst_cx = w - l - r;
    let dst_cy = h - t - b;

    // Target rects in C++ order: UL, U, UR, L, C, R, LL, B, LR
    let target_rects = [
        (x,         y,         l,      t),      // UL (bit 8)
        (x + l,     y,         dst_cx, t),      // U  (bit 5)
        (x + w - r, y,         r,      t),      // UR (bit 2)
        (x,         y + t,     l,      dst_cy), // L  (bit 7)
        (x + l,     y + t,     dst_cx, dst_cy), // C  (bit 4)
        (x + w - r, y + t,     r,      dst_cy), // R  (bit 1)
        (x,         y + h - b, l,      b),      // LL (bit 6)
        (x + l,     y + h - b, dst_cx, b),      // B  (bit 3)
        (x + w - r, y + h - b, r,      b),      // LR (bit 0)
    ];

    let source_rects = [
        (0,              0,              src_l,  src_t),  // UL
        (src_l,          0,              src_cx, src_t),  // U
        (img_w - src_r,  0,              src_r,  src_t),  // UR
        (0,              src_t,          src_l,  src_cy), // L
        (src_l,          src_t,          src_cx, src_cy), // C
        (img_w - src_r,  src_t,          src_r,  src_cy), // R
        (0,              img_h - src_b,  src_l,  src_b),  // LL
        (src_l,          img_h - src_b,  src_cx, src_b),  // B
        (img_w - src_r,  img_h - src_b,  src_r,  src_b),  // LR
    ];

    Some(BorderImageSlices {
        adj_l: l,
        adj_t: t,
        adj_r: r,
        adj_b: b,
        target_rects,
        source_rects,
    })
}
```

- [ ] **Step 3: Refactor PaintBorderImage to use compute_border_image_slices**

Replace lines 2277-2342 of `PaintBorderImage` with a call to `compute_border_image_slices`, then iterate the returned rects. The refactored body (after the `try_record` block) becomes:

```rust
        let slices = match self.compute_border_image_slices(
            x, y, w, h, l, t, r, b,
            image.GetWidth() as i32, image.GetHeight() as i32,
            src_l, src_t, src_r, src_b,
            canvas_color,
        ) {
            Some(s) => s,
            None => return,
        };

        let saved_alpha = self.state.alpha;
        let saved_canvas = self.state.canvas_color;
        self.state.canvas_color = canvas_color;
        if alpha < 255 {
            self.state.alpha = ((self.state.alpha as u16 * alpha as u16 + 128) >> 8) as u8;
        }

        let ext = super::emTexture::ImageExtension::Clamp;

        // Bit flags: 8=UL 5=U 2=UR / 7=L 4=C 1=R / 6=LL 3=B 0=LR
        // Slice index to bit mapping:
        let bit_flags: [u16; 9] = [1<<8, 1<<5, 1<<2, 1<<7, 1<<4, 1<<1, 1<<6, 1<<3, 1<<0];
        // Slices 1,3,4,5,7 have conditional guards on dst_cx/dst_cy > 0:
        let dst_cx = slices.target_rects[1].2; // U width = dst_cx
        let dst_cy = slices.target_rects[3].3; // L height = dst_cy
        let needs_cx: [bool; 9] = [false, true, false, false, true, false, false, true, false];
        let needs_cy: [bool; 9] = [false, false, false, true, true, true, false, false, false];

        for i in 0..9 {
            if which_sub_rects & bit_flags[i] == 0 { continue; }
            if needs_cx[i] && dst_cx <= 0.0 { continue; }
            if needs_cy[i] && dst_cy <= 0.0 { continue; }
            let (tx, ty, tw, th) = slices.target_rects[i];
            let (sx, sy, sw, sh) = slices.source_rects[i];
            self.paint_image_rect(proof, tx, ty, tw, th, image, sx, sy, sw, sh, ext);
        }

        self.state.canvas_color = saved_canvas;
        self.state.alpha = saved_alpha;
```

- [ ] **Step 4: Run tests to verify refactoring is behavior-preserving**

Run:
```bash
cargo test --test golden -- --test-threads=1
```
Expected: Same 205 pass / 36 fail as before. No regressions.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emPainter.rs
git commit -m "refactor: extract compute_border_image_slices from PaintBorderImage"
```

---

### Task 2: Add FFI structs and `rust_border_image_boundaries` export

**Files:**
- Modify: `harness/src/lib.rs`

- [ ] **Step 1: Add C-compatible structs**

Add at the top of `harness/src/lib.rs`, after existing struct definitions:

```rust
#[repr(C)]
pub struct CBorderImageParams {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub l: f64,
    pub t: f64,
    pub r: f64,
    pub b: f64,
    pub img_w: i32,
    pub img_h: i32,
    pub src_l: i32,
    pub src_t: i32,
    pub src_r: i32,
    pub src_b: i32,
    pub scale_x: f64,
    pub scale_y: f64,
    pub origin_x: f64,
    pub origin_y: f64,
    pub canvas_r: u8,
    pub canvas_g: u8,
    pub canvas_b: u8,
    pub canvas_a: u8,
}

#[repr(C)]
pub struct CBorderImageBoundaries {
    pub adj_l: f64,
    pub adj_t: f64,
    pub adj_r: f64,
    pub adj_b: f64,
    /// 9 target rects: [i][0]=x, [i][1]=y, [i][2]=w, [i][3]=h
    pub target_rects: [[f64; 4]; 9],
    /// 9 source rects: [i][0]=src_x, [i][1]=src_y, [i][2]=src_w, [i][3]=src_h
    pub source_rects: [[i32; 4]; 9],
}
```

- [ ] **Step 2: Add the `rust_border_image_boundaries` FFI function**

```rust
#[no_mangle]
pub unsafe extern "C" fn rust_border_image_boundaries(
    params: *const CBorderImageParams,
    out: *mut CBorderImageBoundaries,
) -> i32 {
    let p = &*params;

    // Create a minimal 1x1 image just to construct a painter (we won't paint).
    let mut dummy_img = emImage::new(1, 1, 4);
    let mut painter = emPainter::new(&mut dummy_img);

    // Set the transform to match the params.
    painter.scale(p.scale_x, p.scale_y);
    painter.set_offset(p.origin_x, p.origin_y);

    let canvas = emColor::rgba(p.canvas_r, p.canvas_g, p.canvas_b, p.canvas_a);

    match painter.compute_border_image_slices(
        p.x, p.y, p.w, p.h,
        p.l, p.t, p.r, p.b,
        p.img_w, p.img_h,
        p.src_l, p.src_t, p.src_r, p.src_b,
        canvas,
    ) {
        Some(slices) => {
            let o = &mut *out;
            o.adj_l = slices.adj_l;
            o.adj_t = slices.adj_t;
            o.adj_r = slices.adj_r;
            o.adj_b = slices.adj_b;
            for i in 0..9 {
                let (x, y, w, h) = slices.target_rects[i];
                o.target_rects[i] = [x, y, w, h];
                let (sx, sy, sw, sh) = slices.source_rects[i];
                o.source_rects[i] = [sx, sy, sw, sh];
            }
            0 // success
        }
        None => -1, // skipped (zero-size)
    }
}
```

- [ ] **Step 3: Add necessary imports**

Add to the imports section of `harness/src/lib.rs`:

```rust
use emcore::emPainter::{emPainter, BorderImageSlices};
```

Note: `emPainter::new`, `scale`, `set_offset`, and `compute_border_image_slices` must all be `pub`. Verify `scale` and `set_offset` exist and are public. If the method is named differently (e.g., `SetOffset`), use the actual name. Check with:
```bash
grep -n 'pub fn scale\|pub fn set_offset\|pub fn SetOffset\|pub fn translate' crates/emcore/src/emPainter.rs | head -10
```

- [ ] **Step 4: Build the harness**

```bash
cd harness && cargo build 2>&1
```
Expected: Compiles successfully. Fix any visibility or naming issues.

- [ ] **Step 5: Commit**

```bash
git add harness/src/lib.rs
git commit -m "feat(harness): add rust_border_image_boundaries FFI export"
```

---

### Task 3: Add `rust_paint_border_image` full-pipeline FFI export

**Files:**
- Modify: `harness/src/lib.rs`

- [ ] **Step 1: Add the FFI function**

```rust
/// Paint a full border image onto a pre-allocated framebuffer.
/// The framebuffer must be fb_w * fb_h * 4 bytes (RGBA).
#[no_mangle]
pub unsafe extern "C" fn rust_paint_border_image(
    params: *const CBorderImageParams,
    img_data: *const u8,
    img_channels: i32,
    alpha: u8,
    which_sub_rects: u16,
    framebuffer: *mut u8,
    fb_w: i32,
    fb_h: i32,
) -> i32 {
    let p = &*params;

    // Build source image from raw data.
    let mut src_img = emImage::new(p.img_w as u32, p.img_h as u32, img_channels as u32);
    let src_size = (p.img_w * p.img_h * img_channels) as usize;
    let src_slice = std::slice::from_raw_parts(img_data, src_size);
    src_img.GetWritableMap().copy_from_slice(src_slice);

    // Wrap framebuffer as target image.
    let fb_size = (fb_w * fb_h * 4) as usize;
    let fb_slice = std::slice::from_raw_parts_mut(framebuffer, fb_size);
    let mut target_img = emImage::new(fb_w as u32, fb_h as u32, 4);
    target_img.GetWritableMap().copy_from_slice(fb_slice);

    {
        let mut painter = emPainter::new(&mut target_img);
        painter.scale(p.scale_x, p.scale_y);
        painter.set_offset(p.origin_x, p.origin_y);

        let canvas = emColor::rgba(p.canvas_r, p.canvas_g, p.canvas_b, p.canvas_a);

        painter.PaintBorderImage(
            p.x, p.y, p.w, p.h,
            p.l, p.t, p.r, p.b,
            &src_img,
            p.src_l, p.src_t, p.src_r, p.src_b,
            alpha,
            canvas,
            which_sub_rects,
        );
    }

    // Copy result back to caller's framebuffer.
    fb_slice.copy_from_slice(target_img.GetMap());
    0
}
```

- [ ] **Step 2: Build**

```bash
cd harness && cargo build 2>&1
```
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add harness/src/lib.rs
git commit -m "feat(harness): add rust_paint_border_image FFI export"
```

---

### Task 4: Write C++ boundary comparison test

This is a self-contained C++ test (no emCore dependency) that reimplements the ~20-line C++ RoundX/RoundY + boundary computation and compares against the Rust FFI function.

**Files:**
- Create: `harness/test_border_boundaries.cpp`

- [ ] **Step 1: Write the test**

```cpp
// test_border_boundaries.cpp
// Compares C++ PaintBorderImage boundary computation against Rust FFI.
// Self-contained: reimplements RoundX/RoundY + 9-slice formula from
// emPainter.cpp:275-284, 1903-1982. No emCore dependency.

#include <cstdio>
#include <cstring>
#include <cmath>
#include <cstdlib>

// --- C-compatible structs matching harness/src/lib.rs ---

struct CBorderImageParams {
    double x, y, w, h;
    double l, t, r, b;
    int img_w, img_h;
    int src_l, src_t, src_r, src_b;
    double scale_x, scale_y;
    double origin_x, origin_y;
    unsigned char canvas_r, canvas_g, canvas_b, canvas_a;
};

struct CBorderImageBoundaries {
    double adj_l, adj_t, adj_r, adj_b;
    double target_rects[9][4];  // [i][x,y,w,h]
    int source_rects[9][4];     // [i][sx,sy,sw,sh]
};

// --- Rust FFI ---
extern "C" int rust_border_image_boundaries(
    const CBorderImageParams* params,
    CBorderImageBoundaries* out
);

// --- C++ reference: RoundX/RoundY (emPainter.cpp:275-284) ---
static double CppRoundX(double x, double ScaleX, double OriginX) {
    return (floor(x * ScaleX + OriginX + 0.5) - OriginX) / ScaleX;
}

static double CppRoundY(double y, double ScaleY, double OriginY) {
    return (floor(y * ScaleY + OriginY + 0.5) - OriginY) / ScaleY;
}

// --- C++ reference: boundary computation (emPainter.cpp:1892-1982) ---
static void CppBorderImageBoundaries(
    const CBorderImageParams& p,
    CBorderImageBoundaries& out
) {
    double l = p.l, t = p.t, r = p.r, b = p.b;

    // Pixel-round when canvas is non-opaque (emPainter.cpp:1903-1908)
    bool canvas_opaque = (p.canvas_a == 255);
    if (!canvas_opaque) {
        double f;
        f = CppRoundX(p.x + l, p.scale_x, p.origin_x) - p.x;
        if (f > 0 && f < p.w - r) l = f;
        f = p.x + p.w - CppRoundX(p.x + p.w - r, p.scale_x, p.origin_x);
        if (f > 0 && f < p.w - l) r = f;
        f = CppRoundY(p.y + t, p.scale_y, p.origin_y) - p.y;
        if (f > 0 && f < p.h - b) t = f;
        f = p.y + p.h - CppRoundY(p.y + p.h - b, p.scale_y, p.origin_y);
        if (f > 0 && f < p.h - t) b = f;
    }

    out.adj_l = l;
    out.adj_t = t;
    out.adj_r = r;
    out.adj_b = b;

    int src_cx = p.img_w - p.src_l - p.src_r;
    int src_cy = p.img_h - p.src_t - p.src_b;
    double dst_cx = p.w - l - r;
    double dst_cy = p.h - t - b;

    double x = p.x, y = p.y, w = p.w, h = p.h;
    int iw = p.img_w, ih = p.img_h;
    int sl = p.src_l, st = p.src_t, sr = p.src_r, sb = p.src_b;

    // Target rects: UL, U, UR, L, C, R, LL, B, LR
    double tr[9][4] = {
        {x,         y,         l,      t},
        {x + l,     y,         dst_cx, t},
        {x + w - r, y,         r,      t},
        {x,         y + t,     l,      dst_cy},
        {x + l,     y + t,     dst_cx, dst_cy},
        {x + w - r, y + t,     r,      dst_cy},
        {x,         y + h - b, l,      b},
        {x + l,     y + h - b, dst_cx, b},
        {x + w - r, y + h - b, r,      b},
    };
    memcpy(out.target_rects, tr, sizeof(tr));

    // Source rects
    int sr_arr[9][4] = {
        {0,          0,          sl,     st},
        {sl,         0,          src_cx, st},
        {iw - sr,    0,          sr,     st},
        {0,          st,         sl,     src_cy},
        {sl,         st,         src_cx, src_cy},
        {iw - sr,    st,         sr,     src_cy},
        {0,          ih - sb,    sl,     sb},
        {sl,         ih - sb,    src_cx, sb},
        {iw - sr,    ih - sb,    sr,     sb},
    };
    memcpy(out.source_rects, sr_arr, sizeof(sr_arr));
}

// --- Test cases ---

struct TestCase {
    const char* name;
    CBorderImageParams params;
};

static const TestCase cases[] = {
    // Case 1: Checkbox-like (286x286 border image, transparent canvas, 800x600 viewport)
    {
        "checkbox_286x286_transparent",
        {
            // x, y, w, h (logical coords for a checkbox-sized widget)
            0.0, 0.0, 1.0, 0.75,
            // l, t, r, b (insets proportional to 286/286 image)
            286.0/286.0 * 0.3, 286.0/286.0 * 0.3, 286.0/286.0 * 0.3, 286.0/286.0 * 0.3,
            // img_w, img_h
            286, 286,
            // src_l, src_t, src_r, src_b (equal borders for checkbox)
            286, 286, 286, 286,
            // scale_x, scale_y (800px wide)
            800.0, 800.0,
            // origin_x, origin_y
            0.0, 0.0,
            // canvas RGBA (transparent)
            0, 0, 0, 0
        }
    },
    // Case 2: Button-like (asymmetric insets, opaque canvas)
    {
        "button_asymmetric_opaque",
        {
            0.1, 0.05, 0.8, 0.4,
            278.0/264.0 * 0.15, 278.0/264.0 * 0.15, 278.0/264.0 * 0.15, 278.0/264.0 * 0.15,
            920, 920,
            278, 278, 278, 278,
            800.0, 800.0,
            0.0, 0.0,
            255, 255, 255, 255  // opaque white — no rounding
        }
    },
    // Case 3: Splitter-like (non-square, non-transparent canvas)
    {
        "splitter_colored_canvas",
        {
            0.2, 0.1, 0.6, 0.3,
            0.05, 0.05, 0.05, 0.05,
            600, 600,
            150, 150, 149, 149,
            1024.0, 1024.0,
            10.0, 5.0,  // non-zero origin
            128, 128, 128, 200  // semi-transparent
        }
    },
    // Case 4: IO field (asymmetric src insets)
    {
        "iofield_asymmetric_src",
        {
            0.0, 0.0, 1.0, 0.5,
            300.0/216.0 * 0.1, 346.0/216.0 * 0.1, 0.1, 0.1,
            592, 592,
            300, 346, 216, 216,
            800.0, 800.0,
            0.0, 0.0,
            0, 0, 0, 0  // transparent
        }
    },
    // Case 5: Sub-pixel edge case (fractional scale, non-zero origin)
    {
        "subpixel_fractional_scale",
        {
            0.15, 0.08, 0.7, 0.35,
            0.12, 0.12, 0.12, 0.12,
            400, 400,
            100, 100, 100, 100,
            753.0, 753.0,
            3.7, 2.1,
            0, 0, 0, 128  // semi-transparent
        }
    },
};

static const int NUM_CASES = sizeof(cases) / sizeof(cases[0]);

int main() {
    int total_pass = 0, total_fail = 0;

    for (int c = 0; c < NUM_CASES; c++) {
        const TestCase& tc = cases[c];

        CBorderImageBoundaries cpp_out, rust_out;
        memset(&cpp_out, 0, sizeof(cpp_out));
        memset(&rust_out, 0, sizeof(rust_out));

        CppBorderImageBoundaries(tc.params, cpp_out);
        int rc = rust_border_image_boundaries(&tc.params, &rust_out);

        if (rc != 0) {
            printf("FAIL %s: Rust returned %d (skipped)\n", tc.name, rc);
            total_fail++;
            continue;
        }

        bool pass = true;
        const double eps = 1e-12;

        // Compare adjusted insets
        if (fabs(cpp_out.adj_l - rust_out.adj_l) > eps ||
            fabs(cpp_out.adj_t - rust_out.adj_t) > eps ||
            fabs(cpp_out.adj_r - rust_out.adj_r) > eps ||
            fabs(cpp_out.adj_b - rust_out.adj_b) > eps) {
            printf("  DIVERGE %s adjusted insets:\n", tc.name);
            printf("    C++:  l=%.15f t=%.15f r=%.15f b=%.15f\n",
                   cpp_out.adj_l, cpp_out.adj_t, cpp_out.adj_r, cpp_out.adj_b);
            printf("    Rust: l=%.15f t=%.15f r=%.15f b=%.15f\n",
                   rust_out.adj_l, rust_out.adj_t, rust_out.adj_r, rust_out.adj_b);
            pass = false;
        }

        // Compare 9 target rects
        const char* rect_names[] = {"UL","U","UR","L","C","R","LL","B","LR"};
        for (int i = 0; i < 9; i++) {
            for (int j = 0; j < 4; j++) {
                if (fabs(cpp_out.target_rects[i][j] - rust_out.target_rects[i][j]) > eps) {
                    const char* dim[] = {"x","y","w","h"};
                    printf("  DIVERGE %s target_rect[%s].%s: C++=%.15f Rust=%.15f\n",
                           tc.name, rect_names[i], dim[j],
                           cpp_out.target_rects[i][j], rust_out.target_rects[i][j]);
                    pass = false;
                }
            }
        }

        // Compare 9 source rects
        for (int i = 0; i < 9; i++) {
            for (int j = 0; j < 4; j++) {
                if (cpp_out.source_rects[i][j] != rust_out.source_rects[i][j]) {
                    const char* dim[] = {"sx","sy","sw","sh"};
                    printf("  DIVERGE %s source_rect[%s].%s: C++=%d Rust=%d\n",
                           tc.name, rect_names[i], dim[j],
                           cpp_out.source_rects[i][j], rust_out.source_rects[i][j]);
                    pass = false;
                }
            }
        }

        if (pass) {
            printf("PASS %s\n", tc.name);
            total_pass++;
        } else {
            total_fail++;
        }
    }

    printf("\n--- Results: %d PASS, %d FAIL ---\n", total_pass, total_fail);
    return total_fail > 0 ? 1 : 0;
}
```

- [ ] **Step 2: Compile**

```bash
cd harness
g++ -O2 -o test_border_boundaries test_border_boundaries.cpp \
    -L target/debug -lem_harness \
    -Wl,-rpath,"$(pwd)/target/debug"
```
Expected: Compiles without errors.

- [ ] **Step 3: Run**

```bash
cd harness
LD_LIBRARY_PATH=target/debug ./test_border_boundaries
```
Expected: Reports PASS or DIVERGE for each test case. This is diagnostic — any DIVERGE result is the root cause data we need.

- [ ] **Step 4: Commit**

```bash
git add harness/test_border_boundaries.cpp
git commit -m "test(harness): add boundary comparison test for PaintBorderImage 9-slice"
```

---

### Task 5: Write C++ end-to-end pipeline comparison test

This test uses the actual C++ emPainter to render a border image and compares pixel output against the Rust FFI pipeline. Requires linking against libemCore.so.

**Files:**
- Create: `harness/test_border_e2e.cpp`

- [ ] **Step 1: Write the test**

```cpp
// test_border_e2e.cpp
// End-to-end PaintBorderImage comparison: C++ emPainter vs Rust FFI.
// Requires libemCore.so from eaglemode-0.96.4.

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cmath>
#include <emCore/emPainter.h>
#include <emCore/emImage.h>
#include <emCore/emTexture.h>

struct CBorderImageParams {
    double x, y, w, h;
    double l, t, r, b;
    int img_w, img_h;
    int src_l, src_t, src_r, src_b;
    double scale_x, scale_y;
    double origin_x, origin_y;
    unsigned char canvas_r, canvas_g, canvas_b, canvas_a;
};

extern "C" int rust_paint_border_image(
    const CBorderImageParams* params,
    const unsigned char* img_data,
    int img_channels,
    unsigned char alpha,
    unsigned short which_sub_rects,
    unsigned char* framebuffer,
    int fb_w, int fb_h
);

// Minimal TGA loader for uncompressed 32-bit BGRA
// (same pattern used in test_real_tga.cpp)
static bool load_tga_rgba(const char* path, unsigned char*& data, int& w, int& h) {
    FILE* f = fopen(path, "rb");
    if (!f) return false;
    unsigned char header[18];
    if (fread(header, 1, 18, f) != 18) { fclose(f); return false; }
    w = header[12] | (header[13] << 8);
    h = header[14] | (header[15] << 8);
    int bpp = header[16];
    int img_type = header[2];
    if (bpp != 32) { fclose(f); return false; }
    // Skip image ID
    if (header[0] > 0) fseek(f, header[0], SEEK_CUR);

    data = (unsigned char*)malloc(w * h * 4);
    if (img_type == 2) {
        // Uncompressed
        for (int row = h - 1; row >= 0; row--) {
            unsigned char* rowp = data + row * w * 4;
            fread(rowp, 4, w, f);
            // BGRA -> RGBA
            for (int i = 0; i < w; i++) {
                unsigned char tmp = rowp[i*4];
                rowp[i*4] = rowp[i*4+2];
                rowp[i*4+2] = tmp;
            }
        }
    } else if (img_type == 10) {
        // RLE compressed
        int pixels_read = 0;
        int total = w * h;
        unsigned char* buf = (unsigned char*)malloc(total * 4);
        while (pixels_read < total) {
            unsigned char rep;
            fread(&rep, 1, 1, f);
            if (rep & 0x80) {
                int count = (rep & 0x7f) + 1;
                unsigned char px[4];
                fread(px, 4, 1, f);
                for (int i = 0; i < count && pixels_read < total; i++) {
                    memcpy(buf + pixels_read * 4, px, 4);
                    pixels_read++;
                }
            } else {
                int count = rep + 1;
                for (int i = 0; i < count && pixels_read < total; i++) {
                    fread(buf + pixels_read * 4, 4, 1, f);
                    pixels_read++;
                }
            }
        }
        // Flip vertically + BGRA->RGBA
        for (int row = 0; row < h; row++) {
            int src_row = h - 1 - row;
            unsigned char* dst_rowp = data + row * w * 4;
            unsigned char* src_rowp = buf + src_row * w * 4;
            for (int i = 0; i < w; i++) {
                dst_rowp[i*4+0] = src_rowp[i*4+2]; // R
                dst_rowp[i*4+1] = src_rowp[i*4+1]; // G
                dst_rowp[i*4+2] = src_rowp[i*4+0]; // B
                dst_rowp[i*4+3] = src_rowp[i*4+3]; // A
            }
        }
        free(buf);
    } else {
        free(data); data = NULL;
        fclose(f); return false;
    }
    fclose(f);
    return true;
}

// Create a synthetic gradient test image (no TGA dependency)
static void create_test_image(unsigned char* data, int w, int h) {
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int i = (y * w + x) * 4;
            data[i+0] = (unsigned char)(x * 255 / (w-1));  // R: horizontal gradient
            data[i+1] = (unsigned char)(y * 255 / (h-1));  // G: vertical gradient
            data[i+2] = 128;                                 // B: constant
            data[i+3] = 255;                                 // A: opaque
        }
    }
}

struct E2ECase {
    const char* name;
    CBorderImageParams params;
    unsigned char alpha;
    unsigned short which_sub_rects;
    int fb_w, fb_h;
    // If NULL, use synthetic image
    const char* tga_path;
};

int main() {
    // Initialize C++ pixel format hash tables (required by emPainter)
    emPainter::SharedPixelFormat pf;
    memset(&pf, 0, sizeof(pf));

    // Test with synthetic 400x400 image
    int img_w = 400, img_h = 400;
    unsigned char* img_data = (unsigned char*)malloc(img_w * img_h * 4);
    create_test_image(img_data, img_w, img_h);

    E2ECase cases[] = {
        {
            "synthetic_400_transparent_canvas",
            {
                0.0, 0.0, 1.0, 0.75,
                0.25, 0.25, 0.25, 0.25,
                img_w, img_h,
                100, 100, 100, 100,
                200.0, 200.0,
                0.0, 0.0,
                0, 0, 0, 0  // transparent
            },
            255, 0757, 200, 150
        },
        {
            "synthetic_400_opaque_canvas",
            {
                0.0, 0.0, 1.0, 0.75,
                0.25, 0.25, 0.25, 0.25,
                img_w, img_h,
                100, 100, 100, 100,
                200.0, 200.0,
                0.0, 0.0,
                255, 255, 255, 255  // opaque
            },
            255, 0757, 200, 150
        },
    };
    int num_cases = sizeof(cases) / sizeof(cases[0]);

    int total_pass = 0, total_fail = 0;

    for (int c = 0; c < num_cases; c++) {
        E2ECase& tc = cases[c];
        int fb_size = tc.fb_w * tc.fb_h * 4;

        // --- C++ reference ---
        unsigned char* cpp_fb = (unsigned char*)calloc(fb_size, 1);
        // Fill with canvas color
        for (int i = 0; i < tc.fb_w * tc.fb_h; i++) {
            cpp_fb[i*4+0] = tc.params.canvas_r;
            cpp_fb[i*4+1] = tc.params.canvas_g;
            cpp_fb[i*4+2] = tc.params.canvas_b;
            cpp_fb[i*4+3] = tc.params.canvas_a;
        }
        {
            emImage cpp_target;
            cpp_target.Setup(tc.fb_w, tc.fb_h, 4);
            memcpy(cpp_target.GetWritableMap(), cpp_fb, fb_size);

            emImage cpp_src;
            cpp_src.Setup(tc.params.img_w, tc.params.img_h, 4);
            memcpy(cpp_src.GetWritableMap(), img_data, tc.params.img_w * tc.params.img_h * 4);

            emPainter cpp_painter;
            cpp_painter.SetTarget(&cpp_target);
            cpp_painter.SetScaling(tc.params.scale_x, tc.params.scale_y);
            cpp_painter.SetOrigin(tc.params.origin_x, tc.params.origin_y);

            emColor canvas(tc.params.canvas_r, tc.params.canvas_g,
                          tc.params.canvas_b, tc.params.canvas_a);

            cpp_painter.PaintBorderImage(
                tc.params.x, tc.params.y, tc.params.w, tc.params.h,
                tc.params.l, tc.params.t, tc.params.r, tc.params.b,
                cpp_src,
                tc.params.src_l, tc.params.src_t, tc.params.src_r, tc.params.src_b,
                tc.alpha, canvas, tc.which_sub_rects
            );
            memcpy(cpp_fb, cpp_target.GetMap(), fb_size);
        }

        // --- Rust FFI ---
        unsigned char* rust_fb = (unsigned char*)calloc(fb_size, 1);
        for (int i = 0; i < tc.fb_w * tc.fb_h; i++) {
            rust_fb[i*4+0] = tc.params.canvas_r;
            rust_fb[i*4+1] = tc.params.canvas_g;
            rust_fb[i*4+2] = tc.params.canvas_b;
            rust_fb[i*4+3] = tc.params.canvas_a;
        }
        rust_paint_border_image(
            &tc.params, img_data, 4, tc.alpha, tc.which_sub_rects,
            rust_fb, tc.fb_w, tc.fb_h
        );

        // --- Compare ---
        int max_diff = 0;
        int divergent_pixels = 0;
        for (int i = 0; i < fb_size; i++) {
            int d = abs((int)cpp_fb[i] - (int)rust_fb[i]);
            if (d > 0) {
                divergent_pixels++;
                if (d > max_diff) max_diff = d;
            }
        }

        if (divergent_pixels == 0) {
            printf("PASS %s: byte-perfect\n", tc.name);
            total_pass++;
        } else {
            printf("DIVERGE %s: %d divergent bytes, max_diff=%d\n",
                   tc.name, divergent_pixels, max_diff);
            // Dump first 10 divergent pixels
            int shown = 0;
            for (int py = 0; py < tc.fb_h && shown < 10; py++) {
                for (int px = 0; px < tc.fb_w && shown < 10; px++) {
                    int idx = (py * tc.fb_w + px) * 4;
                    bool diff = false;
                    for (int ch = 0; ch < 4; ch++) {
                        if (cpp_fb[idx+ch] != rust_fb[idx+ch]) diff = true;
                    }
                    if (diff) {
                        printf("  pixel(%d,%d): C++=[%d,%d,%d,%d] Rust=[%d,%d,%d,%d]\n",
                               px, py,
                               cpp_fb[idx], cpp_fb[idx+1], cpp_fb[idx+2], cpp_fb[idx+3],
                               rust_fb[idx], rust_fb[idx+1], rust_fb[idx+2], rust_fb[idx+3]);
                        shown++;
                    }
                }
            }
            total_fail++;
        }

        free(cpp_fb);
        free(rust_fb);
    }

    free(img_data);
    printf("\n--- E2E Results: %d PASS, %d FAIL ---\n", total_pass, total_fail);
    return total_fail > 0 ? 1 : 0;
}
```

**Important note for implementor:** The C++ emPainter API in eaglemode-0.96.4 may differ from the API shown above (e.g., `SetTarget`, `SetScaling`, `SetOrigin` may not exist as separate methods — emPainter may be constructed with an emRootContext or emView). Before implementing, read the actual C++ emPainter constructor and initialization code at:
- `/home/a0/git/eaglemode-0.96.4/include/emCore/emPainter.h` — check constructor signature
- `/home/a0/git/eaglemode-0.96.4/src/emCore/emPainter.cpp` — check how ScaleX, ScaleY, OriginX, OriginY are set

The existing test `harness/test_real_tga_e2e.cpp` already does this correctly — follow its pattern for setting up the C++ painter.

- [ ] **Step 2: Compile**

```bash
cd harness
g++ -O2 -o test_border_e2e test_border_e2e.cpp \
    -I /home/a0/git/eaglemode-0.96.4/include \
    -L /home/a0/git/eaglemode-0.96.4/lib -lemCore \
    -L target/debug -lem_harness \
    -Wl,-rpath,"/home/a0/git/eaglemode-0.96.4/lib:$(pwd)/target/debug"
```
Expected: Compiles. If not, adapt the emPainter setup to match the actual C++ API (see note above).

- [ ] **Step 3: Run**

```bash
cd harness
./test_border_e2e
```
Expected: Reports PASS or DIVERGE for each case. DIVERGE results with max_diff and pixel locations are the diagnostic output.

- [ ] **Step 4: Commit**

```bash
git add harness/test_border_e2e.cpp
git commit -m "test(harness): add end-to-end PaintBorderImage comparison test"
```

---

### Task 6: Analyze diagnostic results and document findings

**Files:**
- Modify: `docs/superpowers/specs/2026-04-10-ffi-harness-extension-design.md` (add results section)

- [ ] **Step 1: Run boundary test and capture output**

```bash
cd harness
./test_border_boundaries 2>&1 | tee boundary_results.txt
```

- [ ] **Step 2: Run e2e test and capture output**

```bash
cd harness
./test_border_e2e 2>&1 | tee e2e_results.txt
```

- [ ] **Step 3: Classify each of the 19 Group A/B/D golden tests**

Based on the boundary test results:
- If boundaries DIVERGE: root cause is RoundX/RoundY or boundary formula mismatch
- If boundaries MATCH but e2e DIVERGES: root cause is in paint_image_rect (lower layer)
- If both MATCH: root cause is elsewhere (compositing, alpha, etc.)

Document the classification in the spec file under a new `## Phase 1 Results` section.

- [ ] **Step 4: Commit findings**

```bash
git add docs/superpowers/specs/2026-04-10-ffi-harness-extension-design.md harness/boundary_results.txt harness/e2e_results.txt
git commit -m "docs: document Phase 1 diagnostic results"
```

---

## Phase 2: Layer 9 — Polygon Rasterizer

Covers 6+ of 36 failures. Only start after Phase 1 diagnostic results are documented.

### Task 7: Visibility changes for polygon rasterizer

**Files:**
- Modify: `crates/emcore/src/emPainterScanline.rs`

- [ ] **Step 1: Change `Span` and `rasterize` to `pub`**

In `crates/emcore/src/emPainterScanline.rs`:

Change:
```rust
pub(crate) struct Span {
```
To:
```rust
pub struct Span {
```

Change:
```rust
pub(crate) fn rasterize(
```
To:
```rust
pub fn rasterize(
```

Also make `ClipBounds` and `WindingRule` `pub` if they aren't already:
```bash
grep -n 'pub.crate. struct ClipBounds\|pub.crate. enum WindingRule' crates/emcore/src/emPainterScanline.rs
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emPainterScanline.rs
git commit -m "refactor: make Span, rasterize, ClipBounds, WindingRule pub for FFI harness"
```

---

### Task 8: Add `rust_rasterize_polygon` FFI export

**Files:**
- Modify: `harness/src/lib.rs`

- [ ] **Step 1: Add C-compatible structs**

```rust
#[repr(C)]
pub struct CPolygonVertex {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
pub struct CSpan {
    pub x_start: i32,
    pub x_end: i32,
    pub opacity_beg: i32,
    pub opacity_mid: i32,
    pub opacity_end: i32,
}

#[repr(C)]
pub struct CScanlineSpans {
    pub y: i32,
    pub span_count: i32,
    pub spans: [CSpan; 64],
}
```

- [ ] **Step 2: Add the FFI function**

```rust
use emcore::emPainterScanline::{rasterize, Span, ClipBounds, WindingRule};

#[no_mangle]
pub unsafe extern "C" fn rust_rasterize_polygon(
    vertices: *const CPolygonVertex,
    n_vertices: i32,
    clip_x1: f64,
    clip_y1: f64,
    clip_x2: f64,
    clip_y2: f64,
    winding_rule: i32,  // 0=NonZero, 1=EvenOdd
    out_scanlines: *mut CScanlineSpans,
    max_scanlines: i32,
    out_scanline_count: *mut i32,
) -> i32 {
    let verts: Vec<(f64, f64)> = std::slice::from_raw_parts(vertices, n_vertices as usize)
        .iter()
        .map(|v| (v.x, v.y))
        .collect();

    let clip = ClipBounds {
        x1: clip_x1,
        y1: clip_y1,
        x2: clip_x2,
        y2: clip_y2,
    };

    let rule = if winding_rule == 0 { WindingRule::NonZero } else { WindingRule::EvenOdd };

    let result = rasterize(&verts, clip, rule);

    let out_sl = std::slice::from_raw_parts_mut(out_scanlines, max_scanlines as usize);
    let count = result.len().min(max_scanlines as usize);

    for (i, (y, spans)) in result.iter().enumerate().take(count) {
        out_sl[i].y = *y;
        let sc = spans.len().min(64);
        out_sl[i].span_count = sc as i32;
        for (j, span) in spans.iter().enumerate().take(sc) {
            out_sl[i].spans[j] = CSpan {
                x_start: span.x_start,
                x_end: span.x_end,
                opacity_beg: span.opacity_beg,
                opacity_mid: span.opacity_mid,
                opacity_end: span.opacity_end,
            };
        }
    }
    *out_scanline_count = count as i32;
    0
}
```

**Note:** The struct field names (`ClipBounds`, `WindingRule`) and their exact definitions may differ. Check:
```bash
grep -n 'struct ClipBounds\|enum WindingRule\|pub x1\|pub y1' crates/emcore/src/emPainterScanline.rs | head -20
```
Adapt the code to match actual names.

- [ ] **Step 3: Build**

```bash
cd harness && cargo build 2>&1
```

- [ ] **Step 4: Add `rust_paint_polygon` full-pipeline FFI function**

```rust
/// Paint a polygon onto a pre-allocated RGBA framebuffer.
#[no_mangle]
pub unsafe extern "C" fn rust_paint_polygon(
    vertices: *const CPolygonVertex,
    n_vertices: i32,
    scale_x: f64,
    scale_y: f64,
    origin_x: f64,
    origin_y: f64,
    color_r: u8, color_g: u8, color_b: u8, color_a: u8,
    canvas_r: u8, canvas_g: u8, canvas_b: u8, canvas_a: u8,
    framebuffer: *mut u8,
    fb_w: i32,
    fb_h: i32,
) -> i32 {
    let fb_size = (fb_w * fb_h * 4) as usize;
    let fb_slice = std::slice::from_raw_parts_mut(framebuffer, fb_size);
    let mut target_img = emImage::new(fb_w as u32, fb_h as u32, 4);
    target_img.GetWritableMap().copy_from_slice(fb_slice);

    {
        let mut painter = emPainter::new(&mut target_img);
        painter.scale(scale_x, scale_y);
        painter.set_offset(origin_x, origin_y);

        let color = emColor::rgba(color_r, color_g, color_b, color_a);
        let canvas = emColor::rgba(canvas_r, canvas_g, canvas_b, canvas_a);
        painter.SetCanvasColor(canvas);

        // Build vertex array as flat f64 pairs matching C++ PaintPolygon signature
        let verts = std::slice::from_raw_parts(vertices, n_vertices as usize);
        let xy: Vec<(f64, f64)> = verts.iter().map(|v| (v.x, v.y)).collect();

        painter.paint_polygon(&xy, color);
    }

    fb_slice.copy_from_slice(target_img.GetMap());
    0
}
```

Note: The actual `paint_polygon` method signature may differ. Check:
```bash
grep -n 'pub fn paint_polygon\|pub fn PaintPolygon' crates/emcore/src/emPainter.rs | head -5
```
Adapt to match the actual API (it may take `&[(f64,f64)]` or `&[f64]` flat pairs, and may need a texture argument).

- [ ] **Step 5: Build**

```bash
cd harness && cargo build 2>&1
```

- [ ] **Step 6: Commit**

```bash
git add harness/src/lib.rs
git commit -m "feat(harness): add rust_rasterize_polygon and rust_paint_polygon FFI exports"
```

---

### Task 9: Write C++ polygon span comparison test

**Files:**
- Create: `harness/test_polygon_spans.cpp`

- [ ] **Step 1: Write the test**

```cpp
// test_polygon_spans.cpp
// Compares C++ polygon rasterization spans against Rust FFI.
// Requires libemCore.so for C++ emPainter::PaintPolygon.

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cmath>
#include <vector>
#include <emCore/emPainter.h>

struct CPolygonVertex { double x, y; };
struct CSpan { int x_start, x_end, opacity_beg, opacity_mid, opacity_end; };
struct CScanlineSpans { int y, span_count; CSpan spans[64]; };

extern "C" int rust_rasterize_polygon(
    const CPolygonVertex* vertices, int n_vertices,
    double clip_x1, double clip_y1, double clip_x2, double clip_y2,
    int winding_rule,
    CScanlineSpans* out_scanlines, int max_scanlines,
    int* out_scanline_count
);

// Capture PaintScanline calls from C++ PaintPolygon.
// Strategy: render onto a small framebuffer, then compare the rendered pixels
// rather than trying to intercept internal PaintScanline calls.
// This is more reliable than instrumenting C++ internals.

struct PolygonTestCase {
    const char* name;
    std::vector<double> xy;  // x0,y0, x1,y1, ... (flat pairs, pixel space)
    double clip_x1, clip_y1, clip_x2, clip_y2;
    int fb_w, fb_h;
};

int main() {
    PolygonTestCase cases[] = {
        {
            "unit_square",
            {10.0, 10.0,  40.0, 10.0,  40.0, 40.0,  10.0, 40.0},
            0.0, 0.0, 50.0, 50.0,
            50, 50
        },
        {
            "subpixel_triangle",
            {15.3, 10.7,  35.8, 10.2,  25.1, 38.9},
            0.0, 0.0, 50.0, 50.0,
            50, 50
        },
        {
            "star_5pt",
            {
                25.0, 2.0,   29.0, 18.0,  48.0, 18.0,  33.0, 28.0,
                38.0, 46.0,  25.0, 34.0,  12.0, 46.0,  17.0, 28.0,
                2.0, 18.0,   21.0, 18.0
            },
            0.0, 0.0, 50.0, 50.0,
            50, 50
        },
        {
            "clipped_rect",
            {-5.0, -5.0,  30.0, -5.0,  30.0, 30.0,  -5.0, 30.0},
            0.0, 0.0, 25.0, 25.0,
            25, 25
        },
    };
    int num_cases = sizeof(cases) / sizeof(cases[0]);

    int total_pass = 0, total_fail = 0;

    for (int c = 0; c < num_cases; c++) {
        PolygonTestCase& tc = cases[c];

        // --- Rust: get spans ---
        int n_verts = tc.xy.size() / 2;
        std::vector<CPolygonVertex> verts(n_verts);
        for (int i = 0; i < n_verts; i++) {
            verts[i].x = tc.xy[i*2];
            verts[i].y = tc.xy[i*2+1];
        }

        int max_sl = tc.fb_h + 10;
        CScanlineSpans* rust_sl = new CScanlineSpans[max_sl];
        int rust_sl_count = 0;
        rust_rasterize_polygon(
            verts.data(), n_verts,
            tc.clip_x1, tc.clip_y1, tc.clip_x2, tc.clip_y2,
            0,  // NonZero winding
            rust_sl, max_sl, &rust_sl_count
        );

        // --- C++ reference: render polygon to framebuffer ---
        // We compare rendered pixels rather than internal spans,
        // because intercepting C++ PaintScanline calls requires
        // modifying C++ source. Pixel comparison is the ground truth.
        int fb_size = tc.fb_w * tc.fb_h * 4;
        unsigned char* cpp_fb = (unsigned char*)calloc(fb_size, 1);
        {
            emImage cpp_img;
            cpp_img.Setup(tc.fb_w, tc.fb_h, 4);
            memset(cpp_img.GetWritableMap(), 0, fb_size);

            // Set up emPainter with identity transform (1:1 pixel mapping)
            // Follow the pattern from test_real_tga_e2e.cpp for painter setup.
            // NOTE: Actual emPainter setup may require adaptation — see the
            // existing test_real_tga_e2e.cpp for the correct initialization.
            emPainter cpp_p;
            // ... (adapt from test_real_tga_e2e.cpp pattern)

            emColor color(255, 0, 0, 255);  // solid red
            cpp_p.PaintPolygon(tc.xy.data(), n_verts,
                               emColorTexture(color), emColor(0,0,0,0));
            memcpy(cpp_fb, cpp_img.GetMap(), fb_size);
        }

        // --- Rust: render using spans (blit onto framebuffer) ---
        // For pixel comparison, also render Rust polygon to framebuffer
        // via rust_paint_polygon (Task 10 below).
        // For now, just report the span data.

        printf("TEST %s: %d Rust scanlines\n", tc.name, rust_sl_count);
        for (int i = 0; i < rust_sl_count && i < 5; i++) {
            printf("  y=%d: %d spans", rust_sl[i].y, rust_sl[i].span_count);
            for (int j = 0; j < rust_sl[i].span_count && j < 3; j++) {
                CSpan& s = rust_sl[i].spans[j];
                printf("  [%d..%d beg=%d mid=%d end=%d]",
                       s.x_start, s.x_end, s.opacity_beg, s.opacity_mid, s.opacity_end);
            }
            printf("\n");
        }

        free(cpp_fb);
        delete[] rust_sl;
        total_pass++;  // Informational for now
    }

    printf("\n--- Polygon Span Results: %d cases tested ---\n", num_cases);
    return 0;
}
```

**Note for implementor:** The C++ emPainter setup in this test is incomplete — it depends on how eaglemode-0.96.4's emPainter is constructed. Check `harness/test_real_tga_e2e.cpp` for the actual working pattern and adapt. The emPainter constructor likely takes an emRootContext or requires specific initialization beyond just `emPainter()`.

- [ ] **Step 2: Compile and run**

```bash
cd harness
g++ -O2 -o test_polygon_spans test_polygon_spans.cpp \
    -I /home/a0/git/eaglemode-0.96.4/include \
    -L /home/a0/git/eaglemode-0.96.4/lib -lemCore \
    -L target/debug -lem_harness \
    -Wl,-rpath,"/home/a0/git/eaglemode-0.96.4/lib:$(pwd)/target/debug"
./test_polygon_spans
```

- [ ] **Step 3: Commit**

```bash
git add harness/test_polygon_spans.cpp
git commit -m "test(harness): add polygon rasterizer span comparison test"
```

---

## Phase 3: Layer 10 — Linear Gradient + eagle_logo Rewrite

Only start after Phase 2 diagnostics are complete.

### Task 10: Add `rust_interpolate_linear_gradient` FFI export

**Files:**
- Modify: `harness/src/lib.rs`
- Modify: `crates/emcore/src/emPainterInterpolation.rs` (make `sample_linear_gradient` `pub`)

- [ ] **Step 1: Make sample_linear_gradient pub**

In `crates/emcore/src/emPainterInterpolation.rs`, change:
```rust
pub(crate) fn sample_linear_gradient(
```
To:
```rust
pub fn sample_linear_gradient(
```

- [ ] **Step 2: Add FFI structs and function**

In `harness/src/lib.rs`:

```rust
#[repr(C)]
pub struct CGradientParams {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[no_mangle]
pub unsafe extern "C" fn rust_interpolate_linear_gradient(
    params: *const CGradientParams,
    scanline_x: i32,
    scanline_y: i32,
    width: i32,
    out_buffer: *mut u8,
) -> i32 {
    use emcore::emPainterInterpolation::sample_linear_gradient;
    use emcore::emColor::emColor;

    let p = &*params;
    let buf = std::slice::from_raw_parts_mut(out_buffer, width as usize);

    let start = (p.x1, p.y1);
    let end = (p.x2, p.y2);
    // Use white/black as colors — we only care about the interpolation value (0-255)
    let c0 = emColor::rgba(0, 0, 0, 255);
    let c1 = emColor::rgba(255, 255, 255, 255);

    for i in 0..width as usize {
        let px = scanline_x as f64 + i as f64 + 0.5;
        let py = scanline_y as f64 + 0.5;
        let color = sample_linear_gradient(start, end, c0, c1, (px, py));
        // The R channel gives us the interpolation value 0-255
        buf[i] = color.GetRed();
    }
    0
}
```

- [ ] **Step 3: Build**

```bash
cd harness && cargo build 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emPainterInterpolation.rs harness/src/lib.rs
git commit -m "feat(harness): add rust_interpolate_linear_gradient FFI export"
```

---

### Task 11: Write C++ gradient interpolation comparison test

**Files:**
- Create: `harness/test_gradient_interp.cpp`

- [ ] **Step 1: Write the test**

```cpp
// test_gradient_interp.cpp
// Compares C++ InterpolateLinearGradient against Rust FFI.
// Reimplements C++ 40-bit fixed-point gradient walk (emPainter_ScTlIntGra.cpp:24-39).

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cmath>

struct CGradientParams { double x1, y1, x2, y2; };

extern "C" int rust_interpolate_linear_gradient(
    const CGradientParams* params,
    int scanline_x, int scanline_y, int width,
    unsigned char* out_buffer
);

// C++ reference: InterpolateLinearGradient (emPainter_ScTlIntGra.cpp:24-39)
// Setup from emPainter_ScTl.cpp:155-189
static void CppInterpolateLinearGradient(
    double gx1, double gy1, double gx2, double gy2,  // gradient endpoints (pixel space)
    int scanline_x, int scanline_y, int width,
    unsigned char* buf
) {
    // Setup (emPainter_ScTl.cpp:155-189)
    double nx = gx2 - gx1;
    double ny = gy2 - gy1;
    double nn = nx * nx + ny * ny;

    double f = (nn < 1E-3 ? 0.0 : ((double)((long long)255 << 24)) / nn);
    nx *= f;
    ny *= f;

    // TX uses (gx1-0.5), (gy1-0.5) for pixel-center offset
    double tx_d = (gx1 - 0.5) * nx + (gy1 - 0.5) * ny;
    long long TX = (long long)tx_d - 0x7fffff;
    long long TDX = (long long)nx;
    long long TDY = (long long)ny;

    // Per-scanline walk (emPainter_ScTlIntGra.cpp:24-39)
    long long tdx = TDX;
    long long t = (long long)scanline_x * tdx + (long long)scanline_y * TDY - TX;

    for (int i = 0; i < width; i++) {
        long long u = t >> 24;
        if ((unsigned long long)u > 255) u = ~(u >> 48);
        buf[i] = (unsigned char)u;
        t += tdx;
    }
}

struct GradientTestCase {
    const char* name;
    CGradientParams params;
    int x, y, width;
};

int main() {
    GradientTestCase cases[] = {
        {"horizontal_100", {0.0, 50.0, 100.0, 50.0}, 0, 50, 100},
        {"horizontal_0_row0", {0.0, 50.0, 100.0, 50.0}, 0, 0, 100},
        {"diagonal_100", {0.0, 0.0, 100.0, 100.0}, 0, 50, 100},
        {"vertical_100", {50.0, 0.0, 50.0, 100.0}, 0, 50, 100},
        {"wide_800", {0.0, 300.0, 800.0, 300.0}, 0, 300, 800},
        {"offset_origin", {100.0, 100.0, 700.0, 500.0}, 50, 300, 700},
    };
    int num_cases = sizeof(cases) / sizeof(cases[0]);

    int total_pass = 0, total_fail = 0;

    for (int c = 0; c < num_cases; c++) {
        GradientTestCase& tc = cases[c];

        unsigned char* cpp_buf = new unsigned char[tc.width];
        unsigned char* rust_buf = new unsigned char[tc.width];

        CppInterpolateLinearGradient(
            tc.params.x1, tc.params.y1, tc.params.x2, tc.params.y2,
            tc.x, tc.y, tc.width, cpp_buf
        );

        rust_interpolate_linear_gradient(&tc.params, tc.x, tc.y, tc.width, rust_buf);

        int max_diff = 0, divergent = 0;
        for (int i = 0; i < tc.width; i++) {
            int d = abs((int)cpp_buf[i] - (int)rust_buf[i]);
            if (d > 0) {
                divergent++;
                if (d > max_diff) max_diff = d;
            }
        }

        if (divergent == 0) {
            printf("PASS %s: byte-perfect (%d pixels)\n", tc.name, tc.width);
            total_pass++;
        } else {
            printf("DIVERGE %s: %d/%d divergent, max_diff=%d\n",
                   tc.name, divergent, tc.width, max_diff);
            // Show first 10 divergent pixels
            int shown = 0;
            for (int i = 0; i < tc.width && shown < 10; i++) {
                if (cpp_buf[i] != rust_buf[i]) {
                    printf("  x=%d: C++=%d Rust=%d (diff=%d)\n",
                           tc.x + i, cpp_buf[i], rust_buf[i],
                           (int)rust_buf[i] - (int)cpp_buf[i]);
                    shown++;
                }
            }
            total_fail++;
        }

        delete[] cpp_buf;
        delete[] rust_buf;
    }

    printf("\n--- Gradient Results: %d PASS, %d FAIL ---\n", total_pass, total_fail);
    return total_fail > 0 ? 1 : 0;
}
```

- [ ] **Step 2: Compile and run**

```bash
cd harness
g++ -O2 -o test_gradient_interp test_gradient_interp.cpp \
    -L target/debug -lem_harness \
    -Wl,-rpath,"$(pwd)/target/debug"
./test_gradient_interp
```

- [ ] **Step 3: Commit**

```bash
git add harness/test_gradient_interp.cpp
git commit -m "test(harness): add gradient interpolation comparison test"
```

---

### Task 12: Rewrite eagle_logo golden test

The current eagle_logo test delegates to `panel.Paint()` while the C++ generator manually paints gradient + eagle polygons. Rewrite to match C++ generator structure.

**Files:**
- Modify: `crates/eaglemode/tests/golden/eagle_logo.rs`

- [ ] **Step 1: Read the C++ generator to extract exact paint calls**

Read `/home/a0/git/eaglemode-rs/tests/golden/gen/gen_golden.cpp` function `gen_eagle_logo()` (around line 4630). Note the exact:
- Gradient parameters (x1, y1, color1, x2, y2, color2)
- PaintPolygon vertex arrays for the eagle shape
- Canvas color, fill color, compositing order

- [ ] **Step 2: Rewrite the test to manually call paint functions**

Replace the `panel.Paint()` call with direct calls to `painter.paint_linear_gradient(...)` and `painter.PaintPolygon(...)` matching the C++ generator's sequence. The exact code depends on what `gen_eagle_logo()` does — write it after reading step 1.

- [ ] **Step 3: Run the test**

```bash
cargo test --test golden eagle_logo -- --test-threads=1
```
Expected: Either passes (matching C++ output) or fails with a smaller max_diff than before (175).

- [ ] **Step 4: Commit**

```bash
git add crates/eaglemode/tests/golden/eagle_logo.rs
git commit -m "test: rewrite eagle_logo golden test to match C++ generator structure"
```

---

## Summary of Deliverables

| Phase | Task | Files | Purpose |
|-------|------|-------|---------|
| 1 | Task 1 | emPainter.rs | Extract boundary computation |
| 1 | Task 2 | harness/src/lib.rs | FFI boundary export |
| 1 | Task 3 | harness/src/lib.rs | FFI full pipeline export |
| 1 | Task 4 | test_border_boundaries.cpp | C++ boundary comparison |
| 1 | Task 5 | test_border_e2e.cpp | C++ full pipeline comparison |
| 1 | Task 6 | spec + results | Diagnostic classification |
| 2 | Task 7 | emPainterScanline.rs | Visibility changes |
| 2 | Task 8 | harness/src/lib.rs | FFI polygon span export |
| 2 | Task 9 | test_polygon_spans.cpp | C++ span comparison |
| 3 | Task 10 | harness/src/lib.rs + emPainterInterpolation.rs | FFI gradient export |
| 3 | Task 11 | test_gradient_interp.cpp | C++ gradient comparison |
| 3 | Task 12 | eagle_logo.rs | Test rewrite |
