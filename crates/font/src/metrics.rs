//! Verbatim Rust port of WT's `AtlasEngine::_resolveFontMetrics`
//! (`windows/src/renderer/atlas/AtlasEngine.api.cpp`, lines ~616–876).
//!
//! This module reproduces the *exact* math WT uses to turn a
//! `FontInfoDesired` (face name, point size, weight, optional `CellWidth`/
//! `CellHeight` overrides) into the cell-grid metrics that drive the
//! AtlasEngine: `cellSize`, `baseline`, `descender`, `advanceWidth`,
//! `thinLineWidth`, and every `FontDecorationPosition` (underline,
//! strikethrough, double-underline, overline, gridlines).
//!
//! ## Shape
//! The algorithm is split in two:
//! 1. [`FontSource`] — a trait that supplies the DirectWrite-derived numbers
//!    (font metrics, `'0'` advance). The Windows implementation lives in
//!    [`crate::dwrite`]; tests supply a fake source so the *math* can be
//!    verified without Windows.
//! 2. [`resolve_font_metrics`] — pure arithmetic on those numbers, copied
//!    line-for-line from the C++.
//!
//! This matches the structure of the C++ source, where `_resolveFontMetrics`
//! first asks DirectWrite for `DWRITE_FONT_METRICS` + the `'0'` glyph
//! advance and then does the rounding/clamping math.

use crate::font_info::{FontInfo, FontInfoDesired};

/// A pair `(position, height)` for a font decoration, identical to WT's
/// `FontDecorationPosition`. Both fields are in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontDecorationPosition {
    pub position: u16,
    pub height: u16,
}

impl FontDecorationPosition {
    pub const fn new(position: u16, height: u16) -> Self {
        Self { position, height }
    }
}

/// The resolved, integer cell-grid metrics that the renderer paints against.
/// Field-for-field equivalent of WT's `atlas::FontSettings` (the subset we
/// need for layout + decoration; we omit the COM handles, which live in the
/// DirectWrite layer).
#[derive(Clone, Debug)]
pub struct FontRuntime {
    /// Resolved font face name (the primary family actually used).
    pub font_name: String,
    /// Font size in **pixels** (the input was in points).
    pub font_size: f32,
    /// Cell grid `(width, height)` in px.
    pub cell_size: (u16, u16),
    /// Weight actually applied (matches WT's `fontWeight` u16).
    pub font_weight: u16,
    /// Advance width of `'0'` in px (the "ch" unit).
    pub advance_width: u16,
    /// Baseline offset from the cell top in px.
    pub baseline: u16,
    /// `cell_height - baseline` (in px).
    pub descender: u16,
    pub thin_line_width: u16,

    pub grid_top: FontDecorationPosition,
    pub grid_bottom: FontDecorationPosition,
    pub grid_left: FontDecorationPosition,
    pub grid_right: FontDecorationPosition,

    pub underline: FontDecorationPosition,
    pub strikethrough: FontDecorationPosition,
    pub double_underline: [FontDecorationPosition; 2],
    pub overline: FontDecorationPosition,

    pub builtin_glyphs: bool,
    pub color_glyphs: bool,
}

/// A summary a caller can log/serialize without holding DirectWrite handles.
#[derive(Clone, Debug)]
pub struct FontSettingsSummary {
    pub font_name: String,
    pub font_size_px: f32,
    pub cell_size: (u16, u16),
    pub font_weight: u16,
    pub advance_width: u16,
    pub baseline: u16,
    pub descender: u16,
    pub thin_line_width: u16,
}

impl From<&FontRuntime> for FontSettingsSummary {
    fn from(f: &FontRuntime) -> Self {
        Self {
            font_name: f.font_name.clone(),
            font_size_px: f.font_size,
            cell_size: f.cell_size,
            font_weight: f.font_weight,
            advance_width: f.advance_width,
            baseline: f.baseline,
            descender: f.descender,
            thin_line_width: f.thin_line_width,
        }
    }
}

/// DirectWrite numbers we need from a primary font face. The Windows
/// implementation (`crate::dwrite`) calls `IDWriteFontFace::GetMetrics` and
/// `GetDesignGlyphMetrics` for the `'0'` glyph.
#[derive(Clone, Copy, Debug)]
pub struct DWriteFontMetrics {
    pub design_units_per_em: u32,
    pub ascent: i32,
    pub descent: i32,
    pub line_gap: i32,
    /// Negative of DWrite's `underlinePosition` (so we add it to baseline).
    pub underline_position: f32,
    pub underline_thickness: f32,
    /// Negative of DWrite's `strikethroughPosition`.
    pub strikethrough_position: f32,
    pub strikethrough_thickness: f32,
    /// Advance width of the `'0'` glyph in design units, *if* the glyph
    /// exists in the font. `None` → WT falls back to `0.5 * fontSizeInPx`.
    pub zero_advance_design_units: Option<u32>,
}

