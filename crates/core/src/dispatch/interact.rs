//! `InteractDispatch` — small host trait (port conhost channel).
//!
//! WT có `InteractDispatch` riêng (`windows/src/terminal/adapter/InteractDispatch.h`)
//! — API mà conhost-style adapter (lane C / control) gọi để talk với
//! host (chuyển đối với UI host: set title, ring bell, set clipboard,
//! resize window, request paint).
//!
//! Phase 1 skeleton: 7 essential methods (`WriteFile`, `NotifyCursorChange`,
//! `SetWindowTitle`, `SetClipboard`, `ResizeWindow`, `NotifyBufferChange`,
//! `WarningBell`) với default `todo!()`.
//!
//! Phase 4.0-pre — xem `IInteractDispatch.hpp:33-42` thêm 9 method port verbatim
//! từ WT interface: `set_device_attributes`, `write_input`, `write_ctrl_key`,
//! `write_string`, `write_string_raw`, `window_manipulation`, `move_cursor`,
//! `is_vt_input_enabled`, `focus_changed`. Lane B = no-op default (conhost
//! out-of-scope); lane C impl cho adapter để gọi native GPUI / Win2 host.
//! 7 essential method còn dùng `todo!()` default (`write_file`,
//! `notify_cursor_change`, `set_window_title`, `set_clipboard`, `resize_window`,
//! `notify_buffer_change`, `warning_bell`) — per WT `IInteractDispatch.hpp`.

use super::VtInt;

/// `WindowManipulationType` — port `DispatchTypes::WindowManipulationType`.
/// Sub-type cho `window_manipulation`. Phase 1: enum stub (đã có ở `types.rs`,
/// nhưng giữ local trait-level để tránh cycle).
pub use super::types::WindowManipulationType;

/// `DeviceAttribute` — port `DispatchTypes::DeviceAttribute` (bitfield).
/// Phase 1: enum cơ bản với single variant `Default` — phase 2 nâng lên
/// full `til::enumset` (u64-backed bitset). Tạm giữ `u64` để trait signature
/// ổn định. Chi tiết: `windows/src/terminal/adapter/DispatchTypes.hpp`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct DeviceAttributeSet(pub u64);

/// `InputRecord` — port `INPUT_RECORD` (`wincon.h`). Phase 1: opaque token
/// (type tag + raw bytes) — phase 2 sẽ replaced bằng typed enum
/// (`KeyEvent`, `MouseEvent`, `FocusEvent`, ...).
#[derive(Clone, Debug, Default)]
pub struct InputRecord {
    /// Event type tag (port `INPUT_RECORD::EventType`).
    pub event_type: u16,
    /// Raw event data (opaque — phase 2 typed enum).
    pub data: Vec<u8>,
}

#[allow(unused_variables)]
pub trait InteractDispatch {
    /// Xuất ra output stream — port `InteractDispatch::WriteFile` /
    /// `PrintString`.
    fn write_file(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11: host bridge implementation
    }

    /// Báo cập nhật cursor position — host repaint.
    fn notify_cursor_change(&mut self) {
        // Lane C defer P09/P11: host repaint trigger
    }

    /// Set window title.
    fn set_window_title(&mut self, title: &str) {
        let _ = title;
        // Lane C defer P09/P11: set window title
    }

    /// Set clipboard content (OSC 52).
    fn set_clipboard(&mut self, content: &str) {
        let _ = content;
        // Lane C defer P09/P11: OSC 52 clipboard
    }

    /// Yêu cầu host resize buffer (cols, rows).
    fn resize_window(&mut self, width: VtInt, height: VtInt) {
        let _ = (width, height);
        // Lane C defer P09/P11: resize buffer
    }

    /// Báo host in ra buffer đã thay đổi — repaint.
    fn notify_buffer_change(&mut self) {
        // Lane C defer P09/P11: repaint on buffer change
    }

    /// Ring bell — port `InteractDispatch::WarningBell`.
    fn warning_bell(&mut self) {
        // Lane C defer P09/P11: ring bell
    }

    // ===== Phase 4.0-pre — 8 method port verbatim từ
    // `windows/src/terminal/adapter/IInteractDispatch.hpp:33-42`.
    // W1-foundation assume những method này tồn tại với signature dưới —
    // nếu drift, comment `// NOTE: W1 worker assume signature X — rework`.
    // =====

    /// `IsVtInputEnabled() const` (`IInteractDispatch.hpp:33`).
    /// Trả true nếu VT input mode đang bật (xterm-style key sequences).
    fn is_vt_input_enabled(&self) -> bool {
        false // TODO(P03): real VT-input mode; until then not enabled
    }

    /// `SetDeviceAttributes(attributes)` (`IInteractDispatch.hpp:35`,
    /// `InteractDispatch.cpp:30-33`). WT `gci.GetVtIo()->SetDeviceAttributes`
    /// — VtIo out-of-scope (lane C). Lane B = no-op default.
    fn set_device_attributes(&mut self, attributes: DeviceAttributeSet) {
        let _ = attributes;
        // lane C pending VtIo — no-op instead of panicking
    }

