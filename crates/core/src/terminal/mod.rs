//! `Terminal` — port của `windows/src/cascadia/TerminalCore/Terminal.hpp`.
//!
//! Phase 09 / Loop 01: struct tối thiểu + scroll-notification seam (1/54 test
//! `wt_terminalcore__test_notify_scrolling`). `scroll.rs` giữ callback + delta
//! accumulate; `mod.rs` struct + constructor + `process_string` thin bridge.
//!
//! Verbatim-in-intent port — không phải line-by-line vì:
//! - WT `Terminal::Create` dựng `StateMachine` + `AdaptDispatch` parser; parser
//!   sống ở `control` `ControlCore`). Nay `process_string` làm direct
//!   `TextBuffer` writes — chỉ đủ cho scroll pattern (GH#5540 là scroll-delta
//!   overflow, không phải parser correctness). Xem ADR-0014.
//! - WT circle path nằm ở `AdaptDispatch::CursorLineFeed` (`adaptDispatch.cpp:2461`,
//!   `:2484`); ta fold vào `process_string::handle_linefeed`. Same semantics
//!   (pan viewport khi chưa tới cuối buffer; circle khi tới cuối).

pub mod scroll;

use std::cell::RefCell;
use std::rc::Rc;

use crate::buffer::TextBuffer;
use crate::screen::Viewport;
use crate::terminal::scroll::ScrollSeam;

/// Test-only constructor seam — mirror WT `Terminal::TestDummyMarker`
/// (`Terminal.hpp:83`). Tránh side-effects (settings, render hooks) trong
/// unit test.
#[derive(Debug, Default, Clone, Copy)]
pub struct TestDummyMarker;

/// `(i32, i32)` scroll delta — port `til::point`. `y = -1` =行了 lên 1 row.
pub type ScrollDelta = (i32, i32);

/// `Terminal` — port `Terminal.hpp:43-489`.
///
/// Phase 1 field tối thiểu cho `TestNotifyScrolling`. 188 `todo!()` ở core
/// không tăng (loop này green — zero `todo!()` add).
pub struct Terminal {
    /// `_mainBuffer` (`Terminal.hpp:411`). Page = 32 rows; khi circle, row 0
    /// đẩy vào `scrollback` (VecDeque) — port WT `IncrementCircularBuffer`.
    pub main_buffer: TextBuffer,
    /// `_mutableViewport` (`Terminal.hpp:413`) — view rect (inclusive).
    pub mutable_viewport: Viewport,
    /// `_scrollbackLines` (`Terminal.hpp:414`) — `TerminalHistoryLength`.
    pub scrollback_lines: i32,
    /// `_scrollOffset` (`Terminal.hpp:423`) — rows above viewport visible.
    pub scroll_offset: i32,
    /// `AlwaysNotifyOnBufferRotation` (`Terminal.hpp:127`).
    pub always_notify_on_buffer_rotation: bool,
    /// `_snapOnInput` — port WT field; loop 01 giữ default `true`.
    pub snap_on_input: bool,
    /// Scroll seam — callback + delta accumulate. Xem `scroll.rs`.
    pub scroll: ScrollSeam,
}

impl Terminal {
    /// `Terminal(TestDummyMarker)` ctor —
    /// port `Terminal::Create({w,h}, scrollbackLines, renderer)`.
    ///
    /// Khởi tạo `main_buffer` 32 row page (`TextBuffer::new(w, h, attr)`)
    /// + `scrollback_limit = scrollback_lines`. Total buffer height = view_h
    /// + scrollback_lines (mirror WT `termTb.GetSize().Height()`, line 49-50
    /// Terminal.cpp). Loop 01 không có renderer thực — dùng `ScrollSeam` seam.
    pub fn new_with_marker(
        _marker: TestDummyMarker,
        viewport_size: (i32, i32),
        scrollback_lines: i32,
    ) -> Self {
        Self::new(viewport_size, scrollback_lines)
    }

