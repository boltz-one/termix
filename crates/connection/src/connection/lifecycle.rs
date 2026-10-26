//! Lifecycle methods — free functions trên `ConptyConnection`. `impl ITerminalConnection`
//! ở `mod.rs` delegate tới đây. Tách riêng cho <200 LOC per-file (workflows rule).
//!
//! Port:
//! - `start()`           — `ConptyConnection.cpp:400-478`
//! - `write_input()`     — `ConptyConnection.cpp:553-600`
//! - `resize()`          — `ConptyConnection.cpp:602-612`
//! - `close()`           — `ConptyConnection.cpp:667-703`
//! - `reset_size()`      — `ConptyConnection.cpp:614-620`
//! - `clear_buffer()`    — `ConptyConnection.cpp:622-630`
//! - `initialize()`      — `ConptyConnection.cpp:245-324`
//! - `new()`             — `ConptyConnection.cpp:197-202`
//! - `last_conpty_client_disconnected` — `ConptyConnection.cpp:537-551`
//!
//! Manual FFI được gọi qua `conpty.rs` wrappers —KHÔNG link kernel32 trực tiếp ở đây.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use crossbeam_channel::bounded;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::System::Console::COORD;
use windows::Win32::System::Threading::{
    CreateEventExW, CreateProcessW, CreateThread, DeleteProcThreadAttributeList,
    GetExitCodeProcess, InitializeProcThreadAttributeList, SetThreadDescription,
    UpdateProcThreadAttribute, WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT,
    EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW_FLAGS,
    THREAD_CREATION_FLAGS,
};

use super::settings::ConptySettings;
use super::state::ConnectionState;
use super::ConptyConnection;
use super::PseudoConsoleFlags;
use crate::conpty::{
    self, close_pseudo_console, create_overlapped_pipe, create_pseudo_console, output_thread_loop,
    resize_pseudo_console, OutputThreadCtx, ThreadExit,
};

/// Default ctor — port `ConptyConnection.cpp:197-202`. Tạo `_writeOverlappedEvent`.
///
/// `_writeOverlappedEvent` (`CreateEventExW(NULL, NULL, CREATE_EVENT_MANUAL_RESET, EVENT_ALL_ACCESS)`)
/// gắn vào `write_overlapped.hEvent` để `WriteFile` overlapped. cpp:198-201.
pub fn new_conn() -> ConptyConnection {
    let mut conn = ConptyConnection::new();
    if let Ok(ev) = unsafe {
        CreateEventExW(
            None,
            PCWSTR::null(),
            windows::Win32::System::Threading::CREATE_EVENT_MANUAL_RESET,
            0x1F0003,
        )
    } {
        conn.write_overlapped.hEvent = ev;
    }
    conn
}

/// Port `ConptyConnection.cpp:245-324` — parse 9+3 keys từ `ConptySettings`.
/// Khác WT: ta không resolve `ExpandEnvironmentStringsW` ở đây (app layer đảm nhiệm);
/// ta lưu settings, rows/cols, và tính `flags` từ bool fields (gap #6 — port
/// `ConptyConnection.cpp:262-295`).
pub fn initialize(conn: &mut ConptyConnection, settings: ConptySettings) -> anyhow::Result<()> {
    conn.settings = settings;
    conn.rows = conn.settings.initial_rows;
    conn.cols = conn.settings.initial_cols;
    // Gap #6 — port `ConptyConnection.cpp:262-295`: compute `PseudoConsoleFlags`
    // từ `inherit_cursor`/`text_measurement`/`ambiguous_is_wide` (không chỉ copy
    // `settings.flags`). Bắt đầu từ `flags` caller truyền (`0` mặc định), OR-in
    // từng bit riêng per WT Initialize body.
    let mut flags = conn.settings.flags;
    if conn.settings.inherit_cursor {
        flags |= PseudoConsoleFlags::INHERIT_CURSOR;
    }
    // WT textMeasurement dùng 3 string keys ("graphemes"/"wcswidth"/"console") →
    // map sang 3 numeric modes. `GLYPH_WIDTH_WIDE` = glyph-width flag duy nhất
    // có sẵn trong Rust enum (=0x4 — không match WT hằng số, dùng làm pass-through).
    if conn.settings.text_measurement != 0 {
        flags |= PseudoConsoleFlags::GLYPH_WIDTH_WIDE;
    }
    if conn.settings.ambiguous_is_wide {
        flags |= PseudoConsoleFlags::AMBIGUOUS_IS_WIDE;
    }
    conn.flags = flags;
    Ok(())
}

