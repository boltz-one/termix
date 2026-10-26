//! `Titlebar` — port `TitlebarControl.xaml` (3 cột: content | DragBar | caption
//! buttons) + `MinMaxCloseControl.xaml`. Trên Windows, GPUI hit-test cửa sổ qua
//! `window_control_area` nên không cần handler click.

use gpui::{
    div, px, AnyElement, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    Window, WindowControlArea,
};

use super::theme::{metrics, Theme, ICON_FONT};

/// Titlebar tuỳ biến: `content` (tab row) bên trái, vùng kéo, nút caption phải.
#[derive(IntoElement)]
pub struct Titlebar {
    content: AnyElement,
    theme: Theme,
    maximized: bool,
}

impl Titlebar {
    pub fn new(content: impl IntoElement, theme: Theme, maximized: bool) -> Self {
        Self { content: content.into_any_element(), theme, maximized }
    }
}

impl RenderOnce for Titlebar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.theme;
        let max_glyph = if self.maximized { '\u{E923}' } else { '\u{E922}' };
        div()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .h(px(metrics::TITLEBAR_H))
            .flex_none()
            .bg(t.tab_row_bg)
            // Bottom rule drawn first so the selected tab (same colour as the
            // body) paints over it and merges with the content below.
            .child(div().absolute().left_0().right_0().bottom_0().h(px(1.)).bg(t.stroke))
            .child(div().flex().flex_1().min_w(px(0.)).h_full().child(self.content))
            .child(
                // DragBar (`MinWidth=45`) — vùng kéo cửa sổ.
                div()
                    .h_full()
                    .min_w(px(metrics::DRAG_MIN_W))
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(caption_button('\u{E921}', WindowControlArea::Min, t, false))
            .child(caption_button(max_glyph, WindowControlArea::Max, t, false))
            .child(caption_button('\u{E8BB}', WindowControlArea::Close, t, true))
    }
}

/// 46×30 caption button; Close đổi sang nền đỏ khi hover/press.
fn caption_button(glyph: char, area: WindowControlArea, t: Theme, close: bool) -> impl IntoElement {
    let (hover, pressed) = if close {
        (t.close_hover_bg, t.close_pressed_bg)
    } else {
        (t.caption_hover_bg, t.caption_pressed_bg)
    };
    div()
        .id(("caption", glyph as u32))
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .w(px(metrics::CAPTION_W))
        .h_full()
        .font_family(ICON_FONT)
        .text_size(px(10.))
        .text_color(t.fg)
        .window_control_area(area)
        .hover(move |s| {
            let s = s.bg(hover);
            if close { s.text_color(gpui::white()) } else { s }
        })
        .active(move |s| s.bg(pressed))
        .child(glyph.to_string())
}
