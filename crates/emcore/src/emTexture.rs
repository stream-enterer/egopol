use crate::emCore::emColor::emColor;
use crate::emCore::emImage::emImage;

/// How to extend an image beyond its bounds.
///
/// Matches C++ `emTexture::ExtensionType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageExtension {
    /// Clamp to edge pixels (C++ `EXTEND_EDGE`).
    Clamp,
    /// Repeat (tile) (C++ `EXTEND_TILED`).
    Repeat,
    /// Zero/transparent beyond bounds (C++ `EXTEND_ZERO`).
    Zero,
    /// Auto-resolve: Zero if the image has alpha or the texture uses a
    /// transparent gradient color; otherwise Clamp.  C++ `EXTEND_EDGE_OR_ZERO`.
    EdgeOrZero,
}

impl ImageExtension {
    /// Resolve `EdgeOrZero` into a concrete variant for `paint_image_colored`.
    ///
    /// C++ rule (emTexture.h:102-107): if IMAGE_COLORED and one of the gradient
    /// colors has zero alpha → EXTEND_ZERO, else if image has alpha channel →
    /// EXTEND_ZERO, else EXTEND_EDGE.
    pub(crate) fn resolve_for_colored(self, color1: emColor, color2: emColor) -> Self {
        match self {
            Self::EdgeOrZero => {
                if color1.GetAlpha() == 0 || color2.GetAlpha() == 0 {
                    Self::Zero
                } else {
                    Self::Clamp
                }
            }
            other => other,
        }
    }
}

/// Quality hint for image rendering.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageQuality {
    /// Nearest-neighbor sampling.
    Nearest,
    /// Bilinear interpolation.
    Bilinear,
    /// Box filter for downscaling.
    AreaSampled,
    /// Catmull-Rom bicubic (4x4 kernel).
    Bicubic,
    /// Windowed sinc (4-tap).
    Lanczos,
    /// Edge-sensitive adaptive (Hermite/bicubic blend).
    Adaptive,
}

/// A texture describes how a shape is filled.
#[derive(Clone, Debug)]
pub enum emTexture {
    /// Solid color fill.
    SolidColor(emColor),
    /// emImage fill with extension and quality options.
    emImage {
        image: emImage,
        extension: ImageExtension,
        quality: ImageQuality,
    },
    /// Linear gradient between two colors.
    LinearGradient {
        color_a: emColor,
        color_b: emColor,
        /// Start point (x, y) in local coordinates.
        start: (f64, f64),
        /// End point (x, y) in local coordinates.
        end: (f64, f64),
    },
    /// Radial gradient between two colors.
    RadialGradient {
        color_inner: emColor,
        color_outer: emColor,
        /// Center (x, y) in local coordinates.
        center: (f64, f64),
        /// Radius.
        radius: f64,
    },
    /// emImage tinted with a color (multiplied).
    ImageColored {
        image: emImage,
        color: emColor,
        extension: ImageExtension,
        quality: ImageQuality,
    },
}

impl emTexture {
    /// Create a solid color texture.
    pub fn GetColor(c: emColor) -> Self {
        emTexture::SolidColor(c)
    }
}