/// Port `ConptyConnection.cpp:400-478` — `ConptyCreatePseudoConsole` +
/// spawn child + output thread.
///
/// Order chính xác theo cpp:
/// 1. `transition_to_state(Connecting)` (cpp:403)
/// 2. Tạo pipes nếu chưa có (`Utils::CreateOverlappedPipe` cpp:411)
/// 3. `CreatePseudoConsole(size, pipe_in, pipe_out, flags, &h_pc)` (cpp:412)
/// 4. `_LaunchAttachedClient` (cpp:426) — `STARTUPINFOEXW` + `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`
/// 5. `ConptyReleasePseudoConsole(h_pc)` (cpp:451) — skip (windows-rs 0.61 thiếu)
/// 6. `CreateThread(output_thread_loop)` (cpp:458-471)
/// 7. `SetThreadDescription(h_thread, "ConptyConnection Output Thread")` (cpp:475)
/// 8. `transition_to_state(Connected)` (cpp:477)
///
/// Failure-path: `transition_to_state(Failed)` + `close_pseudo_console` (cpp:479-511).
pub fn start(conn: &mut ConptyConnection) -> anyhow::Result<()> {
    conn.transition_to_state(ConnectionState::Connecting)?;

    // Tạo 2 overlapped named pipes:
    // - `pipe_in` (server) — host ghi input tới PTY. Cùng handle dùng cho WriteFile.
    // - `pipe_out` (server) — host đọc output từ PTY. Cùng handle dùng cho ReadFile.
    // WT tạo 1 duplex pipe và dùng both ends với conhost. ta đơn giản hoá bằng 2 pipes
    // rời (mỗi pipe 1 hướng, named với GUID để unique).
    let session_guid = conn.settings.session_id.to_string().replace('-', "");
    let pipe_in_name = format!("\\\\.\\pipe\\tg-pty-in-{session_guid}");
    let pipe_out_name = format!("\\\\.\\pipe\\tg-pty-out-{session_guid}");

    let (pipe_in_server, pipe_in_client) = create_named_pipe_pair(&pipe_in_name)?;
    let (pipe_out_server, pipe_out_client) = create_named_pipe_pair(&pipe_out_name)?;

    // Host giữ server ends: `pipe` (đọc ở output thread, ghi ở write_input) → nhưng
    // mỗi bên cần separate handle. Ta đặt `pipe` = pipe_in_server (ghi input) cho
    // write_input path; output_thread_loop चाहिञ đọc pipe_out_server.
    // Vì `ConptyConnection` chỉ có 1 `pipe` field, ta dùng nó cho input write path.
    // Output read dùng field riêng `pipe_out` (added internal field — OK theo brief).
    conn.pipe = pipe_in_server;
    conn.pipe_out = pipe_out_server;

    // CreatePseudoConsole — cpp:412.
    // PTY input = pipe_in_client (child reads), PTY output = pipe_out_client (child writes).
    let size = COORD {
        X: conn.cols as i16,
        Y: conn.rows as i16,
    };
    let flags = conn.flags.bits();
    match create_pseudo_console(size, pipe_in_client, pipe_out_client, flags) {
        Ok(hpc) => conn.h_pc = hpc,
        Err(e) => {
            let _ = unsafe { CloseHandle(pipe_in_server) };
            let _ = unsafe { CloseHandle(pipe_in_client) };
            let _ = unsafe { CloseHandle(pipe_out_server) };
            let _ = unsafe { CloseHandle(pipe_out_client) };
            // Gap #4 — failure sub-msg trước transition (port `ConptyConnection.cpp:485-507`).
            // WT format `RS_fmt(L"ProcessFailedToLaunch", ...)` — ta hardcode English per
            // Resolved note #5. Raise text ra `TerminalOutput` cho UI render error.
            conn.terminal_output
                .raise(format!("[failed to launch: {e}]\r\n"));
            conn.transition_to_state(ConnectionState::Failed)?;
            anyhow::bail!("CreatePseudoConsole: {e}");
        }
    }
    // _LaunchAttachedClient — cpp:32-193.
    if let Err(e) = launch_attached_client(conn) {
        close_pseudo_console(conn.h_pc);
        conn.h_pc = hpcon_default_placeholder();
        let _ = unsafe { CloseHandle(pipe_in_server) };
        let _ = unsafe { CloseHandle(pipe_out_server) };
        // Gap #4 — failure sub-msg trước transition.
        conn.terminal_output
            .raise(format!("[failed to launch: {e}]\r\n"));
        conn.transition_to_state(ConnectionState::Failed)?;
        anyhow::bail!("LaunchAttachedClient: {e}");
    }

    // Spawn output thread — cpp:458-471.
    // Design choice (plan B — C2 fix): thread KHÔNG giữ `Arc<ConptyConnection>`.
    // Truyền riêng các phần thread cần vào `OutputThreadCtx` (owned by-value):
    // - `pipe_out`: output server end (read path), KHÔNG phải `pipe` (input).
    // - `output_tx`: clone từ `Event::subs[0]` qua `Event::borrow_sender`.
    // - `state`: `Arc<AtomicU8>` shared với main thread (sync ở `transition_to_state`).
    // - `done_tx`: cleanup rendezvous — thread signal `ThreadExit`, main thread
    //   nhận ở `close()` trước `last_conpty_client_disconnected`.
    //
    // `borrow_sender` panic nếu chưa có subscriber — `ControlCore::initialize`
    // (subscribe `terminal_output`) phải chạy trước `start()` (verify order
    // `control/src/control.rs:60` → `:90`).
    let output_tx = conn.terminal_output.borrow_sender();
    let state_atomic = conn.shared_state.clone();
    let (done_tx, done_rx) = bounded::<ThreadExit>(1);
    conn.done_rx = Some(done_rx);

    // Set startTime cho "ReceivedFirstByte" telemetry (skip telemetry ở phase 2).
    conn._start_time = Some(std::time::Instant::now());

    // Box ctx → raw ptr truyền cho `CreateThread`. Thread take ownership qua
    // `Box::from_raw` ở trampoline. Heap-alloc vì ctx lớn (128KB buffer không
    // ở đây — buffer ở thread stack; ctx chỉ ~64 bytes).
    let ctx = OutputThreadCtx {
        pipe_out: conn.pipe_out,
        output_tx,
        state: state_atomic,
        done_tx,
    };
    let ctx_ptr = Box::into_raw(Box::new(ctx));

    let h_thread = unsafe {
        CreateThread(
            None,
            0,
            Some(thread_start_trampoline),
            Some(ctx_ptr as *mut std::ffi::c_void),
            THREAD_CREATION_FLAGS::default(),
            None,
        )
    };
    match h_thread {
        Ok(h) => {
            conn.h_output_thread = h;
            // SetThreadDescription — cpp:475.
            let desc: Vec<u16> = OsStr::new("ConptyConnection Output Thread")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let _ = unsafe { SetThreadDescription(h, PCWSTR(desc.as_ptr())) };
        }
        Err(e) => {
            close_pseudo_console(conn.h_pc);
            conn.h_pc = hpcon_default_placeholder();
            let _ = unsafe { CloseHandle(conn.pipe) };
            let _ = unsafe { CloseHandle(conn.pipe_out) };
            conn.pipe = INVALID_HANDLE_VALUE;
            conn.pipe_out = INVALID_HANDLE_VALUE;
            // Gap #4 — failure sub-msg trước transition.
            conn.terminal_output
                .raise(format!("[failed to launch: {e}]\r\n"));
            conn.transition_to_state(ConnectionState::Failed)?;
            anyhow::bail!("CreateThread: {e}");
        }
    }

    // transition_to_state(Connected) — cpp:477.
    conn.transition_to_state(ConnectionState::Connected)?;
    spawn_exit_watcher(conn);
    Ok(())
}

