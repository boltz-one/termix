//! `Appearance` — per-pane appearance (color scheme, foreground/background,
//! cursor, font + cell size). Combines what WT splits across
//! `AppearanceConfig`, `FontConfig` and a `ColorScheme` reference.
//!
//! Field set mirrors `AppearanceConfig.h:22-58` + `MTSM_APPEARANCE_SETTINGS`
//! (`MTSMSettings.h:146-158`).

use crate::appearance_enums::{
    AdjustTextMode, BackgroundImageAlignment, CursorShape, IntenseStyle, Stretch,
};
use crate::color::{campbell_palette, Color};
use crate::font::FontConfig;
use gpui::Rgba;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Appearance {
    /// 16-entry palette. The C++ split into scheme-name vs resolved-palette
    /// is a known survey gap (ported in a later phase); kept here.
    #[serde(default = "default_palette")]
    pub palette: [Rgba; 16],
    /// `Foreground` (`AppearanceConfig.h:31`).
    #[serde(default = "default_foreground")]
    pub foreground: Rgba,
    /// `Background` (`AppearanceConfig.h:32`).
    #[serde(default = "default_background")]
    pub background: Rgba,
    /// `SelectionBackground` (Nullable Setting, `AppearanceConfig.h:33`,
    /// JSON `"selectionBackground"`).
    #[serde(default, rename = "selectionBackground")]
    pub selection_background: Option<Color>,
    /// `CursorColor` (`AppearanceConfig.h:34`, JSON `"cursorColor"`).
    #[serde(default = "default_cursor_color", rename = "cursorColor")]
    pub cursor_color: Rgba,
    /// Legacy cursor height as percent of cell
    /// (`MTSM_APPEARANCE_SETTINGS`, JSON `"cursorHeight"`, default 25).
    #[serde(default = "default_cursor_height", rename = "cursorHeight")]
    pub cursor_height: u8,
    /// `CursorShape` (`TerminalSettingsSerializationHelpers.h:18-26`,
    /// JSON `"cursorShape"`).
    #[serde(default, rename = "cursorShape")]
    pub cursor_shape: CursorShape,
    /// Font + cell-size configuration.
    #[serde(default)]
    pub font: FontConfig,
    /// `Opacity` 0..=1.0 — JSON is a uint percent in `[0, 100]`
    /// (`IntAsFloatPercentConversionTrait`, `.h:382-400`).
    #[serde(
        default = "default_opacity_one",
        serialize_with = "ser_opacity",
        deserialize_with = "de_opacity"
    )]
    pub opacity: f32,
    /// `useAcrylic` (substrate blur on Windows).
    #[serde(default, rename = "useAcrylic")]
    pub use_acrylic: bool,
    /// `backgroundImage` (`MTSM_APPEARANCE_SETTINGS`, JSON `"backgroundImage"`).
    #[serde(default, rename = "backgroundImage")]
    pub background_image: Option<String>,
    /// `backgroundImageStretchMode` (`Stretch`, JSON `"backgroundImageStretchMode"`,
    /// default `UniformToFill`).
    #[serde(default, rename = "backgroundImageStretchMode")]
    pub background_image_stretch_mode: Stretch,
    /// `backgroundImageOpacity` 0.0..=1.0 (JSON number, default 1.0).
    #[serde(default = "default_opacity_one", rename = "backgroundImageOpacity")]
    pub background_image_opacity: f32,
    /// `backgroundImageAlignment` (`ConvergedAlignment`, JSON
    /// `"backgroundImageAlignment"`, default `center`).
    #[serde(default, rename = "backgroundImageAlignment")]
    pub background_image_alignment: BackgroundImageAlignment,
    /// `intenseTextStyle` (`IntenseStyle`, JSON `"intenseTextStyle"`,
    /// default `Bright`).
    #[serde(default, rename = "intenseTextStyle")]
    pub intense_text_style: IntenseStyle,
    /// `adjustIndistinguishableColors` (`AdjustTextMode`, JSON
    /// `"adjustIndistinguishableColors"`, default `Automatic`).
    #[serde(default, rename = "adjustIndistinguishableColors")]
    pub adjust_indistinguishable_colors: AdjustTextMode,
    /// `experimental.retroTerminalEffect` (default false).
    #[serde(default, rename = "experimental.retroTerminalEffect")]
    pub retro_terminal_effect: bool,
    /// `experimental.pixelShaderPath` (optional path).
    #[serde(default, rename = "experimental.pixelShaderPath")]
    pub pixel_shader_path: Option<String>,
    /// `experimental.pixelShaderImagePath` (optional path).
    #[serde(default, rename = "experimental.pixelShaderImagePath")]
    pub pixel_shader_image_path: Option<String>,
    /// Padding (left, top, right, bottom) in px. Runtime-only; not in WT JSON.
    #[serde(skip)]
    pub padding: (f32, f32, f32, f32),
}

