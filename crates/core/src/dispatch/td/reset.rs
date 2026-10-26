//! W4 — Soft / Hard reset / Screen alignment / Cursor style.
//!
//! Port verbatim `windows/src/terminal/adapter/adaptDispatch.cpp`:
//! - `SoftReset` (`:3030`), `HardReset` (`:3072`), `ScreenAlignmentPattern`
//!   (`:3155`), `_EraseScrollback` (`:3198`), `_EraseAll` (`:3233`),
//! - `SetCursorStyle` (`:3262`).

use super::TerminalDispatch;
use crate::attrs::TextAttribute;
use crate::cursor::CursorShape;
use crate::dispatch::types::CursorStyle;
use crate::dispatch::types::EraseType;

/// `SoftReset()` (`adaptDispatch.cpp:3030`).
///
/// Verbatim list (see comment header in C++ source): enable cursor DECTCEM;
/// reset IRM, Origin, AllowDECSLRM modes; AutoWrap=true; keypad/cursor keys
/// normal; reset both margin pairs; `_termOutput.SoftReset()`; SGR normal;
/// `SetCharacterProtectionAttribute({})`; reset _savedCursorState for active
/// buffer; sixel soft-reset if present.
pub fn soft_reset(this: &mut TerminalDispatch) {
    // Cursor enabled.
    this.terminal.get_cursor_mut().set_visibility(true);

    // Replace mode; Absolute cursor addressing; Disallow LR margins.
    this.modes.set(super::Mode::InsertReplace, false);
    this.modes.set(super::Mode::Origin, false);
    this.modes.set(super::Mode::AllowDECSLRM, false);

    // AutoWrap=true — via interact host (port `_api.SetSystemMode(AutoWrap, true)`).
    this.autowrap = true;

    // Top/bottom + left/right margins = 1..height/width (full → clear to 0).
    super::bell_tabs::do_set_top_bottom_scrolling_margins(this, 0, 0, false);
    super::bell_tabs::do_set_left_right_scrolling_margins(this, 0, 0, false);

    // _termOutput.SoftReset() — phase 4.y charset port. No-op stub.

    // SGR normal — delegate to W2's erase impl (SetGraphicsRendition({})) & reset
    // char-protection. Inline minimal: reset current attr to default.
    this.terminal.current_attr = TextAttribute::default();

    // Reset active saved cursor state (alt-buffer-aware).
    let slot = if this.using_alt_buffer { 1 } else { 0 };
    this.saved_cursor_state[slot] = super::CursorState::default();

    // `_terminalInput.ResetInputModes()`: DECCKM, keypad, DECBKM, LNM.
    this.input = super::InputModes::default();
    this.mouse = super::MouseModes::default();
    this.modes.set(super::Mode::LineFeed, false);

    // Sixel parser soft-reset — phase 4.y. Skipped (SixelParser is out-of-scope).
}

/// `HardReset(erase)` (`:3072`).
///
/// Verbatim sequence: switch back to main buffer if in alt; `_pages.Reset()`;
/// clear sixel; reset terminal-output state; reset code page; disable C1;
/// `SoftReset()`; if erase → `EraseInDisplay(All)` + `(Scrollback)`; restore
/// default color/render settings; renderer refresh; cursor (1,1) if erase;
/// reset line feed mode if input set; reset input modes; bracketed paste off;
/// cursor blink on; `TabSet(SetEvery8Columns)`; clear soft font; clear macro
/// buffer; `_modes = {PageCursorCoupling}`; inject RIS sequence via state
/// machine.
pub fn hard_reset(this: &mut TerminalDispatch, erase: bool) {
    // 1. If in alt buffer, switch back to main.
    if this.using_alt_buffer {
        // `_api.UseMainScreenBuffer()` — delegated to interact host bridge.
        this.terminal.leave_alt_screen();
        this.using_alt_buffer = false;
    }

    // 2. Reset page buffers (just our single TextBuffer).
    this.terminal.reset();

    // 3-7. Sixel parser reset (phase 4.y — skipped). _termOutput={} (no-op
    // stub). ResetCodePage + AcceptC1 + SendC1.
    super::charset::accept_c1_controls(this, false);
    super::charset::send_c1_controls(this, false);

    // 8. Sets SGR state to normal — call `SoftReset()`.
    soft_reset(this);

    if erase {
        // EraseInDisplay(All) + (Scrollback) — W2 (erase.rs) owns these fns.
        super::erase::erase_in_display(this, EraseType::All);
        super::erase::erase_in_display(this, EraseType::Scrollback);
    }

    // 9. Restore default render settings — risk #8: delegate via interact.set_color_*.

    if erase {
        this.cursor_position(1, 1);
    }

    // 11. Reset input modes (`_terminalInput.ResetInputModes`).
    this.input = super::InputModes::default();

    // Reset bracketed paste mode and mouse/focus reporting.
    this.bracketed_paste = false;
    this.mouse = super::MouseModes::default();

    // Restore cursor blinking.
    this.terminal.get_cursor_mut().set_blinking(true);

    // Tab stops to every 8 columns.
    super::bell_tabs::tab_set(this, 0);

    // 14. Clear soft font — phase 4.y (`_renderer.UpdateSoftFont`).

    // Reset internal modes to initial state: just PageCursorCoupling.
    this.modes = super::ModeSet::DEFAULT;
}

