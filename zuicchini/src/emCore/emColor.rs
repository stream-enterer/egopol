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
pub struct emColor(u32);

impl emColor {
    pub const BLACK: emColor = emColor::rgb(0, 0, 0);
    pub const WHITE: emColor = emColor::rgb(255, 255, 255);
    pub const RED: emColor = emColor::rgb(255, 0, 0);
    pub const GREEN: emColor = emColor::rgb(0, 255, 0);
    pub const BLUE: emColor = emColor::rgb(0, 0, 255);
    pub const GRAY: emColor = emColor::rgb(128, 128, 128);
    pub const YELLOW: emColor = emColor::rgb(255, 255, 0);
    pub const CYAN: emColor = emColor::rgb(0, 255, 255);
    pub const MAGENTA: emColor = emColor::rgb(255, 0, 255);
    pub const TRANSPARENT: emColor = emColor(0);

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
    pub fn lighten(self, amount: f64) -> emColor {
        self.lerp(emColor::WHITE, amount)
    }

    /// Darken the color by mixing with black. `amount` in [0.0, 1.0].
    pub fn darken(self, amount: f64) -> emColor {
        self.lerp(emColor::BLACK, amount)
    }

    /// Standard alpha blend: `self` over `other` using `alpha` (0–255).
    ///
    /// Uses `/256` integer math matching C++ emPainter precision.
    pub fn blend(self, other: emColor, alpha: u8) -> emColor {
        let a = alpha as u16;
        let inv_a = 256 - a;
        let r = (self.r() as u16 * a + other.r() as u16 * inv_a) >> 8;
        let g = (self.g() as u16 * a + other.g() as u16 * inv_a) >> 8;
        let b = (self.b() as u16 * a + other.b() as u16 * inv_a) >> 8;
        let out_a = (self.a() as u16 * a + other.a() as u16 * inv_a) >> 8;
        emColor::rgba(r as u8, g as u8, b as u8, out_a as u8)
    }

    /// Return a copy with the alpha channel replaced.
    #[inline]
    pub const fn with_alpha(self, a: u8) -> emColor {
        emColor::rgba(self.r(), self.g(), self.b(), a)
    }

    /// Linearly interpolate between `self` and `other` by factor `t` (0.0–1.0).
    ///
    /// Matches C++ `emColor::GetBlended(color, weight)` with 16-bit precision:
    /// `w2 = (int)(weight * 655.36 + 0.5)`, `result = (a*w1 + b*w2 + 32768) >> 16`.
    /// C++ weight is 0–100; our `t` is 0.0–1.0, so `t * 100.0 * 655.36 = t * 65536.0`.
    pub fn lerp(self, other: emColor, t: f64) -> emColor {
        let t = t.clamp(0.0, 1.0);
        let w2 = (t * 65536.0 + 0.5) as i32;
        let w1 = 65536 - w2;
        let mix = |a: i32, b: i32| -> u8 { ((a * w1 + b * w2 + 32768) >> 16) as u8 };
        emColor::rgba(
            mix(self.r() as i32, other.r() as i32),
            mix(self.g() as i32, other.g() as i32),
            mix(self.b() as i32, other.b() as i32),
            mix(self.a() as i32, other.a() as i32),
        )
    }

    /// emCore canvas blend: `target += round(source*alpha/255) - round(canvas*alpha/255)`.
    ///
    /// `self` is the current target pixel, `source` is the color being painted,
    /// `canvas` is the background canvas color, `alpha` is blend strength (0–255).
    /// Uses two separate round-to-nearest `/255` terms matching C++ emPainter hash table.
    pub fn canvas_blend(self, source: emColor, canvas: emColor, alpha: u8) -> emColor {
        let a = alpha as i32;
        let blend_ch = |target: u8, src: u8, cvs: u8| -> u8 {
            let src_term = (src as i32 * a + 127) / 255;
            let cvs_term = (cvs as i32 * a + 127) / 255;
            (target as i32 + src_term - cvs_term).clamp(0, 255) as u8
        };
        emColor::rgba(
            blend_ch(self.r(), source.r(), canvas.r()),
            blend_ch(self.g(), source.g(), canvas.g()),
            blend_ch(self.b(), source.b(), canvas.b()),
            blend_ch(self.a(), source.a(), canvas.a()),
        )
    }

