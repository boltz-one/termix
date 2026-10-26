//! Profile-scoped enums — verbatim port of WT's
//! `Microsoft::Terminal::Settings::Model` and `Microsoft::Terminal::Control`
//! enums held by `Profile` (`MTSM_PROFILE_SETTINGS`, `MTSMSettings.h:93`).
//!
//! `BellStyle` is in [`crate::bell_style`] (separate module so this file
//! stays under the size cap). `OriginTag` is in [`crate::origin_tag`] (shared).
//!
//! String mappings mirror `TerminalSettingsSerializationHelpers.h`.

/// `Microsoft::Terminal.Control.ScrollbarState` (helpers:74-81; wire
/// `scrollbarState`). Default `Visible` (`MTSMSettings.h:99`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollbarState {
    #[default]
    Visible,
    Hidden,
    Always,
}

/// `Microsoft::Terminal.Control.TextAntialiasingMode` (helpers:158-165; wire
/// `antialiasingMode`). Default `Grayscale` (`MTSMSettings.h:100`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAntialiasingMode {
    #[default]
    Grayscale,
    Cleartype,
    Aliased,
}

/// `Microsoft::Terminal.Settings.Model.CloseOnExitMode` (`Profile.idl:17`;
/// helpers:171-176; wire `closeOnExit`). The legacy JSON `bool` form (`true →
/// Graceful`, `false → Never`, helpers:179-186) is not exposed via this enum
/// — the profile layer preserves existing values when the key is absent or
/// null. Default `Automatic` (`MTSMSettings.h:105`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloseOnExitMode {
    Never,
    Graceful,
    Always,
    #[default]
    Automatic,
}

/// `Microsoft::Terminal.Control.PathTranslationStyle` (`IControlSettings.idl:23`;
/// helpers:866-875; wire `pathTranslationStyle`). Default `None`
/// (`MTSMSettings.h:124`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathTranslationStyle {
    #[default]
    None,
    Wsl,
    Cygwin,
    Msys2,
    Mingw,
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_state_default_and_round_trip() {
        assert_eq!(ScrollbarState::default(), ScrollbarState::Visible);
        assert_eq!(
            serde_json::from_str::<ScrollbarState>("\"hidden\"").unwrap(),
            ScrollbarState::Hidden,
        );
        assert_eq!(
            serde_json::to_string(&ScrollbarState::Always).unwrap(),
            "\"always\"",
        );
    }

    #[test]
    fn antialiasing_default_and_round_trip() {
        assert_eq!(
            TextAntialiasingMode::default(),
            TextAntialiasingMode::Grayscale
        );
        assert_eq!(
            serde_json::from_str::<TextAntialiasingMode>("\"cleartype\"").unwrap(),
            TextAntialiasingMode::Cleartype,
        );
    }

    #[test]
    fn close_on_exit_default_and_round_trip() {
        assert_eq!(CloseOnExitMode::default(), CloseOnExitMode::Automatic);
        assert_eq!(
            serde_json::from_str::<CloseOnExitMode>("\"graceful\"").unwrap(),
            CloseOnExitMode::Graceful,
        );
    }

    #[test]
    fn path_translation_default_and_round_trip() {
        assert_eq!(PathTranslationStyle::default(), PathTranslationStyle::None);
        assert_eq!(
            serde_json::from_str::<PathTranslationStyle>("\"wsl\"").unwrap(),
            PathTranslationStyle::Wsl,
        );
    }
}
