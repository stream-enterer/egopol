use crate::foundation::Color;

/// C++ MaxInterpolationBytesAtOnce = 1024.
/// Buffer is always 1024 bytes; pixel count = 1024 / channel_count.
pub(crate) const MAX_INTERP_BYTES: usize = 1024;

/// Stack-allocated interpolation buffer matching C++ ScanlineTool's 1024-byte buffer.
/// Holds up to `MAX_INTERP_BYTES / ch` pixels of interpolated source data.
pub(crate) struct InterpolationBuffer {
    data: [u8; MAX_INTERP_BYTES],
    len: usize,
    ch: u8,
}

impl InterpolationBuffer {
    #[inline]
    pub(crate) fn new(ch: u8) -> Self {
        Self {
            data: [0u8; MAX_INTERP_BYTES],
            len: 0,
            ch,
        }
    }

    #[inline]
    pub(crate) fn max_pixels(&self) -> usize {
        MAX_INTERP_BYTES / self.ch as usize
    }

    #[inline]
    pub(crate) fn set_len(&mut self, n: usize) {
        debug_assert!(n <= self.max_pixels());
        self.len = n;
    }

    /// Raw data slice for the first `len` pixels. Length = len * ch bytes.
    #[inline]
    pub(crate) fn raw_data(&self) -> &[u8] {
        &self.data[..self.len * self.ch as usize]
    }

    /// Expand pixel `i` to RGBA for blending. Missing channels filled per C++ convention.
    #[inline]
    pub(crate) fn pixel_rgba(&self, i: usize) -> [u8; 4] {
        let ch = self.ch as usize;
        let off = i * ch;
        match ch {
            1 => {
                let g = self.data[off];
                [g, g, g, 255]
            }
            3 => [self.data[off], self.data[off + 1], self.data[off + 2], 255],
            4 => [
                self.data[off],
                self.data[off + 1],
                self.data[off + 2],
                self.data[off + 3],
            ],
            _ => [0, 0, 0, 0],
        }
    }

    #[inline]
    pub(crate) fn set_pixel(&mut self, i: usize, rgba: [u8; 4]) {
        let ch = self.ch as usize;
        let off = i * ch;
        match ch {
            1 => self.data[off] = rgba[0],
            3 => {
                self.data[off] = rgba[0];
                self.data[off + 1] = rgba[1];
                self.data[off + 2] = rgba[2];
            }
            4 => {
                self.data[off] = rgba[0];
                self.data[off + 1] = rgba[1];
                self.data[off + 2] = rgba[2];
                self.data[off + 3] = rgba[3];
            }
            _ => {}
        }
    }

}

/// Blend mode determined once per paint call. Controls which blend *path*
/// (canvas vs source-over), not the alpha value (which varies per pixel).
pub(crate) enum BlendMode {
    /// Canvas-color compositing (C++ HAVE_CVC). Writes RGB only.
    CanvasBlend { canvas: Color, painter_alpha: u8 },
    /// Standard source-over alpha compositing. Writes RGBA.
    SourceOver { painter_alpha: u8 },
}

impl BlendMode {
    /// Construct from current painter state (after canvas/alpha overrides).
    pub(crate) fn from_state(canvas_color: Color, alpha: u8) -> Self {
        if canvas_color.is_opaque() {
            BlendMode::CanvasBlend {
                canvas: canvas_color,
                painter_alpha: alpha,
            }
        } else {
            BlendMode::SourceOver {
                painter_alpha: alpha,
            }
        }
    }
}

// ── Blend functions ────────────────────────────────────────────────

/// Blend `count` straight-alpha RGBA pixels from `buf` onto destination.
/// Matches `blend_pixel_unchecked` + `blend_with_coverage_unchecked` exactly.
///
/// Coverage application:
/// - cov >= 0x1000: use source alpha as-is
/// - cov > 0: adjusted_alpha = (src_alpha * cov + 0x800) >> 12
/// - cov <= 0: skip pixel
///
/// Then combined_alpha = (adjusted_alpha * painter_alpha + 127) / 255.
pub(crate) fn blend_scanline(
    dest: &mut [u8],
    buf: &InterpolationBuffer,
    count: usize,
    coverages: Option<&[i32]>,
    mode: &BlendMode,
) {
    match mode {
        BlendMode::CanvasBlend {
            canvas,
            painter_alpha,
        } => blend_scanline_canvas(dest, buf, count, coverages, *canvas, *painter_alpha),
        BlendMode::SourceOver { painter_alpha } => {
            blend_scanline_source_over(dest, buf, count, coverages, *painter_alpha)
        }
    }
}

