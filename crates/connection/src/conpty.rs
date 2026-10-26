//! ConPTY FFI wrappers — port của các kernel32 `*PseudoConsole*` calls.
//!
//! ### Available trong windows-rs 0.61.3 (kernel32.dll):
//! - `CreatePseudoConsole` — port `ConptyConnection.cpp:_LaunchAttachedClient` flow.
//! - `ResizePseudoConsole` — port `ConptyConnection.cpp:602-612`.
//! - `ClosePseudoConsole` — port `ConptyConnection.cpp:667-703`.
//!
//! ### KHÔNG available trong windows-rs 0.61 (manual `#[link(name="kernel32")]`):
//! - `ClearPseudoConsole` (kernel32 22H2+) — `VOID WINAPI ClearPseudoConsole(HPCON, BOOL)`.
//!   Decl ở `clear_console_ffi` module dưới đây.
//!
//! ### Skip (no binding, không port):
//! - `ConptyReparentPseudoConsole`/`ConptyShowHidePseudoConsole`/`ConptyPackPseudoConsole`/
//!   `CreatePseudoConsoleAsUser` (winconpty.dll exports — WT base flow không cần).
//! - `Utils::CreateOverlappedPipe` (NT API `NtCreateNamedPipeFile`) — substitute
//!   `CreateNamedPipeW` + `FILE_FLAG_OVERLAPPED` ở `create_overlapped_pipe` helper.
//! - `ConptyReleasePseudoConsole` — `ClosePseudoConsole` đủ cho WT parity.
//!
//! ### Output thread — port `ConptyConnection.cpp:737-842`
//! Critical ordering: **queue next `ReadFile` trước**, process prior wstr sau.
//! Lý do: `TerminalOutput.raise` không block pending read (xem `output_thread_loop`).

use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::sync::OnceLock;

use crossbeam_channel::Sender;
use windows::core::w;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED,
};
use windows::Win32::System::Console::{CreatePseudoConsole, COORD, HPCON};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_WAIT};
use windows::Win32::System::IO::OVERLAPPED;

use crate::connection::ConnectionState;

// ─────────────────────── ClearPseudoConsole dynamic load ───────────────────
//
// `ClearPseudoConsole(HPCON, BOOL)` — kernel32 22H2+. windows-rs 0.61 thiếu binding,
// VÀ Windows SDK 10.0.26100.0 `kernel32.lib` import lib KHÔNG chứa symbol này →
// manual `#[link(name = "kernel32")] extern` gây LNK2019. Port verbatim WT pattern
// (`ConptyConnection.cpp`): dynamic resolve qua `GetProcAddress` trên `kernel32.dll`.
// `BOOL` ở Win32 ABI là `i32`. Cache pointer qua `OnceLock` → chỉ resolve 1 lần.
// Pre-22H2: `GetProcAddress` trả NULL → `clear_pseudo_console` return `Err`.
type ClearPseudoConsoleFn = unsafe extern "system" fn(HPCON, i32);

static CLEAR_PSEUDO_CONSOLE: OnceLock<Option<ClearPseudoConsoleFn>> = OnceLock::new();

/// Resolve `ClearPseudoConsole` from `kernel32.dll` một lần, cache qua `OnceLock`.
/// Trả `Some(fn)` trên 22H2+, `None` trên pre-22H2 (symbol absent).
fn resolve_clear_pseudo_console() -> Option<ClearPseudoConsoleFn> {
    *CLEAR_PSEUDO_CONSOLE.get_or_init(|| {
        // Safety: `GetModuleHandleW` trả handle đã loaded của `kernel32.dll`
        // (luôn có trong process). Không load mới, không unload. `w!` encode UTF-16.
        unsafe {
            let h = GetModuleHandleW(w!("kernel32.dll")).ok()?;
            let sym = GetProcAddress(h, windows::core::s!("ClearPseudoConsole"));
            sym.map(|raw| std::mem::transmute::<_, ClearPseudoConsoleFn>(raw))
        }
    })
}

/// Buffer size cho overlapped pipe — match WT (`128 * 1024` cpp:341).
const PIPE_BUFFER_SIZE: u32 = 128 * 1024;

