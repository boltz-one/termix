---
loop: 7
status: pending
preservation_constraints:
  - p1_loop06_workspace_lib_regression            # cargo test --workspace --lib ≥489 pass / 0 fail (loop06 489 base)
  - p2_loop06_colorfix_unit_intact                 # renderer --lib colorfix 8 pass
  - p3_loop06_settings18_intact                    # renderer --lib settings::tests 18 pass (13+5)
  - p4_loop06_colortable256_default                # RenderSettings::default() 256-table unchanged
  - p5_loop06_terminal_seam_intact                 # cargo test -p core --test wt_terminalcore 1 pass
  - p6_loop06_wt_colorfix_integration_intact       # cargo test -p renderer --test wt_renderer_colorfix 3 pass
  - p7_loop06_renderer_tests9_intact              # renderer::tests 9 pass (VR1-VR5 loop05 + VR1-VR4 loop06 consolidate, không regress)
  - p8_loop06_clippy_lib_renderer_clean            # cargo clippy -p renderer --all-targets -- -D warnings clean
  - p9_loop06_no_bbox_accumulation                 # trigger_redraw dispatch per-engine, không bbox union (L04-G01 closed, giữ)
  - p10_loop06_core_lib_baseline                   # cargo test -p core --lib ≥206 pass / 0 fail (loop06 core 206 base — loop07 chạm core phải ≥206)
  - p11_loop06_clippy_lib_core_clean               # cargo clippy -p core --all-targets -- -D warnings clean
  - p12_loop06_no_interact_trait_method_regression  # 7 landed InteractDispatch method (write_file/notify_cursor_change/set_window_title/set_clipboard/resize_window/notify_buffer_change/warning_bell) + Phase-5 wave-2 mode set/get (10+10+4+9 = 33) bodies unchanged
candidate: A_7 (crates/core/src/dispatch/interact.rs trait add focus_changed + change set_device_attributes default todo→no-op; crates/core/src/dispatch/interact_impl.rs fold 8 inherent todo!() methods into impl InteractDispatch with real host-bridge bodies + add focus_changed override + drop WAIT 4.0-pre marker + footer wait_phase40_pre module; tests +4 covering write_string/write_string_raw/focus_changed/is_vt_input_enabled/move_cursor)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 07 — Land `InteractDispatch` Phase 4.0-pre ext surface (9 method) + close `WAIT 4.0-pre`

## Objective

Port `IInteractDispatch.hpp:33-42` ext surface (9 virtual method) vào `InteractDispatch` trait, fold 8 inherent `todo!()` của `TerminalInteractDispatch` thành override thật minimal host-bridge, add `focus_changed` port — slice port-faithful nhỏ, locally-complete, KHÔNG wire conhost/ServiceLocator/InputBuffer (lane C defer P09/P11).

## Context (citations verified read-only `windows/src/terminal/adapter/`)