/// Canvas-blend path: writes RGB only (dest alpha unchanged).
fn blend_scanline_canvas(
    dest: &mut [u8],
    buf: &InterpolationBuffer,
    count: usize,
    coverages: Option<&[i32]>,
    canvas: Color,
    painter_alpha: u8,
) {
    for i in 0..count {
        let cov = coverages.map_or(0x1000, |c| c[i]);
        if cov <= 0 {
            continue;
        }

        let src = buf.pixel_rgba(i);
        let src_a = src[3];

        // Apply coverage to source alpha
        let adjusted_a = if cov >= 0x1000 {
            src_a
        } else {
            ((src_a as i32 * cov + 0x800) >> 12).clamp(0, 255) as u8
        };

        // Combine with painter alpha
        let combined_alpha = if painter_alpha == 255 {
            adjusted_a
        } else {
            ((adjusted_a as u32 * painter_alpha as u32 + 127) / 255) as u8
        };

        if combined_alpha == 0 {
            continue;
        }

        let off = i * 4;
        let existing = Color::rgba(dest[off], dest[off + 1], dest[off + 2], dest[off + 3]);
        let src_color = Color::rgba(src[0], src[1], src[2], adjusted_a);
        let result = existing.canvas_blend(src_color, canvas, combined_alpha);
        dest[off] = result.r();
        dest[off + 1] = result.g();
        dest[off + 2] = result.b();
        // Canvas blend: dest alpha unchanged
    }
}

/// Source-over path: writes RGBA.
fn blend_scanline_source_over(
    dest: &mut [u8],
    buf: &InterpolationBuffer,
    count: usize,
    coverages: Option<&[i32]>,
    painter_alpha: u8,
) {
    // AVX2 fast path: full coverage, painter_alpha=255, 4-channel buffer.
    #[cfg(target_arch = "x86_64")]
    if coverages.is_none()
        && painter_alpha == 255
        && buf.ch == 4
        && is_x86_feature_detected!("avx2")
    {
        unsafe {
            super::scanline_avx2::blend_source_over_avx2(dest, buf.raw_data(), count);
        }
        return;
    }

    for i in 0..count {
        let cov = coverages.map_or(0x1000, |c| c[i]);
        if cov <= 0 {
            continue;
        }

        let src = buf.pixel_rgba(i);
        let src_a = src[3];

        // Apply coverage to source alpha
        let adjusted_a = if cov >= 0x1000 {
            src_a
        } else {
            ((src_a as i32 * cov + 0x800) >> 12).clamp(0, 255) as u8
        };

        let ea = if painter_alpha == 255 {
            adjusted_a as u16
        } else {
            ((adjusted_a as u32 * painter_alpha as u32 + 127) / 255) as u16
        };

        if ea == 0 {
            continue;
        }

        let off = i * 4;

        // Opaque fast path
        if ea >= 255 {
            dest[off] = src[0];
            dest[off + 1] = src[1];
            dest[off + 2] = src[2];
            dest[off + 3] = 255;
            continue;
        }

        // Blinn div255: (x * 257 + 0x8073) >> 16
        let alpha = ea as u32;
        let t = (255 - alpha) * 257;
        dest[off] = (((dest[off] as u32 * t + 0x8073) >> 16)
            + ((src[0] as u32 * alpha * 257 + 0x8073) >> 16)) as u8;
        dest[off + 1] = (((dest[off + 1] as u32 * t + 0x8073) >> 16)
            + ((src[1] as u32 * alpha * 257 + 0x8073) >> 16)) as u8;
        dest[off + 2] = (((dest[off + 2] as u32 * t + 0x8073) >> 16)
            + ((src[2] as u32 * alpha * 257 + 0x8073) >> 16)) as u8;
        dest[off + 3] = (((dest[off + 3] as u32 * t + 0x8073) >> 16)
            + ((255u32 * alpha * 257 + 0x8073) >> 16)) as u8;
    }
}

