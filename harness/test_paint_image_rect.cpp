// Compare C++ PaintImage vs Rust paint_image_rect for identical inputs.
// Links against libemCore.so (C++ reference) and libem_harness.so (Rust).
//
// Build:
//   g++ -std=c++11 -O2 \
//     -I ~/git/eaglemode-0.96.4/include \
//     -L ~/git/eaglemode-0.96.4/lib \
//     -L target/debug \
//     -o harness/test_paint_image_rect \
//     harness/test_paint_image_rect.cpp \
//     -lemCore -lem_harness \
//     -Wl,-rpath,$HOME/git/eaglemode-0.96.4/lib \
//     -Wl,-rpath,target/debug

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cmath>
#include <cstdint>

#include <emCore/emScheduler.h>
#include <emCore/emContext.h>
#include <emCore/emPainter.h>
#include <emCore/emImage.h>
#include <emCore/emTexture.h>

// Rust FFI
extern "C" int rust_paint_image_rect(
    uint8_t *canvas, int canvas_w, int canvas_h,
    double scale_x, double scale_y,
    double offset_x, double offset_y,
    const uint8_t *img_data, int img_w, int img_h, int img_ch,
    double x, double y, double w, double h,
    int src_x, int src_y, int src_w, int src_h,
    int alpha, uint32_t canvas_color, int extension
);

// Synthetic gradient: pixel (x,y) = RGBA(x*17, y*17, (x+y)*8, 255)
static void fill_gradient(unsigned char* data, int w, int h) {
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * 4;
            data[off+0] = (unsigned char)((x * 17) & 0xFF);
            data[off+1] = (unsigned char)((y * 17) & 0xFF);
            data[off+2] = (unsigned char)(((x + y) * 8) & 0xFF);
            data[off+3] = 255;
        }
    }
}

struct TestCase {
    const char* name;
    int canvas_w, canvas_h;
    int img_w, img_h;
    double scale_x, scale_y, offset_x, offset_y;
    double x, y, w, h;
    int src_x, src_y, src_w, src_h;
    int alpha;
    uint32_t canvas_color;
    int extension;  // 0=TILED,1=EDGE,2=ZERO,3=EDGE_OR_ZERO
};

// Compare only RGB channels (C++ pixel format has no alpha hash, so alpha byte is garbage)
static int compare_buffers(const char* name,
                           const uint8_t* cpp_buf, const uint8_t* rust_buf,
                           int w, int h) {
    int diffs = 0;
    int first_x = -1, first_y = -1;
    uint8_t first_cpp[4] = {}, first_rust[4] = {};
    int max_diff = 0;
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * 4;
            // Compare only R,G,B (bytes 0,1,2), skip alpha (byte 3)
            if (memcmp(cpp_buf + off, rust_buf + off, 3) != 0) {
                if (diffs == 0) {
                    first_x = x; first_y = y;
                    memcpy(first_cpp, cpp_buf + off, 4);
                    memcpy(first_rust, rust_buf + off, 4);
                }
                for (int c = 0; c < 3; c++) {
                    int d = abs((int)cpp_buf[off+c] - (int)rust_buf[off+c]);
                    if (d > max_diff) max_diff = d;
                }
                diffs++;
            }
        }
    }
    if (diffs == 0) {
        printf("  [PASS] %s: RGB-identical (%dx%d)\n", name, w, h);
    } else {
        printf("  [FAIL] %s: %d divergent pixels (max_diff=%d) out of %d\n",
               name, diffs, max_diff, w * h);
        printf("    first at (%d,%d): C++=[%d,%d,%d,%d] Rust=[%d,%d,%d,%d]\n",
               first_x, first_y,
               first_cpp[0], first_cpp[1], first_cpp[2], first_cpp[3],
               first_rust[0], first_rust[1], first_rust[2], first_rust[3]);
    }
    return diffs == 0 ? 0 : 1;
}

