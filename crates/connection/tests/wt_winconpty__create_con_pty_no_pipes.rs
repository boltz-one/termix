//! Winconpty red-test port — `ConPtyTests::CreateConPtyNoPipes` (`ConPtyTests.cpp:155-177`).
//!
//! Verify `CreatePseudoConsole` reject `INVALID_HANDLE_VALUE` cho cả 2 pipes. WT
//! dùng `_CreatePseudoConsole` direct; Rust port qua `ConptyConnection::start()`
//! pipeline tạo named pipes trước, vì vậy red-test quy về: spawn với settings
//! thiếu/invalid path → start() fail clean, không panic, không leak handle.
//!
//! Naming rule per Resolved #2: `wt_winconpty__<method>` (lower-case snake).
//! `#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]`
//! vì live spawn ConPTY cần Windows host (skip ở CI non-Windows).

use connection::{ConnectionState, ConptyConnection, ConptySettings, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__create_con_pty_no_pipes_invalid_pipe_fails_cleanly() {
    // RED stage — adapt WT `CreateConPtyNoPipes` sang Rust `ConptyConnection::start`.
    // Trong WT test, caller pass `INVALID_HANDLE_VALUE` direct cho `_CreatePseudoConsole`.
    // Rust port ta không expose pipe-handle param (ConptyConnection tạo named pipes
    // internal). Substitute: khởi tạo connection với commandline rỗng + row/col 0,
    // expect start() fail clean (ProcessArgs validation hoặc CreateProcess fail).
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: String::new(), // empty cmdline → CreateProcess fail
        initial_rows: 0,            // bad size → CreatePseudoConsole fail
        initial_cols: 0,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    let result = conn.start();
    assert!(
        result.is_err(),
        "start() với cmdline rỗng + size 0 phải fail"
    );
    // State phải Failed (không flap.Closed).
    assert_eq!(conn.state(), ConnectionState::Failed);
}
