//! Windows-only DirectWrite binding for `font`.
//!
//! Implements [`crate::metrics::FontSource`] on top of the real
//! DirectWrite font collection, mirroring the family-resolution and
//! `IDWriteFontFace` queries WT performs in
//! `AtlasEngine::_resolveFontMetrics`:
//! - `IDWriteFactory::GetSystemFontCollection` (or a caller-supplied custom
//!   collection) for the family lookup.
//! - `IDWriteFontFamily::GetFirstMatchingFont(weight, NORMAL stretch,
//!   NORMAL style)` to pick the actual `IDWriteFont`.
//! - `IDWriteFont::CreateFontFace` for the `IDWriteFontFace`.
//! - `IDWriteFontFace::GetMetrics` for `DWRITE_FONT_METRICS`.
//! - `GetGlyphIndices('0')` + `GetDesignGlyphMetrics` for the `'0'` advance.
//!
//! The arithmetic that turns these numbers into `cellSize` / `baseline` /
//! decorations lives in [`crate::metrics`]; this file only fetches the
//! DirectWrite-supplied input.

use crate::metrics::{DWriteFontMetrics, FontSource, PrimaryFont};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteFontFace,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_GLYPH_METRICS,
};

/// A DirectWrite-backed font collection. By default we use the system font
/// collection; callers can plug in a custom one (e.g. WT's nearby-font
/// package collection) by constructing with [`Self::with_collection`].
pub struct DWriteFontCollection {
    factory: IDWriteFactory,
    collection: IDWriteFontCollection,
}

impl DWriteFontCollection {
    /// Create using the system font collection (the WT default path).
    pub fn system() -> anyhow::Result<Self> {
        unsafe {
            let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let mut collection: Option<IDWriteFontCollection> = None;
            factory.GetSystemFontCollection(&mut collection, false)?;
            let collection = collection.ok_or_else(|| {
                anyhow::anyhow!("IDWriteFactory::GetSystemFontCollection returned None")
            })?;
            Ok(Self {
                factory,
                collection,
            })
        }
    }

    /// Create against a pre-resolved font collection (e.g. a custom
    /// collection that includes app-bundled `.ttf` files — WT's
    /// "NearbyFontLoading" path).
    pub fn with_collection(factory: IDWriteFactory, collection: IDWriteFontCollection) -> Self {
        Self {
            factory,
            collection,
        }
    }

    pub fn factory(&self) -> &IDWriteFactory {
        &self.factory
    }
    pub fn collection(&self) -> &IDWriteFontCollection {
        &self.collection
    }

    /// Look up a family name. Returns the family index and `exists` flag,
    /// matching `IDWriteFontCollection::FindFamilyName`.
    fn find_family(&self, name: &str) -> Option<u32> {
        unsafe {
            let mut index = 0u32;
            let mut exists = windows::Win32::Foundation::FALSE;
            let familyname = windows::core::HSTRING::from(name);
            self.collection
                .FindFamilyName(&familyname, &mut index, &mut exists)
                .ok()?;
            if exists.as_bool() {
                Some(index)
            } else {
                None
            }
        }
    }
}

impl FontSource for DWriteFontCollection {
    fn find(&self, family_name: &str, weight: u32) -> Option<DWriteFontMetrics> {
        let index = self.find_family(family_name)?;
        unsafe {
            let family = self.collection.GetFontFamily(index).ok()?;
            let font = family
                .GetFirstMatchingFont(
                    DWRITE_FONT_WEIGHT(weight as i32),
                    DWRITE_FONT_STRETCH_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                )
                .ok()?;
            let face: IDWriteFontFace = font.CreateFontFace().ok()?;
            Some(metrics_from_face(&face))
        }
    }

    fn resolve_primary(&self, primary_name: &str, weight: u32) -> Option<PrimaryFont> {
        let metrics = self.find(primary_name, weight)?;
        Some(PrimaryFont {
            name: primary_name.to_string(),
            metrics,
        })
    }
}

/// Build `DWriteFontMetrics` from an `IDWriteFontFace`, querying the `'0'`
/// glyph advance. Negates `underlinePosition` / `strikethroughPosition` so
/// the math in `metrics.rs` can add them directly to the baseline.
pub(crate) fn metrics_from_face(face: &IDWriteFontFace) -> DWriteFontMetrics {
    unsafe {
        let mut m = DWRITE_FONT_METRICS::default();
        face.GetMetrics(&mut m);

        // '0' glyph index + design-units advance.
        let codepoints = [0x0030u32]; // U+0030 '0'
        let mut glyph_indices = [0u16];
        let _ = face.GetGlyphIndices(
            codepoints.as_ptr(),
            codepoints.len() as u32,
            glyph_indices.as_mut_ptr(),
        );

        let zero_advance = if glyph_indices[0] != 0 {
            let mut gm = [DWRITE_GLYPH_METRICS::default()];
            let _ = face.GetDesignGlyphMetrics(
                glyph_indices.as_ptr(),
                glyph_indices.len() as u32,
                gm.as_mut_ptr(),
                false,
            );
            if gm[0].advanceWidth > 0 {
                Some(gm[0].advanceWidth)
            } else {
                None
            }
        } else {
            None
        };

        DWriteFontMetrics {
            design_units_per_em: m.designUnitsPerEm as u32,
            ascent: m.ascent as i32,
            descent: m.descent as i32,
            line_gap: m.lineGap as i32,
            // WT: `static_cast<f32>(-metrics.underlinePosition)`.
            underline_position: -(m.underlinePosition as f32),
            underline_thickness: m.underlineThickness as f32,
            strikethrough_position: -(m.strikethroughPosition as f32),
            strikethrough_thickness: m.strikethroughThickness as f32,
            zero_advance_design_units: zero_advance,
        }
    }
}