/// Outcome of primary-font resolution. Mirrors the two branches of WT's
/// `til::iterate_font_families` loop: the first family found becomes the
/// primary; the rest are appended as fallback mappings.
#[derive(Clone, Debug)]
pub struct PrimaryFont {
    /// The family name WT actually used (possibly `"Consolas"` if none matched).
    pub name: String,
    pub metrics: DWriteFontMetrics,
}

/// Abstraction over the DirectWrite font collection. The Windows
/// implementation calls into `IDWriteFactory` / `IDWriteFontCollection`; a
/// test implementation returns canned `DWriteFontMetrics` so the arithmetic
/// below can be verified without Windows.
pub trait FontSource {
    /// Try to find a family by name and, if present, return its
    /// `DWriteFontMetrics` (by querying the first matching font at the
    /// requested weight). Returns `None` if the family isn't installed —
    /// matching WT's `FindFamilyName(..., &exists)` with `exists == FALSE`.
    fn find(&self, family_name: &str, weight: u32) -> Option<DWriteFontMetrics>;

    /// Resolve the final primary font after the comma list is processed.
    /// `primary_name` is the first name that matched (or `"Consolas"` if
    /// nothing matched). Default implementation just calls `find`.
    fn resolve_primary(&self, primary_name: &str, weight: u32) -> Option<PrimaryFont> {
        self.find(primary_name, weight).map(|metrics| PrimaryFont {
            name: primary_name.to_string(),
            metrics,
        })
    }
}

/// Result of a full metric resolution. Carries the updated `FontInfo`
/// (with `set_from_engine` applied) and the `FontRuntime` for the renderer.
#[derive(Clone, Debug)]
pub struct ResolveFontResult {
    pub font_info: FontInfo,
    pub runtime: FontRuntime,
    /// Names from the comma list that were not installed — surfaced to the
    /// UI as a "missing font" warning, exactly like WT's `warningCallback`.
    pub missing_font_names: Vec<String>,
}