- **`IInteractDispatch` hpp wt_anchors** (verified via `grep -nE "\b<sym>\b"` against `windows/src/terminal/adapter/IInteractDispatch.hpp`): `IsVtInputEnabled` `:33`, `SetDeviceAttributes` `:35`, `WriteInput` `:36`, `WriteCtrlKey` `:37`, `WriteString` `:38`, `WriteStringRaw` `:39`, `WindowManipulation` `:40`, `MoveCursor` `:41`, `FocusChanged` `:42`. 9 method — 8 đã có trong trait `interact.rs:88-150` (`is_vt_input_enabled` `:88`, `set_device_attributes` `:99` `todo!()`, `write_input`/`write_ctrl_key`/`write_string`/`write_string_raw`/`window_manipulation`/`move_cursor` `:104-150` no-op default). `focus_changed` (`FocusChanged`) — **CHƯA có trong trait** (grep `focus_changed\|FocusChanged` chỉ hit `interact_impl.rs` WAIT comment, không hit trait). Loop 07 add trait method `focus_changed(&mut self, focused: bool)`.
- **`InteractDispatch.cpp` bodies** (verified read-only, wt_anchors grep-exact): `IsVtInputEnabled` `:184-186` `return _api.IsVtInputEnabled()`, `SetDeviceAttributes` `:30-33` `gci.GetVtIo()->SetDeviceAttributes(attributes)`, `WriteInput` `:44`, `WriteCtrlKey` `:57`, `WriteString` `:67-105` (CharToKeyEvents xfrm qua `gci.GetActiveInputBuffer()` — needs InputBuffer, **out-of-scope conhost**), `WriteStringRaw` `:106-108` `gci.GetActiveInputBuffer()->WriteString(string)`, `WindowManipulation` `:122`, `MoveCursor` `:156`, `FocusChanged` `:197-211` (`ServiceLocator::LocateGlobals().getConsoleInformation()` + `IsInVtIoMode()` + `pInputBuffer->WriteFocusEvent(focused)` — conhost, **out-of-scope**).
- **Port-faithful strategy** (lane B): tất cả 9 method WT bodies phụ thuộc `_api` (ITerminalApi) hoặc `gci` (ServiceLocator/ConsoleInformation/InputBuffer) — out-of-scope theo spec (`conhost/host/server` out-of-scope). Loop 07 port **interface surface** + **minimal host-bridge override** vào `HostContext`:
  - `is_vt_input_enabled() -> bool`: trait default `false` (giữ), `TerminalInteractDispatch` override → `true` (port-faithful heuristic: WT `_api.IsVtInputEnabled()` delegate to host; lane C sẽ set real). Dev: verify override `true` vs WT default hành vi — nếu WT default `false`, override `false` cũng OK; chọn `true` vì port đang ở VT-input mode (xterm adapter context).
  - `set_device_attributes(attributes)`: trait default đổi `todo!()` `:99` **→ no-op** (lane C pending VtIo).
  - `write_input(input_events)`: trait default no-op (giữ), `TerminalInteractDispatch` override no-op (lane C pending InputBuffer).
  - `write_ctrl_key(event)`: trait default no-op (giữ), `TerminalInteractDispatch` override no-op.
  - `write_string(string)`: trait default no-op (giữ), `TerminalInteractDispatch` override **push bytes vào `HostContext::write_buffer`** (`string.as_bytes()`) — port-faithful intent: response path pump đi ngược (mirror existing `write_file` `interact_impl.rs:147`). CharToKeyEvents xfrm lane C (out-of-scope).
  - `write_string_raw(string)`: trait default no-op (giữ), `TerminalInteractDispatch` override **push bytes vào `write_buffer`** (mirror `write_string`; WT `WriteStringRaw` cũng chỉ `WriteString(string)` vào `InputBuffer` — semantically equivalent ở `HostContext` surface, không xfrm).
  - `window_manipulation(function, parameter1, parameter2)`: trait default no-op (giữ), `TerminalInteractDispatch` override no-op (lane C pending ShowWindow/ResizeWindow).
  - `move_cursor(row, col)`: trait default no-op (giữ), `TerminalInteractDispatch` override no-op (lane C pending SetConsoleCursorPosition).
  - `focus_changed(focused)` (NEW): trait default no-op, `TerminalInteractDispatch` override no-op (lane C pending WriteFocusEvent + CONSOLE_HAS_FOCUS flag). **Pure interface port** — không có host-bridge ngữ nghĩa ở core vì `HostContext` không có focus field (lane C inject gpui Window focus).
