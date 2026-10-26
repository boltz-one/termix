//! `TextBuffer` + `Row` — port của `windows/src/buffer/out/textBuffer.hpp`
//! và `Row.hpp`. Phase 2 fill verbatim C++ logic.
//!
//! Row dùng `SmallVec<[(char, u8)]>` để hold glyph + column-width. C++
//! RLE-attr decompress_per_column (Row.cpp:547 `ReplaceAttributes`) ở phase 1
//! hạ xuống 1 attr/row — phase 2 chưa nâng full RLE.
//!
//! Anchors:
//! - `textBuffer.cpp:539` — `Replace`.
//! - `textBuffer.cpp:548` — `Insert`.
//! - `textBuffer.cpp:583` — `FillRect`.
//! - `textBuffer.cpp:728` — `IncrementCircularBuffer`.
//! - `textBuffer.cpp:1038` — `ResizeTraditional`.
//! - `Row.cpp:226` — `Reset`.
//! - `Row.cpp:613` — `ReplaceText` (width-aware).
//! - `Row.hpp:133-134` — `SetWrapForced`/`WasWrapForced`.

use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use crate::attrs::{LineRendition, TextAttribute};
use crate::cursor::Cursor;
use crate::screen::Viewport;

/// `\x20` space — port `UNICODE_SPACE`. Mặc định khi Reset.
pub const UNICODE_SPACE: char = '\x20';

/// Một glyph cell: (char, column-width). `width` = 0..=2 (0 = combining).
pub type GlyphCell = (char, u8);

/// `Row` — port `Row.hpp`. Phase 1: `glyphs: SmallVec<[(char, u8)]>` + 1 attr.
///
/// Phase 2 sẽ nâng RLE-attr decompressed-per-column (`RowAttributes`).
#[derive(Clone, Debug)]
pub struct Row {
    /// Base attribute (`Row.hpp:307`): applies to every cell not covered by a
    /// run in `attr_runs` — in particular the blank tail of the row.
    pub attr: TextAttribute,
    /// Per-cell attributes as `(first column, attribute)` runs, sorted by
    /// column; a run lasts until the next one. Replaces the original
    /// single-attribute-per-row limitation (RLE like WT's `ATTR_ROW`).
    pub attr_runs: Vec<(u16, TextAttribute)>,
    pub line_rendition: LineRendition,
    /// Glyphs + width — port `_charsBuffer` + `_charOffsets` rút gọn.
    pub glyphs: SmallVec<[GlyphCell; 128]>,
    /// `_columnCount` (`Row.hpp:308`).
    pub column_count: u16,
    /// `_wrapForced` (`Row.hpp:313`).
    pub wrap_forced: bool,
    /// `_doubleBytePadded` (`Row.hpp:315`).
    pub double_byte_padded: bool,
    /// `WasTextCleared` — phase 2 mark; phase 1: false.
    pub text_cleared: bool,
}

impl Default for Row {
    fn default() -> Self {
        Self {
            attr: TextAttribute::default(),
            attr_runs: Vec::new(),
            line_rendition: LineRendition::SingleWidth,
            glyphs: SmallVec::new(),
            column_count: 80,
            wrap_forced: false,
            double_byte_padded: false,
            text_cleared: false,
        }
    }
}

impl Row {
    pub fn new(column_count: u16, attr: TextAttribute) -> Self {
        Self {
            attr,
            attr_runs: Vec::new(),
            line_rendition: LineRendition::SingleWidth,
            glyphs: SmallVec::new(),
            column_count,
            wrap_forced: false,
            double_byte_padded: false,
            text_cleared: false,
        }
    }

    /// `Reset(attr)` (`Row.cpp:226`). Fill row với SPACE 1-attr-per-row.
    /// Verbatim semantics: clear glyphs + reset line_rendition + wrap flags.
    pub fn reset(&mut self, attr: TextAttribute) {
        self.glyphs.clear();
        self.attr = attr;
        self.attr_runs.clear();
        self.line_rendition = LineRendition::SingleWidth;
        self.wrap_forced = false;
        self.double_byte_padded = false;
        self.text_cleared = false;
    }

    /// `MeasureRight()` (`Row.cpp:1047`). Trả 1 + index của glyph rightmost
    /// non-space; row rỗng → 0. Port `ContainsText` semantics.
    pub fn measure_right(&self) -> i32 {
        let mut right: i32 = 0;
        for (i, (c, _w)) in self.glyphs.iter().enumerate() {
            if *c != UNICODE_SPACE {
                right = (i + 1) as i32;
            }
        }
        right
    }

    /// `ContainsText()` (`Row.cpp:1062`). Row có ít nhất 1 non-space glyph.
    pub fn contains_text(&self) -> bool {
        self.glyphs.iter().any(|(c, _)| *c != UNICODE_SPACE)
    }

    /// Helper: lay glyph tại `col` (None nếu outside).
    pub fn glyph_at(&self, col: usize) -> Option<(char, u8)> {
        self.glyphs.get(col).copied()
    }

    /// Replace text từ `col_begin` tới max `col_limit`. Trả về
    /// `(col_end, col_begin_dirty, col_end_dirty)` theo Row.cpp:613
    /// `ReplaceText`. Dùng `UnicodeSegmentation` để tách grapheme +
    /// `UnicodeWidthChar` cho width.
    ///
    /// Verbatim semantics (không dùng RLE/char-offset buffer):
    /// - Mỗi grapheme cluster -> 1 glyph cell `(char, width)`.
    /// - Wide glyph (2 cols): lead `(ch, 2)` + trailer `(' ', 0)`, so the cell
    ///   index of every glyph equals its column.
    /// - Nếu col_end vượt col_limit → bỏ qua phần còn lại (state.columnEnd=colLimit).
    pub fn replace_text(
        &mut self,
        col_begin: usize,
        col_limit: usize,
        text: &str,
    ) -> (usize, usize, usize) {
        if col_begin >= col_limit || text.is_empty() {
            return (col_begin, col_begin, col_begin);
        }
        let col_begin_dirty = col_begin;
        let mut col_end = col_begin;
        let mut consumed_bytes = 0usize;
        for grapheme in text.graphemes(true) {
            if col_end >= col_limit {
                break;
            }
            // Width: max(1, width of first char); wide chars cần 2 cols.
            let first_char = grapheme.chars().next().unwrap_or(UNICODE_SPACE);
            let width = UnicodeWidthChar::width(first_char).unwrap_or(0).max(1) as u8;
            let new_col_end = col_end + width as usize;
            if new_col_end > col_limit {
                break;
            }
            // Mở rộng glyphs buffer tới col_end (fill missing với SPACE).
            while self.glyphs.len() < new_col_end.max(col_end + 1) {
                self.glyphs.push((UNICODE_SPACE, 1));
            }
            // Overwriting half of a wide glyph blanks the other half.
            if col_end > 0 && self.glyphs[col_end].1 == 0 {
                self.glyphs[col_end - 1] = (UNICODE_SPACE, 1);
            }
            if self.glyphs[col_end].1 == 2 && col_end + 1 < self.glyphs.len() {
                self.glyphs[col_end + 1] = (UNICODE_SPACE, 1);
            }
            self.glyphs[col_end] = (first_char, width);
            if width == 2 {
                // Explicit trailer: cell index always equals column.
                if self.glyphs[col_end + 1].1 == 2 && col_end + 2 < self.glyphs.len() {
                    self.glyphs[col_end + 2] = (UNICODE_SPACE, 1);
                }
                self.glyphs[col_end + 1] = (UNICODE_SPACE, 0);
            } else if col_end + 1 < self.glyphs.len() && self.glyphs[col_end + 1].1 == 0 && width == 1 {
                // A trailer whose lead we just replaced.
                self.glyphs[col_end + 1] = (UNICODE_SPACE, 1);
            }
            col_end = new_col_end;
            consumed_bytes += grapheme.len();
        }
        let _ = consumed_bytes;
        let col_end_dirty = col_end;
        (col_end, col_begin_dirty, col_end_dirty)
    }

