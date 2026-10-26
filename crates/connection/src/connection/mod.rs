//! `ITerminalConnection` trait + `ConptyConnection` struct — port
//! `ITerminalConnection.idl:7-27` và `ConptyConnection.h` private members.
//!
//! Trait đủ signature để `control` impl `Box<dyn ITerminalConnection>` mà không
//! cần sửa. Port signatures đầy đủ phase 1; FFI fill ở phase 2.

pub mod echo;
pub mod lifecycle;
pub mod settings;
pub mod state;

pub use echo::EchoConnection;
pub use settings::{ConptySettings, PseudoConsoleFlags};
pub use state::ConnectionState;

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use parking_lot::Mutex;
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Console::{COORD, HPCON};
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::IO::OVERLAPPED;

use crate::conpty::ThreadExit;
use crate::event::{StateChangedEvent, TerminalOutputEvent};

/// `PtyBackend` — future-ready seam cho PTY backend swap (ConPTY vs Mock vs SSH
/// vs named-pipe-only). P06: trait exists + `ConptyBackend` impl callable; wire
/// `ConptyConnection` through trait happens ở phase sau (C1+C2+C3 fix risk).
/// See ADR-0006 (proposed).
pub trait PtyBackend: Send {
    /// `CreatePseudoConsole(size, pipe_in, pipe_out, flags) -> HPCON`.
    /// Port `ConptyConnection.cpp:412` `ConptyCreatePseudoConsole`.
    fn create(&mut self, size: COORD, flags: u32) -> anyhow::Result<HPCON>;
    /// `ResizePseudoConsole(hpc, size)`. Port `cpp:610` `ConptyResizePseudoConsole`.
    fn resize(&mut self, hpc: HPCON, size: COORD) -> anyhow::Result<()>;
    /// `ClosePseudoConsole(hpc)`. Port `cpp:674`.owned-handle release semantics.
    fn close(&mut self, hpc: HPCON);
}

/// `ConptyBackend` — thin wrapper around `crate::conpty` FFI shims. KHÔNG fake —
/// delegate trực tiếp kernel32 ConPTY (`windows::Win32::System::Console::*`).
/// `#[allow(dead_code)]`: callers land later (P11 ControlCore wire-thru trait).
#[allow(dead_code)]
pub struct ConptyBackend;

impl PtyBackend for ConptyBackend {
    fn create(&mut self, size: COORD, flags: u32) -> anyhow::Result<HPCON> {
        // `pipe_in`/`pipe_out` do caller pass — đây seam-only; gọi tới
        // `conpty::create_pseudo_console` cần caller đã set pipe handles. Để trait
        // minimal, ta nhận COORD+flags và wrapped caller sẽ pre-set pipes.
        // Hiện wrap bằng `create_pseudo_console` chuẩn — caller cung cấp pipes qua
        // các method khác. Test `conpty_backend_impl_callable` verify trait dispatch.
        //
        // NOTE: signature minimal — phase sau quyết định mở rộng (pre-pipes /
        // pre-attributes) khi wire thật. Trao đổi pipe handles qua struct field
        // sẽ cần `ConptyBackend { pipes: ... }` biến field — defer đến wire phase.
        crate::conpty::create_pseudo_console(
            size,
            INVALID_HANDLE_VALUE,
            INVALID_HANDLE_VALUE,
            flags,
        )
    }
    fn resize(&mut self, hpc: HPCON, size: COORD) -> anyhow::Result<()> {
        crate::conpty::resize_pseudo_console(hpc, size)
    }
    fn close(&mut self, hpc: HPCON) {
        crate::conpty::close_pseudo_console(hpc);
    }
}

