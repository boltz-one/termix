//! `OutputStateMachineEngine` — wrapper trên `vte::Perform` mapping tới
//! `ITermDispatch`. Port của `windows/src/terminal/parser/OutputStateMachineEngine.hpp`.
//!
//! KHÔNG re-port `StateMachine.cpp` (~113k LOC) — `vte` đã có state machine
//! chuẩn ANSI. Ta chỉ cần adapter layer map `vte::Perform` callbacks → 123
//! `ITermDispatch` methods via match-arms theo dispatch tables ở
//! `OutputStateMachineEngine.hpp:58-233`.
//!
//! Phase 2: full mapping cho ESC/CSI/OSC/DCS với verbatim C++ semantics.

use vte::{Params, Perform};

use crate::dispatch::ITermDispatch;

/// `INVALID_COLOR = 0xffffffff` (`conattrs.hpp:52`).
pub const INVALID_COLOR: u32 = 0xffff_ffff;
/// `COLOR_INQUIRY_COLOR = 0xfeffffff` (`OutputStateMachineEngine.cpp:17`).
pub const COLOR_INQUIRY_COLOR: u32 = 0xfeff_ffff;

/// `OscActionCodes` (`OutputStateMachineEngine.hpp:207-233`).
mod osc_code {
    pub const SET_ICON_AND_WINDOW_TITLE: usize = 0;
    pub const SET_WINDOW_ICON: usize = 1;
    pub const SET_WINDOW_TITLE: usize = 2;
    pub const SET_COLOR: usize = 4;
    pub const CURRENT_WORKING_DIRECTORY: usize = 7;
    pub const HYPERLINK: usize = 8;
    pub const CONEMU_ACTION: usize = 9;
    pub const SET_FOREGROUND_COLOR: usize = 10;
    pub const SET_BACKGROUND_COLOR: usize = 11;
    pub const SET_CURSOR_COLOR: usize = 12;
    pub const SET_HIGHLIGHT_COLOR: usize = 17;
    pub const DECSWT_SET_WINDOW_TITLE: usize = 21;
    pub const SET_CLIPBOARD: usize = 52;
    pub const RESET_COLOR: usize = 104;
    pub const RESET_FOREGROUND_COLOR: usize = 110;
    pub const RESET_BACKGROUND_COLOR: usize = 111;
    pub const RESET_CURSOR_COLOR: usize = 112;
    pub const RESET_HIGHLIGHT_COLOR: usize = 117;
    pub const FINALTERM_ACTION: usize = 133;
    pub const VSCODE_ACTION: usize = 633;
    pub const URXVT_ACTION: usize = 777;
    pub const ITERM2_ACTION: usize = 1337;
    pub const WT_ACTION: usize = 9001;
}

/// Parse color từ xterm color string (`Utils::ColorFromXTermColor`).
/// Hỗ trợ: `rgb:RR/GG/BB`, `#RRGGBB`, decimal `RR GG BB` ...
pub fn parse_xterm_color(s: &str) -> Option<u32> {
    let s = s.trim();
    if s == "?" {
        return Some(COLOR_INQUIRY_COLOR);
    }
    // `rgb:rr/gg/bb` format (XParseColor spec).
    if let Some(rest) = s.strip_prefix("rgb:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 3 {
            let r = u32::from_str_radix(parts[0], 16).ok()?;
            let g = u32::from_str_radix(parts[1], 16).ok()?;
            let b = u32::from_str_radix(parts[2], 16).ok()?;
            return Some((r & 0xff) | ((g & 0xff) << 8) | ((b & 0xff) << 16));
        }
    }
    // `#rrggbb` HTML format.
    if let Some(rest) = s.strip_prefix('#') {
        if rest.len() == 6 {
            let r = u32::from_str_radix(&rest[0..2], 16).ok()?;
            let g = u32::from_str_radix(&rest[2..4], 16).ok()?;
            let b = u32::from_str_radix(&rest[4..6], 16).ok()?;
            return Some((r & 0xff) | ((g & 0xff) << 8) | ((b & 0xff) << 16));
        }
    }
    // Decimal "R G B" hoặc "R;G;B".
    let parts: Vec<&str> = s.split([' ', ';', ',']).collect();
    if parts.len() == 3 {
        let r = parts[0].trim().parse::<u32>().ok()?;
        let g = parts[1].trim().parse::<u32>().ok()?;
        let b = parts[2].trim().parse::<u32>().ok()?;
        return Some((r & 0xff) | ((g & 0xff) << 8) | ((b & 0xff) << 16));
    }
    None
}

/// `OutputStateMachineEngine` — port `class OutputStateMachineEngine`
/// (`OutputStateMachineEngine.hpp:44`). Holds dispatch target.
///
/// Generic trên `D: ITermDispatch`. Callers cần trait-object dispatch dùng
/// `OutputStateMachineEngine<Box<dyn ITermDispatch>>` (xem blank impl trong
/// `dispatch/mod.rs`); callers cần access concrete dispatch fields (e.g.
/// `control` đọc `dispatch.terminal` sau vte parse) dùng
/// `OutputStateMachineEngine<TerminalDispatch>`.
pub struct OutputStateMachineEngine<D: ITermDispatch = Box<dyn ITermDispatch>> {
    pub dispatch: D,
    /// Cuối byte cuối CSI — để map vào `ITermDispatch` method.
    last_byte: u8,
    /// `_lastPrintedChar` (`OutputStateMachineEngine.hpp:56`) — char cuối của
    /// print run close, dùng cho `REP_RepeatCharacter` (`CSI b`). Reset về
    /// `'\0'` ở mọi dispatch action (port `_ClearLastChar`).
    last_printed_char: char,
}

impl<D: ITermDispatch> OutputStateMachineEngine<D> {
    pub fn new(dispatch: D) -> Self {
        Self {
            dispatch,
            last_byte: 0,
            last_printed_char: '\0',
        }
    }

    /// `_ClearLastChar()` (`OutputStateMachineEngine.cpp:1155-1158`) — reset
    /// `_lastPrintedChar = NUL`. Gọi ở cuối mọi dispatch action (Execute,
    /// CsiDispatch, EscDispatch, DcsDispatch, OscDispatch, Ss3Dispatch,
    /// Vt52Dispatch) để rời print run.
    fn clear_last_char(&mut self) {
        self.last_printed_char = '\0';
    }
}

/// Phase 1: map `vte::Perform` callbacks → `ITermDispatch`. Lane B điền
/// match-arms theo `OutputStateMachineEngine.hpp:58-233` (CsiActionCodes,
/// EscActionCodes, DcsActionCodes, OscActionCodes).
#[allow(unused_variables)]
impl<D: ITermDispatch> Perform for OutputStateMachineEngine<D> {
    /// `print()` → `ITermDispatch::Print(c)`. Port `ActionPrint`
    /// (`OutputStateMachineEngine.cpp:132-141`): stash `c` vào
    /// `last_printed_char` nếu `c >= SPC` (graphical char).
    fn print(&mut self, c: char) {
        if c >= ' ' {
            self.last_printed_char = c;
        }
        self.dispatch.print(c);
    }