/// Trampoline cho `CreateThread` — nhận `*mut OutputThreadCtx` (owned, heap-alloc
/// qua `Box::into_raw` ở `start`) và dispatch đến `output_thread_loop`. Lấy lại
/// ownership qua `Box::from_raw` (drop ctx khi `output_thread_loop` return).
/// cpp:461-467.
///
/// Safety: `lp_parameter` phải là `Box::into_raw(Box::new(ctx))` raw pointer từ
/// `start()`. Ta take ownership + drop — không leak, không double-free.
unsafe extern "system" fn thread_start_trampoline(lp_parameter: *mut std::ffi::c_void) -> u32 {
    let ctx = Box::from_raw(lp_parameter as *mut OutputThreadCtx);
    output_thread_loop(*ctx);
    0
}

/// Port `ConptyConnection.cpp:_LaunchAttachedClient` (cpp:32-193).
///
/// `STARTUPINFOEXW` + `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` + `CreateProcessW`.
/// Simplification so với WT:
/// - Skip `ExpandEnvironmentStringsW` — caller resolve trước (brief).
/// - Skip `til::env` map / `WT_SESSION`/`WT_PROFILE_ID` env injection — app layer.
/// - Skip `Utils::MangleStartingDirectoryForWSL` — caller xử lý.
/// - Skip `TraceLoggingWrite` telemetry.
/// - Skip `GetModuleFileNameExW` client name extraction (telemetry).
fn launch_attached_client(conn: &mut ConptyConnection) -> anyhow::Result<()> {
    // STARTUPINFOEXW init — cpp:34-44.
    let mut si_ex: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si_ex.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    // cpp:36 — `STARTF_USESTDHANDLES` không thực sự cần cho PTY (process không thấy std handles),
    // WT giữ cho parity với legacy. ta không set.
    si_ex.StartupInfo.dwFlags = STARTUPINFOW_FLAGS(STARTF_USESTDHANDLES.0);

    // InitializeProcThreadAttributeList — cpp:38-44.
    let mut attr_size: usize = 0;
    unsafe {
        let _ = InitializeProcThreadAttributeList(None, 1, Some(0), &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    si_ex.lpAttributeList = LPPROC_THREAD_ATTRIBUTE_LIST(attr_buf.as_mut_ptr() as *mut _);
    unsafe {
        InitializeProcThreadAttributeList(Some(si_ex.lpAttributeList), 1, Some(0), &mut attr_size)
            .map_err(|e| anyhow::anyhow!("InitializeProcThreadAttributeList: {e}"))?;
    }

    // UpdateProcThreadAttribute — cpp:46-53.
    // `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` = `h_pc`. sizeof(HPCON) là `isize` (8 trên x64).
    unsafe {
        UpdateProcThreadAttribute(
            si_ex.lpAttributeList,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(conn.h_pc.0 as *const std::ffi::c_void),
            std::mem::size_of::<HpconInner>(),
            None,
            None,
        )
        .map_err(|e| anyhow::anyhow!("UpdateProcThreadAttribute: {e}"))?;
    }

    // UTF-16 buffers giữ sống qua `CreateProcessW` call (lives-through-function).
    let mut cmdline: Vec<u16> = OsStr::new(&conn.settings.commandline)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Optional starting directory — phải sống đến sau CreateProcessW.
    let cwd_buf: Vec<u16> = if conn.settings.starting_directory.is_empty() {
        Vec::new()
    } else {
        OsStr::new(&conn.settings.starting_directory)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let cwd_ptr: PCWSTR = if cwd_buf.is_empty() {
        PCWSTR::null()
    } else {
        PCWSTR(cwd_buf.as_ptr())
    };

    // Optional starting title — cpp:156-161. Cũng phải sống đến sau CreateProcessW.
    let title_buf: Vec<u16> = match &conn.settings.starting_title {
        Some(title) => OsStr::new(title)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect(),
        None => Vec::new(),
    };
    if !title_buf.is_empty() {
        si_ex.StartupInfo.lpTitle = windows::core::PWSTR(title_buf.as_ptr() as *mut _);
    }

    // Gap #7 — env block injection (port `ConptyConnection.cpp:56-152`). Build UTF-16
    // `KEY=VALUE\0...\0\0` block. Nếu `reload_env`: start từ `std::env::vars()` (current
    // process env) + overlay `settings.environment` (caller overrides). Auto-add
    // `WT_SESSION={session_id}` và `WT_PROFILE_ID={profile_guid}` per WT cpp:60-64.
    // `CREATE_UNICODE_ENVIRONMENT` đã set ở line bên dưới (cpp:172).
    let env_buf = build_env_block(&conn.settings);
    let env_ptr: *const u16 = if env_buf.is_empty() {
        std::ptr::null()
    } else {
        env_buf.as_ptr()
    };

    // CreateProcessW — cpp:166-177. `lpcommandline` yêu cầu `Option<PWSTR>` (mutable wchar_t*).
    let mut pi = PROCESS_INFORMATION::default();
    let result = unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(windows::core::PWSTR(cmdline.as_mut_ptr())),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            if env_ptr.is_null() {
                None
            } else {
                Some(env_ptr as *const std::ffi::c_void)
            },
            cwd_ptr,
            &si_ex.StartupInfo,
            &mut pi,
        )
    };

    // DeleteProcThreadAttributeList — cpp:179. Gọi trước khi return (cả success/fail).
    unsafe { DeleteProcThreadAttributeList(si_ex.lpAttributeList) };

    // Pin buffers cho đến đây để không drop sớm.
    let _ = (&cmdline, &cwd_buf, &title_buf, &env_buf);

    match result {
        Ok(()) => {
            conn.pi_client = pi;
            Ok(())
        }
        Err(e) => anyhow::bail!("CreateProcessW: {e}"),
    }
}

/// Tạo cặp (server, client) named pipe — wrapper round `conpty::create_overlapped_pipe`
/// để debug error context. Trả server (host giữ, overlapped I/O) + client (inherit
/// vào child, sync I/O).
fn create_named_pipe_pair(name: &str) -> anyhow::Result<(HANDLE, HANDLE)> {
    let name_w: Vec<u16> = OsStr::new(name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    create_overlapped_pipe(PCWSTR(name_w.as_ptr()))
        .map_err(|e| anyhow::anyhow!("pipe({name}): {e}"))
}

/// Port `ConptyConnection.cpp:553-600` — UTF-16→UTF-8, overlapped WriteFile.
///
/// Order:
/// 1. `state == Connected` check (cpp:557 — `_isConnected`)
/// 2. `write_lock` ticket (cpp:564)
/// 3. Nếu `write_pending`: `GetOverlappedResult(pipe, &write_overlapped, &read, TRUE)`
///    (cpp:566-578); failure → `close_pseudo_console` + return.
/// 4. `til::u16u8(data, write_buffer)` (cpp:580) — UTF-16 → UTF-8.
/// 5. `WriteFile(pipe, write_buffer, ..., &write_overlapped)` (cpp:585)
///    - `ERROR_BROKEN_PIPE` → `close_pseudo_console` (cpp:590)
///    - `ERROR_IO_PENDING` → `write_pending = true` (cpp:593)
///    - default → log error (skip log, just continue)
pub fn write_input(conn: &mut ConptyConnection, buffer: &[u16]) -> anyhow::Result<()> {
    if conn.state != ConnectionState::Connected {
        return Ok(()); // cpp:557-560 — silent return nếu chưa Connected.
    }
    let _guard = conn.write_lock.lock();

    // GetOverlappedResult cho pending write trước — cpp:566-578.
    if conn.write_pending {
        conn.write_pending = false;
        let mut read: u32 = 0;
        let ok = unsafe {
            windows::Win32::System::IO::GetOverlappedResult(
                conn.pipe,
                &conn.write_overlapped,
                &mut read,
                true,
            )
        };
        if ok.is_err() {
            // Pipe broken hoặc IO fail — kill connection.
            close_pseudo_console(conn.h_pc);
            conn.h_pc = hpcon_default_placeholder();
            return Ok(());
        }
    }

    // UTF-16 → UTF-8 — `til::u16u8(data, _writeBuffer)` (cpp:580).
    conn.write_buffer = utf16_to_utf8(buffer)?;
    if conn.write_buffer.is_empty() {
        return Ok(());
    }

    // WriteFile overlapped — cpp:585-599.
    let result = unsafe {
        windows::Win32::Storage::FileSystem::WriteFile(
            conn.pipe,
            Some(&conn.write_buffer),
            None,
            Some(&mut conn.write_overlapped),
        )
    };
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let code = e.code();
            if code == windows::Win32::Foundation::ERROR_BROKEN_PIPE.to_hresult() {
                // cpp:590 — pipe broken, kill connection.
                close_pseudo_console(conn.h_pc);
                conn.h_pc = hpcon_default_placeholder();
                Ok(())
            } else if code == windows::Win32::Foundation::ERROR_IO_PENDING.to_hresult() {
                // cpp:593 — overlapped pending, mark for next write to await.
                conn.write_pending = true;
                Ok(())
            } else {
                // cpp:596 — log error, continue (drop data). ta không log.
                Ok(())
            }
        }
    }
}

