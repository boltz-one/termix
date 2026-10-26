//! Enum types for `GlobalAppSettings` (`GlobalAppSettings.h` +
//! `MTSMSettings.h`). All variants serde-rename to WT's camelCase JSON keys
//! (`defaults.json`), matching the C++ enum names in `LaunchMode.idl` /
//! `TabWidthMode` / `TabSwitcherMode` / `ConfirmOnClose`.

/// `Model::LaunchMode` (`GlobalAppSettings.h:64`, `defaults.json:8`).
/// Accepts hyphenated forms WT emits (`maximizedFocus`, `maximizedFullscreen`)
/// and the bare `default`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LaunchMode {
    #[default]
    Default,
    Maximized,
    Fullscreen,
    Focus,
    MaximizedFocus,
    MaximizedFullscreen,
}

/// Tab sizing mode (`MTSM_WINDOW_SETTINGS` `TabWidthMode`; C++ enum
/// `winrt::Microsoft::UI::Xaml::Controls::TabViewWidthMode`. Defaults to
/// `Equal` (`defaults.json:22`).
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TabWidthMode {
    #[default]
    Equal,
    TitleLength,
    Compact,
}

/// `Model::TabSwitcherMode` (`GlobalAppSettings.h`, `defaults.json:23`).
/// Defaults to `InOrder`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TabSwitcherMode {
    #[default]
    InOrder,
    MostRecentlyUsed,
    Disabled,
}

/// `Model::ConfirmOnClose` (`warning.confirmOnClose`, `defaults.json:27`).
/// Defaults to `Automatic` (warn only when > 1 pane).
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConfirmOnClose {
    Never,
    Always,
    #[default]
    Automatic,
}