    /// `execute()` — C0 control (BEL, CR, LF, BS, HT, ...).
    /// Port verbatim `ActionExecute` `OutputStateMachineEngine.cpp:54-108`.
    ///
    /// Mapped byte → ITermDispatch method (theo WT match order):
    ///   ENQ 0x05 → EnquireAnswerback
    ///   BEL 0x07 → WarningBell
    ///   BS  0x08 → CursorBackward(1)
    ///   HT  0x09 → ForwardTab(1)
    ///   CR  0x0d → CarriageReturn
    ///   LF/FF/VT 0x0a/0x0c/0x0b → LineFeed(DependsOnMode)
    ///   SO  0x0e → LockingShift(1)
    ///   SI  0x0f → LockingShift(0)
    ///   SUB 0x1a → Print('\u{2426}')
    ///   DEL 0x7f → Print(byte as char)
    /// Other C0: filter out (GH#1825, GH#10786) — không fill buffer.
    fn execute(&mut self, byte: u8) {
        match byte {
            // ENQ = 0x05 — EnquireAnswerback.
            0x05 => self.dispatch.enquire_answerback(),
            // BEL = 0x07.
            0x07 => self.dispatch.warning_bell(),
            // BS = 0x08 → CursorBackward(1).
            0x08 => self.dispatch.cursor_backward(1),
            // HT = 0x09 → ForwardTab(1).
            0x09 => self.dispatch.forward_tab(1),
            // CR = 0x0d.
            0x0d => self.dispatch.carriage_return(),
            // LF = 0x0a, FF = 0x0c, VT = 0x0b — identical, DependsOnMode (LNM
            // read inside `line_feed` body).
            0x0a..=0x0c => {
                self.dispatch
                    .line_feed(crate::dispatch::LineFeedType::DependsOnMode);
            }
            // SO = 0x0e → LockingShift(1).
            0x0e => self.dispatch.locking_shift(1),
            // SI = 0x0f → LockingShift(0).
            0x0f => self.dispatch.locking_shift(0),
            // SUB = 0x1a → cancel sequence + print "substitute" glyph U+2426.
            0x1a => self.dispatch.print('\u{2426}'),
            // DEL = 0x7f → pass-through to Print (96-char set may translate it;
            // else filtered inside Print).
            0x7f => self.dispatch.print(byte as char),
            // Other C0/1-byte: filter out — không fill buffer (GH#1825/10786).
            _ => {}
        }
        // `ActionExecute` ends with `_ClearLastChar()` (`OutputStateMachineEngine.cpp:105`).
        self.clear_last_char();
    }

    /// Hook cho DCS string — xterm sixel, DECRQSS, ...
    /// Port `OutputStateMachineEngine::ActionDcsDispatch` theo
    /// `DcsActionCodes` (`OutputStateMachineEngine.hpp:177-186`).
    fn hook(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        // DCS dispatch theo final byte + intermediates. Phase 2 stub: ghi
        // log + trì handle sau khi sixel/DECRQSS impl xong.
        let _ = (params, intermediates, action);
        // `ActionDcsDispatch` ends with `_ClearLastChar()` (`:750`).
        self.clear_last_char();
    }

    fn put(&mut self, _byte: u8) {
        // Phase 2: feed vào DCS handler. Skeleton no-op.
    }

    fn unhook(&mut self) {
        // Phase 2: finalize DCS handler.
    }