- **State** (`interact_impl.rs:23-27` `HostContext`): `window_title: String`, `clipboard: String`, `write_buffer: Vec<u8>`. **NO `is_focused`/`focus` field** — lane C add khi wire gpui Window. Loop 07 không thêm field (YAGNI — focus override là no-op).
- **Marker xóa** (`interact_impl.rs`): line 13 `(`todo!()` WAIT guard), 1 (`focus_changed`) also pending Phase 4.0-pre`, lines 77-83 comment block `8 ext methods below are written as inherent methods with todo!() bodies plus WAIT 4.0-pre: trait ext not landed marker`, lines 86-140 8 inherent method `todo!()` bodies, lines 373-384 `mod wait_phase40_pre` compile-time guard (8 ext + 1 focus = 9 pending). Loop 07 xóa hết → trait ext landed.
- **trait method count** (loop 06 snapshot): trait có 8+7+33+ ext nhóm (Phase 4.0-pre: 8 method đã có, kitty keyboard 4, OSC 9, terminal_input mode 10 set + 10 get, plus 7 essential default `todo!()` đã thành no-op trong wait_phase40_pre module). Đọc `interact.rs:1-25` header note "Tổng 15 method với default `todo!()` body" — cũ, không match. Loop 07: focus_changed thêm → 9 method Phase 4.0-pre.
- **Trait default `wait_phase40_pre`**: WAIT marker `interact_impl.rs:373-384` `mod wait_phase40_pre` (compile-time guard 8+1=9 pending). Loop 07 xóa module + `_COMPILE_TIME_GUARD`.
- **Test stub** (`interact.rs`: không có; `interact_impl.rs::tests` 15 test loop-06). td/mod.rs mock dispatch `interact.rs:1025/1035/1036` có `is_vt_input_enabled`/`write_string`/`write_string_raw` stub riêng — không affected.

## Tasks

- **T1 — `InteractDispatch` trait add `focus_changed(&mut self, focused: bool)`** (`crates/renderer/src/dispatch/interact.rs`): port slice `IInteractDispatch.hpp:42`. Insert sau `move_cursor` (template `:142-150`), before Phase 5 Wave 2 section (`:154`). Default impl no-op: `let _ = focused;` (mirror non-panic pattern). Doc-comment cite `IInteractDispatch.hpp:42` + `InteractDispatch.cpp:197-211` (FocusChanged — conhost out-of-scope, lane C pending).
- **T2 — `InteractDispatch` trait change `set_device_attributes` default `todo!()` → no-op** (`interact.rs:99-102`): hiện `todo!("InteractDispatch::SetDeviceAttributes")`. Đổi thành no-op: `let _ = attributes;` (mirror `write_input`/`write_ctrl_key` pattern `:104-110`). Closes Wait guard #8 (s019 trait ext). Doc-comment giữ cite `IInteractDispatch.hpp:35`.
- **T3 — `TerminalInteractDispatch` fold 8 inherent `todo!()` → `impl InteractDispatch` override** (`interact_impl.rs:86-140`): xóa `impl TerminalInteractDispatch { ... 8 method ... }` block, di chuyển 8 method vào `impl InteractDispatch` block (`:147-356`, sau `warning_bell` `:194`). Real bodies (lane B host-bridge):
  - `is_vt_input_enabled(&self) -> bool`: `true` (override trait default `false`). Doc cite cpp:184-186 + note "lane C pending real VT-input mode; `HostContext` chưa có field".
  - `set_device_attributes(&mut self, attributes: DeviceAttributeSet)`: `let _ = attributes;` no-op. Doc cite cpp:30-33 + note "VtIo lane C".
  - `write_input(&mut self, input_events: &[InputRecord])`: `let _ = input_events;` no-op. Doc cite cpp:44 + note "InputBuffer lane C".
  - `write_ctrl_key(&mut self, event: &InputRecord)`: `let _ = event;` no-op. Doc cite cpp:57 + note "InputBuffer lane C".
  - `write_string(&mut self, string: &str)`: `self.host.borrow_mut().write_buffer.extend(string.as_bytes());` (mirror `write_file` `:147-150`). Doc cite cpp:67-105 + note "CharToKeyEvents xfrm out-of-scope conhost; `HostContext::write_buffer` mirror response path".
  - `write_string_raw(&mut self, string: &str)`: `self.host.borrow_mut().write_buffer.extend(string.as_bytes());` (mirror `write_string`; no xfrm). Doc cite cpp:106-108 + note "raw bytes path, no encoding xfrm".
  - `window_manipulation(&mut self, function, parameter1, parameter2)`: `let _ = (function, parameter1, parameter2);` no-op. Doc cite cpp:122 + note "ShowWindow/ResizeWindow lane C".
  - `move_cursor(&mut self, row: VtInt, col: VtInt)`: `let _ = (row, col);` no-op. Doc cite cpp:156 + note "SetConsoleCursorPosition lane C".
  - **Imports**: ensure `DeviceAttributeSet`, `InputRecord`, `VtInt`, `VtParameter`, `WindowManipulationType` accessible (traits re-export in `interact.rs` already). Giữ `use super::types::WindowManipulationType;` line. Verify unused import lints resolved (now used in override).
