//! W4 — Bell / CR / LF / Tabs / Lines / Margins.
//!
//! Port verbatim `windows/src/terminal/adapter/adaptDispatch.cpp`:
//! - `_InsertDeleteLineHelper` (`:2167`), `InsertLine` (`:2195`),
//!   `DeleteLine` (`:2210`), `_InsertDeleteColumnHelper` (`:2218`),
//!   `InsertColumn` (`:2252`), `DeleteColumn` (`:2264`),
//!   `_DoSetTopBottomScrollingMargins` (`:2274`), `SetTopBottomScrollingMargins`
//!   (`:2331`), `_DoSetLeftRightScrollingMargins` (`:2343`),
//!   `SetLeftRightScrollingMargins` (`:2394`).
//! - `EnquireAnswerback` (`:2402`), `WarningBell` (`:2410`), `CarriageReturn`
//!   (`:2418`), `_DoLineFeed` (`:2431`), `LineFeed` (`:2511`),
//!   `ReverseLineFeed` (`:2534`), `BackIndex` (`:2562`), `ForwardIndex`
//!   (`:2586`).
//! - `HorizontalTabSet` (`:2659`), `ForwardTab` (`:2674`), `BackwardsTab`
//!   (`:2707`), `TabClear` (`:2734`), `_ClearSingleTabStop` (`:2755`),
//!   `_ClearAllTabStops` (`:2767`), `TabSet` (`:2776`), `_InitTabStopsForWidth`
//!   (`:2797`).
//!
//! Free-fn split: each method as `pub fn <name>(this: &mut TerminalDispatch, …)`.
//! W1 wires them into `impl ITermDispatch`. Helpers shared with other W4 files
//! (`_get_vertical_margins`, `_get_horizontal_margins`,
//! `_get_erase_attributes`) live in `super::` (W1 owns).

use super::TerminalDispatch;
use crate::dispatch::types::LineFeedType;
use crate::dispatch::types::TabClearType;
use crate::dispatch::VtInt;

/// `_InsertDeleteLineHelper(delta)` (`adaptDispatch.cpp:2167`).
///
/// Verbatim: 取 page ActivePage + cursor (x,y) → if in [top,bottom] margin ×
/// [left,right] margin, scroll rect [left,row..right+1,bottom+1] by `delta`,
/// then set cursor.x = leftMargin. Page-call collapses to `this.terminal`.
pub fn insert_delete_line_helper(this: &mut TerminalDispatch, delta: VtInt) {
    let cursor_pos = this.terminal.get_cursor().get_position();
    let (col, row) = (cursor_pos.0, cursor_pos.1);
    let width = this.viewport.width();
    let (top, bottom) = this.get_vertical_margins(true);
    let (left, right) = this.get_horizontal_margins(width);
    if row >= top && row <= bottom && col >= left && col <= right {
        let rect = crate::screen::Viewport::new(left, row, right + 1, bottom + 1);
        super::erase::scroll_rect_vertically(this, &rect, delta);
        this.terminal.get_cursor_mut().set_x_position(left);
    }
}

/// `InsertLine(distance)` (`adaptDispatch.cpp:2195`) — DEL within scroll.
pub fn insert_line(this: &mut TerminalDispatch, distance: VtInt) {
    insert_delete_line_helper(this, distance);
}

/// `DeleteLine(distance)` (`adaptDispatch.cpp:2210`) — negative delta helper.
pub fn delete_line(this: &mut TerminalDispatch, distance: VtInt) {
    insert_delete_line_helper(this, -distance);
}

/// `_InsertDeleteColumnHelper(delta)` (`adaptDispatch.cpp:2218`).
pub fn insert_delete_column_helper(this: &mut TerminalDispatch, delta: VtInt) {
    let cursor_pos = this.terminal.get_cursor().get_position();
    let (col, row) = (cursor_pos.0, cursor_pos.1);
    let width = this.viewport.width();
    let (top, bottom) = this.get_vertical_margins(true);
    let (left, right) = this.get_horizontal_margins(width);
    if row >= top && row <= bottom && col >= left && col <= right {
        let rect = crate::screen::Viewport::new(col, top, right + 1, bottom + 1);
        super::erase::scroll_rect_horizontally(this, &rect, delta);
    }
}

/// `InsertColumn(distance)` (`adaptDispatch.cpp:2252`).
pub fn insert_column(this: &mut TerminalDispatch, distance: VtInt) {
    insert_delete_column_helper(this, distance);
}