    /// `osc_dispatch()` — port `OutputStateMachineEngine::ActionOscDispatch`
    /// theo `OscActionCodes` (`OutputStateMachineEngine.cpp:763-923`).
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // vte::Params: `params[0]` = OSC number (bytes), còn lại là payload
        // split bằng `;`. Re-join với `;` để reconstruct chuỗi gốc.
        if params.is_empty() {
            return;
        }
        let osc_num = parse_usize_from_bytes(params[0]).unwrap_or(0);
        // Join params[1..] với ';' để restore nội dung payload gốc.
        let payload = if params.len() > 1 {
            params[1..]
                .iter()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .collect::<Vec<_>>()
                .join(";")
        } else {
            String::new()
        };
        match osc_num {
            osc_code::SET_ICON_AND_WINDOW_TITLE
            | osc_code::SET_WINDOW_ICON
            | osc_code::SET_WINDOW_TITLE
            | osc_code::DECSWT_SET_WINDOW_TITLE => {
                self.dispatch.set_window_title(&payload);
            }
            osc_code::SET_COLOR => {
                // `_GetOscSetColorTable` (`OutputStateMachineEngine.cpp:955`).
                let parts: Vec<&str> = payload.split(';').collect();
                if parts.len() < 2 {
                    return;
                }
                let mut i = 0;
                let mut j = 1;
                while j < parts.len() {
                    if let Some(table_index) = parse_usize_from_str(parts[i]) {
                        let color_str = parts[j];
                        if color_str == "?" {
                            self.dispatch.request_color_table_entry(table_index);
                        } else if let Some(rgb) = parse_xterm_color(color_str) {
                            self.dispatch.set_color_table_entry(table_index, rgb);
                        }
                    }
                    i += 2;
                    j += 2;
                }
            }
            osc_code::SET_FOREGROUND_COLOR
            | osc_code::SET_BACKGROUND_COLOR
            | osc_code::SET_CURSOR_COLOR
            | osc_code::SET_HIGHLIGHT_COLOR => {
                // `_GetOscSetColor` (`OutputStateMachineEngine.cpp:1062`).
                let resource = osc_num;
                let parts: Vec<&str> = payload.split(';').collect();
                for (offset, part) in parts.iter().enumerate() {
                    let current_resource = resource + offset;
                    if *part == "?" {
                        self.dispatch.request_xterm_color_resource(current_resource);
                    } else if let Some(rgb) = parse_xterm_color(part) {
                        if rgb != INVALID_COLOR {
                            self.dispatch
                                .set_xterm_color_resource(current_resource, rgb);
                        }
                    }
                }
            }
            osc_code::SET_CLIPBOARD => {
                // `_GetOscSetClipboard` (`OutputStateMachineEngine.cpp:1105`).
                let pos = payload.find(';');
                if let Some(p) = pos {
                    let substr = &payload[p + 1..];
                    if substr == "?" {
                        // Query clipboard — phase 2 wiring; skip.
                    } else {
                        // Base64 decode để lấy clipboard content. Phase 2
                        // dùng base64 crate. Tạm pass-through raw content
                        // (đã decoded chưa base64) — caller cần decode.
                        self.dispatch.set_clipboard(substr);
                    }
                }
            }
            osc_code::RESET_COLOR => {
                if payload.is_empty() {
                    self.dispatch.reset_color_table();
                } else {
                    for c in payload.split(';') {
                        if let Some(index) = parse_usize_from_str(c) {
                            self.dispatch.reset_color_table_entry(index);
                        } else {
                            break;
                        }
                    }
                }
            }
            osc_code::RESET_FOREGROUND_COLOR
            | osc_code::RESET_BACKGROUND_COLOR
            | osc_code::RESET_CURSOR_COLOR
            | osc_code::RESET_HIGHLIGHT_COLOR => {
                if payload.is_empty() {
                    // Reset codes = set codes + 100 (`OutputStateMachineEngine.cpp:863`).
                    self.dispatch.reset_xterm_color_resource(osc_num - 100);
                }
            }
            osc_code::CURRENT_WORKING_DIRECTORY => {
                self.dispatch.set_current_working_directory(&payload);
            }
            osc_code::HYPERLINK => {
                // `_ParseHyperlink` (`OutputStateMachineEngine.cpp:1016`).
                if payload == ";" {
                    self.dispatch.end_hyperlink();
                } else if let Some(mid) = payload.find(';') {
                    let uri = &payload[mid + 1..];
                    let params_str = &payload[..mid];
                    let mut params_out = String::new();
                    for part in params_str.split(':') {
                        if let Some(id_pos) = part.find("id=") {
                            params_out = part[id_pos + 4..].to_string();
                        }
                    }
                    if !uri.is_empty() {
                        self.dispatch.add_hyperlink(uri, &params_out);
                    }
                }
            }
            osc_code::CONEMU_ACTION => self.dispatch.do_con_emu_action(&payload),
            osc_code::ITERM2_ACTION => self.dispatch.do_iterm2_action(&payload),
            osc_code::FINALTERM_ACTION => self.dispatch.do_final_term_action(&payload),
            osc_code::VSCODE_ACTION => self.dispatch.do_vs_code_action(&payload),
            osc_code::WT_ACTION => self.dispatch.do_wt_action(&payload),
            osc_code::URXVT_ACTION => self.dispatch.do_urxvt_action(&payload),
            _ => self.dispatch.unknown_sequence(),
        }
        // `ActionOscDispatch` ends with `_ClearLastChar()` (`OutputStateMachineEngine.cpp:920`).
        self.clear_last_char();
    }

    /// `csi_dispatch()` — port verbatim `OutputStateMachineEngine::ActionCsiDispatch`
    /// (`OutputStateMachineEngine.cpp:411-698`) theo `CsiActionCodes`
    /// (`OutputStateMachineEngine.hpp:93-175`). WT `VTID` mã hóa intermediates
    /// + final byte thành 1 string (e.g. `"?J"`, `"$r"`, `"!p"`, `"#p"`).
    ///
    /// Vte truyền `intermediates: &[u8]` + `action: char` (final byte) riêng.
    /// Build `id` buffer = `intermediates ++ [byte]` rồi match `&str` để tái
    /// lập `switch (id)` parity.
    ///
    /// WT `parameters.at(idx).value_or(N)` maps: omit (empty) hoặc `<0` → `N`.
    /// Helper `default_param(params, idx, N)` thực hiện chính xác semantic đó.
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        self.last_byte = action as u8;
        // Build `VTID`-like string: intermediates (≤3 bytes theo WT limit) +
        // final byte. Match lại para `switch (id)`.
        let mut id_buf = [0u8; 4];
        let mut id_len = 0;
        for &b in intermediates {
            if id_len >= 3 {
                break;
            }
            id_buf[id_len] = b;
            id_len += 1;
        }
        id_buf[id_len] = action as u8;
        id_len += 1;
        let id_str = match std::str::from_utf8(&id_buf[..id_len]) {
            Ok(s) => s,
            Err(_) => {
                self.clear_last_char();
                return;
            }
        };
        match id_str {
            // CUU = 'A' → CursorUp (`cpp:421-423`).
            "A" => self.dispatch.cursor_up(default_param(params, 0, 1)),
            // CUD = 'B' → CursorDown (`:424-426`).
            "B" => self.dispatch.cursor_down(default_param(params, 0, 1)),
            // CUF = 'C' → CursorForward (`:427-429`).
            "C" => self.dispatch.cursor_forward(default_param(params, 0, 1)),
            // CUB = 'D' → CursorBackward (`:430-432`).
            "D" => self.dispatch.cursor_backward(default_param(params, 0, 1)),
            // CNL = 'E' → CursorNextLine (`:433-435`).
            "E" => self.dispatch.cursor_next_line(default_param(params, 0, 1)),
            // CPL = 'F' → CursorPrevLine (`:436-438`).
            "F" => self.dispatch.cursor_prev_line(default_param(params, 0, 1)),
            // CHA / HPA = 'G' / '`' — cùng method `CursorHorizontalPositionAbsolute`
            // (`:439-442`). CHA final 'G', HPA final '`' — both unresolved here.
            "G" | "`" => self
                .dispatch
                .cursor_horizontal_position_absolute(default_param(params, 0, 1)),
            // VPA = 'd' → VerticalLinePositionAbsolute (`:443-445`).
            "d" => self
                .dispatch
                .vertical_line_position_absolute(default_param(params, 0, 1)),
            // HPR = 'a' → HorizontalPositionRelative (`:446-448`).
            "a" => self
                .dispatch
                .horizontal_position_relative(default_param(params, 0, 1)),
            // VPR = 'e' → VerticalPositionRelative (`:449-451`).
            "e" => self
                .dispatch
                .vertical_position_relative(default_param(params, 0, 1)),
            // CUP = 'H', HVP = 'f' — cùng method `CursorPosition` (`:452-455`).
            "H" | "f" => {
                let line = default_param(params, 0, 1);
                let col = default_param(params, 1, 1);
                self.dispatch.cursor_position(line, col);
            }
            // DECSTBM = 'r' → SetTopBottomScrollingMargins (`:456-458`).
            // `value_or(0)` — omit → 0 (host-side clamp to viewport).
            "r" => {
                let top = default_param(params, 0, 0);
                let bottom = default_param(params, 1, 0);
                self.dispatch.set_top_bottom_scrolling_margins(top, bottom);
            }
            // DECSLRM = 's' → SetLeftRightScrollingMargins (`:459-462`).
            "s" => {
                let left = default_param(params, 0, 0);
                let right = default_param(params, 1, 0);
                self.dispatch.set_left_right_scrolling_margins(left, right);
            }
            // ICH = '@' → InsertCharacter (`:463-465`).
            "@" => self.dispatch.insert_character(default_param(params, 0, 1)),
            // DCH = 'P' → DeleteCharacter (`:466-468`).
            "P" => self.dispatch.delete_character(default_param(params, 0, 1)),
            // ED = 'J' — `parameters.for_each` iterate mọi param →
            // `EraseInDisplay` (`:469-473`). Empty → 1× `ToEnd`.
            "J" => {
                for_each_erase_type(params, |et| {
                    self.dispatch.erase_in_display(et);
                });
            }
            // DECSED = '?J' — SelectiveEraseInDisplay (`:474-478`).
            "?J" => {
                for_each_erase_type(params, |et| {
                    self.dispatch.selective_erase_in_display(et);
                });
            }
            // EL = 'K' — `EraseInLine` (`:479-483`).
            "K" => {
                for_each_erase_type(params, |et| {
                    self.dispatch.erase_in_line(et);
                });
            }
            // DECSEL = '?K' — SelectiveEraseInLine (`:484-488`).
            "?K" => {
                for_each_erase_type(params, |et| {
                    self.dispatch.selective_erase_in_line(et);
                });
            }
            // SM = 'h' — `for_each` set ANSI standard mode (`:489-493`).
            "h" => {
                for_each_mode(params, |raw| {
                    self.dispatch
                        .set_mode(crate::dispatch::types::ansi_standard_mode_from_i32(raw));
                });
            }
            // DECSET = '?h' — `for_each` set DEC private mode (`:494-498`).
            "?h" => {
                for_each_mode(params, |raw| {
                    self.dispatch
                        .set_mode(crate::dispatch::types::dec_private_mode_from_i32(raw));
                });
            }
            // RM = 'l' — reset ANSI mode (`:499-503`).
            "l" => {
                for_each_mode(params, |raw| {
                    self.dispatch
                        .reset_mode(crate::dispatch::types::ansi_standard_mode_from_i32(raw));
                });
            }
            // DECRST = '?l' — reset DEC private mode (`:504-508`).
            "?l" => {
                for_each_mode(params, |raw| {
                    self.dispatch
                        .reset_mode(crate::dispatch::types::dec_private_mode_from_i32(raw));
                });
            }
            // SGR = 'm' — pass full params slice (`:509-511`).
            "m" => {
                let slice: Vec<i32> = collect_params(params);
                self.dispatch.set_graphics_rendition(&slice);
            }
            // DSR = 'n' — ANSI status (`:512-514`).
            "n" => {
                let status = crate::dispatch::types::ansi_standard_status_from_i32(default_param(
                    params, 0, 0,
                ));
                let id = default_param(params, 1, 0);
                self.dispatch.device_status_report(status, id);
            }
            // DSR private = '?n' (`:515-517`).
            "?n" => {
                let status = crate::dispatch::types::dec_private_status_from_i32(default_param(
                    params, 0, 0,
                ));
                let id = default_param(params, 1, 0);
                self.dispatch.device_status_report(status, id);
            }
            // DA = 'c' (`:518-523`). Only if `at(0).value_or(0) == 0`.
            "c" => {
                if default_param(params, 0, 0) == 0 {
                    self.dispatch.device_attributes();
                }
            }
            // DA2 = '>c' (`:524-529`).
            ">c" => {
                if default_param(params, 0, 0) == 0 {
                    self.dispatch.secondary_device_attributes();
                }
            }
            // DA3 = '=c' (`:530-535`).
            "=c" => {
                if default_param(params, 0, 0) == 0 {
                    self.dispatch.tertiary_device_attributes();
                }
            }
            // DECREQTPARM = 'x' (`:536-538`). `at(0)` cast `ReportingPermission`.
            "x" => {
                let perm = crate::dispatch::types::ReportingPermission::from_i32(default_param(
                    params, 0, 0,
                ));
                self.dispatch.request_terminal_parameters(perm);
            }
            // SU = 'S' → ScrollUp (`:539-541`).
            "S" => self.dispatch.scroll_up(default_param(params, 0, 1)),
            // SD = 'T' → ScrollDown (`:542-544`).
            "T" => self.dispatch.scroll_down(default_param(params, 0, 1)),
            // NP = 'U' → NextPage (`:545-547`).
            "U" => self.dispatch.next_page(default_param(params, 0, 1)),
            // PP = 'V' → PrecedingPage (`:548-550`).
            "V" => self.dispatch.preceding_page(default_param(params, 0, 1)),
            // ANSISYSRC = 'u' → CursorRestoreState (`:551-553`).
            "u" => self.dispatch.cursor_restore_state(),
            // IL = 'L' → InsertLine (`:554-556`).
            "L" => self.dispatch.insert_line(default_param(params, 0, 1)),
            // DL = 'M' → DeleteLine (`:557-559`).
            "M" => self.dispatch.delete_line(default_param(params, 0, 1)),
            // CHT = 'I' → ForwardTab (`:560-562`).
            "I" => self.dispatch.forward_tab(default_param(params, 0, 1)),
            // CBT = 'Z' → BackwardsTab (`:563-565`).
            "Z" => self.dispatch.backwards_tab(default_param(params, 0, 1)),
            // TBC = 'g' — `for_each` → TabClear (`:566-570`).
            "g" => {
                for_each_tab_clear_type(params, |ct| {
                    self.dispatch.tab_clear(ct);
                });
            }
            // DECST8C = '?W' — `for_each` → TabSet (`:571-575`).
            "?W" => {
                for_each_param(params, |raw| {
                    self.dispatch.tab_set(raw);
                });
            }
            // ECH = 'X' → EraseCharacters (`:576-578`).
            "X" => self.dispatch.erase_characters(default_param(params, 0, 1)),
            // DTTERM_WindowManipulation = 't' (`:579-581`). Args 2..n truyền
            // dạng `&[i32]` (skip param 0 = function code).
            "t" => {
                let func = crate::dispatch::types::WindowManipulationType::from_i32(default_param(
                    params, 0, 0,
                ));
                let slice: Vec<i32> = collect_params_skip(params, 1);
                self.dispatch.window_manipulation(func, &slice);
            }
            // REP = 'b' — xử lý w/o dispatch (`:582-594`): nếu `_lastPrintedChar`
            // != NUL, `PrintString(last_char × repeatCount)`. `at(0)` cast `size_t`
            // via `operator VTInt()` = `_value <= 0 ? 1 : _value` → clamp 1.
            "b" => {
                if self.last_printed_char != '\0' {
                    let raw = default_param(params, 0, 1);
                    let repeat_count = raw.max(1) as usize;
                    let s: String =
                        std::iter::repeat_n(self.last_printed_char, repeat_count).collect();
                    self.dispatch.print_string(&s);
                }
            }
            // PPA = ' P' → PagePositionAbsolute (`:595-597`).
            " P" => self
                .dispatch
                .page_position_absolute(default_param(params, 0, 1)),
            // PPR = ' Q' → PagePositionRelative (`:598-600`).
            " Q" => self
                .dispatch
                .page_position_relative(default_param(params, 0, 1)),
            // PPB = ' R' → PagePositionBack (`:601-603`).
            " R" => self
                .dispatch
                .page_position_back(default_param(params, 0, 1)),
            // DECSCUSR = ' q' → SetCursorStyle (`:604-606`).
            " q" => {
                let style =
                    crate::dispatch::types::CursorStyle::from_i32(default_param(params, 0, 0));
                self.dispatch.set_cursor_style(style);
            }
            // DECSTR = '!p' → SoftReset (`:607-609`).
            "!p" => self.dispatch.soft_reset(),
            // DECSCA = '"q' → SetCharacterProtectionAttribute (`:610-612`).
            "\"q" => {
                let slice: Vec<i32> = collect_params(params);
                self.dispatch.set_character_protection_attribute(&slice);
            }
            // DECRQDE = '"v' → RequestDisplayedExtent (`:613-615`).
            "\"v" => self.dispatch.request_displayed_extent(),
            // XT_PushSgr / alias = '#{' / '#p' (`:616-619`).
            "#{" | "#p" => {
                let slice: Vec<i32> = collect_params(params);
                self.dispatch.push_graphics_rendition(&slice);
            }
            // XT_PopSgr / alias = '#}' / '#q' (`:620-623`).
            "#}" | "#q" => self.dispatch.pop_graphics_rendition(),
            // DECRQM = '$p' → RequestMode ANSI (`:624-626`).
            "$p" => {
                let mp = crate::dispatch::types::ansi_standard_mode_from_i32(default_param(
                    params, 0, 0,
                ));
                self.dispatch.request_mode(mp);
            }
            // DECRQM private = '?$p' → RequestMode DEC (`:627-629`).
            "?$p" => {
                let mp =
                    crate::dispatch::types::dec_private_mode_from_i32(default_param(params, 0, 0));
                self.dispatch.request_mode(mp);
            }
            // DECCARA = '$r' → ChangeAttributesRectangularArea (`:630-632`).
            "$r" => {
                let top = default_param(params, 0, 1);
                let left = default_param(params, 1, 1);
                let bottom = default_param_or_zero(params, 2);
                let right = default_param_or_zero(params, 3);
                let attrs: Vec<i32> = collect_params_skip(params, 4);
                self.dispatch
                    .change_attributes_rectangular_area(top, left, bottom, right, &attrs);
            }
            // DECRARA = '$t' → ReverseAttributesRectangularArea (`:633-635`).
            "$t" => {
                let top = default_param(params, 0, 1);
                let left = default_param(params, 1, 1);
                let bottom = default_param_or_zero(params, 2);
                let right = default_param_or_zero(params, 3);
                let attrs: Vec<i32> = collect_params_skip(params, 4);
                self.dispatch
                    .reverse_attributes_rectangular_area(top, left, bottom, right, &attrs);
            }
            // DECRQTSR = '$u' → RequestTerminalStateReport (`:639-641`).
            // WT sig `RequestTerminalStateReport(format, pageId)` — sig order
            // ngược lại `at()` order (format trước, pageId sau).
            "$u" => {
                let format =
                    crate::dispatch::types::ReportFormat::from_i32(default_param(params, 0, 0));
                let page_id = default_param(params, 1, 0);
                self.dispatch.request_terminal_state_report(format, page_id);
            }
            // DECRQPSR = '$w' → RequestPresentationStateReport (`:642-644`).
            "$w" => {
                let fmt = crate::dispatch::types::PresentationReportFormat::from_i32(
                    default_param(params, 0, 0),
                );
                self.dispatch.request_presentation_state_report(fmt);
            }
            // DECCRA = '$v' → CopyRectangularArea (`:636-638`). 8 args.
            "$v" => {
                let top = default_param(params, 0, 1);
                let left = default_param(params, 1, 1);
                let bottom = default_param_or_zero(params, 2);
                let right = default_param_or_zero(params, 3);
                let page = default_param_or_zero(params, 4);
                let dst_top = default_param(params, 5, 1);
                let dst_left = default_param(params, 6, 1);
                let dst_page = default_param_or_zero(params, 7);
                self.dispatch.copy_rectangular_area(
                    top, left, bottom, right, page, dst_top, dst_left, dst_page,
                );
            }
            // DECFRA = '$x' → FillRectangularArea (`:645-647`).
            "$x" => {
                let ch = default_param(params, 0, 0x20);
                let top = default_param(params, 1, 1);
                let left = default_param(params, 2, 1);
                let bottom = default_param_or_zero(params, 3);
                let right = default_param_or_zero(params, 4);
                self.dispatch
                    .fill_rectangular_area(ch, top, left, bottom, right);
            }
            // DECERA = '$z' → EraseRectangularArea (`:648-650`).
            "$z" => {
                let top = default_param(params, 0, 1);
                let left = default_param(params, 1, 1);
                let bottom = default_param_or_zero(params, 2);
                let right = default_param_or_zero(params, 3);
                self.dispatch
                    .erase_rectangular_area(top, left, bottom, right);
            }
            // DECSERA = '${' → SelectiveEraseRectangularArea (`:651-653`).
            "${" => {
                let top = default_param(params, 0, 1);
                let left = default_param(params, 1, 1);
                let bottom = default_param_or_zero(params, 2);
                let right = default_param_or_zero(params, 3);
                self.dispatch
                    .selective_erase_rectangular_area(top, left, bottom, right);
            }
            // DECRQUPSS = '&u' → RequestUserPreferenceCharset (`:654-656`).
            "&u" => self.dispatch.request_user_preference_charset(),
            // DECIC = ''}'' → InsertColumn (`:657-659`).
            "'}" => self.dispatch.insert_column(default_param(params, 0, 1)),
            // DECDC = ''~' → DeleteColumn (`:660-662`).
            "'~" => self.dispatch.delete_column(default_param(params, 0, 1)),
            // DECSACE = '*x' → SelectAttributeChangeExtent (`:663-665`).
            "*x" => {
                let ext =
                    crate::dispatch::types::ChangeExtent::from_i32(default_param(params, 0, 0));
                self.dispatch.select_attribute_change_extent(ext);
            }
            // DECRQCRA = '*y' → RequestChecksumRectangularArea (`:666-668`).
            "*y" => {
                let id = default_param_or_zero(params, 0);
                let page = default_param_or_zero(params, 1);
                let top = default_param(params, 2, 1);
                let left = default_param(params, 3, 1);
                let bottom = default_param_or_zero(params, 4);
                let right = default_param_or_zero(params, 5);
                self.dispatch
                    .request_checksum_rectangular_area(id, page, top, left, bottom, right);
            }
            // DECINVM = '*z' → InvokeMacro (`:669-671`).
            "*z" => {
                self.dispatch.invoke_macro(default_param_or_zero(params, 0));
            }
            // DECAC = ',|' → AssignColor (`:672-674`).
            ",|" => {
                let item = crate::dispatch::types::ColorItem::from_i32(default_param(params, 0, 0));
                let fg = default_param_or_zero(params, 1);
                let bg = default_param_or_zero(params, 2);
                self.dispatch.assign_color(item, fg, bg);
            }
            // DECPS = ',~' → PlaySounds (`:675-677`).
            ",~" => {
                let slice: Vec<i32> = collect_params(params);
                self.dispatch.play_sounds(&slice);
            }
            // Kitty keyboard set = '=u' (`:678-680`).
            "=u" => {
                self.dispatch.set_kitty_keyboard_protocol(
                    default_param(params, 0, 0),
                    default_param(params, 1, 0),
                );
            }
            // Kitty keyboard query = '?u' (`:681-683`).
            "?u" => self.dispatch.query_kitty_keyboard_protocol(),
            // Kitty keyboard push = '>u' (`:684-686`).
            ">u" => self
                .dispatch
                .push_kitty_keyboard_protocol(default_param(params, 0, 0)),
            // Kitty keyboard pop = '<u' (`:687-689`).
            "<u" => self
                .dispatch
                .pop_kitty_keyboard_protocol(default_param(params, 0, 0)),
            // Default — UnknownSequence (`:690-692`).
            _ => self.dispatch.unknown_sequence(),
        }
        // `ActionCsiDispatch` ends with `_ClearLastChar()` (`:695`).
        self.clear_last_char();
    }

    /// `esc_dispatch()` — port verbatim `OutputStateMachineEngine::ActionEscDispatch`
    /// (`OutputStateMachineEngine.cpp:197-331`) theo `EscActionCodes`
    /// (`OutputStateMachineEngine.hpp:58-91`). Match single-byte final lẫn
    /// intermediate-prefix (charset designate `% ( ) * + - . /`, `# 3/4/5/6/8`).
    ///
    /// Vte truyền `intermediates: &[u8]` riêng + `byte` final. WT `VTID` gộp
    /// intermediates + final thành string (`"#3"`, `" F"`); tái lập parity.
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        self.last_byte = byte;
        // Build `VTID`-like: intermediates + final byte.
        let mut id_buf = [0u8; 4];
        let mut id_len = 0;
        for &b in intermediates {
            if id_len >= 3 {
                break;
            }
            id_buf[id_len] = b;
            id_len += 1;
        }
        id_buf[id_len] = byte;
        id_len += 1;
        let id_str = match std::str::from_utf8(&id_buf[..id_len]) {
            Ok(s) => s,
            Err(_) => {
                self.clear_last_char();
                return;
            }
        };
        match id_str {
            // ST = '\' — 7-bit string terminator, no-op (`:201-203`).
            "\\" => {}
            // DECBI = '6' → BackIndex (`:204-206`).
            "6" => self.dispatch.back_index(),
            // DECSC = '7' → CursorSaveState (`:207-209`).
            "7" => self.dispatch.cursor_save_state(),
            // DECRC = '8' → CursorRestoreState (`:210-212`).
            "8" => self.dispatch.cursor_restore_state(),
            // DECFI = '9' → ForwardIndex (`:213-215`).
            "9" => self.dispatch.forward_index(),
            // DECKPAM = '=' → SetKeypadMode(true) (`:216-218`).
            "=" => self.dispatch.set_keypad_mode(true),
            // DECKPNM = '>' → SetKeypadMode(false) (`:219-221`).
            ">" => self.dispatch.set_keypad_mode(false),
            // NEL = 'E' → LineFeed(WithReturn) (`:222-224`).
            "E" => self
                .dispatch
                .line_feed(crate::dispatch::LineFeedType::WithReturn),
            // IND = 'D' → LineFeed(WithoutReturn) (`:225-227`).
            "D" => self
                .dispatch
                .line_feed(crate::dispatch::LineFeedType::WithoutReturn),
            // RI = 'M' → ReverseLineFeed (`:228-230`).
            "M" => self.dispatch.reverse_line_feed(),
            // HTS = 'H' → HorizontalTabSet (`:231-233`).
            "H" => self.dispatch.horizontal_tab_set(),
            // DECID = 'Z' → DeviceAttributes (`:234-236`).
            "Z" => self.dispatch.device_attributes(),
            // RIS = 'c' → HardReset(true) (`:237-239`).
            "c" => self.dispatch.hard_reset(true),
            // SS2 = 'N' → SingleShift(2) (`:240-242`).
            "N" => self.dispatch.single_shift(2),
            // SS3 = 'O' → SingleShift(3) (`:243-245`).
            "O" => self.dispatch.single_shift(3),
            // LS2 = 'n' → LockingShift(2) (`:246-248`).
            "n" => self.dispatch.locking_shift(2),
            // LS3 = 'o' → LockingShift(3) (`:249-251`).
            "o" => self.dispatch.locking_shift(3),
            // LS1R = '~' → LockingShiftRight(1) (`:252-254`).
            "~" => self.dispatch.locking_shift_right(1),
            // LS2R = '}' → LockingShiftRight(2) (`:255-257`).
            "}" => self.dispatch.locking_shift_right(2),
            // LS3R = '|' → LockingShiftRight(3) (`:258-260`).
            "|" => self.dispatch.locking_shift_right(3),
            // DECAC1 = ' 7' → AcceptC1Controls(true) (`:261-263`).
            " 7" => self.dispatch.accept_c1_controls(true),
            // S7C1T = ' F' → SendC1Controls(false) (`:264-266`).
            " F" => self.dispatch.send_c1_controls(false),
            // S8C1T = ' G' → SendC1Controls(true) (`:267-269`).
            " G" => self.dispatch.send_c1_controls(true),
            // ACS level 1/2/3 = ' L' / ' M' / ' N' → AnnounceCodeStructure (`:270-278`).
            " L" => self.dispatch.announce_code_structure(1),
            " M" => self.dispatch.announce_code_structure(2),
            " N" => self.dispatch.announce_code_structure(3),
            // DECDHL top/bottom = '#3' / '#4' (`:279-284`).
            "#3" => self
                .dispatch
                .set_line_rendition(crate::attrs::LineRendition::DoubleHeightTop),
            "#4" => self
                .dispatch
                .set_line_rendition(crate::attrs::LineRendition::DoubleHeightBottom),
            // DECSWL = '#5' → SingleWidth (`:285-287`).
            "#5" => self
                .dispatch
                .set_line_rendition(crate::attrs::LineRendition::SingleWidth),
            // DECDWL = '#6' → DoubleWidth (`:288-290`).
            "#6" => self
                .dispatch
                .set_line_rendition(crate::attrs::LineRendition::DoubleWidth),
            // DECALN = '#8' → ScreenAlignmentPattern (`:291-293`).
            "#8" => self.dispatch.screen_alignment_pattern(),
            // Charset designate (`:294-326`). Intermediate `%` → coding system;
            // `( ) * +` → 94-set G0/1/2/3; `- . /` → 96-set G1/2/3. Final byte
            // truyền vào `commandParameter` (id.SubSequence(1)).
            "%" => {
                self.dispatch
                    .designate_coding_system(id_str.get(1..).unwrap_or(""));
            }
            "(" => self
                .dispatch
                .designate94_charset(0, id_str.get(1..).unwrap_or("")),
            ")" => self
                .dispatch
                .designate94_charset(1, id_str.get(1..).unwrap_or("")),
            "*" => self
                .dispatch
                .designate94_charset(2, id_str.get(1..).unwrap_or("")),
            "+" => self
                .dispatch
                .designate94_charset(3, id_str.get(1..).unwrap_or("")),
            "-" => self
                .dispatch
                .designate96_charset(1, id_str.get(1..).unwrap_or("")),
            "." => self
                .dispatch
                .designate96_charset(2, id_str.get(1..).unwrap_or("")),
            "/" => self
                .dispatch
                .designate96_charset(3, id_str.get(1..).unwrap_or("")),
            _ => {
                // Off-spec — WT default no-op (`:323-325`).
            }
        }
        // `ActionEscDispatch` ends with `_ClearLastChar()` (`:328`).
        self.clear_last_char();
    }
}

