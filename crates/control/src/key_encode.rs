//! Keystroke → VT bytes for the shell. Port of `TerminalInput::HandleKey`
//! (`windows/src/terminal/input/terminalInput.cpp`: `_encodeRegular`,
//! `_formatEncodingHelper`, `_formatFallback`, `_makeCtrlChar`), regular
//! (non-Kitty, non-win32) ANSI mode.
//!
//! Not ported: Kitty keyboard protocol, win32-input-mode, VT52, SendC1 and the
//! keypad (GPUI does not tell numpad keys from the main block).

use gpui::Keystroke;

/// `TerminalInput::Mode` bits that change key translation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyModes {
    /// DECCKM: unmodified cursor keys use SS3.
    pub cursor_key: bool,
    /// DECBKM: Backspace sends BS (Ctrl inverts), default DEL.
    pub backarrow: bool,
    /// LNM: Enter sends CR LF.
    pub line_feed: bool,
}

const CSI_SHIFT: u32 = 1;
const CSI_ALT: u32 = 2;
const CSI_CTRL: u32 = 4;

/// `_makeCtrlChar`.
fn ctrl_char(ch: char) -> char {
    let c = ch as u32;
    let out = match ch {
        '@'..='~' => c & 0x1f,
        ' ' => 0x00,
        '/' => 0x1f,
        '?' => 0x7f,
        '2'..='8' => [0u32, 27, 28, 29, 30, 31, 127][(c - '2' as u32) as usize],
        _ => c,
    };
    char::from_u32(out).unwrap_or(ch)
}

/// `EncodingHelper`: what `_encodeRegular` decided.
#[derive(Default)]
struct Enc {
    alt_prefix: bool,
    plain: Option<&'static str>,
    ss3_final: Option<char>,
    csi_final: Option<char>,
    csi_code: u32,
    csi_modifier: u32,
}

fn format(enc: &Enc, alt: bool) -> Option<String> {
    let mut seq = String::new();
    if enc.alt_prefix && alt {
        seq.push('\x1b');
    }
    if let Some(fin) = enc.csi_final {
        let mut code = enc.csi_code;
        let mut modifier = enc.csi_modifier;
        if modifier != 0 {
            modifier += 1; // the CSI modifier parameter is 1-based
            code = code.max(1);
        }
        seq.push_str("\x1b[");
        if code != 0 {
            seq.push_str(&code.to_string());
        }
        if modifier != 0 {
            seq.push_str(&format!(";{modifier}"));
        }
        seq.push(fin);
        return Some(seq);
    }
    if let Some(fin) = enc.ss3_final {
        seq.push_str("\x1bO");
        seq.push(fin);
        return Some(seq);
    }
    enc.plain.map(|p| {
        seq.push_str(p);
        seq
    })
}

/// `_encodeRegular` for the key named `key` (GPUI key names).
fn encode_regular(key: &str, ctrl: bool, shift: bool, modifier: u32, m: KeyModes) -> Enc {
    let mut e = Enc { csi_modifier: modifier, ..Default::default() };
    let modified = modifier != 0;
    // Cursor/Home/End: SS3 only when unmodified and DECCKM is set.
    let cursor = |c: char, e: &mut Enc| {
        if modified || !m.cursor_key {
            e.csi_final = Some(c);
        } else {
            e.ss3_final = Some(c);
        }
    };
    match key {
        "backspace" => {
            e.alt_prefix = true;
            e.plain = Some(if ctrl == m.backarrow { "\x7f" } else { "\x08" });
        }
        "tab" => {
            e.alt_prefix = true;
            e.plain = Some(if shift { "\x1b[Z" } else { "\t" });
        }
        "enter" => {
            e.alt_prefix = true;
            e.plain = Some(if ctrl {
                "\n"
            } else if m.line_feed {
                "\r\n"
            } else {
                "\r"
            });
        }
        "pause" => e.plain = Some("\x1a"),
        "f1" | "f2" | "f3" | "f4" => {
            let c = (b'P' + (key.as_bytes()[1] - b'1')) as char;
            if modified {
                e.csi_final = Some(c);
            } else {
                e.ss3_final = Some(c);
            }
        }
        "f5" | "f6" | "f7" | "f8" | "f9" | "f10" | "f11" | "f12" | "f13" | "f14" | "f15" | "f16" | "f17"
        | "f18" | "f19" | "f20" => {
            const LUT: [u32; 16] = [15, 17, 18, 19, 20, 21, 23, 24, 25, 26, 28, 29, 31, 32, 33, 34];
            let n: usize = key[1..].parse().unwrap_or(5);
            e.csi_final = Some('~');
            e.csi_code = LUT[n - 5];
        }
        "left" => cursor('D', &mut e),
        "up" => cursor('A', &mut e),
        "right" => cursor('C', &mut e),
        "down" => cursor('B', &mut e),
        "home" => cursor('H', &mut e),
        "end" => cursor('F', &mut e),
        "insert" | "delete" | "pageup" | "pagedown" => {
            e.csi_final = Some('~');
            e.csi_code = match key {
                "insert" => 2,
                "delete" => 3,
                "pageup" => 5,
                _ => 6,
            };
        }
        _ => {}
    }
    e
}

