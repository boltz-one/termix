//! `TerminalInteractDispatch` — port of `InteractDispatch.cpp` (211 LOC)
//! to a host-bridging Rust struct. Phase 4 wave 0.
//!
//! WT struct owns `_api` (`ITerminalApi`) — host console bridge. In Rust we
//! bridge via `Rc<RefCell<HostContext>>` where `HostContext` wraps the gpui
//! host state (window title / clipboard / write_file -> stdout pipe).
//!
//! `core` keeps no direct GPUI dep (HostContext is plain state; lane C
//! `control` injects the actual gpui Window/App). Phase 4.0 embeds
//! `HostContext::default` test only.
//!
//! 16 method target: 7 existing landed (real body), 9 Phase 4.0-pre ext
//! landed as `impl InteractDispatch` overrides (lane B host-bridge minimal:
//! no-op for conhost-out-of-scope, `write_buffer` push for `write_string`/
//! `write_string_raw`, `is_vt_input_enabled` override `true`).

use std::cell::RefCell;
use std::rc::Rc;

use super::interact::InteractDispatch;
use super::types::WindowManipulationType;
use super::VtInt;
/// `HostContext` — host-bridge state. Lane C (`control`) injects gpui.
///
/// `write_buffer` collects DSR/DA report payloads ("pump đi ngược") — Phase 5
/// wire-up drains it to host stdin. Phase 4.0 embeds `default` test only.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HostContext {
    /// Window title — `SetWindowTitle`.
    pub window_title: String,
    /// Clipboard content (OSC 52) — `SetClipboard`.
    pub clipboard: String,
    /// Inbound bytes for host stdin — DSR/DA reports, `WriteFile` payload.
    pub write_buffer: Vec<u8>,
}

impl HostContext {
    pub fn new() -> Self {
        Self::default()
    }
}

/// `TerminalInteractDispatch` — port of WT `InteractDispatch` (conhost adapter).
///
/// Cloneable cheaply via `Rc<RefCell<HostContext>>` so tests + lane C can share
/// host state with the parser/dispatch pipeline.
#[derive(Debug)]
pub struct TerminalInteractDispatch {
    pub host: Rc<RefCell<HostContext>>,
}

impl Default for TerminalInteractDispatch {
    fn default() -> Self {
        Self::new(HostContext::default())
    }
}

impl TerminalInteractDispatch {
    pub fn new(host: HostContext) -> Self {
        Self {
            host: Rc::new(RefCell::new(host)),
        }
    }

    /// Borrow host immutably — test + internal helper.
    pub fn with_host<R>(&self, f: impl FnOnce(&HostContext) -> R) -> R {
        f(&self.host.borrow())
    }

    /// Borrow host mutably — test + internal helper.
    pub fn with_host_mut<R>(&self, f: impl FnOnce(&mut HostContext) -> R) -> R {
        f(&mut self.host.borrow_mut())
    }
}

// —— Phase 4.0-pre ext methods folded into `impl InteractDispatch` below
// (lane B host-bridge). The WAIT guard marker + footer compile-time-guard
// module are removed. See trait `interact.rs` for `IInteractDispatch.hpp:33-42`
// anchors. ——

// —— Landed `InteractDispatch` impl (7 existing trait methods) ——

#[allow(unused_variables)]
impl InteractDispatch for TerminalInteractDispatch {
    /// `WriteFile` — push payload into `HostContext::write_buffer`. Per WT
    /// InteractDispatch.cpp, this is the host response path (pump đi ngược).
    fn write_file(&mut self, s: &str) {
        self.host.borrow_mut().write_buffer.extend(s.as_bytes());
    }

    /// `NotifyCursorChange` — no-op. Host (lane C / gpui Window) reads cursor
    /// from terminal via redraw, not via this notify.
    fn notify_cursor_change(&mut self) {
        // no-op — port `InteractDispatch::NotifyCursorChange` (also no-op here).
    }

    /// `SetWindowTitle` — assign `host.window_title`. Port
    /// `InteractDispatch::SetWindowTitle`.
    fn set_window_title(&mut self, title: &str) {
        self.host.borrow_mut().window_title = title.to_string();
    }

    /// `SetClipboard` — assign `host.clipboard`. Port `InteractDispatch::SetClipboard`.
    fn set_clipboard(&mut self, content: &str) {
        self.host.borrow_mut().clipboard = content.to_string();
    }