    /// `ReplaceAttributes(begin, end, attr)` (`Row.cpp:547`). Phase 1:
    /// 1-attr/row nên set whole-row attr nếu `begin < end`.
    pub fn replace_attributes(&mut self, begin: usize, end: usize, attr: &TextAttribute) {
        self.set_attr_range(begin, end, attr);
    }

    /// Attribute of the cell at `col`.
    pub fn attr_at(&self, col: usize) -> &TextAttribute {
        self.attr_runs
            .iter()
            .rev()
            .find(|(start, _)| (*start as usize) <= col)
            .map(|(_, a)| a)
            .unwrap_or(&self.attr)
    }

    /// Give cells `[begin, end)` attribute `attr`, keeping the rest as is.
    pub fn set_attr_range(&mut self, begin: usize, end: usize, attr: &TextAttribute) {
        if begin >= end {
            return;
        }
        let after = *self.attr_at(end);
        self.attr_runs.retain(|(s, _)| (*s as usize) < begin || (*s as usize) > end);
        let cap = u16::MAX as usize;
        self.attr_runs.push((begin.min(cap) as u16, *attr));
        if end <= cap {
            self.attr_runs.push((end as u16, after));
        }
        self.attr_runs.sort_by_key(|(s, _)| *s);
        // Merge neighbours with identical attributes, and a leading run equal
        // to the base.
        self.attr_runs.dedup_by(|b, a| a.1 == b.1);
        if self.attr_runs.first().is_some_and(|(s, a)| *s == 0 && *a == self.attr) {
            self.attr_runs.remove(0);
        }
    }

    /// Fill `[begin, end)` (erase): a full-width fill resets the base attribute.
    pub fn fill_attr_range(&mut self, begin: usize, end: usize, attr: &TextAttribute) {
        if begin == 0 && end >= self.column_count as usize {
            self.attr = *attr;
            self.attr_runs.clear();
        } else {
            self.set_attr_range(begin, end, attr);
        }
    }

    /// Copy the attributes of `src` columns `[begin, end)` into this row.
    pub fn copy_attr_range_from(&mut self, src: &Row, begin: usize, end: usize) {
        let mut col = begin;
        while col < end {
            let a = *src.attr_at(col);
            let mut next = col + 1;
            while next < end && *src.attr_at(next) == a {
                next += 1;
            }
            self.set_attr_range(col, next, &a);
            col = next;
        }
    }

    /// `CopyFrom(other)` (`Row.cpp:356`). Copy all fields trừ `_charsHeap`.
    pub fn copy_from(&mut self, other: &Row) {
        self.attr = other.attr;
        self.attr_runs.clone_from(&other.attr_runs);
        self.line_rendition = other.line_rendition;
        self.glyphs.clear();
        self.glyphs.extend_from_slice(&other.glyphs);
        self.column_count = other.column_count;
        self.wrap_forced = other.wrap_forced;
        self.double_byte_padded = other.double_byte_padded;
        self.text_cleared = other.text_cleared;
    }

    /// `WasWrapForced()` (`Row.hpp:134`).
    pub fn was_wrap_forced(&self) -> bool {
        self.wrap_forced
    }

    /// `SetWrapForced(bool)` (`Row.hpp:133`).
    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap_forced = wrap;
    }

    /// `WasTextCleared()` — phase 2 mark for incremental repaint.
    pub fn was_text_cleared(&self) -> bool {
        self.text_cleared
    }

    pub fn clear_text(&mut self) {
        self.glyphs.clear();
        self.text_cleared = true;
    }

    /// Iterator-yield glyphs cho renderer. `Row` không reproduce
    /// `TextBufferCellIterator` (`textBufferCellIterator.hpp:24`) — walk
    /// trực tiếp.
    pub fn glyphs(&self) -> &[(char, u8)] {
        &self.glyphs
    }

    pub fn column_count(&self) -> u16 {
        self.column_count
    }

    pub fn line_rendition(&self) -> LineRendition {
        self.line_rendition
    }

    pub fn set_line_rendition(&mut self, r: LineRendition) {
        self.line_rendition = r;
    }

    /// `IsDoubleWidth()` — port `TextBuffer::IsDoubleWidthLine`.
    pub fn is_double_width(&self) -> bool {
        self.line_rendition.is_double_width()
    }

    /// `attr()` accessor — phase 1 single-attr-per-row.
    pub fn attr(&self) -> &TextAttribute {
        &self.attr
    }

    pub fn attr_mut(&mut self) -> &mut TextAttribute {
        &mut self.attr
    }

    pub fn set_attr(&mut self, attr: TextAttribute) {
        self.attr = attr;
    }
}

/// Cell iterator rút gọn — port `TextBufferCellIterator`. Phase 1: borrow
/// `&[GlyphCell]` từ row; phase 2 làm RLE decompress.
#[derive(Clone, Debug)]
pub struct CellIter<'a> {
    row: &'a Row,
    col: usize,
}

impl<'a> CellIter<'a> {
    pub fn new(row: &'a Row, col: usize) -> Self {
        Self { row, col }
    }
}

