# paint_image_rect FFI Comparison Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export Rust `paint_image_rect` via FFI and compare against C++ `PaintImage` for identical inputs to mechanically identify where divergence enters.

**Architecture:** Single C++ test binary links against `libemCore.so` (C++ reference) and `libem_harness.so` (Rust cdylib). For each test case, renders the same image rect with both implementations and compares framebuffers byte-by-byte.

**Tech Stack:** Rust cdylib FFI, C++11, `libemCore.so` from `~/git/eaglemode-0.96.4/lib/`

---

### Task 1: Add `rust_paint_image_rect` FFI export

**Files:**
- Modify: `harness/src/lib.rs` (append after line 541)

- [ ] **Step 1: Add the FFI function**

Add after the `rust_paint_border_image` function (line 541):

```rust
// ── Layer 12: Full paint_image_rect pipeline ────────────────────

/// Paint an image rect using the full Rust pipeline.
///
/// `img_data`: source image pixels (`img_w * img_h * img_ch` bytes).
/// `canvas`: target framebuffer RGBA pixels (`canvas_w * canvas_h * 4` bytes), modified in place.
/// `extension`: 0=TILED, 1=EDGE, 2=ZERO, 3=EDGE_OR_ZERO.
///
/// Returns 0 on success.
///
/// # Safety
/// `img_data`: `img_w * img_h * img_ch` readable bytes.
/// `canvas`: `canvas_w * canvas_h * 4` read/write bytes.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn rust_paint_image_rect(
    canvas: *mut u8,
    canvas_w: i32,
    canvas_h: i32,
    scale_x: f64,
    scale_y: f64,
    offset_x: f64,
    offset_y: f64,
    clip_x1: f64,
    clip_y1: f64,
    clip_x2: f64,
    clip_y2: f64,
    img_data: *const u8,
    img_w: i32,
    img_h: i32,
    img_ch: i32,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    src_x: i32,
    src_y: i32,
    src_w: i32,
    src_h: i32,
    alpha: i32,
    canvas_color: u32,
    extension: i32,
) -> i32 {
    let fb_size = (canvas_w * canvas_h * 4) as usize;
    let fb_slice = std::slice::from_raw_parts(canvas, fb_size);
    let mut target = emImage::new(canvas_w as u32, canvas_h as u32, 4);
    target.GetWritableMap()[..fb_size].copy_from_slice(fb_slice);

    let img_size = (img_w * img_h * img_ch) as usize;
    let img_slice = std::slice::from_raw_parts(img_data, img_size);
    let mut source = emImage::new(img_w as u32, img_h as u32, img_ch as u32);
    source.GetWritableMap()[..img_size].copy_from_slice(img_slice);

    let mut painter = emPainter::new(&mut target);
    painter.SetOrigin(offset_x, offset_y);
    painter.SetScaling(scale_x, scale_y);
    // SetClipping works in user coords; we need pixel coords for clip.
    // Directly set pixel-space clip via the state.
    painter.set_pixel_clip(clip_x1, clip_y1, clip_x2, clip_y2);
    painter.set_alpha(alpha as u8);

    let cc = emColor::from_u32(canvas_color);

    let ext = match extension {
        0 => emTexture::ImageExtension::Tiled,
        1 => emTexture::ImageExtension::Edge,
        2 => emTexture::ImageExtension::Zero,
        _ => emTexture::ImageExtension::EdgeOrZero,
    };

    painter.PaintImage(x, y, w, h, &source, src_x, src_y, src_w, src_h, alpha as u8, cc, ext);

    // Copy result back
    let result = target.GetMap();
    let out_slice = std::slice::from_raw_parts_mut(canvas, fb_size);
    out_slice.copy_from_slice(&result[..fb_size]);

    0
}
```

**Note:** This relies on `PaintImage` being public and `set_pixel_clip` / `set_alpha` existing. If `set_pixel_clip` does not exist, we'll need to add a minimal method or use the existing `SetClipping` with appropriate user-space coordinates. Check `emPainter.rs` for available methods and adapt.