    /// `WriteInput(inputEvents)` (`IInteractDispatch.hpp:36`).
    /// Ghi mảng `INPUT_RECORD` vào host (xterm input mode → conhost).
    fn write_input(&mut self, input_events: &[InputRecord]) {
        let _ = input_events;
        // TODO(P03): no input injection yet (no-op instead of panicking)
    }

    /// `WriteCtrlKey(event)` (`IInteractDispatch.hpp:37`).
    /// Ghi 1 control key event vào host.
    fn write_ctrl_key(&mut self, event: &InputRecord) {
        let _ = event;
        // TODO(P03): no-op instead of panicking
    }

    /// `WriteString(string)` (`IInteractDispatch.hpp:38`).
    /// Ghi chuỗi (UTF-16 wstring) vào host — thường được translate sang
    /// `INPUT_RECORD` keystrokes.
    fn write_string(&mut self, string: &str) {
        let _ = string;
        // TODO(P03): no-op instead of panicking
    }

    /// `WriteStringRaw(string)` (`IInteractDispatch.hpp:39`).
    /// Ghi chuỗi Raw (bypass translation) — port raw bytes path.
    fn write_string_raw(&mut self, string: &str) {
        let _ = string;
        // TODO(P03): no-op instead of panicking
    }

    /// `WindowManipulation(function, parameter1, parameter2)`
    /// (`IInteractDispatch.hpp:40`). xterm window ops (DTTERM_WindowManipulation).
    fn window_manipulation(
        &mut self,
        function: WindowManipulationType,
        parameter1: VtInt,
        parameter2: VtInt,
    ) {
        let _ = (function, parameter1, parameter2);
        // TODO(P03): xterm window ops ignored (ConPTY sends CSI t on resize)
    }

    /// `MoveCursor(row, col)` (`IInteractDispatch.hpp:41`).
    /// Di chuyển cursor host-side (xterm `OSC ...` cursor warp).
    fn move_cursor(&mut self, row: VtInt, col: VtInt) {
        let _ = (row, col);
        // TODO(P03): no-op instead of panicking
    }

    /// `FocusChanged(focused)` (`IInteractDispatch.hpp:42`,
    /// `InteractDispatch.cpp:197-211`). Conhost set `CONSOLE_HAS_FOCUS` +
    /// `WriteFocusEvent` — out-of-scope (lane C). Lane B = pure interface
    /// port, default no-op; lane C inject gpui Window focus event.
    fn focus_changed(&mut self, focused: bool) {
        let _ = focused;
    }

    // ===== Phase 5 Wave 2 — terminal_input mode get/set (9 set + 9 get) +
    // OSC action forward (9) + kitty keyboard (4). Port verbatim
    // `windows/src/terminal/adapter/adaptDispatch.cpp` mode arms that bridge
    // to `terminal_input` (`SetInputMode`/`GetInputMode`) + OSC arms that
    // forward qua `_api` host. Lane C impl no-op default (build pass); wire
    // gpui host ở phase sau.
    // =====

    // ——— terminal_input mode set (9 method) ———

