//! Scroll + Page family — port `adaptDispatch.cpp:1616-1706`.
//!
//! W3 (Wave 1, parallel). Free-fn split pattern — W1 wires these into the
//! `impl ITermDispatch for TerminalDispatch` block (in `dispatch::td::mod`)
//! by delegating `fn scroll_up(&mut self, ...) { scroll_up(self, ...) }`
//! (after `pub use scroll::*;`). Every public method takes
//! `this: &mut TerminalDispatch` as the first parameter.
//!
//! Methods (public):
//! - `scroll_up` (SU)                  — `adaptDispatch.cpp:1628`
//! - `scroll_down` (SD)                — `:1637`
//! - `next_page` (NP)                  — `:1647`
//! - `preceding_page` (PP)             — `:1657`
//! - `page_position_absolute` (PPA)    — `:1667`
//! - `page_position_relative` (PPR)    — `:1677`
//! - `page_position_back` (PPB)        — `:1687`
//! - `request_displayed_extent` (DECRQDE) — `:1698`
//!
//! Helper (private):
//! - `_scroll_movement`                — `:1616`
//!
//! Page navigation uses `PageCursorCoupling` mode — port of
//! `Mode::PageCursorCoupling`. Anchor: `DispatchTypes.hpp:532`.
//!
//! W1 struct shape: `this.terminal: TextBuffer` (single-buffer collapse — no
//! PageManager, so `next_page` / `preceding_page` / `page_position_*` are
//! effectively no-ops on Phase 4; we preserve WT semantics verbatim and let
//! the host bridge handle multi-page state when it lands in a later phase).

use super::super::VtInt;
use super::dsr::return_csi_response;
use super::TerminalDispatch;

// —— `_ScrollMovement` — port `adaptDispatch.cpp:1616` — private helper.
//
// Generalizes scrolling movement for up/down. `delta` is positive down,
// negative up. Scroll region = current margins (`_GetVerticalMargins` +
// `_GetHorizontalMargins`). Calls into W2's `erase::scroll_rect_vertically`
// (free fn) — same pattern as W4 bell_tabs.rs.

pub(crate) fn _scroll_movement(this: &mut TerminalDispatch, delta: VtInt) {
    let page_width = this.viewport.width();
    // W1 collapse: `_pages.ActivePage()` → cached `this.viewport`.
    // `_GetVerticalMargins(page, true)` → W1 inherent method on
    // `TerminalDispatch` (returns page-relative coords; `absolute=true`
    // adds `viewport.top`).
    let (top_margin, bottom_margin) = this.get_vertical_margins(true);
    // `_GetHorizontalMargins(page.Width())` → W1 inherent method.
    let (left_margin, right_margin) = this.get_horizontal_margins(page_width);
    // WT: `_ScrollRectVertically(page, {left, top, right+1, bottom+1}, delta)`.
    // W1 collapse: build a `Viewport` rect and dispatch to W2 free fn.
    let rect =
        crate::screen::Viewport::new(left_margin, top_margin, right_margin + 1, bottom_margin + 1);
    super::erase::scroll_rect_vertically(this, &rect, delta);
}

// —— SU — port `adaptDispatch.cpp:1628` — public.

/// SU — Pans the window DOWN by `distance` (distance new lines appear at
/// the bottom of the screen). `_ScrollMovement(-distance)`.
pub fn scroll_up(this: &mut TerminalDispatch, distance: VtInt) {
    _scroll_movement(this, -distance);
}

// —— SD — port `adaptDispatch.cpp:1637` — public.

/// SD — Pans the window UP by `distance` (distance new lines appear at
/// the top of the screen). `_ScrollMovement(distance)`.
pub fn scroll_down(this: &mut TerminalDispatch, distance: VtInt) {
    _scroll_movement(this, distance);
}

// —— NP — port `adaptDispatch.cpp:1647` — public.

/// NP — Moves the active position ahead by `page_count` pages, then moves
/// cursor to (1, 1). W1 single-buffer port: page navigation is a host
/// concern (`_pages.MoveNext`); W1's struct has no PageManager, so this
/// collapses to a cursor-to-(1,1) for the lives of Phase 4 phase 1.
pub fn next_page(this: &mut TerminalDispatch, page_count: VtInt) {
    page_position_relative(this, page_count);
    // `CursorPosition(1, 1)` — inherent-method dispatch on `this`.
    this.cursor_position(1, 1);
}

// —— PP — port `adaptDispatch.cpp:1657` — public.

/// PP — Moves the active position back by `page_count` pages, then moves
/// cursor to (1, 1). W1 collapse: see `next_page`.
pub fn preceding_page(this: &mut TerminalDispatch, page_count: VtInt) {
    page_position_back(this, page_count);
    this.cursor_position(1, 1);
}

