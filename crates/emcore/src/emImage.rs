use std::collections::BTreeSet;

use crate::emCore::emColor::emColor;
use crate::emCore::emPainterInterpolation::sample_bilinear;
use crate::emCore::emTexture::ImageExtension;

/// Pack XPM symbol bytes into a u32 key for binary search.
fn pack_symbol(bytes: &[u8], sym_size: usize) -> u32 {
    let mut key: u32 = 0;
    for &b in &bytes[..sym_size] {
        key = (key << 8) | b as u32;
    }
    key
}

/// Parse an XPM color value from the portion after the symbol.
/// Scans for key types in priority order: c, g, g4, m, s.
fn parse_xpm_color(s: &str) -> Option<emColor> {
    // Split into whitespace-separated tokens
    let tokens: Vec<&str> = s.split_whitespace().collect();
    // emLook for color keys in priority order
    for key in &["c", "g", "g4", "m", "s"] {
        for i in 0..tokens.len() {
            if tokens[i].eq_ignore_ascii_case(key) && i + 1 < tokens.len() {
                return emColor::TryParse(tokens[i + 1]);
            }
        }
    }
    None
}

/// Convert a single pixel between channel counts.
///
/// Port of C++ emImage.cpp:717-822 conversion table.
fn convert_pixel(s: &[u8], scc: usize, d: &mut [u8], dcc: usize) {
    match (scc, dcc) {
        (1, 1) => d[0] = s[0],
        (1, 2) => {
            d[0] = s[0];
            d[1] = 255;
        }
        (1, 3) => {
            d[0] = s[0];
            d[1] = s[0];
            d[2] = s[0];
        }
        (1, 4) => {
            d[0] = s[0];
            d[1] = s[0];
            d[2] = s[0];
            d[3] = 255;
        }
        (2, 1) => d[0] = s[0],
        (2, 2) => {
            d[0] = s[0];
            d[1] = s[1];
        }
        (2, 3) => {
            d[0] = s[0];
            d[1] = s[0];
            d[2] = s[0];
        }
        (2, 4) => {
            d[0] = s[0];
            d[1] = s[0];
            d[2] = s[0];
            d[3] = s[1];
        }
        (3, 1) => {
            d[0] = ((s[0] as u16 + s[1] as u16 + s[2] as u16 + 1) / 3) as u8;
        }
        (3, 2) => {
            d[0] = ((s[0] as u16 + s[1] as u16 + s[2] as u16 + 1) / 3) as u8;
            d[1] = 255;
        }
        (3, 3) => {
            d[0] = s[0];
            d[1] = s[1];
            d[2] = s[2];
        }
        (3, 4) => {
            d[0] = s[0];
            d[1] = s[1];
            d[2] = s[2];
            d[3] = 255;
        }
        (4, 1) => {
            d[0] = ((s[0] as u16 + s[1] as u16 + s[2] as u16 + 1) / 3) as u8;
        }
        (4, 2) => {
            d[0] = ((s[0] as u16 + s[1] as u16 + s[2] as u16 + 1) / 3) as u8;
            d[1] = s[3];
        }
        (4, 3) => {
            d[0] = s[0];
            d[1] = s[1];
            d[2] = s[2];
        }
        (4, 4) => {
            d[0] = s[0];
            d[1] = s[1];
            d[2] = s[2];
            d[3] = s[3];
        }
        _ => unreachable!(),
    }
}

/// CPU bitmap image with 1–4 channels per pixel.
#[derive(Clone, Debug, PartialEq)]
pub struct emImage {
    width: u32,
    height: u32,
    channel_count: u8,
    data: Vec<u8>,
}

impl emImage {
    /// Create a zero-filled image.
    ///
    /// # Panics
    /// Panics if `channel_count` is not 1, 2, 3, or 4.
    pub fn new(width: u32, height: u32, channel_count: u8) -> Self {
        assert!(
            (1..=4).contains(&channel_count),
            "channel_count must be 1, 2, 3, or 4"
        );
        let len = width as usize * height as usize * channel_count as usize;
        Self {
            width,
            height,
            channel_count,
            data: vec![0; len],
        }
    }

    /// Create an image from pre-existing pixel data.
    ///
    /// # Panics
    /// Panics if `channel_count` is not 1, 2, 3, or 4, or if `data.len()`
    /// does not equal `width * height * channel_count`.
    pub fn from_raw(width: u32, height: u32, channel_count: u8, data: Vec<u8>) -> Self {
        assert!(
            (1..=4).contains(&channel_count),
            "channel_count must be 1, 2, 3, or 4"
        );
        let expected = width as usize * height as usize * channel_count as usize;
        assert_eq!(
            data.len(),
            expected,
            "data length {} does not match {}x{}x{}={}",
            data.len(),
            width,
            height,
            channel_count,
            expected,
        );
        Self {
            width,
            height,
            channel_count,
            data,
        }
    }

