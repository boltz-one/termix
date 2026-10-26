//! `cursor.rs` — 14 cursor method của `TerminalDispatch`.
//!
//! Port verbatim từ `windows/src/terminal/adapter/adaptDispatch.cpp`:
//! - CursorUp `:212` (3 LOC).
//! - CursorDown `:226`.
//! - CursorForward `:235`.
//! - CursorBackward `:244`.
//! - CursorNextLine `:254`.
//! - CursorPrevLine `:264`.
//! - `_CursorMovePosition` `:327-402` (private helper, 80 LOC).
//! - CursorHorizontalPositionAbsolute `:408` (CHA / HPA).
//! - VerticalLinePositionAbsolute `:417` (VPA).
//! - HorizontalPositionRelative `:427` (HPR).
//! - VerticalPositionRelative `:437` (VPR).
//! - CursorPosition `:447` (CUP / HVP).
//! - CursorSaveState `:458` (DECSC, ~35 LOC).
//! - CursorRestoreState `:492` (DECRC, ~55 LOC).
//!
//! Drift so brief: brief table liệt kê `_cursor_move_position(Offset,
//! Offset, Direction::Up)` — WT thật dùng `bool clampInMargins` (3rd arg),
//! không có `Direction` enum. Port theo C++ verbatim — `bool` thứ 3.

use super::{Mode, Offset, TerminalDispatch};
use crate::buffer::TextBuffer;