/// Resolve font metrics given a `FontSource`. Pure arithmetic on top of the
/// DirectWrite-supplied [`DWriteFontMetrics`] — copied line-for-line from
/// `AtlasEngine::_resolveFontMetrics`.
///
/// `dpi` is the effective display DPI (96 at 100% scale, 144 at 150%, ...).
pub fn resolve_font_metrics(
    source: &dyn FontSource,
    desired: &FontInfoDesired,
    family: u8,
    code_page: u32,
    dpi: f32,
) -> Option<ResolveFontResult> {
    let face_name = desired.face_name();
    let requested_weight = desired.weight();
    let mut font_size = desired.font_size;
    let mut requested_size = desired.engine_size();

    let mut missing_font_names = Vec::new();

    // -- Mirror WT clamp + zero-height fallback -----------------------------------
    font_size = font_size.clamp(1.0, 100.0);
    if requested_size.1 == 0 {
        font_size = 12.0;
        requested_size = (0, 12);
    }
    let weight = if requested_weight == 0 {
        400
    } else {
        requested_weight
    };

    // -- Resolve primary font via the comma-separated list ------------------------
    // WT walks the *entire* list: every missing family is recorded (for the
    // warning callback), the first present family becomes the primary, and
    // every subsequent present family is added to a DirectWrite fallback
    // builder. The fallback chain is consumed at paint time, not by the
    // metric math, so here we only need to pick the primary + record missing
    // names.
    let mut primary_name: Option<String> = None;
    crate::iterate_font_families(face_name, |name| {
        match source.find(&name, weight) {
            Some(_) if primary_name.is_none() => primary_name = Some(name),
            None => missing_font_names.push(name),
            _ => {} // present but a primary was already chosen → fallback slot
        }
    });

    // -- Fall back to Consolas if nothing matched ---------------------------------
    let primary_name = primary_name.unwrap_or_else(|| "Consolas".to_string());
    let primary = source.resolve_primary(&primary_name, weight)?;
    let m = primary.metrics;

    // -- Reproduce the design-units → pixels math verbatim ------------------------
    let font_size_in_px = font_size / 72.0 * dpi;
    let design_units_per_px = font_size_in_px / m.design_units_per_em as f32;
    let ascent = m.ascent as f32 * design_units_per_px;
    let descent = m.descent as f32 * design_units_per_px;
    let line_gap = m.line_gap as f32 * design_units_per_px;
    let underline_position = m.underline_position * design_units_per_px;
    let underline_thickness = m.underline_thickness * design_units_per_px;
    let strikethrough_position = m.strikethrough_position * design_units_per_px;
    let strikethrough_thickness = m.strikethrough_thickness * design_units_per_px;
    let advance_height = ascent + descent + line_gap;

    // CSS "ch" unit: '0' advance, or 0.5em if '0' is missing.
    let mut advance_width = 0.5 * font_size_in_px;
    if let Some(z) = m.zero_advance_design_units {
        advance_width = z as f32 * design_units_per_px;
    }

    let adjusted_width = desired
        .cell_width
        .resolve(advance_width, dpi, font_size_in_px, advance_width)
        .round();
    let adjusted_height = desired
        .cell_height
        .resolve(advance_height, dpi, font_size_in_px, advance_width)
        .round();

    // Protection against bad user values: WT clamps both to ≥1.
    let adjusted_width = adjusted_width.max(1.0);
    let adjusted_height = adjusted_height.max(1.0);

    let baseline = (advance_height + (line_gap + adjusted_height - advance_height) / 2.0).round();
    let underline_pos = (baseline + underline_position).round();
    let underline_width = underline_thickness.round().max(1.0);
    let strikethrough_pos = (baseline + strikethrough_position).round();
    let strikethrough_width = strikethrough_thickness.round().max(1.0);
    let double_underline_width = (underline_thickness / 2.0).round().max(1.0);
    let thin_line_width = (adjusted_width / 16.0)
        .max(adjusted_height / 32.0)
        .round()
        .max(1.0);

    // -- Double-underline geometry (WT's 5-step Word-style placement) -------------
    let double_underline_pos_bottom = underline_pos + underline_width - double_underline_width;
    let mut double_underline_pos_top =
        ((baseline + double_underline_pos_bottom - double_underline_width) / 2.0).round();
    double_underline_pos_top = double_underline_pos_top.max(baseline + double_underline_width);
    let double_underline_gap = (1.2 / 72.0 * dpi).round().max(1.0);
    let double_underline_pos_bottom_raw =
        double_underline_pos_top + double_underline_gap + double_underline_width;
    let double_underline_pos_bottom =
        double_underline_pos_bottom.max(double_underline_pos_bottom_raw);
    // Clamp the bottom line inside the cell.
    let double_underline_pos_bottom =
        double_underline_pos_bottom.min(adjusted_height - double_underline_width);

    let cell_width = lrintf(adjusted_width) as u16;
    let cell_height = lrintf(adjusted_height) as u16;

    // -- Fill FontInfo (WT's `fontInfo.SetFromEngine`) ----------------------------
    let mut font_info = FontInfo::new(primary_name.clone(), family, weight, (0, 0), code_page);
    let coord_size = (cell_width as i32, cell_height as i32);
    let mut unscaled = requested_size;
    if unscaled.0 == 0 {
        unscaled.0 = lrintf(font_size / cell_height as f32 * cell_width as f32);
    }
    font_info.set_from_engine(&primary_name, family, weight, false, coord_size, unscaled);

    // -- Fill FontRuntime (WT's `fontMetrics` writes) ----------------------------
    let baseline_u16 = lrintf(baseline) as u16;
    let descender_u16 = cell_height - baseline_u16;
    let advance_width_u16 = lrintf(advance_width) as u16;
    let thin_line_width_u16 = lrintf(thin_line_width) as u16;

    let grid_bottom = cell_height.saturating_sub(thin_line_width_u16);
    let grid_right = cell_width.saturating_sub(thin_line_width_u16);

    let runtime = FontRuntime {
        font_name: primary_name.clone(),
        font_size: font_size_in_px,
        cell_size: (cell_width, cell_height),
        font_weight: weight as u16,
        advance_width: advance_width_u16,
        baseline: baseline_u16,
        descender: descender_u16,
        thin_line_width: thin_line_width_u16,
        grid_top: FontDecorationPosition::new(0, thin_line_width_u16),
        grid_bottom: FontDecorationPosition::new(grid_bottom, thin_line_width_u16),
        grid_left: FontDecorationPosition::new(0, thin_line_width_u16),
        grid_right: FontDecorationPosition::new(grid_right, thin_line_width_u16),
        underline: FontDecorationPosition::new(
            lrintf(underline_pos) as u16,
            lrintf(underline_width) as u16,
        ),
        strikethrough: FontDecorationPosition::new(
            lrintf(strikethrough_pos) as u16,
            lrintf(strikethrough_width) as u16,
        ),
        double_underline: [
            FontDecorationPosition::new(
                lrintf(double_underline_pos_top) as u16,
                lrintf(double_underline_width) as u16,
            ),
            FontDecorationPosition::new(
                lrintf(double_underline_pos_bottom) as u16,
                lrintf(double_underline_width) as u16,
            ),
        ],
        overline: FontDecorationPosition::new(0, lrintf(underline_width) as u16),
        builtin_glyphs: desired.builtin_glyphs,
        color_glyphs: desired.color_glyphs,
    };

    Some(ResolveFontResult {
        font_info,
        runtime,
        missing_font_names,
    })
}

