//! Verbatim Rust port of Windows Terminal's `CSSLengthPercentage`
//! (`windows/src/renderer/base/CSSLengthPercentage.cpp` +
//! `windows/src/renderer/inc/CSSLengthPercentage.h`).
//!
//! WT uses this tiny CSS-like length type to parse the per-profile
//! `CellWidth` / `CellHeight` settings strings. We reproduce the exact
//! unit-resolution rules so a profile with `"cellWidth": "100%"`,
//! `"cellHeight": "1.2"`, `"cellWidth": "12ch"` or `"cellWidth": "9pt"`
//! yields the *same* pixel value WT's AtlasEngine computes.
//!
//! Reference-frame semantics (copied from the C++ header):
//! - `None`        → empty/unset; `resolve()` returns the fallback.
//! - `Absolute`    → value is in CSS px (`"px"` → ÷96) or pt (`"pt"` → ÷72);
//!   `resolve()` multiplies by the target `dpi`.
//! - `FontSize`    → bare number or `"%"` (percent → ÷100);
//!   `resolve()` multiplies by `font_size`.
//! - `AdvanceWidth`→ `"ch"` unit; `resolve()` multiplies by the `'0'`
//!   glyph advance width.

/// What a `CssLengthPercentage` is relative to. Mirrors WT's
/// `CSSLengthPercentage::ReferenceFrame`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum ReferenceFrame {
    /// Empty / unset — `resolve()` returns the fallback.
    #[default]
    None,
    /// Absolute length (parsed from `"px"` / `"pt"`); scaled by `dpi`.
    Absolute,
    /// Relative to the font size (bare number or `%`).
    FontSize,
    /// Relative to the `'0'` glyph advance width (`"ch"`).
    AdvanceWidth,
}

/// A parsed CSS-like length/percentage. Field-for-field equivalent of WT's
/// `CSSLengthPercentage`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CssLengthPercentage {
    value: f32,
    reference_frame: ReferenceFrame,
}

impl CssLengthPercentage {
    /// Construct from the two private fields — used internally + by tests.
    pub fn new(value: f32, reference_frame: ReferenceFrame) -> Self {
        Self {
            value,
            reference_frame,
        }
    }

    /// Parse a settings string the way WT's `CSSLengthPercentage::FromString`
    /// does. Returns `None` on any parse failure (matches WT returning a
    /// default-constructed empty object the caller treats as unset).
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        // WT uses `std::wcstof`, which parses a leading float and leaves the
        // rest in `end`. Rust's `parse::<f32>()` is stricter (whole input must
        // be a number), so we split the leading numeric prefix manually.
        let (num_part, unit) = split_numeric_prefix(trimmed)?;

        let value: f32 = num_part.parse().ok()?;
        if !value.is_finite() {
            return None;
        }

        let mut reference_frame = ReferenceFrame::FontSize;
        let mut value = value;

        match unit {
            "" => {}
            "%" => value /= 100.0,
            "px" => {
                reference_frame = ReferenceFrame::Absolute;
                value /= 96.0;
            }
            "pt" => {
                reference_frame = ReferenceFrame::Absolute;
                value /= 72.0;
            }
            "ch" => reference_frame = ReferenceFrame::AdvanceWidth,
            // Unknown unit → WT returns an empty object.
            _ => return None,
        }

        Some(Self {
            value,
            reference_frame,
        })
    }

    /// Resolve to pixels using the reference frame. Mirrors WT's
    /// `CSSLengthPercentage::Resolve(fallback, dpi, fontSize, advanceWidth)`.
    pub fn resolve(&self, fallback: f32, dpi: f32, font_size: f32, advance_width: f32) -> f32 {
        match self.reference_frame {
            ReferenceFrame::Absolute => self.value * dpi,
            ReferenceFrame::FontSize => self.value * font_size,
            ReferenceFrame::AdvanceWidth => self.value * advance_width,
            ReferenceFrame::None => fallback,
        }
    }

    pub fn reference_frame(&self) -> ReferenceFrame {
        self.reference_frame
    }
    pub fn value(&self) -> f32 {
        self.value
    }
    pub fn is_empty(&self) -> bool {
        self.reference_frame == ReferenceFrame::None
    }
}