/// Report the client exiting on its own (`_LastConPtyClientDisconnected`
/// trigger): the output pipe stays open until `ClosePseudoConsole`, so WT
/// waits on the client process handle instead. A watcher thread waits on a
/// duplicate of it, then raises `Closed` (exit code 0) or prints the exit
/// message and raises `Failed`.
fn spawn_exit_watcher(conn: &ConptyConnection) {
    use windows::Win32::Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS};
    use windows::Win32::System::Threading::{GetCurrentProcess, INFINITE};

    let mut dup = HANDLE::default();
    let cur = unsafe { GetCurrentProcess() };
    let ok = unsafe {
        DuplicateHandle(cur, conn.pi_client.hProcess, cur, &mut dup, 0, false, DUPLICATE_SAME_ACCESS)
    };
    if ok.is_err() {
        return;
    }
    let raw = dup.0 as usize;
    let state_tx = conn.state_changed.try_borrow_sender();
    let output_tx = conn.terminal_output.try_borrow_sender();
    let _ = std::thread::Builder::new().name("tg-client-wait".into()).spawn(move || {
        const STILL_ACTIVE: u32 = 259;
        let h = HANDLE(raw as *mut _);
        unsafe { WaitForSingleObject(h, INFINITE) };
        let mut code = 0u32;
        let _ = unsafe { GetExitCodeProcess(h, &mut code) };
        let _ = unsafe { CloseHandle(h) };
        let failed = code != 0 && code != STILL_ACTIVE;
        // ConPTY repaints its own screen right after the client exits and would
        // erase the message; let that repaint pass first.
        std::thread::sleep(std::time::Duration::from_millis(400));
        if failed {
            if let Some(tx) = &output_tx {
                let _ = tx.send(process_exit_message(code));
            }
        }
        if let Some(tx) = &state_tx {
            let _ = tx.send(if failed { ConnectionState::Failed } else { ConnectionState::Closed });
        }
    });
}

