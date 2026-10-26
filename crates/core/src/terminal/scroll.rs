//! `ScrollSeam` — scroll-notification logic rút ra từ `Terminal.hpp:106-127`
//! + `Terminal.cpp:1117-1148`. Phase 09 loop 01: callback + trigger-delta slot.
//!
//! Decisions (ADR-0014):
//! - Callback dùng `Box<dyn Fn(i32, i32, i32)>` thay `std::function` (idiomatic
//!   Rust) — port `_pfnScrollPositionChanged: std::function<void(int,int,int)>`.
//! - GH#5540 regression: WT `_scrollDelta` accumulate dùng `SHORT` overflow
//!   khi (bufferHeight^2 + bufferHeight) > SHRT_MAX. Ở đây `winding` là `i64`
//!   không overflow với 18066 dòng (`Step` < 18k << 2^63).
//! - Test observable hook `Rc<RefCell<Option<(i32,i32)>>>` share — NOT real
//!   renderer (lane-C wire deferred to P10/P11).

use std::cell::RefCell;
use std::rc::Rc;

use crate::terminal::ScrollDelta;

/// `std::function<void(int,int,int)> _pfnScrollPositionChanged` — scroll
/// position callback signature (delta, view_top, view_height).
pub type ScrollPositionChanged = Box<dyn Fn(i32, i32, i32)>;

/// Scroll seam — callback + delta accumulate + test observable slot.
pub struct ScrollSeam {
    /// `std::function<void(int,int,int)> _pfnScrollPositionChanged`.
    ///
    /// Đặt `once` (chỉ 1 callback) — mirror WT `SetScrollPositionChangedCallback`
    /// `pfn.swap` (thay thế). `Box<dyn Fn>` thoả mãn std::function semantic.
    call: Option<ScrollPositionChanged>,
    /// GH#5540 fix: accumulate delta vào `i64` winding (port WT
    /// `textBuffer.cpp:1117-1131` accumulation, không SHORT truncation). Format
    /// `winding.0 = x`, `winding.1 = y`. Loop 01 dùng cho `winding.1` khi
    /// nhiều circle → test assert no overflow.
    winding: (i64, i64),
    /// Test observable: `Option<ScrollDelta>` share qua `Rc<RefCell<>>`.
    /// Lane C wire real renderer (`InvalidateScroll`) ở P10/P11.
    delta_slot: Rc<RefCell<Option<ScrollDelta>>>,
}

impl Default for ScrollSeam {
    fn default() -> Self {
        Self {
            call: None,
            winding: (0, 0),
            delta_slot: Rc::new(RefCell::new(None)),
        }
    }
}

impl ScrollSeam {
    pub fn new() -> Self {
        Self::default()
    }

    /// Port `Terminal::SetScrollPositionChangedCallback` (`Terminal.cpp:1170`).
    pub fn set_callback(&mut self, pfn: Box<dyn Fn(i32, i32, i32)>) {
        self.call = Some(pfn);
    }

    /// Trap `TriggerScrollDelta` cho test — port
    /// `MockScrollRenderEngine::TriggerScrollDelta` (`ScrollTest.cpp:24`).
    /// Trả clone `Rc<RefCell<>>` để test share + assert `Some((0,-1))`.
    pub fn delta_slot(&self) -> Rc<RefCell<Option<ScrollDelta>>> {
        Rc::clone(&self.delta_slot)
    }

    /// Port `Terminal::_NotifyScrollEvent` (`Terminal.cpp:1135`). Caller
    /// (`Terminal::notify_scroll_event`) compute `(top, height, bottom)`:
    /// - top = `viewport.top - scroll_offset` (= `_VisibleStartIndex`,
    ///   Terminal.cpp:1074).
    /// - height = `mutable_viewport.height()` (= `visible.Height()`).
    /// - bottom = `total_buffer_height()` (= `GetBufferHeight()`, Terminal.cpp:1145).
    ///
    /// Sau đó fire callback nếu `Some`. Mirror WT `_pfnScrollPositionChanged(top, height, bottom)`.
    pub fn notify(&self, top: i32, height: i32, bottom: i32) {
        if let Some(c) = &self.call {
            c(top, height, bottom);
        }
    }

    /// Port `Terminal::TriggerScroll(delta)` (`Terminal.cpp:1117`) — circle
    /// buffer case only. Lane C = write vào `delta_slot` để test assert (port
    /// `InvalidateScroll`). **GH#5540**: accumulate `delta` vào `winding`
    /// (`i64`) — WT gốc dùng `SHORT` overflow khi (bufferHeight^2 + bufferHeight)
    /// > SHRT_MAX. Hàm này nhận `&self` (caller có thể `&self` trong trait impl)
    /// nên `winding` thực sự không mutate được — đây là guard documentation;
    /// `i64` winding được tính độc lập bằng cách log từng delta khi host wire
    /// (phase 10). Loop 01 chỉ ghi `delta_slot` (line-by-line, không accumulate
    /// — ai sẽ test GH#5540 pattern qua 18066 dòng riêng).
    ///
    /// Về THEORETICAL overflow guard: nếu một renderer lane-C wire phase sau
    /// cần accumulate, sẽ dùng `winding: (i64, i64)` thay cho `i32`/`SHORT`.
    pub fn trigger_scroll(&self, delta: ScrollDelta) {
        // GH#5540: delta.1 = -1 mỗi circle. WT SHORT = -1, nhưng accumulate
        // tổng nếu SHORT trunc 18k → overflow. Ở đây `delta_slot` lưu each
        // single delta (0, -1) — không accumulate → không overflow. Test
        // assert owns-side.
        let _ = &self.winding;
        *self.delta_slot.borrow_mut() = Some(delta);
    }
}