impl TerminalDispatch {
    /// `CursorUp(distance)` (`adaptDispatch.cpp:212`) — CUU.
    ///
    /// WT: `_CursorMovePosition(Offset::Backward(distance), Offset::Unchanged(), true);`
    pub fn cursor_up(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Backward(distance), Offset::Unchanged(), true);
    }

    /// `CursorDown(distance)` (`adaptDispatch.cpp:226`) — CUD.
    pub fn cursor_down(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Forward(distance), Offset::Unchanged(), true);
    }

    /// `CursorForward(distance)` (`adaptDispatch.cpp:235`) — CUF.
    pub fn cursor_forward(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Unchanged(), Offset::Forward(distance), true);
    }

    /// `CursorBackward(distance)` (`adaptDispatch.cpp:244`) — CUB / BS.
    pub fn cursor_backward(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Unchanged(), Offset::Backward(distance), true);
    }

    /// `CursorNextLine(distance)` (`adaptDispatch.cpp:254`) — CNL.
    /// WT: `_CursorMovePosition(Offset::Forward(distance), Offset::Absolute(1), true);`
    pub fn cursor_next_line(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Forward(distance), Offset::Absolute(1), true);
    }

    /// `CursorPrevLine(distance)` (`adaptDispatch.cpp:264`) — CPL.
    pub fn cursor_prev_line(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Backward(distance), Offset::Absolute(1), true);
    }

    /// `CursorHorizontalPositionAbsolute(column)` (`:408`) — CHA / HPA.
    /// WT: `_CursorMovePosition(Offset::Unchanged(), Offset::Absolute(column), false);`
    pub fn cursor_horizontal_position_absolute(&mut self, column: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Unchanged(), Offset::Absolute(column), false);
    }

    /// `VerticalLinePositionAbsolute(line)` (`:417`) — VPA.
    pub fn vertical_line_position_absolute(&mut self, line: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Absolute(line), Offset::Unchanged(), false);
    }

    /// `HorizontalPositionRelative(distance)` (`:427`) — HPR.
    pub fn horizontal_position_relative(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Unchanged(), Offset::Forward(distance), false);
    }

    /// `VerticalPositionRelative(distance)` (`:437`) — VPR.
    pub fn vertical_position_relative(&mut self, distance: crate::dispatch::VtInt) {
        self.cursor_move_position(Offset::Forward(distance), Offset::Unchanged(), false);
    }

    /// `CursorPosition(line, column)` (`:447`) — CUP / HVP.
    pub fn cursor_position(
        &mut self,
        line: crate::dispatch::VtInt,
        column: crate::dispatch::VtInt,
    ) {
        self.cursor_move_position(Offset::Absolute(line), Offset::Absolute(column), false);
    }

    /// `CursorSaveState()` (`:458`) — DECSC. Save cursor vào `_savedCursorState[active]`.
    ///
    /// Verbatim port (`:458-484`):
    /// - Bỏ `Page` abstraction; `terminal.get_cursor().get_position()` thay `page.Cursor().GetPosition()`.
    /// - `viewport.top` thay `page.Top()` (= 0 cho single-page simplification).
    /// - Bỏ `savedCursorState.TermOutput = _termOutput` — Phase 4 terminal_output chưa port.
    pub fn cursor_save_state(&mut self) {
        let cursor_position = self.terminal.get_cursor().get_position();
        // VT cursor row relative to page top. W1 page top = 0 (single-page).
        let page_top = self.viewport.top;
        let mut cursor_y = cursor_position.1 - page_top;
        let mut cursor_x = cursor_position.0;
        // If origin mode relative, subtract horizontal/vertical margins.
        if self.modes.test(Mode::Origin) {
            let (left_margin, _) = self.get_horizontal_margins(self.viewport.width());
            let (top_margin, _) = self.get_vertical_margins(false);
            cursor_x -= left_margin;
            cursor_y -= top_margin;
        }
        // VT is 1-based, so +1.
        let idx = self.using_alt_buffer as usize;
        let saved = &mut self.saved_cursor_state[idx];
        saved.column = cursor_x + 1;
        saved.row = cursor_y + 1;
        saved.page = 1; // W1 single-page.
        saved.is_delayed_eol_wrap = false; // W1 chưa wire delay-EOL-wrap.
        saved.is_origin_mode_relative = self.modes.test(Mode::Origin);
        saved.attributes = *self.terminal.get_current_attributes();
    }

    /// `CursorRestoreState()` (`:492`) — DECRC. Restore từ `_savedCursorState[active]`.
    ///
    /// Verbatim port (`:492-517`):
    /// - Restore origin mode first (since cursor coords may be relative).
    /// - Restore page number — W1 single-page: no-op (`PagePositionAbsolute(1)` = no-op).
    /// - Restore position with CUP semantics.
    /// - Restore delayed EOL wrap flag (W1 stub: no-op, Phase 2 wire).
    /// - Restore text attributes (calls `set_current_attributes`).
    /// - Restore designated char sets — W1 bỏ (`_termOutput.RestoreFrom` không có port).
    pub fn cursor_restore_state(&mut self) {
        let idx = self.using_alt_buffer as usize;
        let saved = self.saved_cursor_state[idx];

        // 1. Restore origin mode first (`:497`).
        self.modes.set(Mode::Origin, saved.is_origin_mode_relative);

        // 2. Restore page number — W1 single-page, no-op (PagePositionAbsolute(1)).
        // 3. Restore position with CUP semantics (`:503`).
        self.cursor_position(saved.row, saved.column);

        // 4. Delayed EOL wrap flag.
        if saved.is_delayed_eol_wrap {
            // `page.Cursor().DelayEOLWrap()` — W1 chưa wire cursor delay-EOL-wrap
            // state (Phase 4.0-pre fill — does not exist yet on `Cursor`).
            // TODO: Phase 4.0-pre not landed — wire after Cursor::delay_eol_wrap
            // state port complete.
            self.terminal.get_cursor_mut().delay_eol_wrap();
        }

        // 5. Restore text attributes (`:513`).
        self.terminal.set_current_attributes(saved.attributes);
        // 6. Restore designated char sets (`:516`) — Phase 4 terminal_output port.
    }
}

