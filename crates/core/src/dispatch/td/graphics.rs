//! `td/graphics.rs` — SGR + line/char-protection + push/pop.
//!
//! Port verbatim `windows/src/terminal/adapter/adaptDispatchGraphics.cpp:1-481`
//! + `adaptDispatch.cpp:1355-1369` (`SetLineRendition`).
//!
//! Anchors:
//! - `adaptDispatchGraphics.cpp:22-30`  — `_SetUnderlineStyleHelper`.
//! - `adaptDispatchGraphics.cpp:43-84`  — `_SetRgbColorsHelper` (38;2/5;48;2/5).
//! - `adaptDispatchGraphics.cpp:175-364` — `_ApplyGraphicsOption` (full SGR table).
//! - `adaptDispatchGraphics.cpp:404-411` — `_ApplyGraphicsOptions` (for-loop).
//! - `adaptDispatchGraphics.cpp:421-427` — `SetGraphicsRendition`.
//! - `adaptDispatchGraphics.cpp:435-456` — `SetCharacterProtectionAttribute` (DECSCA).
//! - `adaptDispatchGraphics.cpp:464-480` — `PushGraphicsRendition` / `PopGraphicsRendition`.
//! - `adaptDispatch.cpp:1355-1369`       — `SetLineRendition` (DECSWL/DECDWL/DECDHL).
//!
//! Port-side deviation: WT `_ApplyGraphicsOption` used `VTParameters::at(i).value_or(0)`
//! returning `GraphicsOptions` enum directly. Rust port pass `&[i32]` slice →
//! convert via `GraphicsOptions::from_i32`. Sub-params (SGR 4:x, 38:2:cs:r:g:b)
//! chưa port (Wave sau) — chỉ flat params path.

use super::TerminalDispatch;
use crate::attrs::{TextAttribute, UnderlineStyle};
use crate::dispatch::types::{GraphicsOptions, LogicalAttributeOptions};
use crate::dispatch::{VtParameter, VtParameters};

/// `_SetUnderlineStyleHelper(option, attr)` (`adaptDispatchGraphics.cpp:22-30`).
///
/// Apply underline style 0..=5 nếu trong range (`UnderlineStyle::Max`).
pub fn set_underline_style_helper(option: VtParameter, attr: &mut TextAttribute) {
    let style = option;
    if (0..=UnderlineStyle::DashedUnderlined as i32).contains(&style) {
        // SAFETY: `style` is in 0..=5, all valid discriminants of `UnderlineStyle`
        // which is `#[repr(u8)]`. Cast `i32 -> u8` is lossless within this range.
        let safe_u8 = style as u8;
        attr.set_underline_style(unsafe { core::mem::transmute::<u8, UnderlineStyle>(safe_u8) });
    }
}

/// `_SetRgbColorsHelper(options, attr, isForeground)` (`:43-84`).
///
/// Parse `38;2;r;g;b` / `48;2;r;g;b` (truecolor) và `38;5;n` / `48;5;n`
/// (256-index). Trả số param consumed (không tính marker 38/48).
pub fn set_rgb_colors_helper(
    options: VtParameters,
    attr: &mut TextAttribute,
    is_foreground: bool,
) -> usize {
    let mut consumed = 1usize;
    let type_opt = GraphicsOptions::from_i32(options.first().copied().unwrap_or(0));
    match type_opt {
        GraphicsOptions::RGBColorOrFaint => {
            consumed = 4;
            let red = options.get(1).copied().unwrap_or(0) as usize;
            let green = options.get(2).copied().unwrap_or(0) as usize;
            let blue = options.get(3).copied().unwrap_or(0) as usize;
            // Match XTerm/VTE: chỉ apply nếu RGB <= 255.
            if red <= 255 && green <= 255 && blue <= 255 {
                let rgb = pack_rgb(red, green, blue);
                attr.set_color(rgb, is_foreground);
            }
        }
        GraphicsOptions::BlinkOrXterm256Index => {
            consumed = 2;
            let table_index = options.get(1).copied().unwrap_or(0) as usize;
            if table_index <= 255 {
                let idx = table_index as u8;
                if is_foreground {
                    attr.set_indexed_foreground_256(idx);
                } else {
                    attr.set_indexed_background_256(idx);
                }
            }
        }
        _ => {}
    }
    consumed
}