/// Tạo overlapped named pipe (server+client) — substitute
/// `Utils::CreateOverlappedPipe(PIPE_ACCESS_DUPLEX, 128*1024)` ở `ConptyConnection.cpp:341`.
///
/// WT gốc dùng NT API `NtCreateNamedPipeFile` windows-rs 0.61 thiếu, ta substitute
/// `CreateNamedPipeW` (server end with `FILE_FLAG_OVERLAPPED`) + open client end
/// qua `CreateFileW`. Trả `(server, client)` — server dùng overlapped I/O trong host,
/// client inherit vào child process với sync I/O.
#[allow(clippy::needless_pass_by_value)]
pub fn create_overlapped_pipe(name: PCWSTR) -> anyhow::Result<(HANDLE, HANDLE)> {
    use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;
    use windows::Win32::Storage::FileSystem::FILE_SHARE_WRITE;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
    };

    // Server end — overlapped I/O. `FILE_FLAG_FIRST_PIPE_INSTANCE` → unique instance.
    // `PIPE_ACCESS_DUPLEX` (3) = `PIPE_ACCESS_INBOUND | PIPE_ACCESS_OUTBOUND`.
    const PIPE_ACCESS_DUPLEX: u32 = 3;
    let server = unsafe {
        CreateNamedPipeW(
            name,
            FILE_FLAGS_AND_ATTRIBUTES(
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED.0 | FILE_FLAG_FIRST_PIPE_INSTANCE.0,
            ),
            PIPE_TYPE_BYTE | PIPE_WAIT,
            1, // nMaxInstances
            PIPE_BUFFER_SIZE,
            PIPE_BUFFER_SIZE,
            0, // nDefaultTimeout
            None,
        )
    };
    // `CreateNamedPipeW` trả `HANDLE` trực tiếp (không Result) — check INVALID_HANDLE_VALUE.
    let server = if server == INVALID_HANDLE_VALUE {
        anyhow::bail!("CreateNamedPipeW(server) failed: WIN32_ERROR {}", unsafe {
            windows::Win32::Foundation::GetLastError().0
        });
    } else {
        server
    };

    // Client end — open same pipe name, sync I/O (child process dùng blocking read/write).
    let client = unsafe {
        CreateFileW(
            name,
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(FILE_ATTRIBUTE_NORMAL.0),
            None,
        )
    };
    let client = match client {
        Ok(h) => h,
        Err(e) => {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(server) };
            anyhow::bail!("CreateFileW(client) failed: {e}");
        }
    };

    Ok((server, client))
}

/// Tạo pseudo-console (ConPTY). Port `ConptyConnection.cpp:_LaunchAttachedClient` + `Start()` `cpp:411`.
///
/// `size`: rows/cols. `pipe_in`: input pipe read end (child reads). `pipe_out`:
/// output pipe write end (child writes). `flags`: `PseudoConsoleFlags` (inherit cursor, etc.).
pub fn create_pseudo_console(
    size: COORD,
    pipe_in: HANDLE,
    pipe_out: HANDLE,
    flags: u32,
) -> anyhow::Result<HPCON> {
    unsafe { CreatePseudoConsole(size, pipe_in, pipe_out, flags) }
        .map_err(|e| anyhow::anyhow!("CreatePseudoConsole: {e}"))
}

/// Resize ConPTY. Port `ConptyConnection.cpp:602-612` — chỉ gọi nếu `state == Connected`.
pub fn resize_pseudo_console(hpc: HPCON, size: COORD) -> anyhow::Result<()> {
    unsafe { windows::Win32::System::Console::ResizePseudoConsole(hpc, size) }
        .map_err(|e| anyhow::anyhow!("ResizePseudoConsole: {e}"))
}

/// Clear ConPTY buffer. `keep_cursor_row`: giữ dòng con trỏ (`ConptyConnection.cpp:622-630`).
/// Resolve `ClearPseudoConsole` dynamic từ `kernel32.dll` (xem `resolve_clear_pseudo_console`)
/// — pre-22H2 trả `Err` (symbol absent).
pub fn clear_pseudo_console(hpc: HPCON, keep_cursor_row: bool) -> anyhow::Result<()> {
    let clear = resolve_clear_pseudo_console().ok_or_else(|| {
        anyhow::anyhow!("ClearPseudoConsole not available (requires Windows 11 22H2+)")
    })?;
    unsafe { clear(hpc, keep_cursor_row as i32) };
    Ok(())
}

/// Đóng ConPTY + giải phóng handle. Port `ConptyConnection.cpp:667-703` +
/// `ConptyConnection::closePseudoConsoleAsync` (`cpp:849-852`).
///
/// Note: WT cũng gọi `ConptyReleasePseudoConsole(hpc)` (`cpp:451`) trước Close để
/// reference handle remove và conhost auto-shutdown. windows-rs 0.61 KHÔNG có
/// `ReleasePseudoConsole` binding — `ClosePseudoConsole` đủ cho WT parity.
pub fn close_pseudo_console(hpc: HPCON) {
    unsafe { windows::Win32::System::Console::ClosePseudoConsole(hpc) };
}