/// Helper — port `VTParameters::at(idx).value_or(default)` (`DispatchTypes.hpp`).
/// Omit (empty sub-param) OR raw `< 0` → `default`. Else `raw`.
fn default_param(params: &Params, idx: usize, default: i32) -> i32 {
    match params.iter().nth(idx) {
        Some(slice) if !slice.is_empty() => {
            let raw = slice[0] as i32;
            if raw < 0 {
                default
            } else {
                raw
            }
        }
        _ => default,
    }
}

/// `default_param_or_zero` — chỉ `value_or(0)`. Dành cho optional coords
/// (bottom/right margins, page) nơi `0` = unspecified → host clamps.
fn default_param_or_zero(params: &Params, idx: usize) -> i32 {
    default_param(params, idx, 0)
}

/// Port `VTParameters::for_each` (`DispatchTypes.hpp:282-295`) generic trên
/// `int`. Map mỗi top-level param → `i32` rồi gọi `f`. Empty params → 1 call
/// với `defaultParameter` (value=0).
fn for_each_param<F: FnMut(i32)>(params: &Params, mut f: F) {
    let mut count = 0;
    for slice in params.iter() {
        let raw = if slice.is_empty() { 0 } else { slice[0] as i32 };
        f(raw);
        count += 1;
    }
    if count == 0 {
        f(0);
    }
}