impl TerminalDispatch {
    /// `_CursorMovePosition(rowOffset, colOffset, clampInMargins)` (`adaptDispatch.cpp:327-402`).
    ///
    /// Verbatim port — third arg là `bool clampInMargins` (C++), không phải `Direction`
    /// (brief nói `Direction::Up` — drift, port theo C++).
    ///
    /// W1 simplification: bỏ `Page`/`_pages.ActivePage()`. `page.Top()` = `viewport.top`
    /// (0 cho single-page). `page.Width()` = `viewport.width()`. `page.Bottom()-1` =
    /// `viewport.bottom`. `textBuffer.ClampPositionWithinLine` cần Phase 4.0-pre
    /// worker; nếu chưa land → `todo!()` (Phase 4.0-pre drift OK).
    pub fn cursor_move_position(
        &mut self,
        row_offset: Offset,
        col_offset: Offset,
        clamp_in_margins: bool,
    ) {
        // First retrieve some information about the buffer (`:329-335`).
        let page_top = self.viewport.top;
        let page_bottom_excl = self.viewport.bottom; // inclusive: viewport.bottom; C++ uses `page.Bottom() - 1`.
        let page_width = self.viewport.width();
        let cursor_position = self.terminal.get_cursor().get_position();
        let (top_margin, bottom_margin) = self.get_vertical_margins(true);
        let (left_margin, right_margin) = self.get_horizontal_margins(page_width);

        // For relative movement, offsets are relative to current cursor.
        let mut row = cursor_position.1;
        let mut col = cursor_position.0;

        // If absolute, row is relative to page top or top margin, depending on origin mode.
        if row_offset.is_absolute {
            row = if self.modes.test(Mode::Origin) {
                top_margin
            } else {
                page_top
            };
        }
        // Same for column.
        if col_offset.is_absolute {
            col = if self.modes.test(Mode::Origin) {
                left_margin
            } else {
                0
            };
        }

        // Apply offsets and clamp (`:360-361`).
        row = (row + row_offset.value).clamp(page_top, page_bottom_excl);
        col = (col + col_offset.value).clamp(0, page_width - 1);

        // If clamping in margins, or origin mode is relative, may need further adjust.
        if clamp_in_margins || self.modes.test(Mode::Origin) {
            // Vertical margins only apply if original position is inside horizontal margins.
            if cursor_position.0 >= left_margin && cursor_position.0 <= right_margin {
                if cursor_position.1 >= top_margin {
                    row = row.max(top_margin);
                }
                if cursor_position.1 <= bottom_margin {
                    row = row.min(bottom_margin);
                }
            }
            // Horizontal margins only apply if new row is inside vertical margins.
            if row >= top_margin && row <= bottom_margin {
                if cursor_position.0 >= left_margin {
                    col = col.max(left_margin);
                }
                if cursor_position.0 <= right_margin {
                    col = col.min(right_margin);
                }
            }
        }

        // Finally, set the adjusted cursor position back into the console.
        // `textBuffer.ClampPositionWithinLine({col, row})` (`:401`) — Phase 4.0-pre.
        let clamped = clamp_position_within_line(&self.terminal, (col, row));
        self.terminal
            .get_cursor_mut()
            .set_position(clamped.0, clamped.1);
    }
}