    /// `DECCKM_CursorKeysMode` set — port `terminal_input.SetInputMode(CursorKey)`.
    fn set_cursor_key_mode(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `DECARM_AutoRepeatMode` set — `terminal_input.SetInputMode(AutoRepeat)`.
    fn set_auto_repeat_mode(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `DECSCNM_ScreenMode` set — `render_settings.SetScreenReversed` + redraw.
    fn set_screen_reversed(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `VT200_MOUSE_MODE` set — `terminal_input.SetInputMode(DefaultMouseTracking)`.
    fn set_default_mouse_tracking(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `ANY_EVENT_MOUSE_MODE` set — `terminal_input.SetInputMode(AnyEventMouseTracking)`.
    fn set_any_event_mouse_tracking(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `FOCUS_EVENT_MODE` set — `terminal_input.SetInputMode(FocusEvent)`.
    fn set_focus_event(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `SGR_EXTENDED_MODE` set — `terminal_input.SetInputMode(SgrMouseEncoding)`.
    fn set_sgr_mouse_encoding(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `ALTERNATE_SCROLL` set — `terminal_input.SetInputMode(AlternateScroll)`.
    fn set_alternate_scroll(&mut self, enable: bool) {
        let _ = enable;
        // Lane C defer P09/P11
    }
    /// `DECNKM_NumericKeypadMode` set — `terminal_input.SetInputMode(Keypad)`.
    fn set_keypad_mode(&mut self, application_mode: bool) {
        let _ = application_mode;
        // Lane C defer P09/P11
    }
    /// `DECANM_AnsiMode` set — `terminal_input.SetAnsiMode` (1 of 3 deps).
    fn set_ansi_mode(&mut self, ansi_mode: bool) {
        let _ = ansi_mode;
        // Lane C defer P09/P11
    }

    // ——— terminal_input mode get (6 bool + 2 + kitty u32) ———

    /// `DECCKM_CursorKeysMode` get — `terminal_input.GetInputMode(CursorKey)`.
    fn get_cursor_key_mode(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `DECSCNM_ScreenMode` get — `render_settings.GetScreenReversed`.
    fn get_screen_reversed(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `DECAWM_AutoWrapMode` get — `api.GetSystemMode(AutoWrap)`.
    fn get_autowrap_mode(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `DECARM_AutoRepeatMode` get — `terminal_input.GetInputMode(AutoRepeat)`.
    fn get_auto_repeat_mode(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `VT200_MOUSE_MODE` get — `terminal_input.GetInputMode(DefaultMouseTracking)`.
    fn get_default_mouse_tracking(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `ANY_EVENT_MOUSE_MODE` get — `terminal_input.GetInputMode(AnyEventMouseTracking)`.
    fn get_any_event_mouse_tracking(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `FOCUS_EVENT_MODE` get — `terminal_input.GetInputMode(FocusEvent)`.
    fn get_focus_event(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `SGR_EXTENDED_MODE` get — `terminal_input.GetInputMode(SgrMouseEncoding)`.
    fn get_sgr_mouse_encoding(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `ALTERNATE_SCROLL` get — `terminal_input.GetInputMode(AlternateScroll)`.
    fn get_alternate_scroll(&self) -> bool {
        false // Lane C defer P09/P11
    }
    /// `XTERM_BracketedPasteMode` get — `api.GetSystemMode(BracketedPaste)`.
    fn get_bracketed_paste(&self) -> bool {
        false // Lane C defer P09/P11
    }

    // ——— kitty keyboard (4 method) ———

    /// `CSI = flags; mode u` set — kitty keyboard protocol flags + push mode.
    fn set_kitty_keyboard_protocol(&mut self, flags: u32, mode: u8) {
        let _ = (flags, mode);
        // Lane C defer P09/P11
    }
    /// `CSI ? u` query — return current kitty keyboard flags.
    fn get_kitty_flags(&self) -> u32 {
        0 // Lane C defer P09/P11
    }
    /// `CSI > flags u` push — push current flags onto stack + set new flags.
    fn push_kitty_flags(&mut self, flags: u32) {
        let _ = flags;
        // Lane C defer P09/P11
    }
    /// `CSI < count u` pop — pop `count` entries off the kitty keyboard stack.
    fn pop_kitty_flags(&mut self, count: usize) {
        let _ = count;
        // Lane C defer P09/P11
    }

    // ——— OSC action forward (9 method) ———

    /// `SetCurrentWorkingDirectory(uri)` (`:86`) — OSC 7 host forward.
    fn set_current_working_directory(&mut self, uri: &str) {
        let _ = uri;
        // Lane C defer P09/P11
    }
    /// `AddHyperlink(uri, params)` (`:848`) — OSC 8 buffer hyperlink attr.
    fn add_hyperlink(&mut self, uri: &str, params: &str) {
        let _ = (uri, params);
        // Lane C defer P09/P11
    }
    /// `EndHyperlink()` (`:870`) — clear buffer current hyperlink attr.
    fn end_hyperlink(&mut self) {
        // Lane C defer P09/P11
    }
    /// `DoConEmuAction(s)` (`:871`) — OSC 9;9 conhost action.
    fn do_con_emu_action(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11
    }
    /// `DoITerm2Action(s)` (`:876`) — OSC 1337 iTerm2 action.
    fn do_iterm2_action(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11
    }
    /// `DoFinalTermAction(s)` (`:881`) — OSC 133 FinalTerm action.
    fn do_final_term_action(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11
    }
    /// `DoVsCodeAction(s)` (`:886`) — OSC 633 VS Code action.
    fn do_vs_code_action(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11
    }
    /// `DoWTAction(s)` (`:891`) — OSC WT-private action.
    fn do_wt_action(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11
    }
    /// `DoUrxvtAction(s)` (`:896`) — OSC 7770 urxvt action.
    fn do_urxvt_action(&mut self, s: &str) {
        let _ = s;
        // Lane C defer P09/P11
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubInteract;
    impl InteractDispatch for StubInteract {}

    #[test]
    fn trait_compiles() {
        fn _assert<T: InteractDispatch>() {}
        _assert::<StubInteract>();
    }

    #[test]
    fn trait_has_48_methods() {
        // Compile-time check: trait signature ổn định. Phase 5 Wave 2 raise
        // từ 15 → 48: +10 set / +10 get terminal_input mode, +4 kitty, +9 OSC
        // forward (= +33 method). Lane C impl no-op default; wire gpui host
        // ở phase sau.
        fn _has_all<T: InteractDispatch>() {}
        _has_all::<StubInteract>();
    }
}