impl<'a> Iterator for CellIter<'a> {
    type Item = (char, u8, &'a TextAttribute);
    fn next(&mut self) -> Option<Self::Item> {
        if self.col >= self.row.glyphs.len() {
            return None;
        }
        let (c, w) = self.row.glyphs[self.col];
        self.col += 1;
        Some((c, w, &self.row.attr))
    }
}

/// `TextBuffer` — port `textBuffer.hpp:50`. Phase 1 skeleton.
#[derive(Clone, Debug)]
pub struct TextBuffer {
    /// `Vec<Row>` thay VirtualAlloc (port spec).
    pub rows: Vec<Row>,
    /// `(width, height)` cells — port `_size`.
    pub size: (u16, u16),
    pub cursor: Cursor,
    /// `SetCurrentAttributes` (`textBuffer.hpp:144`).
    pub current_attr: TextAttribute,
    /// Rows scrolled off the top of the page (oldest first).
    pub scrollback: std::collections::VecDeque<Row>,
    /// Maximum rows kept in `scrollback` (`historySize`, WT default 9001).
    pub scrollback_limit: usize,
    /// Main-screen rows parked while the alternate screen is active.
    pub alt_main: Option<Vec<Row>>,
    /// Rows dropped from the front of `scrollback`; with the index in
    /// `scrollback` it gives a row id that stays valid while history trims.
    pub scrollback_dropped: usize,
}

impl TextBuffer {
    /// `TextBuffer(...)` ctor. Phase 1: khởi tạo `Vec<Row::default()>`.
    pub fn new(width: u16, height: u16, attr: TextAttribute) -> Self {
        let row = Row::new(width, attr);
        Self {
            rows: vec![row; height as usize],
            size: (width, height),
            cursor: Cursor::default(),
            current_attr: attr,
            scrollback: std::collections::VecDeque::new(),
            scrollback_limit: 9001,
            scrollback_dropped: 0,
            alt_main: None,
        }
    }

    /// Row by absolute index: scrollback first, then the page.
    pub fn row_abs(&self, abs: usize) -> Option<&Row> {
        let sb = self.scrollback.len();
        if abs < sb {
            self.scrollback.get(abs)
        } else {
            self.rows.get(abs - sb)
        }
    }

    /// Scrollback rows + page rows.
    pub fn total_rows(&self) -> usize {
        self.scrollback.len() + self.rows.len()
    }

    /// Append a row to the scrollback, trimming the oldest beyond the limit.
    pub fn push_history(&mut self, row: Row) {
        // The alternate screen has no history.
        if self.scrollback_limit == 0 || self.alt_main.is_some() {
            return;
        }
        self.scrollback.push_back(row);
        if self.scrollback.len() > self.scrollback_limit {
            self.scrollback.pop_front();
            self.scrollback_dropped += 1;
        }
    }

    /// Switch to a blank alternate screen, parking the main rows
    /// (`UseAlternateScreenBuffer`).
    pub fn enter_alt_screen(&mut self, fill: &TextAttribute) {
        if self.alt_main.is_some() {
            return;
        }
        let blank = vec![Row::new(self.size.0, *fill); self.size.1 as usize];
        self.alt_main = Some(std::mem::replace(&mut self.rows, blank));
    }

    /// Return to the main screen, resized to the current page
    /// (`UseMainScreenBuffer`).
    pub fn leave_alt_screen(&mut self) {
        let Some(mut main) = self.alt_main.take() else { return };
        let (w, h) = (self.size.0, self.size.1 as usize);
        main.truncate(h);
        while main.len() < h {
            main.push(Row::new(w, self.current_attr));
        }
        for r in &mut main {
            r.column_count = w;
        }
        self.rows = main;
    }

    /// Drop all history (`ClearBuffer`/ED 3).
    pub fn clear_history(&mut self) {
        self.scrollback_dropped += self.scrollback.len();
        self.scrollback.clear();
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn rows_mut(&mut self) -> &mut [Row] {
        &mut self.rows
    }

    /// `GetRowByOffset(index)` (`textBuffer.hpp:90`).
    pub fn get_row(&self, index: usize) -> &Row {
        &self.rows[index % self.rows.len()]
    }

    pub fn get_row_mut(&mut self, index: usize) -> &mut Row {
        let len = self.rows.len();
        &mut self.rows[index % len]
    }

    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    /// `GetSize()` (`textBuffer.hpp:135`) → `Viewport`.
    pub fn get_size(&self) -> Viewport {
        Viewport::from_size(self.size.0 as i32, self.size.1 as i32)
    }

    /// `GetTotalBufferSize()` — port spec. Trả `(width, height)`.
    pub fn get_total_buffer_size(&self) -> (i32, i32) {
        (self.size.0 as i32, self.size.1 as i32)
    }

    /// `GetCursor()` (`textBuffer.hpp:129`).
    pub fn get_cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn get_cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    /// `GetCellDataAt(row, col)` → `CellIter`. Phase 1: trực tiếp index.
    pub fn get_cell_data(&self, row: usize, col: usize) -> CellIter<'_> {
        CellIter::new(self.get_row(row), col)
    }

    /// `WriteLine` / `Replace(row, attr, RowWriteState)` (`textBuffer.cpp:539`).
    /// Phase 2 port: replace glyphs tại (row, col) width-aware + set attr.
    pub fn set_text(&mut self, row: usize, col: usize, text: &str) {
        if row >= self.rows.len() {
            return;
        }
        let col_limit = self.size.0 as usize;
        let r = &mut self.rows[row];
        r.replace_text(col, col_limit, text);
        r.replace_attributes(col, col_limit, &self.current_attr);
        self.trigger_redraw(&Viewport::from_size(col_limit as i32, 1));
    }

    /// `ClearRow(row, attr)` — `Row::Reset` (`Row.cpp:226`).
    pub fn clear_row(&mut self, row: usize, attr: TextAttribute) {
        if row >= self.rows.len() {
            return;
        }
        self.rows[row].reset(attr);
    }

    /// `Replace(row, attr, RowWriteState)` (`textBuffer.cpp:539`).
    /// Verbatim: r.ReplaceText(state) + r.ReplaceAttributes() + TriggerRedraw.
    pub fn replace(&mut self, row: usize, attr: &TextAttribute, text: &str) {
        if row >= self.rows.len() {
            return;
        }
        let col_limit = self.size.0 as usize;
        let (col_end, col_begin_dirty, col_end_dirty) =
            self.rows[row].replace_text(0, col_limit, text);
        self.rows[row].replace_attributes(0, col_end, attr);
        self.trigger_redraw(&Viewport::new(
            col_begin_dirty as i32,
            row as i32,
            (col_end_dirty as i32).max(0),
            row as i32,
        ));
    }