/// Translate `ks` into the bytes to send; `None` = nothing to send.
pub fn encode_keystroke(ks: &Keystroke, m: KeyModes) -> Option<String> {
    let mods = &ks.modifiers;
    if mods.platform {
        return None;
    }
    let key = ks.key.as_str();
    let typed = ks.key_char.as_deref().and_then(|s| s.chars().next());
    // AltGr (Ctrl+Alt on international layouts) that produced a graphic char is text.
    let alt_gr = mods.alt && mods.control && typed.is_some_and(|c| (c as u32) > 0x20 && c as u32 != 0x7f);
    let (ctrl, alt) = (mods.control && !alt_gr, mods.alt && !alt_gr);
    let shift = mods.shift;
    let modifier = (shift as u32) * CSI_SHIFT + (alt as u32) * CSI_ALT + (ctrl as u32) * CSI_CTRL;

    let enc = encode_regular(key, ctrl, shift, modifier, m);
    if let Some(seq) = format(&enc, alt) {
        return Some(seq);
    }

    // `_formatFallback`: one character, Ctrl applied, ESC prefix for Alt.
    let mut cp = if key == "escape" {
        '\x1b'
    } else if ctrl && key == "space" {
        ' '
    } else if let Some(c) = typed.filter(|c| !c.is_control()) {
        c
    } else if alt || ctrl {
        // The layout character of the key itself (no text was produced).
        match key {
            "space" => ' ',
            k if k.chars().count() == 1 => k.chars().next()?,
            _ => return None,
        }
    } else {
        return None;
    };
    if ctrl {
        cp = ctrl_char(cp);
    }
    let mut seq = String::new();
    if alt {
        seq.push('\x1b');
    }
    seq.push(cp);
    Some(seq)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(s: &'static str) -> Option<String> {
        enc_m(s, KeyModes::default())
    }

    fn enc_m(s: &'static str, m: KeyModes) -> Option<String> {
        let mut ks = Keystroke::parse(s).unwrap();
        let plain = !ks.modifiers.control && !ks.modifiers.alt;
        if ks.key.chars().count() == 1 && plain {
            ks.key_char = Some(if ks.modifiers.shift { ks.key.to_uppercase() } else { ks.key.clone() });
        }
        if ks.key == "space" && plain {
            ks.key_char = Some(" ".into());
        }
        encode_keystroke(&ks, m)
    }

    #[test]
    fn plain_keys() {
        assert_eq!(enc("enter").as_deref(), Some("\r"));
        assert_eq!(enc("tab").as_deref(), Some("\t"));
        assert_eq!(enc("shift-tab").as_deref(), Some("\x1b[Z"));
        assert_eq!(enc("backspace").as_deref(), Some("\x7f"));
        assert_eq!(enc("escape").as_deref(), Some("\x1b"));
        assert_eq!(enc("a").as_deref(), Some("a"));
        assert_eq!(enc("shift-a").as_deref(), Some("A"));
        assert_eq!(enc("space").as_deref(), Some(" "));
    }

    #[test]
    fn backspace_enter_and_modes() {
        assert_eq!(enc("ctrl-backspace").as_deref(), Some("\x08"));
        assert_eq!(enc("alt-backspace").as_deref(), Some("\x1b\x7f"));
        let bk = KeyModes { backarrow: true, ..Default::default() };
        assert_eq!(enc_m("backspace", bk).as_deref(), Some("\x08"));
        assert_eq!(enc_m("ctrl-backspace", bk).as_deref(), Some("\x7f"));
        assert_eq!(enc("ctrl-enter").as_deref(), Some("\n"));
        assert_eq!(enc("alt-enter").as_deref(), Some("\x1b\r"));
        let lnm = KeyModes { line_feed: true, ..Default::default() };
        assert_eq!(enc_m("enter", lnm).as_deref(), Some("\r\n"));
        assert_eq!(enc("alt-tab").as_deref(), Some("\x1b\t"));
        assert_eq!(enc("alt-escape").as_deref(), Some("\x1b\x1b"));
    }

    #[test]
    fn cursor_keys_follow_decckm() {
        assert_eq!(enc("up").as_deref(), Some("\x1b[A"));
        assert_eq!(enc("home").as_deref(), Some("\x1b[H"));
        let app = KeyModes { cursor_key: true, ..Default::default() };
        assert_eq!(enc_m("up", app).as_deref(), Some("\x1bOA"));
        assert_eq!(enc_m("end", app).as_deref(), Some("\x1bOF"));
        assert_eq!(enc_m("ctrl-left", app).as_deref(), Some("\x1b[1;5D"));
        assert_eq!(enc("shift-up").as_deref(), Some("\x1b[1;2A"));
        assert_eq!(enc("ctrl-alt-right").as_deref(), Some("\x1b[1;7C"));
    }

    #[test]
    fn function_and_editing_keys() {
        assert_eq!(enc("f1").as_deref(), Some("\x1bOP"));
        assert_eq!(enc("shift-f1").as_deref(), Some("\x1b[1;2P"));
        assert_eq!(enc("ctrl-f4").as_deref(), Some("\x1b[1;5S"));
        assert_eq!(enc("f5").as_deref(), Some("\x1b[15~"));
        assert_eq!(enc("f12").as_deref(), Some("\x1b[24~"));
        assert_eq!(enc("f13").as_deref(), Some("\x1b[25~"));
        assert_eq!(enc("f20").as_deref(), Some("\x1b[34~"));
        assert_eq!(enc("delete").as_deref(), Some("\x1b[3~"));
        assert_eq!(enc("ctrl-delete").as_deref(), Some("\x1b[3;5~"));
        assert_eq!(enc("pageup").as_deref(), Some("\x1b[5~"));
        assert_eq!(enc("shift-insert").as_deref(), Some("\x1b[2;2~"));
        assert_eq!(enc("pause").as_deref(), Some("\x1a"));
    }

    #[test]
    fn control_characters() {
        assert_eq!(enc("ctrl-c").as_deref(), Some("\x03"));
        assert_eq!(enc("ctrl-l").as_deref(), Some("\x0c"));
        assert_eq!(enc("ctrl-space").as_deref(), Some("\0"));
        assert_eq!(enc("ctrl-/").as_deref(), Some("\x1f"));
        assert_eq!(enc("ctrl-2").as_deref(), Some("\0"));
        assert_eq!(enc("ctrl-3").as_deref(), Some("\x1b"));
        assert_eq!(enc("ctrl-8").as_deref(), Some("\x7f"));
        assert_eq!(enc("ctrl-[").as_deref(), Some("\x1b"));
        assert_eq!(enc("alt-b").as_deref(), Some("\x1bb"));
        assert_eq!(enc("ctrl-alt-c").as_deref(), Some("\x1b\x03"));
    }
}