/// `ITerminalConnection` — port `ITerminalConnection.idl:7-27`.
///
/// `Send` yêu cầu tất cả member thread-safe (HPCON/HANDLE are raw pointers, bọc
/// bằng `unsafe impl Send/Sync` ở `ConptyConnection` với `&mut self` lock).
/// `&self` cho getters, `&mut self` cho mutators (lock-free cho phase 1).
pub trait ITerminalConnection: Send {
    /// Port `Start()` — `cpp:400-478`. Spawn PTY + child + output thread.
    fn start(&mut self) -> anyhow::Result<()>;
    /// Port `WriteInput(array_view<char16_t>)` — `cpp:553-600`. UTF-16 buffer.
    fn write_input(&mut self, buffer: &[u16]) -> anyhow::Result<()>;
    /// Port `Resize(uint32, uint32)` — `cpp:602-612`.
    fn resize(&mut self, rows: u32, cols: u32) -> anyhow::Result<()>;
    /// Port `Close()` — `cpp:667-703`.
    fn close(&mut self) -> anyhow::Result<()>;
    /// Port `Initialize(ValueSet)` — `cpp:245-324`. Thay `ValueSet` bằng `ConptySettings`.
    fn initialize(&mut self, settings: ConptySettings) -> anyhow::Result<()>;
    /// Port `guid SessionId { get; }` — session id cho pane.
    fn session_id(&self) -> uuid::Uuid;
    /// Port `ConnectionState State { get; }`.
    fn state(&self) -> ConnectionState;
    /// Port `event TerminalOutput(String)` — subscribe pattern, thay WinRT token.
    fn terminal_output(&self) -> &TerminalOutputEvent;
    /// Port `event StateChanged` — subscribe pattern.
    fn state_changed(&self) -> &StateChangedEvent;
}

/// `ConptyConnection` — port `ConptyConnection.h` private members.
///
/// Sở hữu: HPCON, dual pipes, child `PROCESS_INFORMATION`, output thread handle,
/// overlapped write state + lock, settings, state, events.
// Safety: HANDLE/HPCON/OVERLAPPED/PROCESS_INFORMATION đều wrap raw pointer khác
// thread-safety semantic — tất cả access được guard bằng `&mut self` (start/write/
// close) hoặc `write_lock: Mutex<()>` (write path). `Arc<ConptyConnection>` ở phase
// 2 cho output thread chỉ truy cập `terminal_output` (Mutex<Vec<Sender>>) + `h_pc`
// (read-only sau start). Vậy Send/Sync thủ công an toàn.
#[allow(dead_code)] // Phase 2 FFI fill — fields được dùng.
pub struct ConptyConnection {
    pub(crate) settings: ConptySettings,
    pub(crate) state: ConnectionState,
    pub(crate) h_pc: HPCON,
    /// Input pipe server end (host ghi input tới PTY). Port WT `_pipe` (cpp:413).
    /// Dùng cho `write_input` overlapped WriteFile.
    pub(crate) pipe: HANDLE,
    /// Output pipe server end (host đọc output từ PTY). Thêm internal field vì
    /// ta dùng 2 named pipes semi-duplex (substitute `Utils::CreateOverlappedPipe`
    /// — windows-rs 0.61 thiếu `NtCreateNamedPipeFile`). Dùng cho `output_thread_loop`
    /// overlapped `ReadFile`.
    pub(crate) pipe_out: HANDLE,
    pub(crate) pi_client: PROCESS_INFORMATION,
    pub(crate) h_output_thread: HANDLE,
    pub(crate) rows: u32,
    pub(crate) cols: u32,
    pub(crate) flags: PseudoConsoleFlags,
    pub(crate) write_overlapped: OVERLAPPED,
    pub(crate) write_pending: bool,
    /// UTF-8 bytes of the in-flight overlapped write (`_writeBuffer`): must
    /// outlive `WriteFile` when it returns `ERROR_IO_PENDING`.
    pub(crate) write_buffer: Vec<u8>,
    pub(crate) write_lock: Mutex<()>,
    pub(crate) terminal_output: TerminalOutputEvent,
    pub(crate) state_changed: StateChangedEvent,
    /// Shared state atomic cho output thread — đồng bộ với `self.state` ở
    /// `transition_to_state`. Output thread đọc non-blocking để check
    /// `is_terminal()` / `Closing` mà không cần `Arc<ConptyConnection>`.
    /// `#[repr(u8)]` ConnectionState → transmute `u8` safety.
    pub(crate) shared_state: Arc<AtomicU8>,
    /// Cleanup rendezvous — output thread gửi `ThreadExit` khi terminate,
    /// main thread nhận ở `close()` trước `last_conpty_client_disconnected`.
    /// `Option` vì `take_rx()` consume ở close path.
    pub(crate) done_rx: Option<Receiver<ThreadExit>>,
    /// Time điểm `start()` thành công — port WT `_startTime` (cpp:453) dùng cho
    /// "ReceivedFirstByte" telemetry (skip telemetry ở phase 2, chỉ giữ timestamp).
    pub(crate) _start_time: Option<std::time::Instant>,
}