/// Split `s` into `(leading-numeric, trailing-unit)`, mimicking `wcstof`.
/// Returns `None` if no leading number is present.
fn split_numeric_prefix(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Optional leading sign.
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        i = 1;
    }
    let start = i;

    // Integer digits.
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Fractional part: '.' then digits.
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Exponent: 'e'/'E', optional sign, digits. (wcstof supports this.)
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && matches!(bytes[j], b'+' | b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }

    if i == start {
        // No digits consumed — but allow a lone ".5" form.
        if bytes.first() == Some(&b'.') {
            // fall through handled above; if still nothing, no number.
        }
        return None;
    }
    Some((&s[..i], &s[i..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn empty_string_is_none() {
        assert!(CssLengthPercentage::parse("").is_none());
        assert!(CssLengthPercentage::parse("   ").is_none());
    }

    #[test]
    fn bare_number_is_font_size_reference() {
        let v = CssLengthPercentage::parse("1.2").unwrap();
        assert_eq!(v.reference_frame(), ReferenceFrame::FontSize);
        assert!(approx(v.value(), 1.2));
        // 1.2 * font_size(=16) = 19.2
        assert!(approx(v.resolve(0.0, 96.0, 16.0, 8.0), 19.2));
    }

    #[test]
    fn percent_is_font_size_reference_divided_by_100() {
        let v = CssLengthPercentage::parse("100%").unwrap();
        assert_eq!(v.reference_frame(), ReferenceFrame::FontSize);
        assert!(approx(v.value(), 1.0));
        assert!(approx(v.resolve(0.0, 96.0, 16.0, 8.0), 16.0));
    }

    #[test]
    fn px_is_absolute_divided_by_96() {
        // "12px" at 96 dpi → 12 (1:1). At 144 dpi → 18.
        let v = CssLengthPercentage::parse("12px").unwrap();
        assert_eq!(v.reference_frame(), ReferenceFrame::Absolute);
        assert!(approx(v.resolve(0.0, 96.0, 16.0, 8.0), 12.0));
        assert!(approx(v.resolve(0.0, 144.0, 16.0, 8.0), 18.0));
    }

    #[test]
    fn pt_is_absolute_divided_by_72() {
        // "9pt" at 96 dpi → 12. At 144 dpi → 18.
        let v = CssLengthPercentage::parse("9pt").unwrap();
        assert_eq!(v.reference_frame(), ReferenceFrame::Absolute);
        assert!(approx(v.resolve(0.0, 96.0, 16.0, 8.0), 12.0));
        assert!(approx(v.resolve(0.0, 144.0, 16.0, 8.0), 18.0));
    }

    #[test]
    fn ch_is_advance_width_reference() {
        // "1.5ch" with advanceWidth=8 → 12.
        let v = CssLengthPercentage::parse("1.5ch").unwrap();
        assert_eq!(v.reference_frame(), ReferenceFrame::AdvanceWidth);
        assert!(approx(v.resolve(0.0, 96.0, 16.0, 8.0), 12.0));
    }

    #[test]
    fn unknown_unit_is_none() {
        assert!(CssLengthPercentage::parse("12em").is_none());
        assert!(CssLengthPercentage::parse("12vw").is_none());
    }

    #[test]
    fn missing_fallback_is_returned_when_unset() {
        let v = CssLengthPercentage::default();
        assert!(v.is_empty());
        assert!(approx(v.resolve(99.0, 96.0, 16.0, 8.0), 99.0));
    }

    #[test]
    fn leading_dot_number_parses() {
        let v = CssLengthPercentage::parse(".5").unwrap();
        assert_eq!(v.reference_frame(), ReferenceFrame::FontSize);
        assert!(approx(v.value(), 0.5));
    }
}