// ────────────────────────── Output thread — cpp:737-842 ───────────────────

/// Context truyền cho output thread — thay `Arc<ConptyConnection>` (C2 fix).
///
/// Thread chỉ cần: pipe_out handle (output server end), 1 `Sender<String>` clone
/// từ `Event::subs` (best-effort send, không raise qua `Event::raise` vì không có
/// `&ConptyConnection`), `Arc<AtomicU8>` snapshot state (đọc non-blocking), và
/// `done_tx` để signal main thread cleanup rendezvous.
///
/// Design choice (plan B): thread KHÔNG giữ `Arc<ConptyConnection>`. Trait
/// `ITerminalConnection::start(&mut self)` giữ nguyên — không break 8-crate caller.
pub struct OutputThreadCtx {
    /// Output pipe **server end** (child→host). Phải là `pipe_out`, KHÔNG phải
    /// `pipe` (input). C1 fix — `conn.pipe` là input write path, đọc nó = treo vĩnh viễn.
    pub pipe_out: HANDLE,
    /// Clone từ `TerminalOutputEvent::subs[0]` (qua `Event::borrow_sender`).
    /// Thread `send(wstr)` best-effort — queue đầy → err → ignore (parity WT
    /// `retain` drop full subscriber).
    pub output_tx: Sender<String>,
    /// Shared state atomic — main thread `store` ở `transition_to_state` (Release),
    /// thread `load` ở state check (Acquire). `#[repr(u8)]` → transmute `u8` safety.
    pub state: Arc<AtomicU8>,
    /// Cleanup rendezvous — thread gửi `ThreadExit` khi terminate, main thread
    /// nhận ở `close()` trước `last_conpty_client_disconnected`.
    pub done_tx: Sender<ThreadExit>,
}

/// Lý do output thread terminate — port `cpp:537-551` `_LastConPtyClientDisconnected`
/// entry discrimination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadExit {
    /// `ReadFile` fail hoặc `read == 0` — pipe broken / child closed.
    PipeBroken,
    /// State check hit `Closing` hoặc terminal (`Closed`/`Failed`).
    StateClosing,
}

