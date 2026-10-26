//! `ControlCore` — port `windows/src/cascadia/TerminalControl/ControlCore.cpp/.h`
//! (~3500 LOC). Là lớp wrapping `Terminal` (core) + `ITerminalConnection`
//! (connection): owns connection events, output-thread pump, terminal →
//! connection wire-up.
//!
//! Wire Lane A đầy đủ (Phase 4):
//! - `parser: vte::Parser` + `engine: OutputStateMachineEngine<TerminalDispatch>`
//!   — `connection_output_handler` gọi `parser.advance(&mut engine, bytes)`
//!   → `Perform` callbacks → `TerminalDispatch::method(&mut self.terminal, ...)`.
//! - `connection: Box<dyn ITerminalConnection>` (Lane A).
//! - `renderer: Box<dyn renderer::RenderEngine>` (Lane B).
//!
//! Anchors:
//! - `ControlCore::Initialize` `ControlCore.cpp:340-400`.
//! - `ControlCore::Start`     `ControlCore.cpp:400-480`.
//! - `_connectionOutputEventRevoker = _connection.TerminalOutput(...)`
//!   `ControlCore.cpp:352`.
//! - `_connectionStateChangedRevoker = newConnection.StateChanged(...)`
//!   `ControlCore.cpp:331`.
//!
//! Còn lại:
//! - `cx.notify()` đòi hỏi pump chạy trên main-thread entity (GPUI). Pump
//!   spawn đã chuyển sang `TermControl::spawn_output_thread` (cần `Context<T>`
//!   để notify).

use std::cell::RefCell;
use std::rc::Rc;

use connection::{ConnectionState, ITerminalConnection};
use core::dispatch::interact_impl::{HostContext, TerminalInteractDispatch};
use core::dispatch::td::TerminalDispatch;
use core::parser::OutputStateMachineEngine;
use core::TextBuffer;
use crossbeam_channel::Receiver;
use settings::Profile;

/// Resize the buffer and re-sync everything the dispatch derives from the page
/// size (viewport, scroll margins, tab stops, cursor).
pub fn resize_dispatch(d: &mut TerminalDispatch, rows: u16, cols: u16) {
    d.terminal.resize_traditional((cols, rows));
    d.viewport = d.terminal.get_size();
    d.scroll_margins = (0, rows as i32 - 1, 0, cols as i32 - 1);
    d.tab_stop_columns = (0..cols).map(|c| (c % 8 == 0) as u8).collect();
    let (x, y) = d.terminal.get_cursor().get_position();
    d.terminal
        .get_cursor_mut()
        .set_position(x.min(cols as i32 - 1), y.min(rows as i32 - 1));
}