    #[inline]
    pub fn GetWidth(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn GetHeight(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn GetChannelCount(&self) -> u8 {
        self.channel_count
    }

    #[inline]
    pub fn GetMap(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn GetWritableMap(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Returns `true` if either dimension is zero.
    #[inline]
    pub fn IsEmpty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    fn pixel_offset(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width && y < self.height);
        (y as usize * self.width as usize + x as usize) * self.channel_count as usize
    }

    /// Access the raw channel bytes for a pixel.
    pub fn GetPixel(&self, x: u32, y: u32) -> &[u8] {
        let offset = self.pixel_offset(x, y);
        &self.data[offset..offset + self.channel_count as usize]
    }

    /// Row slice starting at pixel (0, y). Length = width * channel_count.
    /// Caller must guarantee y < height.
    #[inline(always)]
    pub fn row_slice(&self, y: u32) -> &[u8] {
        let stride = self.width as usize * self.channel_count as usize;
        let offset = y as usize * stride;
        &self.data[offset..offset + stride]
    }

    /// Mutably access the raw channel bytes for a pixel.
    pub fn SetPixel(&mut self, x: u32, y: u32) -> &mut [u8] {
        let offset = self.pixel_offset(x, y);
        let cc = self.channel_count as usize;
        &mut self.data[offset..offset + cc]
    }

    /// Fill all pixels with the given color.
    ///
    /// Port of C++ `emImage::Fill`. Converts the color to the image's channel
    /// count: 1-ch uses grey, 2-ch uses grey+alpha, 3-ch uses RGB, 4-ch uses RGBA.
    pub fn fill(&mut self, color: emColor) {
        match self.channel_count {
            1 => {
                let g = color.GetGrey();
                self.data.fill(g);
            }
            2 => {
                let bytes = [color.GetGrey(), color.GetAlpha()];
                for chunk in self.data.chunks_exact_mut(2) {
                    chunk.copy_from_slice(&bytes);
                }
            }
            3 => {
                let bytes = [color.GetRed(), color.GetGreen(), color.GetBlue()];
                for chunk in self.data.chunks_exact_mut(3) {
                    chunk.copy_from_slice(&bytes);
                }
            }
            4 => {
                let bytes = [color.GetRed(), color.GetGreen(), color.GetBlue(), color.GetAlpha()];
                for chunk in self.data.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&bytes);
                }
            }
            _ => unreachable!(),
        }
    }

    /// Reinitialize in-place with new dimensions, zero-filled.
    pub fn setup(&mut self, w: u32, h: u32, cc: u8) {
        assert!((1..=4).contains(&cc), "channel_count must be 1, 2, 3, or 4");
        self.width = w;
        self.height = h;
        self.channel_count = cc;
        let len = w as usize * h as usize * cc as usize;
        self.data.clear();
        self.data.resize(len, 0);
    }

    /// Reset to 0×0 empty image.
    pub fn Clear(&mut self) {
        self.width = 0;
        self.height = 0;
        self.data.clear();
    }

    /// Get a single channel value for a pixel.
    pub fn get_pixel_channel(&self, x: u32, y: u32, ch: u8) -> u8 {
        let offset = self.pixel_offset(x, y);
        self.data[offset + ch as usize]
    }

    /// Set a single channel value for a pixel.
    pub fn set_pixel_channel(&mut self, x: u32, y: u32, ch: u8, val: u8) {
        let offset = self.pixel_offset(x, y);
        self.data[offset + ch as usize] = val;
    }

    /// Fill a rectangle with a color. Clips to image bounds.
    ///
    /// Port of C++ `emImage::Fill` with rect clipping. Converts color per
    /// channel count (same as `fill`).
    pub fn Fill(&mut self, x: u32, y: u32, w: u32, h: u32, color: emColor) {
        let x1 = x.min(self.width);
        let y1 = y.min(self.height);
        let x2 = (x.saturating_add(w)).min(self.width);
        let y2 = (y.saturating_add(h)).min(self.height);
        let cc = self.channel_count as usize;
        let stride = self.width as usize * cc;
        match self.channel_count {
            1 => {
                let g = color.GetGrey();
                for row in y1..y2 {
                    let row_start = row as usize * stride + x1 as usize;
                    for col in 0..(x2 - x1) as usize {
                        self.data[row_start + col] = g;
                    }
                }
            }
            2 => {
                let bytes = [color.GetGrey(), color.GetAlpha()];
                for row in y1..y2 {
                    let row_start = row as usize * stride + x1 as usize * 2;
                    for col in 0..(x2 - x1) as usize {
                        let off = row_start + col * 2;
                        self.data[off..off + 2].copy_from_slice(&bytes);
                    }
                }
            }
            3 => {
                let bytes = [color.GetRed(), color.GetGreen(), color.GetBlue()];
                for row in y1..y2 {
                    let row_start = row as usize * stride + x1 as usize * 3;
                    for col in 0..(x2 - x1) as usize {
                        let off = row_start + col * 3;
                        self.data[off..off + 3].copy_from_slice(&bytes);
                    }
                }
            }
            4 => {
                let bytes = [color.GetRed(), color.GetGreen(), color.GetBlue(), color.GetAlpha()];
                for row in y1..y2 {
                    let row_start = row as usize * stride + x1 as usize * 4;
                    for col in 0..(x2 - x1) as usize {
                        let off = row_start + col * 4;
                        self.data[off..off + 4].copy_from_slice(&bytes);
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    /// Fill one channel of the entire image with a value.
    pub fn fill_channel(&mut self, ch: u8, val: u8) {
        let cc = self.channel_count as usize;
        let ch = ch as usize;
        for i in (ch..self.data.len()).step_by(cc) {
            self.data[i] = val;
        }
    }

    /// Fill one channel within a rectangle. Clips to image bounds.
    pub fn FillChannel(&mut self, ch: u8, x: u32, y: u32, w: u32, h: u32, val: u8) {
        let x1 = x.min(self.width);
        let y1 = y.min(self.height);
        let x2 = (x.saturating_add(w)).min(self.width);
        let y2 = (y.saturating_add(h)).min(self.height);
        let cc = self.channel_count as usize;
        let stride = self.width as usize * cc;
        for row in y1..y2 {
            let row_start = row as usize * stride;
            for col in x1..x2 {
                self.data[row_start + col as usize * cc + ch as usize] = val;
            }
        }
    }

    /// Copy entire source image into self at (dx, dy). Channel counts must match.
    pub fn Copy(&mut self, dx: u32, dy: u32, src: &emImage) {
        self.copy_from_rect(dx, dy, src, (0, 0, src.width, src.height));
    }

    /// Copy a rectangle from source image into self. Clips both sides.
    /// `src_rect` is `(x, y, w, h)` within `src`.
    ///
    /// Port of C++ `emImage::CopyRect`. Supports cross-channel conversion
    /// using the same formulas as `get_converted`.
    pub fn copy_from_rect(
        &mut self,
        dx: u32,
        dy: u32,
        src: &emImage,
        src_rect: (u32, u32, u32, u32),
    ) {
        let (sx, sy, sw, sh) = src_rect;
        let scc = src.channel_count as usize;
        let dcc = self.channel_count as usize;

        // Clip source rect to source bounds
        let sx1 = sx.min(src.width);
        let sy1 = sy.min(src.height);
        let sx2 = (sx.saturating_add(sw)).min(src.width);
        let sy2 = (sy.saturating_add(sh)).min(src.height);

        let copy_w = sx2 - sx1;
        let copy_h = sy2 - sy1;

        // Clip destination
        let copy_w = copy_w.min(self.width.saturating_sub(dx));
        let copy_h = copy_h.min(self.height.saturating_sub(dy));

        if scc == dcc {
            // Fast path: same channel count
            let src_stride = src.width as usize * scc;
            let dst_stride = self.width as usize * dcc;
            for row in 0..copy_h {
                let src_off = (sy1 + row) as usize * src_stride + sx1 as usize * scc;
                let dst_off = (dy + row) as usize * dst_stride + dx as usize * dcc;
                let len = copy_w as usize * scc;
                self.data[dst_off..dst_off + len]
                    .copy_from_slice(&src.data[src_off..src_off + len]);
            }
        } else {
            // Cross-channel conversion (C++ emImage.cpp:717-822)
            let src_stride = src.width as usize * scc;
            let dst_stride = self.width as usize * dcc;
            for row in 0..copy_h {
                for col in 0..copy_w {
                    let si = (sy1 + row) as usize * src_stride + (sx1 + col) as usize * scc;
                    let di = (dy + row) as usize * dst_stride + (dx + col) as usize * dcc;
                    let s = &src.data[si..si + scc];
                    let d = &mut self.data[di..di + dcc];
                    convert_pixel(s, scc, d, dcc);
                }
            }
        }
    }

    /// Copy a single channel from src into a (possibly different) channel in self.
    pub fn CopyChannel(&mut self, dst_ch: u8, dx: u32, dy: u32, src: &emImage, src_ch: u8) {
        let copy_w = src.width.min(self.width.saturating_sub(dx));
        let copy_h = src.height.min(self.height.saturating_sub(dy));
        let scc = src.channel_count as usize;
        let dcc = self.channel_count as usize;

        for row in 0..copy_h {
            for col in 0..copy_w {
                let si = (row as usize * src.width as usize + col as usize) * scc + src_ch as usize;
                let di = ((dy + row) as usize * self.width as usize + (dx + col) as usize) * dcc
                    + dst_ch as usize;
                self.data[di] = src.data[si];
            }
        }
    }

    /// Extract a sub-image, optionally converting to a different channel count.
    ///
    /// Port of C++ `emImage::GetCropped(x,y,w,h,channelCount)`.
    /// Pass `None` for `out_cc` to preserve the source channel count.
    pub fn get_cropped(&self, x: u32, y: u32, w: u32, h: u32, out_cc: Option<u8>) -> emImage {
        let x1 = x.min(self.width);
        let y1 = y.min(self.height);
        let x2 = (x.saturating_add(w)).min(self.width);
        let y2 = (y.saturating_add(h)).min(self.height);
        let cw = x2 - x1;
        let ch = y2 - y1;
        let src_cc = self.channel_count;
        let dst_cc = out_cc.unwrap_or(src_cc);

        if dst_cc == src_cc {
            // Fast path: same channel count, direct copy
            let cc = src_cc as usize;
            let mut data = Vec::with_capacity(cw as usize * ch as usize * cc);
            let stride = self.width as usize * cc;
            for row in y1..y2 {
                let start = row as usize * stride + x1 as usize * cc;
                data.extend_from_slice(&self.data[start..start + cw as usize * cc]);
            }
            emImage {
                width: cw,
                height: ch,
                channel_count: src_cc,
                data,
            }
        } else {
            // Different channel count: crop then convert
            let cropped = self.get_cropped(x, y, w, h, None);
            cropped.get_converted(dst_cc)
        }
    }

    /// Convert to a different channel count. All 12 combos (1↔2↔3↔4) supported.
    pub fn get_converted(&self, new_cc: u8) -> emImage {
        assert!(
            (1..=4).contains(&new_cc),
            "channel_count must be 1, 2, 3, or 4"
        );
        let old_cc = self.channel_count;
        if old_cc == new_cc {
            return self.clone();
        }
        let mut out = emImage::new(self.width, self.height, new_cc);
        for y in 0..self.height {
            for x in 0..self.width {
                let src = self.GetPixel(x, y);
                let dst = out.SetPixel(x, y);
                match (old_cc, new_cc) {
                    (1, 2) => {
                        dst[0] = src[0];
                        dst[1] = 255;
                    }
                    (1, 3) => {
                        dst[0] = src[0];
                        dst[1] = src[0];
                        dst[2] = src[0];
                    }
                    (1, 4) => {
                        dst[0] = src[0];
                        dst[1] = src[0];
                        dst[2] = src[0];
                        dst[3] = 255;
                    }
                    (2, 1) => {
                        dst[0] = src[0];
                    }
                    (2, 3) => {
                        dst[0] = src[0];
                        dst[1] = src[0];
                        dst[2] = src[0];
                    }
                    (2, 4) => {
                        dst[0] = src[0];
                        dst[1] = src[0];
                        dst[2] = src[0];
                        dst[3] = src[1];
                    }
                    (3, 1) => {
                        dst[0] = ((src[0] as u16 + src[1] as u16 + src[2] as u16 + 1) / 3) as u8;
                    }
                    (3, 2) => {
                        dst[0] = ((src[0] as u16 + src[1] as u16 + src[2] as u16 + 1) / 3) as u8;
                        dst[1] = 255;
                    }
                    (3, 4) => {
                        dst[0] = src[0];
                        dst[1] = src[1];
                        dst[2] = src[2];
                        dst[3] = 255;
                    }
                    (4, 1) => {
                        dst[0] = ((src[0] as u16 + src[1] as u16 + src[2] as u16 + 1) / 3) as u8;
                    }
                    (4, 2) => {
                        dst[0] = ((src[0] as u16 + src[1] as u16 + src[2] as u16 + 1) / 3) as u8;
                        dst[1] = src[3];
                    }
                    (4, 3) => {
                        dst[0] = src[0];
                        dst[1] = src[1];
                        dst[2] = src[2];
                    }
                    _ => unreachable!(),
                }
            }
        }
        out
    }

    /// Crop to the bounding box of non-zero alpha pixels. Requires 4 or 2 channels.
    ///
    /// Port of C++ `emImage::GetCroppedByAlpha`. Pass `out_cc` to convert
    /// channel count, or `None` to preserve.
    pub fn get_cropped_by_alpha(&self, out_cc: Option<u8>) -> emImage {
        if let Some((x, y, w, h)) = self.calc_alpha_min_max_rect() {
            self.get_cropped(x, y, w, h, out_cc)
        } else {
            emImage::new(0, 0, out_cc.unwrap_or(self.channel_count))
        }
    }

    /// Returns `true` if any pixel has differing R, G, B channels. Requires ≥3 channels.
    pub fn has_any_non_grey_pixel(&self) -> bool {
        if self.channel_count < 3 {
            return false;
        }
        let cc = self.channel_count as usize;
        for chunk in self.data.chunks_exact(cc) {
            if chunk[0] != chunk[1] || chunk[1] != chunk[2] {
                return true;
            }
        }
        false
    }

    /// Returns `true` if any pixel has alpha < 255. Requires 2 or 4 channels.
    pub fn has_any_transparent_pixel(&self) -> bool {
        let alpha_ch = match self.channel_count {
            2 => 1,
            4 => 3,
            _ => return false,
        };
        let cc = self.channel_count as usize;
        for chunk in self.data.chunks_exact(cc) {
            if chunk[alpha_ch] < 255 {
                return true;
            }
        }
        false
    }

    /// Find the bounding rect of pixels differing from `bg`. Returns `(x, y, w, h)`.
    ///
    /// Port of C++ `emImage::CalcMinMaxRect`. Handles all channel counts.
    pub fn calc_min_max_rect(&self, bg: emColor) -> Option<(u32, u32, u32, u32)> {
        let bg_bytes: &[u8] = match self.channel_count {
            1 => &[bg.GetGrey()],
            2 => &[bg.GetGrey(), bg.GetAlpha()],
            3 => &[bg.GetRed(), bg.GetGreen(), bg.GetBlue()],
            4 => &[bg.GetRed(), bg.GetGreen(), bg.GetBlue(), bg.GetAlpha()],
            _ => unreachable!(),
        };
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for y in 0..self.height {
            for x in 0..self.width {
                let p = self.GetPixel(x, y);
                if p != bg_bytes {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if max_x < min_x {
            None
        } else {
            Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
        }
    }

    /// Find the bounding rect of pixels in one channel differing from `bg_val`.
    pub fn calc_channel_min_max_rect(&self, ch: u8, bg_val: u8) -> Option<(u32, u32, u32, u32)> {
        let cc = self.channel_count as usize;
        let ch = ch as usize;
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for y in 0..self.height {
            for x in 0..self.width {
                let off = (y as usize * self.width as usize + x as usize) * cc + ch;
                if self.data[off] != bg_val {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if max_x < min_x {
            None
        } else {
            Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
        }
    }

    /// Find the bounding rect of non-zero alpha pixels. Requires 2 or 4 channels.
    pub fn calc_alpha_min_max_rect(&self) -> Option<(u32, u32, u32, u32)> {
        let alpha_ch = match self.channel_count {
            2 => 1,
            4 => 3,
            _ => return None,
        };
        self.calc_channel_min_max_rect(alpha_ch, 0)
    }

    /// Sample a pixel using bilinear or area-sampling interpolation.
    ///
    /// `x`, `y` are the top-left of the sampling footprint in source pixel
    /// coordinates. `w`, `h` describe the footprint size. When both are < 1.0,
    /// bilinear sampling is used. When either >= 1.0, area-sampling averages
    /// over the footprint.
    ///
    /// Port of C++ `emImage::GetPixelInterpolated` (emImage.cpp:389-491).
    /// All intermediate math uses i32 with 0x8000 rounding bias and >>16 final
    /// shift.
    pub fn get_pixel_interpolated(
        &self,
        mut x: f64,
        mut y: f64,
        mut w: f64,
        mut h: f64,
        bg: emColor,
    ) -> emColor {
        if self.IsEmpty() {
            return bg;
        }

        // Clamp footprint to >= 1.0, adjusting center (C++ lines 393-398)
        if h < 1.0 {
            y = (y * 2.0 + h - 1.0) * 0.5;
            h = 1.0;
        }
        if w < 1.0 {
            x = (x * 2.0 + w - 1.0) * 0.5;
            w = 1.0;
        }

        // If footprint is exactly 1x1, use bilinear for speed
        if w == 1.0 && h == 1.0 {
            if x < -0.5 || y < -0.5 || x >= self.width as f64 - 0.5 || y >= self.height as f64 - 0.5
            {
                return bg;
            }
            return sample_bilinear(self, x, y, ImageExtension::Clamp);
        }

        let x2 = x + w;
        let y2 = y + h;
        let mut r: i32 = 0x8000;
        let mut g: i32 = 0x8000;
        let mut b: i32 = 0x8000;
        let mut a: i32 = 0x8000;
        let rh = 65536.0 / h;
        let cc = self.channel_count as usize;
        let img_w = self.width as i32;
        let img_h = self.height as i32;
        let bg_r = bg.GetRed() as i32;
        let bg_g = bg.GetGreen() as i32;
        let bg_b = bg.GetBlue() as i32;
        let bg_a = bg.GetAlpha() as i32;

        let mut ym = y.floor() as i32;
        let mut yn = ym + 1;
        let mut fy = ((yn as f64 - y) * rh) as i32;

        loop {
            if ym < 0 || ym >= img_h {
                // Out-of-bounds row: use bg
                let ifx = fy;
                r += bg_r * ifx;
                g += bg_g * ifx;
                b += bg_b * ifx;
                a += bg_a * ifx;
            } else {
                let row_offset = ym as usize * self.width as usize * cc;
                let rw = fy as f64 / w;
                let mut xm = x.floor() as i32;
                let mut xn = xm + 1;
                let mut ifx = ((xn as f64 - x) * rw) as i32;
                let irw = rw as i32;

                loop {
                    if xm < 0 || xm >= img_w {
                        // Out-of-bounds column: use bg
                        r += bg_r * ifx;
                        g += bg_g * ifx;
                        b += bg_b * ifx;
                        a += bg_a * ifx;
                    } else {
                        let pi = row_offset + xm as usize * cc;
                        match cc {
                            1 => {
                                let v = self.data[pi] as i32;
                                r += v * ifx;
                                g += v * ifx;
                                b += v * ifx;
                                a += 255 * ifx;
                            }
                            2 => {
                                let v = self.data[pi] as i32;
                                r += v * ifx;
                                g += v * ifx;
                                b += v * ifx;
                                a += self.data[pi + 1] as i32 * ifx;
                            }
                            3 => {
                                r += self.data[pi] as i32 * ifx;
                                g += self.data[pi + 1] as i32 * ifx;
                                b += self.data[pi + 2] as i32 * ifx;
                                a += 255 * ifx;
                            }
                            4 => {
                                r += self.data[pi] as i32 * ifx;
                                g += self.data[pi + 1] as i32 * ifx;
                                b += self.data[pi + 2] as i32 * ifx;
                                a += self.data[pi + 3] as i32 * ifx;
                            }
                            _ => unreachable!(),
                        }
                    }

                    xm = xn;
                    xn += 1;
                    ifx = irw;
                    if (xn as f64) <= x2 {
                        continue;
                    }
                    if xm as f64 >= x2 {
                        break;
                    }
                    ifx = ((x2 - xm as f64) * rw) as i32;
                }
            }

            ym = yn;
            yn += 1;
            fy = rh as i32;
            if (yn as f64) <= y2 {
                continue;
            }
            if ym as f64 >= y2 {
                break;
            }
            fy = ((y2 - ym as f64) * rh) as i32;
        }

        emColor::rgba(
            (r >> 16).clamp(0, 255) as u8,
            (g >> 16).clamp(0, 255) as u8,
            (b >> 16).clamp(0, 255) as u8,
            (a >> 16).clamp(0, 255) as u8,
        )
    }

    /// Apply an affine transformation from `src` into a region of `self`.
    ///
    /// `x`, `y`, `w`, `h` define the target clip rectangle (in `self` pixel
    /// coords).  `matrix` is a 2×3 affine mapping **source → target**:
    ///
    /// ```text
    /// target_x = matrix[0]*src_x + matrix[1]*src_y + matrix[2]
    /// target_y = matrix[3]*src_x + matrix[4]*src_y + matrix[5]
    /// ```
    ///
    /// The matrix is inverted internally so that each target pixel can be
    /// mapped back to source coordinates.
    ///
    /// * `interpolate` – `true` for bilinear sampling, `false` for nearest.
    /// * `bg_color` – used for source samples that fall outside the source
    ///   image bounds.
    ///
    /// Both `self` and `src` must be 4-channel RGBA images.
    pub fn copy_transformed(
        &mut self,
        clip: (i32, i32, i32, i32),
        matrix: &[f64; 6],
        src: &emImage,
        interpolate: bool,
        bg_color: emColor,
    ) {
        let (x, y, w, h) = clip;

        if w <= 0 || h <= 0 || self.IsEmpty() {
            return;
        }

        // Invert the source→target affine matrix to get target→source.
        //
        //   | a  b  c |        | a  b |
        //   | d  e  f |   M =  | d  e |
        //
        // inv(M) = (1/det) *  |  e  -b |
        //                     | -d   a |
        let a = matrix[0];
        let b = matrix[1];
        let c = matrix[2];
        let d = matrix[3];
        let e = matrix[4];
        let f = matrix[5];

        let det = a * e - b * d;
        if det.abs() < 1e-15 {
            // Degenerate (singular) matrix — fill with bg.
            let bg_bytes = [bg_color.GetRed(), bg_color.GetGreen(), bg_color.GetBlue(), bg_color.GetAlpha()];
            for py in y..y + h {
                for px in x..x + w {
                    if px >= 0 && py >= 0 && (px as u32) < self.width && (py as u32) < self.height {
                        self.SetPixel(px as u32, py as u32)
                            .copy_from_slice(&bg_bytes);
                    }
                }
            }
            return;
        }

        let inv_det = 1.0 / det;
        let ia = e * inv_det;
        let ib = -b * inv_det;
        let id = -d * inv_det;
        let ie = a * inv_det;
        // Inverted translation: inv_M * (-t)
        let ic = -(ia * c + ib * f);
        let ifc = -(id * c + ie * f);

        let scc = src.channel_count as usize;
        let dcc = self.channel_count as usize;

        // Compute sampling footprint (C++ lines 910-913).
        // sw/sh estimate how many source pixels one target pixel covers.
        let sw = if interpolate {
            let d = ia * 0.0 + ib * 0.0 + ic;
            (ia * 1.0 + ib * 0.0 + ic - d)
                .abs()
                .max((ia * 0.0 + ib * 1.0 + ic - d).abs())
        } else {
            1.0
        };
        let sh = if interpolate {
            let d = id * 0.0 + ie * 0.0 + ifc;
            (id * 1.0 + ie * 0.0 + ifc - d)
                .abs()
                .max((id * 0.0 + ie * 1.0 + ifc - d).abs())
        } else {
            1.0
        };

        for py in y..y + h {
            if py < 0 || (py as u32) >= self.height {
                continue;
            }
            for px in x..x + w {
                if px < 0 || (px as u32) >= self.width {
                    continue;
                }

                let tx = px as f64;
                let ty = py as f64;

                // Map target pixel back to source coordinates.
                let sx = ia * tx + ib * ty + ic;
                let sy = id * tx + ie * ty + ifc;

                let color = if interpolate {
                    src.get_pixel_interpolated(sx, sy, sw, sh, bg_color)
                } else {
                    // Nearest-neighbor with bounds check.
                    let ix = sx.round() as i32;
                    let iy = sy.round() as i32;
                    if ix < 0 || iy < 0 || ix >= src.width as i32 || iy >= src.height as i32 {
                        bg_color
                    } else {
                        let p = src.GetPixel(ix as u32, iy as u32);
                        // Convert from source cc to RGBA emColor
                        match scc {
                            1 => emColor::rgba(p[0], p[0], p[0], 255),
                            2 => emColor::rgba(p[0], p[0], p[0], p[1]),
                            3 => emColor::rgba(p[0], p[1], p[2], 255),
                            4 => emColor::rgba(p[0], p[1], p[2], p[3]),
                            _ => unreachable!(),
                        }
                    }
                };

                // Write per destination channel count
                let dst = self.SetPixel(px as u32, py as u32);
                match dcc {
                    1 => dst[0] = color.GetGrey(),
                    2 => {
                        dst[0] = color.GetGrey();
                        dst[1] = color.GetAlpha();
                    }
                    3 => {
                        dst[0] = color.GetRed();
                        dst[1] = color.GetGreen();
                        dst[2] = color.GetBlue();
                    }
                    4 => {
                        dst[0] = color.GetRed();
                        dst[1] = color.GetGreen();
                        dst[2] = color.GetBlue();
                        dst[3] = color.GetAlpha();
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    /// Get an affine-transformed copy of this image.
    ///
    /// `matrix` maps source coordinates to target coordinates:
    /// ```text
    /// target_x = matrix[0]*src_x + matrix[1]*src_y + matrix[2]
    /// target_y = matrix[3]*src_x + matrix[4]*src_y + matrix[5]
    /// ```
    ///
    /// Any translation in the matrix is ignored — the output is sized to
    /// contain the transformed image without offset.
    ///
    /// * `interpolate` – `true` for bilinear sampling, `false` for nearest.
    /// * `bg_color` – fills areas outside the source bounds.
    /// * `channel_count` – output channel count, or `None` to keep the same.
    ///
    /// Requires 4-channel images (both source and result).
    ///
    /// Port of C++ `emImage::GetTransformed`.
    pub fn get_transformed(
        &self,
        matrix: &[f64; 6],
        interpolate: bool,
        bg_color: emColor,
        channel_count: Option<u8>,
    ) -> emImage {
        let out_cc = channel_count.unwrap_or(self.channel_count);
        if self.IsEmpty() {
            return emImage::new(0, 0, out_cc);
        }

        // Compute the bounding box of the four transformed corners
        // (ignoring translation — we only use the linear part).
        let a = matrix[0];
        let b = matrix[1];
        let d = matrix[3];
        let e = matrix[4];

        let sw = self.width as f64;
        let sh = self.height as f64;

        let corners = [(0.0, 0.0), (sw, 0.0), (0.0, sh), (sw, sh)];

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for &(cx, cy) in &corners {
            let tx = a * cx + b * cy;
            let ty = d * cx + e * cy;
            if tx < min_x {
                min_x = tx;
            }
            if ty < min_y {
                min_y = ty;
            }
            if tx > max_x {
                max_x = tx;
            }
            if ty > max_y {
                max_y = ty;
            }
        }

        let out_w = (max_x - min_x).ceil() as u32;
        let out_h = (max_y - min_y).ceil() as u32;

        if out_w == 0 || out_h == 0 {
            return emImage::new(0, 0, out_cc);
        }

        // Build the matrix with translation set so output starts at (0,0)
        let adjusted = [a, b, -min_x, d, e, -min_y];

        let mut result = emImage::new(out_w, out_h, out_cc);
        result.copy_transformed(
            (0, 0, out_w as i32, out_h as i32),
            &adjusted,
            self,
            interpolate,
            bg_color,
        );
        result
    }

    /// Store a user-provided memory-mapped buffer, replacing owned data.
    ///
    /// Port of C++ `emImage::SetUserMap`. The buffer must have exactly
    /// `w * h * channels` bytes.
    ///
    /// # Panics
    /// Panics if `channels` is not 1..=4 or the slice length is wrong.
    pub fn set_user_map(&mut self, ptr: &[u8], w: u32, h: u32, channels: u8) {
        assert!(
            (1..=4).contains(&channels),
            "channel_count must be 1, 2, 3, or 4"
        );
        let expected = w as usize * h as usize * channels as usize;
        assert_eq!(
            ptr.len(),
            expected,
            "user map length {} does not match {}x{}x{}={}",
            ptr.len(),
            w,
            h,
            channels,
            expected,
        );
        self.width = w;
        self.height = h;
        self.channel_count = channels;
        self.data.clear();
        self.data.extend_from_slice(ptr);
    }

    /// Returns `true` if the image was set up via [`set_user_map`](Self::set_user_map).
    ///
    /// In this Rust port the data is always owned (we copy the user map), so
    /// this always returns `false`. Provided for API parity with C++
    /// `emImage::HasUserMap`.
    pub fn has_user_map(&self) -> bool {
        false
    }

    /// Parse an XPM image from string slices.
    ///
    /// Port of C++ `emImage::TryParseXpm` (emImage.cpp:111-282).
    /// `xpm` is the array of strings from the XPM data (header, colors, pixels).
    /// `out_cc` specifies output channel count, or `None` to auto-detect.
    pub fn try_parse_xpm(xpm: &[&str], out_cc: Option<u8>) -> Option<emImage> {
        if xpm.is_empty() {
            return None;
        }

        // Parse header: "width height num_colors sym_size"
        let header = xpm[0];
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }
        let width: u32 = parts[0].parse().ok()?;
        let height: u32 = parts[1].parse().ok()?;
        let num_colors: usize = parts[2].parse().ok()?;
        let sym_size: usize = parts[3].parse().ok()?;

        if sym_size == 0 || sym_size > 4 {
            return None;
        }
        if xpm.len() < 1 + num_colors + height as usize {
            return None;
        }

        // Parse color entries
        // Each entry: first `sym_size` chars are the symbol, then space-separated
        // key/value pairs like "c #FF0000" or "c red"
        let mut color_table: Vec<(u32, emColor)> = Vec::with_capacity(num_colors);
        for i in 0..num_colors {
            let line = xpm[1 + i];
            if line.len() < sym_size {
                return None;
            }
            // Extract symbol and pack into u32
            let sym_str = &line[..sym_size];
            let sym_key = pack_symbol(sym_str.as_bytes(), sym_size);

            // Parse color value: scan for key types c, g, g4, m, s
            let rest = &line[sym_size..];
            let color = parse_xpm_color(rest)?;
            color_table.push((sym_key, color));
        }

        // Sort by symbol for binary search
        color_table.sort_by_key(|&(k, _)| k);

        // Auto-detect channel count
        let cc = out_cc.unwrap_or_else(|| {
            let mut cc: u8 = 1;
            for &(_, color) in &color_table {
                if !color.IsGrey() {
                    cc = cc.max(3);
                }
                if color.GetAlpha() != 255 {
                    cc = if cc >= 3 { 4 } else { 2 };
                }
            }
            cc
        });

        let mut img = emImage::new(width, height, cc);

        // Map pixel rows
        for y in 0..height {
            let row_line = xpm[1 + num_colors + y as usize];
            let row_bytes = row_line.as_bytes();
            for x in 0..width {
                let start = x as usize * sym_size;
                if start + sym_size > row_bytes.len() {
                    return None;
                }
                let sym_key = pack_symbol(&row_bytes[start..start + sym_size], sym_size);
                let color = match color_table.binary_search_by_key(&sym_key, |&(k, _)| k) {
                    Ok(idx) => color_table[idx].1,
                    Err(_) => return None,
                };
                let dst = img.SetPixel(x, y);
                match cc {
                    1 => dst[0] = color.GetGrey(),
                    2 => {
                        dst[0] = color.GetGrey();
                        dst[1] = color.GetAlpha();
                    }
                    3 => {
                        dst[0] = color.GetRed();
                        dst[1] = color.GetGreen();
                        dst[2] = color.GetBlue();
                    }
                    4 => {
                        dst[0] = color.GetRed();
                        dst[1] = color.GetGreen();
                        dst[2] = color.GetBlue();
                        dst[3] = color.GetAlpha();
                    }
                    _ => unreachable!(),
                }
            }
        }

        Some(img)
    }

    /// Prepare the image for use with a `emPainter`.
    ///
    /// Returns `true` if the image's channel count is paintable (currently
    /// only 4-channel RGBA). This is the Rust equivalent of C++
    /// `emImage::PreparePainter` -- actual painter setup is handled by the
    /// `emPainter` constructor in this port.
    pub fn prepare_painter(&self) -> bool {
        Self::is_channel_count_paintable(self.channel_count)
    }

    /// Whether a `emPainter` can paint into an image with this channel count.
    ///
    /// Currently only 4-channel (RGBA) images are paintable.
    ///
    /// Port of C++ `emImage::IsChannelCountPaintable`.
    pub fn is_channel_count_paintable(channel_count: u8) -> bool {
        channel_count == 4
    }

    /// Collect all unique colors, sorted by packed u32 value.
    ///
    /// Port of C++ `emImage::DetermineAllColorsSorted`. If unique colors exceed
    /// `limit`, returns an empty vec. Handles all channel counts by packing
    /// per-cc bytes into a u32 key.
    pub fn determine_all_colors_sorted(&self, limit: usize) -> Vec<emColor> {
        let cc = self.channel_count as usize;
        let mut set = BTreeSet::new();
        for chunk in self.data.chunks_exact(cc) {
            let packed = match cc {
                1 => (chunk[0] as u32) << 24,
                2 => (chunk[0] as u32) << 24 | (chunk[1] as u32) << 16,
                3 => (chunk[0] as u32) << 24 | (chunk[1] as u32) << 16 | (chunk[2] as u32) << 8,
                4 => {
                    (chunk[0] as u32) << 24
                        | (chunk[1] as u32) << 16
                        | (chunk[2] as u32) << 8
                        | chunk[3] as u32
                }
                _ => unreachable!(),
            };
            set.insert(packed);
            if set.len() > limit {
                return Vec::new();
            }
        }
        set.into_iter()
            .map(|v| emColor::rgba((v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_zero_filled() {
        let img = emImage::new(4, 4, 4);
        assert!(img.GetMap().iter().all(|&b| b == 0));
        assert_eq!(img.GetMap().len(), 4 * 4 * 4);
    }

    #[test]
    fn pixel_access() {
        let mut img = emImage::new(2, 2, 3);
        let p = img.SetPixel(1, 0);
        p[0] = 10;
        p[1] = 20;
        p[2] = 30;
        assert_eq!(img.GetPixel(1, 0), &[10, 20, 30]);
    }

    #[test]
    fn fill_rgba() {
        let mut img = emImage::new(3, 2, 4);
        img.fill(emColor::RED);
        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(img.GetPixel(x, y), &[255, 0, 0, 255]);
            }
        }
    }

    #[test]
    #[should_panic(expected = "channel_count must be 1, 2, 3, or 4")]
    fn invalid_channel_count() {
        emImage::new(1, 1, 0);
    }

    #[test]
    fn fill_non_rgba() {
        let mut img = emImage::new(2, 1, 3);
        img.fill(emColor::rgb(10, 20, 30));
        assert_eq!(img.GetPixel(0, 0), &[10, 20, 30]);
        assert_eq!(img.GetPixel(1, 0), &[10, 20, 30]);

        let mut img1 = emImage::new(2, 1, 1);
        img1.fill(emColor::rgb(10, 20, 30));
        // (10+20+30+1)/3 = 20
        assert_eq!(img1.GetPixel(0, 0), &[20]);

        let mut img2 = emImage::new(2, 1, 2);
        img2.fill(emColor::rgba(10, 20, 30, 128));
        assert_eq!(img2.GetPixel(0, 0), &[20, 128]);
    }

    #[test]
    fn from_raw_valid() {
        let data = vec![10, 20, 30, 255, 40, 50, 60, 128];
        let img = emImage::from_raw(2, 1, 4, data);
        assert_eq!(img.GetPixel(0, 0), &[10, 20, 30, 255]);
        assert_eq!(img.GetPixel(1, 0), &[40, 50, 60, 128]);
    }

    #[test]
    #[should_panic(expected = "does not match")]
    fn from_raw_wrong_length() {
        emImage::from_raw(2, 2, 4, vec![0; 15]);
    }

    #[test]
    fn single_channel() {
        let mut img = emImage::new(2, 2, 1);
        img.SetPixel(0, 0)[0] = 128;
        assert_eq!(img.GetPixel(0, 0), &[128]);
        assert_eq!(img.GetPixel(1, 0), &[0]);
    }

    #[test]
    fn partial_eq() {
        let a = emImage::new(2, 2, 4);
        let b = emImage::new(2, 2, 4);
        assert_eq!(a, b);
        let c = emImage::new(3, 2, 4);
        assert_ne!(a, c);
    }

    #[test]
    fn setup_and_clear() {
        let mut img = emImage::new(4, 4, 4);
        img.fill(emColor::RED);
        img.setup(2, 3, 1);
        assert_eq!(img.GetWidth(), 2);
        assert_eq!(img.GetHeight(), 3);
        assert_eq!(img.GetChannelCount(), 1);
        assert!(img.GetMap().iter().all(|&b| b == 0));

        img.Clear();
        assert_eq!(img.GetWidth(), 0);
        assert_eq!(img.GetHeight(), 0);
        assert!(img.IsEmpty());
    }

    #[test]
    fn pixel_channel_round_trip() {
        let mut img = emImage::new(3, 3, 4);
        img.set_pixel_channel(1, 2, 2, 42);
        assert_eq!(img.get_pixel_channel(1, 2, 2), 42);
        assert_eq!(img.get_pixel_channel(1, 2, 0), 0);
    }

    #[test]
    fn fill_rect_region_isolation() {
        let mut img = emImage::new(4, 4, 4);
        img.Fill(1, 1, 2, 2, emColor::RED);
        // Inside rect
        assert_eq!(img.GetPixel(1, 1), &[255, 0, 0, 255]);
        assert_eq!(img.GetPixel(2, 2), &[255, 0, 0, 255]);
        // Outside rect
        assert_eq!(img.GetPixel(0, 0), &[0, 0, 0, 0]);
        assert_eq!(img.GetPixel(3, 3), &[0, 0, 0, 0]);
    }

    #[test]
    fn fill_channel_works() {
        let mut img = emImage::new(2, 2, 3);
        img.fill_channel(1, 128);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(img.GetPixel(x, y), &[0, 128, 0]);
            }
        }
    }

    #[test]
    fn fill_channel_rect_clips() {
        let mut img = emImage::new(3, 3, 1);
        img.FillChannel(0, 1, 1, 100, 100, 77);
        assert_eq!(img.get_pixel_channel(0, 0, 0), 0);
        assert_eq!(img.get_pixel_channel(1, 1, 0), 77);
        assert_eq!(img.get_pixel_channel(2, 2, 0), 77);
    }

    #[test]
    fn copy_from_correctness() {
        let mut src = emImage::new(2, 2, 4);
        src.fill(emColor::rgb(10, 20, 30));
        let mut dst = emImage::new(4, 4, 4);
        dst.Copy(1, 1, &src);
        assert_eq!(dst.GetPixel(0, 0), &[0, 0, 0, 0]);
        assert_eq!(dst.GetPixel(1, 1), &[10, 20, 30, 255]);
        assert_eq!(dst.GetPixel(2, 2), &[10, 20, 30, 255]);
        assert_eq!(dst.GetPixel(3, 3), &[0, 0, 0, 0]);
    }

    #[test]
    fn get_cropped_extraction() {
        let mut img = emImage::new(4, 4, 1);
        img.set_pixel_channel(2, 1, 0, 99);
        let sub = img.get_cropped(1, 1, 2, 2, None);
        assert_eq!(sub.GetWidth(), 2);
        assert_eq!(sub.GetHeight(), 2);
        assert_eq!(sub.get_pixel_channel(1, 0, 0), 99);
    }

    #[test]
    fn get_converted_1_to_4() {
        let mut img = emImage::new(1, 1, 1);
        img.SetPixel(0, 0)[0] = 128;
        let rgba = img.get_converted(4);
        assert_eq!(rgba.GetPixel(0, 0), &[128, 128, 128, 255]);
    }

    #[test]
    fn get_converted_4_to_1() {
        let mut img = emImage::new(1, 1, 4);
        img.SetPixel(0, 0).copy_from_slice(&[30, 60, 90, 255]);
        let grey = img.get_converted(1);
        assert_eq!(grey.GetPixel(0, 0), &[60]); // (30+60+90)/3 = 60
    }

    #[test]
    fn get_converted_3_to_4() {
        let mut img = emImage::new(1, 1, 3);
        img.SetPixel(0, 0).copy_from_slice(&[10, 20, 30]);
        let rgba = img.get_converted(4);
        assert_eq!(rgba.GetPixel(0, 0), &[10, 20, 30, 255]);
    }

    #[test]
    fn get_converted_4_to_3() {
        let mut img = emImage::new(1, 1, 4);
        img.SetPixel(0, 0).copy_from_slice(&[10, 20, 30, 128]);
        let rgb = img.get_converted(3);
        assert_eq!(rgb.GetPixel(0, 0), &[10, 20, 30]);
    }

    #[test]
    fn has_any_non_grey_pixel_detects() {
        let mut img = emImage::new(2, 1, 3);
        img.SetPixel(0, 0).copy_from_slice(&[50, 50, 50]);
        img.SetPixel(1, 0).copy_from_slice(&[50, 50, 50]);
        assert!(!img.has_any_non_grey_pixel());
        img.SetPixel(1, 0)[1] = 51;
        assert!(img.has_any_non_grey_pixel());
    }

    #[test]
    fn has_any_transparent_pixel_detects() {
        let mut img = emImage::new(2, 1, 4);
        img.fill(emColor::rgb(0, 0, 0)); // all alpha=255
        assert!(!img.has_any_transparent_pixel());
        img.set_pixel_channel(1, 0, 3, 254);
        assert!(img.has_any_transparent_pixel());
    }

    #[test]
    fn calc_min_max_rect_basic() {
        let mut img = emImage::new(4, 4, 4);
        img.fill(emColor::BLACK);
        img.SetPixel(1, 1).copy_from_slice(&[255, 0, 0, 255]);
        img.SetPixel(2, 2).copy_from_slice(&[0, 255, 0, 255]);
        let r = img.calc_min_max_rect(emColor::BLACK).unwrap();
        assert_eq!(r, (1, 1, 2, 2));
    }

    #[test]
    fn calc_min_max_rect_all_bg() {
        let mut img = emImage::new(3, 3, 4);
        img.fill(emColor::WHITE);
        assert_eq!(img.calc_min_max_rect(emColor::WHITE), None);
    }

    #[test]
    fn calc_alpha_min_max_rect_works() {
        let mut img = emImage::new(4, 4, 4);
        // All alpha=0 initially
        img.set_pixel_channel(2, 1, 3, 255);
        img.set_pixel_channel(3, 3, 3, 128);
        let r = img.calc_alpha_min_max_rect().unwrap();
        assert_eq!(r, (2, 1, 2, 3));
    }

    #[test]
    fn get_cropped_by_alpha_works() {
        let mut img = emImage::new(4, 4, 4);
        img.set_pixel_channel(1, 1, 3, 255);
        img.SetPixel(1, 1).copy_from_slice(&[10, 20, 30, 255]);
        let cropped = img.get_cropped_by_alpha(None);
        assert_eq!(cropped.GetWidth(), 1);
        assert_eq!(cropped.GetHeight(), 1);
        assert_eq!(cropped.GetPixel(0, 0), &[10, 20, 30, 255]);
    }

    #[test]
    fn determine_all_colors_sorted_works() {
        let mut img = emImage::new(3, 1, 4);
        img.SetPixel(0, 0).copy_from_slice(&[0, 0, 255, 255]); // blue
        img.SetPixel(1, 0).copy_from_slice(&[255, 0, 0, 255]); // red
        img.SetPixel(2, 0).copy_from_slice(&[0, 0, 255, 255]); // blue dup
        let colors = img.determine_all_colors_sorted(1000);
        assert_eq!(colors.len(), 2);
        // Red (0xFF0000FF) > Blue (0x0000FFFF) in packed u32
        assert_eq!(colors[0], emColor::rgb(0, 0, 255));
        assert_eq!(colors[1], emColor::rgb(255, 0, 0));
    }

    #[test]
    fn copy_channel_cross_cc() {
        let mut src = emImage::new(2, 2, 1);
        src.SetPixel(0, 0)[0] = 42;
        src.SetPixel(1, 1)[0] = 99;
        let mut dst = emImage::new(3, 3, 4);
        dst.CopyChannel(2, 0, 0, &src, 0); // copy src ch0 into dst ch2 (blue)
        assert_eq!(dst.get_pixel_channel(0, 0, 2), 42);
        assert_eq!(dst.get_pixel_channel(1, 1, 2), 99);
        assert_eq!(dst.get_pixel_channel(0, 0, 0), 0); // red untouched
    }

    #[test]
    fn get_pixel_interpolated_in_bounds() {
        let mut img = emImage::new(2, 2, 4);
        img.fill(emColor::RED);
        let c = img.get_pixel_interpolated(0.0, 0.0, 1.0, 1.0, emColor::BLUE);
        assert_eq!(c.GetRed(), 255);
        assert_eq!(c.GetGreen(), 0);
        assert_eq!(c.GetBlue(), 0);
    }

    #[test]
    fn get_pixel_interpolated_out_of_bounds() {
        let mut img = emImage::new(2, 2, 4);
        img.fill(emColor::RED);
        let c = img.get_pixel_interpolated(-1.0, -1.0, 1.0, 1.0, emColor::BLUE);
        assert_eq!(c, emColor::BLUE);
    }

    #[test]
    fn get_pixel_interpolated_empty_image() {
        let img = emImage::new(0, 0, 4);
        let c = img.get_pixel_interpolated(0.0, 0.0, 1.0, 1.0, emColor::GREEN);
        assert_eq!(c, emColor::GREEN);
    }

    #[test]
    fn copy_transformed_identity() {
        // Identity matrix: target == source coords
        let mut src = emImage::new(4, 4, 4);
        src.fill(emColor::RED);
        src.SetPixel(1, 1).copy_from_slice(&[0, 255, 0, 255]);

        let mut dst = emImage::new(4, 4, 4);
        // Identity: target_x = 1*src_x + 0*src_y + 0, target_y = 0*src_x + 1*src_y + 0
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        dst.copy_transformed((0, 0, 4, 4), &identity, &src, false, emColor::BLACK);

        assert_eq!(dst.GetPixel(0, 0), &[255, 0, 0, 255]);
        assert_eq!(dst.GetPixel(1, 1), &[0, 255, 0, 255]);
        assert_eq!(dst.GetPixel(3, 3), &[255, 0, 0, 255]);
    }

    #[test]
    fn copy_transformed_translation() {
        // Translate source by (+2, +1)
        let mut src = emImage::new(2, 2, 4);
        src.fill(emColor::rgb(10, 20, 30));

        let mut dst = emImage::new(6, 6, 4);
        // target_x = src_x + 2, target_y = src_y + 1
        let translate = [1.0, 0.0, 2.0, 0.0, 1.0, 1.0];
        dst.copy_transformed((0, 0, 6, 6), &translate, &src, false, emColor::BLACK);

        // Source pixel (0,0) maps to target (2,1)
        assert_eq!(dst.GetPixel(2, 1), &[10, 20, 30, 255]);
        assert_eq!(dst.GetPixel(3, 2), &[10, 20, 30, 255]);
        // Outside source -> bg
        assert_eq!(dst.GetPixel(0, 0), &[0, 0, 0, 255]);
    }

    #[test]
    fn copy_transformed_scale() {
        // Scale 2x: target_x = 2*src_x, target_y = 2*src_y
        let mut src = emImage::new(2, 2, 4);
        src.SetPixel(0, 0).copy_from_slice(&[255, 0, 0, 255]);
        src.SetPixel(1, 0).copy_from_slice(&[0, 255, 0, 255]);
        src.SetPixel(0, 1).copy_from_slice(&[0, 0, 255, 255]);
        src.SetPixel(1, 1).copy_from_slice(&[255, 255, 0, 255]);

        let mut dst = emImage::new(4, 4, 4);
        let scale = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0];
        dst.copy_transformed((0, 0, 4, 4), &scale, &src, false, emColor::BLACK);

        // Source (0,0) maps to target (0,0); nearest for (0,0) -> src(0,0)
        assert_eq!(dst.GetPixel(0, 0), &[255, 0, 0, 255]);
        // Source (1,0) maps to target (2,0); nearest for (2,0) -> src(1,0)
        assert_eq!(dst.GetPixel(2, 0), &[0, 255, 0, 255]);
    }

    #[test]
    fn copy_transformed_interpolated() {
        // Simple identity with interpolation
        let mut src = emImage::new(2, 2, 4);
        src.fill(emColor::rgb(100, 100, 100));

        let mut dst = emImage::new(2, 2, 4);
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        dst.copy_transformed((0, 0, 2, 2), &identity, &src, true, emColor::BLACK);

        // With bilinear on a uniform image, result should be same
        assert_eq!(dst.GetPixel(0, 0), &[100, 100, 100, 255]);
        assert_eq!(dst.GetPixel(1, 1), &[100, 100, 100, 255]);
    }

    #[test]
    fn copy_transformed_clips_target() {
        // Clip rectangle smaller than target
        let mut src = emImage::new(4, 4, 4);
        src.fill(emColor::RED);

        let mut dst = emImage::new(4, 4, 4);
        dst.fill(emColor::BLACK);
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        // Only transform the top-left 2x2 region
        dst.copy_transformed((0, 0, 2, 2), &identity, &src, false, emColor::BLUE);

        assert_eq!(dst.GetPixel(0, 0), &[255, 0, 0, 255]); // transformed
        assert_eq!(dst.GetPixel(1, 1), &[255, 0, 0, 255]); // transformed
        assert_eq!(dst.GetPixel(2, 2), &[0, 0, 0, 255]); // untouched
        assert_eq!(dst.GetPixel(3, 3), &[0, 0, 0, 255]); // untouched
    }

    #[test]
    fn copy_transformed_singular_matrix() {
        // Degenerate matrix (all zeros) should fill with bg color
        let src = emImage::new(2, 2, 4);
        let mut dst = emImage::new(4, 4, 4);
        let singular = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        dst.copy_transformed((1, 1, 2, 2), &singular, &src, false, emColor::rgb(42, 42, 42));

        assert_eq!(dst.GetPixel(1, 1), &[42, 42, 42, 255]);
        assert_eq!(dst.GetPixel(2, 2), &[42, 42, 42, 255]);
        assert_eq!(dst.GetPixel(0, 0), &[0, 0, 0, 0]); // outside clip
    }

    #[test]
    fn copy_transformed_zero_size_noop() {
        let src = emImage::new(2, 2, 4);
        let mut dst = emImage::new(4, 4, 4);
        dst.fill(emColor::WHITE);
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        // Zero-width clip should be a no-op
        dst.copy_transformed((0, 0, 0, 4), &identity, &src, false, emColor::BLACK);
        assert_eq!(dst.GetPixel(0, 0), &[255, 255, 255, 255]);
    }
}
