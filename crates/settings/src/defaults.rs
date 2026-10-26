//! Verbatim port of `windows/src/inc/DefaultSettings.h`.
//!
//! These constants define the exact default behavior of Windows Terminal. We
//! reproduce them with the same numeric values and named types so that a fresh
//! `termix` instance displays the same default font, size, weight,
//! dimensions and padding as Windows Terminal.

use gpui::Rgba;

/// `COLOR_WHITE = til::color{0xff, 0xff, 0xff}` (`DefaultSettings.h:17`).
///
/// GPUI's `rgb()` is not `const`, so we construct the literal directly:
/// `f32` channels in 0..1, alpha 1.0.
pub const COLOR_WHITE_RGBA: Rgba = Rgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
/// `COLOR_BLACK = til::color{0, 0, 0}` (`DefaultSettings.h:18`).
pub const COLOR_BLACK_RGBA: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// `DEFAULT_FOREGROUND = COLOR_WHITE`.
pub fn default_foreground() -> Rgba {
    COLOR_WHITE_RGBA
}
/// `DEFAULT_BACKGROUND = COLOR_BLACK`.
pub fn default_background() -> Rgba {
    COLOR_BLACK_RGBA
}

/// `DEFAULT_FONT_FACE = L"Cascadia Mono"` (`DefaultSettings.h:29`).
pub const DEFAULT_FONT_FACE: &str = "Cascadia Mono";

/// Faces tried in order when `face` is unavailable. WT resolves font
/// fallbacks through DirectWrite's font family resolution; this list is the
/// hard-coded fallback chain WT ships for the in-box control. We keep the
/// primary monospace candidates from WT's `defaults.json` `font.fallbacks`.
pub const DEFAULT_FONT_FALLBACKS: &[&str] = &[
    "Cascadia Mono",
    "Cascadia Code",
    "Consolas",
    "Courier New",
    "MS Gothic",
];

/// `DEFAULT_FONT_SIZE = 12` (points) (`DefaultSettings.h:30`).
pub const DEFAULT_FONT_SIZE: f32 = 12.0;

/// `DEFAULT_FONT_WEIGHT = 400` (Normal) (`DefaultSettings.h:31`).
pub const DEFAULT_FONT_WEIGHT: u16 = 400;

/// `DEFAULT_ROWS = 30` (`DefaultSettings.h:33`).
pub const DEFAULT_ROWS: u16 = 30;
/// `DEFAULT_COLS = 120` (`DefaultSettings.h:34`).
pub const DEFAULT_COLS: u16 = 120;

/// `MINIMUM_VISIBLE_CELLS = 2` (`DefaultSettings.h:38`) — VT theoretical
/// minimum (DECSTBM / DECSLRM). A 1-cell viewport can hang `TextBuffer::Reflow`.
pub const MINIMUM_VISIBLE_CELLS: u16 = 2;

/// `DEFAULT_PADDING = L"8, 8, 8, 8"` (left, top, right, bottom; px)
/// (`DefaultSettings.h:40`). WT stores padding as a string for JSON
/// round-tripping; the runtime consumes it as `(f32, f32, f32, f32)`.
pub const DEFAULT_PADDING_STR: &str = "8, 8, 8, 8";
/// Default padding tuple `(left, top, right, bottom)` in px.
pub const DEFAULT_PADDING: (f32, f32, f32, f32) = (8.0, 8.0, 8.0, 8.0);

/// `DEFAULT_STARTING_DIRECTORY = L"%USERPROFILE%"` (`DefaultSettings.h:41`).
/// Kept verbatim; resolved at launch via env expansion.
pub const DEFAULT_STARTING_DIRECTORY: &str = "%USERPROFILE%";

/// `DEFAULT_CURSOR_COLOR = COLOR_WHITE` (`DefaultSettings.h:43`).
pub fn default_cursor_color() -> Rgba {
    COLOR_WHITE_RGBA
}

/// `DEFAULT_CURSOR_HEIGHT = 25` (`DefaultSettings.h:44`) — percent of cell
/// height for the legacy ("vintage") block cursor.
pub const DEFAULT_CURSOR_HEIGHT: u8 = 25;

/// `DEFAULT_HISTORY_SIZE = 9001` (`DefaultSettings.h:24`) — scrollback lines.
pub const DEFAULT_HISTORY_SIZE: u32 = 9001;

