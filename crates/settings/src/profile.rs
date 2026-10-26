//! `Profile` — port of WT's `Microsoft::Terminal::Settings::Model::Profile`
//! (`windows\src\cascadia\TerminalSettingsModel\Profile.h`).
//!
//! Fields follow `MTSM_PROFILE_SETTINGS` (`MTSMSettings.h:93`) verbatim; the
//! profile-private settings (`guid/name/source/hidden/...`,
//! `Profile.h:120-134`) are kept alongside. The default appearance is stored
//! as [`default_appearance`] so the field name matches the C++ `_DefaultAppearance`.
//!
//! Serde is split into [`crate::profile_serde`] (`from_json` / `layer_json` /
//! `to_json`).

use crate::appearance::Appearance;
use crate::bell_style::BellStyle;
use crate::color::Color;
use crate::defaults::{
    DEFAULT_COMMAND_PROMPT_GUID, DEFAULT_HISTORY_SIZE, DEFAULT_PADDING_STR,
    DEFAULT_STARTING_DIRECTORY, DEFAULT_WINDOWS_POWERSHELL_GUID,
};
use crate::origin_tag::OriginTag;
use crate::profile_enums::{
    CloseOnExitMode, PathTranslationStyle, ScrollbarState, TextAntialiasingMode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// `Microsoft::Terminal.Settings.Model.Profile` — terminal profile.
///
/// Ported verbatim from `Profile.h:79-163` plus `MTSM_PROFILE_SETTINGS`
/// (`MTSMSettings.h:94-124`). Nullable C++ settings are `Option<T>`;
/// settings with defaults are plain `T`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    // -- Profile-private (`Profile.h:120-134`) -------------------------------
    /// `Guid` (wire `guid`). For inbox profiles the canonical PowerShell/cmd
    /// GUIDs (`CascadiaSettingsSerialization.cpp:66-67`).
    pub guid: String,
    /// `Name` (wire `name`).
    pub name: String,
    /// `Source` (wire `source`) — fragment generator name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// `Hidden` (wire `hidden`).
    #[serde(default)]
    pub hidden: bool,
    /// `ConnectionType` (wire `connectionType`) — uuid string.
    #[serde(
        default,
        rename = "connectionType",
        skip_serializing_if = "Option::is_none"
    )]
    pub connection_type: Option<String>,
    /// `Icon` (wire `icon`) — media-resource path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// `TabTitle` (wire `tabTitle`).
    #[serde(default, rename = "tabTitle", skip_serializing_if = "Option::is_none")]
    pub tab_title: Option<String>,
    /// `TabColor` (wire `tabColor`) — optional color.
    #[serde(default, rename = "tabColor", skip_serializing_if = "Option::is_none")]
    pub tab_color: Option<Color>,
    /// `SuppressApplicationTitle` (wire `suppressApplicationTitle`) — Option
    /// because C++ treats it as inheritable-nullable (`Profile.h`).
    #[serde(
        default,
        rename = "suppressApplicationTitle",
        skip_serializing_if = "Option::is_none"
    )]
    pub suppress_application_title: Option<bool>,
    /// `ScrollState` (wire `scrollbarState`; `MTSMSettings.h:99`).
    #[serde(default, rename = "scrollbarState")]
    pub scroll_state: ScrollbarState,
    /// `Padding` (wire `padding`) — verbatim string form (C++ uses
    /// `PermissiveStringConverter<wstring>`, `Profile.cpp:192`). Default
    /// `"8, 8, 8, 8"` (`DEFAULT_PADDING_STR`).
    #[serde(default = "default_padding", rename = "padding")]
    pub padding: String,
    /// `Commandline` (wire `commandline`; `MTSMSettings.h:98`).
    #[serde(default = "default_commandline", rename = "commandline")]
    pub commandline: String,
    /// `StartingDirectory` (wire `startingDirectory`).
    #[serde(
        default,
        rename = "startingDirectory",
        skip_serializing_if = "Option::is_none"
    )]
    pub starting_directory: Option<String>,
    /// `Updates` (`Profile.h:121`; not in the macro, `MTSMSettings.h:128`) —
    /// guid-string pointing at the inbox profile this fragment updates.
    #[serde(default, rename = "updates", skip_serializing_if = "Option::is_none")]
    pub updates: Option<String>,
    /// `Deleted` (`Profile.h:44`) — true if the user explicitly removed this
    /// profile.
    #[serde(
        default,
        rename = "deleted",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub deleted: bool,
    /// `Orphaned` (`Profile.h:46`) — true if the user kept a profile that
    /// disappeared from its generator.
    #[serde(
        default,
        rename = "orphaned",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub orphaned: bool,
    /// `Origin` (`Profile.h:122`; `ISettingsModelObject.idl:7`).
    #[serde(default, skip_serializing_if = "is_origin_none")]
    pub origin: OriginTag,
    /// `_DefaultAppearance` (`Profile.h:131`). Rust-side `to_json`/`from_json`
    /// skip this when it matches the Rust default — appearance round-tripping
    /// is out of scope for `settings-model::Profile` (Open Question in the
    /// worker report).
    #[serde(
        default = "Appearance::default",
        rename = "defaultAppearance",
        skip_serializing_if = "is_default_appearance"
    )]
    pub default_appearance: Appearance,
    /// `UnfocusedAppearance` (`Profile.h:132`; wire `unfocusedAppearance`).
    #[serde(
        default,
        rename = "unfocusedAppearance",
        skip_serializing_if = "Option::is_none"
    )]
    pub unfocused_appearance: Option<Appearance>,

    // -- MTSM_PROFILE_SETTINGS (`MTSMSettings.h:94-124`) --------------------
    /// `HistorySize` (wire `historySize`; `DEFAULT_HISTORY_SIZE = 9001`).
    #[serde(default = "default_history_size", rename = "historySize")]
    pub history_size: u32,
    /// `SnapOnInput` (wire `snapOnInput`).
    #[serde(default = "default_true", rename = "snapOnInput")]
    pub snap_on_input: bool,
    /// `AltGrAliasing` (wire `altGrAliasing`).
    #[serde(default = "default_true", rename = "altGrAliasing")]
    pub alt_gr_aliasing: bool,
    /// `AnswerbackMessage` (wire `answerbackMessage`).
    #[serde(
        default,
        rename = "answerbackMessage",
        skip_serializing_if = "Option::is_none"
    )]
    pub answerback_message: Option<String>,
    /// `AntialiasingMode` (wire `antialiasingMode`).
    #[serde(default, rename = "antialiasingMode")]
    pub antialiasing_mode: TextAntialiasingMode,
    /// `CloseOnExit` (wire `closeOnExit`).
    #[serde(default, rename = "closeOnExit")]
    pub close_on_exit: CloseOnExitMode,
    /// `BellStyle` (wire `bellStyle`) — string OR bool OR array.
    #[serde(default = "default_bell_audible", rename = "bellStyle")]
    pub bell_style: BellStyle,
    /// `BellSound` (wire `bellSound`) — media paths.
    #[serde(default, rename = "bellSound", skip_serializing_if = "Option::is_none")]
    pub bell_sound: Option<Vec<String>>,
    /// `EnvironmentVariables` (wire `environment`).
    #[serde(
        default,
        rename = "environment",
        skip_serializing_if = "Option::is_none"
    )]
    pub environment_variables: Option<HashMap<String, String>>,
    /// `RightClickContextMenu` (wire `rightClickContextMenu`).
    #[serde(default, rename = "rightClickContextMenu")]
    pub right_click_context_menu: bool,
    /// `Elevate` (wire `elevate`).
    #[serde(default, rename = "elevate")]
    pub elevate: bool,
    /// `AutoMarkPrompts` (wire `autoMarkPrompts`).
    #[serde(default = "default_true", rename = "autoMarkPrompts")]
    pub auto_mark_prompts: bool,
    /// `ShowMarks` (wire `showMarksOnScrollbar`).
    #[serde(default, rename = "showMarksOnScrollbar")]
    pub show_marks: bool,
    /// `RepositionCursorWithMouse` (wire `experimental.repositionCursorWithMouse`).
    #[serde(default, rename = "experimental.repositionCursorWithMouse")]
    pub reposition_cursor_with_mouse: bool,
    /// `ReloadEnvironmentVariables` (wire `compatibility.reloadEnvironmentVariables`).
    #[serde(
        default = "default_true",
        rename = "compatibility.reloadEnvironmentVariables"
    )]
    pub reload_environment_variables: bool,
    /// `RainbowSuggestions` (wire `experimental.rainbowSuggestions`).
    #[serde(default, rename = "experimental.rainbowSuggestions")]
    pub rainbow_suggestions: bool,
    /// `ForceVTInput` (wire `compatibility.input.forceVT`).
    #[serde(default, rename = "compatibility.input.forceVT")]
    pub force_vt_input: bool,
    /// `AllowKittyKeyboardMode` (wire `compatibility.kittyKeyboardMode`).
    #[serde(default = "default_true", rename = "compatibility.kittyKeyboardMode")]
    pub allow_kitty_keyboard_mode: bool,
    /// `AllowVtChecksumReport` (wire `compatibility.allowDECRQCRA`).
    #[serde(default, rename = "compatibility.allowDECRQCRA")]
    pub allow_vt_checksum_report: bool,
    /// `AllowVtClipboardWrite` (wire `compatibility.allowOSC52`).
    #[serde(default = "default_true", rename = "compatibility.allowOSC52")]
    pub allow_vt_clipboard_write: bool,
    /// `AllowOscNotifications` (wire `compatibility.allowOSC777`).
    #[serde(default, rename = "compatibility.allowOSC777")]
    pub allow_osc_notifications: bool,
    /// `AllowKeypadMode` (wire `compatibility.allowDECNKM`).
    #[serde(default, rename = "compatibility.allowDECNKM")]
    pub allow_keypad_mode: bool,
    /// `DragDropDelimiter` (wire `dragDropDelimiter`).
    #[serde(default = "default_drag_drop_delimiter", rename = "dragDropDelimiter")]
    pub drag_drop_delimiter: String,
    /// `PathTranslationStyle` (wire `pathTranslationStyle`).
    #[serde(default, rename = "pathTranslationStyle")]
    pub path_translation_style: PathTranslationStyle,
}

