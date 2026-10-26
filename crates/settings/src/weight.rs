//! Font weight mapping; mirrors `DWRITE_FONT_WEIGHT` used by Windows Terminal's
//! AtlasEngine. The numeric weight is what WT persists on the wire (the
//! `FontWeight` setting is a `uint32` that maps 1:1 to OpenType weight class).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// DirectWrite / OpenType weight class values used by WT.
///
/// See: `DWRITE_FONT_WEIGHT` and `FontWeight` in
/// `windows/src/cascadia/TerminalSettingsModel/TerminalSettingsSchema.idl`.
/// Mirrors `TerminalSettingsSerializationHelpers.h:197-248` (FontWeight
/// ConversionTrait, `JSON_MAPPINGS(11)` — eleven named aliases, plus an
/// unsigned int fallback path that clamps to `[100, 990]`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(u16)]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    SemiLight = 350,
    #[default]
    Normal = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
    ExtraBlack = 950,
}

impl FontWeight {
    /// Parse a WT `FontWeight` string ("normal", "bold", or numeric "100".."950").
    /// Mirrors `ConversionTrait::FromJson` (`TerminalSettingsSerializationHelpers.h:224`):
    /// accepts the 11 named aliases plus any unsigned int, then clamps to
    /// `[100, 990]` (keeping the extra-black bucket for `950..=990`).
    pub fn parse(raw: &str) -> Option<Self> {
        let lower = raw.to_ascii_lowercase();
        match lower.as_str() {
            "thin" | "extra-thin" | "extrathin" | "extra thin" => Some(Self::Thin),
            "extra-light" | "extralight" | "extra light" => Some(Self::ExtraLight),
            "light" => Some(Self::Light),
            "semi-light" | "semilight" | "semi light" => Some(Self::SemiLight),
            "normal" | "regular" => Some(Self::Normal),
            "medium" => Some(Self::Medium),
            "semi-bold" | "semibold" | "semi bold" => Some(Self::SemiBold),
            "bold" => Some(Self::Bold),
            "extra-bold" | "extrabold" | "extra bold" => Some(Self::ExtraBold),
            "extra-black" | "extrablack" | "extra black" => Some(Self::ExtraBlack),
            "black" => Some(Self::Black),
            numeric => numeric.parse().ok().and_then(Self::from_u16),
        }
    }

    /// Resolve a numeric OpenType weight class. Clamps to `[100, 990]` per
    /// `ConversionTrait::FromJson` (`TerminalSettingsSerializationHelpers.h:228-237`),
    /// routing `[950, 990]` to `ExtraBlack`.
    pub fn from_u16(value: u16) -> Option<Self> {
        let value = match value {
            0 => return Some(Self::default()),
            100 => Self::Thin,
            200 => Self::ExtraLight,
            300 => Self::Light,
            350 => Self::SemiLight,
            400 => Self::Normal,
            500 => Self::Medium,
            600 => Self::SemiBold,
            700 => Self::Bold,
            800 => Self::ExtraBold,
            900 => Self::Black,
            950 => Self::ExtraBlack,
            v if v < 100 => Self::Thin,
            v if v < 250 => Self::Thin,
            v if v < 325 => Self::ExtraLight,
            v if v < 375 => Self::Light,
            v if v < 450 => Self::Normal,
            v if v < 550 => Self::Medium,
            v if v < 650 => Self::SemiBold,
            v if v < 750 => Self::Bold,
            v if v < 850 => Self::ExtraBold,
            v if v < 950 => Self::Black,
            _ => Self::ExtraBlack,
        };
        Some(value)
    }

    pub fn to_u16(self) -> u16 {
        self as u16
    }

    /// Mirrors `ConversionTrait::ToJson` (`TerminalSettingsSerializationHelpers.h:240-247`):
    /// emit the named alias string if the weight is one of the 11 known
    /// buckets, otherwise emit the raw `${weight}` integer string.
    pub fn to_json_string(self) -> Cow<'static, str> {
        match self {
            Self::Thin => Cow::Borrowed("thin"),
            Self::ExtraLight => Cow::Borrowed("extra-light"),
            Self::Light => Cow::Borrowed("light"),
            Self::SemiLight => Cow::Borrowed("semi-light"),
            Self::Normal => Cow::Borrowed("normal"),
            Self::Medium => Cow::Borrowed("medium"),
            Self::SemiBold => Cow::Borrowed("semi-bold"),
            Self::Bold => Cow::Borrowed("bold"),
            Self::ExtraBold => Cow::Borrowed("extra-bold"),
            Self::Black => Cow::Borrowed("black"),
            Self::ExtraBlack => Cow::Borrowed("extra-black"),
        }
    }
}

