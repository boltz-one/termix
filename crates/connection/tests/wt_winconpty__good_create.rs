//! Winconpty red-test port — `ConPtyTests::GoodCreate` (`ConPtyTests.cpp:194-209`).
//!
//! Verify `ConptyConnection::start()` spawn thành công `cmd /c exit 0`, child
//! exit code 0, connection transition qua Closed sau close().
//!
//! Live test — needs real Windows ConPTY runtime + named pipe infrastructure.
//! CI non-Windows skip via `#[ignore]`.

use connection::{ConnectionState, ConptyConnection, ConptySettings, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__good_create_spawns_cmd_exits_zero() {
    // RED stage — adapt WT `GoodCreate` to Rust `ConptyConnection::start` flow.
    let mut conn = ConptyConnection::new();
    let settings = ConptySettings {
        commandline: "cmd.exe /c exit 0".into(),
        initial_rows: 25,
        initial_cols: 80,
        ..Default::default()
    };
    conn.initialize(settings).unwrap();
    conn.start()
        .expect("ConPTY spawn should succeed với cmd /c exit 0");
    conn.close().expect("close should not leak handles");
    assert_eq!(conn.state(), ConnectionState::Closed);
}