/// `OPACITY_OPAQUE = 0xff000000` (`DefaultSettings.h:19`) — a `COLORREF`
/// (0x00BBGGRR form) with the alpha byte forced to 0xFF; WT stores the full
/// 32-bit `COLORREF`. We expose only the alpha byte as the runtime opacity
/// (0xFF = 100% opaque — matches the high byte of the C++ constant).
pub const OPACITY_OPAQUE: u8 = 0xff;

/// `DEFAULT_WORD_DELIMITERS` (`DefaultSettings.h:46`) — used by the WT
/// `Selection` double-click word expansion. Reproduced verbatim.
pub const DEFAULT_WORD_DELIMITERS: &str = " ./\\()\"'-:,.;<>~!@#$%^&*|+=[]{}~?\u{2502}";

/// `DEFAULT_WINDOWS_POWERSHELL_GUID` (`CascadiaSettingsSerialization.cpp:66`)
/// = `{0x61c54bbd, 0xc2c6, 0x5271, {0x96,0xe7,0x00,0x9a,0x87,0xff,0x44,0xbf}}`.
/// Used as the JSON-string default for `defaultProfile` in `defaults.json:3`.
pub const DEFAULT_WINDOWS_POWERSHELL_GUID: &str = "{61c54bbd-c2c6-5271-96e7-009a87ff44bf}";

/// `DEFAULT_COMMAND_PROMPT_GUID` (`CascadiaSettingsSerialization.cpp:67`)
/// = `{0x0caa0dad, 0x35be, 0x5f56, {0xa8,0xff,0xaf,0xce,0xee,0xaa,0x61,0x01}}`.
pub const DEFAULT_COMMAND_PROMPT_GUID: &str = "{0caa0dad-35be-5f56-a8ff-afceeeaa6101}";

/// `RUNTIME_GENERATED_PROFILE_NAMESPACE_GUID` (`Profile.h:75`) =
/// `{0xf65ddb7e, 0x706b, 0x4499, {0x8a,0x50,0x40,0x31,0x3c,0xaf,0x51,0x0a}}`.
/// WT v5 UUID namespace for auto-generated profile GUIDs (`Profile.cpp:313`).
/// Stored as raw 16-byte little-endian layout (`Data1`/`Data2`/`Data3`/`Data4`)
/// matching `winrt::guid`'s memory representation.
pub const RUNTIME_GENERATED_PROFILE_NAMESPACE_GUID: [u8; 16] = [
    0x7e, 0xdb, 0x5d, 0xf6, 0x6b, 0x70, 0x99, 0x44, 0x8a, 0x50, 0x40, 0x31, 0x3c, 0xaf, 0x51, 0x0a,
];

/// WT's embedded inbox defaults document (`CascadiaSettingsSerialization.cpp:32`,
/// `DefaultsFilename = L"defaults.json"`). Loaded as the lowest-priority
/// layer under the user's `settings.json`.
///
/// JSONC: contains `//` comments; strip those (`serde_json` does not parse
/// them by default — use `Deserializer::from_str(...).disable_comments()` or
/// a pre-strip pass) before `serde_json::from_str`.
pub const INBOX_SETTINGS_JSON: &str =
    include_str!("../../../../windows/src/cascadia/TerminalSettingsModel/defaults.json");

/// First-launch user template (`userDefaults.json`). WT writes it to
/// `%LOCALAPPDATA%\Packages\<pkg>\LocalState\settings.json` on first run.
///
/// JSONC: also contains `//` comments; strip before `serde_json::from_str`.
pub const USER_DEFAULTS_JSON: &str =
    include_str!("../../../../windows/src/cascadia/TerminalSettingsModel/userDefaults.json");

/// Convert a point size in `pt` to pixels at a given DPI.
///
/// WT uses the DirectWrite formula `px = pt * dpi / 72`. The reference DPI is
/// 96 (User DPI); at 96 DPI this is the standard `pt * 4 / 3`.
pub fn points_to_pixels(points: f32, dpi: f32) -> f32 {
    points * dpi / 72.0
}

