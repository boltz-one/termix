//! DECSET/DECRST modes the control reads: bracketed paste, mouse, alt screen, autowrap.
use core::dispatch::interact_impl::TerminalInteractDispatch;
use core::dispatch::td::TerminalDispatch;
use core::parser::OutputStateMachineEngine;
use core::{TextAttribute, TextBuffer};

fn feed(
    engine: &mut OutputStateMachineEngine<TerminalDispatch>,
    parser: &mut vte::Parser,
    s: &str,
) {
    for b in s.as_bytes() {
        parser.advance(engine, *b);
    }
}

fn engine() -> (OutputStateMachineEngine<TerminalDispatch>, vte::Parser) {
    let interact: Box<dyn core::dispatch::InteractDispatch> =
        Box::new(TerminalInteractDispatch::default());
    let d = TerminalDispatch::new(TextBuffer::new(20, 5, TextAttribute::default()), interact);
    (OutputStateMachineEngine::new(d), vte::Parser::new())
}

#[test]
fn bracketed_paste_mouse_and_autowrap_flags_follow_decset() {
    let (mut e, mut p) = engine();
    feed(
        &mut e,
        &mut p,
        "\x1b[?2004h\x1b[?1000h\x1b[?1006h\x1b[?1007h\x1b[?1004h",
    );
    let d = &e.dispatch;
    assert!(d.bracketed_paste);
    assert!(d.mouse.press_release && d.mouse.sgr && d.mouse.alternate_scroll && d.mouse.focus);
    assert!(!d.mouse.button_motion && !d.mouse.any_motion);
    feed(
        &mut e,
        &mut p,
        "\x1b[?1002h\x1b[?1003h\x1b[?2004l\x1b[?1000l\x1b[?7l",
    );
    let d = &e.dispatch;
    assert!(!d.bracketed_paste && !d.autowrap);
    // WT: any tracking-mode change clears all three first (even `?1000l`).
    assert!(!d.mouse.tracking());
}

#[test]
fn alternate_screen_parks_and_restores_the_main_screen() {
    let (mut e, mut p) = engine();
    feed(&mut e, &mut p, "main text");
    feed(&mut e, &mut p, "\x1b[?1049h");
    assert!(e.dispatch.using_alt_buffer);
    assert!(
        !e.dispatch.terminal.rows()[0].contains_text(),
        "alt screen starts blank"
    );
    feed(&mut e, &mut p, "\x1b[Halt text");
    feed(&mut e, &mut p, "\x1b[?1049l");
    assert!(!e.dispatch.using_alt_buffer);
    let row0: String = e.dispatch.terminal.rows()[0]
        .glyphs()
        .iter()
        .map(|(c, _)| *c)
        .collect();
    assert!(row0.starts_with("main text"), "got {row0:?}");
    assert!(
        e.dispatch.terminal.scrollback.is_empty(),
        "alt screen produced no history"
    );
}

#[test]
fn autowrap_off_overwrites_the_last_column() {
    let (mut e, mut p) = engine();
    feed(&mut e, &mut p, "\x1b[?7l");
    feed(&mut e, &mut p, &"x".repeat(25));
    let r0: String = e.dispatch.terminal.rows()[0]
        .glyphs()
        .iter()
        .map(|(c, _)| *c)
        .collect();
    assert_eq!(r0.len(), 20);
    assert!(!e.dispatch.terminal.rows()[1].contains_text());
}

#[test]
fn conpty_combined_mouse_request_sets_both_modes() {
    // ConPTY sends `ESC[?1003;1006h` when a console app enables mouse input.
    let (mut e, mut p) = engine();
    feed(&mut e, &mut p, "[?1003;1006h");
    assert!(e.dispatch.mouse.any_motion && e.dispatch.mouse.sgr);
    feed(&mut e, &mut p, "[?1003;1006l");
    assert!(!e.dispatch.mouse.tracking() && !e.dispatch.mouse.sgr);
}