/// `_ApplyGraphicsOption(options, i, attr)` (`:175-364`).
///
/// Apply 1 SGR option. Trả số entries consumed (1, hoặc 1+helper cho 38/48).
pub fn apply_graphics_option(
    options: VtParameters,
    option_index: usize,
    attr: &mut TextAttribute,
) -> usize {
    let raw = options.get(option_index).copied().unwrap_or(0);
    let opt = GraphicsOptions::from_i32(raw);
    use GraphicsOptions::*;
    match opt {
        Off => {
            attr.set_default_foreground();
            attr.set_default_background();
            attr.set_default_underline_color();
            attr.set_default_rendition_attributes();
            1
        }
        ForegroundDefault => {
            attr.set_default_foreground();
            1
        }
        BackgroundDefault => {
            attr.set_default_background();
            1
        }
        UnderlineColorDefault => {
            attr.set_default_underline_color();
            1
        }
        Intense => {
            attr.set_intense(true);
            1
        }
        RGBColorOrFaint => {
            attr.set_faint(true);
            1
        }
        NotIntenseOrFaint => {
            attr.set_intense(false);
            attr.set_faint(false);
            1
        }
        Italics => {
            attr.set_italic(true);
            1
        }
        NotItalics => {
            attr.set_italic(false);
            1
        }
        BlinkOrXterm256Index | RapidBlink => {
            // RapidBlink = alias of Blink (`adaptDispatchGraphics.cpp:222`).
            attr.set_blinking(true);
            1
        }
        Steady => {
            attr.set_blinking(false);
            1
        }
        Invisible => {
            attr.set_invisible(true);
            1
        }
        Visible => {
            attr.set_invisible(false);
            1
        }
        CrossedOut => {
            attr.set_crossed_out(true);
            1
        }
        NotCrossedOut => {
            attr.set_crossed_out(false);
            1
        }
        Negative => {
            attr.set_reverse_video(true);
            1
        }
        Positive => {
            attr.set_reverse_video(false);
            1
        }
        Underline => {
            attr.set_underline_style(UnderlineStyle::SinglyUnderlined);
            1
        }
        DoublyUnderlined => {
            attr.set_underline_style(UnderlineStyle::DoublyUnderlined);
            1
        }
        NoUnderline => {
            attr.set_underline_style(UnderlineStyle::NoUnderline);
            1
        }
        Overline => {
            attr.set_overlined(true);
            1
        }
        NoOverline => {
            attr.set_overlined(false);
            1
        }
        ForegroundBlack => {
            attr.set_indexed_foreground(DARK_BLACK);
            1
        }
        ForegroundRed => {
            attr.set_indexed_foreground(DARK_RED);
            1
        }
        ForegroundGreen => {
            attr.set_indexed_foreground(DARK_GREEN);
            1
        }
        ForegroundYellow => {
            attr.set_indexed_foreground(DARK_YELLOW);
            1
        }
        ForegroundBlue => {
            attr.set_indexed_foreground(DARK_BLUE);
            1
        }
        ForegroundMagenta => {
            attr.set_indexed_foreground(DARK_MAGENTA);
            1
        }
        ForegroundCyan => {
            attr.set_indexed_foreground(DARK_CYAN);
            1
        }
        ForegroundWhite => {
            attr.set_indexed_foreground(DARK_WHITE);
            1
        }
        BackgroundBlack => {
            attr.set_indexed_background(DARK_BLACK);
            1
        }
        BackgroundRed => {
            attr.set_indexed_background(DARK_RED);
            1
        }
        BackgroundGreen => {
            attr.set_indexed_background(DARK_GREEN);
            1
        }
        BackgroundYellow => {
            attr.set_indexed_background(DARK_YELLOW);
            1
        }
        BackgroundBlue => {
            attr.set_indexed_background(DARK_BLUE);
            1
        }
        BackgroundMagenta => {
            attr.set_indexed_background(DARK_MAGENTA);
            1
        }
        BackgroundCyan => {
            attr.set_indexed_background(DARK_CYAN);
            1
        }
        BackgroundWhite => {
            attr.set_indexed_background(DARK_WHITE);
            1
        }
        BrightForegroundBlack => {
            attr.set_indexed_foreground(BRIGHT_BLACK);
            1
        }
        BrightForegroundRed => {
            attr.set_indexed_foreground(BRIGHT_RED);
            1
        }
        BrightForegroundGreen => {
            attr.set_indexed_foreground(BRIGHT_GREEN);
            1
        }
        BrightForegroundYellow => {
            attr.set_indexed_foreground(BRIGHT_YELLOW);
            1
        }
        BrightForegroundBlue => {
            attr.set_indexed_foreground(BRIGHT_BLUE);
            1
        }
        BrightForegroundMagenta => {
            attr.set_indexed_foreground(BRIGHT_MAGENTA);
            1
        }
        BrightForegroundCyan => {
            attr.set_indexed_foreground(BRIGHT_CYAN);
            1
        }
        BrightForegroundWhite => {
            attr.set_indexed_foreground(BRIGHT_WHITE);
            1
        }
        BrightBackgroundBlack => {
            attr.set_indexed_background(BRIGHT_BLACK);
            1
        }
        BrightBackgroundRed => {
            attr.set_indexed_background(BRIGHT_RED);
            1
        }
        BrightBackgroundGreen => {
            attr.set_indexed_background(BRIGHT_GREEN);
            1
        }
        BrightBackgroundYellow => {
            attr.set_indexed_background(BRIGHT_YELLOW);
            1
        }
        BrightBackgroundBlue => {
            attr.set_indexed_background(BRIGHT_BLUE);
            1
        }
        BrightBackgroundMagenta => {
            attr.set_indexed_background(BRIGHT_MAGENTA);
            1
        }
        BrightBackgroundCyan => {
            attr.set_indexed_background(BRIGHT_CYAN);
            1
        }
        BrightBackgroundWhite => {
            attr.set_indexed_background(BRIGHT_WHITE);
            1
        }
        ForegroundExtended => {
            // C++ `1 + _SetRgbColorsHelper(options.subspan(option_index + 1), attr, true)`.
            let rest = options_rest_from(options, option_index + 1);
            1 + set_rgb_colors_helper(rest, attr, true)
        }
        BackgroundExtended => {
            let rest = options_rest_from(options, option_index + 1);
            1 + set_rgb_colors_helper(rest, attr, false)
        }
        // `UnderlineColor` (`SGR 58`) và các code không nhận diện → skip 1.
        _ => 1,
    }
}