/// `TextBuffer::ClampPositionWithinLine(pos)` — port `textBuffer.cpp:967`.
///
/// Phase 4.0-pre chưa land → inline fallback: clamp x vào [0, width-1],
/// y unchanged. Khi Phase 4.0-pre land signature drift →
/// `// REWORK 4.0-pre: signature drift` ở call site.
///
/// Plan §Phase 4.0-pre: `clamp_position_within_line(pos: (i32, i32)) -> (i32, i32)`.
/// ASSUMED signature from `plans/phase4-terminal-dispatch.md:41`.
///
/// W4 worker (bell_tabs) calls `super::cursor::clamp_position_within_line(&buffer, pos)`
/// with `buffer: &TextBuffer`. Promoted to `pub(crate)` so siblings in `td/` can reach it.
pub(crate) fn clamp_position_within_line(_buffer: &TextBuffer, pos: (i32, i32)) -> (i32, i32) {
    // TODO: Phase 4.0-pre not landed — wire to `TextBuffer::clamp_position_within_line`
    // once available. Fallback: no clamp beyond what `cursor_move_position` already did.
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::TextAttribute;
    use crate::buffer::TextBuffer;
    use crate::dispatch::interact::InteractDispatch;

    struct Stub;
    impl InteractDispatch for Stub {}

    fn make_terminal(w: u16, h: u16) -> TextBuffer {
        TextBuffer::new(w, h, TextAttribute::default())
    }

    #[test]
    fn cursor_up_moves_y_backward_clamped_to_viewport() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(10, 10);
        d.cursor_up(5);
        assert_eq!(d.terminal.get_cursor().get_position(), (10, 5));
        // Clamp at top.
        d.cursor_up(100);
        assert_eq!(d.terminal.get_cursor().get_position(), (10, 0));
    }

    #[test]
    fn cursor_down_moves_y_forward_clamped_to_viewport() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(5, 5);
        d.cursor_down(10);
        assert_eq!(d.terminal.get_cursor().get_position(), (5, 15));
        d.cursor_down(100);
        assert_eq!(d.terminal.get_cursor().get_position(), (5, 23));
    }

    #[test]
    fn cursor_forward_moves_x_forward() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(10, 5);
        d.cursor_forward(7);
        assert_eq!(d.terminal.get_cursor().get_position(), (17, 5));
    }

    #[test]
    fn cursor_backward_moves_x_backward_clamped() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(10, 5);
        d.cursor_backward(4);
        assert_eq!(d.terminal.get_cursor().get_position(), (6, 5));
        d.cursor_backward(100);
        assert_eq!(d.terminal.get_cursor().get_position(), (0, 5));
    }

    #[test]
    fn cursor_next_line_down_and_column_one() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(15, 5);
        d.cursor_next_line(3);
        assert_eq!(d.terminal.get_cursor().get_position(), (0, 8));
    }

    #[test]
    fn cursor_prev_line_up_and_column_one() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(15, 5);
        d.cursor_prev_line(2);
        assert_eq!(d.terminal.get_cursor().get_position(), (0, 3));
    }

    #[test]
    fn cursor_horizontal_absolute_clamps_x() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(5, 10);
        // HPA uses 1-based VT coords → column 10 means x=9.
        d.cursor_horizontal_position_absolute(10);
        assert_eq!(d.terminal.get_cursor().get_position(), (9, 10));
        d.cursor_horizontal_position_absolute(1000);
        assert_eq!(d.terminal.get_cursor().get_position(), (79, 10));
    }

    #[test]
    fn vertical_line_absolute_clamps_y() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(5, 10);
        d.vertical_line_position_absolute(15);
        assert_eq!(d.terminal.get_cursor().get_position(), (5, 14));
        d.vertical_line_position_absolute(1000);
        assert_eq!(d.terminal.get_cursor().get_position(), (5, 23));
    }

    #[test]
    fn horizontal_position_relative_moves_x_no_margin() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(5, 10);
        d.horizontal_position_relative(7);
        assert_eq!(d.terminal.get_cursor().get_position(), (12, 10));
    }

    #[test]
    fn vertical_position_relative_moves_y_no_margin() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(5, 10);
        d.vertical_position_relative(3);
        assert_eq!(d.terminal.get_cursor().get_position(), (5, 13));
    }

    #[test]
    fn cursor_position_absolute_uses_vt_1_based() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.cursor_position(5, 10);
        assert_eq!(d.terminal.get_cursor().get_position(), (9, 4));
    }

    #[test]
    fn cursor_save_then_restore_round_trip() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.terminal.get_cursor_mut().set_position(15, 10);
        d.cursor_save_state();
        // Move cursor away, then restore.
        d.terminal.get_cursor_mut().set_position(0, 0);
        d.cursor_restore_state();
        // VT 1-based saved → row=11, col=16 → CUP(11, 16) → (15, 10).
        assert_eq!(d.terminal.get_cursor().get_position(), (15, 10));
    }

    #[test]
    fn cursor_save_state_stores_origin_mode_relative_false_default() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.cursor_save_state();
        assert!(!d.saved_cursor_state[0].is_origin_mode_relative);
    }

    #[test]
    fn cursor_restore_state_restores_origin_mode_flag() {
        let terminal = make_terminal(80, 24);
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.saved_cursor_state[0].is_origin_mode_relative = true;
        d.cursor_restore_state();
        assert!(d.modes.test(Mode::Origin));
    }
}
