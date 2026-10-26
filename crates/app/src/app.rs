//! `app` — entry point. `Application::run` mở cửa sổ gốc với `TerminalPage` làm
//! root view. Wrap các lỗi khởi tạo thành log + fallback graceful.
//!
//! Port mỏng của `TerminalPage::Create` + `AppHost` flow — ở WT XAML host,
//! ở đây GPUI `Application`. Phase 2 chỉ wire root view; full command dispatch
//! / shortkey (`TerminalPage::_Handle...`) phase 3.

use crate::page::TerminalPage;
use crate::settings_loader::load_settings;
use gpui::{App, AppContext, TitlebarOptions, WindowOptions};

/// Entry point — init `Application`, load settings, open root window.
/// Trả `Result<()>` để `main` propagate lỗi khởi tạo. Lỗi window build (sau khi
/// `Application::run` đã block) log trong closure; không propagate ra
/// (`Application::run` consumes self, không return).
pub fn run() -> anyhow::Result<()> {
    let app = gpui_platform::application();
    app.run(move |cx: &mut App| {
        // Load settings trong closure — `load_settings` cần `&mut App`.
        let settings = match load_settings(cx) {
            Ok(s) => s,
            Err(e) => {
                log::error!(
                    "CascadiaSettings load failed — phase 2 fallback, không mở cửa sổ: {e}"
                );
                return;
            }
        };
        // Titlebar tuỳ biến: tab row vẽ vào vùng caption (`ExtendsContentIntoTitleBar`).
        // WT opens at `initialCols` × `initialRows` cells (120 × 30 by default):
        // the cell grid plus the pane border/padding and the tab row.
        let window_bounds = {
            let default = settings
                .profiles
                .iter()
                .find(|p| {
                    uuid::Uuid::parse_str(p.guid.trim_matches(|c: char| c == '{' || c == '}')).ok()
                        == Some(settings.default_profile_guid)
                })
                .or_else(|| settings.profiles.first());
            let (face, pt) = default
                .map(|p| (p.default_appearance.font.face.clone(), p.default_appearance.font.size))
                .unwrap_or_else(|| ("Cascadia Mono".into(), 12.0));
            let (cell_w, line_h) = control::term_view::cell_size(cx, &face, pt);
            let inset = 2.0 * crate::pane::PANE_INSET;
            let w = settings.globals.initial_cols as f32 * cell_w + inset;
            let h = settings.globals.initial_rows as f32 * line_h + inset + crate::ui::theme::metrics::TITLEBAR_H;
            gpui::WindowBounds::Windowed(gpui::Bounds::centered(None, gpui::size(gpui::px(w), gpui::px(h)), cx))
        };
        let window_opts = WindowOptions {
            window_bounds: Some(window_bounds),
            titlebar: Some(TitlebarOptions {
                title: Some("Termix".into()),
                appears_transparent: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        match cx.open_window(window_opts, |_window, cx| {
            cx.new(|cx| TerminalPage::new(settings, cx))
        }) {
            Ok(_) => log::info!("TerminalPage root window opened"),
            Err(e) => log::error!("cx.open_window failed: {e}"),
        }
    });
    Ok(())
}
