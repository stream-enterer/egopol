// test_gradient_interp.cpp
// Compares C++ 40-bit fixed-point linear gradient walk (emPainter_ScTlIntGra.cpp)
// against Rust sample_linear_gradient via FFI.
// Self-contained: reimplements the C++ gradient setup and walk. No emCore dependency.

#include <cstdio>
#include <cstring>
#include <cmath>
#include <cstdlib>
#include <algorithm>

// --- C-compatible struct matching harness/src/lib.rs ---

struct CGradientParams {
    double x1, y1, x2, y2;
};

// --- Rust FFI ---
extern "C" int rust_interpolate_linear_gradient(
    const CGradientParams* params,
    int scanline_x,
    int scanline_y,
    int width,
    unsigned char* out_buffer
);

// --- C++ reference: gradient setup + walk from emPainter_ScTl.cpp / emPainter_ScTlIntGra.cpp ---

static void cpp_gradient_walk(
    double gx1, double gy1, double gx2, double gy2,
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

// --- Test infrastructure ---

struct TestCase {
    const char* name;
    double gx1, gy1, gx2, gy2;
    int scanline_x, scanline_y;
    int width;
};

static const TestCase cases[] = {
    // Case 1: Horizontal gradient (0,0)->(100,0), y=50, width=100
    {"horizontal_y50", 0.0, 0.0, 100.0, 0.0, 0, 50, 100},
    // Case 2: Horizontal at row 0
    {"horizontal_y0", 0.0, 0.0, 100.0, 0.0, 0, 0, 100},
    // Case 3: Diagonal (0,0)->(100,100), y=50
    {"diagonal_y50", 0.0, 0.0, 100.0, 100.0, 0, 50, 100},
    // Case 4: Vertical (50,0)->(50,100), y=50
    {"vertical_y50", 50.0, 0.0, 50.0, 100.0, 0, 50, 100},
    // Case 5: Wide 800px gradient
    {"wide_800px", 0.0, 0.0, 800.0, 0.0, 0, 0, 800},
    // Case 6: Offset origin gradient
    {"offset_origin", 200.0, 150.0, 600.0, 150.0, 200, 150, 400},
};

static const int NUM_CASES = sizeof(cases) / sizeof(cases[0]);

int main() {
    int total_pass = 0, total_fail = 0;

    for (int c = 0; c < NUM_CASES; c++) {
        const TestCase& tc = cases[c];

        int w = tc.width;
        unsigned char* cpp_buf = new unsigned char[w];
        unsigned char* rust_buf = new unsigned char[w];
        memset(cpp_buf, 0, w);
        memset(rust_buf, 0, w);

        // C++ reference
        cpp_gradient_walk(tc.gx1, tc.gy1, tc.gx2, tc.gy2,
                          tc.scanline_x, tc.scanline_y, w, cpp_buf);

        // Rust FFI
        CGradientParams params = {tc.gx1, tc.gy1, tc.gx2, tc.gy2};
        int rc = rust_interpolate_linear_gradient(
            &params, tc.scanline_x, tc.scanline_y, w, rust_buf);

        if (rc != 0) {
            printf("FAIL %s: Rust returned %d\n", tc.name, rc);
            total_fail++;
            delete[] cpp_buf;
            delete[] rust_buf;
            continue;
        }

        // Compare byte-by-byte
        int divergent_count = 0;
        int max_diff = 0;
        int first_div_idx = -1;
        for (int i = 0; i < w; i++) {
            int diff = abs((int)cpp_buf[i] - (int)rust_buf[i]);
            if (diff > 0) {
                if (first_div_idx < 0) first_div_idx = i;
                divergent_count++;
                if (diff > max_diff) max_diff = diff;
            }
        }

        if (divergent_count == 0) {
            printf("PASS %s (width=%d, byte-exact)\n", tc.name, w);
            total_pass++;
        } else {
            printf("DIVERGE %s: %d/%d pixels differ, max_diff=%d\n",
                   tc.name, divergent_count, w, max_diff);
            // Print first few divergences
            int shown = 0;
            for (int i = 0; i < w && shown < 10; i++) {
                int diff = abs((int)cpp_buf[i] - (int)rust_buf[i]);
                if (diff > 0) {
                    printf("  [%d] cpp=%d rust=%d diff=%d\n",
                           i, cpp_buf[i], rust_buf[i], diff);
                    shown++;
                }
            }
            total_fail++;
        }

        delete[] cpp_buf;
        delete[] rust_buf;
    }

    printf("\n--- Results: %d PASS, %d FAIL ---\n", total_pass, total_fail);
    return total_fail > 0 ? 1 : 0;
}
