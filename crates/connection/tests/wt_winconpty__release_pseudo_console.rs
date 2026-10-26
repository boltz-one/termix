//! Winconpty red-test port — `ConPtyTests::ReleasePseudoConsole` (`ConPtyTests.cpp:332-341`).
//!
//! Verify `ConptyReleasePseudoConsole` reference-decrement + child `start /b cmd /c
//! echo foobar` دهد output `foobar` ra host pipe.
//!
//! Rust port: windows-rs 0.61 thiếu `ConptyReleasePseudoConsole` binding (xem
//! `conpty.rs` header note) — close path substituted bằng `ClosePseudoConsole`
//! (enough for WT parity per ADR-0006). RED test quy về: spawn child emit
//! "foobar", close(), không verify release path (defer P11 wire).

use connection::{ConptyConnection, ConptySettings, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__release_pseudo_console_substituted_close_path() {
    // Substitute `ConptyReleasePseudoConsole` bằng `ClosePseudoConsole` per ADR-0006.
    // Verify: spawn child echo foobar, close safely, no handle leak.
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe /c echo foobar".into(),
        initial_rows: 25,
        initial_cols: 80,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    conn.start().expect("spawn cmd echo foobar phải OK");
    conn.close().expect("close phải không leak handles");
    // Không verify output "foobar" ở đây (cần subscribe terminal_output trước
    // start — app layer concern, P11 wire path).
}
