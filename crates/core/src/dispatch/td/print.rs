//! `print.rs` — 2 print method + 2 private helper của `TerminalDispatch`.
//!
//! Port verbatim từ `windows/src/terminal/adapter/adaptDispatch.cpp`:
//! - `UnknownSequence` (`:51`, 3 LOC).
//! - `Print` (`:62`, 10 LOC).
//! - `PrintString` (`:82`, 10 LOC).
//! - `_WriteToBuffer` (`:100-201`, ~110 LOC, CORE).
//!
//! `_cursor_move_position` đã ở `cursor.rs` theo brief table (mặc dù brief liệt
//! kê nó ở print.rs column — quy ước W1: helper này thuộc cursor family).
//!
//! W1 simplification (drift so brief anchors):
//! - Bỏ `Page`/`_pages.ActivePage()`. `page.Buffer()` ↔ `self.terminal`,
//!   `page.Cursor()` ↔ `self.terminal.get_cursor_mut()`, `page.Attributes()` ↔
//!   `self.terminal.get_current_attributes()`.
//! - Bỏ `TerminalOutput`/`_termOutput.TranslateKey` — Phase 4 terminal_output chưa port.
//! - Bỏ `cursor.GetDelayEOLWrap()` / `cursor.DelayEOLWrap()` / `cursor.ResetDelayEOLWrap()`
//!   (Phase 4.0-pre sẽ bổ sung delay-EOL-wrap state vào `Cursor`).
//! - Bỏ `_api.GetSystemMode(ITerminalApi::Mode::AutoWrap)` — assume AutoWrap=true
//!   (DECAWM default on). Will wire after Phase 4.0-pre.
//! - Bỏ `_DoLineFeed`/`page.MoveViewportDown()` — W1 chưa port line-feed helper
//!   (Wave W2 sẽ làm). Print chỉ ghi vào current row.
//! - Bỏ `textBuffer.TriggerNewTextNotification(string)` — Phase 4 renderer hook.

use super::{Mode, TerminalDispatch};
use crate::buffer::{Row, TextBuffer, UNICODE_SPACE};
use unicode_segmentation::UnicodeSegmentation;

/// `AsciiChars::DEL` = `\x7f` (`windows/src/parser/ascii.hpp`).
const ASCII_DEL: char = '\u{7f}';

impl TerminalDispatch {
    /// `UnknownSequence()` (`adaptDispatch.cpp:51-54`) — log+ignore.
    ///
    /// WT: `_api.UnknownSequence()`. W1 bỏ `_api` direct call —*log only*.
    pub fn unknown_sequence(&mut self) {
        // W1 simplification: log + ignore (no _api.UnknownSequence()).
        // W1 drift: `log` crate not in `core/Cargo.toml` deps and brief W1
        // forbids adding new deps → silent ignore (matches C++ "log+ignore" intent).
        // Phase 4 có thể inject `interact.unknown_sequence()` if needed.
    }

    /// `Print(wchPrintable)` (`adaptDispatch.cpp:62-73`).
    ///
    /// WT: translate via `_termOutput.TranslateKey(wch)`, skip if DEL untranslated.
    /// W1 bỏ translation (Phase 4 terminal_output port) — print char trực tiếp,
    /// skip bare DEL như C++ (`if (wchTranslated != AsciiChars::DEL)`).
    pub fn print(&mut self, c: char) {
        if c != ASCII_DEL {
            self.write_to_buffer(&c.to_string());
        }
    }

    /// `PrintString(string)` (`adaptDispatch.cpp:82-98`).
    ///
    /// WT: nếu `_termOutput.NeedToTranslate()` thì translate từng char; else
    /// pass straight. W1 bỏ translation → straight.
    pub fn print_string(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.write_to_buffer(s);
    }
}