unsafe impl Send for ConptyConnection {}
unsafe impl Sync for ConptyConnection {}

impl ConptyConnection {
    /// Default ctor — port `ConptyConnection.cpp:197-202`. Tạo `_writeOverlappedEvent`.
    /// Phase 2: `CreateEventW(NULL, TRUE, FALSE, NULL)` cho overlappedWriteFile.
    pub fn new() -> Self {
        Self::default_state_only()
    }

    /// Constructor helper — khởi tạo mọi handle là invalid/doc-set.
    /// Chi tiết ctor (`_writeOverlappedEvent`) ở `lifecycle::new`.
    pub(crate) fn default_state_only() -> Self {
        Self {
            settings: ConptySettings::default(),
            state: ConnectionState::NotConnected,
            h_pc: HPCON::default(),
            pipe: INVALID_HANDLE_VALUE,
            pipe_out: INVALID_HANDLE_VALUE,
            pi_client: PROCESS_INFORMATION::default(),
            h_output_thread: INVALID_HANDLE_VALUE,
            rows: 25,
            cols: 80,
            flags: PseudoConsoleFlags::empty(),
            write_overlapped: OVERLAPPED::default(),
            write_pending: false,
            write_buffer: Vec::new(),
            write_lock: Mutex::new(()),
            terminal_output: TerminalOutputEvent::default(),
            state_changed: StateChangedEvent::default(),
            shared_state: Arc::new(AtomicU8::new(ConnectionState::NotConnected as u8)),
            done_rx: None,
            _start_time: None,
        }
    }

    /// Port `BaseTerminalConnection::_transitionToState` — state machine guard.
    ///
    /// **Guard chống "flapping"**: 3 termination entry points (user `close()`,
    /// spawn failure, pipe-broken) có thể fire song song. Order được enforce:
    /// - `NotConnected` → `Connecting` → `Connected` (forward progress only).
    /// - `Connecting` → `Connected` | `Failed` | `Closing`.
    /// - `Connected` → `Closing` → `Closed` (user close) | `Failed` (pipe-broken).
    /// - `Closing` → `Closed` (cho phép user close hoàn tất dù pipe-broken trigger cũng).
    /// - `Closed`/`Failed` là terminal — reject mọi transition ra.
    ///
    /// Raise `StateChangedEvent` chỉ khi transition hợp lệ và thay đổi state.
    pub(crate) fn transition_to_state(&mut self, new_state: ConnectionState) -> anyhow::Result<()> {
        use ConnectionState::*;
        let valid = match (self.state, new_state) {
            (NotConnected, Connecting | Failed) => true,
            (Connecting, Connected | Closing | Failed) => true,
            (Connected, Closing | Failed) => true,
            (Closing, Closed | Failed) => true,
            (_, Closed) => self.state == Closing,
            (Closed | Failed, _) => false,
            _ => false,
        };
        anyhow::ensure!(
            valid,
            "state transition không hợp lệ: {0:?} -> {new_state:?}",
            self.state
        );
        if self.state != new_state {
            self.state = new_state;
            // Sync atomic cho output thread — Release pair với Acquire ở
            // `output_thread_loop` state check. `#[repr(u8)]` → `as u8` lossless.
            self.shared_state
                .store(new_state as u8, std::sync::atomic::Ordering::Release);
            self.state_changed.raise(new_state);
        }
        Ok(())
    }