/// Variant of [`resolve_font_metrics`] that takes an already-resolved
/// [`PrimaryFont`] (skipping the comma-list lookup). Used by the Windows
/// DirectWrite path, which resolves the primary family itself before doing
/// the math.
pub fn resolve_font_metrics_with_collection(
    desired: &FontInfoDesired,
    family: u8,
    code_page: u32,
    dpi: f32,
    primary: PrimaryFont,
    missing_font_names: Vec<String>,
) -> Option<ResolveFontResult> {
    struct StaticSource(Option<(String, DWriteFontMetrics)>);
    impl FontSource for StaticSource {
        fn find(&self, _name: &str, _weight: u32) -> Option<DWriteFontMetrics> {
            None
        }
        fn resolve_primary(&self, name: &str, _weight: u32) -> Option<PrimaryFont> {
            self.0
                .as_ref()
                .filter(|(n, _)| n == name)
                .map(|(_, m)| PrimaryFont {
                    name: name.to_string(),
                    metrics: *m,
                })
        }
    }
    let src = StaticSource(Some((primary.name.clone(), primary.metrics)));
    let mut result = resolve_font_metrics(&src, desired, family, code_page, dpi)?;
    // The static source's `find` always returns None, so `resolve_font_metrics`
    // would push *every* family into `missing_font_names`. Override with the
    // caller-supplied list (which already reflects the truth).
    result.missing_font_names = missing_font_names;
    Some(result)
}