/// `for_each_mode` — SM/RM/DECRQM iterate mỗi mode param (`cpp:489-508`).
fn for_each_mode<F: FnMut(i32)>(params: &Params, f: F) {
    for_each_param(params, f);
}

/// `for_each` cho `TabClear` (`cpp:566-570`). Convert raw → `TabClearType`.
fn for_each_tab_clear_type<F: FnMut(crate::dispatch::TabClearType)>(params: &Params, mut f: F) {
    let mut count = 0;
    for slice in params.iter() {
        let raw = if slice.is_empty() { 0 } else { slice[0] as i32 };
        f(crate::dispatch::TabClearType::from_i32(raw));
        count += 1;
    }
    if count == 0 {
        f(crate::dispatch::TabClearType::from_i32(0));
    }
}

/// Collect toàn bộ top-level params sang `Vec<i32>` — dành cho method nhận
/// full slice (`set_graphics_rendition`, `push_graphics_rendition`,
/// `set_character_protection_attribute`, `play_sounds`). WT `VTParameters`
/// implicit convert sang slice; ở đây flatten top-level sub-param đầu.
fn collect_params(params: &Params) -> Vec<i32> {
    let mut out = Vec::new();
    for slice in params.iter() {
        if slice.is_empty() {
            out.push(0);
        } else {
            out.push(slice[0] as i32);
        }
    }
    if out.is_empty() {
        out.push(0);
    }
    out
}