    /// `Insert(row, attr, RowWriteState)` (`textBuffer.cpp:548`).
    /// Verbatim semantics: replace text tại col 0, sau đó restore
    /// trailing text từ scratch row.
    pub fn insert(&mut self, row: usize, attr: &TextAttribute, text: &str) {
        if row >= self.rows.len() {
            return;
        }
        let col_limit = self.size.0 as usize;
        // scratch.CopyFrom(r) (`textBuffer.cpp:553`)
        let mut scratch = Row::new(self.size.0, self.rows[row].attr);
        scratch.copy_from(&self.rows[row]);
        // r.ReplaceText(state)
        let (col_end, col_begin_dirty, col_end_dirty) =
            self.rows[row].replace_text(0, col_limit, text);
        self.rows[row].replace_attributes(0, col_end, attr);
        // Restore trailing từ scratch: scratch.GetText(col_end, col_limit)
        let mut restore = Row::new(self.size.0, self.rows[row].attr);
        let trailing_text: String = scratch
            .glyphs
            .iter()
            .skip(col_end)
            .take(col_limit.saturating_sub(col_end))
            .map(|(c, _)| *c)
            .collect();
        if !trailing_text.is_empty() {
            let (re_end, _, re_end_dirty) =
                restore.replace_text(col_end, col_limit, &trailing_text);
            // Trailing cells keep their original attributes.
            self.rows[row].copy_attr_range_from(&scratch, col_end, col_limit);
            // Copy trailing glyphs về sau row chính.
            for (i, glyph) in restore.glyphs.iter().enumerate() {
                if i >= col_end && i < col_limit {
                    while self.rows[row].glyphs.len() <= i {
                        self.rows[row].glyphs.push((UNICODE_SPACE, 1));
                    }
                    self.rows[row].glyphs[i] = *glyph;
                }
            }
            self.trigger_redraw(&Viewport::new(
                col_begin_dirty as i32,
                row as i32,
                re_end_dirty.max(re_end).max(col_end_dirty) as i32,
                row as i32,
            ));
        } else {
            self.trigger_redraw(&Viewport::new(
                col_begin_dirty as i32,
                row as i32,
                col_end_dirty as i32,
                row as i32,
            ));
        }
    }

    /// `FillRect(rect, fill, attr)` (`textBuffer.cpp:583`).
    /// Verbatim: fill scratch row với `fill` nhân bản, copy vào mỗi row
    /// trong `rect.top..rect.bottom`.
    pub fn fill_rect(&mut self, rect: &Viewport, fill: &str, attr: &TextAttribute) {
        if rect.is_empty() || fill.is_empty() {
            return;
        }
        let mut scratch = Row::new(self.size.0, *attr);
        // Fill scratch row consecutive copies of `fill` (`textBuffer.cpp:609`).
        let mut col_end = rect.left as usize;
        while col_end < rect.right as usize {
            let (new_end, _, _) = scratch.replace_text(col_end, rect.right as usize, fill);
            if new_end == col_end {
                break;
            }
            col_end = new_end;
        }
        // Copy scratch vào mỗi row trong rect.
        for y in rect.top..rect.bottom {
            let yi = y as usize;
            if yi >= self.rows.len() {
                break;
            }
            self.rows[yi].copy_from(&scratch);
            self.rows[yi].attr = *attr;
            self.trigger_redraw(&Viewport::new(rect.left, y, rect.right, y));
        }
    }

    /// `IncrementCircularBuffer(fillAttr)` (`textBuffer.cpp:728`).
    /// Verbatim: Reset row[0] thành fillAttr, sau đó xoay `rows` (remove
    /// index 0, push new blank row ở cuối). Phase 2 port ring buffer.
    pub fn increment_circular_buffer(&mut self, fill: &TextAttribute) {
        // textBuffer.cpp:734: GetMutableRowByOffset(0).Reset(fillAttributes)
        let new_row = Row::new(self.size.0, *fill);
        // Xoay ring: replace rows[0] = new, swap xuống cuối.
        let moved = std::mem::replace(&mut self.rows[0], new_row);
        // The row leaving the top becomes history.
        self.push_history(moved);
        // Move rows[0] xuống cuối để ring oldest = rows[1..].
        // C++ `_firstRow++` đã được giản lược thành `rotate_left(1)`.
        self.rows.rotate_left(1);
    }

    /// `ResizeTraditional(newSize)` (`textBuffer.cpp:1038`).
    /// Verbatim: tạo newBuffer với newSize, copy `copyableRows` từ cũ.
    /// Giữ cursor row nếu nằm trong bounds; else clip.
    pub fn resize_traditional(&mut self, new_size: (u16, u16)) {
        // Guard against 0 (`textBuffer.cpp:1041`).
        let new_w = new_size.0.max(1);
        let new_h = new_size.1.max(1);
        let cursor_row = self.cursor.position.1;
        let copyable = (self.size.1 as i32).min(new_h as i32) as usize;
        let mut src_row: i32 = 0;
        // Nếu cursor vượt new_h, src_row = cursor - new_h + 1.
        if cursor_row >= new_h as i32 {
            src_row = cursor_row - new_h as i32 + 1;
        }
        let mut new_rows = Vec::with_capacity(new_h as usize);
        for dst_row in 0..copyable {
            let src_idx = (src_row + dst_row as i32) as usize;
            if src_idx < self.rows.len() {
                let mut r = Row::new(new_w, self.current_attr);
                r.copy_from(&self.rows[src_idx]);
                r.column_count = new_w;
                new_rows.push(r);
            } else {
                new_rows.push(Row::new(new_w, self.current_attr));
            }
        }
        while new_rows.len() < new_h as usize {
            new_rows.push(Row::new(new_w, self.current_attr));
        }
        self.rows = new_rows;
        self.size = (new_w, new_h);
    }

    /// `SetCurrentAttributes(attr)` (`textBuffer.hpp:144`).
    pub fn set_current_attributes(&mut self, attr: TextAttribute) {
        self.current_attr = attr;
    }

    pub fn get_current_attributes(&self) -> &TextAttribute {
        &self.current_attr
    }

    /// `SetWrapForced(y, wrap)` (`textBuffer.hpp:146`).
    pub fn set_wrap_forced(&mut self, y: usize, wrap: bool) {
        if y < self.rows.len() {
            self.rows[y].set_wrap(wrap);
        }
    }

    /// `IsDoubleWidthLine(row)` (`textBuffer.hpp:150`).
    pub fn is_double_width_line(&self, row: usize) -> bool {
        self.rows
            .get(row)
            .map(Row::is_double_width)
            .unwrap_or(false)
    }

    // ===== Phase 4.0-pre — 8 TextBuffer methods port verbatim từ
    // `windows/src/buffer/out/textBuffer.cpp`. W1-foundation assume
    // những method này tồn tại với signature khớp plan. (W1: rework if drift)
    // =====

    /// `GetLineRendition(row)` (`textBuffer.cpp:930`). Trả `LineRendition` của
    /// row tại `row` (modulo buffer height), default `SingleWidth`.
    /// Phase 1: không track row committed, nên luôn trả row's field.
    pub fn get_line_rendition(&self, row: i32) -> LineRendition {
        if row < 0 {
            return LineRendition::SingleWidth;
        }
        let idx = row as usize % self.rows.len();
        self.rows[idx].line_rendition
    }