fn default_padding() -> String {
    DEFAULT_PADDING_STR.to_string()
}
fn default_commandline() -> String {
    // C++ default: `%SystemRoot%\System32\cmd.exe` (MTSMSettings.h:98).
    "%SystemRoot%\\System32\\cmd.exe".to_string()
}
fn default_history_size() -> u32 {
    DEFAULT_HISTORY_SIZE
}
fn default_true() -> bool {
    true
}
fn default_bell_audible() -> BellStyle {
    BellStyle {
        audible: true,
        ..BellStyle::NONE
    }
}
fn default_drag_drop_delimiter() -> String {
    " ".to_string()
}

fn is_origin_none(o: &OriginTag) -> bool {
    *o == OriginTag::None
}

fn is_default_appearance(a: &Appearance) -> bool {
    // Compare via the JSON form so we don't require `Appearance: PartialEq`
    // (the appearance module intentionally derives only `Clone, Debug` to
    // keep its derive list tight — the f32 channels would make a naive
    // `#[derive(PartialEq)]` brittle).
    serde_json::to_value(a).ok() == serde_json::to_value(Appearance::default()).ok()
}

impl Default for Profile {
    fn default() -> Self {
        // WT's first-shipped PowerShell inbox profile (`defaults.json:38-56`)
        // with the corrected Windows PowerShell GUID
        // (`CascadiaSettingsSerialization.cpp:66`).
        Self::default_powershell()
    }
}