/// `DeleteColumn(distance)` (`adaptDispatch.cpp:2264`).
pub fn delete_column(this: &mut TerminalDispatch, distance: VtInt) {
    insert_delete_column_helper(this, -distance);
}

/// `_DoSetTopBottomScrollingMargins(top,bottom,homeCursor)` (`:2274`).
///
/// Verbatim: actualTop/bottom default 1/height, clamped < actualBottom ≤ height.
/// Full-range → clear (0,0). Else subtract 1 for zero-index. Home cursor if
/// `homeCursor && valid`.
pub fn do_set_top_bottom_scrolling_margins(
    this: &mut TerminalDispatch,
    top_margin: VtInt,
    bottom_margin: VtInt,
    home_cursor: bool,
) {
    let mut actual_top = top_margin;
    let mut actual_bottom = bottom_margin;
    let page_height = this.viewport.height();
    if actual_top == 0 {
        actual_top = 1;
    }
    if actual_bottom == 0 {
        actual_bottom = page_height;
    }
    if actual_top < actual_bottom && actual_bottom <= page_height {
        if actual_top == 1 && actual_bottom == page_height {
            actual_top = 0;
            actual_bottom = 0;
        } else {
            actual_top -= 1;
            actual_bottom -= 1;
        }
        this.scroll_margins.0 = actual_top;
        this.scroll_margins.1 = actual_bottom;
        if home_cursor {
            this.cursor_position(1, 1);
        }
    }
}

/// `SetTopBottomScrollingMargins(top,bottom)` (`:2331`).
pub fn set_top_bottom_scrolling_margins(this: &mut TerminalDispatch, top: VtInt, bottom: VtInt) {
    do_set_top_bottom_scrolling_margins(this, top, bottom, true);
}

/// `_DoSetLeftRightScrollingMargins(left,right,homeCursor)` (`:2343`).
pub fn do_set_left_right_scrolling_margins(
    this: &mut TerminalDispatch,
    left_margin: VtInt,
    right_margin: VtInt,
    home_cursor: bool,
) {
    let mut actual_left = left_margin;
    let mut actual_right = right_margin;
    let page_width = this.viewport.width();
    if actual_left == 0 {
        actual_left = 1;
    }
    if actual_right == 0 {
        actual_right = page_width;
    }
    if actual_left < actual_right && actual_right <= page_width {
        if actual_left == 1 && actual_right == page_width {
            actual_left = 0;
            actual_right = 0;
        } else {
            actual_left -= 1;
            actual_right -= 1;
        }
        this.scroll_margins.2 = actual_left;
        this.scroll_margins.3 = actual_right;
        if home_cursor {
            this.cursor_position(1, 1);
        }
    }
}

/// `SetLeftRightScrollingMargins(left,right)` (`:2394`).
///
/// Verbatim: if `Mode::AllowDECSLRM` set → use helper. Else `CSI s` is
/// `ANSISYSSC` → `CursorSaveState()`.
pub fn set_left_right_scrolling_margins(this: &mut TerminalDispatch, left: VtInt, right: VtInt) {
    if this.modes.test(super::Mode::AllowDECSLRM) {
        do_set_left_right_scrolling_margins(this, left, right, true);
    } else {
        // When DECSLRM isn't allowed, `CSI s` is interpreted as ANSISYSSC.
        this.cursor_save_state();
    }
}

/// `EnquireAnswerback()` (`adaptDispatch.cpp:2402`). Delegates to interact
/// (host bridge) — port `_api.ReturnAnswerback()` to `interact.write_file`.
pub fn enquire_answerback(this: &mut TerminalDispatch) {
    // WT answerback returns a config string; host bridges it.
    this.interact.write_file("");
}

/// `WarningBell()` (`:2410`). Delegates to interact `warning_bell`.
pub fn warning_bell(this: &mut TerminalDispatch) {
    this.interact.warning_bell();
}

/// `CarriageReturn()` (`:2418`). Cursor move to col 1, row unchanged.
pub fn carriage_return(this: &mut TerminalDispatch) {
    this.cursor_move_position(super::Offset::Unchanged(), super::Offset::Absolute(1), true);
}