    /// `ResizeWindow` — currently no-op (debug log). Host reads resize from
    /// gpui Window events (lane C). Port `InteractDispatch::ResizeWindow`.
    fn resize_window(&mut self, width: VtInt, height: VtInt) {
        // no-op — lane C owns actual gpui Window resize emission. Keep args
        // around in case future impl needs to record desired size (YAGNI for now).
        let _ = (width, height);
    }

    /// `NotifyBufferChange` — no-op. Host repaints via lane C read of buffer.
    fn notify_buffer_change(&mut self) {
        // no-op — port `InteractDispatch::NotifyBufferChange`.
    }

    /// `WarningBell` — debug log only. Lane C plays actual sound via gpui host.
    fn warning_bell(&mut self) {
        // no-op in core (no GPUI dep); lane C handles audible bell.
    }

    // ===== Phase 4.0-pre ext methods (9) — folded from inherent `todo!()`
    // block. Port `IInteractDispatch.hpp:33-42` + `InteractDispatch.cpp` bodies.
    // Lane B host-bridge minimal: conhost (`_api`/`gci`/`InputBuffer`/`VtIo`)
    // out-of-scope per spec; override real-behavior surface vào `HostContext`.
    // =====

    /// `IsVtInputEnabled()` (`IInteractDispatch.hpp:33`, `InteractDispatch.cpp`:
    /// 184-186 — delegates to `_api.IsVtInputEnabled()`). Lane B override `true`
    /// (port context = xterm VT-input adapter; HostContext chưa có field — lane C
    /// inject real VT-input mode).
    fn is_vt_input_enabled(&self) -> bool {
        true
    }

    /// `SetDeviceAttributes(attributes)` (`IInteractDispatch.hpp:35`,
    /// `InteractDispatch.cpp:30-33` — `gci.GetVtIo()->SetDeviceAttributes`).
    /// VtIo out-of-scope; lane B = no-op override.
    fn set_device_attributes(&mut self, attributes: super::interact::DeviceAttributeSet) {
        let _ = attributes;
        // lane C pending VtIo — no-op
    }

    /// `WriteInput(input_events)` (`IInteractDispatch.hpp:36`,
    /// `InteractDispatch.cpp:44`). InputBuffer out-of-scope; lane B = no-op.
    fn write_input(&mut self, input_events: &[super::interact::InputRecord]) {
        let _ = input_events;
        // lane C pending InputBuffer — no-op
    }

    /// `WriteCtrlKey(event)` (`IInteractDispatch.hpp:37`,
    /// `InteractDispatch.cpp:57`). InputBuffer out-of-scope; lane B = no-op.
    fn write_ctrl_key(&mut self, event: &super::interact::InputRecord) {
        let _ = event;
        // lane C pending InputBuffer — no-op
    }

    /// `WriteString(string)` (`IInteractDispatch.hpp:38`,
    /// `InteractDispatch.cpp:67-105` — `CharToKeyEvents` xfrm qua
    /// `gci.GetActiveInputBuffer()`, out-of-scope conhost). Lane B = raw bytes
    /// push to `write_buffer` (port-faithful response-path intent — mirror
    /// `write_file`).
    fn write_string(&mut self, string: &str) {
        self.host
            .borrow_mut()
            .write_buffer
            .extend(string.as_bytes());
    }

    /// `WriteStringRaw(string)` (`IInteractDispatch.hpp:39`,
    /// `InteractDispatch.cpp:106-108` — `WriteString(string)` no-xfrm path).
    /// Lane B = raw bytes push to `write_buffer` (no encoding xfrm).
    fn write_string_raw(&mut self, string: &str) {
        self.host
            .borrow_mut()
            .write_buffer
            .extend(string.as_bytes());
    }

    /// `WindowManipulation(function, parameter1, parameter2)`
    /// (`IInteractDispatch.hpp:40`, `InteractDispatch.cpp:122`).
    /// ShowWindow/ResizeWindow out-of-scope; lane B = no-op.
    fn window_manipulation(
        &mut self,
        function: WindowManipulationType,
        parameter1: VtInt,
        parameter2: VtInt,
    ) {
        let _ = (function, parameter1, parameter2);
        // lane C pending ShowWindow/ResizeWindow — no-op
    }

    /// `MoveCursor(row, col)` (`IInteractDispatch.hpp:41`,
    /// `InteractDispatch.cpp:156`). SetConsoleCursorPosition out-of-scope;
    /// lane B = no-op.
    fn move_cursor(&mut self, row: VtInt, col: VtInt) {
        let _ = (row, col);
        // lane C pending SetConsoleCursorPosition — no-op
    }