/// `collect_params_skip(params, n)` — skip `n` top-level params đầu, collect
/// phần còn lại. Dành cho DECCARA/DECRARA/DttermWindow nhận subspan.
fn collect_params_skip(params: &Params, n: usize) -> Vec<i32> {
    let mut out = Vec::new();
    for (i, slice) in params.iter().enumerate() {
        if i < n {
            continue;
        }
        if slice.is_empty() {
            out.push(0);
        } else {
            out.push(slice[0] as i32);
        }
    }
    if out.is_empty() {
        out.push(0);
    }
    out
}

/// Port `VTParameters::for_each` (`DispatchTypes.hpp:282-295`) cho ED/EL family.
///
///
/// WT: nếu `_params.empty()` iterate 1 lần với `defaultParameter` (value=0 →
/// `EraseType::ToEnd`); else iterate mỗi raw param, convert sang `EraseType`
/// via `VTParameter::operator T()` (= `value_or(0)` rồi `static_cast`).
///
/// vte `Params::iter()` yield sub-param slices; `CSI 2;3 J` → 2 lần với `All`
/// rồi `Scrollback`. Empty `CSI J` → `iter()` empty → fallback default 1 lần.
fn for_each_erase_type<F: FnMut(crate::dispatch::EraseType)>(params: &Params, mut f: F) {
    let mut count = 0;
    for slice in params.iter() {
        // `slice[0]` là first sub-param (sub-sub params bỏ qua — port WT
        // `VTParameters::for_each` cũng iterate top-level params).
        if slice.is_empty() {
            f(crate::dispatch::EraseType::ToEnd);
        } else {
            let raw = slice[0] as i32;
            f(crate::dispatch::EraseType::from_i32(raw));
        }
        count += 1;
    }
    if count == 0 {
        // Match WT `defaultParameters` fallback (value=0 → ToEnd).
        f(crate::dispatch::EraseType::ToEnd);
    }
}

/// Parse usize từ `&[u8]` (UTF-8). Anh port `til::parse_unsigned<size_t>(s, 10)`.
fn parse_usize_from_bytes(bytes: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(bytes).ok()?;
    parse_usize_from_str(s)
}

