//! `GlobalAppSettings` — port of `GlobalAppSettings.h:36-105` plus
//! `MTSM_GLOBAL_ONLY_SETTINGS` (`MTSMSettings.h:22-31`) and
//! `MTSM_WINDOW_SETTINGS` (`MTSMSettings.h:34-79`).
//!
//! Top-level `GlobalAppSettings` object WT persists as the non-profile part of
//! `settings.json`. Field defaults match `defaults.json:1-35` and
//! `DefaultSettings.h`.

use std::collections::HashMap;

use crate::global_enums::{ConfirmOnClose, LaunchMode, TabSwitcherMode, TabWidthMode};
use crate::theme::{ColorScheme, ThemeRef};
use crate::{DEFAULT_COLS, DEFAULT_ROWS, DEFAULT_WINDOWS_POWERSHELL_GUID, DEFAULT_WORD_DELIMITERS};

/// `GlobalAppSettings` (`GlobalAppSettings.h:36-105`). Top-level
/// `settings.json` fields, with serde-renamed keys matching WT's JSON.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GlobalAppSettings {
    /// `"defaultProfile"` — guid string. Defaults to PowerShell inbox profile
    /// (`defaults.json:3` = `DEFAULT_WINDOWS_POWERSHELL_GUID`).
    #[serde(rename = "defaultProfile")]
    pub default_profile: String,
    /// `"initialCols"` (`MTSM_WINDOW_SETTINGS:36` normally defaults 80; inbox
    /// `defaults.json:6` overrides to 120 = `DEFAULT_COLS`).
    #[serde(rename = "initialCols")]
    pub initial_cols: u16,
    /// `"initialRows"` (`MTSM_WINDOW_SETTINGS:35`, `defaults.json:7` = 30).
    #[serde(rename = "initialRows")]
    pub initial_rows: u16,
    /// `"launchMode"` (`GlobalAppSettings.h:64`).
    #[serde(rename = "launchMode")]
    pub launch_mode: LaunchMode,
    /// `"alwaysOnTop"` (`defaults.json:9`).
    #[serde(rename = "alwaysOnTop")]
    pub always_on_top: bool,
    /// `"copyOnSelect"` (`defaults.json:12`).
    #[serde(rename = "copyOnSelect")]
    pub copy_on_select: bool,
    /// `"copyFormatting"` (`defaults.json:13`).
    #[serde(rename = "copyFormatting")]
    pub copy_formatting: bool,
    /// `"trimBlockSelection"` (`defaults.json:14`).
    #[serde(rename = "trimBlockSelection")]
    pub trim_block_selection: bool,
    /// `"trimPaste"` (`defaults.json:15`).
    #[serde(rename = "trimPaste")]
    pub trim_paste: bool,
    /// `"wordDelimiters"` (`defaults.json:16`, `DefaultSettings.h:46`).
    #[serde(rename = "wordDelimiters")]
    pub word_delimiters: String,
    /// `"alwaysShowTabs"` (`defaults.json:19`).
    #[serde(rename = "alwaysShowTabs")]
    pub always_show_tabs: bool,
    /// `"showTabsInTitlebar"` (`defaults.json:20`).
    #[serde(rename = "showTabsInTitlebar")]
    pub show_tabs_in_titlebar: bool,
    /// `"showTerminalTitleInTitlebar"` (`defaults.json:21`).
    #[serde(rename = "showTerminalTitleInTitlebar")]
    pub show_terminal_title_in_titlebar: bool,
    /// `"tabWidthMode"` (`defaults.json:22`).
    #[serde(rename = "tabWidthMode")]
    pub tab_width_mode: TabWidthMode,
    /// `"tabSwitcherMode"` (`defaults.json:23`).
    #[serde(rename = "tabSwitcherMode")]
    pub tab_switcher_mode: TabSwitcherMode,
    /// `"showAdminShield"` (`defaults.json:24`).
    #[serde(rename = "showAdminShield")]
    pub show_admin_shield: bool,
    /// `"warning.confirmOnClose"` (`defaults.json:27`).
    #[serde(rename = "warning.confirmOnClose")]
    pub confirm_on_close: ConfirmOnClose,
    /// `"theme"` (`defaults.json:28`). String OR `{dark,light,system}`.
    pub theme: ThemeRef,
    /// `"snapToGridOnResize"` (`defaults.json:29`).
    #[serde(rename = "snapToGridOnResize")]
    pub snap_to_grid_on_resize: bool,
    /// `"disableAnimations"` (`defaults.json:30`).
    #[serde(rename = "disableAnimations")]
    pub disable_animations: bool,
    /// `"startupActions"` (`defaults.json:31`).
    #[serde(rename = "startupActions")]
    pub startup_actions: String,
    /// `"focusFollowMouse"` (`defaults.json:32`).
    #[serde(rename = "focusFollowMouse")]
    pub focus_follow_mouse: bool,
    /// `"minimizeToNotificationArea"` (`defaults.json:33`).
    #[serde(rename = "minimizeToNotificationArea")]
    pub minimize_to_notification_area: bool,
    /// `"alwaysShowNotificationIcon"` (`defaults.json:34`,
    /// `MTSM_GLOBAL_ONLY_SETTINGS:27`).
    #[serde(rename = "alwaysShowNotificationIcon")]
    pub always_show_notification_icon: bool,
    /// `"debugFeatures"` (`MTSM_GLOBAL_ONLY_SETTINGS:25`,
    /// `GlobalAppSettings.h:80-84`): defaults to `debugFeaturesDefault` =
    /// `true` in debug builds, else `false`.
    #[serde(rename = "debugFeatures")]
    pub debug_features: bool,
    /// `"compatibility.input.forceWin"` (alias for `ForceVTInput`; see
    /// `MTSM_PROFILE_SETTINGS` `ForceVTInput`). Defaults false.
    #[serde(rename = "compatibility.input.forceWin")]
    pub input_force_win: bool,
    /// `"colorSchemes"` (`GlobalAppSettings.h:44`) — name → scheme map. Inbox
    /// `defaults.json` does not list these at the top level; the
    /// `cascadia_settings` loader fills this in.
    #[serde(rename = "colorSchemes", skip_serializing_if = "HashMap::is_empty")]
    pub color_schemes: HashMap<String, ColorScheme>,
}

