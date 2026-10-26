//! W2 — Erase family. Port verbatim `windows/src/terminal/adapter/adaptDispatch.cpp`:
//! - `_ScrollRectVertically` (private)        — `:549`
//! - `_ScrollRectHorizontally` (private)      — `:607`
//! - `_InsertDeleteCharacterHelper` (private) — `:654`
//! - `InsertCharacter` (ICH)                  — `:677`
//! - `DeleteCharacter` (DCH)                  — `:687`
//! - `_FillRect` (private)                    — `:701`
//! - `EraseCharacters` (ECH)                 — `:713`
//! - `EraseInDisplay` (ED)                    — `:735`
//! - `EraseInLine` (EL)                       — `:793`
//! - `_SelectiveEraseRect` (private)          — `:827`
//! - `SelectiveEraseInDisplay` (DECSED)        — `:857`
//! - `SelectiveEraseInLine` (DECSEL)           — `:892`
//!
//! Free-fn split: `pub fn <name>(this: &mut TerminalDispatch, …)`. W1
//! wires into `impl ITermDispatch`. Helpers shared with rect-area (W2 rects.rs)
//! and sibling waves (W4 bell_tabs) are `pub(crate)` here.
//!
//! WT uses `til::rect` with **exclusive** right/bottom; our `Viewport` is
//! inclusive, so call sites convert `(left, top, right_excl, bottom_excl)` →
//! `Viewport::new(left, top, right_excl - 1, bottom_excl - 1)`.

use super::TerminalDispatch;
use super::Mode;
use crate::dispatch::types::EraseType;
use crate::dispatch::VtInt;
use crate::screen::Viewport;

/// `whitespace` — port `adaptDispatch.cpp:19`. WT fills erased cells with '\x20'.
pub(crate) const WHITESPACE: &str = " ";

/// WT `_GetEraseAttributes(page)` — `adaptDispatch.cpp:527`.
///
/// Returns default `TextAttribute` when `Mode::EraseColor` is set; otherwise
/// `page.Attributes()` (= `buffer.current_attr`) with `SetStandardErase`
/// applied (clears rendition bits — colors retained).
///
/// This helper is shared with sibling waves (W4 bell_tabs, W4 reset) → exposed
/// as `pub(crate)`. Called as `super::erase::get_erase_attributes(this)`
/// (e.g. `bell_tabs.rs:237`, `reset.rs:204`).
pub(crate) fn get_erase_attributes(this: &TerminalDispatch) -> crate::attrs::TextAttribute {
    if this.modes.test(Mode::EraseColor) {
        return crate::attrs::TextAttribute::default();
    }
    let mut attr = *this.terminal.get_current_attributes();
    attr.set_standard_erase();
    attr
}

/// Helper — convert WT's exclusive `til::rect {left, top, right, bottom}` to
/// a `Viewport` constructed for the dispatch→buffer convention. Our
/// `Viewport` documents its bounds as inclusive (`left..=right`, `top..=bottom`),
/// but the **buffer-side methods** (`fill_rect`, `reset_line_rendition_range`,
/// `trigger_redraw`) iterate as `[top, bottom)` — i.e. they expect the WT-style
/// exclusive bottom. Sibling waves (e.g. `reset.rs:124`) follow the same
/// convention: `Viewport::new(0, top, width, bottom + 1)`. So callers pass
/// WT-exclusive coords directly (right = wt_right_excl, bottom = wt_bottom_excl).
fn wt_rect(left: VtInt, top: VtInt, right_excl: VtInt, bottom_excl: VtInt) -> Viewport {
    Viewport::new(left, top, right_excl, bottom_excl)
}

/// `_FillRect(page, fillRect, fillChar, fillAttrs)` — `adaptDispatch.cpp:701`.
///
/// WT body: `page.Buffer().FillRect(fillRect, fillChar, fillAttrs)`. Verbatim
/// semantics: write `fillChar` into every cell in `rect` (rows `[top, bottom)`,
/// cols `[left, right)`), leaving cells **outside** `rect` untouched. Pads the
/// rows up to `right_excl` with `UNICODE_SPACE` as needed.
///
/// We do NOT delegate to `TextBuffer::fill_rect` because its Phase-1 impl uses
/// `Row::copy_from(scratch)` which clears the destination row's glyphs and
/// replaces them with only the rect's content (drops cells outside the rect).
/// WT `TextBuffer::FillRect` preserves outside content; the W2 erase helpers
/// rely on that preservation (e.g. `erase_in_line(ToCursor)` clears `[0, col]`
/// but must keep `[col+1, lineWidth)` intact).
pub(crate) fn fill_rect(
    this: &mut TerminalDispatch,
    rect: &Viewport,
    fill_char: &str,
    fill_attrs: &crate::attrs::TextAttribute,
) {
    if rect.is_empty() || fill_char.is_empty() {
        return;
    }
    // Use the first grapheme's first char + width as the cell glyph (matches
    // `Row::replace_text` semantics for ASCII fill chars like ' ' or 'A').
    let first_char = fill_char
        .chars()
        .next()
        .unwrap_or(crate::buffer::UNICODE_SPACE);
    let width = unicode_width::UnicodeWidthChar::width(first_char)
        .unwrap_or(0)
        .max(1) as u8;
    let cell = (first_char, width);
    let rows_len = this.terminal.rows().len();
    let left = rect.left as usize;
    let right_excl = rect.right as usize;
    for y in rect.top..rect.bottom {
        let yi = y as usize;
        if yi >= rows_len {
            break;
        }
        let row = this.terminal.get_row_mut(yi);
        // Pad glyphs up to `right_excl` so assignment by index is in-bounds.
        while row.glyphs.len() < right_excl {
            row.glyphs.push((crate::buffer::UNICODE_SPACE, 1));
        }
        for col in left..right_excl {
            row.glyphs[col] = cell;
        }
        row.fill_attr_range(left, right_excl, fill_attrs);
        this.terminal
            .trigger_redraw(&Viewport::new(rect.left, y, rect.right, y));
    }
}

