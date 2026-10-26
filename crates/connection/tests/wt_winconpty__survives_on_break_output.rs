//! Winconpty red-test port — `ConPtyTests::SurvivesOnBreakOutput` (`ConPtyTests.cpp:238-270`).
//!
//! Verify ConPTY process survives khi output pipe broken (host read end close).
//! WT: spawn ConPTY, attach `cmd.exe`, close `pipes.our.out` (output), chờ 2s —
//! conhost process vẫn `STILL_ACTIVE`.
//!
//! Rust port: spawn `cmd.exe` run-via-`cmd /k` để keep alive, close() connection
//! (đóng pipes), verify state transition Closed (không panic, không hang).

use connection::{ConnectionState, ConptyConnection, ConptySettings, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__survives_on_break_output_no_panic_on_close() {
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe /k echo alive".into(),
        initial_rows: 25,
        initial_cols: 80,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    conn.start().expect("spawn cmd /k phải OK");
    // Host-side close: đóng pipe (output broken). ConPTY process sẽ shutdown
    // qua conhost refcount drop, không panic.
    conn.close()
        .expect("close sau output break phải không panic");
    assert_eq!(conn.state(), ConnectionState::Closed);
}