    /// `GetLineWidth(row)` (`textBuffer.cpp:944`).
    /// Double-width lines chiếm nửa width (shift right 1).
    pub fn get_line_width(&self, row: i32) -> i32 {
        let scale = if self.is_double_width_line(row as usize) {
            1
        } else {
            0
        };
        self.size.0 as i32 >> scale
    }

    /// `ScrollRows(firstRow, size, delta)` (`textBuffer.cpp:806`).
    /// Verbatim port: rotate sub-range `[firstRow, firstRow+size)` theo
    /// `delta` dùng `CopyRow`. Empty slot tự nhiên fill bởi `CopyRow` của
    /// row đã có (cùng semantic như C++ dùng `CopyRow` vào chính nó).
    ///
    /// WT C++ logic (port verbatim):
    /// - delta == 0 → return.
    /// - size < 0 → clamp về 0.
    /// - delta < 0 (lên): walk y = firstRow..firstRow+size (step +1),
    ///   CopyRow(y, y+delta, self).
    /// - delta > 0 (xuống): walk y = firstRow+size-1 down tới firstRow
    ///   (step -1), CopyRow(y, y+delta, self).
    ///
    /// Phase 1 port: `CopyRow` chỉ deep copy `Row` (glyphs + attr + rendition)
    /// — không có `ImageSlice` (phase 2 sẽ add).
    pub fn scroll_rows(&mut self, first_row: i32, size: i32, delta: i32) {
        if delta == 0 {
            return;
        }
        // C++: size = std::max(0, size);
        let size = size.max(0);
        let height = self.rows.len() as i32;
        if first_row < 0 || first_row >= height || size <= 0 {
            return;
        }
        // Defensive: clamp range vào [0, height).
        let end_exclusive = (first_row + size).min(height);
        if first_row >= end_exclusive {
            return;
        }
        // delta < 0: up (step +1). delta > 0: down (step -1).
        // Chúng ta không thể slice_mut để rotate_within trừ khi delta khớp
        // ring range; WT dùng `CopyRow` từng ô → port verbatim.
        if delta < 0 {
            let mut y = first_row;
            while y < end_exclusive {
                let dst = y + delta;
                self.copy_row_in_range(y as u32, dst);
                y += 1;
            }
        } else {
            // delta > 0: walk ngược để tránh overwrite chưa đọc.
            let mut y = end_exclusive - 1;
            while y >= first_row {
                let dst = y + delta;
                self.copy_row_in_range(y as u32, dst);
                y -= 1;
            }
        }
    }

    /// Helper nội bộ — `CopyRow(src, dst, *this)` (`textBuffer.cpp:876`)
    /// với dst trong cùng buffer. Defensive: nếu dst ngoài phạm vi thì bỏ
    /// qua.
    fn copy_row_in_range(&mut self, src_idx: u32, dst_idx: i32) {
        let height = self.rows.len() as i32;
        if dst_idx < 0 || dst_idx >= height {
            return;
        }
        let src = src_idx as usize % self.rows.len();
        let dst = dst_idx as usize;
        if src == dst {
            return;
        }
        let src_row = self.rows[src].clone();
        self.rows[dst].copy_from(&src_row);
    }

    /// `GetLastNonSpaceCharacter(viewOptional)` (`textBuffer.cpp:760`).
    /// Verbatim port: scan từ viewport.bottom ngược lên, tìm row có
    /// non-space char → trả (row, col) = (y, x) với x = MeasureRight() - 1.
    /// Skip trailing empty rows. Defensive: không âm (clamp về 0).
    ///
    /// WT layout (port verbatim):
    /// - view = Some(v) → dùng v. None → toàn buffer (`GetSize()`).
    /// - coordEndOfText.y = min(view.bottom, last_committed_row).
    /// - x = row.MeasureRight() - 1.
    /// - while x < 0 && y > viewport.top: y--, x = new_row.MeasureRight()-1.
    /// - clamp x,y ≥ 0.
    ///
    /// Phase 1: không track committed row → dùng `self.rows.len()-1`.
    /// Trả (row, col) dưới dạng `(i32, i32)`.
    pub fn get_last_non_space_character(&self, view: Option<&Viewport>) -> (i32, i32) {
        let viewport = view.copied().unwrap_or_else(|| self.get_size());
        let last_committed = (self.rows.len() as i32) - 1;
        let mut coord_y = viewport.bottom.min(last_committed);
        let mut coord_x = if coord_y >= 0 {
            self.measure_right_of(coord_y) - 1
        } else {
            -1
        };
        let viewport_top = viewport.top;
        while coord_x < 0 && coord_y > viewport_top {
            coord_y -= 1;
            coord_x = self.measure_right_of(coord_y) - 1;
        }
        (coord_y.max(0), coord_x.max(0))
    }

    /// Helper nội bộ — `MeasureRight()` của row tại `row` (port `Row::MeasureRight`).
    fn measure_right_of(&self, row: i32) -> i32 {
        if row < 0 || row as usize >= self.rows.len() {
            return 0;
        }
        self.rows[row as usize].measure_right()
    }

    /// `ClampPositionWithinLine(position)` (`textBuffer.cpp:967`).
    /// Verbatim: rightmost = GetLineWidth(position.y) - 1;
    /// return {min(position.x, rightmost), position.y}.
    /// Defensive: y ngoài [0, height-1] → vẫn trả y nguyên (C++ không clamp y).
    pub fn clamp_position_within_line(&self, pos: (i32, i32)) -> (i32, i32) {
        let (x, y) = pos;
        let rightmost = self.get_line_width(y) - 1;
        (x.min(rightmost), y)
    }

    /// `ResetLineRenditionRange(startRow, endRow)` (`textBuffer.cpp:938`).
    /// Verbatim: for row = startRow..endRow: SetLineRendition(SingleWidth).
    /// Defensive: clamp range vào [0, height).
    pub fn reset_line_rendition_range(&mut self, start: u32, end: u32) {
        let height = self.rows.len() as u32;
        if height == 0 {
            return;
        }
        let s = start.min(height);
        let e = end.min(height);
        for row in s..e {
            self.rows[row as usize].set_line_rendition(LineRendition::SingleWidth);
        }
    }