/// Port `ConptyConnection.cpp:602-612` — store rows/cols, resize ConPTY nếu Connected.
/// `ResetSize` (`cpp:614-620`) — wrapper gọi hàm này vứt `rows/cols` từ `Terminal`'s `_viewport`.
pub fn resize(conn: &mut ConptyConnection, rows: u32, cols: u32) -> anyhow::Result<()> {
    // Always keep these in case we ever want to disconnect/restart — cpp:605-606.
    conn.rows = rows;
    conn.cols = cols;

    if conn.state == ConnectionState::Connected {
        // ClampToShortMax — cpp:610.
        let clamped = COORD {
            X: clamp_to_short_max(cols, 1) as i16,
            Y: clamp_to_short_max(rows, 1) as i16,
        };
        resize_pseudo_console(conn.h_pc, clamped)?;
    }
    Ok(())
}

/// Port `ConptyConnection.cpp:614-620` — store-only, không gọi ConPTY.
pub fn reset_size(conn: &mut ConptyConnection, rows: u32, cols: u32) {
    conn.rows = rows;
    conn.cols = cols;
}

/// Port `ConptyConnection.cpp:622-630` — clear ConPTY buffer, giữ dòng con trỏ.
pub fn clear_buffer(conn: &ConptyConnection, keep_cursor_row: bool) -> anyhow::Result<()> {
    if conn.state != ConnectionState::Connected {
        return Ok(()); // cpp:626 — không cần clear nếu chưa Connected.
    }
    conpty::clear_pseudo_console(conn.h_pc, keep_cursor_row)
}