impl Profile {
    /// Inbox Windows PowerShell profile (`defaults.json:38`).
    ///
    /// GUID `{61c54bbd-c2c6-5271-96e7-009a87ff44bf}`
    /// (`CascadiaSettingsSerialization.cpp:66`).
    pub fn default_powershell() -> Self {
        Self {
            guid: DEFAULT_WINDOWS_POWERSHELL_GUID.to_string(),
            name: "Windows PowerShell".to_string(),
            commandline: "%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
                .to_string(),
            icon: Some(format!(
                "ms-appx:///ProfileIcons/{DEFAULT_WINDOWS_POWERSHELL_GUID}.png"
            )),
            starting_directory: Some(DEFAULT_STARTING_DIRECTORY.to_string()),
            source: None,
            hidden: false,
            connection_type: None,
            tab_title: None,
            tab_color: None,
            suppress_application_title: None,
            scroll_state: ScrollbarState::Visible,
            padding: DEFAULT_PADDING_STR.to_string(),
            updates: None,
            deleted: false,
            orphaned: false,
            origin: OriginTag::None,
            default_appearance: Appearance::default(),
            unfocused_appearance: None,
            history_size: DEFAULT_HISTORY_SIZE,
            snap_on_input: true,
            alt_gr_aliasing: true,
            answerback_message: None,
            antialiasing_mode: TextAntialiasingMode::Grayscale,
            close_on_exit: CloseOnExitMode::Automatic,
            bell_style: default_bell_audible(),
            bell_sound: None,
            environment_variables: None,
            right_click_context_menu: false,
            elevate: false,
            auto_mark_prompts: true,
            show_marks: false,
            reposition_cursor_with_mouse: false,
            reload_environment_variables: true,
            rainbow_suggestions: false,
            force_vt_input: false,
            allow_kitty_keyboard_mode: true,
            allow_vt_checksum_report: false,
            allow_vt_clipboard_write: true,
            allow_osc_notifications: false,
            allow_keypad_mode: false,
            drag_drop_delimiter: " ".to_string(),
            path_translation_style: PathTranslationStyle::None,
        }
    }