    /// `Terminal::Create(viewportSize, scrollbackLines, renderer)` ctor.
    pub fn new(viewport_size: (i32, i32), scrollback_lines: i32) -> Self {
        let (w, h) = viewport_size;
        let attr = crate::attrs::TextAttribute::default();
        let mut main_buffer = TextBuffer::new(w as u16, h as u16, attr);
        main_buffer.scrollback_limit = scrollback_lines.max(0) as usize;
        let mutable_viewport = Viewport::from_size(w, h);
        main_buffer.cursor.set_position(0, 0);
        Self {
            main_buffer,
            mutable_viewport,
            scrollback_lines,
            scroll_offset: 0,
            always_notify_on_buffer_rotation: false,
            snap_on_input: true,
            scroll: ScrollSeam::new(),
        }
    }

    /// Tổng chiều cao buffer (page + scrollback limit) — port
    /// WT `termTb.GetSize().Height()`. Mirror `Terminal.cpp:49` `bufferSize =
    /// (w, view_h + scrollbackLines)`.
    pub fn total_buffer_height(&self) -> i32 {
        let view_h = self.mutable_viewport.height();
        view_h + self.scrollback_lines.max(0)
    }

    /// `AlwaysNotifyOnBufferRotation(bool)` setter — port `Terminal.hpp:127`.
    pub fn always_notify_on_buffer_rotation(&mut self, v: bool) {
        self.always_notify_on_buffer_rotation = v;
    }

    /// `SetScrollPositionChangedCallback(pfn)` — port `Terminal.cpp:1170`.
    /// `ScrollSeam::set_callback` lưu `Box<dyn Fn(i32, i32, i32)>`.
    pub fn set_scroll_position_changed_callback(&mut self, pfn: Box<dyn Fn(i32, i32, i32)>) {
        self.scroll.set_callback(pfn);
    }

    /// Trap `TriggerScrollDelta` cho test — share `Rc<RefCell<Option<(i32,i32)>>>`
    /// để test inject và assert. Lane C wire real renderer phase sau (ADR-0014).
    pub fn trigger_scroll_delta_slot(&self) -> Rc<RefCell<Option<ScrollDelta>>> {
        self.scroll.delta_slot()
    }

    /// `Write(stringView)` (`Terminal.hpp:106`) — port `StateMachine::ProcessString`.
    ///
    /// LOOP-01 SIMPLIFICATION (ADR-0014): NOT VTE parser. Xử lý trực tiếp các
    /// escape sequence tối thiểu cần cho `TestNotifyScrolling` (`"X\r\n"`):
    /// - `\r` (CR) → `cursor.x = 0`.
    /// - `\n` (LF) → xem `handle_linefeed` (pan hoặc circle).
    /// - printable char → viết tại cursor (replace_text), cursor.x++.
    pub fn process_string(&mut self, s: &str) {
        for ch in s.chars() {
            match ch {
                '\r' => {
                    let y = self.main_buffer.cursor.position.1;
                    self.main_buffer.cursor.set_position(0, y);
                }
                '\n' => self.handle_linefeed(),
                c if !c.is_control() => self.write_printable(c),
                _ => {}
            }
        }
    }

    /// Viết một printable char tại cursor + advance. Defensive: nếu
    /// `cursor.x + 1 > width`, wrap xuống dòng (mirror WT AutoWrap default).
    /// Loop 01 chỉ cần path `"X\r\n"` (1 char/cell x 80 cols), không reach wrap.
    fn write_printable(&mut self, c: char) {
        let (w, _h) = self.main_buffer.size();
        let (cx, cy) = self.main_buffer.cursor.position;
        // Wrap khi hết width — chưa xảy ra trong test (X width 1, \r reset x).
        if cx >= w as i32 {
            let _ = (cx, c);
            return;
        }
        let row_idx = cy as usize;
        if row_idx >= self.main_buffer.rows.len() {
            return;
        }
        let col_idx = cx as usize;
        let row = &mut self.main_buffer.rows[row_idx];
        let (new_end, _, _) = row.replace_text(col_idx, w as usize, &c.to_string());
        let next_x = new_end as i32;
        self.main_buffer.cursor.set_position(next_x, cy);
    }