/// Port `ConptyConnection.cpp:667-703` — state→Closing, reset `_hPC`, cancel thread, Close.
///
/// Order chính xác:
/// 1. `transition_to_state(Closing)` (cpp:670)
/// 2. `ClosePseudoConsole(h_pc)` (cpp:674 — `_hPC.reset()`)
/// 3. Loop `CancelIoEx(pipe) + WaitForSingleObject(h_output_thread, 1000)` (cpp:679-693)
/// 4. `CloseHandle(h_output_thread)` (cpp:695)
/// 5. `piClient.reset()` → CloseHandle process + thread (cpp:696)
/// 6. `pipe.reset()` (cpp:697)
/// 7. `transition_to_state(Closed)` (cpp:701)
pub fn close(conn: &mut ConptyConnection) -> anyhow::Result<()> {
    conn.transition_to_state(ConnectionState::Closing)?;

    // ClosePseudoConsole — cpp:674.
    if conn.h_pc != hpcon_default_placeholder() {
        close_pseudo_console(conn.h_pc);
        conn.h_pc = hpcon_default_placeholder();
    }

    // CancelIoEx + WaitForSingleObject loop — cpp:679-693.
    // C3 fix: cancel `pipe_out` (output server end — thread đang chờ ReadFile
    // trên pipe_out), KHÔNG phải `pipe` (input). Cancel sai pipe → thread không
    // unblock → WaitForSingleObject loop treo vĩnh viễn.
    if conn.h_output_thread != INVALID_HANDLE_VALUE {
        loop {
            let _ = unsafe { windows::Win32::System::IO::CancelIoEx(conn.pipe_out, None) };
            let result = unsafe { WaitForSingleObject(conn.h_output_thread, 1000) };
            if result == WAIT_OBJECT_0 {
                break;
            }
        }
        let _ = unsafe { CloseHandle(conn.h_output_thread) };
        conn.h_output_thread = INVALID_HANDLE_VALUE;

        // Drain cleanup rendezvous — output thread gửi `ThreadExit` khi join.
        // Discard reason (PipeBroken/StateClosing) — không phân nhánh cleanup.
        if let Some(rx) = conn.done_rx.take() {
            let _ = rx.recv();
        }
        // `_LastConPtyClientDisconnected` — port `cpp:537-551`. Chạy ở main thread
        // sau khi output thread đã join (no alias). Đọc `pi_client.hProcess` exit
        // code + transition `Closed`/`Failed`. Non-arc variant — `&mut self` đủ.
        last_conpty_client_disconnected(conn);
    }

    // CloseHandle PROCESS_INFORMATION (hProcess + hThread) — cpp:696 (`piClient.reset()`).
    if conn.pi_client.hProcess != INVALID_HANDLE_VALUE {
        let _ = unsafe { CloseHandle(conn.pi_client.hProcess) };
        conn.pi_client.hProcess = INVALID_HANDLE_VALUE;
    }
    if conn.pi_client.hThread != INVALID_HANDLE_VALUE {
        let _ = unsafe { CloseHandle(conn.pi_client.hThread) };
        conn.pi_client.hThread = INVALID_HANDLE_VALUE;
    }

    // CloseHandle pipes — cpp:697 (`_pipe.reset()`).
    if conn.pipe != INVALID_HANDLE_VALUE {
        let _ = unsafe { CloseHandle(conn.pipe) };
        conn.pipe = INVALID_HANDLE_VALUE;
    }
    if conn.pipe_out != INVALID_HANDLE_VALUE {
        let _ = unsafe { CloseHandle(conn.pipe_out) };
        conn.pipe_out = INVALID_HANDLE_VALUE;
    }

    // transition_to_state(Closed) — cpp:701.
    // Output thread có thể đã chuyển → Closing/Closed, guard ở `transition_to_state`
    // chấp nhận `Closing → Closed`.
    if conn.state != ConnectionState::Closed {
        conn.transition_to_state(ConnectionState::Closed)?;
    }
    Ok(())
}

