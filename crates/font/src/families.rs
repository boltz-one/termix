//! Verbatim Rust port of `til::iterate_font_families`
//! (`windows/src/inc/til/string.h`).
//!
//! Splits a CSS-style comma-separated font-family list, honoring quoted
//! strings (`"..."` / `'...'`) and backslash escapes. Whitespace around
//! entries is trimmed and deduplicated, exactly like the C++ original — so
//! `"Cascadia Mono, Consolas, \"MS Gothic\""` yields the three family names
//! in order with no leading/trailing spaces.

/// Call `callback(family_name)` once per family in the list, in order.
pub fn iterate_font_families<F: FnMut(String)>(families: &str, mut callback: F) {
    let mut family = String::new();
    let mut escape = false;
    let mut delayed_space = false;
    let mut string_type: u8 = 0; // 0 = none, b'"' = double, b'\'' = single

    for ch in families.chars() {
        if !escape {
            match ch {
                ' ' => {
                    if string_type != 0 {
                        // Spaces are literal inside strings.
                    } else {
                        delayed_space = !family.is_empty();
                        continue;
                    }
                }
                '"' | '\'' => {
                    let c = ch as u32 as u8;
                    if string_type != 0 && string_type != c {
                        // Single inside double (and vice versa) is literal.
                    } else {
                        string_type = if string_type == c { 0 } else { c };
                        continue;
                    }
                }
                ',' => {
                    if string_type != 0 {
                        // Commas are literal inside strings.
                    } else {
                        if !family.is_empty() {
                            callback(std::mem::take(&mut family));
                            delayed_space = false;
                        }
                        continue;
                    }
                }
                '\\' => {
                    escape = true;
                    continue;
                }
                _ => {}
            }
        }

        if delayed_space {
            delayed_space = false;
            family.push(' ');
        }
        family.push(ch);
        escape = false;
    }

    // Trailing family (only if not mid-string).
    if string_type == 0 && !family.is_empty() {
        callback(family);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        iterate_font_families(s, |f| out.push(f));
        out
    }

    #[test]
    fn plain_comma_list() {
        assert_eq!(
            collect("Cascadia Mono, Consolas, MS Gothic"),
            ["Cascadia Mono", "Consolas", "MS Gothic"]
        );
    }

    #[test]
    fn whitespace_trimmed_and_dedup() {
        assert_eq!(
            collect("  Cascadia Mono  ,   Consolas  "),
            ["Cascadia Mono", "Consolas"]
        );
    }

    #[test]
    fn quoted_with_comma_inside() {
        assert_eq!(collect("\"My, Font\", Consolas"), ["My, Font", "Consolas"]);
        assert_eq!(collect("'My, Font', Consolas"), ["My, Font", "Consolas"]);
    }

    #[test]
    fn single_inside_double_is_literal() {
        assert_eq!(
            collect("\"It's fine\", Consolas"),
            ["It's fine", "Consolas"]
        );
    }

    #[test]
    fn escape_preserves_special_char() {
        assert_eq!(
            collect(r"Cascadia\, Mono, Consolas"),
            ["Cascadia, Mono", "Consolas"]
        );
    }

    #[test]
    fn empty_entries_skipped() {
        assert_eq!(collect(",,,"), [] as [String; 0]);
        assert_eq!(collect("A,,B"), ["A".to_string(), "B".into()]);
    }
}
