use std::fmt;
use std::str::FromStr;

/// Error returned when parsing a hex color string fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorParseError {
    _private: (),
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid color string: expected #RRGGBB or #RRGGBBAA")
    }
}

impl std::error::Error for ColorParseError {}

/// RGBA color packed into a `u32` with layout R[31:24] G[23:16] B[15:8] A[7:0].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color(u32);

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const TRANSPARENT: Color = Color(0);

    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self((r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | a as u32)
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    #[inline]
    pub const fn r(self) -> u8 {
        (self.0 >> 24) as u8
    }

    #[inline]
    pub const fn g(self) -> u8 {
        (self.0 >> 16) as u8
    }

    #[inline]
    pub const fn b(self) -> u8 {
        (self.0 >> 8) as u8
    }

    #[inline]
    pub const fn a(self) -> u8 {
        self.0 as u8
    }

    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Create a color from HSV values. `h` in [0, 360), `s` and `v` in [0, 1].
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let h = ((h % 360.0) + 360.0) % 360.0;

        let c = v * s;
        let h_prime = h / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        let m = v - c;

        let (r1, g1, b1) = match h_prime as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::rgb(
            ((r1 + m) * 255.0 + 0.5) as u8,
            ((g1 + m) * 255.0 + 0.5) as u8,
            ((b1 + m) * 255.0 + 0.5) as u8,
        )
    }

    /// Convert to HSV. Returns `(h, s, v)` with h in [0, 360), s and v in [0, 1].
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let r = self.r() as f32 / 255.0;
        let g = self.g() as f32 / 255.0;
        let b = self.b() as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0 + 6.0) % 360.0
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        let s = if max == 0.0 { 0.0 } else { delta / max };

        (h, s, max)
    }

    /// Lighten the color by mixing with white. `amount` in [0.0, 1.0].
    pub fn lighten(self, amount: f64) -> Color {
        self.lerp(Color::WHITE, amount)
    }

    /// Darken the color by mixing with black. `amount` in [0.0, 1.0].
    pub fn darken(self, amount: f64) -> Color {
        self.lerp(Color::BLACK, amount)
    }

    /// Standard alpha blend: `self` over `other` using `alpha` (0–255).
    ///
    /// Uses `/256` integer math matching C++ emPainter precision.
    pub fn blend(self, other: Color, alpha: u8) -> Color {
        let a = alpha as u16;
        let inv_a = 256 - a;
        let r = (self.r() as u16 * a + other.r() as u16 * inv_a) >> 8;
        let g = (self.g() as u16 * a + other.g() as u16 * inv_a) >> 8;
        let b = (self.b() as u16 * a + other.b() as u16 * inv_a) >> 8;
        let out_a = (self.a() as u16 * a + other.a() as u16 * inv_a) >> 8;
        Color::rgba(r as u8, g as u8, b as u8, out_a as u8)
    }

    /// Return a copy with the alpha channel replaced.
    #[inline]
    pub const fn with_alpha(self, a: u8) -> Color {
        Color::rgba(self.r(), self.g(), self.b(), a)
    }

    /// Linearly interpolate between `self` and `other` by factor `t` (0.0–1.0).
    ///
    /// Uses integer math with 8-bit fractional weight to match C++ emPainter
    /// gradient precision: `result = a + (b - a) * t256 / 256`.
    pub fn lerp(self, other: Color, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let t256 = (t * 256.0) as i32;
        let r = (self.r() as i32 + (other.r() as i32 - self.r() as i32) * t256 / 256) as u8;
        let g = (self.g() as i32 + (other.g() as i32 - self.g() as i32) * t256 / 256) as u8;
        let b = (self.b() as i32 + (other.b() as i32 - self.b() as i32) * t256 / 256) as u8;
        let a = (self.a() as i32 + (other.a() as i32 - self.a() as i32) * t256 / 256) as u8;
        Color::rgba(r, g, b, a)
    }

    /// emCore canvas blend: `target += (source - canvas) * alpha / 256`.
    ///
    /// `self` is the current target pixel, `source` is the color being painted,
    /// `canvas` is the background canvas color, `alpha` is blend strength (0–255).
    /// Uses `/256` (right-shift by 8) matching C++ emPainter integer precision.
    pub fn canvas_blend(self, source: Color, canvas: Color, alpha: u8) -> Color {
        let a = alpha as i32;
        let blend_ch = |target: u8, src: u8, cvs: u8| -> u8 {
            let result = target as i32 + ((src as i32 - cvs as i32) * a) / 256;
            result.clamp(0, 255) as u8
        };
        Color::rgba(
            blend_ch(self.r(), source.r(), canvas.r()),
            blend_ch(self.g(), source.g(), canvas.g()),
            blend_ch(self.b(), source.b(), canvas.b()),
            blend_ch(self.a(), source.a(), canvas.a()),
        )
    }

    /// Return a copy with the red channel replaced.
    #[inline]
    pub const fn with_red(self, r: u8) -> Color {
        Color::rgba(r, self.g(), self.b(), self.a())
    }

    /// Return a copy with the green channel replaced.
    #[inline]
    pub const fn with_green(self, g: u8) -> Color {
        Color::rgba(self.r(), g, self.b(), self.a())
    }

    /// Return a copy with the blue channel replaced.
    #[inline]
    pub const fn with_blue(self, b: u8) -> Color {
        Color::rgba(self.r(), self.g(), b, self.a())
    }

    /// Returns `true` if the alpha channel is zero.
    #[inline]
    pub const fn is_transparent(self) -> bool {
        self.a() == 0
    }

    /// Returns `true` if the alpha channel is 255.
    #[inline]
    pub const fn is_opaque(self) -> bool {
        self.a() == 255
    }

    /// Returns `true` if all RGB channels are equal.
    #[inline]
    pub const fn is_grey(self) -> bool {
        self.r() == self.g() && self.g() == self.b()
    }

    /// Average of RGB channels as a grey value.
    pub fn to_grey(self) -> u8 {
        ((self.r() as u16 + self.g() as u16 + self.b() as u16) / 3) as u8
    }

    /// Construct a grey color with `a=255`.
    #[inline]
    pub const fn grey(val: u8) -> Color {
        Color::rgba(val, val, val, 255)
    }

    /// Return a copy with the HSV hue replaced, preserving saturation, value, and alpha.
    pub fn with_hue(self, h: f32) -> Color {
        let (_old_h, s, v) = self.to_hsv();
        Color::from_hsv(h, s, v).with_alpha(self.a())
    }

    /// Return a copy with the HSV saturation replaced, preserving hue, value, and alpha.
    pub fn with_saturation(self, s: f32) -> Color {
        let (h, _old_s, v) = self.to_hsv();
        Color::from_hsv(h, s, v).with_alpha(self.a())
    }

    /// Return a copy with the HSV value replaced, preserving hue, saturation, and alpha.
    pub fn with_value(self, v: f32) -> Color {
        let (h, s, _old_v) = self.to_hsv();
        Color::from_hsv(h, s, v).with_alpha(self.a())
    }

    /// Scale alpha by `amount` in \[-100, 100\].
    /// Positive values make more transparent, negative values make more opaque.
    pub fn transparented(self, amount: f64) -> Color {
        let amount = amount.clamp(-100.0, 100.0);
        let a = self.a() as f64;
        let new_a = if amount >= 0.0 {
            a * (1.0 - amount / 100.0)
        } else {
            a + (255.0 - a) * (-amount / 100.0)
        };
        self.with_alpha((new_a + 0.5) as u8)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_opaque() {
            write!(f, "#{:02X}{:02X}{:02X}", self.r(), self.g(), self.b())
        } else {
            write!(
                f,
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r(),
                self.g(),
                self.b(),
                self.a()
            )
        }
    }
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError { _private: () };
        if !s.starts_with('#') {
            return Err(err());
        }
        let hex = &s[1..];
        match hex.len() {
            6 => {
                let val = u32::from_str_radix(hex, 16).map_err(|_| err())?;
                Ok(Color::rgb((val >> 16) as u8, (val >> 8) as u8, val as u8))
            }
            8 => {
                let val = u32::from_str_radix(hex, 16).map_err(|_| err())?;
                Ok(Color::rgba(
                    (val >> 24) as u8,
                    (val >> 16) as u8,
                    (val >> 8) as u8,
                    val as u8,
                ))
            }
            _ => Err(err()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_access() {
        let c = Color::rgba(10, 20, 30, 40);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 30);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn rgb_sets_alpha_255() {
        let c = Color::rgb(1, 2, 3);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn named_constants() {
        assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
        assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
        assert_eq!(Color::TRANSPARENT.a(), 0);
    }

    #[test]
    fn blend_extremes() {
        let a = Color::rgb(255, 0, 0);
        let b = Color::rgb(0, 0, 255);
        // Full alpha -> nearly source (C++ /256 precision: 255*255/256 = 254)
        let full = a.blend(b, 255);
        assert!((full.r() as i16 - a.r() as i16).abs() <= 1);
        assert!((full.b() as i16 - a.b() as i16).abs() <= 1);
        // Zero alpha -> dest
        assert_eq!(a.blend(b, 0), b);
    }

    #[test]
    fn canvas_blend_identity() {
        let target = Color::rgb(100, 100, 100);
        // source == canvas -> no change
        let result = target.canvas_blend(Color::rgb(50, 50, 50), Color::rgb(50, 50, 50), 255);
        assert_eq!(result.r(), 100);
        assert_eq!(result.g(), 100);
        assert_eq!(result.b(), 100);
    }

    #[test]
    fn hsv_round_trip() {
        let original = Color::rgb(200, 100, 50);
        let (h, s, v) = original.to_hsv();
        let reconstructed = Color::from_hsv(h, s, v);
        // Allow ±1 due to rounding
        assert!((original.r() as i16 - reconstructed.r() as i16).abs() <= 1);
        assert!((original.g() as i16 - reconstructed.g() as i16).abs() <= 1);
        assert!((original.b() as i16 - reconstructed.b() as i16).abs() <= 1);
    }

    #[test]
    fn hsv_pure_colors() {
        let (h, s, v) = Color::RED.to_hsv();
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);

        let (h, _, _) = Color::GREEN.to_hsv();
        assert!((h - 120.0).abs() < 1.0);

        let (h, _, _) = Color::BLUE.to_hsv();
        assert!((h - 240.0).abs() < 1.0);
    }

    #[test]
    fn with_red_preserves_other_channels() {
        let c = Color::rgba(10, 20, 30, 40).with_red(99);
        assert_eq!(c.r(), 99);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 30);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn with_green_preserves_other_channels() {
        let c = Color::rgba(10, 20, 30, 40).with_green(99);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 99);
        assert_eq!(c.b(), 30);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn with_blue_preserves_other_channels() {
        let c = Color::rgba(10, 20, 30, 40).with_blue(99);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 99);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn query_methods() {
        assert!(Color::TRANSPARENT.is_transparent());
        assert!(!Color::BLACK.is_transparent());
        assert!(Color::WHITE.is_opaque());
        assert!(!Color::rgba(0, 0, 0, 128).is_opaque());
        assert!(Color::grey(128).is_grey());
        assert!(!Color::RED.is_grey());
    }

    #[test]
    fn grey_round_trip() {
        let g = Color::grey(128);
        assert_eq!(g.r(), 128);
        assert_eq!(g.g(), 128);
        assert_eq!(g.b(), 128);
        assert_eq!(g.a(), 255);
        assert_eq!(g.to_grey(), 128);
    }

    #[test]
    fn to_grey_averages() {
        let c = Color::rgb(10, 20, 30);
        assert_eq!(c.to_grey(), 20); // (10+20+30)/3 = 20
    }

    #[test]
    fn with_hue_preserves_sv() {
        let c = Color::from_hsv(120.0, 0.8, 0.6);
        let shifted = c.with_hue(240.0);
        let (h, s, v) = shifted.to_hsv();
        assert!((h - 240.0).abs() < 2.0);
        assert!((s - 0.8).abs() < 0.02);
        assert!((v - 0.6).abs() < 0.02);
    }

    #[test]
    fn with_saturation_preserves_hv() {
        let c = Color::from_hsv(120.0, 0.8, 0.6);
        let changed = c.with_saturation(0.3);
        let (h, s, v) = changed.to_hsv();
        assert!((h - 120.0).abs() < 2.0);
        assert!((s - 0.3).abs() < 0.02);
        assert!((v - 0.6).abs() < 0.02);
    }

    #[test]
    fn with_value_preserves_hs() {
        let c = Color::from_hsv(120.0, 0.8, 0.6);
        let changed = c.with_value(0.9);
        let (h, s, v) = changed.to_hsv();
        assert!((h - 120.0).abs() < 2.0);
        assert!((s - 0.8).abs() < 0.02);
        assert!((v - 0.9).abs() < 0.02);
    }

    #[test]
    fn with_hue_preserves_alpha() {
        let c = Color::rgba(100, 50, 50, 128);
        let shifted = c.with_hue(180.0);
        assert_eq!(shifted.a(), 128);
    }

    #[test]
    fn transparented_extremes() {
        let c = Color::rgba(100, 100, 100, 200);
        let fully = c.transparented(100.0);
        assert_eq!(fully.a(), 0);
        let none = c.transparented(0.0);
        assert_eq!(none.a(), 200);
        let opaque = Color::rgba(100, 100, 100, 0).transparented(-100.0);
        assert_eq!(opaque.a(), 255);
    }

    #[test]
    fn display_opaque() {
        assert_eq!(format!("{}", Color::rgb(255, 128, 0)), "#FF8000");
    }

    #[test]
    fn display_with_alpha() {
        assert_eq!(format!("{}", Color::rgba(255, 128, 0, 128)), "#FF800080");
    }

    #[test]
    fn from_str_round_trip() {
        let c = Color::rgba(10, 200, 30, 128);
        let s = format!("{}", c);
        let parsed: Color = s.parse().unwrap();
        assert_eq!(parsed, c);

        let opaque = Color::rgb(255, 0, 128);
        let s2 = format!("{}", opaque);
        let parsed2: Color = s2.parse().unwrap();
        assert_eq!(parsed2, opaque);
    }

    #[test]
    fn from_str_rejects_invalid() {
        assert!("not a color".parse::<Color>().is_err());
        assert!("#GG0000".parse::<Color>().is_err());
        assert!("#12345".parse::<Color>().is_err());
        assert!("#123456789".parse::<Color>().is_err());
        assert!("".parse::<Color>().is_err());
    }
}
