//! Font configuration — port of WT's per-profile font settings.
//!
//! Mirrors `MTSM_FONT_SETTINGS` (`MTSMSettings.h:135-144`) on the wire:
//! `face, size, weight, axes, features, builtinGlyphs, colorGlyphs,
//! cellWidth, cellHeight`. No `lineHeight` (the verbatim port drops it — the
//! C++ table has no such key).

use crate::weight::FontWeight;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-profile font configuration. Fields mirror WT's `Profile::FontFace` and
/// friends (see `windows/src/cascadia/TerminalSettingsModel/Profile.idl` and
/// `MTSM_FONT_SETTINGS`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontConfig {
    /// `FontFace` (JSON `"face"`, default `Cascadia Mono`).
    #[serde(rename = "face")]
    pub face: String,
    /// Fallback faces tried in order when `face` is missing a glyph. NOT in
    /// C++ `FontConfig` — this is a `renderer` extension for Glyph fallback;
    /// not serialized (resolved at the renderer from the same default list).
    #[serde(skip, default = "default_fallbacks")]
    pub fallbacks: Vec<String>,
    /// `FontSize` (JSON `"size"`, default 12 pt).
    #[serde(rename = "size")]
    pub size: f32,
    /// `FontWeight` (JSON `"weight"`, default Normal 400).
    #[serde(rename = "weight")]
    pub weight: FontWeight,
    /// `FontAxes` (JSON `"axes"`) — OpenType axis tag → value map
    /// (`IFontAxesMap = IMap<hstring, float>`). Default `None`.
    #[serde(default, rename = "axes")]
    pub axes: Option<HashMap<String, f32>>,
    /// `FontFeatures` (JSON `"features"`) — OpenType feature tag → value map
    /// (`IFontFeatureMap = IMap<hstring, float>`). Default `None`.
    #[serde(default, rename = "features")]
    pub features: Option<HashMap<String, f32>>,
    /// `EnableBuiltinGlyphs` (JSON `"builtinGlyphs"`, default `true`).
    #[serde(default = "default_true", rename = "builtinGlyphs")]
    pub enable_builtin_glyphs: bool,
    /// `EnableColorGlyphs` (JSON `"colorGlyphs"`, default `true`).
    #[serde(default = "default_true", rename = "colorGlyphs")]
    pub enable_color_glyphs: bool,
    /// `CellWidth` (JSON `"cellWidth"`) — CSS length string. Default `None`.
    #[serde(default, rename = "cellWidth")]
    pub cell_width: Option<String>,
    /// `CellHeight` (JSON `"cellHeight"`) — CSS length string. Default `None`.
    #[serde(default, rename = "cellHeight")]
    pub cell_height: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Default fallback list for serde-decode: the WT `DEFAULT_FONT_FALLBACKS`
/// chain. Matches `FontConfig::default` so a skipped `fallbacks` field keeps
/// the runtime-resolved list instead of emptying it.
fn default_fallbacks() -> Vec<String> {
    crate::DEFAULT_FONT_FALLBACKS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for FontConfig {
    fn default() -> Self {
        // Matches WT `MTSM_FONT_SETTINGS` defaults: Cascadia Mono at 12pt,
        // weight Normal, no axes/features, glyphs on, no cell-size overrides.
        Self {
            face: crate::DEFAULT_FONT_FACE.to_string(),
            fallbacks: crate::DEFAULT_FONT_FALLBACKS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            size: crate::DEFAULT_FONT_SIZE,
            weight: FontWeight::Normal,
            axes: None,
            features: None,
            enable_builtin_glyphs: true,
            enable_color_glyphs: true,
            cell_width: None,
            cell_height: None,
        }
    }
}

/// Convert this `FontConfig` into a GPUI `Font` description with the right
/// family + fallbacks + weight. The returned `Font` sizes are applied at
/// render time from the resolved pixel size, not stored on the `Font`.
pub fn to_gpui_font(config: &FontConfig) -> gpui::Font {
    let fallbacks = config.fallbacks.clone();
    gpui::Font {
        family: config.face.clone().into(),
        features: gpui::FontFeatures::default(),
        fallbacks: Some(gpui::FontFallbacks::from_fonts(fallbacks)),
        weight: gpui::FontWeight(config.weight.to_u16() as f32),
        style: gpui::FontStyle::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity: round-trip preserves the WT JSON keys + defaults.
    #[test]
    fn round_trips_cpp_keys() {
        let f = FontConfig::default();
        let j = serde_json::to_string(&f).unwrap();
        assert!(j.contains("\"face\":\"Cascadia Mono\""), "got: {j}");
        assert!(j.contains("\"size\":12"), "got: {j}");
        assert!(j.contains("\"weight\":\"normal\""), "got: {j}");
        assert!(j.contains("\"builtinGlyphs\":true"), "got: {j}");
        assert!(j.contains("\"colorGlyphs\":true"), "got: {j}");
        assert!(!j.contains("lineHeight"), "must drop line_height: {j}");
        assert!(!j.contains("\"fallbacks\""), "fallbacks should skip: {j}");
        let back: FontConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(back.face, "Cascadia Mono");
        assert_eq!(back.weight, FontWeight::Normal);
        assert!(back.enable_builtin_glyphs);
        assert!(back.enable_color_glyphs);
        assert_eq!(back.axes, None);
        assert_eq!(back.features, None);
        assert_eq!(back.cell_width, None);
        assert_eq!(back.cell_height, None);
        // `fallbacks` runtime list survives (skip deserialization → default).
        assert!(!back.fallbacks.is_empty());
    }

    /// Parity: custom axes/features deserialize to maps (C++ `IMap<hstring,float>`).
    #[test]
    fn axes_features_deserialize_as_maps() {
        let j = r#"{
            "face": "Cascadia Code",
            "size": 14.0,
            "weight": "bold",
            "axes": { "wght": 700.0 },
            "features": { "ss01": 1.0 },
            "builtinGlyphs": false,
            "colorGlyphs": false,
            "cellWidth": "1.2ch",
            "cellHeight": "1.0em"
        }"#;
        let f: FontConfig = serde_json::from_str(j).unwrap();
        assert_eq!(f.face, "Cascadia Code");
        assert_eq!(f.weight, FontWeight::Bold);
        assert!((f.size - 14.0).abs() < 1e-6);
        assert_eq!(f.axes.as_ref().unwrap().get("wght"), Some(&700.0));
        assert_eq!(f.features.as_ref().unwrap().get("ss01"), Some(&1.0));
        assert!(!f.enable_builtin_glyphs);
        assert!(!f.enable_color_glyphs);
        assert_eq!(f.cell_width.as_deref(), Some("1.2ch"));
        assert_eq!(f.cell_height.as_deref(), Some("1.0em"));
    }
}
