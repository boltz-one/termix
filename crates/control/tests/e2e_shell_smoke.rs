//! Headless pipeline smoke: ConPTY -> vte -> TerminalDispatch -> TextBuffer.
use std::time::{Duration, Instant};
use connection::{ConptyConnection, ConptySettings, ITerminalConnection};
use core::dispatch::interact_impl::TerminalInteractDispatch;
use core::dispatch::td::TerminalDispatch;
use core::parser::OutputStateMachineEngine;
use core::{TextAttribute, TextBuffer};

fn rows_text(tb: &TextBuffer) -> Vec<String> {
    tb.rows()
        .iter()
        .map(|r| r.glyphs().iter().map(|(c, _)| *c).collect::<String>().trim_end().to_string())
        .collect()
}

#[test]
fn conpty_echo_reaches_text_buffer() {
    let mut conn = ConptyConnection::new();
    conn.initialize(ConptySettings {
        commandline: "cmd.exe /c echo hello-from-conpty".into(),
        ..Default::default()
    })
    .unwrap();
    let rx = conn.terminal_output().subscribe();
    conn.start().unwrap();

    let interact: Box<dyn core::dispatch::InteractDispatch> = Box::new(TerminalInteractDispatch::default());
    let mut engine = OutputStateMachineEngine::new(TerminalDispatch::new(
        TextBuffer::new(80, 25, TextAttribute::default()),
        interact,
    ));
    let mut parser = vte::Parser::new();
    let end = Instant::now() + Duration::from_secs(5);
    let mut raw = String::new();
    while Instant::now() < end {
        if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(200)) {
            raw.push_str(&chunk);
            for b in chunk.as_bytes() {
                parser.advance(&mut engine, *b);
            }
            if rows_text(&engine.dispatch.terminal).iter().any(|l| l.contains("hello-from-conpty")) {
                break;
            }
        }
    }
    eprintln!("RAW: {raw:?}");
    let rows = rows_text(&engine.dispatch.terminal);
    eprintln!("ROWS: {:#?}", rows.iter().filter(|l| !l.is_empty()).collect::<Vec<_>>());
    assert!(rows.iter().any(|l| l.contains("hello-from-conpty")), "no echo in buffer");
}

fn pump(
    rx: &crossbeam_channel::Receiver<String>,
    parser: &mut vte::Parser,
    engine: &mut OutputStateMachineEngine<TerminalDispatch>,
    ms: u64,
) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(100)) {
            eprintln!("CHUNK {chunk:?}");
            for b in chunk.as_bytes() {
                parser.advance(engine, *b);
            }
        }
    }
}

#[test]
fn resize_keeps_screen_content() {
    let mut conn = ConptyConnection::new();
    conn.initialize(ConptySettings { commandline: "cmd.exe".into(), ..Default::default() }).unwrap();
    let rx = conn.terminal_output().subscribe();
    conn.start().unwrap();
    let interact: Box<dyn core::dispatch::InteractDispatch> = Box::new(TerminalInteractDispatch::default());
    let mut engine = OutputStateMachineEngine::new(TerminalDispatch::new(
        TextBuffer::new(80, 25, TextAttribute::default()),
        interact,
    ));
    let mut parser = vte::Parser::new();
    pump(&rx, &mut parser, &mut engine, 1500);
    conn.write_input(&"echo before-resize\r".encode_utf16().collect::<Vec<_>>()).unwrap();
    pump(&rx, &mut parser, &mut engine, 800);
    eprintln!("BEFORE: {:#?}", rows_text(&engine.dispatch.terminal).iter().filter(|l| !l.is_empty()).collect::<Vec<_>>());
    control::core::resize_dispatch(&mut engine.dispatch, 40, 120);
    conn.resize(40, 120).unwrap();
    pump(&rx, &mut parser, &mut engine, 1000);
    eprintln!("AFTER RESIZE: {:#?}", rows_text(&engine.dispatch.terminal).iter().enumerate().filter(|(_, l)| !l.is_empty()).collect::<Vec<_>>());
    conn.write_input(&"echo after-resize\r".encode_utf16().collect::<Vec<_>>()).unwrap();
    pump(&rx, &mut parser, &mut engine, 800);
    let rows = rows_text(&engine.dispatch.terminal);
    eprintln!("FINAL: {:#?}", rows.iter().enumerate().filter(|(_, l)| !l.is_empty()).collect::<Vec<_>>());
    assert!(rows.iter().any(|l| l.contains("before-resize")), "lost old content");
    assert!(rows.iter().any(|l| l.starts_with("after-resize")), "after-resize output misplaced");
}
