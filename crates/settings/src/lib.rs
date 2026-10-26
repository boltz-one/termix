//! Settings model for termix.
//!
//! Ports the Windows Terminal `TerminalSettingsModel` semantics into native Rust.
//! The values below are copied verbatim from
//! `windows/src/inc/DefaultSettings.h` so that the default font face, size and
//! weight match Windows Terminal exactly.

pub mod appearance;
pub mod appearance_enums;
pub mod bell_style;
pub mod builtin_schemes;
pub mod color;
pub mod color_scheme;
pub mod defaults;
pub mod editor;
pub mod font;
pub mod global_app_settings;
pub mod global_enums;
pub mod origin_tag;
pub mod profile;
pub mod profile_enums;
pub mod profile_serde;
pub mod terminal_settings;
pub mod theme;
pub mod weight;

pub use appearance::Appearance;
pub use bell_style::BellStyle;
pub use color_scheme::ColorScheme;
pub use defaults::*;
pub use font::FontConfig;
pub use global_app_settings::GlobalAppSettings;
pub use origin_tag::OriginTag;
pub use profile::Profile;
pub use profile_enums::{
    CloseOnExitMode, PathTranslationStyle, ScrollbarState, TextAntialiasingMode,
};
pub use terminal_settings::TerminalSettings;
pub use weight::FontWeight;