    /// `FocusChanged(focused)` (`IInteractDispatch.hpp:42`,
    /// `InteractDispatch.cpp:197-211` — `IsInVtIoMode()` +
    /// `pInputBuffer->WriteFocusEvent(focused)` + `CONSOLE_HAS_FOCUS` flag;
    /// conhost out-of-scope). Lane B = pure interface port no-op. HostContext
    /// không có focus field (YAGNI — lane C inject gpui Window focus event).
    fn focus_changed(&mut self, focused: bool) {
        let _ = focused;
        // lane C pending WriteFocusEvent + CONSOLE_HAS_FOCUS — no-op
    }

    // ===== Phase 5 Wave 2 — terminal_input mode get/set (10 set + 10 get) +
    // kitty keyboard (4) + OSC action forward (9). Default no-op / `false` /
    // `0` để build pass + parser gọi không panic. Wire gpui host ở Lane C phase
    // sau (`screen_reversed` cần RenderSettings; keypad/ansi/kitty cần
    // TerminalInput port).
    // =====

    // ——— terminal_input mode set (10 method) ———
    fn set_cursor_key_mode(&mut self, _enable: bool) {}
    fn set_auto_repeat_mode(&mut self, _enable: bool) {}
    fn set_screen_reversed(&mut self, _enable: bool) {}
    fn set_default_mouse_tracking(&mut self, _enable: bool) {}
    fn set_any_event_mouse_tracking(&mut self, _enable: bool) {}
    fn set_focus_event(&mut self, _enable: bool) {}
    fn set_sgr_mouse_encoding(&mut self, _enable: bool) {}
    fn set_alternate_scroll(&mut self, _enable: bool) {}
    fn set_keypad_mode(&mut self, _application_mode: bool) {}
    fn set_ansi_mode(&mut self, _ansi_mode: bool) {}

    // ——— terminal_input mode get (10 method) ———
    fn get_cursor_key_mode(&self) -> bool {
        false
    }
    fn get_screen_reversed(&self) -> bool {
        false
    }
    fn get_autowrap_mode(&self) -> bool {
        false
    }
    fn get_auto_repeat_mode(&self) -> bool {
        false
    }
    fn get_default_mouse_tracking(&self) -> bool {
        false
    }
    fn get_any_event_mouse_tracking(&self) -> bool {
        false
    }
    fn get_focus_event(&self) -> bool {
        false
    }
    fn get_sgr_mouse_encoding(&self) -> bool {
        false
    }
    fn get_alternate_scroll(&self) -> bool {
        false
    }
    fn get_bracketed_paste(&self) -> bool {
        false
    }

    // ——— kitty keyboard (4 method) ———
    fn set_kitty_keyboard_protocol(&mut self, _flags: u32, _mode: u8) {}
    fn get_kitty_flags(&self) -> u32 {
        0
    }
    fn push_kitty_flags(&mut self, _flags: u32) {}
    fn pop_kitty_flags(&mut self, _count: usize) {}