- **T4 — `TerminalInteractDispatch` add `focus_changed` override** (in `impl InteractDispatch` block sau `move_cursor`): `let _ = focused;` no-op. Doc cite `IInteractDispatch.hpp:42` + `InteractDispatch.cpp:197-211`. Note "conhost CONSOLE_HAS_FOCUS/WriteFocusEvent out-of-scope; lane C inject gpui Window focus".
- **T5 — Drop `WAIT 4.0-pre` marker** (`interact_impl.rs`): xóa comment block lines 77-85 (`// —— Phase 4.0-pre trait ext, NOT landed yet ... // Target todo!() count = 8`). Update header doc-comment line 12-16: xóa "(`todo!()` WAIT guard), 1 (`focus_changed`) also pending Phase 4.0-pre" + các mention `WAIT 4.0-pre` (replace bằng "8 Phase 4.0-pre ext methods landed as `impl InteractDispatch` overrides" comment).
- **T6 — Drop `mod wait_phase40_pre` footer** (`interact_impl.rs:372-384`): xóa `#[cfg(test)] #[allow(dead_code)] mod wait_phase40_pre { ... }` block (compile-time guard). Closes WAIT guard hoàn toàn.
- **T7 — Ensure `interact.rs` header note match** (`:1-17`): update "Phase 4.0-pre — xem `IInteractDispatch.hpp:33-42` thêm 8 method" → "... thêm 9 method ... `focus_changed` (FocusChanged `:42`)". Update "Tổng 15 method với default `todo!()` body" → reflect 9 Phase 4.0-pre method landed (no `todo!()` in trait except 7 essential default `todo!()` per WT `IInteractDispatch.hpp` 7 method `WriteFile`/`NotifyCursorChange`/`SetWindowTitle`/`SetClipboard`/`ResizeWindow`/`NotifyBufferChange`/`WarningBell`).
- **T8 — Tests +4 unit** (`interact_impl.rs::tests` hoặc `interact.rs::tests` nếu có): cover Lane B override behaviors:
  - `test_is_vt_input_enabled_returns_true` — `TerminalInteractDispatch::default().is_vt_input_enabled() == true`.
  - `test_write_string_appends_to_write_buffer` — `d.write_string("a")` + `d.write_string("b")` → `write_buffer == b"ab"` (mirror write_file test).
  - `test_write_string_raw_appends_bytes_verbatim` — `d.write_string_raw("X")` → `write_buffer == b"X"`.
  - `test_focus_changed_is_no_op_and_does_not_panic` — `d.focus_changed(true); d.focus_changed(false);` → write_buffer empty, host state unchanged.
  - `test_move_cursor_is_no_op` — `d.move_cursor(10, 20)` → write_buffer empty, host unchanged.
  - `test_set_device_attributes_is_no_op` — `d.set_device_attributes(DeviceAttributeSet(0xA9))` → write_buffer empty.
  - (6 test cho symmetry với VR count; Dev có thể thêm `test_write_input_no_op` / `test_window_manipulation_no_op` nếu cần — but 6 là đủ để cover all 9 method paths non-panic).