impl<'de> Deserialize<'de> for FontWeight {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Accept either a string ("bold" / "500") or an unsigned int, matching
        // `CanConvert` + `FromJson` (`TerminalSettingsSerializationHelpers.h:243-252`).
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Str(String),
            U(u64),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Str(s) => Self::parse(&s)
                .ok_or_else(|| serde::de::Error::custom(format!("unknown font weight: {s}"))),
            Raw::U(n) => Self::from_u16(n.min(u16::MAX as u64) as u16)
                .ok_or_else(|| serde::de::Error::custom(format!("out-of-range weight: {n}"))),
        }
    }
}

impl Serialize for FontWeight {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Named → name string; otherwise raw integer — matches `ToJson`.
        s.serialize_str(&self.to_json_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity: the 12 numeric values match the C++ `JSON_MAPPINGS(11)` table.
    #[test]
    fn numeric_values_match_cpp() {
        assert_eq!(FontWeight::Thin as u16, 100);
        assert_eq!(FontWeight::ExtraLight as u16, 200);
        assert_eq!(FontWeight::Light as u16, 300);
        assert_eq!(FontWeight::SemiLight as u16, 350);
        assert_eq!(FontWeight::Normal as u16, 400);
        assert_eq!(FontWeight::Medium as u16, 500);
        assert_eq!(FontWeight::SemiBold as u16, 600);
        assert_eq!(FontWeight::Bold as u16, 700);
        assert_eq!(FontWeight::ExtraBold as u16, 800);
        assert_eq!(FontWeight::Black as u16, 900);
        assert_eq!(FontWeight::ExtraBlack as u16, 950);
    }

    /// Parity: `parse` accepts all 11 named aliases plus the extra-black
    /// spellings; `heavy` is rejected (C++ does not accept it).
    #[test]
    fn parse_matches_cpp_aliases() {
        assert_eq!(FontWeight::parse("bold"), Some(FontWeight::Bold));
        assert_eq!(FontWeight::parse("Bold"), Some(FontWeight::Bold));
        assert_eq!(
            FontWeight::parse("extra-black"),
            Some(FontWeight::ExtraBlack)
        );
        assert_eq!(
            FontWeight::parse("extra black"),
            Some(FontWeight::ExtraBlack)
        );
        assert_eq!(
            FontWeight::parse("extrablack"),
            Some(FontWeight::ExtraBlack)
        );
        assert_eq!(FontWeight::parse("900"), Some(FontWeight::Black));
        assert_eq!(FontWeight::parse("950"), Some(FontWeight::ExtraBlack));
        // `heavy` is NOT in the C++ table — verbatim port rejects it.
        assert_eq!(FontWeight::parse("heavy"), None);
    }

    /// Parity: `from_u16` clamps inside `[100, 990]` and routes `[950, 990]`
    /// to `ExtraBlack` to match `std::clamp(value, 100u, 990u)`.
    #[test]
    fn from_u16_clamps_to_cpp_buckets() {
        assert_eq!(FontWeight::from_u16(950), Some(FontWeight::ExtraBlack));
        assert_eq!(FontWeight::from_u16(990), Some(FontWeight::ExtraBlack));
        assert_eq!(FontWeight::from_u16(120), Some(FontWeight::Thin));
        assert_eq!(FontWeight::from_u16(0), Some(FontWeight::Normal));
    }

    /// Parity: serde round-trips the C++ name string and a raw uint.
    #[test]
    fn serde_round_trips_cpp_format() {
        let j = serde_json::to_string(&FontWeight::Bold).unwrap();
        assert_eq!(j, "\"bold\"");
        let j = serde_json::to_string(&FontWeight::ExtraBlack).unwrap();
        assert_eq!(j, "\"extra-black\"");
        let w: FontWeight = serde_json::from_str("\"bold\"").unwrap();
        assert_eq!(w, FontWeight::Bold);
        let w: FontWeight = serde_json::from_str("800").unwrap();
        assert_eq!(w, FontWeight::ExtraBold);
    }
}
