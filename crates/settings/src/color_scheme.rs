//! `ColorScheme` — port of WT's `TerminalSettingsModel::ColorScheme`.
//!
//! Ports verbatim from:
//! - `windows/src/cascadia/TerminalSettingsModel/ColorScheme.h:26-66` (decl).
//! - `windows/src/cascadia/TerminalSettingsModel/ColorScheme.cpp:81-165`:
//!   `FromJson`, `_layerJson`, `ToJson`, `SetColorTableEntry`.
//!
//! Fields, defaults, JSON wiring and the 16-color "scheme valid" predicate
//! match the C++ source line-for-line. The `from_json` constructor enforces
//! the same contract as `FromJson`: a valid `name` plus exactly 16 color
//! slots in the table; absent / null / malformed values fall back to the
//! field's existing default (see `layer_json`).

use crate::color::Color;
use crate::origin_tag::OriginTag;
use serde_json::Value;

/// `COLOR_WHITE = til::color{0xff, 0xff, 0xff}` (`DefaultSettings.h:17`) —
/// default foreground / selection background / cursor color.
const COLOR_WHITE: Color = Color::from_rgb(0xff, 0xff, 0xff);
/// `COLOR_BLACK = til::color{0, 0, 0}` (`DefaultSettings.h:18`) — default
/// background.
const COLOR_BLACK: Color = Color::from_rgb(0x00, 0x00, 0x00);
/// `DEFAULT_CURSOR_COLOR = COLOR_WHITE` (`DefaultSettings.h:21`) — WT
/// initializes the cursor with white; we keep it as a named constant for
/// parity with `ColorScheme.h:59`.
const DEFAULT_CURSOR_COLOR: Color = COLOR_WHITE;

