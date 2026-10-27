// Color operations - RGB, Oklab, ColorFix
// Port: types/inc/ColorFix.hpp, types/ColorFix.cpp

/// RGB color
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn from_u32(rgb: u32) -> Self {
        Self {
            r: (rgb & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: ((rgb >> 16) & 0xFF) as u8,
        }
    }

    pub const fn to_u32(self) -> u32 {
        (self.r as u32) | ((self.g as u32) << 8) | ((self.b as u32) << 16)
    }

    /// Convert RGB [0-255] to linear RGB [0.0-1.0]
    fn to_linear(srgb: u8) -> f32 {
        let s = srgb as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Convert linear RGB [0.0-1.0] to sRGB [0-255]
    fn from_linear(linear: f32) -> u8 {
        let s = if linear <= 0.0031308 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).clamp(0.0, 255.0) as u8
    }

    /// Convert to Oklab color space
    pub fn to_oklab(self) -> Oklab {
        // Convert sRGB to linear RGB
        let lr = Self::to_linear(self.r);
        let lg = Self::to_linear(self.g);
        let lb = Self::to_linear(self.b);

        // Linear RGB to LMS (cone response)
        let l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
        let m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
        let s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;

        // Apply cube root
        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();

        // LMS to Oklab
        Oklab {
            L: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
        }
    }

    /// Convert from Oklab color space
    pub fn from_oklab(oklab: Oklab) -> Self {
        // Oklab to LMS
        let l_ = oklab.L + 0.3963377774 * oklab.a + 0.2158037573 * oklab.b;
        let m_ = oklab.L - 0.1055613458 * oklab.a - 0.0638541728 * oklab.b;
        let s_ = oklab.L - 0.0894841775 * oklab.a - 1.2914855480 * oklab.b;

        // Cube to get LMS
        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        // LMS to linear RGB
        let lr = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
        let lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
        let lb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

        // Linear RGB to sRGB
        Self {
            r: Self::from_linear(lr),
            g: Self::from_linear(lg),
            b: Self::from_linear(lb),
        }
    }

    /// Fix color for accessibility - adjust lightness if needed
    /// Port of ColorFix algorithm
    pub fn fix_for_contrast(self, background: Color, min_contrast: f32) -> Self {
        let fg_oklab = self.to_oklab();
        let bg_oklab = background.to_oklab();

        let contrast = fg_oklab.contrast_ratio(bg_oklab);
        if contrast >= min_contrast {
            return self; // Already sufficient contrast
        }

        // Adjust lightness to meet contrast requirement
        let target_L = if fg_oklab.L > bg_oklab.L {
            // Foreground brighter - increase lightness
            fg_oklab.L + (min_contrast - contrast) * 0.1
        } else {
            // Foreground darker - decrease lightness
            fg_oklab.L - (min_contrast - contrast) * 0.1
        };

        let adjusted = Oklab {
            L: target_L.clamp(0.0, 1.0),
            a: fg_oklab.a,
            b: fg_oklab.b,
        };

        Color::from_oklab(adjusted)
    }
}

/// Oklab color space - perceptually uniform
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Oklab {
    pub L: f32, // Lightness [0.0-1.0]
    pub a: f32, // Green-red
    pub b: f32, // Blue-yellow
}

impl Oklab {
    pub fn new(L: f32, a: f32, b: f32) -> Self {
        Self { L, a, b }
    }

    /// Approximate contrast ratio (simplified)
    pub fn contrast_ratio(self, other: Oklab) -> f32 {
        let l1 = self.L.max(other.L);
        let l2 = self.L.min(other.L);
        (l1 + 0.05) / (l2 + 0.05)
    }

    /// Distance in Oklab space
    pub fn distance(self, other: Oklab) -> f32 {
        let dL = self.L - other.L;
        let da = self.a - other.a;
        let db = self.b - other.b;
        (dL * dL + da * da + db * db).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_u32() {
        let c = Color::from_u32(0x00FF00); // Green
        assert_eq!(c.r, 0x00);
        assert_eq!(c.g, 0xFF);
        assert_eq!(c.b, 0x00);
    }

    #[test]
    fn color_to_u32() {
        let c = Color::new(0xFF, 0x00, 0x00); // Red
        assert_eq!(c.to_u32(), 0x0000FF);
    }

    #[test]
    fn color_roundtrip_u32() {
        let original = 0xABCDEF;
        let c = Color::from_u32(original);
        assert_eq!(c.to_u32(), original);
    }

    #[test]
    fn color_to_oklab_black() {
        let black = Color::new(0, 0, 0);
        let oklab = black.to_oklab();
        assert!(oklab.L < 0.01); // Near 0
        assert!(oklab.a.abs() < 0.01);
        assert!(oklab.b.abs() < 0.01);
    }

    #[test]
    fn color_to_oklab_white() {
        let white = Color::new(255, 255, 255);
        let oklab = white.to_oklab();
        assert!(oklab.L > 0.99); // Near 1.0
        assert!(oklab.a.abs() < 0.01);
        assert!(oklab.b.abs() < 0.01);
    }

    #[test]
    fn color_oklab_roundtrip() {
        let original = Color::new(128, 64, 192);
        let oklab = original.to_oklab();
        let recovered = Color::from_oklab(oklab);

        // Allow small rounding error (±2)
        assert!((original.r as i32 - recovered.r as i32).abs() <= 2);
        assert!((original.g as i32 - recovered.g as i32).abs() <= 2);
        assert!((original.b as i32 - recovered.b as i32).abs() <= 2);
    }

    #[test]
    fn oklab_contrast_ratio_black_white() {
        let black = Color::new(0, 0, 0).to_oklab();
        let white = Color::new(255, 255, 255).to_oklab();
        let ratio = black.contrast_ratio(white);
        assert!(ratio > 20.0); // WCAG AAA is 7:1, black/white is ~21:1
    }

    #[test]
    fn color_fix_improves_contrast() {
        let fg = Color::new(100, 100, 100); // Dark gray
        let bg = Color::new(80, 80, 80);    // Slightly darker gray

        let fixed = fg.fix_for_contrast(bg, 4.5); // WCAG AA
        let fixed_oklab = fixed.to_oklab();
        let bg_oklab = bg.to_oklab();

        let original_contrast = fg.to_oklab().contrast_ratio(bg_oklab);
        let fixed_contrast = fixed_oklab.contrast_ratio(bg_oklab);

        assert!(fixed_contrast > original_contrast);
    }
}