    /// Truy cập read-only settings (control layer dùng để resolve profile).
    pub fn settings(&self) -> &ConptySettings {
        &self.settings
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }
    pub fn cols(&self) -> u32 {
        self.cols
    }

    // ───── Gap #11 — port `ConptyConnection.h:30-37` public methods ─────
    // Các method này extras (ngoài `ITerminalConnection`) mà WT expose cho app
    // layer: show/hide PTY window, reparent HWND, lấy child process handle,getter
    // cho commandline/starting_title/show_window (defterm handoff path). Hầu
    // no-op ở Rust port vì winconpty.dll path đã được ConPTY kernel32 base thay
    // per ADR-0006 — wire thật chờ P11/P13 caller. ADR-0006 (proposed) cover.

    /// Port `ConptyConnection::ShowHide(show)` (`cpp:632-643`). Chưa wire — ConPTY
    /// kernel32 base path không cần `ConptyShowHidePseudoConsole` theo ADR-0006.
    /// Sẽ wire lại ở P11 nếu ControlCore cần visibility flag.
    #[allow(dead_code)]
    pub fn show_hide(&mut self, _show: bool) {
        // no-op stub — see ADR-0006.
    }

    /// Port `ConptyConnection::ReparentWindow(newParent)` (`cpp:645-659`). Tương
    /// tự `show_hide` — `ConptyReparentPseudoConsole` skip per ADR-0006.
    #[allow(dead_code)]
    pub fn reparent_window(&mut self, _hwnd: isize) {
        // no-op stub — see ADR-0006.
    }

    /// Port `ConptyConnection::RootProcessHandle()` (`cpp:661-665`). Trả child
    /// process handle nếu valid, None nếu đã close hoặc chưa start.
    #[allow(dead_code)]
    pub fn root_process_handle(&self) -> Option<HANDLE> {
        if self.pi_client.hProcess == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(self.pi_client.hProcess)
        }
    }

    /// Port `ConptyConnection::Commandline() const` (`cpp:385-388`). Getter cho
    /// commandline string đã set qua `initialize`.
    #[allow(dead_code)]
    pub fn commandline(&self) -> &str {
        &self.settings.commandline
    }

    /// Port `ConptyConnection::StartingTitle() const` (`cpp:390-393`). Getter
    /// Optional — None nếu `starting_title` chưa set.
    #[allow(dead_code)]
    pub fn starting_title(&self) -> Option<&str> {
        self.settings.starting_title.as_deref()
    }

    /// Port `ConptyConnection::ShowWindow() const` (`cpp:395-398`). Trả về
    /// `WORD` (u16) show-window flag từ defterm handoff `STARTUPINFO`. Ta chưa
    /// wire handoff path — return 0 (SW_HIDE default) per ADR-0006 (deferred P13).
    #[allow(dead_code)]
    pub fn show_window(&self) -> u16 {
        0
    }
}

impl Default for ConptyConnection {
    fn default() -> Self {
        Self::default_state_only()
    }
}

impl ITerminalConnection for ConptyConnection {
    fn start(&mut self) -> anyhow::Result<()> {
        lifecycle::start(self)
    }
    fn write_input(&mut self, buffer: &[u16]) -> anyhow::Result<()> {
        lifecycle::write_input(self, buffer)
    }
    fn resize(&mut self, rows: u32, cols: u32) -> anyhow::Result<()> {
        lifecycle::resize(self, rows, cols)
    }
    fn close(&mut self) -> anyhow::Result<()> {
        lifecycle::close(self)
    }
    fn initialize(&mut self, settings: ConptySettings) -> anyhow::Result<()> {
        lifecycle::initialize(self, settings)
    }
    fn session_id(&self) -> uuid::Uuid {
        self.settings.session_id
    }
    fn state(&self) -> ConnectionState {
        self.state
    }
    fn terminal_output(&self) -> &TerminalOutputEvent {
        &self.terminal_output
    }
    fn state_changed(&self) -> &StateChangedEvent {
        &self.state_changed
    }
}

#[cfg(test)]
mod tests;