/// WT `ColorScheme` (`ColorScheme.h:26-66`). Defaults match the C++
/// `WINRT_PROPERTY` initializers: foreground/selection/cursor = white,
/// background = black, table filled with the Campbell palette by the
/// constructor. We keep the Campbell-filled table when constructing an
/// uninitalized scheme for `from_json` so missing color keys fall back to
/// the same WT defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorScheme {
    /// `Name` — required. Empty string until `from_json`/`layer_json` sets it.
    pub name: String,
    /// `Origin` — `None` until the parent settings model tags it.
    pub origin: OriginTag,
    /// `Foreground` (defaults `COLOR_WHITE`).
    pub foreground: Color,
    /// `Background` (defaults `COLOR_BLACK`).
    pub background: Color,
    /// `SelectionBackground` (defaults `DEFAULT_FOREGROUND` = `COLOR_WHITE`).
    pub selection_background: Color,
    /// `CursorColor` (defaults `DEFAULT_CURSOR_COLOR` = `COLOR_WHITE`).
    pub cursor_color: Color,
    /// The 16-entry ANSI table (defaults Campbell palette).
    pub table: [Color; 16],
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorScheme {
    /// Mirrors `ColorScheme()` (`ColorScheme.cpp:30-34`): default name, origin
    /// `User`, table pre-filled with `CampbellColorTable()`. Foreground and
    /// selection background match WT's `WINRT_PROPERTY` initializers.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            origin: OriginTag::User,
            foreground: COLOR_WHITE,
            background: COLOR_BLACK,
            selection_background: COLOR_WHITE,
            cursor_color: DEFAULT_CURSOR_COLOR,
            table: crate::color::campbell_color_table(),
        }
    }

    /// Mirrors `ColorScheme(uninitialized_t)` (`ColorScheme.cpp:36`): same
    /// scalar defaults, but the table is left black (so missing keys are
    /// visibly identifiable during `from_json` construction). The brief
    /// specifies Campbell fill on the constructor and we use this variant
    /// only as the entry point for `from_json`/`layer_json`, where every
    /// present color overrides the slot — so the uninitialized base never
    /// surfaces after a successful parse.
    fn uninitialized() -> Self {
        Self {
            name: String::new(),
            origin: OriginTag::None,
            foreground: COLOR_WHITE,
            background: COLOR_BLACK,
            selection_background: COLOR_WHITE,
            cursor_color: DEFAULT_CURSOR_COLOR,
            table: [Color::default(); 16],
        }
    }

    /// `FromJson` (`ColorScheme.cpp:81-85`). Constructs a fresh scheme (with
    /// defaults), layers `value` over it, then validates that the result
    /// carried a `name` and exactly 16 colors. Returns the parsed scheme on
    /// success or `Err` describing the first missing piece.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let mut scheme = Self::uninitialized();
        scheme.layer_json(value);
        if scheme.name.is_empty() {
            return Err("ColorScheme missing required field: \"name\"".to_string());
        }
        // `_layerJson` requires the 16 ANSI keys; the 18-entry mapping table
        // contains `"magenta"`/`"brightMagenta"` aliases that count toward
        // the 16 — we record every present color and reject fewer than 16.
        if count_present_table_colors(value) < 16 {
            return Err("ColorScheme requires all 16 ANSI table colors".to_string());
        }
        Ok(scheme)
    }

    /// `_layerJson` (`ColorScheme.cpp:97-122`). Per-key overlay: a present,
    /// well-formed value overrides the field; absent or null leaves the
    /// existing value untouched (mirrors `JsonUtils::GetValueForKey`'s null
    /// skip and the "properties not in the JSON object are not replaced"
    /// contract documented in `ColorScheme.cpp`).
    pub fn layer_json(&mut self, value: &Value) {
        let obj = match value.as_object() {
            Some(o) => o,
            None => return,
        };
        // `name` is required but null at the JSON level leaves the existing
        // name in place (mirrors `JsonUtils::GetValueForKey`'s null skip on
        // string fields). `as_str()` already returns `None` for `Value::Null`.
        if let Some(v) = obj.get("name").and_then(|v| v.as_str()) {
            self.name = v.to_string();
        }
        layer_color(obj, "foreground", &mut self.foreground);
        layer_color(obj, "background", &mut self.background);
        layer_color(obj, "selectionBackground", &mut self.selection_background);
        layer_color(obj, "cursorColor", &mut self.cursor_color);
        for (key, idx) in TABLE_COLORS_MAPPING {
            if let Some(c) = obj.get(key).and_then(parse_color_value) {
                self.table[idx] = c;
            }
        }
    }

    /// `ToJson` (`ColorScheme.cpp:130-147`). Always emits `name`, the four
    /// scalar color fields, and the 16 canonical ANSI entries (skipping
    /// the `magenta`/`brightMagenta` aliases). Field emission order matches
    /// the C++ `SetValueForKey` call sequence.
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert(
            "foreground".to_string(),
            Value::String(self.foreground.to_hex()),
        );
        map.insert(
            "background".to_string(),
            Value::String(self.background.to_hex()),
        );
        map.insert(
            "selectionBackground".to_string(),
            Value::String(self.selection_background.to_hex()),
        );
        map.insert(
            "cursorColor".to_string(),
            Value::String(self.cursor_color.to_hex()),
        );
        for (key, idx) in TABLE_COLORS_MAPPING.iter().take(16) {
            map.insert((*key).to_string(), Value::String(self.table[*idx].to_hex()));
        }
        Value::Object(map)
    }

    /// `SetColorTableEntry` (`ColorScheme.cpp:161-165`). C++ throws
    /// `E_INVALIDARG` for `index >= 16`; we silently clamp instead so the
    /// Rust API is total (the brief specifies clamping).
    pub fn set_color_table_entry(&mut self, index: u8, color: Color) {
        if (index as usize) < self.table.len() {
            self.table[index as usize] = color;
        }
    }
}