- [ ] **Step 2: Verify `PaintImage` signature and available state-setting methods**

Read `crates/emcore/src/emPainter.rs` and check:
- Does `PaintImage` exist with `(x, y, w, h, image, src_x, src_y, src_w, src_h, alpha, canvas_color, ext)` signature?
- Is there a way to set pixel-space clip directly? If not, set clip via user-space: `clip_user_x1 = (clip_x1 - offset_x) / scale_x` etc.
- Is there a `set_alpha` method? If not, check how `state.alpha` is set (it's set by `PaintImage` internally from the alpha parameter).

Adapt the FFI function code as needed based on what methods actually exist.

- [ ] **Step 3: Build and fix compilation**

Run: `cargo build -p em-harness`
Expected: Success (or fix any visibility/signature issues)

- [ ] **Step 4: Commit**

```bash
git add harness/src/lib.rs crates/emcore/src/emPainter.rs
git commit -m "feat(harness): add rust_paint_image_rect FFI export"
```

---

### Task 2: Write C++ comparison test

**Files:**
- Create: `harness/test_paint_image_rect.cpp`

- [ ] **Step 1: Write the test program**

```cpp
// Compare C++ PaintImage vs Rust paint_image_rect for identical inputs.
// Links against libemCore.so (C++ reference) and libem_harness.so (Rust).

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cmath>
#include <cstdint>

// Make private members accessible for direct field setup
#define private public
#define protected public
#include <emCore/emPainter.h>
#include <emCore/emImage.h>
#include <emCore/emTexture.h>
#undef private
#undef protected

// Rust FFI
extern "C" int rust_paint_image_rect(
    uint8_t *canvas, int canvas_w, int canvas_h,
    double scale_x, double scale_y,
    double offset_x, double offset_y,
    double clip_x1, double clip_y1, double clip_x2, double clip_y2,
    const uint8_t *img_data, int img_w, int img_h, int img_ch,
    double x, double y, double w, double h,
    int src_x, int src_y, int src_w, int src_h,
    int alpha, uint32_t canvas_color, int extension
);

static void setup_pixel_format(emPainter::SharedPixelFormat& pf) {
    memset(&pf, 0, sizeof(pf));
    pf.BytesPerPixel = 4;
    pf.RedRange = 255; pf.GreenRange = 255; pf.BlueRange = 255;
    pf.RedShift = 0; pf.GreenShift = 8; pf.BlueShift = 16;
    pf.RedHash = malloc(256 * 256 * 4);
    pf.GreenHash = malloc(256 * 256 * 4);
    pf.BlueHash = malloc(256 * 256 * 4);
    int range = 255;
    for (int ch = 0; ch < 3; ch++) {
        void* hash = ch == 0 ? pf.RedHash : ch == 1 ? pf.GreenHash : pf.BlueHash;
        int shift = ch == 0 ? pf.RedShift : ch == 1 ? pf.GreenShift : pf.BlueShift;
        for (int a1 = 0; a1 < 128; a1++) {
            int c1 = (a1 * range + 127) / 255;
            for (int a2 = 0; a2 < 128; a2++) {
                int c2 = (a2 * range + 127) / 255;
                int c3 = (a1 * a2 * range + 32512) / 65025;
                ((unsigned*)hash)[(a1 << 8) + a2] = c3 << shift;
                ((unsigned*)hash)[(a1 << 8) + (255 - a2)] = (c1 - c3) << shift;
                ((unsigned*)hash)[((255 - a1) << 8) + a2] = (c2 - c3) << shift;
                ((unsigned*)hash)[((255 - a1) << 8) + (255 - a2)] = (range + c3 - c1 - c2) << shift;
            }
        }
    }
    pf.OPFIndex = emPainter::OPFI_8888_0BGR;
}

// Create a synthetic gradient image: pixel (x,y) = RGBA(x*17, y*17, (x+y)*8, 255)
static void fill_gradient(unsigned char* data, int w, int h) {
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * 4;
            data[off+0] = (x * 17) & 0xFF;
            data[off+1] = (y * 17) & 0xFF;
            data[off+2] = ((x + y) * 8) & 0xFF;
            data[off+3] = 255;
        }
    }
}

struct TestCase {
    const char* name;
    int canvas_w, canvas_h;
    int img_w, img_h;
    double scale_x, scale_y, offset_x, offset_y;
    double x, y, w, h;           // user-space dest rect
    int src_x, src_y, src_w, src_h;
    int alpha;
    uint32_t canvas_color;       // packed RGBA
    int extension;               // 0=TILED,1=EDGE,2=ZERO,3=EDGE_OR_ZERO
};

static int compare_buffers(const char* name,
                           const uint8_t* cpp_buf, const uint8_t* rust_buf,
                           int w, int h) {
    int diffs = 0;
    int first_x = -1, first_y = -1;
    uint8_t first_cpp[4], first_rust[4];
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * 4;
            if (memcmp(cpp_buf + off, rust_buf + off, 4) != 0) {
                if (diffs == 0) {
                    first_x = x; first_y = y;
                    memcpy(first_cpp, cpp_buf + off, 4);
                    memcpy(first_rust, rust_buf + off, 4);
                }
                diffs++;
            }
        }
    }
    if (diffs == 0) {
        printf("  [PASS] %s: byte-identical (%dx%d)\n", name, w, h);
    } else {
        printf("  [FAIL] %s: %d divergent pixels out of %d\n", name, diffs, w * h);
        printf("    first divergence at (%d,%d): C++=[%d,%d,%d,%d] Rust=[%d,%d,%d,%d]\n",
               first_x, first_y,
               first_cpp[0], first_cpp[1], first_cpp[2], first_cpp[3],
               first_rust[0], first_rust[1], first_rust[2], first_rust[3]);
    }
    return diffs == 0 ? 0 : 1;
}

int main() {
    static emPainter::SharedPixelFormat pf;
    setup_pixel_format(pf);

    // Test cases: synthetic images with known parameters
    TestCase cases[] = {
        // Case 1: Small upscale (8x8 → 200x200 canvas area)
        {"upscale_8x8", 200, 200, 8, 8,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         255, 0, 1},

        // Case 2: Downscale (32x32 → 10x10 canvas area)
        {"downscale_32x32", 200, 200, 32, 32,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 0.05, 0.05,
         0, 0, 32, 32,
         255, 0, 1},

        // Case 3: 1:1 exact (16x16 → 16x16 pixels)
        {"exact_1to1", 200, 200, 16, 16,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 0.08, 0.08,
         0, 0, 16, 16,
         255, 0, 3},

        // Case 4: Sub-rect upscale with offset (source sub-region)
        {"subrect_upscale", 200, 200, 16, 16,
         200.0, 200.0, 10.0, 10.0,
         0.0, 0.0, 0.5, 0.5,
         2, 2, 8, 8,
         255, 0, 1},

        // Case 5: With canvas color (opaque white) + partial alpha
        {"canvas_color_alpha", 200, 200, 8, 8,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         200, 0xFFFFFFFF, 1},

        // Case 6: Sub-pixel placement (offset causes non-integer pixel boundaries)
        {"subpixel_offset", 200, 200, 16, 16,
         200.0, 200.0, 0.3, 0.7,
         0.0, 0.0, 0.5, 0.5,
         0, 0, 16, 16,
         255, 0, 1},
    };

    int n_cases = sizeof(cases) / sizeof(cases[0]);
    int failures = 0;

    for (int c = 0; c < n_cases; c++) {
        TestCase& tc = cases[c];
        printf("Test: %s\n", tc.name);

        // Create source image
        int img_size = tc.img_w * tc.img_h * 4;
        unsigned char* img_data = (unsigned char*)malloc(img_size);
        fill_gradient(img_data, tc.img_w, tc.img_h);

        int fb_size = tc.canvas_w * tc.canvas_h * 4;

        // --- C++ render ---
        emImage cpp_canvas;
        cpp_canvas.Setup(tc.canvas_w, tc.canvas_h, 4);
        memset((void*)cpp_canvas.GetMap(), 0, fb_size);

        emImage srcImg;
        srcImg.Setup(tc.img_w, tc.img_h, 4);
        memcpy((void*)srcImg.GetMap(), img_data, img_size);

        emPainter p;
        p.Map = (void*)cpp_canvas.GetMap();
        p.BytesPerRow = tc.canvas_w * 4;
        p.PixelFormat = &pf;
        p.ClipX1 = 0; p.ClipY1 = 0;
        p.ClipX2 = tc.canvas_w; p.ClipY2 = tc.canvas_h;
        p.OriginX = tc.offset_x; p.OriginY = tc.offset_y;
        p.ScaleX = tc.scale_x; p.ScaleY = tc.scale_y;
        p.UserSpaceMutex = NULL;
        p.USMLockedByThisThread = NULL;
        // Fake Model pointer (non-null required for some code paths)
        static char fake_model[4096];
        memset(fake_model, 0, sizeof(fake_model));
        void* fm = (void*)fake_model;
        memcpy(&p.Model, &fm, sizeof(void*));

        emColor cc;
        cc = tc.canvas_color;

        emTexture::ExtensionType ext;
        switch(tc.extension) {
            case 0: ext = emTexture::EXTEND_TILED; break;
            case 1: ext = emTexture::EXTEND_EDGE; break;
            case 2: ext = emTexture::EXTEND_ZERO; break;
            default: ext = emTexture::EXTEND_EDGE_OR_ZERO; break;
        }

        p.PaintImage(tc.x, tc.y, tc.w, tc.h, srcImg,
                     tc.src_x, tc.src_y, tc.src_w, tc.src_h,
                     tc.alpha, cc, ext);

        // --- Rust render ---
        unsigned char* rust_canvas = (unsigned char*)calloc(fb_size, 1);

        rust_paint_image_rect(
            rust_canvas, tc.canvas_w, tc.canvas_h,
            tc.scale_x, tc.scale_y, tc.offset_x, tc.offset_y,
            0.0, 0.0, (double)tc.canvas_w, (double)tc.canvas_h,
            img_data, tc.img_w, tc.img_h, 4,
            tc.x, tc.y, tc.w, tc.h,
            tc.src_x, tc.src_y, tc.src_w, tc.src_h,
            tc.alpha, tc.canvas_color, tc.extension
        );

        // --- Compare ---
        failures += compare_buffers(tc.name,
            (const uint8_t*)cpp_canvas.GetMap(), rust_canvas,
            tc.canvas_w, tc.canvas_h);

        free(img_data);
        free(rust_canvas);
        // Clear Model to avoid emCore trying to dereference it on destruction
        void* np = nullptr;
        memcpy(&p.Model, &np, sizeof(void*));
    }

    printf("\nResult: %d/%d passed\n", n_cases - failures, n_cases);
    return failures > 0 ? 1 : 0;
}
```

- [ ] **Step 2: Commit**

```bash
git add harness/test_paint_image_rect.cpp
git commit -m "test(harness): add C++ paint_image_rect comparison test"
```

---

### Task 3: Build and run the comparison test

**Files:**
- No new files; compile and link existing

- [ ] **Step 1: Build Rust harness**

Run: `cargo build -p em-harness`
Expected: Success

- [ ] **Step 2: Compile C++ test**

```bash
g++ -std=c++11 -O2 \
    -I ~/git/eaglemode-0.96.4/include \
    -L ~/git/eaglemode-0.96.4/lib \
    -L target/debug \
    -o harness/test_paint_image_rect \
    harness/test_paint_image_rect.cpp \
    -lemCore -lem_harness \
    -Wl,-rpath,$HOME/git/eaglemode-0.96.4/lib \
    -Wl,-rpath,$(pwd)/target/debug
```

If `#define private public` causes issues with templates or inline functions, try compiling with `-Dprivate=public -Dprotected=public` instead.

If linking fails due to missing symbols, check:
- `nm -D target/debug/libem_harness.so | grep rust_paint_image_rect` — Rust export present?
- `nm -D ~/git/eaglemode-0.96.4/lib/libemCore.so | grep PaintImage` — C++ symbol present?

- [ ] **Step 3: Run the test**

```bash
LD_LIBRARY_PATH=$HOME/git/eaglemode-0.96.4/lib:$(pwd)/target/debug \
    harness/test_paint_image_rect
```

Expected: Mix of PASS/FAIL results identifying exactly which cases diverge.

- [ ] **Step 4: Analyze results**

For each FAIL case:
- Note the first divergent pixel coordinates and values
- Check if the divergence is systematic (all pixels off by same amount) or localized (only edge pixels)
- Localized edge divergence → boundary computation bug
- Systematic interior divergence → interpolation or blend bug
- Single-pixel-offset pattern → off-by-one in loop bounds

Document findings in stdout. Do NOT attempt to fix — just report.

- [ ] **Step 5: Commit test binary to .gitignore if needed**

```bash
echo "harness/test_paint_image_rect" >> .gitignore
git add .gitignore
git commit -m "chore: gitignore compiled harness test binaries"
```

---

### Task 4: Add real TGA test case (GroupBorder.tga)

**Files:**
- Modify: `harness/test_paint_image_rect.cpp`

- [ ] **Step 1: Add TGA loader and real-image test case**

After the synthetic test cases, add a test using the real GroupBorder.tga from Eagle Mode
(the same image used in golden tests for checkbox/button borders). Load it with the minimal
TGA loader from the prototype (`load_tga_rgba`).

Use the same parameters that `widget_checkbox_unchecked` uses. These can be extracted by
adding a temporary `eprintln!` in the Rust `PaintBorderImage` and running the golden test.

This is the most important test case because it matches the exact rendering path of 19+ failing
golden tests.

- [ ] **Step 2: Run and report**

Run the test. Document the TGA test result.

- [ ] **Step 3: Commit**

```bash
git add harness/test_paint_image_rect.cpp
git commit -m "test(harness): add GroupBorder.tga real-image comparison case"
```

---

### Task 5: Diagnose divergence root cause

This task depends on results from Tasks 3-4.

- [ ] **Step 1: If transforms diverge — export and compare transform params**

Add a `rust_get_paint_transforms` FFI export that returns the SubPixelEdges and
AreaSampleTransform/ScaleTransform values for given inputs. Compare against C++ values
computed in the test. This bisects whether the bug is in transform computation vs interpolation.

- [ ] **Step 2: If only edge pixels diverge — compare boundary/coverage values**

The C++ PaintRect clips BEFORE computing sub-pixel edges (line 344). The Rust version
computes sub-pixel edges from unclipped coordinates then clips the loop bounds (line 1200-1228).
If divergence is only at rect edges, this is the likely cause.

- [ ] **Step 3: If interior pixels diverge — compare per-scanline intermediate buffers**

Add FFI exports for the interpolation buffer contents (before blend). Compare the raw
interpolated pixel values for a single scanline between C++ and Rust. This isolates whether
the interpolation or the blend is the source.

- [ ] **Step 4: Document findings and fix**

Once the exact root cause is mechanically identified, fix the Rust code to match C++ behavior.
Run golden tests to verify improvement.

```bash
cargo test --test golden -- --test-threads=1
```

- [ ] **Step 5: Commit fix**

```bash
git add -A
git commit -m "fix(paint_image_rect): [describe the mechanical finding]"
```
