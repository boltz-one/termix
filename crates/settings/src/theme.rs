//! Theme-related types for `GlobalAppSettings` — `Model::ThemePair` and the
//! `ColorSchemes` map value (`GlobalAppSettings.h:44`). Defined here so the
//! main `global_app_settings.rs` stays under 200 lines.

/// `Model::ThemePair` — accepts either a single string (applied to all three
/// slots) or a `{dark, light, system}` object. Mirrors the C++ `ThemePair`
/// FromJson in `GlobalAppSettings.cpp` (JSON `theme` key, `defaults.json:28`).
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ThemeRef {
    /// Bare string (e.g. `"dark"`) — broadcast to `dark`, `light`, `system`.
    Single(String),
    /// `{dark, light, system}` object form (C++ MTSM_THEME_* keys).
    Themed {
        dark: String,
        light: String,
        system: String,
    },
}

impl Default for ThemeRef {
    fn default() -> Self {
        Self::Single("dark".to_string())
    }
}

impl ThemeRef {
    /// Returns the effective theme name for the requested slot.
    pub fn resolve(&self, which: ThemeSlot) -> &str {
        match (self, which) {
            (ThemeRef::Single(s), _) => s.as_str(),
            (ThemeRef::Themed { dark, .. }, ThemeSlot::Dark) => dark.as_str(),
            (ThemeRef::Themed { light, .. }, ThemeSlot::Light) => light.as_str(),
            (ThemeRef::Themed { system, .. }, ThemeSlot::System) => system.as_str(),
        }
    }
}

pub enum ThemeSlot {
    Dark,
    Light,
    System,
}

/// A minimal color-scheme stub. The full `ColorScheme` port lives elsewhere;
/// `GlobalAppSettings::ColorSchemes` (`GlobalAppSettings.h:44`) is a name →
/// scheme map populated by the `cascadia_settings` loader before use.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ColorScheme {
    pub name: String,
}