/// Round-to-nearest-even (`lrintf` equivalent). f32::round() rounds half
/// away from zero; the C `lrintf` rounds half to *even* under the default
/// `FE_TONEAREST` mode. WT uses `lrintf` for the `cellWidth`/`cellHeight`
/// cast, so we reproduce half-to-even. The difference is at most 1px on
/// exact `.5` values.
fn lrintf(x: f32) -> i32 {
    // round() gives half-away-from-zero; the floor+0.5 trick gives half-to-even
    // when the integer part is even. We replicate that explicitly.
    let floor = x.floor() as i32;
    let frac = x - floor as f32;
    // |frac| ≤ 0.5 rounds to nearest; ties go to even.
    let abs_frac = frac.abs();
    if (abs_frac - 0.5).abs() < 1e-4 {
        // tie: pick the even neighbor.
        let lo = floor;
        let hi = floor + 1;
        if lo % 2 == 0 {
            lo
        } else {
            hi
        }
    } else {
        x.round() as i32
    }
    .max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css_length::CssLengthPercentage;

    /// A canned DirectWrite metrics set roughly matching Cascadia Mono at
    /// 12pt / 96dpi. The exact numbers aren't important for unit tests; what
    /// matters is that the math is internally consistent with WT's algorithm.
    fn cascadia_like() -> DWriteFontMetrics {
        DWriteFontMetrics {
            design_units_per_em: 2048,
            ascent: 1900,
            descent: 500,
            line_gap: 0,
            // WT negates underlinePosition; we store the already-negated value.
            underline_position: -150.0,
            underline_thickness: 100.0,
            strikethrough_position: -550.0,
            strikethrough_thickness: 50.0,
            zero_advance_design_units: Some(1228),
        }
    }

    struct FakeSource;
    impl FontSource for FakeSource {
        fn find(&self, name: &str, _w: u32) -> Option<DWriteFontMetrics> {
            if name == "Cascadia Mono" || name == "Consolas" {
                Some(cascadia_like())
            } else {
                None
            }
        }
    }

    #[test]
    fn default_resolution_uses_font_advance_for_ch() {
        let desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 12.0, 0);
        let result = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        let r = &result.runtime;

        // fontSizeInPx = 12/72*96 = 16
        assert!(
            (r.font_size - 16.0).abs() < 1e-3,
            "font_size={}px",
            r.font_size
        );
        // advanceWidth = 1228/2048 * 16 = 9.59375 → rounds to 10
        assert_eq!(r.advance_width, 10);
        // cellWidth = round(advanceWidth) since cellWidth is unset → fallback = advanceWidth
        // → 9.59375 rounded = 10 (lrintf: 9.59375 is not half → rounds to 10)
        assert_eq!(r.cell_size.0, 10);
        // advanceHeight = (1900+500+0)/2048 * 16 = 18.75 → 19
        assert_eq!(r.cell_size.1, 19);
        // baseline = round(18.75 + (0 + 19 - 18.75)/2) = round(18.875) = 19
        assert_eq!(r.baseline, 19);
        assert_eq!(r.descender, 0);
    }

    #[test]
    fn cell_height_override_via_font_size_reference() {
        // cellHeight = "1.2" → FontSize reference, value=1.2 → 1.2 * 16 = 19.2 → round = 19
        let mut desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 12.0, 0);
        desired.set_cell_size(
            CssLengthPercentage::default(),
            CssLengthPercentage::parse("1.2").unwrap(),
        );
        let r = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        assert_eq!(r.runtime.cell_size.1, 19);
    }

    #[test]
    fn cell_width_in_ch_units() {
        // cellWidth = "1.5ch" → 1.5 * advanceWidth(=9.59375) = 14.39 → 14
        let mut desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 12.0, 0);
        desired.set_cell_size(
            CssLengthPercentage::parse("1.5ch").unwrap(),
            CssLengthPercentage::default(),
        );
        let r = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        assert_eq!(r.runtime.cell_size.0, 14);
    }

    #[test]
    fn cell_width_in_px_units_scales_with_dpi() {
        // cellWidth = "12px" → 12/96 * dpi. At 144 dpi → 18.
        let mut desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 12.0, 0);
        desired.set_cell_size(
            CssLengthPercentage::parse("12px").unwrap(),
            CssLengthPercentage::default(),
        );
        let r = resolve_font_metrics(&FakeSource, &desired, 0, 0, 144.0).unwrap();
        assert_eq!(r.runtime.cell_size.0, 18);
    }

    #[test]
    fn missing_font_falls_back_to_consolas() {
        // A face name that isn't installed → primary becomes "Consolas".
        let desired = FontInfoDesired::new("NoSuchFont XYZ", 0, 400, 12.0, 0);
        let result = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        assert_eq!(result.runtime.font_name, "Consolas");
        assert_eq!(
            result.missing_font_names,
            vec!["NoSuchFont XYZ".to_string()]
        );
    }

    #[test]
    fn comma_list_first_match_wins() {
        // First of multiple that match becomes primary; the rest are silently
        // accepted (they'd become fallback mappings at paint time). Missing
        // ones are recorded for the warning callback.
        let desired = FontInfoDesired::new("Cascadia Mono, NoSuchFont, Consolas", 0, 400, 12.0, 0);
        let result = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        assert_eq!(result.runtime.font_name, "Cascadia Mono");
        // The first matched name is primary; names that failed *before* the
        // match are recorded as missing. Names after the match are added to
        // the fallback builder (not the missing list), so "Consolas" is not
        // recorded.
        assert_eq!(result.missing_font_names, vec!["NoSuchFont".to_string()]);
    }

    #[test]
    fn font_size_clamped_to_1_100() {
        let desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 500.0, 0);
        let r = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        // Clamped to 100pt → 100/72*96 = 133.33px
        assert!(
            (r.runtime.font_size - 100.0 / 72.0 * 96.0).abs() < 1e-3,
            "got {}",
            r.runtime.font_size
        );
    }

    #[test]
    fn zero_engine_height_forces_12pt() {
        let mut desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 7.0, 0);
        desired.engine_size = (0, 0); // height 0 → WT forces 12pt
        let r = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        // 12pt at 96dpi = 16px
        assert!((r.runtime.font_size - 16.0).abs() < 1e-3);
    }

    #[test]
    fn thin_line_width_floor_is_one() {
        // At a tiny font (1pt clamped), thinLineWidth must still be ≥1.
        let desired = FontInfoDesired::new("Cascadia Mono", 0, 400, 1.0, 0);
        let r = resolve_font_metrics(&FakeSource, &desired, 0, 0, 96.0).unwrap();
        assert!(
            r.runtime.thin_line_width >= 1,
            "thinLineWidth={}",
            r.runtime.thin_line_width
        );
    }
}