/// Blend `count` premultiplied-alpha RGBA pixels from `buf` onto destination.
/// Matches `blend_pixel_premul_unchecked` + `blend_premul_with_coverage_unchecked`.
///
/// Coverage in premul path: o_eff = (cov * painter_alpha + 127) / 255,
/// then all 4 premul channels scaled by (ch * o_eff + 0x800) >> 12.
pub(crate) fn blend_scanline_premul(
    dest: &mut [u8],
    buf: &InterpolationBuffer,
    count: usize,
    coverages: Option<&[i32]>,
    mode: &BlendMode,
) {
    match mode {
        BlendMode::CanvasBlend {
            canvas,
            painter_alpha,
        } => blend_scanline_premul_canvas(dest, buf, count, coverages, *canvas, *painter_alpha),
        BlendMode::SourceOver { painter_alpha } => {
            blend_scanline_premul_source_over(dest, buf, count, coverages, *painter_alpha)
        }
    }
}

/// Premul canvas-blend path.
fn blend_scanline_premul_canvas(
    dest: &mut [u8],
    buf: &InterpolationBuffer,
    count: usize,
    coverages: Option<&[i32]>,
    canvas: Color,
    painter_alpha: u8,
) {
    for i in 0..count {
        let cov = coverages.map_or(0x1000, |c| c[i]);
        if cov <= 0 {
            continue;
        }

        let src = buf.pixel_rgba(i);

        // Apply coverage + painter_alpha
        let o_eff = if painter_alpha == 255 {
            cov
        } else {
            (cov * painter_alpha as i32 + 127) / 255
        };

        let pm = if o_eff >= 0x1000 {
            src
        } else if o_eff > 0 {
            [
                ((src[0] as i32 * o_eff + 0x800) >> 12) as u8,
                ((src[1] as i32 * o_eff + 0x800) >> 12) as u8,
                ((src[2] as i32 * o_eff + 0x800) >> 12) as u8,
                ((src[3] as i32 * o_eff + 0x800) >> 12) as u8,
            ]
        } else {
            continue;
        };

        let a = pm[3] as u32;
        if a == 0 {
            continue;
        }

        let off = i * 4;
        // Canvas path: (cv * a + 127) / 255 rounding
        let cr = (canvas.r() as u32 * a + 127) / 255;
        let cg = (canvas.g() as u32 * a + 127) / 255;
        let cb = (canvas.b() as u32 * a + 127) / 255;
        dest[off] = (dest[off] as i32 + pm[0] as i32 - cr as i32).clamp(0, 255) as u8;
        dest[off + 1] = (dest[off + 1] as i32 + pm[1] as i32 - cg as i32).clamp(0, 255) as u8;
        dest[off + 2] = (dest[off + 2] as i32 + pm[2] as i32 - cb as i32).clamp(0, 255) as u8;
        // Canvas blend: dest alpha unchanged
    }
}

