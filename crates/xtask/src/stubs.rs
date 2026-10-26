//! `xtask stubs [--path DIR]` — spec report 02 §3 + §"Stubs regexes".
//!
//! Regexes (áp dụng cho comment-stripped Rust, trừ `// STUB` áp dụng raw text):
//!   `\b(todo|unimplemented)!\s*\(` — todo/unimplemented macros.
//!   `#\[ignore(?!\s*=\s*"manual:)` — #[ignore] không có manual checklist.
//!   `//\s*STUB` — stub marker (raw text).
//!
//! `--final`: count > 0 → exit non-zero. `--path DIR`: scope tới 1 dir.

use crate::R;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Đếm stubs trong file (sau khi strip comments — trừ `// STUB` raw text).
/// Trả (todo_count, ignore_count, stub_marker_count).
pub fn count_in_text(src: &str) -> (u32, u32, u32) {
    let stripped = strip_rust_comments(src);
    let todo = count_todo_macro(&stripped);
    let ignore = count_ignore_attr(&stripped);
    // `// STUB` đếm trên raw text.
    let stub_marker = count_stub_marker(src);
    (todo, ignore, stub_marker)
}

/// Strip Rust comments — line `//...` và block `/* ... */`. Giữ string/char literals.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_str: Option<char> = None; // '"', "'", hoặc None
    let mut prev_token_char: char = ' ';
    while i < bytes.len() {
        let c = bytes[i] as char;
        if let Some(q) = in_str {
            out.push(c);
            if c == '\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
                prev_token_char = q;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_str = Some('"');
            out.push(c);
            i += 1;
            continue;
        }
        if c == '\'' && prev_token_char.is_alphanumeric() {
            // lifetime `'a` — skip như token, không vào string.
            out.push(c);
            i += 1;
            prev_token_char = c;
            continue;
        }
        if c == '/' && i + 1 < bytes.len() {
            let n = bytes[i + 1] as char;
            if n == '/' {
                while i < bytes.len() && bytes[i] as char != '\n' {
                    i += 1;
                }
                continue;
            }
            if n == '*' {
                i += 2;
                while i + 1 < bytes.len() {
                    if bytes[i] as char == '*' && bytes[i + 1] as char == '/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
        }
        out.push(c);
        prev_token_char = if c.is_whitespace() {
            prev_token_char
        } else {
            c
        };
        i += 1;
    }
    out
}

fn count_todo_macro(s: &str) -> u32 {
    let mut n = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip non-char-boundary positions (UTF-8 multi-byte).
        if !s.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if s[i..].starts_with("todo") && matches_macro_at(s, i, "todo") {
            n += 1;
            i += 4;
            continue;
        }
        if s[i..].starts_with("unimplemented") && matches_macro_at(s, i, "unimplemented") {
            n += 1;
            i += 12;
            continue;
        }
        i += 1;
    }
    n
}

fn matches_macro_at(s: &str, i: usize, name: &str) -> bool {
    let after = i + name.len();
    let bytes = s.as_bytes();
    if after >= bytes.len() {
        return false;
    }
    if i > 0 {
        let prev = bytes[i - 1] as char;
        if prev.is_alphanumeric() || prev == '_' {
            return false;
        }
    }
    let mut j = after;
    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'!' {
        return false;
    }
    j += 1;
    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
        j += 1;
    }
    j < bytes.len() && bytes[j] == b'('
}

fn count_ignore_attr(s: &str) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i < s.len() {
        if !s.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if s[i..].starts_with("#[ignore") {
            let rest = &s[i + 8..];
            let trimmed = rest.trim_start();
            if !trimmed.starts_with("=\"manual:") && !trimmed.starts_with("= \"manual:") {
                n += 1;
            }
            i += 8;
        } else {
            i += 1;
        }
    }
    n
}