int main() {
    // Proper emCore initialization
    emStandardScheduler scheduler;
    emRootContext rootContext(scheduler);

    TestCase cases[] = {
        // Case 1: Upscale (8x8 → ~200x200 pixels)
        {"upscale_8x8", 200, 200, 8, 8,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         255, 0, 1},

        // Case 2: Downscale (32x32 → ~10x10 pixels)
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

        // Case 4: Sub-rect upscale with offset
        {"subrect_upscale", 200, 200, 16, 16,
         200.0, 200.0, 10.0, 10.0,
         0.0, 0.0, 0.5, 0.5,
         2, 2, 8, 8,
         255, 0, 1},

        // Case 5: Canvas color (opaque white) + partial alpha
        {"canvas_color_alpha", 200, 200, 8, 8,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         200, 0xFFFFFFFF, 1},

        // Case 6: Sub-pixel placement
        {"subpixel_offset", 200, 200, 16, 16,
         200.0, 200.0, 0.3, 0.7,
         0.0, 0.0, 0.5, 0.5,
         0, 0, 16, 16,
         255, 0, 1},

        // Case 7: Upscale with EXTEND_ZERO (no canvas color)
        {"upscale_ext_zero", 200, 200, 8, 8,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         255, 0, 2},  // EXTEND_ZERO

        // Case 8: Upscale with integer pixel boundaries (no sub-pixel AA)
        {"upscale_aligned", 200, 200, 8, 8,
         25.0, 25.0, 0.0, 0.0,
         0.0, 0.0, 8.0, 8.0,
         0, 0, 8, 8,
         255, 0, 1},

        // Case 9: Upscale with opaque canvas color, full alpha
        {"upscale_opaque_canvas", 200, 200, 8, 8,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         255, 0xFF808080, 1},  // grey canvas

        // Case 10: Upscale 2x (16x16 → 32x32)
        {"upscale_2x", 200, 200, 16, 16,
         100.0, 100.0, 0.0, 0.0,
         0.0, 0.0, 2.0, 2.0,
         0, 0, 16, 16,
         255, 0, 1},
    };

    int n_cases = (int)(sizeof(cases) / sizeof(cases[0]));
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

        // Use proper emPainter constructor with emRootContext
        // C++ pixel format: 4 bytes/pixel, R at byte 0 = mask 0xFF, G at byte 1 = mask 0xFF00, B at byte 2 = mask 0xFF0000
        emPainter painter(
            rootContext,
            (void*)cpp_canvas.GetMap(),
            tc.canvas_w * 4,    // bytesPerRow
            4,                  // bytesPerPixel
            0x000000FF,         // redMask
            0x0000FF00,         // greenMask
            0x00FF0000,         // blueMask
            0.0, 0.0,          // clipX1, clipY1
            (double)tc.canvas_w, (double)tc.canvas_h,  // clipX2, clipY2
            tc.offset_x, tc.offset_y,
            tc.scale_x, tc.scale_y
        );

        emColor cc = (emUInt32)tc.canvas_color;
        emTexture::ExtensionType ext;
        switch(tc.extension) {
            case 0: ext = emTexture::EXTEND_TILED; break;
            case 1: ext = emTexture::EXTEND_EDGE; break;
            case 2: ext = emTexture::EXTEND_ZERO; break;
            default: ext = emTexture::EXTEND_EDGE_OR_ZERO; break;
        }

        painter.PaintImage(tc.x, tc.y, tc.w, tc.h, srcImg,
                           tc.src_x, tc.src_y, tc.src_w, tc.src_h,
                           tc.alpha, cc, ext);

        // --- Rust render ---
        unsigned char* rust_canvas = (unsigned char*)calloc(fb_size, 1);

        rust_paint_image_rect(
            rust_canvas, tc.canvas_w, tc.canvas_h,
            tc.scale_x, tc.scale_y, tc.offset_x, tc.offset_y,
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
    }

    printf("\nResult: %d/%d passed\n", n_cases - failures, n_cases);
    return failures > 0 ? 1 : 0;
}