/// Port `winrt::TerminalControl::ControlCore` (project réunion). Tách ra
/// khỏi XAML — bây giờ là thuần Rust + GPUI Event channel.
///
/// Output thread lifecycle (port `_ConnectionOutputHandler`)
/// `ControlCore.cpp:200-330`:
/// 1. `ITerminalConnection::TerminalOutput` callback feed bytes vào queue.
/// 2. `spawn_output_thread` bơm queue → `terminal` VTE parse (port `tg_core
///    ::parser::OutputStateMachineEngine`; phase 3 đợi `TerminalDispatch`
///    impl).
/// 3. `terminal.TensorAutoResize()` triggers `Connection.Resize(rows, cols)`.
pub struct ControlCore {
    /// VTE parser state-machine — persistent giữa các chunk (`vte::Parser` giữ
    /// internal state cho CSI/OSC/DCS sequence parsing). Port `OutputStateMachine`
    /// trong WT (đã thay bằng `vte` crate — xem `plans/phase4-terminal-dispatch.md`
    /// decision §vte 0.13 thay StateMachine.cpp).
    pub parser: vte::Parser,
    /// Lane A `core::parser::OutputStateMachineEngine<TerminalDispatch>` —
    /// engine owns dispatch owns `TextBuffer` + cursor. `connection_output_handler`
    /// gọi `parser.advance(&mut engine, byte)` → `Perform` callbacks →
    /// `TerminalDispatch::method(&mut self.terminal, ...)`. Port
    /// `_ConnectionOutputHandler` `ControlCore.cpp:200-330` lane C wire-up.
    pub engine: OutputStateMachineEngine<TerminalDispatch>,
    /// Lane A `connection::ITerminalConnection`.
    pub connection: Box<dyn ITerminalConnection>,
    /// Lane B `renderer::RenderEngine` (port xác nhận: `RenderEngine` là
    /// trait với `paint_*` step sequence).
    pub renderer: Box<dyn renderer::RenderEngine>,
    pub settings: Profile,
    pub rows: u16,
    pub cols: u16,
    /// Subscribe `ITerminalConnection::TerminalOutput` channel; fill ở
    /// `initialize()` và consume ở `spawn_output_thread()`.
    pub output_rx: Option<Receiver<String>>,
    /// Subscribe `ITerminalConnection::StateChanged`.
    pub state_rx: Option<Receiver<ConnectionState>>,
    /// Shared host state of `TerminalInteractDispatch`: window title and the
    /// reply bytes (DSR/DA) the parser wants sent back to the shell.
    pub host: Rc<RefCell<HostContext>>,
    /// Set once the shell has exited: the connection is no longer resized
    /// (ConPTY would repaint over the exit message).
    pub exited: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ControlCore {
    /// Port `ControlCore::ControlCore(Terminal, IConnection, IRenderer,
    /// Settings)` `ControlCore.cpp:120-180`. WT ctor nhận `Terminal` (sở hữu
    /// `TextBuffer`); ta thay bằng `TextBuffer` trực tiếp + wrap trong
    /// `OutputStateMachineEngine<TerminalDispatch>` (engine owns dispatch owns
    /// textbuffer). `parser` init once (stateless giữa các ctor call).
    ///
    /// `interact` bridge: lane C (control) inject `TerminalInteractDispatch::default`
    /// (test host context) — Phase 5 wire-up injects real gpui host state via
    /// `HostContext` (window title / clipboard / write_buffer DSR reports).
    pub fn new(
        terminal: TextBuffer,
        connection: Box<dyn ITerminalConnection>,
        renderer: Box<dyn renderer::RenderEngine>,
        settings: Profile,
    ) -> Self {
        let (cols, rows) = terminal.size();
        let interact = TerminalInteractDispatch::default();
        let host = interact.host.clone();
        let interact: Box<dyn core::dispatch::InteractDispatch> = Box::new(interact);
        let dispatch = TerminalDispatch::new(terminal, interact);
        let engine = OutputStateMachineEngine::new(dispatch);
        Self {
            parser: vte::Parser::new(),
            engine,
            connection,
            renderer,
            settings,
            rows,
            cols,
            output_rx: None,
            state_rx: None,
            host,
            exited: Default::default(),
        }
    }

    /// Port `ControlCore::Initialize` `ControlCore.cpp:340-400`: wire
    /// `_connectionOutputEventRevoker = _connection.TerminalOutput(...)`
    /// (`ControlCore.cpp:352`) và
    /// `_connectionStateChangedRevoker = newConnection.StateChanged(...)`
    /// (`ControlCore.cpp:331`). Subscribe cả hai `Event<T>` channel.
    pub fn initialize(&mut self) {
        // `TerminalOutputEvent::subscribe() -> Receiver<String>` — port
        // WinRT `event_token` pattern (`ConptyConnection.cpp:779`).
        self.output_rx = Some(self.connection.terminal_output().subscribe());
        self.state_rx = Some(self.connection.state_changed().subscribe());
    }

    /// Port `ControlCore::Start` `ControlCore.cpp:400-480`: gọi
    /// `connection.Start()`. Anchor `_Start()` khi Terminal ready. Returns
    /// `anyhow::Result` từ `ITerminalConnection::start` (port: `cpp:478`).
    pub fn start(&mut self) -> anyhow::Result<()> {
        self.connection.start()
    }

    /// Accessor cho output channel receiver — `spawn_output_thread` consume.
    pub fn take_output_rx(&mut self) -> Option<Receiver<String>> {
        self.output_rx.take()
    }

    /// Accessor cho state channel receiver.
    pub fn take_state_rx(&mut self) -> Option<Receiver<ConnectionState>> {
        self.state_rx.take()
    }

    /// Port `ControlCore::SizeChanged` `ControlCore.cpp:480-540`: gọi
    /// `connection.Resize(rows, cols)` + cập nhật terminal viewport. WT
    /// `_SizeChanged(newSize)` fire khi XAML SizeChanged event → ta dùng
    /// GPUI `Entity::on_window_resized`.
    pub fn size_changed(&mut self, new_rows: u16, new_cols: u16) -> anyhow::Result<()> {
        let (new_rows, new_cols) = (new_rows.max(1), new_cols.max(1));
        if (new_rows, new_cols) == (self.rows, self.cols) {
            return Ok(());
        }
        self.rows = new_rows;
        self.cols = new_cols;
        // Resize the buffer, then re-sync everything the dispatch derives from
        // the page size (viewport, scroll margins, tab stops, cursor).
        resize_dispatch(&mut self.engine.dispatch, new_rows, new_cols);
        if self.exited.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        self.connection.resize(new_rows as u32, new_cols as u32)
    }

    /// Port `ControlCore::UpdateSettings` `ControlCore.cpp:540-620`: thay
    /// profile, propagate tới terminal (font/color/bell) + renderer. Phase
    /// 3 wire — renderer chưa có `update_settings(profile)`.
    pub fn update_settings(&mut self, new_settings: Profile) {
        self.settings = new_settings;
        // Phase 3: propagate tới renderer (`RenderEngine::update_font` ...)
        // khi Lane B finalize per-engine settings API.
    }

    /// Port `ControlCore::SendInput` `ControlCore.cpp:620-680`: gọi
    /// `connection.WriteInput(text)`. WT API nhận UTF-16 — port
    /// `ConptyConnection.cpp:553-600` encode UTF-8 → UTF-16 here.
    pub fn send_input(&mut self, text: &str) -> anyhow::Result<()> {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        self.connection.write_input(&utf16)
    }

    /// Port `ControlCore::Close` `ControlCore.cpp:680-760`: connection.close()
    /// + final teardown + join output thread. Returns Result; caller spawn
    ///   `Task` phải detach hoặc giữ.
    pub fn close(&mut self) -> anyhow::Result<()> {
        self.connection.close()
    }

    /// Port `_ConnectionOutputHandler` `ControlCore.cpp:200-330`: nhận bytes
    /// từ `ITerminalConnection::TerminalOutput`, gửi vào VTE parser →
    /// `terminal.Write()`. Synchronous — gọi từ output-thread pump (không
    /// lock; chỉ lock nếu pump cross-thread).
    ///
    /// Wire Lane A đầy đủ: `vte::Parser::advance(&mut engine, bytes)` → engine
    /// là `OutputStateMachineEngine<TerminalDispatch>` impl `vte::Perform` —
    /// callbacks (`print`, `csi`, `osc` ...) gọi `TerminalDispatch::method(&mut
    /// self.terminal, ...)` mutate `TextBuffer`. Parser giữ state giữa các
    /// chunk (CSI/OSC/DCS sequence có thể split cross-chunk).
    pub fn connection_output_handler(&mut self, output: &str) {
        // vte 0.13 `Parser::advance(byte: u8)` per-byte; parser state persists
        // across chunks (CSI/OSC/DCS can be split).
        for byte in output.as_bytes() {
            self.parser.advance(&mut self.engine, *byte);
        }
        self.flush_replies();
    }

    /// Send parser replies (DSR/DA, `WriteFile`) back to the shell.
    fn flush_replies(&mut self) {
        let bytes = std::mem::take(&mut self.host.borrow_mut().write_buffer);
        if !bytes.is_empty() {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            if let Err(e) = self.send_input(&text) {
                log::warn!("failed to send VT reply to shell: {e}");
            }
        }
    }

    /// Latest window title set by the shell (OSC 0/2); empty if none.
    pub fn title(&self) -> String {
        self.host.borrow().window_title.clone()
    }

    /// Port `_ConnectionStateChangedHandler` `ControlCore.cpp:331-350`:
    /// consume `StateChangedEvent` channel và fire `terminal_output()` /
    /// `state_changed()` event để UI. Trả về state nếu có event chờ, để
    /// caller quyết định raise tới GPUI entity.
    pub fn connection_state_changed(&mut self) -> Option<ConnectionState> {
        self.state_rx.as_ref().and_then(|rx| rx.try_recv().ok())
    }

    /// `ControlCore::spawn_output_thread` đã **chuyển sang `TermControl::spawn_output_thread`**
    /// (`control.rs:124-181`) — `cx: Context<TermControl>` cho phép `WeakEntity::update`
    /// để notify GPUI render. Lớp `ControlCore` chỉ còn giữ
    /// `connection_output_handler(chunk)` (`core.rs:181-188`) cho callback + VTE parse.
    ///
    /// Hàm giữ dạng thin wrapper trả `None` cho parity anchor reference — caller
    /// phải dùng TermControl path để có `cx.spawn` re-render. Drain `output_rx`
    /// tránh subscription leak nếu gọi nhầm. Phong tran strategy: giữ chữ ký này
    /// cho même nếu Phase 5+ grow trait surface cross-crate.
    #[deprecated(
        note = "use TermControl::spawn_output_thread — ControlCore không có WeakEntity context"
    )]
    pub fn spawn_output_thread(&mut self, _cx: &mut gpui::App) -> Option<gpui::Task<()>> {
        let _ = self.take_output_rx()?;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirror: `TextBuffer` thật từ `tg_core` (not assumed shim).
    #[test]
    fn core_owns_real_text_buffer() {
        let tb = TextBuffer::new(80, 24, core::TextAttribute::default());
        assert_eq!(tb.size(), (80, 24));
    }

    /// Verify `ConnectionState` enum là real (`NotConnected`..`Failed`).
    #[test]
    fn connection_state_real_variants() {
        assert_eq!(ConnectionState::NotConnected as u8, 0);
        assert!(ConnectionState::Closed.is_terminal());
        assert!(ConnectionState::Failed.is_terminal());
    }
}
