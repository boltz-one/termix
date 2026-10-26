//! Port `ScrollTest::TestNotifyScrolling` (`ScrollTest.cpp:131-236`).
//!
//! GH#5540 regression guard: accumulate scroll-delta qua 2x bufferSize
//! không overflow (WT gốc dùng `SHORT` overflow khi
//! (bufferHeight^2 + bufferHeight) > SHRT_MAX).
//!
//! Verbatim-in-intent port. WT dùng `MockScrollRenderEngine` therad `InvalidateScroll`
//! + `_scrollBarNotification` shared. Ta dùng `Terminal::trigger_scroll_delta_slot`
//! (`Rc<RefCell<Option<(i32,i32)>>>`) + `scroll_position_changed` callback
//! (`Rc<RefCell<Option<(i32,i32,i32)>>>`). Multi value via `Box<dyn Fn>`.

// Test name convention `wt_<suite>__<method>` (double underscore) — port
// naming. non_snake_case intentional cho parity tracking.
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use core::terminal::{Terminal, TestDummyMarker};

/// Mirror WT `ScrollTest` const (`ScrollTest.cpp:88-92`).
/// 80 width, 32 height, 9001 history. totalBufferSize = 32 + 9001 = 9033.
const TERMINAL_VIEW_WIDTH: i32 = 80;
const TERMINAL_VIEW_HEIGHT: i32 = 32;
const TERMINAL_HISTORY_LENGTH: i32 = 9001;

/// `(ViewportTop, ViewportHeight, BufferHeight)` — port `ScrollBarNotification`
/// (`ScrollTest.cpp:69-74`).
type ScrollBarNotification = (i32, i32, i32);

/// Khởi tạo terminal test fixture — port `MethodSetup` (`ScrollTest.cpp:98-116`).
/// Trả (terminal, scroll_bar_shared, trigger_scroll_shared).
fn make_terminal(
    notify_on_circling: bool,
) -> (
    Terminal,
    Rc<RefCell<Option<ScrollBarNotification>>>,
    Rc<RefCell<Option<(i32, i32)>>>,
) {
    let mut term = Terminal::new_with_marker(
        TestDummyMarker,
        (TERMINAL_VIEW_WIDTH, TERMINAL_VIEW_HEIGHT),
        TERMINAL_HISTORY_LENGTH,
    );
    term.always_notify_on_buffer_rotation(notify_on_circling);

    let scroll_bar: Rc<RefCell<Option<ScrollBarNotification>>> =
        Rc::new(RefCell::new(None));
    let scroll_bar_for_cb = Rc::clone(&scroll_bar);
    term.set_scroll_position_changed_callback(Box::new(move |top, height, bottom| {
        *scroll_bar_for_cb.borrow_mut() = Some((top, height, bottom));
    }));

    let trigger = term.trigger_scroll_delta_slot();

    (term, scroll_bar, trigger)
}

/// Assert pattern cho 1 row — port loop body `ScrollTest.cpp:169-234`.
fn assert_row(
    row: i32,
    term: &mut Terminal,
    scroll_bar: &Rc<RefCell<Option<ScrollBarNotification>>>,
    trigger: &Rc<RefCell<Option<(i32, i32)>>>,
    notify_on_circling: bool,
) {
    // Reset per-row observations.
    *scroll_bar.borrow_mut() = None;
    *trigger.borrow_mut() = None;

    term.process_string("X\r\n");

    let total_buffer_size = term.total_buffer_height();

    // `scrolled` = currentRow >= TerminalViewHeight - 1 (line 178).
    let scrolled = row >= TERMINAL_VIEW_HEIGHT - 1;
    // `circledBuffer` = currentRow >= totalBufferSize - 1 (line 183).
    let circled_buffer = row >= total_buffer_size - 1;
    // `expectScrollBarNotification` (line 184-185):
    //   (scrolled ∧ ¬circled) ∨ (circled ∧ notifyOnCircling)
    let expect_scroll_bar = (scrolled && !circled_buffer)
        || (circled_buffer && notify_on_circling);

    if expect_scroll_bar {
        assert!(
            scroll_bar.borrow().is_some(),
            "row {row}: expected scroll-bar notification"
        );
    } else {
        assert!(
            scroll_bar.borrow().is_none(),
            "row {row}: expected NO scroll-bar notification"
        );
    }

    // `TriggerScroll` delta (line 200-214): chỉ fire khi scrolled ∧ circled.
    if scrolled && circled_buffer {
        let delta = trigger
            .borrow()
            .expect(&format!("row {row}: expected trigger-scroll delta"));
        assert_eq!(
            delta,
            (0, -1),
            "row {row}: wrong trigger-scroll delta"
        );
    } else {
        assert!(
            trigger.borrow().is_none(),
            "row {row}: expected NO trigger-scroll delta"
        );
    }

    // Viewport values khi callback fire (line 216-233).
    if let Some((top, height, bottom)) = *scroll_bar.borrow() {
        let expected_top = (row - TERMINAL_VIEW_HEIGHT + 2)
            .clamp(0, TERMINAL_HISTORY_LENGTH);
        let expected_height = TERMINAL_VIEW_HEIGHT;
        let expected_bottom = expected_top + TERMINAL_VIEW_HEIGHT;
        assert_eq!(
            top, expected_top,
            "row {row}: ViewportTop mismatch"
        );
        assert_eq!(
            height, expected_height,
            "row {row}: ViewportHeight mismatch"
        );
        assert_eq!(
            bottom, expected_bottom,
            "row {row}: BufferHeight mismatch"
        );
    }
}

/// Parametrized port — `notify_on_circling ∈ {false, true}` (TAE data row,
/// `ScrollTest.cpp:147-150`).
fn run_test_notify_scrolling(notify_on_circling: bool) {
    let (mut term, scroll_bar, trigger) = make_terminal(notify_on_circling);
    let total_buffer_size = term.total_buffer_height();
    // GH#5540 điểm nhấn: emit 2x totalBufferSize lines (~18000).
    for row in 0..(total_buffer_size * 2) {
        assert_row(
            row,
            &mut term,
            &scroll_bar,
            &trigger,
            notify_on_circling,
        );
    }
}

/// Port `ScrollTest::TestNotifyScrolling` — single test, 2 data rows
/// (`notify_on_circling ∈ {false, true}`, `ScrollTest.cpp:147-150`). WT TAE
/// parameterization → loop inline (Rust no native data-row framework).
#[test]
fn wt_terminalcore__test_notify_scrolling() {
    // TAE data row false → true (mirror declaration order `ScrollTest.cpp:148`).
    for notify_on_circling in [false, true] {
        run_test_notify_scrolling(notify_on_circling);
    }
}