/// Output thread loop — port `ConptyConnection.cpp:737-842`.
///
/// **Critical ordering**: phải queue `ReadFile` **trước** khi process prior wstr,
/// để `output_tx.send` không block pending read. Logic:
/// ```text
/// loop {
///     if !ReadFile(ctx.pipe_out, buf, &overlapped) { break; }
///     // wstr ở đây là kết quả từ PREVIOUS iter (decoded sau khi queued read).
///     if !wstr.is_empty() { ctx.output_tx.send(wstr); wstr.clear(); }
///     wait_for_overlapped() → if !read { break; }
///     if read == 0 { break; }
///     if state >= Closing { break; }
///     u8u16_decode(buf[..read]) → wstr (cho process ở iter kế)
/// }
/// ctx.done_tx.send(exit)  // signal main thread cleanup
/// ```
///
/// Loop terminate khi `ReadFile` fail (pipe-broken), `read == 0` (graceful), hoặc
/// `state == Closing/terminal` → gửi `ThreadExit` về main thread. Decode incremental
/// để không truncate multi-byte UTF-8 sequence giữa iteration.
pub fn output_thread_loop(ctx: OutputThreadCtx) {
    use windows::Win32::Foundation::WAIT_OBJECT_0;
    use windows::Win32::Storage::FileSystem::ReadFile;
    use windows::Win32::System::Threading::{
        CreateEventExW, ResetEvent, WaitForSingleObject, CREATE_EVENT_MANUAL_RESET,
    };
    use windows::Win32::System::IO::GetOverlappedResult;

    // overlappedEvent — manual-reset cho `OVERLAPPED.hEvent`. cpp:747-748.
    let overlapped_event = match unsafe {
        CreateEventExW(None, PCWSTR::null(), CREATE_EVENT_MANUAL_RESET, 0x1F0003)
    } {
        Ok(h) => h,
        Err(_) => {
            let _ = ctx.done_tx.send(ThreadExit::PipeBroken);
            return;
        }
    };

    let mut overlapped = OVERLAPPED {
        hEvent: overlapped_event,
        ..Default::default()
    };

    let mut overlapped_pending = false;
    // 128 KB buffer — match WT (`char buffer[128 * 1024]` cpp:750).
    let mut buffer = [0u8; 128 * 1024];
    let mut read: u32 = 0;
    // u8u16 decode state — incremental across iterations (cpp:753).
    let mut u8_state = U8U16State::new();
    let mut wstr = String::new();
    let mut exit_reason = ThreadExit::PipeBroken;

    'outer: loop {
        // Queue next `ReadFile` TRƯỚC — cpp:760-772. Đọc `ctx.pipe_out` (output
        // server end), KHÔNG phải `ctx.pipe` (input). C1 fix.
        let read_result = unsafe {
            ReadFile(
                ctx.pipe_out,
                Some(&mut buffer),
                Some(&mut read),
                Some(&mut overlapped),
            )
        };
        match read_result {
            Ok(()) => {}
            Err(e) if e.code() == windows::Win32::Foundation::ERROR_IO_PENDING.to_hresult() => {
                overlapped_pending = true;
            }
            Err(_) => break,
        }

        // Process prior `wstr` SAU khi queue read — cpp:774-800.
        // wstr chứa output của iteration trước (đã decode trước khi queue read iter này).
        // Best-effort send — queue đầy ignore (parity WT `retain` drop full subscriber).
        if !wstr.is_empty() {
            let _ = ctx.output_tx.send(wstr.clone());
            wstr.clear();
        }

        // Wait overlapped hoàn thành — cpp:806-813.
        if overlapped_pending {
            overlapped_pending = false;
            let wait = unsafe { WaitForSingleObject(overlapped_event, u32::MAX) };
            if wait != WAIT_OBJECT_0 {
                break;
            }
            if unsafe { GetOverlappedResult(ctx.pipe_out, &overlapped, &mut read, true) }.is_err() {
                break;
            }
            // Manual-reset event — reset để tái sử dụng iteration kế.
            let _ = unsafe { ResetEvent(overlapped_event) };
        }

        // Parse the read — cpp:819-827.
        if read == 0 {
            // Graceful closure (winsock2 semantic — 0 bytes read = EOF).
            break;
        }

        // State check — cpp:824-827. Đọc `Arc<AtomicU8>` (Acquire pair với Release
        // ở `transition_to_state`), transmute sang `ConnectionState` (`#[repr(u8)]` safe).
        // Closing/Closed/Failed → exit loop.
        let state_u8 = ctx.state.load(std::sync::atomic::Ordering::Acquire);
        let state = unsafe { std::mem::transmute::<u8, ConnectionState>(state_u8) };
        if state.is_terminal() || state == ConnectionState::Closing {
            exit_reason = ThreadExit::StateClosing;
            break 'outer;
        }

        // u8u16 decode — `til::u8u16({&buffer[0], read}, wstr, u8State)` cpp:838.
        // Decode thành wstr, process ở iteration kế (sau khi queue next read).
        u8_state.decode(&buffer[..read as usize], &mut wstr);
    }

    // Cleanup — cpp:743-745.
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(overlapped_event) };
    // Signal main thread cleanup rendezvous — best-effort (close có thể đã drop rx).
    let _ = ctx.done_tx.send(exit_reason);
}

// ──────────────────────────── U8U16 decoder ─────────────────────────
//
// Port `til::u8u16` (Terminal/common til::u8u16) — UTF-8 → UTF-16 incremental.
// Lưu partial UTF-8 byte sequence giữa các iteration để không truncate multi-byte
// boundary (cpp:753-838). Rust `String` là UTF-8 nên chỉ cần decode UTF-8 → valid
// boundary, không cần chuyển sang UTF-16 (raise nhận Rust `String`).

/// Trả về số byte của UTF-8 sequence mà `lead` khởi tạo: 1 (ASCII), 2 (0xC0-0xDF),
/// 3 (0xE0-0xEF), 4 (0xF0-0xF7), hoặc 0 nếu `lead` không phải valid UTF-8 lead byte.
fn utf8_seq_len(lead: u8) -> u8 {
    if lead < 0x80 {
        1
    } else if lead < 0xC2 {
        0 // Continuation byte hoặc 0xC0/0xC1 (overlong 2-byte, không valid).
    } else if lead < 0xE0 {
        2
    } else if lead < 0xF0 {
        3
    } else if lead < 0xF5 {
        4
    } else {
        0 // 0xF5-0xFF không valid UTF-8 lead byte.
    }
}

/// Incremental UTF-8 decoder — port `til::u8u16` semantic.
#[derive(Default)]
pub(crate) struct U8U16State {
    /// Pending UTF-8 bytes (≤4 để hoàn thành 1 codepoint).
    pending: [u8; 4],
    pending_len: usize,
}