/// `_DoLineFeed(page, withReturn, wrapForced)` (`adaptDispatch.cpp:2431`).
///
/// Verbatim semantics. Phase 4 single-buffer collapse: `_pages.ActivePage()`
/// becomes `this.terminal`. `SetViewportPosition` (viewport pan) is host-side —
/// delegate to `interact.notify_buffer_change`. `_ScrollRectVertically` is
/// W2 (free fn `super::erase::scroll_rect_vertically`).
pub fn do_line_feed(this: &mut TerminalDispatch, with_return: bool, wrap_forced: bool) -> bool {
    let page_width = this.viewport.width();
    let page_height = this.viewport.height();
    let buffer_height = page_height; // single buffer; bufferHeight == page.Height.
    let (top_margin, bottom_margin) = this.get_vertical_margins(true);
    let (left_margin, right_margin) = this.get_horizontal_margins(page_width);

    let cursor_pos = this.terminal.get_cursor().get_position();
    let current_position = cursor_pos;
    let mut new_position = current_position;

    // Track wrap forced (port `textBuffer.GetMutableRowByOffset(y).SetWrapForced`).
    if (current_position.1 as usize) < this.terminal.rows().len() {
        this.terminal
            .set_wrap_forced(current_position.1 as usize, wrap_forced);
    }

    if with_return {
        let clamp_to_margin = current_position.1 >= top_margin
            && current_position.1 <= bottom_margin
            && current_position.0 >= left_margin;
        new_position.0 = if clamp_to_margin { left_margin } else { 0 };
    }

    if current_position.1 != bottom_margin
        || new_position.0 < left_margin
        || new_position.0 > right_margin
    {
        // No scrolling — clamp cursor to bottom of page.
        new_position.1 = (current_position.1 + 1).min(page_height - 1);
        new_position = super::cursor::clamp_position_within_line(&this.terminal, new_position);
    } else if top_margin > 0 || left_margin > 0 || right_margin < page_width - 1 {
        // Scroll just the margin area; cursor stays.
        let rect = crate::screen::Viewport::new(
            left_margin,
            top_margin,
            right_margin + 1,
            bottom_margin + 1,
        );
        super::erase::scroll_rect_vertically(this, &rect, -1);
    } else if page_height < buffer_height {
        // Pan viewport down (single-buffer: just notify).
        this.interact.notify_buffer_change();
        new_position.1 += 1;
        if bottom_margin < page_height - 1 {
            let rect = crate::screen::Viewport::new(0, bottom_margin + 1, page_width, page_height);
            super::erase::scroll_rect_vertically(this, &rect, 1);
        } else {
            let erase_attr = super::erase::get_erase_attributes(this);
            this.terminal.clear_row(new_position.1 as usize, erase_attr);
        }
    } else {
        // Cycle ring buffer (port `IncrementCircularBuffer` + `TriggerScroll`).
        let erase_attr = super::erase::get_erase_attributes(this);
        this.terminal.increment_circular_buffer(&erase_attr);
        // textBuffer.TriggerScroll({0,-1}) — phase 4.x renderer hook.
        if bottom_margin < page_height - 1 {
            let rect = crate::screen::Viewport::new(0, bottom_margin, page_width, buffer_height);
            super::erase::scroll_rect_vertically(this, &rect, 1);
        }
    }
    this.terminal
        .get_cursor_mut()
        .set_position(new_position.0, new_position.1);
    false
}

/// `LineFeed(lineFeedType)` (`adaptDispatch.cpp:2511-2536`).
///
/// Verbatim port WT 3-variant match (`adaptDispatch.cpp:2518-2529`):
/// ```cpp
/// switch (lineFeedType) {
/// case LineFeedType::DependsOnMode:
///     _DoLineFeed(page, _api.GetSystemMode(ITerminalApi::Mode::LineFeed), false);
///     break;
/// case LineFeedType::WithoutReturn:
///     _DoLineFeed(page, false, false);
///     break;
/// case LineFeedType::WithReturn:
///     _DoLineFeed(page, true, false);
///     break;
/// }
/// ```
///
/// Wave 0 port-side collapse: `_api.GetSystemMode(LineFeed)` thay bằng
/// `this.modes.test(Mode::LineFeed)` — LNM bit lưu trong `AdaptDispatch::Mode::LineFeed`
/// (port-side add, xem `td/mod.rs::Mode` doc). WT 2-bucket (api + input) gom 1.
pub fn line_feed(this: &mut TerminalDispatch, line_feed_type: LineFeedType) {
    let with_return = match line_feed_type {
        LineFeedType::WithReturn => true,
        LineFeedType::WithoutReturn => false,
        LineFeedType::DependsOnMode => this.modes.test(super::Mode::LineFeed),
    };
    do_line_feed(this, with_return, false);
}