    /// Xử lý LF — port `AdaptDispatch::CursorLineFeed`
    /// (`adaptDispatch.cpp:2446-2505`). Viewport `bottomMargin` = inclusive
    /// bottom (viewport.top + view_h - 1). Khi cursor CHƯA bottom → chỉ
    /// `cursor.y++` (no scroll, no notify — mirror line 2451 `min(y+1, bottom-1)`).
    /// Khi cursor ở bottom:
    /// - `page.Bottom() < bufferHeight` (pan path, line 2461): viewport.top++
    ///   + cursor.y++ + `_NotifyScrollEvent` (qua `SetViewportPosition`).
    /// - else (circle path, line 2484): `IncrementCircularBuffer(fill)` +
    ///   `NotifyBufferRotation(1)` (notify iff `always_notify_on_buffer_rotation`)
    ///   + `TriggerScroll({0, -1})` (GH#5540 delta accumulate). Cursor.y giữ
    ///   nguyên.
    fn handle_linefeed(&mut self) {
        let view_h = self.mutable_viewport.height();
        let view_top = self.mutable_viewport.top;
        let buffer_h = self.total_buffer_height();
        let (cx, cy) = self.main_buffer.cursor.position;
        let bottom_margin = view_top + view_h - 1; // inclusive bottom (line 2451)
        let view_bottom_excl = view_top + view_h; // exclusive (line 2461 `page.Bottom()`)

        if cy < bottom_margin {
            // Chưa tới bottom → chỉ advance cursor (line 2451). No scroll,
            // no notify.
            self.main_buffer.cursor.set_position(cx, cy + 1);
            return;
        }
        // cy == bottom_margin (đã ở bottom).
        if view_bottom_excl < buffer_h {
            // PAN: scroll viewport xuống 1 row + advance cursor + notify
            // (line 2461-2469 + SetViewportPosition::60-62).
            self.mutable_viewport.top += 1;
            self.mutable_viewport.bottom += 1;
            self.main_buffer.cursor.set_position(cx, cy + 1);
            self.notify_scroll_event();
            return;
        }
        // CIRCLE: port line 2484-2494. Buffer đã tới cuối → increment circular.
        let fill = *self.main_buffer.get_current_attributes();
        self.main_buffer.increment_circular_buffer(&fill);
        // Cursor.y giữ nguyên (line 2484 "cursor không move down"). Cursor.x
        // giữ nguyên (no carriage-return trong LF). Bao quát: sau \r trước \n
        // cx=0 nên đây chỉ zero-cost. Vẫn gọi set_position để clear
        // `is_delayed_eol_wrap`.
        self.main_buffer.cursor.set_position(cx, cy);
        // Trigger scroll delta (line 2494) — GH#5540 accumulate, no overflow.
        self.trigger_scroll((0, -1));
        // `_api.NotifyBufferRotation(1)` (line 2491) → preserve offset + notify
        // iff `AlwaysNotifyOnBufferRotation` (TerminalApi.cpp:422-425).
        if self.always_notify_on_buffer_rotation {
            self.notify_scroll_event();
        }
    }

    /// Port `Terminal::_NotifyScrollEvent` (`Terminal.cpp:1135`). Compute
    /// `(top, height, bottom)` và fire callback. Xem `scroll.rs::notify`.
    ///
    /// `bottom` = `GetBufferHeight()` = `_GetMutableViewport().BottomExclusive()`
    /// (`Terminal.cpp:1145`, `:1026-1029`) = `mutable_viewport.bottom + 1`
    /// (exclusive) — KHÔNG phải `total_buffer_height()`. Khi viewport pan,
    /// `bottom` tăng cùng; khi circle, viewport không tăng nên `bottom` cố
    /// định = `view_top + view_h` (giống WT).
    pub fn notify_scroll_event(&self) {
        let top = self.mutable_viewport.top - self.scroll_offset;
        let height = self.mutable_viewport.height();
        let bottom = self.mutable_viewport.bottom + 1;
        self.scroll.notify(top, height, bottom);
    }

    /// Port `Terminal::TriggerScroll(delta)` (`Terminal.cpp:1117`). Loop 01
    /// ghi vào `delta_slot` (`Rc<RefCell<Option<(i32,i32)>>>`) để test assert
    /// (mirror `MockScrollRenderEngine::InvalidateScroll`). GH#5540: accumulate
    /// dùng `i64` (xem `scroll.rs`) — không overflow `i32`/SHORT.
    pub fn trigger_scroll(&self, delta: ScrollDelta) {
        self.scroll.trigger_scroll(delta);
    }
}