    /// `ClearScrollback(newFirstRow, rowsToKeep)` (`textBuffer.cpp:999`).
    /// Verbatim port (phase 1 giản lược vì không có circular `_firstRow`):
    /// - newFirstRow <= 0 → return (đã ở top).
    /// - rowsToKeep <= 0 → reset toàn bộ.
    /// - Otherwise: scroll `rowsToKeep` row cuối lên top, truncate front.
    ///
    /// WT dùng `_firstRow` circular + `ScrollRows(startAbsolute, rowsToKeep, -startAbsolute)`.
    /// Phase 1 port: dùng `Vec` phẳng (không circular) — bảo toàn
    /// `rowsToKeep` row cuối bằng cách truncate front. Đặt `_firstRow = 0`
    /// ẩn bằng việc rows[0] = row đầu tiên còn lại.
    pub fn clear_scrollback(&mut self, new_first_row: u32, rows_to_keep: u32) {
        if new_first_row == 0 {
            return;
        }
        if rows_to_keep == 0 {
            self.reset();
            return;
        }
        let height = self.rows.len() as u32;
        if rows_to_keep >= height {
            return; // giữ nguyên
        }
        // WT semantic: keep `rows_to_keep` row cuối (`new_first_row` về sau),
        // truncate các row trước `new_first_row`. Trong port phẳng, tương
        // đương: giữ `rows_to_keep` row cuối làm top → truncate front.
        // Nếu rowsToKeep > available từ new_first_row, clamp.
        let available = height.saturating_sub(new_first_row);
        let keep = rows_to_keep.min(available);
        if keep == 0 {
            return;
        }
        let start = (height - keep) as usize;
        let kept: Vec<Row> = self.rows.split_off(start);
        debug_assert_eq!(kept.len() as u32, keep);
        // Pad lại về height với blank rows (port theo WT: rows sau
        // `rowsToKeep` được Reset bằng `_initialAttributes`).
        let fill_attr = self.current_attr;
        let mut new_rows = kept;
        while new_rows.len() < height as usize {
            new_rows.push(Row::new(self.size.0, fill_attr));
        }
        self.rows = new_rows;
    }

    /// `TriggerScroll(delta)` (`textBuffer.cpp:1121`).
    /// Phase 1: no-op skeleton — phase 2 wire tới renderer scroll.
    /// WT fire `Renderer::TriggerScroll(&delta)`; ta chưa có renderer →
    /// log `debug` nếu tracing được wire (hiện no-op).
    pub fn trigger_scroll(&self, _delta: (i32, i32)) {
        // no-op skeleton (phase 2).
    }

    /// `SetCurrentLineRendition(rendition, fillAttributes)` (`textBuffer.cpp:914`).
    /// Verbatim port:
    /// - Lấy cursor position → rowIndex.
    /// - Nếu row.GetLineRendition() != rendition:
    ///   - SetLineRendition(rendition).
    ///   - SetWrapForced(false).
    ///   - Nếu rendition != SingleWidth: FillRect giữa `fillOffset`
    ///     (GetLineWidth) và `til::CoordTypeMax` của row cursor → SPACE,
    ///     + ClampPositionWithinLine(cursor).
    ///   - TriggerRedraw({0, rowIndex, width, rowIndex+1}).
    pub fn set_current_line_rendition(&mut self, rend: LineRendition, attr: &TextAttribute) {
        let cursor_pos = self.cursor.position;
        let row_idx = cursor_pos.1;
        if row_idx < 0 || row_idx as usize >= self.rows.len() {
            return;
        }
        let row_idx_u = row_idx as usize;
        if self.rows[row_idx_u].line_rendition == rend {
            return;
        }
        self.rows[row_idx_u].set_line_rendition(rend);
        self.rows[row_idx_u].set_wrap(false);
        if rend != LineRendition::SingleWidth {
            let fill_offset = self.get_line_width(row_idx);
            // FillRect từ fill_offset đến CoordTypeMax của row cursor với SPACE.
            let fill_char = " ";
            // Defensive: CoordTypeMax = self.size.0 (row width cap).
            let rect_right = self.size.0 as i32;
            let rect = Viewport::new(fill_offset, row_idx, rect_right, row_idx + 1);
            let attr_copy = *attr;
            self.fill_rect(&rect, fill_char, &attr_copy);
            // Row::copy_from trong fill_rect sẽ reset line_rendition về default
            // của scratch row (SingleWidth). Restore rendition đã set ở trên.
            self.rows[row_idx_u].set_line_rendition(rend);
            // Clamp cursor trong new width.
            let clamped = self.clamp_position_within_line(cursor_pos);
            self.cursor.set_position(clamped.0, clamped.1);
            self.trigger_redraw(&Viewport::new(0, row_idx, self.size.0 as i32, row_idx + 1));
        }
    }

    /// `CopyRow(srcRowIndex, dstRowIndex, dstBuffer)` (`textBuffer.cpp:876`).
    /// Verbatim port: `dstRow.CopyFrom(srcRow)` deep copy (`Row::copy_from`).
    /// Phase 1: bỏ `ImageSlice::CopyRow` (chưa port). Defensive: nếu
    /// `dst_buffer` Some → copy sang dst (port `CopyRow` tham số 3);
    /// None → copy trong self.
    pub fn copy_row(&self, src_idx: u32, dst_idx: u32, dst_buffer: Option<&mut TextBuffer>) {
        let height = self.rows.len() as u32;
        if src_idx >= height {
            return;
        }
        let src_row = self.rows[src_idx as usize].clone();
        match dst_buffer {
            Some(dst) => {
                if dst_idx < dst.rows.len() as u32 {
                    dst.rows[dst_idx as usize].copy_from(&src_row);
                }
            }
            None => {
                if dst_idx < height {
                    // Self-copy but we only have &self — caller phải dùng
                    // `scroll_rows` cho trong-buffer copy. Defensive: log+skip.
                    // NOTE: W1 worker có thể gọi variant này; nếu cần, dùng
                    // `scroll_rows` cho mutation trong self.
                    // no-op: immutable self → không thể mutate
                }
            }
        }
    }

    /// `Reset()` (`textBuffer.hpp:157`).
    pub fn reset(&mut self) {
        let attr = self.current_attr;
        for row in &mut self.rows {
            *row = Row::new(self.size.0, attr);
        }
        self.cursor = Cursor::default();
    }

    /// `TriggerRedraw(viewport)` / `TriggerRedrawAll()` (`:168,169`) — phase 2.
    pub fn trigger_redraw(&self, _viewport: &Viewport) {
        // no-op skeleton — phase 2 wire tới GPUI paint cycle.
    }

    pub fn trigger_redraw_all(&self) {
        // no-op skeleton.
    }