    /// Inbox Command Prompt profile (`defaults.json:57`).
    ///
    /// GUID `{0caa0dad-35be-5f56-a8ff-afceeeaa6101}`
    /// (`CascadiaSettingsSerialization.cpp:67`).
    pub fn default_cmd() -> Self {
        let mut p = Self::default_powershell();
        p.guid = DEFAULT_COMMAND_PROMPT_GUID.to_string();
        p.name = "Command Prompt".to_string();
        p.commandline = "%SystemRoot%\\System32\\cmd.exe".to_string();
        p.icon = Some(format!(
            "ms-appx:///ProfileIcons/{DEFAULT_COMMAND_PROMPT_GUID}.png"
        ));
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corrected PowerShell GUID (was previously the bogus
    /// `{61c54bbd-2c96-...}` — tf67e4).
    #[test]
    fn default_powershell_has_correct_guid() {
        assert_eq!(
            Profile::default_powershell().guid,
            DEFAULT_WINDOWS_POWERSHELL_GUID
        );
        assert_eq!(
            DEFAULT_WINDOWS_POWERSHELL_GUID,
            "{61c54bbd-c2c6-5271-96e7-009a87ff44bf}"
        );
    }

    /// The corrected Command Prompt GUID.
    #[test]
    fn default_cmd_has_correct_guid() {
        assert_eq!(Profile::default_cmd().guid, DEFAULT_COMMAND_PROMPT_GUID);
        assert_eq!(
            DEFAULT_COMMAND_PROMPT_GUID,
            "{0caa0dad-35be-5f56-a8ff-afceeeaa6101}"
        );
    }

    /// `Profile::default()` is the PowerShell inbox profile (matches WT).
    #[test]
    fn default_is_default_powershell() {
        assert_eq!(Profile::default().guid, Profile::default_powershell().guid);
    }

    /// `default_cmd` keeps the cmd commandline (`defaults.json:60`).
    #[test]
    fn default_cmd_commandline_matches_defaults_json() {
        assert_eq!(
            Profile::default_cmd().commandline,
            "%SystemRoot%\\System32\\cmd.exe",
        );
    }
}