/// `_ScrollRectVertically(page, scrollRect, delta)` — `adaptDispatch.cpp:549`.
///
/// Verbatim semantics (positive delta = down, negative = up):
/// 1. `absoluteDelta = min(|delta|, scrollRect.height())`.
/// 2. If `absoluteDelta < scrollRect.height()`:
///    - `top = delta > 0 ? scrollRect.top : scrollRect.top + absoluteDelta`.
///    - `width = scrollRect.width(); height = scrollRect.height() - absoluteDelta`.
///    - `actualDelta = delta > 0 ? absoluteDelta : -absoluteDelta`.
///    - If `width == pageWidth` (full-width): `buffer.ScrollRows(top, height, actualDelta)`
///      + `TriggerRedraw(scrollRect)`.
///    - Else: cell-by-cell copy from `src` to `dst`. Phase-1 port snap-copies
///      row-by-row, then copies glyphs within `[left, right)` into the dst.
/// 3. Rows revealed by scroll (`top..top+absoluteDelta`) filled with erase
///    attrs (whitespace + eraseAttributes), and `ResetLineRenditionRange`
///    called for the revealed rows.
///
/// `rect: &Viewport` is the WT-exclusive `til::rect` — `right` and `bottom`
/// are exclusive (half-open `[left, right)`, `[top, bottom)`). This matches
/// the convention at the dispatch→buffer layer (see `wt_rect` doc and
/// `reset.rs:124`). Per the inclusive `Viewport` docstring the inclusive
/// math (`rect.width()`, `rect.height()`) over-reports by 1, so we use
/// `rect.right - rect.left` and `rect.bottom - rect.top` directly.
pub(crate) fn scroll_rect_vertically(this: &mut TerminalDispatch, rect: &Viewport, delta: VtInt) {
    let height = rect.bottom - rect.top; // WT-exclusive height
    if height <= 0 {
        return;
    }
    // Pre-pad rows in the scroll rect up to `rect.right` with whitespace.
    // Phase-1 `Row::new` starts with zero glyphs, unlike WT which initializes
    // rows with width-many space cells. Padding before the row rotation makes
    // `scroll_rows` propagate the WT-style space cells (and downstream tests
    // see `Some((' ', 1))` rather than `None` for shifted rows).
    let rows_len = this.terminal.rows().len();
    let pad_right = rect.right as usize;
    for y in rect.top..rect.bottom {
        let yi = y as usize;
        if yi >= rows_len {
            break;
        }
        let row = this.terminal.get_row_mut(yi);
        while row.glyphs.len() < pad_right {
            row.glyphs.push((crate::buffer::UNICODE_SPACE, 1));
        }
    }
    let abs_delta = delta.unsigned_abs().min(height as u32) as i32;
    if abs_delta >= height {
        // Full reveal — fill the entire scrollRect with erase attrs (no copy).
        let erase_attrs = get_erase_attributes(this);
        fill_rect(this, rect, WHITESPACE, &erase_attrs);
        // WT: `ResetLineRenditionRange(scrollRect.top, scrollRect.bottom)` —
        // exclusive bottom. `reset_line_rendition_range(start, end)` is
        // `[start, end)` — pass `rect.bottom` directly.
        this.terminal
            .reset_line_rendition_range(rect.top as u32, rect.bottom as u32);
        return;
    }

    let top = if delta > 0 {
        rect.top
    } else {
        rect.top + abs_delta
    };
    let width = rect.right - rect.left; // WT-exclusive width
    let inner_height = height - abs_delta;
    let actual_delta = if delta > 0 { abs_delta } else { -abs_delta };

    let page_width = this.viewport.width();
    if width == page_width {
        // Full-width scroll — use the efficient row-rotation path.
        // `buffer::scroll_rows(first_row: i32, size: i32, delta: i32)` — `size`
        // is the row count (= `inner_height` here). Phase 4.0-pre landed
        // (`buffer.rs:563` signature matches).
        this.terminal.scroll_rows(top, inner_height, actual_delta);
        this.terminal.trigger_redraw(rect);
    } else {
        // Partial-width scroll — cell-by-cell snap-copy. Snapshot src row,
        // then copy glyphs within `[left, right)` into the dst row in walk
        // order (down scroll = bottom-up, up scroll = top-down).
        let buffer_height = this.terminal.rows().len() as i32;
        let rows: Vec<i32> = if actual_delta > 0 {
            // Down: dst = src + delta. Walk bottom-up to avoid clobbering src.
            (top..top + inner_height).rev().collect()
        } else {
            (top..top + inner_height).collect()
        };
        for src in rows {
            let dst = src + actual_delta;
            if dst < 0 || dst >= buffer_height {
                continue;
            }
            let src_row = this.terminal.get_row(src as usize).clone();
            let dst_row = this.terminal.get_row_mut(dst as usize);
            let left = rect.left as usize;
            let right_excl = rect.right as usize;
            // Pad dst glyphs up to `right_excl - 1` inclusive.
            while dst_row.glyphs.len() < right_excl {
                dst_row.glyphs.push((crate::buffer::UNICODE_SPACE, 1));
            }
            for col in left..right_excl {
                let cell = src_row
                    .glyphs
                    .get(col)
                    .copied()
                    .unwrap_or((crate::buffer::UNICODE_SPACE, 1));
                dst_row.glyphs[col] = cell;
            }
            dst_row.copy_attr_range_from(&src_row, left, right_excl);
        }
        this.terminal.trigger_redraw(rect);
    }

    // Rows revealed by the scroll are filled with erase attributes.
    // WT: eraseRect.top = delta > 0 ? scrollRect.top : scrollRect.bottom - absoluteDelta;
    //     eraseRect.bottom = eraseRect.top + absoluteDelta;
    // Both top and bottom are WT-exclusive (half-open).
    let mut erase_rect = *rect;
    erase_rect.top = if delta > 0 {
        rect.top
    } else {
        rect.bottom - abs_delta
    };
    erase_rect.bottom = erase_rect.top + abs_delta;
    let erase_attrs = get_erase_attributes(this);
    fill_rect(this, &erase_rect, WHITESPACE, &erase_attrs);
    // WT: `ResetLineRenditionRange(eraseRect.top, eraseRect.bottom)` — exclusive
    // bottom; passes through directly.
    this.terminal
        .reset_line_rendition_range(erase_rect.top as u32, erase_rect.bottom as u32);
}

