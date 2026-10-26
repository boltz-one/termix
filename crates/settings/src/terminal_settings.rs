//! `TerminalSettings` — minimal composite view derived from existing
//! profile/appearance/global-setting structs in this crate.
//!
//! Phase 1 (P01 wave 0): a thin resolved bundle handed to `host`/renderer
//! defaults — color scheme reference + font size/family + cursor shape +
//! default size + scrollbar/acrylic/transparency/history-size. Not a
//! replacement cho `Profile`/`Appearance`/`GlobalAppSettings`; nó là thin
//! wrapper over them, mirroring WT's runtime-flattened `TerminalSettings`
//! (`TerminalSettings.h:38-180`).
//!
//! Field defaults come from [`crate::defaults`] (verbatim port of
//! `DefaultSettings.h`).

use crate::appearance_enums::CursorShape;
use crate::color_scheme::ColorScheme;
use crate::defaults::{
    DEFAULT_COLS, DEFAULT_FONT_FACE, DEFAULT_FONT_SIZE, DEFAULT_HISTORY_SIZE, DEFAULT_ROWS,
};
use crate::font::FontConfig;
use crate::profile_enums::ScrollbarState;
use crate::weight::FontWeight;

/// `TerminalSettings` — thin resolved bundle for a single pane. Sub-set of
/// fields pulled verbatim from `Profile` + `Appearance` + `GlobalAppSettings`
/// (the union WT flattens at runtime into `TerminalSettings`).
#[derive(Clone, Debug)]
pub struct TerminalSettings {
    /// Color scheme reference (`Profile.ColorScheme` / `Appearance.ColorSchemeName`).
    /// Phase 1 giữ resolved snapshot (default Campbell-style `ColorScheme`).
    pub color_scheme: ColorScheme,

    /// `FontFace` (`FontConfig.face`) — default `Cascadia Mono`.
    pub font_face: String,
    /// `FontSize` (`FontConfig.size`) — default 12 pt.
    pub font_size: f32,
    /// `FontWeight` (`FontConfig.weight`) — default 400 Normal.
    pub font_weight: FontWeight,

    /// `CursorShape` (`Appearance.CursorShape`) — default `Bar`.
    pub cursor_shape: CursorShape,

    /// `InitialCols` (`GlobalAppSettings.initial_cols`) — default 120.
    pub default_cols: u16,
    /// `InitialRows` (`GlobalAppSettings.initial_rows`) — default 30.
    pub default_rows: u16,

    /// `ScrollbarState` (`Profile.ScrollbarState`) — default `Visible`.
    pub scrollbar_state: ScrollbarState,

    /// `Opacity` (`Appearance.Opacity`) — 0..=1.0; default 1.0 (opaque).
    pub opacity: f32,
    /// `UseAcrylic` (`Appearance.UseAcrylic`) — default false.
    pub use_acrylic: bool,

    /// `HistorySize` (`Profile.HistorySize`) — default 9001 lines.
    pub history_size: u32,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            font_face: DEFAULT_FONT_FACE.to_string(),
            font_size: DEFAULT_FONT_SIZE,
            font_weight: FontWeight::default(),
            cursor_shape: CursorShape::default(),
            default_cols: DEFAULT_COLS,
            default_rows: DEFAULT_ROWS,
            scrollbar_state: ScrollbarState::default(),
            opacity: 1.0,
            use_acrylic: false,
            history_size: DEFAULT_HISTORY_SIZE,
        }
    }
}

impl TerminalSettings {
    /// Resolve font configuration snapshot — convenience accessor matching
    /// WT's `TerminalSettings::GetFontInfo` shape.
    pub fn font_config(&self) -> FontConfig {
        FontConfig {
            face: self.font_face.clone(),
            size: self.font_size,
            weight: self.font_weight,
            ..FontConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 1 smoke: defaults match `DefaultSettings.h` constants.
    #[test]
    fn defaults_match_inbox_constants() {
        let s = TerminalSettings::default();
        assert_eq!(s.font_face, "Cascadia Mono");
        assert_eq!(s.font_size, 12.0);
        assert_eq!(s.default_cols, DEFAULT_COLS);
        assert_eq!(s.default_rows, DEFAULT_ROWS);
        assert_eq!(s.history_size, DEFAULT_HISTORY_SIZE);
        assert_eq!(s.opacity, 1.0);
        assert!(!s.use_acrylic);
    }

    /// `font_config` accessor rounds-trip the resolved `FontConfig`.
    #[test]
    fn font_config_accessor_round_trips() {
        let s = TerminalSettings::default();
        let f = s.font_config();
        assert_eq!(f.face, s.font_face);
        assert_eq!(f.size, s.font_size);
    }
}