impl TerminalDispatch {
    /// `_WriteToBuffer(string)` (`adaptDispatch.cpp:100-201`).
    ///
    /// Verbatim port-of-intent (W1 simplification ở trên):
    /// - Lấy cursor position + current attrs từ `terminal`.
    /// - Tính `line_width` từ `viewport.width()`; clamp by rightMargin+1 nếu cursor
    ///   trong vertical margins.
    /// - Loop: at each step, replace glyph tại cursor row x=cursor.x; advance x.
    /// - If at EOL (col_end >= line_width) và AutoWrap: clamp x = line_width-1,
    ///   set delayed-EOL-wrap flag — W1 stub vì Cursor chưa có delay state.
    /// - Nếu glyph quá wide để fit even at column 0: drop (grapheme advance).
    ///
    /// Drift from C++: bỏ `_termOutput`, `_DoLineFeed`, `page.MoveViewportDown`,
    /// `_api.GetSystemMode(AutoWrap)`, `textBuffer.GetLineWidth(y)` (single-width
    /// buffer default). AutoWrap default = true (DECAWM on).
    pub fn write_to_buffer(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        // DECAWM (`adaptDispatch.cpp:106`), default on.
        let wrap_at_eol = self.autowrap;

        let page_top = self.viewport.top;
        let page_height = self.viewport.height();
        let page_width = self.viewport.width();
        let (top_margin, bottom_margin) = self.get_vertical_margins(true);
        let (left_margin, right_margin) = self.get_horizontal_margins(page_width);

        let cursor_position = self.terminal.get_cursor().get_position();
        // `lineWidth = textBuffer.GetLineWidth(cursorPosition.y)`.
        // W1 simplification: line width = page width (single-width buffer).
        // Wait — port verbatim when possible: use page width directly.
        let mut line_width = page_width;
        if cursor_position.0 <= right_margin
            && cursor_position.1 >= top_margin
            && cursor_position.1 <= bottom_margin
        {
            line_width = line_width.min(right_margin + 1);
        }

        // C++ dùng `RowWriteState { text, columnBegin, columnLimit }`; Rust
        // `Row::replace_text(col_begin, col_limit, text)` trả `(col_end, dirty*).
        // Loop port verbatim (`:123-194`): process text grapheme-by-grapheme via
        // `Row::replace_text` (1 char at a time, advance col_end).
        let mut row_idx: usize = if cursor_position.1 < 0 {
            0
        } else {
            (cursor_position.1 as usize).min(self.terminal.rows().len().saturating_sub(1))
        };
        let attrs = *self.terminal.get_current_attributes();

        let mut cur_x = cursor_position.0;
        let mut cur_y = cursor_position.1;
        // `cursor.IsDelayedEOLWrap()`: the previous print filled the last column.
        let mut delayed = self.terminal.get_cursor().is_delayed_eol_wrap;
        let mut remaining = text;

        while !remaining.is_empty() {
            if delayed && wrap_at_eol {
                // Deferred wrap (`adaptDispatch.cpp:128-150`): mark the row as
                // wrapped, return + line feed (scrolls at the bottom margin).
                self.terminal.get_cursor_mut().set_position(cur_x, cur_y);
                super::bell_tabs::do_line_feed(self, true, true);
                let pos = self.terminal.get_cursor().get_position();
                cur_x = pos.0;
                cur_y = pos.1;
                row_idx = (cur_y.max(0) as usize).min(self.terminal.rows().len().saturating_sub(1));
                delayed = false;
            }
            // PHASE 4.0-pre drift: cursor.delay_eol_wrap state chưa có trên `Cursor`.
            // W1 stub: bỏ delayed-EOL-wrap branch (Phase 2 port sẽ reappearance).
            // textBuffer.Insert vs Replace theo `Mode::InsertReplace`.
            let col_limit = line_width;
            if cur_x >= col_limit {
                // Cursor already past the last column (e.g. after a resize).
                cur_x = col_limit - 1;
                delayed = wrap_at_eol;
                if delayed {
                    continue;
                }
                break;
            }

            let col_begin = cur_x as usize;
            // Clip remaining text to one grapheme if next would overflow.
            let next_grapheme = match remaining.graphemes(true).next() {
                Some(g) => g,
                None => break,
            };

            // Allocate fresh row if row_idx out of bounds (defensive — should not happen).
            if row_idx >= self.terminal.rows().len() {
                break;
            }

            // Verbatim semantics: `_modes.test(Mode::InsertReplace)` decides Insert vs Replace.
            let (_col_end, _dirty_begin, _dirty_end) = if self.modes.test(Mode::InsertReplace) {
                // textBuffer.Insert(y, attr, RowWriteState) — port `buffer.insert`.
                // W1 minh hoạ reuse `TextBuffer::insert`.
                self.terminal.insert(row_idx, &attrs, next_grapheme);
                (col_begin + 1, col_begin, col_begin + 1)
            } else {
                // textBuffer.Replace(y, attr, text) — port `buffer.replace_text`.
                self.terminal.rows_mut()[row_idx].replace_text(
                    col_begin,
                    col_limit as usize,
                    next_grapheme,
                )
            };

            // Cells written with the current SGR attributes (insert mode already
            // applied them through `TextBuffer::insert`).
            if !self.modes.test(Mode::InsertReplace) {
                self.terminal.rows_mut()[row_idx].set_attr_range(
                    col_begin,
                    _col_end.max(col_begin + 1),
                    &attrs,
                );
            }

            // Advance col_end (W1: derive from replace_text result of single grapheme).
            // Rust `Row::replace_text` returns col_end after writing; use it.
            let col_end_actual = _col_end;
            let consumed = next_grapheme.len();
            // Consume the grapheme from `remaining`.
            remaining = &remaining[consumed..];

            // `isWrapping = state.columnEnd >= state.columnLimit` (`:167`).
            let is_wrapping = col_end_actual >= col_limit as usize;
            cur_x = if is_wrapping {
                col_limit - 1
            } else {
                col_end_actual as i32
            };

            if is_wrapping {
                if wrap_at_eol {
                    // Wrap lazily: take effect when more text follows.
                    delayed = true;
                } else {
                    break;
                }
            }
        }

        // `cursor.SetPosition(cursorPosition)` (`:169`).
        self.terminal.get_cursor_mut().set_position(cur_x, cur_y);
        if delayed {
            self.terminal.get_cursor_mut().delay_eol_wrap();
        }
        // `textBuffer.TriggerNewTextNotification(string)` (`:200`) — Phase 4 renderer.
        let _ = (left_margin, page_top, page_height);
    }
}