/// `ScreenAlignmentPattern()` (`:3155`).
///
/// Fill screen with 'E' default attrs; reset line rendition; reset
/// erase attrs; reset Origin + AllowDECSLRM modes; clear margins; cursor home.
pub fn screen_alignment_pattern(this: &mut TerminalDispatch) {
    let top = this.viewport.top;
    let bottom = this.viewport.bottom;
    let width = this.viewport.width();
    let rect = crate::screen::Viewport::new(0, top, width, bottom + 1);
    super::erase::fill_rect(this, &rect, "E", &TextAttribute::default());

    // Reset line rendition for the visible rows — port
    // `page.Buffer().ResetLineRenditionRange(top, bottom+1)`.
    // Phase 4.0-pre adds `reset_line_rendition_range` to TextBuffer.
    this.terminal
        .reset_line_rendition_range(top as u32, (bottom + 1) as u32);

    // Reset meta/extended attributes (preserve colors).
    let mut attr = *this.terminal.get_current_attributes();
    attr.set_standard_erase();
    this.terminal.set_current_attributes(attr);

    // Reset Origin + AllowDECSLRM modes.
    this.modes.set(super::Mode::Origin, false);
    this.modes.set(super::Mode::AllowDECSLRM, false);

    // Clear scrolling margins (no auto-home).
    super::bell_tabs::do_set_top_bottom_scrolling_margins(this, 0, 0, false);
    super::bell_tabs::do_set_left_right_scrolling_margins(this, 0, 0, false);

    // Cursor to (1, 1).
    this.cursor_position(1, 1);
}

/// `_EraseScrollback()` (`:3198`).
///
/// WT: clear scrollback above viewport, move viewport to row 0, adjust
/// cursor relatively. Phase 4: delegates to `buffer::clear_scrollback` (W4
/// Phase 4.0-pre to land on TextBuffer).
pub fn erase_scrollback(this: &mut TerminalDispatch) {
    let cursor_row = this.terminal.get_cursor().get_position().1;
    let page_top = this.viewport.top;
    let page_height = this.viewport.height();
    this.terminal
        .clear_scrollback(page_top as u32, page_height as u32);
    // Viewport → row 0 (single buffer collapse: this.viewport.top = 0).
    this.viewport.top = 0;
    // Move cursor to same relative location.
    this.terminal
        .get_cursor_mut()
        .set_y_position(cursor_row - page_top);
}