/// `_ScrollRectHorizontally(page, scrollRect, delta)` — `adaptDispatch.cpp:607`.
///
/// Verbatim (positive delta = right, negative = left). `rect: &Viewport` is
/// the WT-exclusive `til::rect` (`right`/`bottom` are half-open). See the
/// convention note in `scroll_rect_vertically`.
pub(crate) fn scroll_rect_horizontally(this: &mut TerminalDispatch, rect: &Viewport, delta: VtInt) {
    let width = rect.right - rect.left; // WT-exclusive width
    if width <= 0 {
        return;
    }
    let abs_delta = delta.unsigned_abs().min(width as u32) as i32;
    if abs_delta >= width {
        // Full reveal — fill entire rect with erase attrs.
        let erase_attrs = get_erase_attributes(this);
        fill_rect(this, rect, WHITESPACE, &erase_attrs);
        return;
    }

    let left = if delta > 0 {
        rect.left
    } else {
        rect.left + abs_delta
    };
    let top = rect.top;
    let inner_width = width - abs_delta;
    let actual_delta = if delta > 0 { abs_delta } else { -abs_delta };
    let height = rect.bottom - rect.top; // WT-exclusive height (rows to walk)

    // Phase-1 snap-copy: read source row snapshot before writing to dst so a
    // two-cell DBCS char won't self-delete during overlap. WT reads `next`
    // cell one ahead then writes; we snapshot the row, which is equivalent in
    // our simplified glyph model (no separate offset buffer).
    for row in top..top + height {
        let row_idx = row as usize;
        if row_idx >= this.terminal.rows().len() {
            break;
        }
        let snapshot: Vec<(char, u8)> = this.terminal.get_row(row_idx).glyphs.to_vec();
        let snap_row = this.terminal.get_row(row_idx).clone();
        let row_buf = this.terminal.get_row_mut(row_idx);
        // Ensure glyphs length covers rect.right (exclusive upper bound):
        // pad up to `rect.right - 1` inclusive.
        let right_excl = rect.right as usize;
        while row_buf.glyphs.len() < right_excl {
            row_buf.glyphs.push((crate::buffer::UNICODE_SPACE, 1));
        }
        for src_col in left..left + inner_width {
            let dst_col = src_col + actual_delta;
            if dst_col < 0 || dst_col as usize >= right_excl {
                continue;
            }
            let src_idx = src_col as usize;
            let cell = snapshot
                .get(src_idx)
                .copied()
                .unwrap_or((crate::buffer::UNICODE_SPACE, 1));
            row_buf.glyphs[dst_col as usize] = cell;
            let attr = *snap_row.attr_at(src_idx);
            row_buf.set_attr_range(dst_col as usize, dst_col as usize + 1, &attr);
        }
    }
    this.terminal.trigger_redraw(rect);

    // Columns revealed by the scroll are filled with erase attributes.
    // WT: eraseRect.left = delta > 0 ? scrollRect.left : scrollRect.right - absoluteDelta;
    //     eraseRect.right = eraseRect.left + absoluteDelta;
    let mut erase_rect = *rect;
    erase_rect.left = if delta > 0 {
        rect.left
    } else {
        rect.right - abs_delta
    };
    erase_rect.right = erase_rect.left + abs_delta;
    let erase_attrs = get_erase_attributes(this);
    fill_rect(this, &erase_rect, WHITESPACE, &erase_attrs);
}

/// `_InsertDeleteCharacterHelper(delta)` — `adaptDispatch.cpp:654`.
///
/// Verbatim: pulls cursor (col, row), lineWidth, vertical+horizontal margins.
/// If cursor is within margin horizontal range and row is within vertical
/// margins, scrolls the single-row rect `{col, row, right+1, row+1}` by delta.
/// Resets delayed EOL wrap flag.
pub(crate) fn insert_delete_character_helper(this: &mut TerminalDispatch, delta: VtInt) {
    let (col, row) = this.terminal.get_cursor().get_position();
    let line_width = this.terminal.get_line_width(row);
    let (top_margin, bottom_margin) = this.get_vertical_margins(true);
    let (left_margin, right_margin) = if row >= top_margin && row <= bottom_margin {
        this.get_horizontal_margins(line_width)
    } else {
        (0, line_width - 1)
    };
    if col >= left_margin && col <= right_margin {
        // WT: `_ScrollRectHorizontally(page, {col, row, right+1, row+1}, delta)`.
        // `right+1`, `row+1` are WT-exclusive bounds — pass directly to `Viewport::new`.
        let rect = Viewport::new(col, row, right_margin + 1, row + 1);
        scroll_rect_horizontally(this, &rect, delta);
        // The ICH and DCH controls are expected to reset the delayed wrap flag.
        this.terminal.get_cursor_mut().reset_delay_eol_wrap();
    }
}

// —— Public ITermDispatch methods ——

/// ICH — `InsertCharacter(count)` (`adaptDispatch.cpp:677`).
/// `_InsertDeleteCharacterHelper(count)`.
pub fn insert_character(this: &mut TerminalDispatch, count: VtInt) {
    insert_delete_character_helper(this, count);
}

