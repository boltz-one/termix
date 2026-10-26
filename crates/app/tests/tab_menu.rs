//! Tab context-menu actions, drag reorder and tear-out of `TerminalPage`.
use app::null_render_engine::NullRenderEngine;
use app::settings_loader::CascadiaSettings;
use app::tab::Tab;
use app::TerminalPage;
use connection::ConptyConnection;
use control::{KeyMapping, TermControl};
use core::{TextAttribute, TextBuffer};
use gpui::{AppContext, Entity, TestAppContext};
use settings::{GlobalAppSettings, Profile};

fn settings() -> CascadiaSettings {
    CascadiaSettings {
        profiles: vec![Profile::default_powershell()],
        default_profile_guid: uuid::Uuid::parse_str("61c54bbd-c2c6-5271-96e7-009a87ff44bf")
            .unwrap(),
        globals: GlobalAppSettings::default(),
    }
}

fn tab(cx: &mut gpui::App) -> Entity<Tab> {
    let control = cx.new(|cx| {
        TermControl::new(
            Box::new(ConptyConnection::new()),
            Box::new(NullRenderEngine),
            TextBuffer::new(80, 25, TextAttribute::default()),
            Profile::default_powershell(),
            KeyMapping::default(),
            cx,
        )
    });
    cx.new(|cx| Tab::new(control, cx))
}

/// Window holding a page with `n` tabs (none of them runs a shell).
fn page_with(cx: &mut TestAppContext, n: usize) -> gpui::WindowHandle<TerminalPage> {
    cx.add_window(|_, cx| {
        let mut page = TerminalPage::with_tab(settings(), tab(cx));
        for _ in 1..n {
            let t = tab(cx);
            page.push_tab(t);
        }
        page
    })
}

#[gpui::test]
fn close_other_and_close_right(cx: &mut TestAppContext) {
    let w = page_with(cx, 4);
    w.update(cx, |p, window, cx| {
        p.tab_menu_action(1, 2, window, cx); // close tabs to the right of tab 1
        assert_eq!(p.tabs().len(), 2);
        assert_eq!(p.active_tab_index(), Some(1));
        p.tab_menu_action(0, 1, window, cx); // close other tabs
        assert_eq!(p.tabs().len(), 1);
        assert_eq!(p.active_tab_index(), Some(0));
    })
    .unwrap();
}

#[gpui::test]
fn drag_reorder_keeps_selection_on_the_moved_tab(cx: &mut TestAppContext) {
    let w = page_with(cx, 3);
    w.update(cx, |p, _, cx| {
        let first = p.tabs()[0].clone();
        p.handle_select_tab_action(0, cx);
        p.move_tab(0, 2, cx);
        assert_eq!(p.tabs()[2], first);
        assert_eq!(p.active_tab_index(), Some(2));
    })
    .unwrap();
}

#[gpui::test]
fn tear_out_moves_the_tab_to_a_new_window(cx: &mut TestAppContext) {
    let w = page_with(cx, 2);
    assert_eq!(cx.update(|cx| cx.windows().len()), 1);
    w.update(cx, |p, window, cx| {
        let moved = p.tabs()[1].clone();
        p.tear_out_tab(1, window.bounds().origin, window, cx);
        assert_eq!(p.tabs().len(), 1);
        assert_ne!(p.tabs()[0], moved);
    })
    .unwrap();
    assert_eq!(cx.update(|cx| cx.windows().len()), 2);
    // The last tab of a window is never torn out.
    w.update(cx, |p, window, cx| {
        p.tear_out_tab(0, window.bounds().origin, window, cx);
        assert_eq!(p.tabs().len(), 1);
    })
    .unwrap();
}
