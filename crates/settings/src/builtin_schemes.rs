//! Built-in color schemes — 16 schemes WT ships in `defaults.json`.
//!
//! Mirrors WT's in-box scheme list
//! (`windows/src/cascadia/TerminalSettingsModel/defaults.json:83-461`).
//! We reuse the embedded [`INBOX_SETTINGS_JSON`](crate::INBOX_SETTINGS_JSON)
//! (which is also JSONC — `//` line comments at the head) and strip those
//! comments before `serde_json` parses. Parsed schemes are tagged
//! [`OriginTag::InBox`](crate::origin_tag::OriginTag) to match WT's
//! `OriginTag::InBox` mark on the in-box defaults.

use crate::color_scheme::ColorScheme;
use crate::defaults::INBOX_SETTINGS_JSON;
use crate::origin_tag::OriginTag;
use serde_json::Value;
use std::sync::OnceLock;

/// Expected scheme order — `defaults.json:83-461`. We assert this verbatim
/// in tests so reordering upstream is caught loudly.
pub const BUILTIN_SCHEME_NAMES: [&str; 16] = [
    "Dimidium",
    "Ottosson",
    "Campbell",
    "Campbell Powershell",
    "Vintage",
    "One Half Dark",
    "One Half Light",
    "Solarized Dark",
    "Solarized Light",
    "Tango Dark",
    "Tango Light",
    "Dark+",
    "VSCode Dark Modern",
    "VSCode Light Modern",
    "CGA",
    "IBM 5153",
];

/// Returns all 16 in-box color schemes parsed from the WT `defaults.json`
/// `schemes` array. Returned order matches `BUILTIN_SCHEME_NAMES` so the
/// caller can index by position; look-ups should still key on `name`.
pub fn builtin_schemes() -> Vec<ColorScheme> {
    static CACHE: OnceLock<Vec<ColorScheme>> = OnceLock::new();
    CACHE
        .get_or_init(|| parse_schemes(&strip_jsonc_comments(INBOX_SETTINGS_JSON)))
        .clone()
}

fn parse_schemes(jsonc: &str) -> Vec<ColorScheme> {
    let root: Value = serde_json::from_str(jsonc).expect("defaults.json parses");
    let arr = root
        .get("schemes")
        .and_then(|v| v.as_array())
        .expect("defaults.json has a schemes array");
    arr.iter()
        .map(|v| {
            let mut s = ColorScheme::from_json(v).expect("in-box scheme is well-formed");
            // All in-box schemes are tagged InBox by WT's settings loader.
            s.origin = OriginTag::InBox;
            s
        })
        .collect()
}

/// Strip `//` line and `/* */` block comments and trailing commas from
/// JSONC text so `serde_json` (which has no JSONC support) can parse it.
/// JSONC permits comments and a trailing comma before `]` / `}`; we drop
/// both. We leave `//` and `,` inside string literals untouched. A full
/// `json5` crate would be overkill for a single strip pass over a static
/// input, so the loop below is a minimal state machine.
fn strip_jsonc_comments(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let mut in_str = false;
    let mut str_escape = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            out.push(b as char);
            if str_escape {
                str_escape = false;
            } else if b == b'\\' {
                str_escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            in_str = true;
            out.push('"');
            i += 1;
            continue;
        }
        // `//` line comment.
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // `/* */` block comment.
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        // Trailing `,` before `]` / `}` (allowing whitespace between): drop
        // the comma and let the loop emit the structural char normally.
        if b == b',' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b']' || bytes[j] == b'}') {
                i += 1; // skip the comma
                continue;
            }
        }
        out.push(b as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::campbell_color_table;

    /// Returns exactly 16 schemes with the expected names in order.
    #[test]
    fn builtin_schemes_returns_16_named() {
        let schemes = builtin_schemes();
        assert_eq!(schemes.len(), 16, "exactly 16 in-box schemes");
        let names: Vec<&str> = schemes.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, BUILTIN_SCHEME_NAMES.to_vec());
        for s in &schemes {
            assert_eq!(s.origin, OriginTag::InBox);
        }
    }

    /// Parity: WT's `"Campbell"` scheme matches `CampbellColorTable()`
    /// (`windows/src/types/colorTable.cpp:11`) byte-for-byte across all 16
    /// table entries.
    #[test]
    fn campbell_builtin_matches_campbell_color_table() {
        let schemes = builtin_schemes();
        let campbell = schemes
            .iter()
            .find(|s| s.name == "Campbell")
            .expect("Campbell present");
        let tbl = campbell_color_table();
        assert_eq!(campbell.table, tbl, "Campbell table matches colorTable.cpp");
    }

    /// Parity: a parsed scheme round-trips through `to_json` ↔ `from_json`
    /// (`ColorScheme.cpp:81` / `ToJson`). We probe the schemes whose JSON is
    /// most likely to diverge — Dimidium (writes every scalar) — alongside
    /// the Campbell probe in `color_scheme::tests`.
    #[test]
    fn dimidium_round_trips() {
        let dimidium = builtin_schemes()
            .iter()
            .find(|s| s.name == "Dimidium")
            .cloned()
            .expect("Dimidium present");
        let j = dimidium.to_json();
        let back = ColorScheme::from_json(&j).expect("round-trip ok");
        assert_eq!(back.name, dimidium.name);
        assert_eq!(back.foreground, dimidium.foreground);
        assert_eq!(back.background, dimidium.background);
        assert_eq!(back.selection_background, dimidium.selection_background);
        assert_eq!(back.cursor_color, dimidium.cursor_color);
        assert_eq!(back.table, dimidium.table);
    }

    /// Parity: legacy `"magenta"`/`"brightMagenta"` aliases count toward the
    /// "16 colors present" predicate and overlay indices 5 / 13
    /// (`ColorScheme.cpp:25-47`, GH#11456). The briefly-shipped `CGA`
    /// schemes omit `cursorColor`/`selectionBackground` on some channels —
    /// confirm all 16 in-box schemes pass the validity predicate.
    #[test]
    fn all_builtin_schemes_parse_with_16_colors() {
        for s in builtin_schemes() {
            // If `to_json` round-trips back through `from_json`.
            let j = s.to_json();
            let back = ColorScheme::from_json(&j).expect("round-trip ok");
            assert_eq!(back.table, s.table, "{} round-trips table", s.name);
        }
    }
}
