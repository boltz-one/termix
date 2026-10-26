//! Enums for `Appearance` (split out so `appearance.rs` stays under 200 lines).
//!
//! Each variant mirrors a `JSON_ENUM_MAPPER` table in
//! `TerminalSettingsSerializationHelpers.h` and round-trips its JSON alias
//! verbatim (no `rename_all` — some aliases are camelCase like
//! `doubleUnderscore` / `uniformToFill`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Macro: string-aliased enum with default variant, `as_str`/`parse` and the
/// standard `Serialize`/`Deserialize` impls. Each generated enum round-trips
/// the named C++ alias as a string.
macro_rules! string_enum {
    ($(#[$attr:meta])* $vis:vis enum $name:ident {
        default = $default:ident;
        $($variant:ident => $alias:literal),* $(,)?
    }) => {
        $(#[$attr])* #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        $vis enum $name { $($variant),* }
        impl Default for $name {
            fn default() -> Self {
                Self::$default
            }
        }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $alias,)*
                }
            }
            pub fn parse(s: &str) -> Option<Self> {
                Some(match s {
                    $($alias => Self::$variant,)*
                    _ => return None,
                })
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                Self::parse(&String::deserialize(d)?)
                    .ok_or_else(|| serde::de::Error::custom(stringify!("unknown " $name)))
            }
        }
    };
}

string_enum! {
    /// `::winrt::Microsoft::Terminal::Core::CursorStyle` (`.h:18-26`). 6 variants.
    pub enum CursorShape {
        default = Bar;
        Bar => "bar", Vintage => "vintage", Underscore => "underscore",
        DoubleUnderscore => "doubleUnderscore", FilledBox => "filledBox", EmptyBox => "emptyBox",
    }
}

string_enum! {
    /// `IntenseTextStyle` (MTSM_APPEARANCE_SETTINGS intValue). 4 variants.
    pub enum IntenseStyle {
        default = Bright;
        None => "none", Bold => "bold", Bright => "bright", All => "all",
    }
}

string_enum! {
    /// `Stretch` (Media Stretch enum). Aliases verbatim from `.h`.
    pub enum Stretch {
        default = UniformToFill;
        None => "none", Fill => "fill", Uniform => "uniform", UniformToFill => "uniformToFill",
    }
}

/// Horizontal component of `ConvergedAlignment` (`.h:142`). `as_str`/`parse`
/// are inlined into `BackgroundImageAlignment` (the only user of these enums).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum HorizontalAlignment {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical component of `ConvergedAlignment` (`.h:142`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

/// `::winrt::Microsoft::Terminal::Core::AdjustTextMode` (`.h:29-58`).
/// C++ also accepts a bool short form (`true`→`Indexed`, `false`→`Never`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum AdjustTextMode {
    Never,
    Indexed,
    Always,
    #[default]
    Automatic,
}
impl AdjustTextMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Indexed => "indexed",
            Self::Always => "always",
            Self::Automatic => "automatic",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "never" => Self::Never,
            "indexed" => Self::Indexed,
            "always" => Self::Always,
            "automatic" => Self::Automatic,
            _ => return None,
        })
    }
}
impl Serialize for AdjustTextMode {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for AdjustTextMode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Str(String),
            Bool(bool),
        }
        match Raw::deserialize(d)? {
            Raw::Bool(true) => Ok(Self::Indexed),
            Raw::Bool(false) => Ok(Self::Never),
            Raw::Str(s) => {
                Self::parse(&s).ok_or_else(|| serde::de::Error::custom("unknown adjust-text mode"))
            }
        }
    }
}

/// `ConvergedAlignment` — Rust splits into horizontal/vertical; JSON form is a
/// single camelCase token (`"topLeft"`, `"center"`, ...). 9 combos.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackgroundImageAlignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}
impl Default for BackgroundImageAlignment {
    fn default() -> Self {
        Self {
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
        }
    }
}
impl BackgroundImageAlignment {
    pub fn as_str(self) -> &'static str {
        match (self.horizontal, self.vertical) {
            (HorizontalAlignment::Center, VerticalAlignment::Center) => "center",
            (HorizontalAlignment::Left, VerticalAlignment::Top) => "topLeft",
            (HorizontalAlignment::Center, VerticalAlignment::Top) => "topCenter",
            (HorizontalAlignment::Right, VerticalAlignment::Top) => "topRight",
            (HorizontalAlignment::Left, VerticalAlignment::Center) => "centerLeft",
            (HorizontalAlignment::Right, VerticalAlignment::Center) => "centerRight",
            (HorizontalAlignment::Left, VerticalAlignment::Bottom) => "bottomLeft",
            (HorizontalAlignment::Center, VerticalAlignment::Bottom) => "bottomCenter",
            (HorizontalAlignment::Right, VerticalAlignment::Bottom) => "bottomRight",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        use HorizontalAlignment as H;
        use VerticalAlignment as V;
        Some(match s {
            "center" => Self::default(),
            "topLeft" => Self {
                horizontal: H::Left,
                vertical: V::Top,
            },
            "topCenter" => Self {
                horizontal: H::Center,
                vertical: V::Top,
            },
            "topRight" => Self {
                horizontal: H::Right,
                vertical: V::Top,
            },
            "centerLeft" => Self {
                horizontal: H::Left,
                vertical: V::Center,
            },
            "centerRight" => Self {
                horizontal: H::Right,
                vertical: V::Center,
            },
            "bottomLeft" => Self {
                horizontal: H::Left,
                vertical: V::Bottom,
            },
            "bottomCenter" => Self {
                horizontal: H::Center,
                vertical: V::Bottom,
            },
            "bottomRight" => Self {
                horizontal: H::Right,
                vertical: V::Bottom,
            },
            _ => return None,
        })
    }
}
impl Serialize for BackgroundImageAlignment {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for BackgroundImageAlignment {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?)
            .ok_or_else(|| serde::de::Error::custom("unknown background-image alignment"))
    }
}