    pub fn notify_paint_frame(&self) {
        // no-op skeleton — port `NotifyPaintFrame()` (textBuffer.hpp:167).
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new(80, 24, TextAttribute::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_inits_blank_rows() {
        let b = TextBuffer::new(80, 24, TextAttribute::default());
        assert_eq!(b.size(), (80, 24));
        assert_eq!(b.rows().len(), 24);
        for r in b.rows() {
            assert_eq!(r.column_count(), 80);
            assert!(!r.was_wrap_forced());
        }
        assert_eq!(b.get_cursor().get_position(), (0, 0));
    }

    #[test]
    fn get_row_modulo_handles_circular() {
        let mut b = TextBuffer::new(10, 3, TextAttribute::default());
        b.rows_mut()[0].set_wrap(true);
        let r = b.get_row(3); // wraps to 0
        assert!(r.was_wrap_forced());
    }

    #[test]
    fn get_size_returns_viewport() {
        let b = TextBuffer::new(80, 24, TextAttribute::default());
        let v = b.get_size();
        assert_eq!(v.width(), 80);
        assert_eq!(v.height(), 24);
        assert!(v.contains(0, 0));
        assert!(v.contains(79, 23));
    }

    #[test]
    fn set_wrap_forced_index_safe() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_wrap_forced(2, true);
        assert!(b.rows[2].was_wrap_forced());
        b.set_wrap_forced(99, true); // out-of-range: no panic.
        assert!(!b.rows[4].was_wrap_forced());
    }

    #[test]
    fn cell_iter_walks_glyphs() {
        let mut r = Row::new(10, TextAttribute::default());
        r.glyphs.extend([('a', 1), ('b', 1), ('日', 2)]);
        let cells: Vec<(char, u8)> = CellIter::new(&r, 0).map(|(c, w, _)| (c, w)).collect();
        assert_eq!(cells, [('a', 1), ('b', 1), ('日', 2)]);
    }

    #[test]
    fn set_text_writes_glyphs_width_aware() {
        let mut b = TextBuffer::default();
        b.set_text(0, 0, "hi");
        assert_eq!(b.rows[0].glyphs.len(), 2);
        assert_eq!(b.rows[0].glyphs[0], ('h', 1));
        assert_eq!(b.rows[0].glyphs[1], ('i', 1));
    }

    #[test]
    fn set_text_writes_wide_char_double_width() {
        let mut b = TextBuffer::default();
        b.set_text(0, 0, "日");
        assert!(!b.rows[0].glyphs.is_empty());
        assert_eq!(b.rows[0].glyphs[0].1, 2, "wide glyph should be 2 cols");
    }

    #[test]
    fn clear_row_resets_glyphs_and_attr() {
        let mut b = TextBuffer::default();
        b.set_text(0, 0, "hello");
        let mut attr = TextAttribute::default();
        attr.set_italic(true);
        b.clear_row(0, attr);
        assert!(b.rows[0].glyphs.is_empty());
        assert!(b.rows[0].attr.is_italic());
    }

    #[test]
    fn replace_overwrites_row_text() {
        let mut b = TextBuffer::default();
        b.replace(0, &TextAttribute::default(), "world");
        assert_eq!(b.rows[0].glyphs.len(), 5);
        assert_eq!(b.rows[0].glyphs[0], ('w', 1));
    }

    #[test]
    fn fill_rect_pads_rows() {
        let mut b = TextBuffer::new(10, 3, TextAttribute::default());
        let rect = Viewport::new(0, 0, 4, 2);
        b.fill_rect(&rect, "X", &TextAttribute::default());
        for y in 0..2 {
            for x in 0..4 {
                assert_eq!(b.rows[y].glyphs[x], ('X', 1), "row {y} col {x} should be X");
            }
        }
    }

    #[test]
    fn increment_circular_buffer_rotates_ring() {
        let mut b = TextBuffer::new(10, 3, TextAttribute::default());
        b.set_text(0, 0, "AAA");
        b.set_text(1, 0, "BBB");
        let attr = TextAttribute::default();
        b.increment_circular_buffer(&attr);
        // rows rotated left by 1: AAA moved out, BBB now at row 0.
        assert!(b.rows[2].glyphs.is_empty(), "new last row should be blank");
    }

    #[test]
    fn resize_traditional_grows_and_preserves() {
        let mut b = TextBuffer::new(5, 2, TextAttribute::default());
        b.set_text(0, 0, "abc");
        b.resize_traditional((5, 5));
        assert_eq!(b.size(), (5, 5));
        assert_eq!(b.rows[0].glyphs.len(), 3, "preserved row 0");
    }

    // ===== Phase 4.0-pre — tests cho 8 TextBuffer methods mới =====

    #[test]
    fn scroll_rows_delta_zero_is_no_op() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_text(0, 0, "AAA");
        b.set_text(1, 0, "BBB");
        b.scroll_rows(0, 5, 0);
        assert_eq!(b.rows[0].glyphs.len(), 3);
        assert_eq!(b.rows[0].glyphs[0].0, 'A');
    }

    #[test]
    fn scroll_rows_negative_moves_up() {
        let mut b = TextBuffer::new(10, 6, TextAttribute::default());
        // rows: 0..5; fill row 0=AAA, row 2=CCC (src), delta=-2 → AAA moves to row -2 (skipped).
        // Just verify rotation doesn't panic and content lands at new spots.
        b.set_text(0, 0, "AAA");
        b.set_text(2, 0, "CCC");
        b.scroll_rows(2, 2, -2);
        // Copy row[2] → row[0], row[3] → row[1]
        assert_eq!(b.rows[0].glyphs[0].0, 'C');
        assert_eq!(b.rows[2].glyphs[0].0, 'C', "src not modified");
    }

    #[test]
    fn scroll_rows_positive_moves_down() {
        let mut b = TextBuffer::new(10, 6, TextAttribute::default());
        b.set_text(0, 0, "AAA");
        b.set_text(1, 0, "BBB");
        b.scroll_rows(0, 2, 2);
        // Copy row[1]→row[3], row[0]→row[2]
        assert_eq!(b.rows[2].glyphs[0].0, 'A');
        assert_eq!(b.rows[3].glyphs[0].0, 'B');
    }

    #[test]
    fn scroll_rows_out_of_range_safe() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.scroll_rows(-1, 5, 2); // negative first_row — defensive return
        b.scroll_rows(3, 5, 1); // size beyond height — clamped, no panic
        b.scroll_rows(99, 5, -1); // first_row overflow — safe
    }

