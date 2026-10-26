//! Port of WT's `FontInfoBase` / `FontInfoDesired` / `FontInfo`
//! (`windows/src/renderer/inc/FontInfo*.hpp`).
//!
//! WT carries three closely related structs:
//! - `FontInfoBase` — face name + family + weight + code page (no size).
//! - `FontInfoDesired` — the user's *requested* font (size + cell-size
//!   overrides as `CSSLengthPercentage`, plus builtin/color glyph flags).
//! - `FontInfo` — the engine's *resolved* font (measured cell size + the
//!   unscaled size reported back to legacy APIs).
//!
//! We model all three with pure-data Rust structs so the metrics algorithm
//! can run against any source (a settings profile, a serialized test
//! fixture, etc.). The GDI-specific fields (`family`, `codepage`,
//! `WasDefaultRasterSetFromEngine`) are kept for parity but unused by the
//! GPUI renderer path.

use crate::css_length::CssLengthPercentage;

/// Sentinel matching WT's `DEFAULT_TT_FONT_FACENAME` (`__DefaultTTFont__`).
pub const DEFAULT_TT_FONT_FACENAME: &str = "__DefaultTTFont__";
/// Sentinel matching WT's `DEFAULT_RASTER_FONT_FACENAME` (`"Terminal"`).
pub const DEFAULT_RASTER_FONT_FACENAME: &str = "Terminal";

/// Base font identity: face name + GDI family + weight + code page.
/// Mirrors `FontInfoBase`.
#[derive(Clone, Debug)]
pub struct FontInfoBase {
    pub face_name: String,
    pub weight: u32,
    /// GDI `lfPitchAndFamily` byte. We carry it for parity; the renderer
    /// doesn't use it.
    pub family: u8,
    pub code_page: u32,
    pub default_raster_set_from_engine: bool,
}

impl FontInfoBase {
    pub fn new(face_name: impl Into<String>, family: u8, weight: u32, code_page: u32) -> Self {
        Self {
            face_name: face_name.into(),
            weight,
            family,
            code_page,
            default_raster_set_from_engine: false,
        }
    }

    pub fn weight(&self) -> u32 {
        self.weight
    }
    pub fn face_name(&self) -> &str {
        &self.face_name
    }
    pub fn family(&self) -> u8 {
        self.family
    }
    pub fn is_true_type_font(&self) -> bool {
        // WT treats the raster sentinel ("Terminal") as the non-TT case.
        self.face_name != DEFAULT_RASTER_FONT_FACENAME && self.face_name != DEFAULT_TT_FONT_FACENAME
    }
}

/// Desired font request: identity + `fontSize` (points) + the `CSSLengthPercentage`
/// cell-size overrides and glyph flags. Mirrors `FontInfoDesired`.
#[derive(Clone, Debug)]
pub struct FontInfoDesired {
    pub base: FontInfoBase,
    /// Requested size in **points**. WT clamps this to `[1.0, 100.0]`.
    pub font_size: f32,
    /// GDI engine size in px (height, width). When height is 0 WT forces 12pt.
    pub engine_size: (i32, i32),
    pub cell_width: CssLengthPercentage,
    pub cell_height: CssLengthPercentage,
    pub builtin_glyphs: bool,
    pub color_glyphs: bool,
}

impl FontInfoDesired {
    pub fn new(
        face_name: impl Into<String>,
        family: u8,
        weight: u32,
        font_size: f32,
        code_page: u32,
    ) -> Self {
        Self {
            base: FontInfoBase::new(face_name, family, weight, code_page),
            font_size,
            engine_size: (0, font_size as i32),
            cell_width: CssLengthPercentage::default(),
            cell_height: CssLengthPercentage::default(),
            builtin_glyphs: true,
            color_glyphs: true,
        }
    }

    pub fn set_cell_size(&mut self, w: CssLengthPercentage, h: CssLengthPercentage) {
        self.cell_width = w;
        self.cell_height = h;
    }

    pub fn face_name(&self) -> &str {
        self.base.face_name()
    }
    pub fn weight(&self) -> u32 {
        self.base.weight()
    }
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    pub fn engine_size(&self) -> (i32, i32) {
        self.engine_size
    }
    pub fn get_cell_width(&self) -> &CssLengthPercentage {
        &self.cell_width
    }
    pub fn get_cell_height(&self) -> &CssLengthPercentage {
        &self.cell_height
    }
    pub fn get_enable_builtin_glyphs(&self) -> bool {
        self.builtin_glyphs
    }
    pub fn get_enable_color_glyphs(&self) -> bool {
        self.color_glyphs
    }
}

/// Resolved font: identity + the measured cell size and the unscaled size
/// legacy callers expect. Mirrors `FontInfo`.
#[derive(Clone, Debug)]
pub struct FontInfo {
    pub base: FontInfoBase,
    /// Measured cell size `(width, height)` in px.
    pub size: (i32, i32),
    /// Unscaled size reported back to legacy `GetConsoleFontSize` callers.
    pub unscaled_size: (i32, i32),
    pub did_fallback: bool,
}

impl FontInfo {
    pub fn new(
        face_name: impl Into<String>,
        family: u8,
        weight: u32,
        size: (i32, i32),
        code_page: u32,
    ) -> Self {
        Self {
            base: FontInfoBase::new(face_name, family, weight, code_page),
            size,
            unscaled_size: size,
            did_fallback: false,
        }
    }

    /// Mirrors WT's `FontInfo::SetFromEngine`. Records the engine's chosen
    /// face/weight, the measured `coord_size`, and the unscaled size used by
    /// legacy APIs.
    pub fn set_from_engine(
        &mut self,
        face_name: impl Into<String>,
        family: u8,
        weight: u32,
        default_raster: bool,
        coord_size: (i32, i32),
        unscaled_size: (i32, i32),
    ) {
        self.base.face_name = face_name.into();
        self.base.family = family;
        self.base.weight = weight;
        self.base.default_raster_set_from_engine = default_raster;
        self.size = coord_size;
        self.unscaled_size = unscaled_size;
    }

    pub fn size(&self) -> (i32, i32) {
        self.size
    }
    pub fn unscaled_size(&self) -> (i32, i32) {
        self.unscaled_size
    }
    pub fn get_fallback(&self) -> bool {
        self.did_fallback
    }
    pub fn set_fallback(&mut self, did_fallback: bool) {
        self.did_fallback = did_fallback;
    }
}
