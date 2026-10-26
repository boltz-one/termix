---
loop: 1
status: pending
preservation_constraints:
  - core/workspace build green (`cargo check --workspace`, 17-18 PascalCase warnings OK, scope P03 remove)
  - 410 baseline tests pass (`cargo test --workspace`)
  - core TextBuffer/Row/Cursor 188 todo!() giữ nguyên (P08/P03 own)
  - control ControlCore ad-hoc glue không đụng (P11 migrate, không P09)
  - parity/ledger P09 shard f00263 (`Terminal.hpp`) status `not-started`
---

# Loop 01 — Terminal struct + scroll-notification seam (red→green)

## Objective
Port `wt_terminalcore__test_notify_scrolling` (1/54) — tạo `Terminal` struct tối thiểu với scroll-position callback + `always_notify_on_buffer_rotation` flag + `_NotifyScrollEvent` + buffer-circle `TriggerScroll`, validate GH#5540 scroll-delta không overflow.

## Tasks
1. **Ledger P05 P09 mark**: update `parity/ledger/P09-terminal-core/files.csv` row `f00263` `status=partial`, `author=P09-worker`, `rust_path=crates/core/src/terminal/mod.rs`. Tạo row `s*` `Terminal::set_scroll_position_changed_callback`/`_notify_scroll_event`/`always_notify_on_buffer_rotation`/`trigger_scroll` trong `symbols.csv` (status `partial`).
2. **Test red**: tạo `crates/core/tests/wt_terminalcore.rs` port `ScrollTest::TestNotifyScrolling` (ScrollTest.cpp:131-236) verbatim-in-intent: parametrize `notifyOnCircling` (`false`,`true`), emit `"X\r\n"` ×2×bufferHeight, assert scroll-position-callback fire pattern (scrolled∧¬circled)∨(circled∧notifyOnCircling), assert circle `TriggerScroll` delta `(0,-1)`, assert viewport {top,height,bottom} per expected formula. Mock `ScrollRenderEngine` + trap `trigger_scroll_delta` qua `Rc<RefCell<Option<(i32,i32)>>>` (NO `MockUi` — use real `DummyRenderer` analogue). Test phải RED (không compile — `Terminal` chưa có).
3. **` crates/core/src/terminal/mod.rs`**: `Terminal` struct — owns `TextBuffer` (`main_buffer`), `Viewport` (`mutable_viewport` тип `crates::screen::Viewport`), `scroll_offset: i32`, `scrollback_lines: i32`, `always_notify_on_buffer_rotation: bool`, `snap_on_input: bool`. Constructor `Terminal::new(viewport_size:(i32,i32), scrollback_lines:i32, renderer)` dùng `TestDummyMarker` seam (mirror WT `Terminal::TestDummyMarker`), init buffer 80×32×9001 history. `lib.rs` add `pub mod terminal;`.
4. **`crates/core/src/terminal/scroll.rs`**: `set_scroll_position_changed_callback(&mut self, pfn: Box<dyn Fn(i32,i32,i32)>)`, `always_notify_on_buffer_rotation(&mut self, v:bool)`, `notify_scroll_event(&self)` private — compute `(top,height,bottom)` = `(clamp(viewport_top - view_height + 2,0,scrollback), view_height, top+view_height)`, invoke callback if `Some`. `trigger_scroll(&mut self, delta:(i32,i32))` — accumulate vào `_scrollDelta` winding vector (port WT `textBuffer.cpp:1117-1131`, không overflow SHORT: dùng `i64` accomulator hoặc `Option<i32>` clamp). Mirror WT `ScrollbarNotification` callback shape `(top,height,bottom)`.
5. **`Terminal::process_string(&mut self, s:&str)`**: thin linh bridge (NOT VTE) — gọi `_stateMachine.ProcessString` analogue. BECAUSE core chưa có seam StateMachine trên Terminal (parser ở ControlCore), dùng direct `TextBuffer` writes: viết `X` tại cursor, "CR" → cursor.x=0, "LF" → `newline()` port WT `_NotifyScrollEvent` trigger sau mỗi LF. Đây là ένα simplification — chịu — chỉ đủ cho `TestNotifyScrolling` pattern (GH#5540 là scroll-delta overflow, không phải parser correctness). Document deviation trong ADR-0009 (deviation row).
6. **Circle buffer logic**: port WT `_ScrollToPoints`/buffer-circle path — khi `currentRow >= totalBufferSize - 1` thì circle: gọi `TextBuffer::increment_circular_buffer(fill_attr)` (buffer.rs:608 đã có — reuse, KHÔNG add method mới), `viewport.top` adjust, `trigger_scroll((0,-1))`. `totalBufferSize = main_buffer.height + scrollback_lines` (mirror WT `termTb.GetSize().Height()`).
7. **Green**: test pass. `cargo test -p core -- wt_terminalcore__test_notify_scrolling` GREEN. `cargo check -p core` clean (cosmetic warning OK).
8. **ADRs**: tạo `docs/decisions/0009-terminal-scroll-notification-port-deviation.md` — document (a) callback dùng `Box<dyn Fn>` thay `std::function` (idiomatic Rust), (b) `process_string` simplification chưa qua VTE parser (lane-C wire deferred), (c) circle buffer reuse `TextBuffer::circle_one_row` nếu phải add. Ref ADR trong ledger row.

## Preservation
- `cargo check --workspace` PASS (17-18 warnings OK scope P03).
- `cargo test --workspace` 410 baseline tests không regress.
- 188 `todo!()` count trong core không tăng (loop này ADD 0 todo — green implementations).
- `parity/ledger/P09-terminal-core` chỉ update row f00263 + thêm symbol rows; không đụng shard khác.
- Không edit `crates/control/**` (P11 ownership).

## Validation Requirements
1. **Given** terminal `Terminal::new((80,32), 9001, dummy_renderer)`, **When** `process_string("X\r\n")` × `2 * totalBufferSize`, **Then** `scroll_position_changed` callback fires cho row `r` iff `(r>=31 ∧ r<totalBufferSize-1) ∨ (r>=totalBufferSize-1 ∧ notifyOnCircling=true)` — assert per-row (mirror ScrollTest.cpp:184-196).
2. **Given** same setup, **When** circle buffer occurs (row >= totalBufferSize-1), **Then** renderer `trigger_scroll_delta` == `Some((0,-1))` — assert per-row (mirror ScrollTest.cpp:200-208).
3. **Given** same setup, **When** no scroll, **Then** `trigger_scroll_delta` == `None` và scroll-position callback không fire — assert per-row (mirror ScrollTest.cpp:210-214).
4. **Given** callback fires, **Then** `(viewport_top, viewport_height, buffer_bottom)` == `(clamp(r-31+2,0,9001), 32, top+32)` — assert per-row (mirror ScrollTest.cpp:220-233).
5. **Given** `notifyOnCircling=true` vs `false` trên 2 data rows, **Then** cả 2 data rows pass (parametrized assert, mirror ScrollTest.cpp:147-150).
6. **Given** `cargo check -p core` + `cargo test -p core -- wt_terminalcore__test_notify_scrolling`, **Then** 0 error + test green.
7. **Given** scroll-delta accumulate trên 18000 dòng (totalBufferSize×2), **Then** không overflow `i32` (GH#5540 regression — delta accumulate use `i64` hoặc clamp pattern, không SHORT truncation).

## Out-of-scope
- Còn 53/54 UnitTests_TerminalCore tests (selection, hyperlink, mark, resize-reflow, alt buffer, send-key/mouse/char, focus, get-text) — loops 2+ của P09 (lane high-risk, 130h estimate).
- `ITerminalApi` impl trên Terminal (P09 step 2 đầy đủ) — loop 2.
- Selection model (P09 step 4) — loop 3+.
- `IRenderData` impl (P10 consume) — phase 10.
- Lane-C wire parser thật (`OutputStateMachineEngine`) vào Terminal::process_string — đây là simplification, ADR-0009 log, lane-C wire loop riêng (request ControlCore migration).
- `cargo xtask audit --wave 3` barrier — P09 partial, chưa đủ wave-barrier (P10/P11 còn outstanding). Chỉ `cargo check -p core` + targeted test green.
- `cargo xtask stubs` = 0 — core 258 stubs pre-existing P03/P08, không scope loop này.
- Performance/bench — correctness only.

## Notes
- **Reuse**: `crates/core/src/screen.rs::Viewport` đã có `from_size/width/height/offset/intersect` — Terminal `mutable_viewport` field là `Viewport`, không tạo struct mới. `TextBuffer::trigger_scroll` (buffer.rs:884 no-op) WIRING — Terminal `trigger_scroll` gọi tới renderer, không record buffer state.
- **No mocks/trait-bloat**: `RenderEngine` trait (paint.rs:50) đã P01 frozen, KHÔNG có `trigger_scroll` method và KHÔNG mở rộng (scope break). Decision: Terminal giữ `trigger_scroll_delta: Rc<RefCell<Option<(i32,i32)>>>` field — test inject clone share, `Terminal::trigger_scroll(delta)` write vào refcell. WT `TextBuffer::TriggerScroll` → renderer push; ở đây = test observable hook (no real renderer path trong loop này). Lane-C wire real renderer = P10/P11 loop riêng. ADR-0009 log seam này.
- **Viewport reuse**: `crates/core/src/screen.rs::Viewport::from_size(w,h)` đã có, dùng cho `mutable_viewport` field — không tạo struct mới.
- **TextBuffer reuse**: `TextBuffer::increment_circular_buffer(fill)` (buffer.rs:608), `scroll_rows` (719), `trigger_scroll` (884 no-op) đã có — Terminal `__circle_buffer` gọi `increment_circular_buffer`, KHÔNG thêm method mới vào `buffer.rs`. `trigger_scroll` của Terminal là NEW (separate field-shape, không rút gọn buffer `trigger_scroll` no-op).
- **Test name**: `wt_terminalcore__test_notify_scrolling` (single rule, ScrollTest.cpp method `TestNotifyScrolling` → snake).
- **Deviation**: GH#5540 là CORE regression test — loop này không thể defer vì là acceptance test #1 của P09 success criteria bullet 3.