/// The reference DPI assumed by Windows Terminal unless per-monitor DPI is
/// applied. We keep 96 as the baseline; the renderer scales up using the
/// platform-reported effective DPI.
pub const REFERENCE_DPI: f32 = 96.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity check: the numeric defaults match `DefaultSettings.h`.
    #[test]
    fn defaults_match_default_settings_h() {
        assert_eq!(DEFAULT_FONT_FACE, "Cascadia Mono");
        assert_eq!(DEFAULT_FONT_SIZE, 12.0);
        assert_eq!(DEFAULT_FONT_WEIGHT, 400);
        assert_eq!(DEFAULT_ROWS, 30);
        assert_eq!(DEFAULT_COLS, 120);
        assert_eq!(DEFAULT_HISTORY_SIZE, 9001);
        assert_eq!(DEFAULT_CURSOR_HEIGHT, 25);
        assert_eq!(MINIMUM_VISIBLE_CELLS, 2);
        assert_eq!(DEFAULT_PADDING, (8.0, 8.0, 8.0, 8.0));
        assert_eq!(DEFAULT_PADDING_STR, "8, 8, 8, 8");
        assert_eq!(DEFAULT_STARTING_DIRECTORY, "%USERPROFILE%");
        assert_eq!(OPACITY_OPAQUE, 0xff);
        assert_eq!(default_foreground(), COLOR_WHITE_RGBA);
        assert_eq!(default_background(), COLOR_BLACK_RGBA);
        assert_eq!(default_cursor_color(), COLOR_WHITE_RGBA);
        // WT's pt→px formula at 96dpi: 12pt -> 16px
        assert_eq!(points_to_pixels(12.0, 96.0) as u32, 16);
    }

    /// Parity check: the default word delimiters match `DefaultSettings.h:46`.
    #[test]
    fn default_word_delimiters_match() {
        // Reproduced byte-for-byte; the trailing entry is U+2502 (│),
        // whose UTF-8 tail byte is 0x82 (after the 0xE2 0x94 lead).
        assert_eq!(
            DEFAULT_WORD_DELIMITERS.as_bytes().last().copied(),
            Some(0x82)
        );
        assert!(DEFAULT_WORD_DELIMITERS.contains('\u{2502}'));
        assert!(DEFAULT_WORD_DELIMITERS.contains(' '));
        assert!(DEFAULT_WORD_DELIMITERS.contains('/'));
        assert!(DEFAULT_WORD_DELIMITERS.contains('"'));
        assert!(DEFAULT_WORD_DELIMITERS.contains('\''));
    }

    /// Parity check: the inbox profile GUIDs match the C++
    /// `DEFAULT_WINDOWS_POWERSHELL_GUID` / `DEFAULT_COMMAND_PROMPT_GUID`
    /// (`CascadiaSettingsSerialization.cpp:66-67`).
    #[test]
    fn inbox_profile_guids_match_cpp() {
        assert_eq!(
            DEFAULT_WINDOWS_POWERSHELL_GUID,
            "{61c54bbd-c2c6-5271-96e7-009a87ff44bf}"
        );
        assert_eq!(
            DEFAULT_COMMAND_PROMPT_GUID,
            "{0caa0dad-35be-5f56-a8ff-afceeeaa6101}"
        );
    }

    /// Parity check: `RUNTIME_GENERATED_PROFILE_NAMESPACE_GUID` matches
    /// `Profile.h:75` byte layout (little-endian `Data1`/`Data2`/`Data3`,
    /// big-endian `Data4`).
    #[test]
    fn runtime_generated_profile_namespace_guid_match_cpp() {
        // Data1 = 0xf65ddb7e in LE = [7e db 5d f6]; Data2 = 0x706b LE = [6b 70];
        // Data3 = 0x4499 LE = [99 44]; Data4 = [8a 50 40 31 3c af 51 0a].
        assert_eq!(
            RUNTIME_GENERATED_PROFILE_NAMESPACE_GUID,
            [
                0x7e, 0xdb, 0x5d, 0xf6, 0x6b, 0x70, 0x99, 0x44, 0x8a, 0x50, 0x40, 0x31, 0x3c, 0xaf,
                0x51, 0x0a
            ]
        );
    }

    /// The inbox JSON documents are embedded via `include_str!`.
    #[test]
    fn inbox_jsons_embedded() {
        assert!(INBOX_SETTINGS_JSON.starts_with("// THIS IS AN AUTO-GENERATED FILE"));
        assert!(INBOX_SETTINGS_JSON.contains("defaultProfile"));
        assert!(USER_DEFAULTS_JSON.contains("defaultProfile"));
    }
}