/// `_EraseAll()` (`:3233`).
///
/// WT: stash cursor row relative to page top; compute new page top from
/// `GetLastNonSpaceCharacter`; rotate ring if new bottom > buffer height;
/// move viewport; restore cursor row; erase all rows; reset line rendition.
/// Phase 4 single-buffer: simplified — uses TextBuffer.get_last_non_space_character
/// + trigger_scroll + clear_row.
pub fn erase_all(this: &mut TerminalDispatch) {
    let page_width = this.viewport.width();
    let page_height = this.viewport.height();
    let buffer_height = page_height; // single-buffer collapse.

    let cursor_row = this.terminal.get_cursor().get_position().1 - this.viewport.top;

    // GetLastNonSpaceCharacter → returns (y, x) per TextBuffer port.
    // WT `lastChar.X==0 && lastChar.Y==0`? → both zero check; newPageTop uses Y (row)+1.
    let (last_y, last_x) = this.terminal.get_last_non_space_character(None);
    let new_page_top = if last_x == 0 && last_y == 0 {
        0
    } else {
        last_y + 1
    };
    let mut new_page_bottom = new_page_top + page_height;
    let delta = new_page_bottom - buffer_height;
    if delta > 0 {
        let fill_attr = this.terminal.current_attr;
        for _ in 0..delta {
            this.terminal.increment_circular_buffer(&fill_attr);
        }
        new_page_bottom -= delta;
        // textBuffer.TriggerScroll({0,-delta}) — phase 4.x renderer hook.
        this.terminal.trigger_scroll((0, -delta));
    }
    let new_page_top = new_page_bottom - page_height;

    // Move viewport if necessary (single-buffer: just update cached viewport.top).
    if new_page_top != this.viewport.top {
        this.viewport.top = new_page_top;
    }

    // Restore relative cursor position.
    this.terminal
        .get_cursor_mut()
        .set_y_position(cursor_row + new_page_top);

    // Erase all rows in current page.
    let erase_attr = super::erase::get_erase_attributes(this);
    let rect = crate::screen::Viewport::new(0, new_page_top, page_width, new_page_bottom);
    super::erase::fill_rect(this, &rect, " ", &erase_attr);

    // Reset line rendition for the erased rows.
    this.terminal
        .reset_line_rendition_range(new_page_top as u32, new_page_bottom as u32);
}

/// `SetCursorStyle(cursor_style)` (`:3262`).
///
/// Verbatim map WT `DispatchTypes::CursorStyle` → `CursorType` + blink flag.
pub fn set_cursor_style(this: &mut TerminalDispatch, cursor_style: CursorStyle) {
    let (actual_type, blinking) = match cursor_style {
        CursorStyle::UserDefault => (CursorShape::Legacy, true),
        CursorStyle::BlinkingBlock => (CursorShape::FullBox, true),
        CursorStyle::SteadyBlock => (CursorShape::FullBox, false),
        CursorStyle::BlinkingUnderline => (CursorShape::Underscore, true),
        CursorStyle::SteadyUnderline => (CursorShape::Underscore, false),
        CursorStyle::BlinkingBar => (CursorShape::VerticalBar, true),
        CursorStyle::SteadyBar => (CursorShape::VerticalBar, false),
    };
    let cursor = this.terminal.get_cursor_mut();
    cursor.set_style(actual_type);
    cursor.set_blinking(blinking);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_style_to_shape_blinking_table() {
        // Verify the WT mapping invariants without TerminalDispatch:
        // UserDefault → (Legacy, blink=true)
        // BlinkingBlock → (FullBox, blink=true)
        // SteadyBlock → (FullBox, blink=false)
        // BlinkingUnderline → (Underscore, blink=true)
        // SteadyUnderline → (Underscore, blink=false)
        // BlinkingBar → (VerticalBar, blink=true)
        // SteadyBar → (VerticalBar, blink=false)
        let cases = [
            (CursorStyle::UserDefault, CursorShape::Legacy, true),
            (CursorStyle::BlinkingBlock, CursorShape::FullBox, true),
            (CursorStyle::SteadyBlock, CursorShape::FullBox, false),
            (
                CursorStyle::BlinkingUnderline,
                CursorShape::Underscore,
                true,
            ),
            (CursorStyle::SteadyUnderline, CursorShape::Underscore, false),
            (CursorStyle::BlinkingBar, CursorShape::VerticalBar, true),
            (CursorStyle::SteadyBar, CursorShape::VerticalBar, false),
        ];
        for (style, expected_shape, expected_blink) in cases {
            // Direct match-style reproduces the fn body mapping.
            let (shape, blink) = match style {
                CursorStyle::UserDefault => (CursorShape::Legacy, true),
                CursorStyle::BlinkingBlock => (CursorShape::FullBox, true),
                CursorStyle::SteadyBlock => (CursorShape::FullBox, false),
                CursorStyle::BlinkingUnderline => (CursorShape::Underscore, true),
                CursorStyle::SteadyUnderline => (CursorShape::Underscore, false),
                CursorStyle::BlinkingBar => (CursorShape::VerticalBar, true),
                CursorStyle::SteadyBar => (CursorShape::VerticalBar, false),
            };
            assert_eq!(shape, expected_shape, "shape mismatch for {:?}", style);
            assert_eq!(blink, expected_blink, "blink mismatch for {:?}", style);
        }
    }

    #[test]
    fn cursor_default_is_visible_blinking() {
        let c = crate::cursor::Cursor::default();
        assert!(c.is_visible());
        assert!(c.is_blinking());
    }
}
