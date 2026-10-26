//! `settings_loader` — port mỏng của `CascadiaSettings` loader
//! (`TerminalApp/CascadiaSettings.cpp`).
//!
//! Phase 1: parse inbox defaults (`INBOX_SETTINGS_JSON`, embedded by
//! `settings::defaults`). JSONC comments strip bằng helper thủ công
//! `strip_jsonc_comments` — `serde_json::Deserializer` không có `disable_comments`
//! (đó là feature cũ `serde_jsonrc`, không còn). WT `JsonUtils.h::StripComments`
//! cũng không tồn tại — WT dựa vào `Microsoft::Terminal::Settings::Model::JsonUtils`'s
//! `DisableComments` flag từ underlying `cppwinrt` JSON parser, không port được.
//!
//! Helper strip `//...` line + `/* ... */` block comments, bỏ qua trong string
//! literal (escape `\"` honored).
//!
//! Phase 2: merge user `settings.json` từ
//! `%LOCALAPPDATA%\Packages\<pkg>\LocalState\settings.json`.

use gpui::App;
use settings::{GlobalAppSettings, Profile, DEFAULT_WINDOWS_POWERSHELL_GUID, INBOX_SETTINGS_JSON};

/// `CascadiaSettings` — port mỏng của WT `CascadiaSettings.h:36-105`.
/// Phase 1 chỉ nắm profiles + default guid + globals.
#[derive(Clone, Debug)]
pub struct CascadiaSettings {
    /// Toàn bộ profiles inbox + (phase 2) user overrides.
    pub profiles: Vec<Profile>,
    /// `defaults.json:3` — `defaultProfile` GUID string. Phase 1 so khớp
    /// PowerShell inbox GUID `{61c54bbd-c2c6-5271-96e7-009a87ff44bf}`.
    pub default_profile_guid: uuid::Uuid,
    /// Top-level `GlobalAppSettings` (`defaults.json:1-35`).
    pub globals: GlobalAppSettings,
}

/// Load Cascadia settings. Phase 1: inbox defaults only.
///
/// `cx` kept cho phase 2 (filesystem read via GPUI asset source / async I/O).
pub fn load_settings(_cx: &mut App) -> anyhow::Result<CascadiaSettings> {
    let globals = parse_globals_from_jsonc(INBOX_SETTINGS_JSON)?;
    let profiles = parse_profiles_from_jsonc(INBOX_SETTINGS_JSON)?;
    let default_guid = parse_default_profile(INBOX_SETTINGS_JSON, &globals)?;
    Ok(CascadiaSettings {
        profiles,
        default_profile_guid: default_guid,
        globals,
    })
}

/// Strip JSONC comments rồi deserialize `GlobalAppSettings`.
fn parse_globals_from_jsonc(s: &str) -> anyhow::Result<GlobalAppSettings> {
    let stripped = strip_jsonc_comments(s);
    let g: GlobalAppSettings = serde_json::from_str(&stripped)?;
    Ok(g)
}

/// Strip JSONC, lấy mảng `profiles` (mỗi object → `Profile::from_json`).
fn parse_profiles_from_jsonc(s: &str) -> anyhow::Result<Vec<Profile>> {
    let stripped = strip_jsonc_comments(s);
    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(default)]
        profiles: Vec<serde_json::Value>,
    }
    let w: Wrapper = serde_json::from_str(&stripped)?;
    let out: Vec<Profile> = w
        .profiles
        .into_iter()
        .filter_map(|v| match Profile::from_json(&v) {
            Ok(p) => Some(p),
            Err(e) => {
                log::warn!("Bỏ qua profile lỗi parse: {e}");
                None
            }
        })
        .filter(|p| !p.hidden && !p.deleted)
        .collect();
    Ok(out)
}

/// Lấy `defaultProfile` GUID string → `Uuid`. Re-dùng `globals.default_profile`
/// để tránh parse 2 lần. Braced GUID `{...}` → strip braces.
fn parse_default_profile(s: &str, globals: &GlobalAppSettings) -> anyhow::Result<uuid::Uuid> {
    let raw = if globals.default_profile.is_empty() {
        // Fallback: scan `defaultProfile` trực tiếp (defensive).
        let stripped = strip_jsonc_comments(s);
        #[derive(serde::Deserialize)]
        struct W {
            #[serde(rename = "defaultProfile")]
            default_profile: String,
        }
        let w: W = serde_json::from_str(&stripped)?;
        w.default_profile
    } else {
        globals.default_profile.clone()
    };
    let trimmed = raw.trim_matches(|c: char| c == '{' || c == '}');
    Ok(uuid::Uuid::parse_str(trimmed)?)
}