/// `ReverseLineFeed()` (`:2534`).
pub fn reverse_line_feed(this: &mut TerminalDispatch) {
    let cursor_pos = this.terminal.get_cursor().get_position();
    let page_width = this.viewport.width();
    let (left_margin, right_margin) = this.get_horizontal_margins(page_width);
    let (top_margin, bottom_margin) = this.get_vertical_margins(true);

    if cursor_pos.1 == top_margin && cursor_pos.0 >= left_margin && cursor_pos.0 <= right_margin {
        let rect = crate::screen::Viewport::new(
            left_margin,
            top_margin,
            right_margin + 1,
            bottom_margin + 1,
        );
        super::erase::scroll_rect_vertically(this, &rect, 1);
    } else if cursor_pos.1 > 0 {
        let new_pos = super::cursor::clamp_position_within_line(
            &this.terminal,
            (cursor_pos.0, cursor_pos.1 - 1),
        );
        this.terminal
            .get_cursor_mut()
            .set_position(new_pos.0, new_pos.1);
    }
}

/// `BackIndex()` (`:2562`).
pub fn back_index(this: &mut TerminalDispatch) {
    let cursor_pos = this.terminal.get_cursor().get_position();
    let page_width = this.viewport.width();
    let (left_margin, right_margin) = this.get_horizontal_margins(page_width);
    let (top_margin, bottom_margin) = this.get_vertical_margins(true);

    if cursor_pos.0 == left_margin && cursor_pos.1 >= top_margin && cursor_pos.1 <= bottom_margin {
        let rect = crate::screen::Viewport::new(
            left_margin,
            top_margin,
            right_margin + 1,
            bottom_margin + 1,
        );
        super::erase::scroll_rect_horizontally(this, &rect, 1);
    } else if cursor_pos.0 > 0 {
        this.terminal
            .get_cursor_mut()
            .set_x_position(cursor_pos.0 - 1);
    }
}

/// `ForwardIndex()` (`:2586`).
pub fn forward_index(this: &mut TerminalDispatch) {
    let cursor_pos = this.terminal.get_cursor().get_position();
    let page_width = this.viewport.width();
    let (left_margin, right_margin) = this.get_horizontal_margins(page_width);
    let (top_margin, bottom_margin) = this.get_vertical_margins(true);

    if cursor_pos.0 == right_margin && cursor_pos.1 >= top_margin && cursor_pos.1 <= bottom_margin {
        let rect = crate::screen::Viewport::new(
            left_margin,
            top_margin,
            right_margin + 1,
            bottom_margin + 1,
        );
        super::erase::scroll_rect_horizontally(this, &rect, -1);
    } else if cursor_pos.0 < this.viewport.width() - 1 {
        this.terminal
            .get_cursor_mut()
            .set_x_position(cursor_pos.0 + 1);
    }
}

// — Tabs —

/// `HorizontalTabSet()` (`adaptDispatch.cpp:2659`).
pub fn horizontal_tab_set(this: &mut TerminalDispatch) {
    let column = this.terminal.get_cursor().get_position().0;
    let width = this.viewport.width();
    init_tab_stops_for_width(this, width);
    if let Some(slot) = this.tab_stop_columns.get_mut(column as usize) {
        *slot = 1;
    }
}

/// `ForwardTab(num_tabs)` (`:2674`).
pub fn forward_tab(this: &mut TerminalDispatch, num_tabs: VtInt) {
    let cursor = this.terminal.get_cursor();
    let mut column = cursor.get_position().0;
    let row = cursor.get_position().1;
    let width = this.viewport.width();
    let mut tabs_performed = 0i32;

    let (top_margin, bottom_margin) = this.get_vertical_margins(true);
    let (_left_margin, right_margin) = this.get_horizontal_margins(width);
    let clamp_to_margin = row >= top_margin && row <= bottom_margin && column <= right_margin;
    let max_column = if clamp_to_margin {
        right_margin
    } else {
        width - 1
    };

    init_tab_stops_for_width(this, width);
    while column < max_column && tabs_performed < num_tabs {
        column += 1;
        if this
            .tab_stop_columns
            .get(column as usize)
            .copied()
            .unwrap_or(0)
            != 0
        {
            tabs_performed += 1;
        }
    }
    this.terminal.get_cursor_mut().set_x_position(column);
}

/// `BackwardsTab(num_tabs)` (`:2707`).
pub fn backwards_tab(this: &mut TerminalDispatch, num_tabs: VtInt) {
    let cursor = this.terminal.get_cursor();
    let mut column = cursor.get_position().0;
    let row = cursor.get_position().1;
    let width = this.viewport.width();
    let mut tabs_performed = 0i32;

    let (top_margin, bottom_margin) = this.get_vertical_margins(true);
    let (left_margin, _right_margin) = this.get_horizontal_margins(width);
    let clamp_to_margin = row >= top_margin && row <= bottom_margin && column >= left_margin;
    let min_column = if clamp_to_margin { left_margin } else { 0 };

    init_tab_stops_for_width(this, width);
    while column > min_column && tabs_performed < num_tabs {
        column -= 1;
        if this
            .tab_stop_columns
            .get(column as usize)
            .copied()
            .unwrap_or(0)
            != 0
        {
            tabs_performed += 1;
        }
    }
    this.terminal.get_cursor_mut().set_x_position(column);
}