    /// Return a copy with the red channel replaced.
    #[inline]
    pub const fn with_red(self, r: u8) -> emColor {
        emColor::rgba(r, self.g(), self.b(), self.a())
    }

    /// Return a copy with the green channel replaced.
    #[inline]
    pub const fn with_green(self, g: u8) -> emColor {
        emColor::rgba(self.r(), g, self.b(), self.a())
    }

    /// Return a copy with the blue channel replaced.
    #[inline]
    pub const fn with_blue(self, b: u8) -> emColor {
        emColor::rgba(self.r(), self.g(), b, self.a())
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
    /// Uses C++ `GetGrey` rounding: `(r + g + b + 1) / 3`.
    pub fn to_grey(self) -> u8 {
        ((self.r() as u16 + self.g() as u16 + self.b() as u16 + 1) / 3) as u8
    }

    /// Construct a grey color with `a=255`.
    #[inline]
    pub const fn grey(val: u8) -> emColor {
        emColor::rgba(val, val, val, 255)
    }

    /// Return a copy with the HSV hue replaced, preserving saturation, value, and alpha.
    pub fn with_hue(self, h: f32) -> emColor {
        let (_old_h, s, v) = self.to_hsv();
        emColor::from_hsv(h, s, v).with_alpha(self.a())
    }

    /// Return a copy with the HSV saturation replaced, preserving hue, value, and alpha.
    pub fn with_saturation(self, s: f32) -> emColor {
        let (h, _old_s, v) = self.to_hsv();
        emColor::from_hsv(h, s, v).with_alpha(self.a())
    }

    /// Return a copy with the HSV value replaced, preserving hue, saturation, and alpha.
    pub fn with_value(self, v: f32) -> emColor {
        let (h, s, _old_v) = self.to_hsv();
        emColor::from_hsv(h, s, v).with_alpha(self.a())
    }

    /// Parse a color string supporting hex formats and X11 named colors.
    ///
    /// Port of C++ `emGetColorFromString`. Supports:
    /// - `#RGB` (3-char hex, 1 digit/channel)
    /// - `#RGBA` (4-char hex)
    /// - `#RRGGBB` (6-char hex)
    /// - `#RRGGBBAA` (8-char hex)
    /// - `#RRRGGGBBB` (9-char hex, 3 digits/channel)
    /// - `#RRRRGGGGBBBB` (12-char hex)
    /// - `#RRRRGGGGBBBBAAAA` (16-char hex)
    /// - `"none"` → transparent grey `rgba(128, 128, 128, 0)`
    /// - X11 named colors (case-insensitive, no spaces)
    pub fn try_parse(s: &str) -> Option<emColor> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("none") {
            return Some(emColor::rgba(128, 128, 128, 0));
        }
        if let Some(hex) = s.strip_prefix('#') {
            return Self::parse_hex(hex);
        }
        // X11 named color lookup (strip spaces, lowercase)
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let rgb = super::emColorX11Colors::lookup_x11_color(&cleaned)?;
        Some(emColor::rgb(rgb[0], rgb[1], rgb[2]))
    }