- **T9 — Ledger P10 (n/a)**: interact/InteractDispatch thuộc **P03** (phase-03-vt-adapter-dispatch.md) không phải P10-renderer. Loop 07 không edit `parity/ledger/P10-renderer/*`. P03 ledger (`parity/ledger/P03-*` nếu tồn tại) — verify tồn tại; nếu không, **out-of-scope** (loop 07 không seed P03 ledger — Planner decision defer to P03 audit loop). Note trong Preservation.
- **T10 — ADR**: no new ADR (port-faithful interface surface, no decision). ADR-0003 (renderer GPUI) không affected; ADR-0015 (colorfix) không affected. Nếu lane B strategy "host-bridge minimal no-op" cần document, optional ADR — **defer** (YAGNI: hành vi có thể suy ra từ spec "conhost out-of-scope").

## Preservation

Re-run loop-06 verified behaviors (E_6): VR1-VR5 (renderer::tests 9/0), P1-P9 (workspace 489/0, colorfix 8/0, settings 18/0, color table 256, terminal seam 1/0, wt_colorfix 3/0, clippy renderer clean, no bbox accumulation). Loop-07 KHÔNG touch renderer crate / colorfix / settings / paint / cell / grid; KHÔNG touch crates/renderer / crates/control / crates/font / crates/connection / crates/app. Single-writer `crates/core/src/dispatch/{interact.rs, interact_impl.rs}` only. Loop-07 chạm `crates/core/src/dispatch/` — re-run P10 (`cargo test -p core --lib` ≥206/0), P11 (`clippy -p core` clean), P12 (33 terminal_input mode set/get/kitty/OSC override không regress, 7 essential IInteractDispatch method không regress).

## Validation Requirements

- **VR1 — `focus_changed(focused)` trait method tồn tại + override no-op** — Given `TerminalInteractDispatch::default()` + `focus_changed(true)` sau `focus_changed(false)`, Then không panic + `write_buffer` rỗng + `window_title` "" + `clipboard` "" (host state không đổi). Mirror WT `FocusChanged` `:197-211` "Does nothing outside of ConPTY" (lane B = no-op).
- **VR2 — `is_vt_input_enabled() == true` override** — Given `TerminalInteractDispatch::default()`, Then `is_vt_input_enabled() == true` (override trait default `false`). Mirror WT `IsVtInputEnabled` `:184-186` delegates to host (lane B default true; lane C sẽ set real).
- **VR3 — `write_string(s)` appends to `write_buffer`** — Given `d.write_string("hello")` + `d.write_string(" world")`, Then `d.host.borrow().write_buffer == b"hello world"`. Mirror `write_file` append semantics (`interact_impl.rs:147-150`); WT `WriteString` `:67-105` CharToKeyEvents xfrm out-of-scope (lane B = raw push).
- **VR4 — `write_string_raw(s)` appends verbatim bytes** — Given `d.write_string_raw("VT")`, Then `d.host.borrow().write_buffer == b"VT"`. Mirror WT `WriteStringRaw` `:106-108` `WriteString(string)` no-xfrm path.
- **VR5 — `set_device_attributes`/`write_input`/`write_ctrl_key`/`window_manipulation`/`move_cursor` no-op + no panic** — Given each method called with sample args (`set_device_attributes(DeviceAttributeSet(0x1))`, `write_input(&[InputRecord::default()])`, `write_ctrl_key(&InputRecord::default())`, `window_manipulation(WindowManipulationType::Default, 0, 0)`, `move_cursor(5, 10)`), Then `write_buffer` rỗng + host state không đổi. Mirror WT bodies `:30-33, :44, :57, :122, :156` (all depend `_api`/`gci` out-of-scope; lane B = no-op).
- **VR6 — `interact_impl.rs` `todo!()` count zero trong Phase 4.0-pre surface** — `grep -c "todo!" crates/core/src/dispatch/interact_impl.rs` returns ≤ pre-count minus 8 (8 inherent `todo!()` removed, replaced bằng override). Verify via `grep -nE "todo!\\\\!|unimplemented" crates/core/src/dispatch/interact_impl.rs` — WAIT marker block (`:77-140`) + `mod wait_phase40_pre` footer (`:372-384`) gone.
- **VR7 — `interact.rs` trait method count = 9 Phase 4.0-pre** — `grep -cE "^(\\s+)?fn (is_vt_input_enabled|set_device_attributes|write_input|write_ctrl_key|write_string|write_string_raw|window_manipulation|move_cursor|focus_changed)" crates/core/src/dispatch/interact.rs` returns ≥9 (focus_changed added).
- **P1-P12 preservation** — `cargo test --workspace --lib` ≥489/0 (target ≥499/0 do +~10 core test); `cargo test -p core --lib` ≥206/0 (target ≥212/0); `cargo test -p renderer --lib renderer::tests` 9/0; `cargo test -p renderer --lib colorfix` 8/0; `cargo test -p renderer --lib settings::tests` 18/0; `cargo test -p core --test wt_terminalcore` 1/0; `cargo test -p renderer --test wt_renderer_colorfix` 3/0; `cargo clippy -p core --all-targets -- -D warnings` 0/0; `cargo clippy -p renderer --all-targets -- -D warnings` 0/0; `trigger_redraw` no bbox accumulate giữ.
- **WAIT 4.0-pre closed** — `grep -n "WAIT 4.0-pre" crates/core/src/dispatch/interact_impl.rs` returns 0 match; `grep -n "wait_phase40_pre" crates/core/src/dispatch/interact_impl.rs` returns 0 match.