/// Parse usize từ `&str` (radix 10).
fn parse_usize_from_str(s: &str) -> Option<usize> {
    s.trim().parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Track được cho verify vte → dispatch mapping. Test giữ `Rc<RefCell>`
    /// để read state sau khi parser bị move xong, vì `OutputStateMachineEngine`
    /// keeps `Box<dyn ITermDispatch>` (type-erased, không read fields được).
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct CallRec {
        printed: String,
        print_string_calls: Vec<String>,
        bell_count: u32,
        cr_count: u32,
        title: String,
        cwd: String,
        clipboard: String,
        hyperlink_uri: String,
        hyperlink_params: String,
        end_hyperlink_count: u32,
        set_color_calls: Vec<(usize, u32)>,
        request_color_calls: Vec<usize>,
        set_xterm_calls: Vec<(usize, u32)>,
        request_xterm_calls: Vec<usize>,
        reset_xterm_calls: Vec<usize>,
        reset_color_table_count: u32,
        reset_color_entries: Vec<usize>,
        conemu_count: u32,
        iterm2_count: u32,
        finalterm_count: u32,
        vscode_count: u32,
        wt_count: u32,
        urxvt_count: u32,
        unknown_count: u32,
        // —— C0 control counters (verify `execute` port).
        enq_count: u32,
        locking_shift_calls: Vec<crate::dispatch::VtInt>,
        // —— CSI ED/EL counters (verify `erase_in_display`/`erase_in_line` port).
        erase_display_calls: Vec<crate::dispatch::EraseType>,
        erase_line_calls: Vec<crate::dispatch::EraseType>,
    }

    /// Wrapper giữ `Rc<RefCell<CallRec>>` để forward tới trace struct.
    struct RecordingDispatch(Rc<RefCell<CallRec>>);

    impl ITermDispatch for RecordingDispatch {
        fn print(&mut self, c: char) {
            self.0.borrow_mut().printed.push(c);
        }
        fn print_string(&mut self, s: &str) {
            self.0.borrow_mut().print_string_calls.push(s.to_string());
        }
        fn warning_bell(&mut self) {
            self.0.borrow_mut().bell_count += 1;
        }
        fn carriage_return(&mut self) {
            self.0.borrow_mut().cr_count += 1;
        }
        fn cursor_backward(&mut self, _distance: crate::dispatch::VtInt) {}
        fn forward_tab(&mut self, _num_tabs: crate::dispatch::VtInt) {}
        fn line_feed(&mut self, _lft: crate::dispatch::LineFeedType) {}
        fn cursor_up(&mut self, _distance: crate::dispatch::VtInt) {}
        fn cursor_down(&mut self, _distance: crate::dispatch::VtInt) {}
        fn cursor_forward(&mut self, _distance: crate::dispatch::VtInt) {}
        fn cursor_next_line(&mut self, _distance: crate::dispatch::VtInt) {}
        fn cursor_prev_line(&mut self, _distance: crate::dispatch::VtInt) {}
        fn cursor_position(&mut self, _line: crate::dispatch::VtInt, _col: crate::dispatch::VtInt) {
        }
        fn erase_in_display(&mut self, erase_type: crate::dispatch::EraseType) {
            self.0.borrow_mut().erase_display_calls.push(erase_type);
        }
        fn erase_in_line(&mut self, erase_type: crate::dispatch::EraseType) {
            self.0.borrow_mut().erase_line_calls.push(erase_type);
        }
        fn set_graphics_rendition(&mut self, _options: &[crate::dispatch::VtParameter]) {}
        fn cursor_save_state(&mut self) {}
        fn cursor_restore_state(&mut self) {}
        fn reverse_line_feed(&mut self) {}
        fn horizontal_tab_set(&mut self) {}
        fn hard_reset(&mut self, _erase: bool) {}
        // —— C0 control methods (verify `execute` port):
        fn enquire_answerback(&mut self) {
            self.0.borrow_mut().enq_count += 1;
        }
        fn locking_shift(&mut self, gset: crate::dispatch::VtInt) {
            self.0.borrow_mut().locking_shift_calls.push(gset);
        }
        // —— OSC captured methods:
        fn set_window_title(&mut self, title: &str) {
            self.0.borrow_mut().title = title.to_string();
        }
        fn set_current_working_directory(&mut self, uri: &str) {
            self.0.borrow_mut().cwd = uri.to_string();
        }
        fn set_clipboard(&mut self, content: &str) {
            self.0.borrow_mut().clipboard = content.to_string();
        }
        fn add_hyperlink(&mut self, uri: &str, params: &str) {
            self.0.borrow_mut().hyperlink_uri = uri.to_string();
            self.0.borrow_mut().hyperlink_params = params.to_string();
        }
        fn end_hyperlink(&mut self) {
            self.0.borrow_mut().end_hyperlink_count += 1;
        }
        fn set_color_table_entry(&mut self, table_index: usize, color: u32) {
            self.0
                .borrow_mut()
                .set_color_calls
                .push((table_index, color));
        }
        fn request_color_table_entry(&mut self, table_index: usize) {
            self.0.borrow_mut().request_color_calls.push(table_index);
        }
        fn set_xterm_color_resource(&mut self, resource: usize, color: u32) {
            self.0.borrow_mut().set_xterm_calls.push((resource, color));
        }
        fn request_xterm_color_resource(&mut self, resource: usize) {
            self.0.borrow_mut().request_xterm_calls.push(resource);
        }
        fn reset_xterm_color_resource(&mut self, resource: usize) {
            self.0.borrow_mut().reset_xterm_calls.push(resource);
        }
        fn reset_color_table(&mut self) {
            self.0.borrow_mut().reset_color_table_count += 1;
        }
        fn reset_color_table_entry(&mut self, table_index: usize) {
            self.0.borrow_mut().reset_color_entries.push(table_index);
        }
        fn do_con_emu_action(&mut self, _s: &str) {
            self.0.borrow_mut().conemu_count += 1;
        }
        fn do_iterm2_action(&mut self, _s: &str) {
            self.0.borrow_mut().iterm2_count += 1;
        }
        fn do_final_term_action(&mut self, _s: &str) {
            self.0.borrow_mut().finalterm_count += 1;
        }
        fn do_vs_code_action(&mut self, _s: &str) {
            self.0.borrow_mut().vscode_count += 1;
        }
        fn do_wt_action(&mut self, _s: &str) {
            self.0.borrow_mut().wt_count += 1;
        }
        fn do_urxvt_action(&mut self, _s: &str) {
            self.0.borrow_mut().urxvt_count += 1;
        }
        fn unknown_sequence(&mut self) {
            self.0.borrow_mut().unknown_count += 1;
        }
    }

    #[test]
    fn print_callback_routes_to_dispatch() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"hi" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(rec.borrow().printed, "hi");
    }

    #[test]
    fn bel_routes_to_warning_bell() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        parser.advance(&mut eng, 0x07);
        parser.advance(&mut eng, 0x07);
        assert_eq!(rec.borrow().bell_count, 2);
    }

    #[test]
    fn cr_routes_to_carriage_return() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        parser.advance(&mut eng, 0x0d);
        assert_eq!(rec.borrow().cr_count, 1);
    }

    /// ENQ (0x05) → EnquireAnswerback — port `ActionExecute:58-60`.
    #[test]
    fn enq_routes_to_enquire_answerback() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        parser.advance(&mut eng, 0x05);
        assert_eq!(rec.borrow().enq_count, 1);
    }

    /// SI (0x0f) → LockingShift(0), SO (0x0e) → LockingShift(1) — port `:79-84`.
    #[test]
    fn si_so_route_to_locking_shift() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        parser.advance(&mut eng, 0x0f); // SI
        parser.advance(&mut eng, 0x0e); // SO
        assert_eq!(rec.borrow().locking_shift_calls, vec![0, 1]);
    }

    /// SUB (0x1a) → Print('\u{2426}') — port `:85-90`.
    #[test]
    fn sub_routes_to_print_substitute_glyph() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        parser.advance(&mut eng, 0x1a);
        assert_eq!(rec.borrow().printed, "\u{2426}");
    }

    /// DEL (0x7f) → Print(byte as char) — port `:91-97`.
    #[test]
    fn del_routes_to_print() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        parser.advance(&mut eng, 0x7f);
        // DEL as char = U+007F — `print` stores raw.
        assert_eq!(rec.borrow().printed, "\u{007f}");
    }

    /// Other C0 controls (NUL 0x00, DC1 0x11, ...) → filter out, no dispatch.
    /// GH#1825/10786: không fill buffer.
    #[test]
    fn unlisted_c0_controls_filtered_out() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in [0x00u8, 0x01, 0x11, 0x12, 0x18] {
            parser.advance(&mut eng, b);
        }
        assert!(rec.borrow().printed.is_empty(), "no glyph printed");
        assert_eq!(rec.borrow().bell_count, 0);
        assert_eq!(rec.borrow().cr_count, 0);
        assert_eq!(rec.borrow().enq_count, 0);
    }

    /// `CSI J` (empty params) → `EraseInDisplay(ToEnd)` 1 lần — port
    /// `OutputStateMachineEngine.cpp:470-473` + `VTParameters::for_each`
    /// defaultParameter fallback (`DispatchTypes.hpp:286-292`).
    #[test]
    fn csi_ed_empty_params_defaults_to_to_end() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b[J" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(
            rec.borrow().erase_display_calls,
            vec![crate::dispatch::EraseType::ToEnd]
        );
    }

    /// `CSI 2 J` → `EraseInDisplay(All)` — port ED param `2`.
    #[test]
    fn csi_ed_param_2_maps_to_all() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b[2J" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(
            rec.borrow().erase_display_calls,
            vec![crate::dispatch::EraseType::All]
        );
    }

    /// `CSI 3 J` → `EraseInDisplay(Scrollback)` — port ED param `3`.
    #[test]
    fn csi_ed_param_3_maps_to_scrollback() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b[3J" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(
            rec.borrow().erase_display_calls,
            vec![crate::dispatch::EraseType::Scrollback]
        );
    }

    /// `CSI 2 K` → `EraseInLine(All)` — port EL routing (`:480-483`).
    #[test]
    fn csi_el_param_2_maps_to_all() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b[2K" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(
            rec.borrow().erase_line_calls,
            vec![crate::dispatch::EraseType::All]
        );
    }

    /// `CSI 1 J` → `EraseInDisplay(FromBeginning)` — port ED param `1`.
    #[test]
    fn csi_ed_param_1_maps_to_from_beginning() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b[1J" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(
            rec.borrow().erase_display_calls,
            vec![crate::dispatch::EraseType::FromBeginning]
        );
    }

    /// `EraseType::from_i32` — port `VTParameter::operator T()` semantic:
    /// 0 → ToEnd, 1 → FromBeginning, 2 → All, 3 → Scrollback, −1 (omit)/>3 clamp ToEnd.
    #[test]
    fn erase_type_from_i32_clamps_to_to_end() {
        use crate::dispatch::EraseType::*;
        assert_eq!(crate::dispatch::EraseType::from_i32(0), ToEnd);
        assert_eq!(crate::dispatch::EraseType::from_i32(1), FromBeginning);
        assert_eq!(crate::dispatch::EraseType::from_i32(2), All);
        assert_eq!(crate::dispatch::EraseType::from_i32(3), Scrollback);
        assert_eq!(
            crate::dispatch::EraseType::from_i32(-1),
            ToEnd,
            "omit → ToEnd"
        );
        assert_eq!(
            crate::dispatch::EraseType::from_i32(4),
            ToEnd,
            "out-of-range → ToEnd"
        );
    }

    /// `ESC ] 0 ; title ST` → SetWindowTitle
    #[test]
    fn osc_set_window_title_routes() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        // "ESC ] 0 ; Hello BEL"
        for b in b"\x1b]0;Hello\x07" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(rec.borrow().title, "Hello");
    }

    #[test]
    fn osc_set_cwd_routes() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b]7;file:///tmp\x07" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(rec.borrow().cwd, "file:///tmp");
    }

    #[test]
    fn osc_set_color_table_index_rgb_spec() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        // OSC 4 ; 1 ; rgb:ff/00/00 ST → rr=ff gg=0 b=0
        for b in b"\x1b]4;1;rgb:ff/00/00\x07" {
            parser.advance(&mut eng, *b);
        }
        let calls = rec.borrow().set_color_calls.clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 1, "table index 1");
        // packed 0x00BBGGRR: BB=0 GG=0 RR=ff → 0x000000ff
        assert_eq!(calls[0].1, 0x0000_00ff, "BB=0 GG=0 RR=ff");
    }

    #[test]
    fn osc_xterm_color_resource_10_routes() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        // OSC 10 ; #aabbcc ST → rr=aa gg=bb b=cc
        for b in b"\x1b]10;#aabbcc\x07" {
            parser.advance(&mut eng, *b);
        }
        let calls = rec.borrow().set_xterm_calls.clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 10, "resource 10");
        // packed 0x00BBGGRR: BB=cc GG=bb RR=aa → 0x00cc_bb_aa
        assert_eq!(calls[0].1, 0x00_cc_bb_aa, "BB=cc GG=bb RR=aa");
    }

    #[test]
    fn parse_xterm_color_rgb_format() {
        // rgb:ff/00/ff → rr=ff gg=0 bb=ff → 0x00ff_00_ff
        assert_eq!(
            parse_xterm_color("rgb:ff/00/ff"),
            Some(0x00_ff_00_ff),
            "BB=ff GG=0 RR=ff"
        );
    }

    #[test]
    fn parse_xterm_color_hex_format() {
        // #rrggbb → rr=11 gg=22 bb=33 → packed 0x00BBGGRR = 0x00_33_22_11
        assert_eq!(
            parse_xterm_color("#112233"),
            Some(0x00_33_22_11),
            "BB=33 GG=22 RR=11"
        );
    }

    #[test]
    fn parse_xterm_color_inquiry_returns_special() {
        assert_eq!(parse_xterm_color("?"), Some(COLOR_INQUIRY_COLOR));
    }

    #[test]
    fn parse_xterm_color_invalid_returns_none() {
        assert_eq!(parse_xterm_color("notacolor"), None);
    }

    // ===== E2E: vte parser → OutputStateMachineEngine → real TerminalDispatch =====
    // Phase 4.x Wave 2 sub-task 1: verify wiring land. Feed `"hello\nworld"` qua
    // `vte::Parser` + `OutputStateMachineEngine` (wrapping real `TerminalDispatch`
    // + dummy `InteractDispatch` host) → assert rows[0]="hello", rows[1]="world".
    use crate::buffer::TextBuffer;
    use crate::dispatch::interact::InteractDispatch;
    use crate::dispatch::td::TerminalDispatch;

    /// Dummy `InteractDispatch` host — mọi method no-op (W1 không cần host
    /// bridge cho e2e text path).
    struct NoopHost;
    impl InteractDispatch for NoopHost {}

    /// `Rc<RefCell<TerminalDispatch>>` wrapper impl `ITermDispatch` — forward
    /// mọi call vào inner, giữ shared handle để test đọc rows sau khi parser
    /// chạy. `OutputStateMachineEngine` owns `Box<dyn ITermDispatch>` (move),
    /// nên cần indirection layer này thay vì raw `Box<TerminalDispatch>`.
    ///
    /// Chỉ forward những method mà `vte::Perform` callback sẽ gọi cho input
    /// `"hello\nworld"`: `print`, `line_feed`, `carriage_return`. Rest dùng
    /// trait default body (`todo!()`) — không trigger vì engine không gọi chúng
    /// cho input này.
    struct DispatchHandle(Rc<RefCell<TerminalDispatch>>);

    impl ITermDispatch for DispatchHandle {
        fn print(&mut self, c: char) {
            self.0.borrow_mut().print(c);
        }
        fn line_feed(&mut self, lft: crate::dispatch::LineFeedType) {
            self.0.borrow_mut().line_feed(lft);
        }
        fn carriage_return(&mut self) {
            self.0.borrow_mut().carriage_return();
        }
    }

    /// Đọc text từ `Row.glyphs` — chỉ lấy char.
    fn row_text(buf: &TextBuffer, idx: usize) -> String {
        buf.rows()[idx].glyphs.iter().map(|(c, _)| *c).collect()
    }

    #[test]
    fn e2e_hello_world_parses_into_two_rows() {
        // Shell convention: each line terminated `\r\n` (CRLF). WT line_feed
        // không auto-CR nếu LNM off (default VT semantics) → feed `"hello\r\nworld"`.
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let inner = Rc::new(RefCell::new(TerminalDispatch::new(
            terminal,
            Box::new(NoopHost),
        )));
        let mut eng = OutputStateMachineEngine::new(Box::new(DispatchHandle(inner.clone())));
        let mut parser = vte::Parser::new();
        for b in b"hello\r\nworld" {
            parser.advance(&mut eng, *b);
        }
        let buf = &inner.borrow().terminal;
        assert_eq!(row_text(buf, 0), "hello", "row 0");
        assert_eq!(row_text(buf, 1), "world", "row 1");
    }

    /// `CSI Ps b` (REP) lặp `last_printed_char` `Ps` lần qua `PrintString`.
    /// Port `OutputStateMachineEngine.cpp:580-595`. Feed `"abc"` → `CSI 5 b`
    /// → `print_string("ccccc")`. `last_printed_char = 'c'` (char cuối run).
    #[test]
    fn rep_repeats_last_printed_char() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"abc" {
            parser.advance(&mut eng, *b);
        }
        for b in b"\x1b[5b" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(rec.borrow().printed, "abc");
        assert_eq!(rec.borrow().print_string_calls, vec!["ccccc"]);
    }

    /// REP omitted param → `operator VTInt()` clamp 1 (`DispatchTypes.hpp:152`).
    /// `CSI b` (no param) → repeat once. `last_printed_char = 'x'`.
    #[test]
    fn rep_omitted_param_defaults_to_one() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"x\x1b[b" {
            parser.advance(&mut eng, *b);
        }
        assert_eq!(rec.borrow().print_string_calls, vec!["x"]);
    }

    /// REP khi `last_printed_char == NUL` (chưa print gì) → no-op. Port
    /// `_lastPrintedChar != AsciiChars::NUL` guard (`:587`).
    #[test]
    fn rep_no_op_when_no_prior_print() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"\x1b[3b" {
            parser.advance(&mut eng, *b);
        }
        assert!(rec.borrow().print_string_calls.is_empty());
        assert!(rec.borrow().printed.is_empty());
    }

    /// `_ClearLastChar` gọi ở cuối mỗi dispatch action: sau `"abc"` + CR +
    /// `CSI 2 b` → `last_printed_char` đã bị CR (C0 `execute()`) clear →
    /// REP no-op. Port `ActionExecute:105` `_ClearLastChar()`.
    #[test]
    fn dispatch_action_clears_last_printed_char() {
        let rec = Rc::new(RefCell::new(CallRec::default()));
        let mut eng = OutputStateMachineEngine::new(Box::new(RecordingDispatch(Rc::clone(&rec))));
        let mut parser = vte::Parser::new();
        for b in b"abc\r" {
            parser.advance(&mut eng, *b);
        }
        for b in b"\x1b[2b" {
            parser.advance(&mut eng, *b);
        }
        // CR là C0 → `execute()` → `clear_last_char()` → REP no-op.
        assert!(rec.borrow().print_string_calls.is_empty());
        assert_eq!(rec.borrow().printed, "abc");
    }
}