/// Premul source-over path.
fn blend_scanline_premul_source_over(
    dest: &mut [u8],
    buf: &InterpolationBuffer,
    count: usize,
    coverages: Option<&[i32]>,
    painter_alpha: u8,
) {
    // AVX2 fast path: full coverage, painter_alpha=255, 4-channel buffer.
    #[cfg(target_arch = "x86_64")]
    if coverages.is_none()
        && painter_alpha == 255
        && buf.ch == 4
        && is_x86_feature_detected!("avx2")
    {
        unsafe {
            super::scanline_avx2::blend_premul_source_over_avx2(dest, buf.raw_data(), count);
        }
        return;
    }

    for i in 0..count {
        let cov = coverages.map_or(0x1000, |c| c[i]);
        if cov <= 0 {
            continue;
        }

        let src = buf.pixel_rgba(i);

        let o_eff = if painter_alpha == 255 {
            cov
        } else {
            (cov * painter_alpha as i32 + 127) / 255
        };

        let pm = if o_eff >= 0x1000 {
            src
        } else if o_eff > 0 {
            [
                ((src[0] as i32 * o_eff + 0x800) >> 12) as u8,
                ((src[1] as i32 * o_eff + 0x800) >> 12) as u8,
                ((src[2] as i32 * o_eff + 0x800) >> 12) as u8,
                ((src[3] as i32 * o_eff + 0x800) >> 12) as u8,
            ]
        } else {
            continue;
        };

        let a = pm[3] as u32;
        if a == 0 {
            continue;
        }

        let off = i * 4;

        if a >= 255 {
            dest[off] = pm[0];
            dest[off + 1] = pm[1];
            dest[off + 2] = pm[2];
            dest[off + 3] = 255;
            continue;
        }

        // Blinn div255: (x * 257 + 0x8073) >> 16
        let t = (255 - a) * 257;
        dest[off] = (((dest[off] as u32 * t + 0x8073) >> 16) + pm[0] as u32) as u8;
        dest[off + 1] = (((dest[off + 1] as u32 * t + 0x8073) >> 16) + pm[1] as u32) as u8;
        dest[off + 2] = (((dest[off + 2] as u32 * t + 0x8073) >> 16) + pm[2] as u32) as u8;
        dest[off + 3] = (((dest[off + 3] as u32 * t + 0x8073) >> 16) + a) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::Image;

    /// Build a minimal painter-like setup for testing blend equivalence.
    /// Returns (dest image data as Vec<u8>, target_width).
    fn make_dest(width: u32, height: u32, fill: Color) -> Vec<u8> {
        let mut data = vec![0u8; (width * height * 4) as usize];
        for chunk in data.chunks_exact_mut(4) {
            chunk[0] = fill.r();
            chunk[1] = fill.g();
            chunk[2] = fill.b();
            chunk[3] = fill.a();
        }
        data
    }

    /// Reference per-pixel blend matching blend_pixel_unchecked (source-over).
    fn ref_blend_source_over(dest: &mut [u8], color: Color, painter_alpha: u8) {
        let ca = color.a() as u16;
        let ea = if painter_alpha == 255 {
            ca
        } else {
            ((ca as u32 * painter_alpha as u32 + 127) / 255) as u16
        };
        if ea == 0 {
            return;
        }
        if ea >= 255 {
            dest[0] = color.r();
            dest[1] = color.g();
            dest[2] = color.b();
            dest[3] = 255;
            return;
        }
        let alpha = ea as u32;
        let t = (255 - alpha) * 257;
        dest[0] = (((dest[0] as u32 * t + 0x8073) >> 16)
            + ((color.r() as u32 * alpha * 257 + 0x8073) >> 16)) as u8;
        dest[1] = (((dest[1] as u32 * t + 0x8073) >> 16)
            + ((color.g() as u32 * alpha * 257 + 0x8073) >> 16)) as u8;
        dest[2] = (((dest[2] as u32 * t + 0x8073) >> 16)
            + ((color.b() as u32 * alpha * 257 + 0x8073) >> 16)) as u8;
        dest[3] = (((dest[3] as u32 * t + 0x8073) >> 16)
            + ((255u32 * alpha * 257 + 0x8073) >> 16)) as u8;
    }

    /// Reference per-pixel blend matching blend_pixel_unchecked (canvas).
    fn ref_blend_canvas(dest: &mut [u8], color: Color, canvas: Color, painter_alpha: u8) {
        let combined_alpha = if painter_alpha == 255 {
            color.a()
        } else {
            ((color.a() as u32 * painter_alpha as u32 + 127) / 255) as u8
        };
        if combined_alpha == 0 {
            return;
        }
        let existing = Color::rgba(dest[0], dest[1], dest[2], dest[3]);
        let result = existing.canvas_blend(color, canvas, combined_alpha);
        dest[0] = result.r();
        dest[1] = result.g();
        dest[2] = result.b();
    }

    /// Reference premul blend matching blend_pixel_premul_unchecked (source-over).
    fn ref_blend_premul_source_over(dest: &mut [u8], pm: [u8; 4]) {
        let a = pm[3] as u32;
        if a == 0 {
            return;
        }
        if a >= 255 {
            dest[0] = pm[0];
            dest[1] = pm[1];
            dest[2] = pm[2];
            dest[3] = 255;
            return;
        }
        let t = (255 - a) * 257;
        dest[0] = (((dest[0] as u32 * t + 0x8073) >> 16) + pm[0] as u32) as u8;
        dest[1] = (((dest[1] as u32 * t + 0x8073) >> 16) + pm[1] as u32) as u8;
        dest[2] = (((dest[2] as u32 * t + 0x8073) >> 16) + pm[2] as u32) as u8;
        dest[3] = (((dest[3] as u32 * t + 0x8073) >> 16) + a) as u8;
    }

    /// Reference premul blend matching blend_pixel_premul_unchecked (canvas).
    fn ref_blend_premul_canvas(dest: &mut [u8], pm: [u8; 4], canvas: Color) {
        let a = pm[3] as u32;
        if a == 0 {
            return;
        }
        let cr = (canvas.r() as u32 * a + 127) / 255;
        let cg = (canvas.g() as u32 * a + 127) / 255;
        let cb = (canvas.b() as u32 * a + 127) / 255;
        dest[0] = (dest[0] as i32 + pm[0] as i32 - cr as i32).clamp(0, 255) as u8;
        dest[1] = (dest[1] as i32 + pm[1] as i32 - cg as i32).clamp(0, 255) as u8;
        dest[2] = (dest[2] as i32 + pm[2] as i32 - cb as i32).clamp(0, 255) as u8;
    }

    #[test]
    fn blend_scanline_source_over_matches_perpixel() {
        let colors = [
            Color::rgba(255, 0, 0, 255),   // opaque red
            Color::rgba(0, 255, 0, 128),    // semi-transparent green
            Color::rgba(0, 0, 255, 0),      // fully transparent
            Color::rgba(200, 100, 50, 200), // arbitrary
        ];
        let bg = Color::rgba(100, 100, 100, 200);
        let painter_alpha = 200u8;

        // Fill buffer
        let mut buf = InterpolationBuffer::new(4);
        for (i, c) in colors.iter().enumerate() {
            buf.set_pixel(i, [c.r(), c.g(), c.b(), c.a()]);
        }
        buf.set_len(colors.len());

        // Scanline blend
        let mut dest_scan = make_dest(colors.len() as u32, 1, bg);
        let mode = BlendMode::SourceOver { painter_alpha };
        blend_scanline(&mut dest_scan, &buf, colors.len(), None, &mode);

        // Reference per-pixel blend
        let mut dest_ref = make_dest(colors.len() as u32, 1, bg);
        for (i, c) in colors.iter().enumerate() {
            ref_blend_source_over(&mut dest_ref[i * 4..(i + 1) * 4], *c, painter_alpha);
        }

        assert_eq!(dest_scan, dest_ref, "source-over scanline vs per-pixel mismatch");
    }

    #[test]
    fn blend_scanline_canvas_matches_perpixel() {
        let colors = [
            Color::rgba(255, 0, 0, 255),
            Color::rgba(0, 255, 0, 128),
            Color::rgba(0, 0, 255, 0),
            Color::rgba(200, 100, 50, 200),
        ];
        let bg = Color::rgba(100, 100, 100, 200);
        let canvas = Color::rgba(50, 50, 50, 255);
        let painter_alpha = 200u8;

        let mut buf = InterpolationBuffer::new(4);
        for (i, c) in colors.iter().enumerate() {
            buf.set_pixel(i, [c.r(), c.g(), c.b(), c.a()]);
        }
        buf.set_len(colors.len());

        let mut dest_scan = make_dest(colors.len() as u32, 1, bg);
        let mode = BlendMode::CanvasBlend {
            canvas,
            painter_alpha,
        };
        blend_scanline(&mut dest_scan, &buf, colors.len(), None, &mode);

        let mut dest_ref = make_dest(colors.len() as u32, 1, bg);
        for (i, c) in colors.iter().enumerate() {
            ref_blend_canvas(&mut dest_ref[i * 4..(i + 1) * 4], *c, canvas, painter_alpha);
        }

        assert_eq!(dest_scan, dest_ref, "canvas scanline vs per-pixel mismatch");
    }

    #[test]
    fn blend_scanline_with_coverage_matches_perpixel() {
        let colors = [
            Color::rgba(255, 0, 0, 255),
            Color::rgba(0, 255, 0, 128),
            Color::rgba(200, 100, 50, 200),
        ];
        let coverages = [0x1000i32, 0x800, 0x100];
        let bg = Color::rgba(100, 100, 100, 200);
        let painter_alpha = 180u8;

        let mut buf = InterpolationBuffer::new(4);
        for (i, c) in colors.iter().enumerate() {
            buf.set_pixel(i, [c.r(), c.g(), c.b(), c.a()]);
        }
        buf.set_len(colors.len());

        let mut dest_scan = make_dest(colors.len() as u32, 1, bg);
        let mode = BlendMode::SourceOver { painter_alpha };
        blend_scanline(
            &mut dest_scan,
            &buf,
            colors.len(),
            Some(&coverages),
            &mode,
        );

        // Reference: apply coverage then blend
        let mut dest_ref = make_dest(colors.len() as u32, 1, bg);
        for (i, c) in colors.iter().enumerate() {
            let cov = coverages[i];
            let adjusted_a = if cov >= 0x1000 {
                c.a()
            } else {
                ((c.a() as i32 * cov + 0x800) >> 12).clamp(0, 255) as u8
            };
            let adjusted = Color::rgba(c.r(), c.g(), c.b(), adjusted_a);
            ref_blend_source_over(&mut dest_ref[i * 4..(i + 1) * 4], adjusted, painter_alpha);
        }

        assert_eq!(
            dest_scan, dest_ref,
            "coverage source-over scanline vs per-pixel mismatch"
        );
    }

    #[test]
    fn blend_scanline_premul_source_over_matches() {
        let pm_pixels: [[u8; 4]; 4] = [
            [200, 0, 0, 200],   // premul red
            [0, 100, 0, 128],   // premul green
            [0, 0, 0, 0],       // transparent
            [255, 255, 255, 255], // opaque white
        ];
        let bg = Color::rgba(100, 100, 100, 200);

        let mut buf = InterpolationBuffer::new(4);
        for (i, pm) in pm_pixels.iter().enumerate() {
            buf.set_pixel(i, *pm);
        }
        buf.set_len(pm_pixels.len());

        // Full coverage, painter_alpha=255
        let mut dest_scan = make_dest(pm_pixels.len() as u32, 1, bg);
        let mode = BlendMode::SourceOver { painter_alpha: 255 };
        blend_scanline_premul(&mut dest_scan, &buf, pm_pixels.len(), None, &mode);

        let mut dest_ref = make_dest(pm_pixels.len() as u32, 1, bg);
        for (i, pm) in pm_pixels.iter().enumerate() {
            ref_blend_premul_source_over(&mut dest_ref[i * 4..(i + 1) * 4], *pm);
        }

        assert_eq!(dest_scan, dest_ref, "premul source-over mismatch");
    }

    #[test]
    fn blend_scanline_premul_canvas_matches() {
        let pm_pixels: [[u8; 4]; 3] = [
            [200, 0, 0, 200],
            [0, 100, 0, 128],
            [255, 255, 255, 255],
        ];
        let bg = Color::rgba(100, 100, 100, 200);
        let canvas = Color::rgba(50, 50, 50, 255);

        let mut buf = InterpolationBuffer::new(4);
        for (i, pm) in pm_pixels.iter().enumerate() {
            buf.set_pixel(i, *pm);
        }
        buf.set_len(pm_pixels.len());

        let mut dest_scan = make_dest(pm_pixels.len() as u32, 1, bg);
        let mode = BlendMode::CanvasBlend {
            canvas,
            painter_alpha: 255,
        };
        blend_scanline_premul(&mut dest_scan, &buf, pm_pixels.len(), None, &mode);

        let mut dest_ref = make_dest(pm_pixels.len() as u32, 1, bg);
        for (i, pm) in pm_pixels.iter().enumerate() {
            ref_blend_premul_canvas(&mut dest_ref[i * 4..(i + 1) * 4], *pm, canvas);
        }

        assert_eq!(dest_scan, dest_ref, "premul canvas mismatch");
    }

    #[test]
    fn blend_scanline_premul_with_coverage_matches() {
        let pm_pixels: [[u8; 4]; 3] = [
            [200, 0, 0, 200],
            [0, 100, 0, 128],
            [50, 50, 50, 100],
        ];
        let coverages = [0x1000i32, 0x800, 0x400];
        let bg = Color::rgba(100, 100, 100, 200);
        let painter_alpha = 200u8;

        let mut buf = InterpolationBuffer::new(4);
        for (i, pm) in pm_pixels.iter().enumerate() {
            buf.set_pixel(i, *pm);
        }
        buf.set_len(pm_pixels.len());

        let mut dest_scan = make_dest(pm_pixels.len() as u32, 1, bg);
        let mode = BlendMode::SourceOver { painter_alpha };
        blend_scanline_premul(
            &mut dest_scan,
            &buf,
            pm_pixels.len(),
            Some(&coverages),
            &mode,
        );

        // Reference: apply coverage + painter_alpha to premul, then blend
        let mut dest_ref = make_dest(pm_pixels.len() as u32, 1, bg);
        for (i, pm) in pm_pixels.iter().enumerate() {
            let cov = coverages[i];
            let o_eff = (cov * painter_alpha as i32 + 127) / 255;
            let pm_mod = if o_eff >= 0x1000 {
                *pm
            } else if o_eff > 0 {
                [
                    ((pm[0] as i32 * o_eff + 0x800) >> 12) as u8,
                    ((pm[1] as i32 * o_eff + 0x800) >> 12) as u8,
                    ((pm[2] as i32 * o_eff + 0x800) >> 12) as u8,
                    ((pm[3] as i32 * o_eff + 0x800) >> 12) as u8,
                ]
            } else {
                continue;
            };
            ref_blend_premul_source_over(&mut dest_ref[i * 4..(i + 1) * 4], pm_mod);
        }

        assert_eq!(dest_scan, dest_ref, "premul coverage mismatch");
    }

    #[test]
    fn interpolation_buffer_basics() {
        let mut buf = InterpolationBuffer::new(4);
        assert_eq!(buf.max_pixels(), 256);
        buf.set_len(2);
        buf.set_pixel(0, [255, 0, 0, 255]);
        buf.set_pixel(1, [0, 255, 0, 128]);
        assert_eq!(buf.pixel_rgba(0), [255, 0, 0, 255]);
        assert_eq!(buf.pixel_rgba(1), [0, 255, 0, 128]);

        let mut buf1 = InterpolationBuffer::new(1);
        assert_eq!(buf1.max_pixels(), 1024);
        buf1.set_len(1);
        buf1.set_pixel(0, [128, 0, 0, 0]);
        assert_eq!(buf1.pixel_rgba(0), [128, 128, 128, 255]);
    }

    #[test]
    fn row_slice_basics() {
        let img = Image::new(4, 4, 4);
        let row = img.row_slice(0);
        assert_eq!(row.len(), 16);
        let row3 = img.row_slice(3);
        assert_eq!(row3.len(), 16);
    }

    #[test]
    fn blend_scanline_zero_coverage_noop() {
        let bg = Color::rgba(100, 100, 100, 200);
        let mut dest = make_dest(2, 1, bg);
        let dest_copy = dest.clone();

        let mut buf = InterpolationBuffer::new(4);
        buf.set_pixel(0, [255, 0, 0, 255]);
        buf.set_pixel(1, [0, 255, 0, 255]);
        buf.set_len(2);

        let coverages = [0i32, 0];
        let mode = BlendMode::SourceOver { painter_alpha: 255 };
        blend_scanline(&mut dest, &buf, 2, Some(&coverages), &mode);

        assert_eq!(dest, dest_copy, "zero coverage should not modify dest");
    }
}