/// Strip JSONC comments (`//` line + `/* */` block) + trailing comma trước
/// `}`/`]`, không chạm trong string literal. Char-walker: state `InStr`,
/// `InLineComment`, `InBlockComment`. Honors `\"` escape. WT `defaults.json`
/// có trailing comma (JSONC-legal, JSON-illegal) nên phải strip luôn.
///
/// Trailing comma strip: khi gặp `}`/`]` (ngoài string), scan quay lại `out`
/// skip whitespace, nếu gặp `,` pop nó. Đơn giản và không错过 nested.
pub fn strip_jsonc_comments(s: &str) -> String {
    let bytes: Vec<char> = s.chars().collect();
    let n = bytes.len();
    let mut out: Vec<char> = Vec::with_capacity(n);
    let mut i = 0;
    let mut in_str = false;
    let mut in_line = false;
    let mut in_block = false;
    while i < n {
        let c = bytes[i];
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push(c);
            }
            i += 1;
            continue;
        }
        if in_block {
            if c == '*' && i + 1 < n && bytes[i + 1] == '/' {
                in_block = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_str {
            out.push(c);
            if c == '\\' && i + 1 < n {
                // escape: keep next char literal
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        // not in comment/string — look for comment start
        if c == '/' && i + 1 < n {
            match bytes[i + 1] {
                '/' => {
                    in_line = true;
                    i += 2;
                    continue;
                }
                '*' => {
                    in_block = true;
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        if c == '"' {
            in_str = true;
            out.push(c);
            i += 1;
            continue;
        }
        // Trailing comma: c là `}` hoặc `]` → if last non-ws out char là `,`,
        // pop nó (và pop whitespace được push sau). Mark-redo simple: leave
        // whitespace, only pop the comma.
        if c == '}' || c == ']' {
            // pop whitespace+newlines out gần-final, then check `,`
            let mut k = out.len();
            while k > 0 && out[k - 1].is_whitespace() {
                k -= 1;
            }
            if k > 0 && out[k - 1] == ',' {
                out.truncate(k - 1);
            }
        }
        out.push(c);
        i += 1;
    }
    out.into_iter().collect()
}

/// Guid inbox PowerShell — parity assertion helper.
pub fn default_powershell_guid() -> uuid::Uuid {
    let s = DEFAULT_WINDOWS_POWERSHELL_GUID.trim_matches(|c: char| c == '{' || c == '}');
    uuid::Uuid::parse_str(s).expect("DEFAULT_WINDOWS_POWERSHELL_GUID phải parse được")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_guid_matches_powershell_inbox() {
        let g = default_powershell_guid();
        assert_eq!(
            g.to_string(),
            "61c54bbd-c2c6-5271-96e7-009a87ff44bf",
            "PowerShell inbox GUID (`defaults.json:3`)",
        );
    }

    #[test]
    fn parse_inbox_succeeds() {
        // Tránh depend vào App; parse inbox trực tiếp.
        let globals = parse_globals_from_jsonc(INBOX_SETTINGS_JSON).unwrap();
        let profiles = parse_profiles_from_jsonc(INBOX_SETTINGS_JSON).unwrap();
        let guid = parse_default_profile(INBOX_SETTINGS_JSON, &globals).unwrap();
        assert_eq!(guid, default_powershell_guid());
        assert_eq!(globals.default_profile, DEFAULT_WINDOWS_POWERSHELL_GUID);
        // Inbox phải có ít nhất PowerShell + cmd profile.
        assert!(profiles.len() >= 2);
        assert!(profiles
            .iter()
            .any(|p| p.guid == DEFAULT_WINDOWS_POWERSHELL_GUID));
    }

    #[test]
    fn strip_line_comment_outside_string() {
        let s = r#"{ "a": 1 // hello
      , "b": 2 }"#;
        let s2 = strip_jsonc_comments(s);
        assert!(!s2.contains("// hello"));
        assert!(s2.contains("\"a\": 1"));
        // parse-roundtrip
        #[derive(serde::Deserialize)]
        struct V {
            a: i32,
            b: i32,
        }
        let v: V = serde_json::from_str(&s2).unwrap();
        assert_eq!((v.a, v.b), (1, 2));
    }

    #[test]
    fn strip_block_comment_outside_string() {
        let s = r#"{ /* block */ "a": 1 }"#;
        let s2 = strip_jsonc_comments(s);
        assert!(!s2.contains("/*"));
        assert!(!s2.contains("*/"));
        #[derive(serde::Deserialize)]
        struct V {
            a: i32,
        }
        let v: V = serde_json::from_str(&s2).unwrap();
        assert_eq!(v.a, 1);
    }

    #[test]
    fn strip_does_not_touch_string_with_slash_slash() {
        // `//` trong string literal phải giữ nguyên.
        let s = r#"{ "url": "https://example.com" }"#;
        let s2 = strip_jsonc_comments(s);
        assert_eq!(s, s2);
        assert!(s2.contains("https://"));
    }

    #[test]
    fn strip_handles_escaped_quote() {
        let s = r#"{ "x": "a\"b // not a comment" , "y": 5 }"#;
        let s2 = strip_jsonc_comments(s);
        assert!(s2.contains("// not a comment"));
        #[derive(serde::Deserialize)]
        struct V {
            x: String,
            y: i32,
        }
        let v: V = serde_json::from_str(&s2).unwrap();
        assert_eq!(v.x, "a\"b // not a comment");
        assert_eq!(v.y, 5);
    }
}
