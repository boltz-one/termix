//! Winconpty red-test port — `ConPtyTests::DiesOnClose` (`ConPtyTests.cpp:272-327`).
//!
//! Verify ConPTY process exits (NOT `STILL_ACTIVE`) sau khi host `ClosePseudoConsole`.
//! WT: spawn ConPTY, attach command, DuplicateHandle conhost process, close
//! ConPTY, chờ 3s — exit code != `STILL_ACTIVE`.
//!
//! Rust port: spawn `cmd.exe`, close(), verify state chuyển Closed, không leak.
//! Handle leak check: `WaitForSingleObject(hConPtyProcess, 3000)` ở WT →
//! simplified, chỉ verify state machine + no panic.

use connection::{ConnectionState, ConptyConnection, ConptySettings, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__dies_on_close_state_transitions_closed() {
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe /c ping localhost".into(),
        initial_rows: 25,
        initial_cols: 80,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    conn.start().expect("spawn cmd.exe phải OK");
    conn.close().expect("close phải clean");
    // Sau close, state Closed (process exit code hoặc close path).
    assert_eq!(conn.state(), ConnectionState::Closed);
}