    // ——— OSC action forward (9 method) ———
    fn set_current_working_directory(&mut self, _uri: &str) {}
    fn add_hyperlink(&mut self, _uri: &str, _params: &str) {}
    fn end_hyperlink(&mut self) {}
    fn do_con_emu_action(&mut self, _s: &str) {}
    fn do_iterm2_action(&mut self, _s: &str) {}
    fn do_final_term_action(&mut self, _s: &str) {}
    fn do_vs_code_action(&mut self, _s: &str) {}
    fn do_wt_action(&mut self, _s: &str) {}
    fn do_urxvt_action(&mut self, _s: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::interact::{DeviceAttributeSet, InputRecord};

    fn fresh() -> TerminalInteractDispatch {
        TerminalInteractDispatch::default()
    }

    #[test]
    fn default_host_is_empty() {
        let d = fresh();
        let h = d.host.borrow();
        assert_eq!(h.window_title, "");
        assert_eq!(h.clipboard, "");
        assert!(h.write_buffer.is_empty());
    }

    #[test]
    fn new_carries_host_state() {
        let host = HostContext {
            window_title: "title0".to_string(),
            clipboard: "clip0".to_string(),
            write_buffer: b"buf0".to_vec(),
        };
        let d = TerminalInteractDispatch::new(host);
        let h = d.host.borrow();
        assert_eq!(h.window_title, "title0");
        assert_eq!(h.clipboard, "clip0");
        assert_eq!(h.write_buffer, b"buf0");
    }

    #[test]
    fn write_file_appends_to_write_buffer() {
        let mut d = fresh();
        d.write_file("hello");
        d.write_file(" world");
        assert_eq!(d.host.borrow().write_buffer, b"hello world");
    }

    #[test]
    fn write_file_handles_utf8() {
        let mut d = fresh();
        d.write_file("Xin chào — VT");
        assert_eq!(d.host.borrow().write_buffer, "Xin chào — VT".as_bytes());
    }

    #[test]
    fn notify_cursor_change_is_no_op() {
        let mut d = fresh();
        d.notify_cursor_change();
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
        assert_eq!(h.window_title, "");
    }

    #[test]
    fn set_window_title_replaces() {
        let mut d = fresh();
        d.set_window_title("first");
        d.set_window_title("second");
        assert_eq!(d.host.borrow().window_title, "second");
    }

    #[test]
    fn set_clipboard_replaces() {
        let mut d = fresh();
        d.set_clipboard("a");
        d.set_clipboard("b");
        assert_eq!(d.host.borrow().clipboard, "b");
    }

    #[test]
    fn resize_window_is_no_op() {
        let mut d = fresh();
        d.resize_window(80, 24);
        let h = d.host.borrow();
        assert_eq!(h.window_title, "");
        assert!(h.write_buffer.is_empty());
    }

    #[test]
    fn notify_buffer_change_is_no_op() {
        let mut d = fresh();
        d.notify_buffer_change();
        assert!(d.host.borrow().write_buffer.is_empty());
    }

    #[test]
    fn warning_bell_is_no_op() {
        let mut d = fresh();
        d.warning_bell();
        assert!(d.host.borrow().write_buffer.is_empty());
    }

    #[test]
    fn with_host_borrows_immutably() {
        let d = fresh();
        d.with_host(|h| assert!(h.write_buffer.is_empty()));
    }

    #[test]
    fn with_host_mut_borrows_mutably() {
        let d = fresh();
        d.with_host_mut(|h| h.window_title = "x".to_string());
        assert_eq!(d.host.borrow().window_title, "x");
    }

    #[test]
    fn host_clone_roundtrips() {
        let d = fresh();
        let h: HostContext = d.host.borrow().clone();
        assert_eq!(h, HostContext::default());
    }

    // —— Phase 4.0-pre ext method override tests (loop 07) ——

    #[test]
    fn test_is_vt_input_enabled_returns_true() {
        let d = fresh();
        assert!(d.is_vt_input_enabled());
    }

    #[test]
    fn test_write_string_appends_to_write_buffer() {
        let mut d = fresh();
        d.write_string("hello");
        d.write_string(" world");
        assert_eq!(d.host.borrow().write_buffer, b"hello world");
    }

    #[test]
    fn test_write_string_raw_appends_bytes_verbatim() {
        let mut d = fresh();
        d.write_string_raw("VT");
        assert_eq!(d.host.borrow().write_buffer, b"VT");
    }

    #[test]
    fn test_focus_changed_is_no_op_and_does_not_panic() {
        let mut d = fresh();
        d.focus_changed(true);
        d.focus_changed(false);
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
        assert_eq!(h.window_title, "");
        assert_eq!(h.clipboard, "");
    }

    #[test]
    fn test_move_cursor_is_no_op() {
        let mut d = fresh();
        d.move_cursor(10, 20);
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
        assert_eq!(h.window_title, "");
        assert_eq!(h.clipboard, "");
    }

    #[test]
    fn test_set_device_attributes_is_no_op() {
        let mut d = fresh();
        d.set_device_attributes(DeviceAttributeSet(0xA9));
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
        assert_eq!(h.window_title, "");
        assert_eq!(h.clipboard, "");
    }

    #[test]
    fn test_write_input_is_no_op() {
        let mut d = fresh();
        d.write_input(&[InputRecord::default()]);
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
    }

    #[test]
    fn test_write_ctrl_key_is_no_op() {
        let mut d = fresh();
        d.write_ctrl_key(&InputRecord::default());
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
    }

    #[test]
    fn test_window_manipulation_is_no_op() {
        let mut d = fresh();
        d.window_manipulation(WindowManipulationType::None, 0, 0);
        let h = d.host.borrow();
        assert!(h.write_buffer.is_empty());
    }
}