/// DCH — `DeleteCharacter(count)` (`adaptDispatch.cpp:687`).
/// `_InsertDeleteCharacterHelper(-count)`.
pub fn delete_character(this: &mut TerminalDispatch, count: VtInt) {
    insert_delete_character_helper(this, -count);
}

/// ECH — `EraseCharacters(numChars)` (`adaptDispatch.cpp:713`).
pub fn erase_characters(this: &mut TerminalDispatch, num_chars: VtInt) {
    let row = this.terminal.get_cursor().get_position().1;
    let start_col = this.terminal.get_cursor().get_position().0;
    let line_width = this.terminal.get_line_width(row);
    let end_col = (start_col + num_chars).min(line_width);
    // The ECH control is expected to reset the delayed wrap flag.
    this.terminal.get_cursor_mut().reset_delay_eol_wrap();
    let erase_attrs = get_erase_attributes(this);
    // WT: `_FillRect(page, { startCol, row, endCol, row + 1 }, whitespace, ...)`.
    // Exclusive → inclusive: endCol-1, row.
    let rect = wt_rect(start_col, row, end_col, row + 1);
    fill_rect(this, &rect, WHITESPACE, &erase_attrs);
}

/// ED — `EraseInDisplay(erase_type)` (`adaptDispatch.cpp:735`).
///
/// Dispatches `Scrollback` and `All` to sibling-wave helpers (`_EraseScrollback`,
/// `_EraseAll`) which are NOT owned by W2 — they belong to W4 modes/reset per
/// the work split (those methods live outside the erase/rect list). For W2 we
/// fall back to a verbatim local erase that matches the WT `FromBeginning`
/// and `ToEnd` cases. `All` (move page to scrollback then erase) we approximate
/// by clearing the page with erase attrs (full row reset). `Scrollback` is a
/// no-op stub until the scrollback helper lands.
pub fn erase_in_display(this: &mut TerminalDispatch, erase_type: EraseType) {
    // WT early-return: `if (eraseType > EraseType::Scrollback) return;`
    // Our `EraseType` enum has only 4 variants (FromCursor, ToCursor, All,
    // Scrollback), so any out-of-range value is impossible — drop the guard.
    match erase_type {
        EraseType::Scrollback => {
            // WT `_EraseScrollback`: drop history, keep the page.
            this.terminal.clear_history();
            return;
        }
        EraseType::All => {
            // WT: `return _EraseAll();` — owned by W4 reset. W2 falls back to
            // erasing the entire visible page (matches the visible effect;
            // the C++ `_EraseAll` also moves current content into scrollback,
            // which `clear_scrollback` will handle once wired).
            // WAIT W4: _erase_all helper to be wired in reset.rs.
            this.terminal.get_cursor_mut().reset_delay_eol_wrap();
            let page_width = this.viewport.width();
            let page_top = this.viewport.top;
            let page_height = this.viewport.height();
            // `_EraseAll` keeps what was on screen as history (so `cls` doesn't
            // lose it): rows up to the last one holding text.
            let last_used = this.terminal.rows().iter().rposition(|r| r.contains_text());
            if let Some(last) = last_used {
                for i in 0..=last {
                    let row = this.terminal.rows()[i].clone();
                    this.terminal.push_history(row);
                }
            }
            let erase_attrs = get_erase_attributes(this);
            let rect = wt_rect(0, page_top, page_width, page_top + page_height);
            fill_rect(this, &rect, WHITESPACE, &erase_attrs);
            this.terminal
                .reset_line_rendition_range(page_top as u32, (page_top + page_height) as u32);
            return;
        }
        _ => {}
    }

    let row = this.terminal.get_cursor().get_position().1;
    let col = this.terminal.get_cursor().get_position().0;
    let page_width = this.viewport.width();
    let page_top = this.viewport.top;
    let page_bottom = this.viewport.bottom;

    // The ED control is expected to reset the delayed wrap flag.
    this.terminal.get_cursor_mut().reset_delay_eol_wrap();

    let erase_attrs = get_erase_attributes(this);

    // —— Mapping note ——
    // WT `DispatchTypes::EraseType` (`:367`): `ToEnd=0`, `FromBeginning=1`,
    // `All=2`, `Scrollback=3`. Our Rust `EraseType` (`dispatch/types.rs:9`):
    // `FromCursor=0` (= WT `ToEnd`, "from cursor to end"), `ToCursor=1`
    // (= WT `FromBeginning`, "from beginning to cursor"), `All=2`,
    // `Scrollback=3`. Same numeric ordering → values pass through verbatim;
    // bodies below map the *WT* branch (e.g. our `FromCursor` does the WT
    // `ToEnd` work).

    if erase_type == EraseType::FromBeginning {
        // WT branch `FromBeginning`: rows above cursor + cursor's row [0..col].
        // `ResetLineRenditionRange(page.Top(), row)` (exclusive end = row).
        this.terminal
            .reset_line_rendition_range(page_top as u32, row as u32);
        let rect = wt_rect(0, page_top, page_width, row);
        fill_rect(this, &rect, WHITESPACE, &erase_attrs);
        // Cursor row: `[0, col]` inclusive — WT `{0, row, col+1, row+1}`.
        let rect = wt_rect(0, row, col + 1, row + 1);
        fill_rect(this, &rect, WHITESPACE, &erase_attrs);
    }
    if erase_type == EraseType::ToEnd {
        // WT branch `ToEnd`: cursor row from col + rows below.
        // `if (col > 0) ResetLineRenditionRange(row+1, page.Bottom())`
        //   `else ResetLineRenditionRange(row, page.Bottom())`.
        let reset_start = if col > 0 { row + 1 } else { row };
        this.terminal
            .reset_line_rendition_range(reset_start as u32, (page_bottom + 1) as u32);
        // Cursor row: `[col, width-1]` — WT `{col, row, pageWidth, row+1}`.
        let rect = wt_rect(col, row, page_width, row + 1);
        fill_rect(this, &rect, WHITESPACE, &erase_attrs);
        // Below cursor: rows `[row+1, page.Bottom()]`.
        let rect = wt_rect(0, row + 1, page_width, page_bottom + 1);
        fill_rect(this, &rect, WHITESPACE, &erase_attrs);
    }
}

