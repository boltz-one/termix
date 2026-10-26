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
//! 16 method target: 7 existing landed (real body), 8 Phase 4.0-pre ext
//! (`todo!()` WAIT guard), 1 (`focus_changed`) also pending Phase 4.0-pre
//! — documented in `WAIT_METHODS` below.

use std::cell::RefCell;
use std::rc::Rc;

use super::interact::InteractDispatch;
use super::types::WindowManipulationType;
use super::{VtInt, VtParameter};

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

// —— Phase 4.0-pre trait ext, NOT landed yet (`interact.rs` trait) ——
// 8 ext methods below are written as inherent methods with `todo!()` bodies
// plus `WAIT 4.0-pre: trait ext not landed` marker. When Phase 4.0-pre adds
// them to `InteractDispatch`, drop the inherent `impl` items and re-attach as
// `impl InteractDispatch for TerminalInteractDispatch { ... }`. `focus_changed`
// (the 9th `IInteractDispatch` virtual) is also pending; add it then.
//
// Target `todo!()` count = 8 (the 8 named ext methods).

impl TerminalInteractDispatch {
    /// `SetDeviceAttributes` (DA report). Writes CSI > c reply into write_buffer.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn set_device_attributes(&mut self, _attributes: u64) {
        todo!("InteractDispatch::SetDeviceAttributes // WAIT 4.0-pre: trait ext not landed")
    }

    /// `WriteInput` — host forwards input events. Lane C (Phase 5) wires this.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn write_input(&mut self, _events: &[u8]) {
        todo!("InteractDispatch::WriteInput // WAIT 4.0-pre: trait ext not landed")
    }

    /// `WriteCtrlKey` — ctrl-key input. Lane C wires in Phase 5.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn write_ctrl_key(&mut self, _event: u16) {
        todo!("InteractDispatch::WriteCtrlKey // WAIT 4.0-pre: trait ext not landed")
    }

    /// `WriteString` — push UTF-8 string into write_buffer.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn write_string(&mut self, _string: &str) {
        todo!("InteractDispatch::WriteString // WAIT 4.0-pre: trait ext not landed")
    }

    /// `WriteStringRaw` — push raw bytes into write_buffer (no encoding xfrm).
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn write_string_raw(&mut self, _bytes: &[u8]) {
        todo!("InteractDispatch::WriteStringRaw // WAIT 4.0-pre: trait ext not landed")
    }

    /// `WindowManipulation` — xterm window ops. Phase 4 minimal: parse + no-op.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn window_manipulation(
        &mut self,
        _function: WindowManipulationType,
        _parameter1: VtParameter,
        _parameter2: VtParameter,
    ) {
        todo!("InteractDispatch::WindowManipulation // WAIT 4.0-pre: trait ext not landed")
    }

    /// `MoveCursor` — host moves cursor visually. Lane C wires in Phase 5.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn move_cursor(&mut self, _row: VtInt, _col: VtInt) {
        todo!("InteractDispatch::MoveCursor // WAIT 4.0-pre: trait ext not landed")
    }

    /// `IsVtInputEnabled` — VT input mode flag. Default `true`.
    /// `// WAIT 4.0-pre: trait ext not landed`
    pub fn is_vt_input_enabled(&self) -> bool {
        todo!("InteractDispatch::IsVtInputEnabled // WAIT 4.0-pre: trait ext not landed")
    }
}

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
}

#[cfg(test)]
#[allow(dead_code)]
/// Count `todo!()` markers expected to remain once Phase 4.0-pre lands the
/// trait ext. Phase 4.0-pre completion → these inherent methods fold into
/// `impl InteractDispatch for TerminalInteractDispatch` and `todo!()` count
/// drops to zero.
mod wait_phase40_pre {
    /// Total ext methods awaiting trait surface = 8 (named in brief) + 1
    /// (`focus_changed`, tracked in WAIT comment block).
    const EXT_METHODS_PENDING: u32 = 8;
    const FOCUS_CHANGED_PENDING: u32 = 1;
    const TOTAL_PENDING: u32 = EXT_METHODS_PENDING + FOCUS_CHANGED_PENDING;
    const _COMPILE_TIME_GUARD: u32 = TOTAL_PENDING;
}
