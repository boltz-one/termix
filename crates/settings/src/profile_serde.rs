//! `Profile` serde ports: `FromJson` / `LayerJson` / `ToJson`
//! (`Profile.cpp:151,170,327`). Reopens the `impl Profile` block defined in
//! [`crate::profile`].

use crate::profile::Profile;

/// JSON keys whose Rust fields are `Option<T>`. A JSON `null` on one of these
/// clears the value (sets `None`); on any other key, `null` is dropped so the
/// existing value persists — mirroring C++ `GetValueForKey`'s "absent or
/// unparseable → leave untouched" contract for non-nullable settings.
const NULLABLE_JSON_KEYS: &[&str] = &[
    "source",
    "connectionType",
    "icon",
    "tabTitle",
    "tabColor",
    "suppressApplicationTitle",
    "startingDirectory",
    "updates",
    "unfocusedAppearance",
    "answerbackMessage",
    "bellSound",
    "environment",
];

impl Profile {
    /// `Profile::FromJson` (`Profile.cpp:151-156`) — deserialize a JSON object
    /// into a fresh profile.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())
    }

    /// `Profile::ToJson` (`Profile.cpp:327-365`) — serialize this profile,
    /// skipping `None` and default fields (the C++ emitter skips empty
    /// optionals via `SetValueForKey`). The default appearance and unfocused
    /// appearance are skipped when they match the Rust default (Open Q).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// `Profile::LayerJson` (`Profile.cpp:170-226`) — overlay a partial JSON
    /// object on top of this profile.
    ///
    /// Semantics (mirror WT's `GetValueForKey`):
    /// - absent key → leave existing value unchanged;
    /// - `null` on a nullable (`Option`) field → set to `None`;
    /// - `null` on a non-nullable field → leave existing (C++ does NOT reset
    ///   to default on null for non-nullable settings);
    /// - present value → override.
    pub fn layer_json(&mut self, json: &serde_json::Value) {
        // Take the current profile's JSON object, overlay the patch's
        // present keys (including explicit nulls where the field is nullable),
        // then re-parse. Nulls on non-nullable fields are dropped before
        // re-parse so the existing value survives.
        let mut base = match self.to_json() {
            serde_json::Value::Object(m) => m.clone(),
            _ => serde_json::Map::new(),
        };
        if let serde_json::Value::Object(patch) = json {
            for (key, value) in patch {
                if value.is_null() && !NULLABLE_JSON_KEYS.contains(&key.as_str()) {
                    // Non-nullable field asked to clear — preserve existing.
                    continue;
                }
                base.insert(key.clone(), value.clone());
            }
        }
        let patched = serde_json::Value::Object(base);
        if let Ok(merged) = serde_json::from_value::<Profile>(patched) {
            *self = merged;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::{DEFAULT_COMMAND_PROMPT_GUID, DEFAULT_WINDOWS_POWERSHELL_GUID};
    use crate::profile_enums::{CloseOnExitMode, ScrollbarState};
    use serde_json::json;

    /// Round-trip: `Profile::from_json(p.to_json())` deep-equals `p`.
    #[test]
    fn default_powershell_round_trips_json() {
        let p = Profile::default_powershell();
        let back = Profile::from_json(&p.to_json()).expect("round-trip parse");
        assert_eq!(back.guid, DEFAULT_WINDOWS_POWERSHELL_GUID);
        assert_eq!(back.name, "Windows PowerShell");
        assert_eq!(back.commandline, p.commandline);
        assert_eq!(back.history_size, p.history_size);
        assert_eq!(back.close_on_exit, CloseOnExitMode::Automatic);
        assert_eq!(back.scroll_state, ScrollbarState::Visible);
        assert!(back.snap_on_input);
        assert!(back.alt_gr_aliasing);
        assert!(!back.elevate);
        assert!(back.allow_kitty_keyboard_mode);
        assert!(back.allow_vt_clipboard_write);
        assert!(back.reload_environment_variables);
        assert_eq!(back.drag_drop_delimiter, " ");
        assert_eq!(back.padding, "8, 8, 8, 8");
        assert_eq!(back.bell_style, p.bell_style);
        assert_eq!(back.icon, p.icon);
        assert_eq!(back.starting_directory, p.starting_directory);
    }

    /// `default_cmd` GUID is preserved by the round-trip.
    #[test]
    fn default_cmd_round_trip_guid() {
        let p = Profile::default_cmd();
        let back = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!(back.guid, DEFAULT_COMMAND_PROMPT_GUID);
    }

    /// `layer_json` present overrides existing.
    #[test]
    fn layer_present_overrides() {
        let mut p = Profile::default_powershell();
        assert_eq!(p.history_size, 9001);
        p.layer_json(&json!({ "historySize": 1000 }));
        assert_eq!(p.history_size, 1000);
        // Show another field too.
        p.layer_json(&json!({ "elevate": true }));
        assert!(p.elevate);
    }

    /// `layer_json` absent keys preserve the existing value.
    #[test]
    fn layer_absent_preserves() {
        let mut p = Profile::default_powershell();
        let before = p.history_size;
        p.layer_json(&json!({ "elevate": true }));
        assert_eq!(p.history_size, before);
    }

    /// `layer_json` null on a nullable field clears it (`None`).
    #[test]
    fn layer_null_clears_nullable() {
        let mut p = Profile::default_powershell();
        assert!(p.icon.is_some());
        p.layer_json(&json!({ "icon": null }));
        assert!(p.icon.is_none(), "null on nullable should clear to None");
        // starting_directory is also nullable.
        p.layer_json(&json!({ "startingDirectory": null }));
        assert!(p.starting_directory.is_none());
    }

    /// `layer_json` null on a non-nullable field leaves existing untouched
    /// (C++ does not reset non-nullable settings on null).
    #[test]
    fn layer_null_preserves_non_nullable() {
        let mut p = Profile::default_powershell();
        let before = p.history_size;
        p.layer_json(&json!({ "historySize": null }));
        assert_eq!(
            p.history_size, before,
            "null on non-nullable should preserve"
        );
        p.layer_json(&json!({ "elevate": null }));
        assert!(!p.elevate, "elevate stays at its previous value");
    }

    /// BellStyle round-trips as a single string and as an array on layer.
    #[test]
    fn layer_bell_style_accepts_string_and_array() {
        let mut p = Profile::default_powershell();
        // String form.
        p.layer_json(&json!({ "bellStyle": "none" }));
        assert_eq!(p.bell_style, crate::bell_style::BellStyle::NONE);
        // Visual deserializes to {window, taskbar}.
        p.layer_json(&json!({ "bellStyle": "visual" }));
        assert!(p.bell_style.window && p.bell_style.taskbar);
        // Array form.
        p.layer_json(&json!({ "bellStyle": ["audible", "notification"] }));
        assert!(p.bell_style.audible && p.bell_style.notification);
        assert!(!p.bell_style.window);
        assert!(!p.bell_style.taskbar);
        // Round-trips through to_json / from_json.
        let back = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!(back.bell_style, p.bell_style);
    }

    /// `to_json` skips `None` and default-valued fields.
    #[test]
    fn to_json_skips_none_and_defaults() {
        let p = Profile::default_powershell();
        let v = p.to_json();
        assert!(v.is_object());
        let obj = v.as_object().unwrap();
        // nullable unset fields are skipped.
        assert!(!obj.contains_key("source"));
        assert!(!obj.contains_key("tabColor"));
        assert!(!obj.contains_key("updates"));
        // default-false bools are still emitted (C++ emits defaults of
        // primitive fields, but skips default appearance). We don't assert
        // positivity here — just the absence of `None`-gated fields.
        // `elevate` defaults false and is still in the output (matches C++).
        assert!(obj.contains_key("elevate"));
    }
}