fn default_opacity_one() -> f32 {
    1.0
}
/// `IntAsFloatPercentConversionTrait::ToJson` (`.h:396-399`).
fn ser_opacity<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u32((v * 100.0f32).round().clamp(0.0, 100.0) as u32)
}
/// `IntAsFloatPercentConversionTrait::FromJson` (`.h:385-389`).
fn de_opacity<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
    Ok(u32::deserialize(d)? as f32 / 100.0)
}

/// Default palette for serde-decode of partial JSON: the Campbell 16-color
/// table. Matches the value set in `Appearance::default`.
fn default_palette() -> [Rgba; 16] {
    campbell_palette()
}
/// Default foreground for serde-decode of partial JSON.
fn default_foreground() -> Rgba {
    crate::defaults::default_foreground()
}
/// Default background for serde-decode of partial JSON.
fn default_background() -> Rgba {
    crate::defaults::default_background()
}
/// Default cursor color for serde-decode of partial JSON.
fn default_cursor_color() -> Rgba {
    crate::defaults::default_cursor_color()
}
/// Default cursor height for serde-decode of partial JSON.
fn default_cursor_height() -> u8 {
    crate::DEFAULT_CURSOR_HEIGHT
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            palette: campbell_palette(),
            foreground: crate::defaults::default_foreground(),
            background: crate::defaults::default_background(),
            selection_background: None,
            cursor_color: crate::defaults::default_cursor_color(),
            cursor_height: crate::DEFAULT_CURSOR_HEIGHT,
            cursor_shape: CursorShape::Bar,
            font: FontConfig::default(),
            opacity: 1.0,
            use_acrylic: false,
            background_image: None,
            background_image_stretch_mode: Stretch::default(),
            background_image_opacity: 1.0,
            background_image_alignment: BackgroundImageAlignment::default(),
            intense_text_style: IntenseStyle::default(),
            adjust_indistinguishable_colors: AdjustTextMode::default(),
            retro_terminal_effect: false,
            pixel_shader_path: None,
            pixel_shader_image_path: None,
            padding: crate::DEFAULT_PADDING,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity: `CursorShape` exactly the 6 C++ variants (no `Double`).
    #[test]
    fn cursor_shape_has_six_cpp_variants() {
        assert_eq!(CursorShape::Bar.as_str(), "bar");
        assert_eq!(CursorShape::Vintage.as_str(), "vintage");
        assert_eq!(CursorShape::Underscore.as_str(), "underscore");
        assert_eq!(CursorShape::DoubleUnderscore.as_str(), "doubleUnderscore");
        assert_eq!(CursorShape::FilledBox.as_str(), "filledBox");
        assert_eq!(CursorShape::EmptyBox.as_str(), "emptyBox");
        assert_eq!(CursorShape::default(), CursorShape::Bar);
    }

    /// Parity: `opacity` is a uint percent on the wire, 0..=1.0 in memory.
    #[test]
    fn opacity_round_trips_as_uint_percent() {
        let a = Appearance {
            opacity: 0.5,
            ..Appearance::default()
        };
        let j = serde_json::to_string(&a).unwrap();
        assert!(j.contains("\"opacity\":50"), "got: {j}");
        let back: Appearance = serde_json::from_str(&j).unwrap();
        assert!((back.opacity - 0.5).abs() < 1e-6);
    }

    /// Parity: `adjustIndistinguishableColors` accepts the bool short-form.
    #[test]
    fn adjust_indistinguishable_colors_accepts_bool() {
        let a: Appearance =
            serde_json::from_str(r#"{"adjustIndistinguishableColors":true}"#).unwrap();
        assert_eq!(a.adjust_indistinguishable_colors, AdjustTextMode::Indexed);
        let j = serde_json::to_string(&a).unwrap();
        assert!(
            j.contains("\"adjustIndistinguishableColors\":\"indexed\""),
            "got: {j}"
        );
    }

    /// Parity: defaults match MTSM_APPEARANCE_SETTINGS.
    #[test]
    fn defaults_match_cpp() {
        let a = Appearance::default();
        assert_eq!(a.opacity, 1.0);
        assert_eq!(a.background_image_stretch_mode, Stretch::UniformToFill);
        assert_eq!(a.background_image_opacity, 1.0);
        assert_eq!(a.intense_text_style, IntenseStyle::Bright);
        assert_eq!(a.adjust_indistinguishable_colors, AdjustTextMode::Automatic);
        assert!(!a.retro_terminal_effect);
        assert_eq!(a.pixel_shader_path, None);
        assert_eq!(a.selection_background, None);
    }
}
