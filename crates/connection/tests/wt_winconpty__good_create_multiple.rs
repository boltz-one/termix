//! Winconpty red-test port — `ConPtyTests::GoodCreateMultiple` (`ConPtyTests.cpp:211-236`).
//!
//! Verify 3 connections concurrent spawn thành công (không handle collision).
//! WT test direct `_CreatePseudoConsole`; Rust port quy về: spawn 3
//! `ConptyConnection` với 3 session_id khác nhau (`Uuid::new_v4` default), close
//! tất cả — không fail.

use connection::{ConptyConnection, ConptySettings, ITerminalConnection};

#[allow(non_snake_case)]
#[test]
#[ignore = "manual: needs Windows ConPTY runtime + named pipe infrastructure"]
fn winconpty__good_create_multiple_3_concurrent_succeeds() {
    let mut conns = Vec::new();
    for _ in 0..3 {
        let mut conn = ConptyConnection::new();
        let settings = ConptySettings {
            commandline: "cmd.exe /c exit 0".into(),
            initial_rows: 25,
            initial_cols: 80,
            ..Default::default()
        };
        conn.initialize(settings).unwrap();
        conn.start().expect("ConPTY spawn concurrent phải OK");
        conns.push(conn);
    }
    for mut c in conns {
        c.close().expect("close should not leak handles");
    }
}