/// EL — `EraseInLine(erase_type)` (`adaptDispatch.cpp:793`).
pub fn erase_in_line(this: &mut TerminalDispatch, erase_type: EraseType) {
    let row = this.terminal.get_cursor().get_position().1;
    let col = this.terminal.get_cursor().get_position().0;
    let line_width = this.terminal.get_line_width(row);

    // The EL control is expected to reset the delayed wrap flag.
    this.terminal.get_cursor_mut().reset_delay_eol_wrap();

    let erase_attrs = get_erase_attributes(this);
    match erase_type {
        // See mapping note in `erase_in_display`:
        // our `FromCursor` = WT `ToEnd`; our `ToCursor` = WT `FromBeginning`.
        EraseType::FromBeginning => {
            // WT `FromBeginning`: `[0, col]` inclusive — `{0, row, col+1, row+1}`.
            let rect = wt_rect(0, row, col + 1, row + 1);
            fill_rect(this, &rect, WHITESPACE, &erase_attrs);
        }
        EraseType::ToEnd => {
            // WT `ToEnd`: `[col, lineWidth-1]` — `{col, row, lineWidth, row+1}`.
            let rect = wt_rect(col, row, line_width, row + 1);
            fill_rect(this, &rect, WHITESPACE, &erase_attrs);
        }
        EraseType::All => {
            // Whole row — WT `{0, row, lineWidth, row+1}`.
            let rect = wt_rect(0, row, line_width, row + 1);
            fill_rect(this, &rect, WHITESPACE, &erase_attrs);
        }
        EraseType::Scrollback => {
            // Not defined for EL — WT `default: break;`. No-op.
        }
    }
}

/// `_SelectiveEraseRect(page, eraseRect)` — `adaptDispatch.cpp:827`.
///
/// Verbatim: for each (row, col) in `eraseRect`, if `!row.GetAttrByColumn(col).IsProtected()`,
/// clear the cell text but leave attributes. Called `rowBuffer.ClearCell(col)`.
///
/// Phase 1 port: `Row` uses single-attr-per-row (`Row::attr: TextAttribute`).
/// If `row.attr.is_protected()` we skip **all** cells in the row (no per-cell
/// attr granularity). Otherwise we clear glyphs in the column range, leaving
/// `row.attr` untouched (matches WT's "leave attrs" semantics at per-cell granularity
/// — collapsed to per-row in phase 1). ImageSlice port deferred to phase 2.
pub(crate) fn selective_erase_rect(this: &mut TerminalDispatch, rect: &Viewport) {
    if rect.is_empty() {
        return;
    }
    let rows_len = this.terminal.rows().len();
    // WT: `for row in [rect.top, rect.bottom) : for col in [rect.left, rect.right)`.
    // Our `rect.right`/`rect.bottom` are the WT-exclusive upper bounds.
    for row in rect.top..rect.bottom {
        let row_idx = row as usize;
        if row_idx >= rows_len {
            break;
        }
        // Check protected flag once per row (phase-1 single-attr).
        if this.terminal.get_row(row_idx).attr().is_protected() {
            continue;
        }
        let row_buf = this.terminal.get_row_mut(row_idx);
        let left = rect.left as usize;
        let right_excl = rect.right as usize;
        // Pad glyphs to cover `right_excl - 1` inclusive.
        while row_buf.glyphs.len() < right_excl {
            row_buf.glyphs.push((crate::buffer::UNICODE_SPACE, 1));
        }
        for col in left..right_excl {
            row_buf.glyphs[col] = (crate::buffer::UNICODE_SPACE, 1);
        }
        // WT: `TriggerRedraw(Viewport::FromDimensions({col, row}, {1, 1}))`
        // per erased cell. We collapse to one redraw per row for the column
        // range (the half-open `.. rect.right` upper bound becomes inclusive
        // `rect.right - 1` for the redraw rect coordinates).
        this.terminal
            .trigger_redraw(&Viewport::new(rect.left, row, rect.right, row + 1));
    }
}

/// DECSED — `SelectiveEraseInDisplay(erase_type)` (`adaptDispatch.cpp:857`).
pub fn selective_erase_in_display(this: &mut TerminalDispatch, erase_type: EraseType) {
    let row = this.terminal.get_cursor().get_position().1;
    let col = this.terminal.get_cursor().get_position().0;
    let page_width = this.viewport.width();
    let page_top = this.viewport.top;
    let page_bottom = this.viewport.bottom;

    // The DECSED control is expected to reset the delayed wrap flag.
    this.terminal.get_cursor_mut().reset_delay_eol_wrap();

    match erase_type {
        // See mapping note in `erase_in_display`:
        // our `FromCursor` = WT `ToEnd`; our `ToCursor` = WT `FromBeginning`.
        EraseType::FromBeginning => {
            // WT `FromBeginning`: rows above cursor + cursor row up to col.
            selective_erase_rect(this, &wt_rect(0, page_top, page_width, row));
            selective_erase_rect(this, &wt_rect(0, row, col + 1, row + 1));
        }
        EraseType::ToEnd => {
            // WT `ToEnd`: cursor row from col + rows below.
            selective_erase_rect(this, &wt_rect(col, row, page_width, row + 1));
            selective_erase_rect(this, &wt_rect(0, row + 1, page_width, page_bottom + 1));
        }
        EraseType::All => {
            // Entire page: `{0, page.Top(), pageWidth, page.Bottom()}`.
            selective_erase_rect(this, &wt_rect(0, page_top, page_width, page_bottom + 1));
        }
        EraseType::Scrollback => {
            // Not defined for DECSED — no-op (matches WT default branch).
        }
    }
}