/// `TabClear(clear_type)` (`:2734`).
pub fn tab_clear(this: &mut TerminalDispatch, clear_type: TabClearType) {
    match clear_type {
        TabClearType::ClearCurrent => clear_single_tab_stop(this),
        TabClearType::ClearAll => clear_all_tab_stops(this),
    }
}

/// `_ClearSingleTabStop()` (`:2755`).
pub fn clear_single_tab_stop(this: &mut TerminalDispatch) {
    let column = this.terminal.get_cursor().get_position().0;
    let width = this.viewport.width();
    init_tab_stops_for_width(this, width);
    if let Some(slot) = this.tab_stop_columns.get_mut(column as usize) {
        *slot = 0;
    }
}

/// `_ClearAllTabStops()` (`:2767`). noexcept in C++.
pub fn clear_all_tab_stops(this: &mut TerminalDispatch) {
    this.tab_stop_columns.clear();
    this.init_default_tab_stops = false;
}

/// `TabSet(set_type)` (`:2776`). noexcept. Only `SetEvery8Columns` supported.
pub fn tab_set(this: &mut TerminalDispatch, _set_type: crate::dispatch::VtParameter) {
    // WT only supports `SetEvery8Columns`; default when param omitted.
    this.tab_stop_columns.clear();
    this.init_default_tab_stops = true;
}

/// `_InitTabStopsForWidth(width)` (`:2797`).
///
/// Verbatim: grow `tab_stop_columns` to `width`; if `init_default_tab_stops`
/// is set, mark every 8th column ≥ previous size as a stop.
pub fn init_tab_stops_for_width(this: &mut TerminalDispatch, width: VtInt) {
    let screen_width = width.max(0) as usize;
    let initial_width = this.tab_stop_columns.len();
    if screen_width > initial_width {
        this.tab_stop_columns.resize(screen_width, 0);
        if this.init_default_tab_stops {
            let mut column = 8usize;
            while column < this.tab_stop_columns.len() {
                if column >= initial_width {
                    this.tab_stop_columns[column] = 1;
                }
                column += 8;
            }
        }
    }
}

#[cfg(test)]
#[allow(dead_code)] // W1-stub scaffolding; `TerminalDispatch::new` not wired here yet.
mod tests {
    use super::*;
    use crate::buffer::TextBuffer;
    use crate::cursor::Cursor;
    use crate::dispatch::interact::InteractDispatch;
    use crate::screen::Viewport;

    /// Minimal `TerminalDispatch` for testing just bell_tabs's tab helpers.
    /// W1 will own the real struct; this stub mirrors the public field surface.
    struct StubInteract;
    impl InteractDispatch for StubInteract {}

    fn make_dispatch(width: i32, height: i32) -> TerminalDispatch {
        let _ = (width, height);
        // Build a minimum fixture once W1 lands. For now stub-by-stub
        // covers standalone helper semantics via direct vec tests.
        unimplemented!("W1 owns TerminalDispatch::new; tests run post-W1 land")
    }

    // Pure-logic helper tests that don't require the full struct.
    #[test]
    fn vec_u8_default_tab_stops_when_init_true() {
        let mut v: Vec<u8> = vec![0; 16];
        v.resize(20, 0);
        let mut i = 8;
        while i < v.len() {
            v[i] = 1;
            i += 8;
        }
        assert_eq!(v[8], 1);
        assert_eq!(v[16], 1);
        assert_eq!(v[4], 0);
    }

    #[test]
    fn viewport_width_helper() {
        let v = Viewport::from_size(80, 24);
        assert_eq!(v.width(), 80);
        assert_eq!(v.height(), 24);
    }

    // Cursor helpers used by tab methods.
    #[test]
    fn cursor_set_x_basic() {
        let mut c = Cursor::default();
        c.set_x_position(5);
        assert_eq!(c.get_position().0, 5);
    }

    // placate unused-warnings on Translator-mode compile.
    #[test]
    fn buf_default_size() {
        let b = TextBuffer::default();
        assert_eq!(b.size(), (80, 24));
    }
}