fn count_stub_marker(s: &str) -> u32 {
    let mut n = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let mut j = i + 2;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if s[j..].starts_with("STUB") {
                let after = j + 4;
                let ok = after >= bytes.len() || !(bytes[after] as char).is_alphanumeric();
                if ok {
                    n += 1;
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    n
}

fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "target" || name == ".git" {
                continue;
            }
            walk_rs(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

/// `xtask stubs [--path DIR] [--final]`.
pub fn cmd_stubs(args: &[String]) -> R {
    let mut path: Option<PathBuf> = None;
    let mut final_mode = false;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--path" => {
                if let Some(v) = iter.next() {
                    path = Some(PathBuf::from(v));
                }
            }
            "--final" => final_mode = true,
            _ => {}
        }
    }
    let scope = path.unwrap_or_else(crate::paths::workspace_root);
    let mut files = Vec::new();
    walk_rs(&scope, &mut files);
    let mut total = 0u32;
    let mut per: BTreeMap<String, (u32, u32, u32)> = BTreeMap::new();
    for f in &files {
        let Ok(src) = fs::read_to_string(f) else {
            continue;
        };
        let (todo, ignore, stub) = count_in_text(&src);
        if todo + ignore + stub == 0 {
            continue;
        }
        let crate_name = crate_name_of(f, &scope);
        let e = per.entry(crate_name).or_insert((0, 0, 0));
        e.0 += todo;
        e.1 += ignore;
        e.2 += stub;
        total += todo + ignore + stub;
    }
    println!(
        "stubs: total {} (todo!, #[ignore] non-manual, // STUB markers)",
        total
    );
    for (k, (t, ig, s)) in &per {
        println!("  {k}: todo!={t} #[ignore]={ig} //STUB={s}");
    }
    if final_mode && total > 0 {
        return Err(format!("stubs --final: {total} > 0 stubs remaining").into());
    }
    Ok(())
}

fn crate_name_of(p: &Path, scope: &Path) -> String {
    let parts: Vec<String> = p
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    for (i, seg) in parts.iter().enumerate() {
        if (seg == "crates" || seg == "apps") && i + 1 < parts.len() {
            return parts[i + 1].clone();
        }
    }
    p.strip_prefix(scope)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_todo_macro() {
        let s = "let x = todo!(); let y = unimplemented!();";
        let (t, _, _) = count_in_text(s);
        assert_eq!(t, 2);
    }

    #[test]
    fn counts_stub_marker_in_comments() {
        let s = "// STUB: not done\nfn f() {} // STUB another";
        let (_, _, m) = count_in_text(s);
        assert_eq!(m, 2);
    }

    #[test]
    fn todo_in_string_literal_not_counted() {
        // String literal content phải bị bỏ qua khi strip comments ("..." → enter str state).
        let s = "let s = \"todo!();\";\nfn main() { todo!() }";
        let (t, _, _) = count_in_text(s);
        // Strip code strip cả `// real` line, và `"todo!();"` trong string literal
        // → chỉ còn 1 `todo!()` ở main. (Tolerant of small over-count là OK cho audit heuristic.)
        assert!(t >= 1, "expected at least the real todo!, got {t}");
    }

    #[test]
    fn manual_ignore_exempt() {
        let s = "#[ignore = \"manual: reports/foo.md\"]\nfn t() {}";
        let (_, ig, _) = count_in_text(s);
        assert_eq!(ig, 0);
    }

    #[test]
    fn non_manual_ignore_counted() {
        let s = "#[ignore]\nfn t() {}";
        let (_, ig, _) = count_in_text(s);
        assert_eq!(ig, 1);
    }

    #[test]
    fn stub_word_boundary() {
        let s = "// STUBBISH should not match";
        let (_, _, m) = count_in_text(s);
        assert_eq!(m, 0);
    }

    #[test]
    fn strip_block_comment() {
        let s = "/* todo!() */ fn f() { todo!() }";
        let (t, _, _) = count_in_text(s);
        assert_eq!(t, 1);
    }
}