    /// Parse a hex string (without the '#' prefix) into a emColor.
    fn parse_hex(hex: &str) -> Option<emColor> {
        let len = hex.len();
        // Validate all hex digits
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        match len {
            3 => {
                // #RGB: 1 hex digit per channel, replicate (0xF -> 0xFF)
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(emColor::rgb(r << 4 | r, g << 4 | g, b << 4 | b))
            }
            4 => {
                // #RGBA
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                let a = u8::from_str_radix(&hex[3..4], 16).ok()?;
                Some(emColor::rgba(r << 4 | r, g << 4 | g, b << 4 | b, a << 4 | a))
            }
            6 => {
                // #RRGGBB
                let val = u32::from_str_radix(hex, 16).ok()?;
                Some(emColor::rgb((val >> 16) as u8, (val >> 8) as u8, val as u8))
            }
            8 => {
                // #RRGGBBAA
                let val = u32::from_str_radix(hex, 16).ok()?;
                Some(emColor::rgba(
                    (val >> 24) as u8,
                    (val >> 16) as u8,
                    (val >> 8) as u8,
                    val as u8,
                ))
            }
            9 => {
                // #RRRGGGBBB: 3 hex digits per channel, use high byte
                let r = u16::from_str_radix(&hex[0..3], 16).ok()?;
                let g = u16::from_str_radix(&hex[3..6], 16).ok()?;
                let b = u16::from_str_radix(&hex[6..9], 16).ok()?;
                Some(emColor::rgb((r >> 4) as u8, (g >> 4) as u8, (b >> 4) as u8))
            }
            12 => {
                // #RRRRGGGGBBBB: 4 hex digits per channel, use high byte
                let r = u16::from_str_radix(&hex[0..4], 16).ok()?;
                let g = u16::from_str_radix(&hex[4..8], 16).ok()?;
                let b = u16::from_str_radix(&hex[8..12], 16).ok()?;
                Some(emColor::rgb((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8))
            }
            16 => {
                // #RRRRGGGGBBBBAAAA
                let r = u16::from_str_radix(&hex[0..4], 16).ok()?;
                let g = u16::from_str_radix(&hex[4..8], 16).ok()?;
                let b = u16::from_str_radix(&hex[8..12], 16).ok()?;
                let a = u16::from_str_radix(&hex[12..16], 16).ok()?;
                Some(emColor::rgba(
                    (r >> 8) as u8,
                    (g >> 8) as u8,
                    (b >> 8) as u8,
                    (a >> 8) as u8,
                ))
            }
            _ => None,
        }
    }

    /// Scale alpha by `amount` in \[-100, 100\].
    /// Positive values make more transparent, negative values make more opaque.
    pub fn transparented(self, amount: f64) -> emColor {
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

impl fmt::Display for emColor {
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

impl FromStr for emColor {
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
                Ok(emColor::rgb((val >> 16) as u8, (val >> 8) as u8, val as u8))
            }
            8 => {
                let val = u32::from_str_radix(hex, 16).map_err(|_| err())?;
                Ok(emColor::rgba(
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
        let c = emColor::rgba(10, 20, 30, 40);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 30);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn rgb_sets_alpha_255() {
        let c = emColor::rgb(1, 2, 3);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn named_constants() {
        assert_eq!(emColor::BLACK, emColor::rgb(0, 0, 0));
        assert_eq!(emColor::WHITE, emColor::rgb(255, 255, 255));
        assert_eq!(emColor::TRANSPARENT.a(), 0);
    }

    #[test]
    fn blend_extremes() {
        let a = emColor::rgb(255, 0, 0);
        let b = emColor::rgb(0, 0, 255);
        // Full alpha -> nearly source (C++ /256 precision: 255*255/256 = 254)
        let full = a.blend(b, 255);
        assert!((full.r() as i16 - a.r() as i16).abs() <= 1);
        assert!((full.b() as i16 - a.b() as i16).abs() <= 1);
        // Zero alpha -> dest
        assert_eq!(a.blend(b, 0), b);
    }

    #[test]
    fn canvas_blend_identity() {
        let target = emColor::rgb(100, 100, 100);
        // source == canvas -> no change
        let result = target.canvas_blend(emColor::rgb(50, 50, 50), emColor::rgb(50, 50, 50), 255);
        assert_eq!(result.r(), 100);
        assert_eq!(result.g(), 100);
        assert_eq!(result.b(), 100);
    }

    #[test]
    fn hsv_round_trip() {
        let original = emColor::rgb(200, 100, 50);
        let (h, s, v) = original.to_hsv();
        let reconstructed = emColor::from_hsv(h, s, v);
        // Allow ±1 due to rounding
        assert!((original.r() as i16 - reconstructed.r() as i16).abs() <= 1);
        assert!((original.g() as i16 - reconstructed.g() as i16).abs() <= 1);
        assert!((original.b() as i16 - reconstructed.b() as i16).abs() <= 1);
    }

    #[test]
    fn hsv_pure_colors() {
        let (h, s, v) = emColor::RED.to_hsv();
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);

        let (h, _, _) = emColor::GREEN.to_hsv();
        assert!((h - 120.0).abs() < 1.0);

        let (h, _, _) = emColor::BLUE.to_hsv();
        assert!((h - 240.0).abs() < 1.0);
    }

    #[test]
    fn with_red_preserves_other_channels() {
        let c = emColor::rgba(10, 20, 30, 40).with_red(99);
        assert_eq!(c.r(), 99);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 30);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn with_green_preserves_other_channels() {
        let c = emColor::rgba(10, 20, 30, 40).with_green(99);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 99);
        assert_eq!(c.b(), 30);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn with_blue_preserves_other_channels() {
        let c = emColor::rgba(10, 20, 30, 40).with_blue(99);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 99);
        assert_eq!(c.a(), 40);
    }

    #[test]
    fn query_methods() {
        assert!(emColor::TRANSPARENT.is_transparent());
        assert!(!emColor::BLACK.is_transparent());
        assert!(emColor::WHITE.is_opaque());
        assert!(!emColor::rgba(0, 0, 0, 128).is_opaque());
        assert!(emColor::grey(128).is_grey());
        assert!(!emColor::RED.is_grey());
    }

    #[test]
    fn grey_round_trip() {
        let g = emColor::grey(128);
        assert_eq!(g.r(), 128);
        assert_eq!(g.g(), 128);
        assert_eq!(g.b(), 128);
        assert_eq!(g.a(), 255);
        assert_eq!(g.to_grey(), 128);
    }

    #[test]
    fn to_grey_averages() {
        let c = emColor::rgb(10, 20, 30);
        assert_eq!(c.to_grey(), 20); // (10+20+30)/3 = 20
    }

    #[test]
    fn with_hue_preserves_sv() {
        let c = emColor::from_hsv(120.0, 0.8, 0.6);
        let shifted = c.with_hue(240.0);
        let (h, s, v) = shifted.to_hsv();
        assert!((h - 240.0).abs() < 2.0);
        assert!((s - 0.8).abs() < 0.02);
        assert!((v - 0.6).abs() < 0.02);
    }

    #[test]
    fn with_saturation_preserves_hv() {
        let c = emColor::from_hsv(120.0, 0.8, 0.6);
        let changed = c.with_saturation(0.3);
        let (h, s, v) = changed.to_hsv();
        assert!((h - 120.0).abs() < 2.0);
        assert!((s - 0.3).abs() < 0.02);
        assert!((v - 0.6).abs() < 0.02);
    }

    #[test]
    fn with_value_preserves_hs() {
        let c = emColor::from_hsv(120.0, 0.8, 0.6);
        let changed = c.with_value(0.9);
        let (h, s, v) = changed.to_hsv();
        assert!((h - 120.0).abs() < 2.0);
        assert!((s - 0.8).abs() < 0.02);
        assert!((v - 0.9).abs() < 0.02);
    }

    #[test]
    fn with_hue_preserves_alpha() {
        let c = emColor::rgba(100, 50, 50, 128);
        let shifted = c.with_hue(180.0);
        assert_eq!(shifted.a(), 128);
    }

    #[test]
    fn transparented_extremes() {
        let c = emColor::rgba(100, 100, 100, 200);
        let fully = c.transparented(100.0);
        assert_eq!(fully.a(), 0);
        let none = c.transparented(0.0);
        assert_eq!(none.a(), 200);
        let opaque = emColor::rgba(100, 100, 100, 0).transparented(-100.0);
        assert_eq!(opaque.a(), 255);
    }

    #[test]
    fn display_opaque() {
        assert_eq!(format!("{}", emColor::rgb(255, 128, 0)), "#FF8000");
    }

    #[test]
    fn display_with_alpha() {
        assert_eq!(format!("{}", emColor::rgba(255, 128, 0, 128)), "#FF800080");
    }

    #[test]
    fn from_str_round_trip() {
        let c = emColor::rgba(10, 200, 30, 128);
        let s = format!("{}", c);
        let parsed: emColor = s.parse().unwrap();
        assert_eq!(parsed, c);

        let opaque = emColor::rgb(255, 0, 128);
        let s2 = format!("{}", opaque);
        let parsed2: emColor = s2.parse().unwrap();
        assert_eq!(parsed2, opaque);
    }

    #[test]
    fn test_blend_interpolates_alpha() {
        // blend(self, other, alpha) uses /256 math on all 4 channels including alpha.
        // self=RGBA(100,100,100,200), other=RGBA(200,200,200,50), alpha=128
        let a = emColor::rgba(100, 100, 100, 200);
        let b = emColor::rgba(200, 200, 200, 50);
        let result = a.blend(b, 128);

        // Expected: out_ch = (self_ch * 128 + other_ch * (256-128)) >> 8
        let expected_a = ((200u16 * 128 + 50u16 * 128) >> 8) as u8;
        assert_eq!(
            result.a(),
            expected_a,
            "blend alpha: got {} expected {}",
            result.a(),
            expected_a
        );

        // Verify RGB uses the same formula (sanity)
        let expected_r = ((100u16 * 128 + 200u16 * 128) >> 8) as u8;
        assert_eq!(result.r(), expected_r);
    }

    #[test]
    fn test_lerp_interpolates_alpha() {
        // lerp RGBA(0,0,0,0) -> RGBA(255,255,255,255) at t=0.5
        let a = emColor::rgba(0, 0, 0, 0);
        let b = emColor::rgba(255, 255, 255, 255);
        let result = a.lerp(b, 0.5);

        // C++ formula: w2 = (0.5 * 65536 + 0.5) as i32 = 32768
        // mix(0, 255) = (0 * (65536-32768) + 255 * 32768 + 32768) >> 16
        //             = (8355840 + 32768) >> 16 = 8388608 >> 16 = 128
        assert_eq!(
            result.a(),
            128,
            "lerp alpha at t=0.5: got {} expected 128",
            result.a()
        );
        // RGB should match alpha since inputs are symmetric
        assert_eq!(result.r(), result.a());
        assert_eq!(result.g(), result.a());
        assert_eq!(result.b(), result.a());

        // Verify endpoints
        let at_zero = a.lerp(b, 0.0);
        assert_eq!(at_zero.a(), 0, "lerp alpha at t=0.0 should be 0");
        let at_one = a.lerp(b, 1.0);
        assert_eq!(at_one.a(), 255, "lerp alpha at t=1.0 should be 255");
    }

    #[test]
    fn test_canvas_blend_computes_alpha() {
        // canvas_blend applies the blend_ch formula to all 4 channels including alpha.
        // target=RGBA(100,100,100,200), source=RGBA(200,200,200,180),
        // canvas=RGBA(80,80,80,255), alpha=128
        let target = emColor::rgba(100, 100, 100, 200);
        let source = emColor::rgba(200, 200, 200, 180);
        let canvas = emColor::rgba(80, 80, 80, 255);
        let result = target.canvas_blend(source, canvas, 128);

        // blend_ch for alpha: target_a + round(src_a * alpha / 255) - round(cvs_a * alpha / 255)
        let src_term = (180i32 * 128 + 127) / 255; // = 90
        let cvs_term = (255i32 * 128 + 127) / 255; // = 128
        let expected_a = (200i32 + src_term - cvs_term).clamp(0, 255) as u8; // 200 + 90 - 128 = 162

        assert_eq!(
            result.a(),
            expected_a,
            "canvas_blend alpha: got {} expected {}",
            result.a(),
            expected_a
        );

        // Verify it's different from input (the blend did modify alpha)
        assert_ne!(result.a(), target.a(), "canvas_blend should modify alpha channel");
    }

    #[test]
    fn from_str_rejects_invalid() {
        assert!("not a color".parse::<emColor>().is_err());
        assert!("#GG0000".parse::<emColor>().is_err());
        assert!("#12345".parse::<emColor>().is_err());
        assert!("#123456789".parse::<emColor>().is_err());
        assert!("".parse::<emColor>().is_err());
    }
}
