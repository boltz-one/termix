---
adr: 0014
title: Terminal scroll-notification port deviations
status: accepted
date: 2026-10-26
phase: P09
supersedes: []
related: []
---

# ADR-0014 — Terminal scroll-notification port deviations

## Context

Phase 09 loop 01 port `wt_terminalcore__test_notify_scrolling` (`ScrollTest.cpp:131-236`)
— GH#5540 regression guard. WT `Terminal::_NotifyScrollEvent` (`Terminal.cpp:1135-1148`)
+ `TriggerScroll` (`Terminal.cpp:1117`) dùng:

- `std::function<void(int,int,int)> _pfnScrollPositionChanged` (callback).
- `MockScrollRenderEngine::InvalidateScroll` trap scroll-delta qua `std::optional<til::point>`.
- `_scrollDelta` `SHORT` accumulate (GH#5540 overflow khi
  `(bufferHeight^2 + bufferHeight) > SHRT_MAX`).

Vì termix chưa wire VTE parser + real renderer vào `Terminal` (parser ở
`control`, renderer ở `renderer`), loop 01 cần seam test-observable mà không
phải mock lớn.

## Decisions

### D1 — Callback dùng `Box<dyn Fn(i32, i32, i32)>` thay `std::function`

Idiomatic Rust. `Box<dyn Fn>` thoả mãn `std::function` semantic (callable, swap-on-set,
single-callback). `ScrollSeam::set_callback(&mut self, pfn)` thay thế — mirror WT
`SetScrollPositionChangedCallback` `pfn.swap`.

### D2 — `process_string` simplification (NOT VTE parser)

WT `Terminal::Write` → `StateMachine::ProcessString` (VTE parser). Vì parser sống
ở `control` `ControlCore` (P11), `Terminal::process_string` loop-01 làm direct
`TextBuffer` writes cho tối thiểu escape sequences cần cho `TestNotifyScrolling`
(`"X\r\n"`):

- `\r` (CR) → `cursor.x = 0`.
- `\n` (LF) → `handle_linefeed`: pan viewport khi chưa tới cuối buffer; circle
  khi tới cuối.
- printable char → viết tại cursor + advance.

Circle path fold vào `handle_linefeed` (port `AdaptDispatch::CursorLineFeed`
`adaptDispatch.cpp:2461/2484`). Same semantics. Lane-C wire real
`OutputStateMachineEngine` vào `Terminal::process_string` = P11 loop riêng
(ControlCore migration). Đây là simplification có chủ đích — không ảnh hưởng
GH#5540 scroll-delta overflow (correctness path không qua parser).

### D3 — Scroll-delta seam `Rc<RefCell<Option<(i32, i32)>>>`

WT `MockScrollRenderEngine::InvalidateScroll` ghi `_triggerScrollDelta` từ renderer
path. Ta không có real renderer loop 01 — `Terminal::trigger_scroll_delta_slot`
trả clone `Rc<RefCell<Option<ScrollDelta>>>` để test inject + assert. Lane-C wire
real renderer (`InvalidateScroll`) = P10/P11 loop riêng.

`RenderEngine` trait (`paint.rs:50`) đã P01 frozen — KHÔNG mở rộng trait
(scope break). Terminal giữ `ScrollSeam` (callback + delta slot) là trường riêng.

### D4 — `winding: (i64, i64)` accumulator (GH#5540 guard)

WT `_scrollDelta` dùng `SHORT` overflow khi `(bufferHeight^2 + bufferHeight) > SHRT_MAX`.
Ở 18066 dòng (2x 9033), `Step` < 18k << 2^63 — `i64` không overflow. `ScrollSeam::winding`
field (i64, i64) documentation guard; loop-01 `trigger_scroll` ghi `delta_slot` tuyến
từng delta đơn lẻ `(0, -1)` (không accumulate) — GH#5540 pattern test riêng qua
18066 dòng航线 phase sau. Cái guard i64 được ghi nhận đây để future lane-C wire
accumulate mà không cần revert type signature.

## Consequences

- `Terminal` struct có `scroll: ScrollSeam` field (callback + delta slot) — không
  pollution sang `RenderEngine` trait hay `TextBuffer` (frozen).
- Test `wt_terminalcore__test_notify_scrolling` green qua 2x totalBufferSize rows
  cho cả 2 data row (`notify_on_circling ∈ {false, true}`).
- `bottom` callback value = `mutable_viewport.bottom + 1` (exclusive,
  `GetBufferHeight()` = `BottomExclusive()`) — KHÔNG phải `total_buffer_height()`.
  Test assert khớp với WT `expectedBottom = expectedTop + TerminalViewHeight`
  (`ScrollTest.cpp:224`).

## Verification

- `cargo test -p core --test wt_terminalcore` → 1 test `wt_terminalcore__test_notify_scrolling`
  green (2 data row inline loop, port TAE parameterization).
- `cargo check -p core` clean.
- `cargo test --workspace --lib` 462 pass (no regression vs Phase 5 baseline).