/// Port `ConptyConnection.cpp:537-551` — output thread terminate entry.
///
/// `GetExitCodeProcess` + transition `Closed` (success/STILL_ACTIVE) hoặc `Failed` (non-zero).
/// Guard chống flapping: WT (`cpp:543`) `piClient.reset()` mọi case. ta cũng close handle.
///
/// Được gọi từ `close()` sau khi output thread đã join (main thread, `&mut self`).
/// Plan B — không cần `Arc<ConptyConnection>` variant nữa.
pub(crate) fn last_conpty_client_disconnected(conn: &mut ConptyConnection) {
    let exit_code = if conn.pi_client.hProcess != INVALID_HANDLE_VALUE {
        let mut code: u32 = 0;
        let _ = unsafe { GetExitCodeProcess(conn.pi_client.hProcess, &mut code) };
        // CloseHandle process + thread handles (piClient.reset() — cpp:543).
        let _ = unsafe { CloseHandle(conn.pi_client.hProcess) };
        let _ = unsafe { CloseHandle(conn.pi_client.hThread) };
        conn.pi_client.hProcess = INVALID_HANDLE_VALUE;
        conn.pi_client.hThread = INVALID_HANDLE_VALUE;
        code
    } else {
        0
    };

    // transition_to_state: Closed nếu exit_code == 0 hoặc STILL_ACTIVE, ngược lại Failed.
    // STILL_ACTIVE = 259 (0x103) — `get_strong` client có thể FreeConsole mà process còn sống.
    const STILL_ACTIVE: u32 = 259;
    let new_state = if exit_code == 0 || exit_code == STILL_ACTIVE {
        ConnectionState::Closed
    } else {
        ConnectionState::Failed
    };
    // Gap #3 — port `ConptyConnection.cpp:549` (`_indicateExitWithStatus`):
    // raise "\r\n[process exited with code {N}]\r\n[press Ctrl+D to close]\r\n"
    // CHỈ khi exit_code != 0 AND != STILL_ACTIVE. Raise trước `transition_to_state`
    // để UI thấy message ngay cả nếu guard reject transition (Closing/Closed đã set).
    if exit_code != 0 && exit_code != STILL_ACTIVE {
        conn.terminal_output.raise(process_exit_message(exit_code));
    }
    // Guard: nếu user `close()` đã transitioned sang `Closed`, không flap ngược lại `Failed`.
    if conn.state == ConnectionState::Closing || conn.state == ConnectionState::Connected {
        let _ = conn.transition_to_state(new_state);
    }
}

// ─────────────────────────── Small helpers ─────────────────────────

/// UTF-16 → UTF-8 — port `til::u16u8` (cpp:580). Std `String::from_utf16` đủ đúng.
fn utf16_to_utf8(buffer: &[u16]) -> anyhow::Result<Vec<u8>> {
    let s = String::from_utf16(buffer).map_err(|e| anyhow::anyhow!("u16u8: {e}"))?;
    Ok(s.into_bytes())
}

/// Gap #7 — Build UTF-16 environment block cho `CreateProcessW` (`CREATE_UNICODE_ENVIRONMENT`).
/// Port `ConptyConnection.cpp:56-152`. Trả `Vec<u16>` con `KEY=VALUE\0...\0\0` rỗng ↔ env empty.
///
/// Order per WT cpp:
/// 1. Nếu `settings.reload_env`: clone `std::env::vars()` làm base. Else: empty.
/// 2. Overlay `settings.environment` (caller overrides).
/// 3. Auto add `WT_SESSION={session_id}` (cpp:60) — luôn override.
/// 4. Auto add `WT_PROFILE_ID={profile_guid}` (cpp:64) nếu `Some`.
///
/// Degenerate case: env empty AND `reload_env == false` → return empty Vec →
/// caller truyền `None` cho `lpEnvironment` (Win32 inherit nothing).
pub(crate) fn build_env_block(settings: &super::settings::ConptySettings) -> Vec<u16> {
    use std::collections::BTreeMap;

    // BTreeMap deterministic order — không quan trọng cho Win32 nhưng stable cho test.
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    if settings.reload_env {
        for (k, v) in std::env::vars() {
            map.insert(k, v);
        }
        // Windows PowerShell 5.1 must not inherit PowerShell 7 module paths from the
        // parent shell: it would load PS7's PSReadLine ("Cannot load PSReadline module").
        if settings.commandline.to_ascii_lowercase().contains("windowspowershell") {
            if let Some(v) = map.get_mut("PSModulePath") {
                *v = v
                    .split(';')
                    .filter(|p| p.to_ascii_lowercase().contains("windowspowershell"))
                    .collect::<Vec<_>>()
                    .join(";");
            }
        }
    }
    for (k, v) in &settings.environment {
        map.insert(k.clone(), v.clone());
    }
    // WT_SESSION — cpp:60. Plain guid string (no braces) per `Utils::GuidToPlainString`.
    map.insert("WT_SESSION".into(), settings.session_id.to_string());
    // WT_PROFILE_ID — cpp:64. Guid with braces per `Utils::GuidToString` (Rust Uuid Display
    // không có braces → wrap thủ công).
    if let Some(profile) = settings.profile_guid {
        map.insert("WT_PROFILE_ID".into(), format!("{{{profile}}}"));
    }

    let mut buf: Vec<u16> = Vec::new();
    for (k, v) in &map {
        let entry = format!("{k}={v}");
        buf.extend(OsStr::new(&entry).encode_wide());
        buf.push(0); // NUL terminator per entry.
    }
    if !buf.is_empty() {
        buf.push(0); // Final double-NUL.
    }
    buf
}