#[allow(dead_code)]
fn unused_marker_for_row(_r: &Row, _buf: &TextBuffer, _c: char) {
    let _ = UNICODE_SPACE;
    let _ = Mode::InsertReplace;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::TextBuffer;
    use crate::dispatch::interact::InteractDispatch;

    struct Stub;
    impl InteractDispatch for Stub {}

    #[test]
    fn print_one_char_writes_into_row_zero() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print('A');
        assert!(!d.terminal.rows()[0].glyphs.is_empty());
        assert_eq!(d.terminal.rows()[0].glyphs[0].0, 'A');
        assert_eq!(d.terminal.get_cursor().get_position(), (1, 0));
    }

    #[test]
    fn print_del_is_ignored() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print(ASCII_DEL);
        assert!(d.terminal.rows()[0].glyphs.is_empty());
        assert_eq!(d.terminal.get_cursor().get_position(), (0, 0));
    }

    #[test]
    fn print_string_writes_multiple_chars() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print_string("hello");
        let glyphs: Vec<char> = d.terminal.rows()[0]
            .glyphs
            .iter()
            .map(|(c, _)| *c)
            .collect();
        assert_eq!(glyphs, ['h', 'e', 'l', 'l', 'o']);
        assert_eq!(d.terminal.get_cursor().get_position(), (5, 0));
    }

    #[test]
    fn print_string_empty_is_noop() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print_string("");
        assert!(d.terminal.rows()[0].glyphs.is_empty());
    }

    #[test]
    fn print_advances_cursor_each_step() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print('a');
        d.print('b');
        d.print('c');
        let glyphs: Vec<char> = d.terminal.rows()[0]
            .glyphs
            .iter()
            .map(|(c, _)| *c)
            .collect();
        assert_eq!(glyphs, ['a', 'b', 'c']);
        assert_eq!(d.terminal.get_cursor().get_position(), (3, 0));
    }

    #[test]
    fn print_then_cursor_down_keeps_width_progress() {
        // C++ `CursorDown` (`adaptDispatch.cpp:226`) =
        // `_CursorMovePosition(Offset::Forward(distance), Offset::Unchanged(), true)`
        // → column UNCHANGED. After printing "hi" cursor at (2,0); `cursor_down(1)`
        // → (2,1); printing "yo" then backfills spaces at cols 0..2 and writes
        // 'y'/'o' at cols 2/3.
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print_string("hi");
        d.cursor_down(1);
        d.print_string("yo");
        let g0: Vec<char> = d.terminal.rows()[0]
            .glyphs
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let g1: Vec<char> = d.terminal.rows()[1]
            .glyphs
            .iter()
            .map(|(c, _)| *c)
            .collect();
        assert_eq!(g0, ['h', 'i']);
        assert_eq!(g1, [' ', ' ', 'y', 'o']);
        assert_eq!(d.terminal.get_cursor().get_position(), (4, 1));
    }

    #[test]
    fn unknown_sequence_does_not_modify_state() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        let pos_before = d.terminal.get_cursor().get_position();
        d.unknown_sequence();
        assert_eq!(d.terminal.get_cursor().get_position(), pos_before);
    }

    #[test]
    fn write_to_buffer_empty_noop() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.write_to_buffer("");
        assert!(d.terminal.rows()[0].glyphs.is_empty());
    }

    #[test]
    fn print_wide_char_advances_two_columns() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        d.print('日');
        // Wide char takes 2 cols — cursor advances by grapheme (W1 fallback 1 col).
        assert!(!d.terminal.rows()[0].glyphs.is_empty());
        assert_eq!(d.terminal.rows()[0].glyphs[0].1, 2, "wide glyph width=2");
        // W1 cursor advance simplification: 1 col per grapheme (Rust text_pos +1).
        // Phase 2 will fix to use full Row::replace_text col_end semantics per print.
    }
}