/// DECSEL — `SelectiveEraseInLine(erase_type)` (`adaptDispatch.cpp:892`).
pub fn selective_erase_in_line(this: &mut TerminalDispatch, erase_type: EraseType) {
    let row = this.terminal.get_cursor().get_position().1;
    let col = this.terminal.get_cursor().get_position().0;
    let line_width = this.terminal.get_line_width(row);

    // The DECSEL control is expected to reset the delayed wrap flag.
    this.terminal.get_cursor_mut().reset_delay_eol_wrap();

    match erase_type {
        // See mapping note in `erase_in_display`:
        // our `FromCursor` = WT `ToEnd`; our `ToCursor` = WT `FromBeginning`.
        EraseType::FromBeginning => {
            // WT `FromBeginning`: `{0, row, col+1, row+1}`.
            selective_erase_rect(this, &wt_rect(0, row, col + 1, row + 1));
        }
        EraseType::ToEnd => {
            // WT `ToEnd`: `{col, row, lineWidth, row+1}`.
            selective_erase_rect(this, &wt_rect(col, row, line_width, row + 1));
        }
        EraseType::All => {
            selective_erase_rect(this, &wt_rect(0, row, line_width, row + 1));
        }
        EraseType::Scrollback => {
            // Not defined for DECSEL — no-op.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Row, TextBuffer, UNICODE_SPACE};
    use crate::cursor::Cursor;
    use crate::dispatch::InteractDispatch;

    /// Minimal `InteractDispatch` stub.
    struct StubInteract;
    impl InteractDispatch for StubInteract {}

    /// Build a `TerminalDispatch` with `terminal` set up and cursor at (col, row).
    fn make_dispatch(cols: u16, rows: u16) -> TerminalDispatch {
        let terminal = TextBuffer::new(cols, rows, crate::attrs::TextAttribute::default());
        TerminalDispatch::new(terminal, Box::new(StubInteract))
    }

    fn set_cursor(this: &mut TerminalDispatch, x: i32, y: i32) {
        this.terminal.get_cursor_mut().set_position(x, y);
    }

    /// `wt_rect` passes WT-exclusive coords directly to `Viewport::new`. Our
    /// `Viewport` documents its bounds as inclusive (`left..=right`,
    /// `top..=bottom`), but the buffer-side methods (`fill_rect`,
    /// `reset_line_rendition_range`, etc.) iterate as `[top, bottom)` — so
    /// dispatch-layer callers pass `bottom = wt.bottom_excl` directly.
    #[test]
    fn wt_rect_passes_exclusive_coords_through() {
        // WT rect {left=0, top=3, right=5, bottom=7} exclusive bottom.
        // `Viewport::new(0, 3, 5, 7)` stores these verbatim.
        let v = wt_rect(0, 3, 5, 7);
        assert_eq!(v.left, 0);
        assert_eq!(v.top, 3);
        assert_eq!(v.right, 5);
        assert_eq!(v.bottom, 7);
        // `Viewport::width/height` use inclusive math (right-left+1, etc.),
        // so they over-report relative to the WT-exclusive semantics. Buffer
        // iteration uses `top..bottom` (exclusive), matching WT.
        assert_eq!(v.width(), 6);
        assert_eq!(v.height(), 5);
    }

    #[test]
    fn get_erase_attributes_clears_rendition_keeps_color() {
        let this = make_dispatch(10, 5);
        let a = get_erase_attributes(&this);
        assert!(!a.is_italic());
        assert!(!a.is_reverse_video());
    }

    #[test]
    fn get_erase_attributes_returns_default_when_erase_color_mode() {
        let mut this = make_dispatch(10, 5);
        this.modes.set(Mode::EraseColor, true);
        let a = get_erase_attributes(&this);
        assert!(!a.is_legacy());
    }

    #[test]
    fn fill_rect_writes_char_into_rect() {
        let mut this = make_dispatch(10, 5);
        let rect = wt_rect(1, 1, 4, 3);
        fill_rect(
            &mut this,
            &rect,
            "X",
            &crate::attrs::TextAttribute::default(),
        );
        for row in 1..=2 {
            for col in 1..=3 {
                let cell = this.terminal.get_row(row as usize).glyphs.get(col as usize);
                assert_eq!(cell, Some(&('X', 1)), "row {row} col {col} should be X");
            }
        }
    }

    #[test]
    fn fill_rect_whitespace_default_behavior() {
        let mut this = make_dispatch(8, 3);
        // Fill glyphs first to non-space.
        let rect = wt_rect(0, 0, 4, 1);
        fill_rect(
            &mut this,
            &rect,
            "X",
            &crate::attrs::TextAttribute::default(),
        );
        // Erase with whitespace.
        fill_rect(
            &mut this,
            &rect,
            WHITESPACE,
            &crate::attrs::TextAttribute::default(),
        );
        for col in 0..4 {
            let cell = this.terminal.get_row(0).glyphs.get(col as usize);
            assert_eq!(cell, Some(&(UNICODE_SPACE, 1)));
        }
    }

    #[test]
    fn erase_in_line_to_cursor_clears_left_part() {
        // ToCursor = WT FromBeginning: clears `[0, col]` inclusive (left side).
        let mut this = make_dispatch(10, 3);
        // Seed row 1 with 'A'*10.
        let rect = wt_rect(0, 1, 10, 2);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 5, 1);
        erase_in_line(&mut this, EraseType::FromBeginning);
        let glyphs = this.terminal.get_row(1).glyphs.clone();
        for col in 0..6 {
            assert_eq!(
                glyphs.get(col as usize),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space",
                col
            );
        }
        for col in 6..10 {
            assert_eq!(
                glyphs.get(col as usize),
                Some(&('A', 1)),
                "col {} expected A",
                col
            );
        }
    }

    #[test]
    fn erase_in_line_from_cursor_clears_right_part() {
        // FromCursor = WT ToEnd: clears `[col, lineWidth-1]` (right side).
        let mut this = make_dispatch(10, 3);
        let rect = wt_rect(0, 1, 10, 2);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 5, 1);
        erase_in_line(&mut this, EraseType::ToEnd);
        let glyphs = this.terminal.get_row(1).glyphs.clone();
        for col in 5..10 {
            assert_eq!(
                glyphs.get(col as usize),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space",
                col
            );
        }
        for col in 0..5 {
            assert_eq!(
                glyphs.get(col as usize),
                Some(&('A', 1)),
                "col {} expected A",
                col
            );
        }
    }

    #[test]
    fn erase_in_line_all_clears_whole_row() {
        let mut this = make_dispatch(10, 3);
        let rect = wt_rect(0, 1, 10, 2);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 3, 1);
        erase_in_line(&mut this, EraseType::All);
        let glyphs = this.terminal.get_row(1).glyphs.clone();
        for col in 0..10 {
            assert_eq!(glyphs.get(col as usize), Some(&(UNICODE_SPACE, 1)));
        }
    }

    #[test]
    fn erase_characters_clears_n_cells_from_cursor() {
        let mut this = make_dispatch(10, 2);
        let rect = wt_rect(0, 0, 10, 1);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 3, 0);
        erase_characters(&mut this, 4);
        let glyphs = this.terminal.get_row(0).glyphs.clone();
        // cols 3,4,5,6 should be space; rest remain 'A'.
        for col in 3..7 {
            assert_eq!(
                glyphs.get(col as usize),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space",
                col
            );
        }
        assert_eq!(glyphs.get(2), Some(&('A', 1)));
        assert_eq!(glyphs.get(7), Some(&('A', 1)));
    }

    #[test]
    fn erase_in_display_to_cursor_clears_above_and_partial() {
        // ToCursor = WT FromBeginning: clear rows above cursor + cursor row [0..col].
        let mut this = make_dispatch(5, 5);
        let rect = wt_rect(0, 0, 5, 5);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 2, 2);
        erase_in_display(&mut this, EraseType::FromBeginning);
        // Rows 0,1 above cursor cleared.
        for row in 0..2 {
            let g = this.terminal.get_row(row).glyphs.clone();
            for col in 0..5 {
                assert_eq!(
                    g.get(col as usize),
                    Some(&(UNICODE_SPACE, 1)),
                    "row {} col {} expected space",
                    row,
                    col
                );
            }
        }
        // Row 2 cols [0, 2] cleared; [3, 4] remain A.
        let row2 = this.terminal.get_row(2).glyphs.clone();
        for col in 0..=2 {
            assert_eq!(
                row2.get(col as usize),
                Some(&(UNICODE_SPACE, 1)),
                "row2 col {} expected space",
                col
            );
        }
        for col in 3..5 {
            assert_eq!(
                row2.get(col as usize),
                Some(&('A', 1)),
                "row2 col {} expected A",
                col
            );
        }
        // Rows 3,4 below cursor unchanged.
        for row in 3..5 {
            for col in 0..5 {
                assert_eq!(
                    this.terminal.get_row(row).glyphs.get(col as usize),
                    Some(&('A', 1))
                );
            }
        }
    }

    #[test]
    fn erase_in_display_from_cursor_clears_below_and_partial() {
        // FromCursor = WT ToEnd: clear cursor row from col + rows below.
        let mut this = make_dispatch(5, 5);
        let rect = wt_rect(0, 0, 5, 5);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 2, 1);
        erase_in_display(&mut this, EraseType::ToEnd);
        // Row 1 col [2..5) cleared (inclusive [2, 4]).
        let row1 = this.terminal.get_row(1).glyphs.clone();
        assert_eq!(row1.first(), Some(&('A', 1)));
        assert_eq!(row1.get(1), Some(&('A', 1)));
        for col in 2..5 {
            assert_eq!(
                row1.get(col as usize),
                Some(&(UNICODE_SPACE, 1)),
                "row1 col {} expected space",
                col
            );
        }
        // Rows 2..4 fully cleared.
        for row in 2..5 {
            let g = this.terminal.get_row(row).glyphs.clone();
            for col in 0..5 {
                assert_eq!(
                    g.get(col as usize),
                    Some(&(UNICODE_SPACE, 1)),
                    "row {} col {} expected space",
                    row,
                    col
                );
            }
        }
        // Row 0 unchanged.
        for col in 0..5 {
            assert_eq!(
                this.terminal.get_row(0).glyphs.get(col as usize),
                Some(&('A', 1))
            );
        }
    }

    #[test]
    fn erase_in_display_all_clears_entire_page() {
        let mut this = make_dispatch(5, 5);
        let rect = wt_rect(0, 0, 5, 5);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 2, 2);
        erase_in_display(&mut this, EraseType::All);
        for row in 0..5 {
            let g = this.terminal.get_row(row).glyphs.clone();
            for col in 0..5 {
                assert_eq!(
                    g.get(col as usize),
                    Some(&(UNICODE_SPACE, 1)),
                    "row {} col {} expected space",
                    row,
                    col
                );
            }
        }
    }

    #[test]
    fn selective_erase_in_line_skips_protected_row() {
        // ToCursor = WT FromBeginning → clears left part; protected rows stay 'A'.
        let mut this = make_dispatch(10, 2);
        let rect = wt_rect(0, 0, 10, 1);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        // Mark row 0 protected.
        let mut attr = *this.terminal.get_row(0).attr();
        attr.set_protected(true);
        this.terminal.get_row_mut(0).set_attr(attr);
        set_cursor(&mut this, 3, 0);
        selective_erase_in_line(&mut this, EraseType::FromBeginning);
        // Protected row → glyphs should remain 'A'.
        for col in 0..10 {
            let g = this.terminal.get_row(0).glyphs.get(col).copied();
            assert_eq!(g, Some(('A', 1)), "col {} should remain A", col);
        }
    }

    #[test]
    fn selective_erase_in_line_clears_unprotected_cells() {
        // ToCursor = WT FromBeginning → clears `[0, col]` inclusive (left side).
        let mut this = make_dispatch(10, 2);
        let rect = wt_rect(0, 0, 10, 1);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 3, 0);
        selective_erase_in_line(&mut this, EraseType::FromBeginning);
        let g = this.terminal.get_row(0).glyphs.clone();
        for col in 0..4 {
            assert_eq!(
                g.get(col as usize),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space",
                col
            );
        }
        for col in 4..10 {
            assert_eq!(
                g.get(col as usize),
                Some(&('A', 1)),
                "col {} expected A",
                col
            );
        }
    }

    #[test]
    fn scroll_rect_vertically_down_within_full_width() {
        let mut this = make_dispatch(4, 5);
        // Populate row 1 with 'A's.
        let rect = wt_rect(0, 1, 4, 2);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        // Scroll entire buffer down by 1: rect = (0,0,4,5) inclusive.
        let scroll_rect = wt_rect(0, 0, 4, 5);
        scroll_rect_vertically(&mut this, &scroll_rect, 1);
        // After down-scroll, row 1 should be cleared (revealed), row 2 has the
        // prior row-1 content 'A's.
        let row1 = this.terminal.get_row(1).glyphs.clone();
        let row2 = this.terminal.get_row(2).glyphs.clone();
        for col in 0..4 {
            assert_eq!(
                row1.get(col),
                Some(&(UNICODE_SPACE, 1)),
                "row1 col {} expected space (revealed)",
                col
            );
            assert_eq!(
                row2.get(col),
                Some(&('A', 1)),
                "row2 col {} expected A (shifted)",
                col
            );
        }
    }

    #[test]
    fn scroll_rect_horizontally_right_shifts_content() {
        let mut this = make_dispatch(6, 1);
        let row_rect = wt_rect(0, 0, 6, 1);
        fill_rect(
            &mut this,
            &row_rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        // Scroll right by 2 within cols [0..6].
        let scroll_rect = wt_rect(0, 0, 6, 1);
        scroll_rect_horizontally(&mut this, &scroll_rect, 2);
        let g = this.terminal.get_row(0).glyphs.clone();
        // Revealed columns 0,1 should be space; shifted columns 2,3,4,5 should be 'A'.
        for col in 0..2 {
            assert_eq!(
                g.get(col),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space",
                col
            );
        }
        for col in 2..6 {
            assert_eq!(g.get(col), Some(&('A', 1)), "col {} expected A", col);
        }
    }

    #[test]
    fn insert_character_shifts_row_right_at_cursor() {
        let mut this = make_dispatch(6, 1);
        let rect = wt_rect(0, 0, 6, 1);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 1, 0);
        insert_character(&mut this, 2);
        let g = this.terminal.get_row(0).glyphs.clone();
        // cols 1,2 should be space (inserted); col 0 stayed 'A'; cols 3..6 should
        // be the previous right-margin content shifted right.
        assert_eq!(g.first(), Some(&('A', 1)));
        for col in 1..3 {
            assert_eq!(
                g.get(col),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space (inserted)",
                col
            );
        }
        for col in 3..6 {
            assert_eq!(g.get(col), Some(&('A', 1)), "col {} expected A", col);
        }
    }

    #[test]
    fn delete_character_shifts_row_left_at_cursor() {
        let mut this = make_dispatch(6, 1);
        let rect = wt_rect(0, 0, 6, 1);
        fill_rect(
            &mut this,
            &rect,
            "A",
            &crate::attrs::TextAttribute::default(),
        );
        set_cursor(&mut this, 1, 0);
        delete_character(&mut this, 2);
        let g = this.terminal.get_row(0).glyphs.clone();
        // After DCH 2: cols 1..3 hold old cols 3..5 ('A'); cols 4..5 filled with
        // space (revealed at right margin).
        assert_eq!(g.first(), Some(&('A', 1)));
        for col in 1..4 {
            assert_eq!(g.get(col), Some(&('A', 1)), "col {} expected A", col);
        }
        for col in 4..6 {
            assert_eq!(
                g.get(col),
                Some(&(UNICODE_SPACE, 1)),
                "col {} expected space",
                col
            );
        }
    }

    // —— Sanity check on helper signature shape so siblings compile against us.

    #[test]
    fn free_fn_arity_is_stable() {
        fn _arity_insert(_: fn(&mut TerminalDispatch, VtInt)) {}
        fn _arity_delete(_: fn(&mut TerminalDispatch, VtInt)) {}
        fn _arity_ech(_: fn(&mut TerminalDispatch, VtInt)) {}
        fn _arity_ed(_: fn(&mut TerminalDispatch, EraseType)) {}
        fn _arity_el(_: fn(&mut TerminalDispatch, EraseType)) {}
        fn _arity_sed(_: fn(&mut TerminalDispatch, EraseType)) {}
        fn _arity_sel(_: fn(&mut TerminalDispatch, EraseType)) {}
        _arity_insert(insert_character);
        _arity_delete(delete_character);
        _arity_ech(erase_characters);
        _arity_ed(erase_in_display);
        _arity_el(erase_in_line);
        _arity_sed(selective_erase_in_display);
        _arity_sel(selective_erase_in_line);
    }

    // Bump `Row`/`Cursor` references to avoid dead-code warnings when only
    // getters are exercised (tests run with full module building).
    #[test]
    fn row_and_cursor_smoke() {
        let _ = Row::new(10, crate::attrs::TextAttribute::default());
        let _ = Cursor::default();
    }
}
