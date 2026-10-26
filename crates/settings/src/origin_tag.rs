//! `OriginTag` — port of WT's `Microsoft.Terminal.Settings.Model.OriginTag`
//! (`windows\src\cascadia\TerminalSettingsModel\ISettingsModelObject.idl:7`).
//!
//! Shared by `Profile`, `Appearance`, `ColorScheme`, and `FontConfig` to
//! record where the object came from (user JSON, inbox defaults, a fragment
//! extension, or generator-driven). Kept in its own module so every model
//! module imports the same enum instead of redefining a parallel one.

use serde::{Deserialize, Serialize};

/// Identifies the source context of a settings object.
///
/// Matches `ISettingsModelObject.idl:7-15` byte-for-byte:
/// `None, User, InBox, Generated, Fragment, ProfilesDefaults`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum OriginTag {
    /// No provenance — the default.
    #[default]
    None = 0,
    /// Came from the user's `settings.json`.
    User = 1,
    /// Came from the embedded inbox `defaults.json`.
    InBox = 2,
    /// Produced by a dynamic profile generator (e.g. PowerShell discovery).
    Generated = 3,
    /// Came from a fragment extension (`TerminalFragment`).
    Fragment = 4,
    /// The synthesized `profiles.defaults` base layer.
    ProfilesDefaults = 5,
}

impl OriginTag {
    /// Serialize as the lowercase WT JSON string. Mirrors the
    /// `ConversionTrait<OriginTag>` mapping used throughout
    /// `TerminalSettingsSerializationHelpers.h`.
    pub fn as_json_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::User => "user",
            Self::InBox => "inbox",
            Self::Generated => "generated",
            Self::Fragment => "fragment",
            Self::ProfilesDefaults => "profilesDefaults",
        }
    }

    /// Inverse of [`Self::as_json_str`]; returns `None` for unknown input.
    pub fn from_json_str(s: &str) -> Option<Self> {
        Some(match s {
            "none" => Self::None,
            "user" => Self::User,
            "inbox" | "inBox" => Self::InBox,
            "generated" => Self::Generated,
            "fragment" => Self::Fragment,
            "profilesDefaults" => Self::ProfilesDefaults,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity: 6 variants with the WT idl ordering and numeric values.
    #[test]
    fn origin_tag_matches_isettings_model_object_idl() {
        assert_eq!(OriginTag::None as u8, 0);
        assert_eq!(OriginTag::User as u8, 1);
        assert_eq!(OriginTag::InBox as u8, 2);
        assert_eq!(OriginTag::Generated as u8, 3);
        assert_eq!(OriginTag::Fragment as u8, 4);
        assert_eq!(OriginTag::ProfilesDefaults as u8, 5);
    }

    /// Round-trip via JSON string matches the lowercase WT naming.
    #[test]
    fn origin_tag_json_round_trip() {
        for v in [
            OriginTag::None,
            OriginTag::User,
            OriginTag::InBox,
            OriginTag::Generated,
            OriginTag::Fragment,
            OriginTag::ProfilesDefaults,
        ] {
            assert_eq!(OriginTag::from_json_str(v.as_json_str()), Some(v));
        }
        assert!(OriginTag::from_json_str("bogus").is_none());
    }
}