## Out-of-scope

- **conhost wiring** — `_api` (ITerminalApi real), `gci` (ServiceLocator/ConsoleInformation), `InputBuffer`, `CharToKeyEvents`, `VtIo`, `WriteFocusEvent`, `CONSOLE_HAS_FOCUS` flag — all out-of-scope theo spec (`conhost/host/server` out-of-scope). Lane C (phase P09/P11) wires concrete gpui/Win32 host.
- **`WriteString` CharToKeyEvents xfrm** (`InteractDispatch.cpp:67-105`) — needs `InputBuffer::IsInVirtualTerminalInputMode()` + `CharToKeyEvents()` + codepage. Out-of-scope (conhost). Loop 07 `write_string` = raw bytes push to `write_buffer` (response path port-faithful intent).
- **`FocusChanged` focus state field** — WT sets `CONSOLE_HAS_FOCUS` + `WriteFocusEvent`. Loop 07 `focus_changed` = pure interface port no-op. `HostContext` không thêm `is_focused` field (YAGNI — lane C add khi wire gpui Window focus event).
- **`HostContext` field рост** (e.g. add `is_focused`/`vt_input_mode`/`cursor_position`) — out-of-scope (lane C owns gpui Window state). Loop 07 giữ `HostContext` 3 field (`window_title`/`clipboard`/`write_buffer`).
- **`SetDeviceAttributes` DA response payload** (CSI > c reply) — Lane C pending VtIo. Loop 07 = no-op.
- **P03 ledger seed** (`parity/ledger/P03-*`) — out-of-scope loop 07. P03 audit loop riêng seed WT `InteractDispatch.cpp` method vào `symbols.csv`.
- **`set_window_title`/`set_clipboard`/`write_file`/`resize_window`/`notify_buffer_change`/`notify_cursor_change`/`warning_bell` (7 essential methods)** — đã landed loop trước (real bodies), KHÔNG regress (P12).
- **Phase 5 Wave 2 (33 terminal_input mode set/get/kitty/OSC)** — đã landed no-op default, KHÔNG regress (P12).
- **`crates/renderer` / colorfix / settings / paint/cell/grid** — not touched (renderer loop-05/06 work).