/// Gap #3 — Process-exit message format — port `ConptyConnection.cpp:517-528`
/// (`_indicateExitWithStatus`). Pure fn cho test: format message từ exit_code.
///
/// WT dùng resource strings `ProcessExited` + `CtrlDToClose`. Ta hardcode English
/// verbatim per Resolved note #5: `"\r\n[process exited with code {N}]\r\n[press Ctrl+D to close]\r\n"`.
pub(crate) fn process_exit_message(exit_code: u32) -> String {
    format!(
        "\r\n[process exited with code {exit_code}]\r\n[press Ctrl+D to close]\r\n"
    )
}

/// `Utils::ClampToShortMax(value, min)` — clamp vào `i16` range (cpp:610).
fn clamp_to_short_max(value: u32, min: u32) -> u32 {
    value.max(min).min(i16::MAX as u32)
}

/// Placeholder `HPCON` cho "invalid" state — `HPCON::default()` (isize=0).
/// WT dùng `wil::unique_hpc` (detached) — ta chỉ check `!= default()`.
fn hpcon_default_placeholder() -> windows::Win32::System::Console::HPCON {
    windows::Win32::System::Console::HPCON::default()
}

/// `HPCON` inner type — `isize` (windows-rs 0.61 `pub struct HPCON(pub isize)`).
type HpconInner = isize;

#[cfg(test)]
mod tests {
    use super::super::ITerminalConnection;
    use super::*;

    #[test]
    fn new_starts_not_connected() {
        let c = new_conn();
        assert_eq!(c.state(), ConnectionState::NotConnected);
        assert_eq!(c.session_id(), c.settings.session_id);
    }

    #[test]
    fn initialize_stores_settings() {
        let mut c = new_conn();
        let s = ConptySettings {
            initial_rows: 10,
            initial_cols: 40,
            ..Default::default()
        };
        initialize(&mut c, s.clone()).unwrap();
        assert_eq!((c.rows, c.cols), (10, 40));
        assert_eq!(c.flags, s.flags);
    }

    #[test]
    fn reset_size_store_only() {
        let mut c = new_conn();
        reset_size(&mut c, 30, 100);
        assert_eq!((c.rows, c.cols), (30, 100));
    }

    #[test]
    fn clamp_to_short_max_clamps_high() {
        assert_eq!(clamp_to_short_max(50_000, 1), i16::MAX as u32);
        assert_eq!(clamp_to_short_max(0, 1), 1);
        assert_eq!(clamp_to_short_max(80, 1), 80);
    }

    #[test]
    fn utf16_to_utf8_roundtrip_ascii() {
        let input: Vec<u16> = "hello".encode_utf16().collect();
        let out = utf16_to_utf8(&input).unwrap();
        assert_eq!(out, b"hello");
    }

    #[test]
    fn utf16_to_utf8_roundtrip_emoji() {
        let input: Vec<u16> = "hi \u{1F600}".encode_utf16().collect();
        let out = utf16_to_utf8(&input).unwrap();
        assert_eq!(out, "hi \u{1F600}".as_bytes());
    }

    #[test]
    fn write_input_returns_silent_when_not_connected() {
        let mut c = new_conn();
        let buf: Vec<u16> = "ls".encode_utf16().collect();
        write_input(&mut c, &buf).unwrap();
        // State still NotConnected — silent return.
        assert_eq!(c.state, ConnectionState::NotConnected);
    }

    #[test]
    fn resize_stores_only_when_not_connected() {
        let mut c = new_conn();
        resize(&mut c, 50, 200).unwrap();
        assert_eq!((c.rows, c.cols), (50, 200));
    }

    #[test]
    fn close_transitions_to_closed_from_notconnected_fails() {
        // Close yêu cầu Connecting/Connected/Closing — NotConnected → Closing là refused
        // mặc dù brief spec cho phép NotConnected → Connecting/Failed ở phase 1.
        // Close() từ NotConnected phải return error.
        let mut c = new_conn();
        let r = close(&mut c);
        assert!(r.is_err(), "close() từ NotConnected phải bị reject");
    }

    #[test]
    fn close_succeeds_after_connecting() {
        let mut c = new_conn();
        c.transition_to_state(ConnectionState::Connecting).unwrap();
        close(&mut c).unwrap();
        assert_eq!(c.state, ConnectionState::Closed);
    }
}
