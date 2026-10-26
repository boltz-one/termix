//! `BellStyle` — verbatim port of WT's flag enum (`Profile.idl:26`;
//! helpers:110-140). Kept in its own module so the enum module stays under
//! the 200-line file-size cap.
//!
//! Accepts the same four JSON shapes WT's `BaseFlagMapper::FromJson`
//! (`JsonUtils.h:1024-1053`) accepts:
//! - string — `"none" | "audible" | "visual" | "window" | "taskbar" |
//!   "notification" | "all"`.
//! - bool — `true` ⇒ all, `false` ⇒ none (helpers:124-128).
//! - array of strings ⇒ OR-matched.
//!
/// `"visual"` deserializes to `{window, taskbar}`. Per `BaseFlagMapper::ToJson`
/// (`JsonUtils.h:1055-1083`), a multi-flag value serializes as an array, so
/// `"visual"` re-emits as `["window","taskbar"]` (not as the original string).
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `Microsoft::Terminal.Settings.Model.BellStyle` four flags. Modeled as
/// booleans to match the editor's
/// `SetBellStyle{Audible,Window,Taskbar,Notification}` accessors
/// (`ProfileViewModel.idl:51-54`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BellStyle {
    pub audible: bool,
    pub window: bool,
    pub taskbar: bool,
    pub notification: bool,
}

impl BellStyle {
    /// All flags clear (`"none"` / `false`).
    pub const NONE: Self = Self {
        audible: false,
        window: false,
        taskbar: false,
        notification: false,
    };
    /// All flags set (`"all"` / `true`).
    pub const ALL: Self = Self {
        audible: true,
        window: true,
        taskbar: true,
        notification: true,
    };

    fn from_flag(name: &str) -> Result<Self, String> {
        Ok(match name {
            "none" => Self::NONE,
            "audible" => Self {
                audible: true,
                ..Self::NONE
            },
            "visual" => Self {
                window: true,
                taskbar: true,
                ..Self::NONE
            },
            "window" => Self {
                window: true,
                ..Self::NONE
            },
            "taskbar" => Self {
                taskbar: true,
                ..Self::NONE
            },
            "notification" => Self {
                notification: true,
                ..Self::NONE
            },
            "all" => Self::ALL,
            other => return Err(format!("invalid bellStyle flag `{other}`")),
        })
    }

    fn merge(&mut self, other: Self) {
        self.audible |= other.audible;
        self.window |= other.window;
        self.taskbar |= other.taskbar;
        self.notification |= other.notification;
    }

    fn count_set(self) -> u8 {
        [self.audible, self.window, self.taskbar, self.notification]
            .iter()
            .filter(|&&b| b)
            .count() as u8
    }

    /// Matches `BaseFlagMapper::ToJson` (`JsonUtils.h:1055-1083`):
    /// - exactly `AllClear` ⇒ `"none"`;
    /// - exactly `AllSet` ⇒ `"all"`;
    /// - single flag ⇒ its string;
    /// - multi-flag (e.g. window+taskbar) ⇒ array `[...]` (in helpers.mappings
    ///   order: `audible, visual(never), window, taskbar, notification`).
    fn to_json_value(self) -> serde_json::Value {
        if self == Self::NONE {
            return serde_json::Value::String("none".to_string());
        }
        if self == Self::ALL {
            return serde_json::Value::String("all".to_string());
        }
        if self.count_set() == 1 {
            let s = if self.audible {
                "audible"
            } else if self.window {
                "window"
            } else if self.taskbar {
                "taskbar"
            } else {
                "notification"
            };
            return serde_json::Value::String(s.to_string());
        }
        // Multi-flag — emit array in helpers.mappings order, skipping "visual"
        // (it's multi-flag, never emitted as an array entry).
        let mut flags = Vec::new();
        if self.audible {
            flags.push("audible".to_string());
        }
        if self.window {
            flags.push("window".to_string());
        }
        if self.taskbar {
            flags.push("taskbar".to_string());
        }
        if self.notification {
            flags.push("notification".to_string());
        }
        serde_json::Value::Array(flags.into_iter().map(serde_json::Value::String).collect())
    }
}

impl Serialize for BellStyle {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        self.to_json_value().serialize(ser)
    }
}

impl<'de> Deserialize<'de> for BellStyle {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(de)?;
        match &v {
            serde_json::Value::String(s) => Self::from_flag(s).map_err(serde::de::Error::custom),
            serde_json::Value::Bool(b) => Ok(if *b { Self::ALL } else { Self::NONE }),
            serde_json::Value::Array(items) => {
                let mut acc = Self::NONE;
                for it in items {
                    let name = it.as_str().ok_or_else(|| {
                        serde::de::Error::custom("bellStyle array entries must be strings")
                    })?;
                    acc.merge(Self::from_flag(name).map_err(serde::de::Error::custom)?);
                }
                Ok(acc)
            }
            other => Err(serde::de::Error::custom(format!(
                "bellStyle must be string, bool, or array — got {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_round_trip_audible() {
        let s: BellStyle = serde_json::from_str("\"audible\"").unwrap();
        assert!(s.audible && !s.window && !s.taskbar && !s.notification);
        assert_eq!(serde_json::to_string(&s).unwrap(), "\"audible\"");
        assert_eq!(serde_json::to_string(&BellStyle::NONE).unwrap(), "\"none\"");
        assert_eq!(serde_json::to_string(&BellStyle::ALL).unwrap(), "\"all\"");
    }

    #[test]
    fn visual_deserializes_to_window_plus_taskbar_re_emits_array() {
        let s: BellStyle = serde_json::from_str("\"visual\"").unwrap();
        assert!(s.window && s.taskbar && !s.audible && !s.notification);
        // Multi-flag serializes as array (matches WT BaseFlagMapper::ToJson).
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            "[\"window\",\"taskbar\"]"
        );
    }

    #[test]
    fn array_round_trip_stable() {
        let s: BellStyle = serde_json::from_str("[\"window\",\"taskbar\"]").unwrap();
        assert!(s.window && s.taskbar);
        let back: BellStyle = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn bool_form_accepts_unified_true_false() {
        assert_eq!(
            serde_json::from_str::<BellStyle>("true").unwrap(),
            BellStyle::ALL
        );
        assert_eq!(
            serde_json::from_str::<BellStyle>("false").unwrap(),
            BellStyle::NONE
        );
    }
}