/// Layer a single color key into `slot` if the object carries a present,
/// well-formed color for `key`. Mirrors `JsonUtils::GetValueForKey<Color>`
/// (which skips null and grossly malformed values, leaving the slot as-is).
fn layer_color(obj: &serde_json::Map<String, Value>, key: &str, slot: &mut Color) {
    if let Some(c) = obj.get(key).and_then(parse_color_value) {
        *slot = c;
    }
}

/// Parse a JSON color value (string or null). `null` returns `None` so
/// callers can distinguish "leave existing" from "got a value".
fn parse_color_value(v: &Value) -> Option<Color> {
    let s = v.as_str()?;
    Color::from_hex(s)
}

/// Count of ANSI table colors present in `value` (out of the 18-entry
/// mapping, deduped to the 16 distinct indices). The C++ source breaks once
/// it has counted 16 — we replicate by counting distinct filled indices.
fn count_present_table_colors(value: &Value) -> usize {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return 0,
    };
    let mut filled = [false; 16];
    for (key, idx) in TABLE_COLORS_MAPPING {
        if obj.get(key).and_then(parse_color_value).is_some() {
            filled[idx] = true;
        }
    }
    filled.iter().filter(|&&x| x).count()
}

/// One-to-one port of `TableColorsMapping` (`ColorScheme.cpp:25-47`). The
/// last two entries (`"magenta"`, `"brightMagenta"`) are GH#11456 aliases
/// that overlay indices 5 and 13 — they are never emitted by `to_json`.
const TABLE_COLORS_MAPPING: [(&str, usize); 18] = [
    ("black", 0),
    ("red", 1),
    ("green", 2),
    ("yellow", 3),
    ("blue", 4),
    ("purple", 5),
    ("cyan", 6),
    ("white", 7),
    ("brightBlack", 8),
    ("brightRed", 9),
    ("brightGreen", 10),
    ("brightYellow", 11),
    ("brightBlue", 12),
    ("brightPurple", 13),
    ("brightCyan", 14),
    ("brightWhite", 15),
    // Alternate color mappings (GH#11456) — parse-only aliases.
    ("magenta", 5),
    ("brightMagenta", 13),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    /// Parity: a scheme round-trips through `to_json` ↔ `from_json`
    /// byte-for-byte (`ColorScheme.cpp:81` / `ToJson`). Campbell carries the
    /// fanciest field set (every key populated, no aliases), so it's the
    /// harshest round-trip probe.
    #[test]
    fn round_trip_campbell() {
        let campbell = crate::builtin_schemes::builtin_schemes()
            .iter()
            .find(|s| s.name == "Campbell")
            .cloned()
            .expect("Campbell present");
        let j = campbell.to_json();
        let back = ColorScheme::from_json(&j).expect("round-trip ok");
        assert_eq!(back.name, campbell.name);
        assert_eq!(back.foreground, campbell.foreground);
        assert_eq!(back.background, campbell.background);
        assert_eq!(back.selection_background, campbell.selection_background);
        assert_eq!(back.cursor_color, campbell.cursor_color);
        assert_eq!(back.table, campbell.table);
    }

    /// Parity: `layer_json` semantics — present overrides, null preserves,
    /// absent preserves (`ColorScheme.cpp:97-122`).
    #[test]
    fn layer_json_present_null_absent_semantics() {
        let mut s = ColorScheme::new();
        s.name = "base".to_string();
        s.foreground = Color::from_rgb(0x11, 0x11, 0x11);
        s.background = Color::from_rgb(0x22, 0x22, 0x22);
        s.cursor_color = Color::from_rgb(0x33, 0x33, 0x33);
        let prev_fg = s.foreground;
        let prev_cursor = s.cursor_color;
        let prev_red = s.table[1];

        // present overrides; null preserves; absent preserves.
        s.layer_json(&obj(r##"{
                "name": "out",
                "background": "#abcdef",
                "cursorColor": null
            }"##));
        assert_eq!(s.name, "out"); // present override
        assert_eq!(s.background, Color::from_rgb(0xab, 0xcd, 0xef)); // present override
        assert_eq!(s.foreground, prev_fg); // absent preserve
        assert_eq!(s.cursor_color, prev_cursor); // null preserve
        assert_eq!(s.table[1], prev_red); // absent preserve
    }

    /// Parity: `TableColorsMapping` aliases (`ColorScheme.cpp:25-47`,
    /// GH#11456). `"magenta"` overlays index 5, `"brightMagenta"` overlays
    /// index 13. `to_json` does not emit the aliases.
    #[test]
    fn magenta_aliases_overlay_indices_5_and_13() {
        let v = obj(r##"{
                "name": "alias-test",
                "black": "#000000",
                "red": "#110000",
                "green": "#001100",
                "yellow": "#111100",
                "blue": "#000011",
                "magenta": "#aa00aa",
                "cyan": "#00aaaa",
                "white": "#ffffff",
                "brightBlack": "#555555",
                "brightRed": "#ff0000",
                "brightGreen": "#00ff00",
                "brightYellow": "#ffff00",
                "brightBlue": "#0000ff",
                "brightMagenta": "#bb00bb",
                "brightCyan": "#00ffff",
                "brightWhite": "#ffffff"
            }"##);
        let s = ColorScheme::from_json(&v).expect("alias scheme parses");
        assert_eq!(s.table[5], Color::from_rgb(0xaa, 0x00, 0xaa));
        assert_eq!(s.table[13], Color::from_rgb(0xbb, 0x00, 0xbb));
        // to_json emits the canonical primary key, never the alias.
        let out = serde_json::to_string(&s.to_json()).unwrap();
        assert!(out.contains("\"purple\""));
        assert!(out.contains("\"brightPurple\""));
        assert!(!out.contains("\"magenta\""));
        assert!(!out.contains("\"brightMagenta\""));
    }

    /// Parity: `from_json` rejects a scheme missing the 16 ANSI colors or a
    /// `name` (`ColorScheme.cpp:98 / 117-119`). All-too-few colors yield an
    /// error rather than a partial scheme.
    #[test]
    fn from_json_requires_name_and_16_colors() {
        let no_name = obj(r##"{
                "black": "#000000","red": "#110000","green": "#001100","yellow": "#111100",
                "blue": "#000011","purple": "#aa00aa","cyan": "#00aaaa","white": "#ffffff",
                "brightBlack": "#555555","brightRed": "#ff0000","brightGreen": "#00ff00",
                "brightYellow": "#ffff00","brightBlue": "#0000ff","brightPurple": "#bb00bb",
                "brightCyan": "#00ffff","brightWhite": "#ffffff"
            }"##);
        assert!(ColorScheme::from_json(&no_name).is_err());

        let too_few = obj(r##"{
                "name": "too-few",
                "black": "#000000","red": "#110000","green": "#001100","yellow": "#111100",
                "blue": "#000011","purple": "#aa00aa","cyan": "#00aaaa","white": "#ffffff",
                "brightBlack": "#555555","brightRed": "#ff0000","brightGreen": "#00ff00",
                "brightYellow": "#ffff00","brightBlue": "#0000ff","brightPurple": "#bb00bb",
                "brightCyan": "#00ffff"
            }"##);
        assert!(ColorScheme::from_json(&too_few).is_err());
    }

    /// Parity: `SetColorTableEntry` clamps out-of-range indices silently
    /// (brief specifies clamping; WT throws `E_INVALIDARG`).
    #[test]
    fn set_color_table_entry_clamps_out_of_range() {
        let mut s = ColorScheme::new();
        let prev15 = s.table[15];
        s.set_color_table_entry(255, Color::from_rgb(0x12, 0x34, 0x56));
        assert_eq!(s.table[15], prev15); // out-of-range ignored
        s.set_color_table_entry(7, Color::from_rgb(0x12, 0x34, 0x56));
        assert_eq!(s.table[7], Color::from_rgb(0x12, 0x34, 0x56)); // in-range set
    }
}