/// `_ApplyGraphicsOptions(options, attr)` (`:404-411`) — for-loop driver.
fn apply_graphics_options(options: VtParameters, attr: &mut TextAttribute) {
    let mut i = 0usize;
    while i < options.len() {
        i += apply_graphics_option(options, i, attr);
    }
}

/// `SetGraphicsRendition(options)` (`adaptDispatchGraphics.cpp:421-427`).
pub fn set_graphics_rendition(this: &mut TerminalDispatch, options: VtParameters) {
    let mut attr = *this.terminal.get_current_attributes();
    apply_graphics_options(options, &mut attr);
    this.terminal.set_current_attributes(attr);
}

/// `SetCharacterProtectionAttribute(options)` (`:435-456`) — DECSCA.
pub fn set_character_protection_attribute(this: &mut TerminalDispatch, options: VtParameters) {
    let mut attr = *this.terminal.get_current_attributes();
    for opt in options.iter().copied() {
        match LogicalAttributeOptions::from_i32(opt) {
            LogicalAttributeOptions::Default | LogicalAttributeOptions::Unprotected => {
                attr.set_protected(false)
            }
            LogicalAttributeOptions::Protected => attr.set_protected(true),
        }
    }
    this.terminal.set_current_attributes(attr);
}

/// `PushGraphicsRendition(options)` (`:464-468`).
pub fn push_graphics_rendition(this: &mut TerminalDispatch, options: VtParameters) {
    let current = *this.terminal.get_current_attributes();
    let save_opts: Vec<_> = options
        .iter()
        .map(|&v| crate::sgr_stack::SgrSaveOptions::from_i32(v))
        .collect();
    this.sgr_stack.push(&current, &save_opts);
}

/// `PopGraphicsRendition()` (`:475-480`).
pub fn pop_graphics_rendition(this: &mut TerminalDispatch) {
    let current = *this.terminal.get_current_attributes();
    let restored = this.sgr_stack.pop(&current);
    this.terminal.set_current_attributes(restored);
}

/// `SetLineRendition(rendition)` (`adaptDispatch.cpp:1355-1369`).
///
/// DECSWL/DECDWL/DECDHL — chỉ apply khi `!modes.test(Mode::AllowDECSLRM)`
/// (line rendition không đổi được nếu LR margins được allow). Reset
/// `delay_eol_wrap` của cursor sau khi set.
pub fn set_line_rendition(this: &mut TerminalDispatch, rendition: crate::attrs::LineRendition) {
    use super::Mode;
    if this.modes.test(Mode::AllowDECSLRM) {
        return;
    }
    let erase_attr = super::erase::get_erase_attributes(this);
    this.terminal
        .set_current_line_rendition(rendition, &erase_attr);
    this.terminal.get_cursor_mut().reset_delay_eol_wrap();
}

// — Indexed color 16-table constants — port `TextColor::DARK_*` / `BRIGHT_*`.
// WT dùng legacy 16-color table indices 0..15 (xterm spec). Phase 1: hardcode
// match WT (`TextColor.hpp`).
const DARK_BLACK: u8 = 0;
const DARK_RED: u8 = 1;
const DARK_GREEN: u8 = 2;
const DARK_YELLOW: u8 = 3;
const DARK_BLUE: u8 = 4;
const DARK_MAGENTA: u8 = 5;
const DARK_CYAN: u8 = 6;
const DARK_WHITE: u8 = 7;
const BRIGHT_BLACK: u8 = 8;
const BRIGHT_RED: u8 = 9;
const BRIGHT_GREEN: u8 = 10;
const BRIGHT_YELLOW: u8 = 11;
const BRIGHT_BLUE: u8 = 12;
const BRIGHT_MAGENTA: u8 = 13;
const BRIGHT_CYAN: u8 = 14;
const BRIGHT_WHITE: u8 = 15;

/// Pack RGB → packed u32 (Phase 1 dùng 0x00BBGGRR khớp `COLORREF`/ABGR layout
/// của `settings::Color`). Hỗ trợ `_SetRgbColorsHelper` truecolor path.
fn pack_rgb(red: usize, green: usize, blue: usize) -> u32 {
    ((blue as u32) << 16) | ((green as u32) << 8) | (red as u32)
}

/// Trả subslice của `options` bắt đầu từ `start` — port `VTParameters::subspan`.
fn options_rest_from<'a>(options: VtParameters<'a>, start: usize) -> VtParameters<'a> {
    if start >= options.len() {
        &options[options.len()..]
    } else {
        &options[start..]
    }
}
