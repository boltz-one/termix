//! Winconpty red-test port — `ConPtyTests::CreateConPtyBadSize` (`ConPtyTests.cpp:179-192`).
//!
//! Verify `CreatePseudoConsole` reject COORD `{0,0}`, `{0,Y}`, `{X,0}` sizes.
//! WT test direct call `_CreatePseudoConsole(badSize, ...)`. Rust port quy về:
//! start() với `initial_rows=0` OR `initial_cols=0` → CreatePseudoConsole fail
//! trong `lifecycle::start()`, transition_to_state(Failed), no handle leak.

use connection::{ConptyConnection, ConptySettings, ConnectionState, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__create_con_pty_bad_size_zero_fails() {
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe".into(),
        initial_rows: 0,
        initial_cols: 0,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    let result = conn.start();
    assert!(result.is_err(), "size (0,0) phải fail CreatePseudoConsole");
    assert_eq!(conn.state(), ConnectionState::Failed);
}

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__create_con_pty_bad_size_zero_cols_fails() {
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe".into(),
        initial_rows: 30,
        initial_cols: 0,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    let result = conn.start();
    assert!(result.is_err(), "size (0 cols) phải fail CreatePseudoConsole");
    assert_eq!(conn.state(), ConnectionState::Failed);
}

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__create_con_pty_bad_size_zero_rows_fails() {
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe".into(),
        initial_rows: 0,
        initial_cols: 80,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    let result = conn.start();
    assert!(result.is_err(), "size (0 rows) phải fail CreatePseudoConsole");
    assert_eq!(conn.state(), ConnectionState::Failed);
}