    #[test]
    fn get_last_non_space_character_finds_text() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_text(2, 0, "hi");
        let (row, col) = b.get_last_non_space_character(None);
        assert_eq!((row, col), (2, 1));
    }

    #[test]
    fn get_last_non_space_character_skips_trailing_blanks() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_text(1, 0, "x");
        // rows 2,3,4 empty → phải scan up tới row 1
        let (row, col) = b.get_last_non_space_character(None);
        assert_eq!((row, col), (1, 0));
    }

    #[test]
    fn get_last_non_space_character_clamped_zero() {
        let b = TextBuffer::new(10, 5, TextAttribute::default());
        // toàn blank → clamp về (0, 0)
        let (row, col) = b.get_last_non_space_character(None);
        assert_eq!((row, col), (0, 0));
    }

    #[test]
    fn clamp_position_within_line_caps_x() {
        let b = TextBuffer::new(10, 5, TextAttribute::default());
        assert_eq!(b.clamp_position_within_line((15, 0)), (9, 0));
        assert_eq!(b.clamp_position_within_line((5, 0)), (5, 0));
    }

    #[test]
    fn reset_line_rendition_range_resets() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        for r in b.rows_mut() {
            r.set_line_rendition(LineRendition::DoubleWidth);
        }
        b.reset_line_rendition_range(1, 4);
        assert_eq!(b.rows[0].line_rendition, LineRendition::DoubleWidth);
        assert_eq!(b.rows[1].line_rendition, LineRendition::SingleWidth);
        assert_eq!(b.rows[3].line_rendition, LineRendition::SingleWidth);
        assert_eq!(b.rows[4].line_rendition, LineRendition::DoubleWidth);
    }

    #[test]
    fn clear_scrollback_first_row_zero_noop() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_text(2, 0, "x");
        b.clear_scrollback(0, 3);
        assert_eq!(b.rows[2].glyphs[0].0, 'x', "no change when newFirstRow=0");
    }

    #[test]
    fn clear_scrollback_zero_keep_resets() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_text(0, 0, "abc");
        b.clear_scrollback(3, 0);
        assert!(b.rows[0].glyphs.is_empty(), "rowsToKeep=0 → reset all");
    }

    #[test]
    fn clear_scrollback_truncates_front() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.set_text(4, 0, "tail");
        b.clear_scrollback(3, 2);
        assert_eq!(b.rows.len(), 5);
        // keep rows 3,4 → rows[0] (empty), rows[1] (has 't')
        assert!(b.rows[0].glyphs.is_empty());
        assert_eq!(b.rows[1].glyphs[0].0, 't');
    }

    #[test]
    fn trigger_scroll_is_safe_noop() {
        let b = TextBuffer::new(10, 5, TextAttribute::default());
        b.trigger_scroll((1, 0));
        b.trigger_scroll((0, -2));
    }

    #[test]
    fn set_current_line_rendition_switches_and_clamps_cursor() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.cursor.set_position(8, 1);
        b.set_current_line_rendition(LineRendition::DoubleWidth, &TextAttribute::default());
        // Single width 10 → double width 5 cols; cursor at x=8 clamp về 4.
        assert_eq!(b.rows[1].line_rendition, LineRendition::DoubleWidth);
        assert_eq!(b.cursor.position.0, 9 - 5, "x clamped to width/2 - 1");
    }

    #[test]
    fn set_current_line_rendition_same_value_no_change() {
        let mut b = TextBuffer::new(10, 5, TextAttribute::default());
        b.rows_mut()[1].set_line_rendition(LineRendition::DoubleWidth);
        b.set_current_line_rendition(LineRendition::DoubleWidth, &TextAttribute::default());
        assert_eq!(b.rows[1].line_rendition, LineRendition::DoubleWidth);
    }

    #[test]
    fn copy_row_cross_buffer() {
        let src = {
            let mut s = TextBuffer::new(10, 3, TextAttribute::default());
            s.set_text(0, 0, "hello");
            s
        };
        let mut dst = TextBuffer::new(10, 3, TextAttribute::default());
        src.copy_row(0, 1, Some(&mut dst));
        assert_eq!(dst.rows[1].glyphs.len(), 5);
        assert_eq!(dst.rows[1].glyphs[0].0, 'h');
    }

    #[test]
    fn copy_row_out_of_range_safe() {
        let b = TextBuffer::new(10, 3, TextAttribute::default());
        let mut dst = TextBuffer::new(10, 3, TextAttribute::default());
        b.copy_row(99, 0, Some(&mut dst)); // src oob
        b.copy_row(0, 99, Some(&mut dst)); // dst oob
    }

    fn red() -> TextAttribute {
        let mut a = TextAttribute::default();
        a.set_indexed_foreground(1);
        a
    }

    #[test]
    fn wide_glyph_has_trailer_and_overwrites_cleanly() {
        let mut r = Row::new(10, TextAttribute::default());
        r.replace_text(0, 10, "\u{65E5}x"); // 日 then x
        assert_eq!(&r.glyphs[..3], &[('\u{65E5}', 2), (' ', 0), ('x', 1)]);
        // Overwrite the trailer: the lead becomes a blank.
        r.replace_text(1, 10, "y");
        assert_eq!(&r.glyphs[..3], &[(' ', 1), ('y', 1), ('x', 1)]);
        // A wide glyph over two narrow ones.
        r.replace_text(1, 10, "\u{672C}");
        assert_eq!(&r.glyphs[..3], &[(' ', 1), ('\u{672C}', 2), (' ', 0)]);
    }

    #[test]
    fn row_attr_runs_set_and_query() {
        let mut r = Row::new(10, TextAttribute::default());
        r.set_attr_range(2, 5, &red());
        assert_eq!(*r.attr_at(1), TextAttribute::default());
        assert_eq!(*r.attr_at(2), red());
        assert_eq!(*r.attr_at(4), red());
        assert_eq!(*r.attr_at(5), TextAttribute::default());
        // Overwrite the middle: neighbours keep their attribute.
        let mut green = TextAttribute::default();
        green.set_indexed_foreground(2);
        r.set_attr_range(3, 4, &green);
        assert_eq!((*r.attr_at(2), *r.attr_at(3), *r.attr_at(4)), (red(), green, red()));
        // Same attribute written again merges into one run.
        r.set_attr_range(3, 4, &red());
        assert_eq!(r.attr_runs.len(), 2);
    }

    #[test]
    fn row_fill_and_copy_attrs() {
        let mut r = Row::new(6, TextAttribute::default());
        r.set_attr_range(1, 3, &red());
        let mut dst = Row::new(6, TextAttribute::default());
        dst.copy_attr_range_from(&r, 0, 6);
        assert_eq!(*dst.attr_at(2), red());
        dst.fill_attr_range(0, 6, &TextAttribute::default());
        assert!(dst.attr_runs.is_empty());
        assert_eq!(*dst.attr_at(2), TextAttribute::default());
    }

    #[test]
    fn scrolled_off_rows_go_to_scrollback_and_trim() {
        let mut b = TextBuffer::new(3, 2, TextAttribute::default());
        b.scrollback_limit = 2;
        for (i, ch) in ["a", "b", "c"].into_iter().enumerate() {
            b.set_text(0, 0, ch);
            b.increment_circular_buffer(&TextAttribute::default());
            assert_eq!(b.scrollback.len(), (i + 1).min(2));
        }
        assert_eq!(b.scrollback_dropped, 1);
        assert_eq!(b.total_rows(), 4);
        assert_eq!(b.row_abs(0).unwrap().glyphs[0].0, 'b');
        assert_eq!(b.row_abs(1).unwrap().glyphs[0].0, 'c');
        b.clear_history();
        assert_eq!(b.scrollback.len(), 0);
        assert_eq!(b.scrollback_dropped, 3);
    }
}