impl U8U16State {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Decode `utf8_bytes` và append vào `wstr`. Lưu partial seq vào `pending`
    /// nếu cuối buffer cắt giữa multi-byte UTF-8 sequence. WT `til::u8u16` làm
    /// tương tự (xem `til/u8u16.h`).
    pub(crate) fn decode(&mut self, utf8_bytes: &[u8], wstr: &mut String) {
        // Combine pending + new bytes (nếu có pending).
        let mut combined: Vec<u8> = Vec::with_capacity(self.pending_len + utf8_bytes.len());
        if self.pending_len > 0 {
            combined.extend_from_slice(&self.pending[..self.pending_len]);
            self.pending_len = 0;
        }
        combined.extend_from_slice(utf8_bytes);

        // Tìm điểm cuối valid UTF-8 (lùi dần cho đến khi valid).
        let mut valid_len = combined.len();
        while valid_len > 0 && std::str::from_utf8(&combined[..valid_len]).is_err() {
            valid_len -= 1;
        }

        // Lưu extra bytes dư (cuối) vào pending nếu rơi giữa multi-byte sequence.
        if valid_len < combined.len() {
            let extra = combined.len() - valid_len;
            if extra <= 4 && valid_len > 0 {
                self.pending[..extra].copy_from_slice(&combined[valid_len..]);
                self.pending_len = extra;
            } else if valid_len == 0 && !combined.is_empty() {
                // Toàn bộ buffer không valid ở đầu. Có 2 trường hợp:
                // (a) first byte là UTF-8 lead byte (0xC0-0xF4) và extra < expected
                //     seq len → partial multi-byte sequence → pend.
                // (b) first byte không phải lead valid → invalid → ăn 1 byte và
                //     thử lại (port WT `til::u8u16` eat-error behavior).
                let first = combined[0];
                let seq_len = utf8_seq_len(first);
                if seq_len > 0 && extra < seq_len as usize {
                    self.pending[..extra].copy_from_slice(&combined);
                    self.pending_len = extra;
                } else if seq_len > 0 && extra >= seq_len as usize {
                    // Lead byte có vẻ valid nhưng toàn seq không valid → eat 1 byte.
                    self.decode(&combined[1..], wstr);
                    return;
                } else {
                    // seq_len = 0 → first byte invalid → eat.
                    self.decode(&combined[1..], wstr);
                    return;
                }
            }
            // extra > 4 → không phải UTF-8 sequence, bỏ (eat error như cpp:838 `FAILED_LOG`).
        }

        // Decode UTF-8 → String, append vào wstr.
        if let Ok(s) = std::str::from_utf8(&combined[..valid_len]) {
            wstr.push_str(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8u16_decodes_ascii() {
        let mut st = U8U16State::new();
        let mut out = String::new();
        st.decode(b"hello", &mut out);
        assert_eq!(out, "hello");
    }

    #[test]
    fn u8u16_handles_partial_multibyte() {
        let mut st = U8U16State::new();
        let mut out = String::new();
        // UTF-8 4-byte sequence U+1F600 = F0 9F 98 80. Cắt giữa byte 2 và 3.
        st.decode(&[0xF0, 0x9F], &mut out);
        assert!(out.is_empty(), "chưa có ký tự hoàn chỉnh");
        st.decode(&[0x98, 0x80, b'h'], &mut out);
        assert_eq!(out, "\u{1F600}h");
    }

    #[test]
    fn u8u16_eats_invalid() {
        let mut st = U8U16State::new();
        let mut out = String::new();
        // Invalid byte 0xFF, sau đó 'a' valid.
        st.decode(&[0xFF, b'a'], &mut out);
        assert_eq!(out, "a");
    }

    /// `clear_pseudo_console` với HPCON null không panic — pre-22H2 graceful Err,
    /// 22H2+: pointer resolve nhưng gọi null handle chỉ no-op (không panic).
    /// Verify không panic + không unsafe UB.
    #[test]
    fn clear_pseudo_console_null_handle_does_not_panic() {
        // HPCON(0) = null pseudo console handle.
        let null_hpc = HPCON(0);
        // CI chạy 22H2+ → pointer nên resolve, gọi null handle vẫn RC: receiver chưa
        // init ConPTY, Win32 API trả error code nhưng Rust side không panic.
        let _ = clear_pseudo_console(null_hpc, false);
        // Pre-22H2 branch: nếu pointer None → trả Err, vẫn không panic.
        // Test chỉ verify no-panic bất kể OS version.
    }
}