impl Default for GlobalAppSettings {
    fn default() -> Self {
        Self {
            default_profile: DEFAULT_WINDOWS_POWERSHELL_GUID.to_string(),
            initial_cols: DEFAULT_COLS,
            initial_rows: DEFAULT_ROWS,
            launch_mode: LaunchMode::default(),
            always_on_top: false,
            copy_on_select: false,
            copy_formatting: true,
            trim_block_selection: true,
            trim_paste: true,
            word_delimiters: DEFAULT_WORD_DELIMITERS.to_string(),
            always_show_tabs: true,
            show_tabs_in_titlebar: true,
            show_terminal_title_in_titlebar: true,
            tab_width_mode: TabWidthMode::default(),
            tab_switcher_mode: TabSwitcherMode::default(),
            show_admin_shield: true,
            confirm_on_close: ConfirmOnClose::default(),
            theme: ThemeRef::default(),
            snap_to_grid_on_resize: true,
            disable_animations: false,
            startup_actions: String::new(),
            focus_follow_mouse: false,
            minimize_to_notification_area: false,
            always_show_notification_icon: false,
            debug_features: cfg!(debug_assertions),
            input_force_win: false,
            color_schemes: HashMap::new(),
        }
    }
}

impl GlobalAppSettings {
    /// Deserialize from a JSON document.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Serialize back to a JSON document.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeSlot;

    /// Parity check: `GlobalAppSettings::default()` matches `defaults.json:1-35`.
    #[test]
    fn global_app_settings_defaults_match_defaults_json() {
        let g = GlobalAppSettings::default();
        assert_eq!(g.default_profile, DEFAULT_WINDOWS_POWERSHELL_GUID);
        assert_eq!(g.initial_cols, 120);
        assert_eq!(g.initial_rows, 30);
        assert_eq!(g.launch_mode, LaunchMode::Default);
        assert!(!g.always_on_top);
        assert!(!g.copy_on_select);
        assert!(g.copy_formatting);
        assert!(g.trim_block_selection && g.trim_paste);
        assert_eq!(g.word_delimiters, DEFAULT_WORD_DELIMITERS);
        assert!(g.always_show_tabs && g.show_tabs_in_titlebar);
        assert!(g.show_terminal_title_in_titlebar);
        assert_eq!(g.tab_width_mode, TabWidthMode::Equal);
        assert_eq!(g.tab_switcher_mode, TabSwitcherMode::InOrder);
        assert!(g.show_admin_shield);
        assert_eq!(g.confirm_on_close, ConfirmOnClose::Automatic);
        assert_eq!(g.theme, ThemeRef::Single("dark".to_string()));
        assert!(g.snap_to_grid_on_resize);
        assert!(!g.disable_animations && g.startup_actions.is_empty());
        assert!(!g.focus_follow_mouse);
        assert!(!g.minimize_to_notification_area);
        assert!(!g.always_show_notification_icon);
    }

    /// `theme` accepts either a bare string or a `{dark,light,system}` object.
    #[test]
    fn theme_ref_accepts_string_or_object() {
        let single: ThemeRef = serde_json::from_str("\"dark\"").unwrap();
        assert_eq!(single, ThemeRef::Single("dark".to_string()));
        assert_eq!(
            ThemeRef::Single("dark".into()).resolve(ThemeSlot::Light),
            "dark"
        );

        let themed: ThemeRef =
            serde_json::from_str(r#"{"dark":"dark","light":"light","system":"system"}"#).unwrap();
        assert_eq!(themed.resolve(ThemeSlot::Dark), "dark");
        assert_eq!(themed.resolve(ThemeSlot::Light), "light");
        assert_eq!(themed.resolve(ThemeSlot::System), "system");
    }

    /// `debug_features` toggles with build profile (C++ `debugFeaturesDefault`).
    #[test]
    fn debug_features_default_follows_cfg() {
        assert_eq!(
            GlobalAppSettings::default().debug_features,
            cfg!(debug_assertions)
        );
    }

    /// Round-trip preserves the default values.
    #[test]
    fn global_app_settings_round_trip_preserves_defaults() {
        let g = GlobalAppSettings::default();
        let back = GlobalAppSettings::from_json(&g.to_json().unwrap()).unwrap();
        assert_eq!(back.default_profile, g.default_profile);
        assert_eq!(back.initial_cols, g.initial_cols);
        assert_eq!(back.launch_mode, g.launch_mode);
        assert_eq!(back.theme, g.theme);
        // Empty color_schemes skipped on serialize but defaults to empty.
        assert!(back.color_schemes.is_empty());
    }

    /// `INBOX_SETTINGS_JSON` and `USER_DEFAULTS_JSON` are embedded (non-empty).
    #[test]
    fn embedded_jsons_are_nonempty() {
        assert!(crate::INBOX_SETTINGS_JSON.starts_with("// THIS IS AN AUTO-GENERATED FILE"));
        assert!(crate::USER_DEFAULTS_JSON.contains("defaultProfile"));
    }
}