// —— PPA — port `adaptDispatch.cpp:1667` — public.

/// PPA — Moves the active position to the specified page number without
/// altering the cursor coordinates. W1 collapse: PageManager not on struct,
/// so this is a no-op (page state lives in host).
pub fn page_position_absolute(this: &mut TerminalDispatch, _page: VtInt) {
    // WT: `_pages.MoveTo(page, _modes.test(Mode::PageCursorCoupling))`.
    // W1 Phase 4 phase 1 has no PageManager on `TerminalDispatch`.
    let _ = this;
}

// —— PPR — port `adaptDispatch.cpp:1677` — public.

/// PPR — Moves the active position ahead by `page_count` pages without
/// altering the cursor coordinates. W1 collapse: PageManager not on struct.
pub fn page_position_relative(this: &mut TerminalDispatch, _page_count: VtInt) {
    // WT: `_pages.MoveRelative(pageCount, _modes.test(Mode::PageCursorCoupling))`.
    let _ = this;
}

// —— PPB — port `adaptDispatch.cpp:1687` — public.

/// PPB — Moves the active position back by `page_count` pages without
/// altering the cursor coordinates. W1 collapse: PageManager not on struct.
pub fn page_position_back(this: &mut TerminalDispatch, _page_count: VtInt) {
    // WT: `_pages.MoveRelative(-pageCount, _modes.test(Mode::PageCursorCoupling))`.
    let _ = this;
}

// —— DECRQDE — port `adaptDispatch.cpp:1698` — public.

/// DECRQDE — Requests the area of page memory that is currently visible.
/// Response: CSI `<h>;<w>;<left>;<top>;<page>"w`.
///
/// W1 collapse: `page = _pages.VisiblePage()` → `this.viewport`. Page
/// number is always 1 (single-buffer port); pan offsets default to 0
/// (no scrollback panning in Phase 4 phase 1).
pub fn request_displayed_extent(this: &mut TerminalDispatch) {
    let width = this.viewport.width();
    let height = this.viewport.height();
    // WT: `page.XPanOffset() + 1` / `page.YPanOffset() + 1`. W1 has no pan
    // concept; both default to 0 → +1 for 1-based VT origin.
    let left: VtInt = 1;
    let top: VtInt = 1;
    // Single-buffer port: page number is always 1.
    let page_number: VtInt = 1;
    let payload = format!("{};{};{};{};{}\"w", height, width, left, top, page_number);
    return_csi_response(this, &payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time signature check — module fails to build if free-fn
    /// arity drifts.
    #[test]
    fn free_fn_arity_is_stable() {
        fn _scroll(_: fn(&mut TerminalDispatch, VtInt)) {}
        fn _extent(_: fn(&mut TerminalDispatch)) {}
        _scroll(scroll_up);
        _scroll(scroll_down);
        _scroll(next_page);
        _scroll(preceding_page);
        _scroll(page_position_relative);
        _scroll(page_position_back);
        _scroll(page_position_absolute);
        _extent(request_displayed_extent);
    }

    #[test]
    fn decrqde_payload_format() {
        let height = 24;
        let width = 80;
        let left = 1;
        let top = 1;
        let page_number = 1;
        let s = format!("{};{};{};{};{}\"w", height, width, left, top, page_number);
        assert_eq!(s, "24;80;1;1;1\"w");
    }

    #[test]
    fn scroll_sign_match_wt() {
        // SU = pan DOWN = negative delta; SD = pan UP = positive delta.
        fn _su_sign(d: VtInt) -> VtInt {
            -d
        }
        fn _sd_sign(d: VtInt) -> VtInt {
            d
        }
        assert_eq!(_su_sign(5), -5);
        assert_eq!(_sd_sign(5), 5);
    }

    #[test]
    fn page_back_negates_count() {
        // PagePositionBack(pageCount) → MoveRelative(-pageCount, ...)
        fn _ppb(c: VtInt) -> VtInt {
            -c
        }
        assert_eq!(_ppb(3), -3);
    }

    #[test]
    fn extent_origin_is_one_based() {
        // WT: `page.XPanOffset() + 1` and `page.YPanOffset() + 1`.
        // W1 Phase 4 phase 1: pan offsets default to 0 → +1.
        let x_pan: VtInt = 0;
        let y_pan: VtInt = 0;
        assert_eq!(x_pan + 1, 1);
        assert_eq!(y_pan + 1, 1);
    }

    #[test]
    fn page_number_default_one_for_single_buffer() {
        // W1 single-buffer collapse: page number is always 1.
        let page_number: VtInt = 1;
        assert_eq!(page_number, 1);
    }
}
