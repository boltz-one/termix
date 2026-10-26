//! `TabStrip` — port `TabRowControl.xaml` (TabView, `TabWidthMode=Equal`,
//! new-tab button) + `TabHeaderControl.xaml` (status glyph + title + close).

use std::rc::Rc;

use gpui::{
    deferred, div, px, App, ClickEvent, InteractiveElement, IntoElement, MouseButton,
    MouseMoveEvent, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window,
};

use super::tab_strip_model::TabStripModel;
use super::theme::{metrics, Theme, ICON_FONT};
use gpui::prelude::FluentBuilder as _;

type IndexCb = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type VoidCb = Rc<dyn Fn(&mut Window, &mut App)>;
type ContextCb = Rc<dyn Fn(usize, gpui::Point<gpui::Pixels>, &mut Window, &mut App)>;
type MenuPickCb = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// Callback của tab row. `Rc` vì `RenderOnce` rebuild mỗi frame.
#[derive(Clone)]
pub struct TabStripCallbacks {
    /// Click tab → chọn.
    pub on_select: IndexCb,
    /// Click nút đóng trên tab.
    pub on_close: IndexCb,
    /// Click nút `+` (`OnNewTabButtonClick`).
    pub on_new_tab: VoidCb,
    /// Mouse down on a tab: starts a possible drag-reorder.
    pub on_drag_start: IndexCb,
    /// Pointer (left button held) moved over tab `i` while dragging.
    pub on_drag_over: IndexCb,
    /// Right-click on tab `i` opens its context menu.
    pub on_context: ContextCb,
    /// Pick item `item` of tab `tab`'s context menu.
    pub on_tab_menu_pick: MenuPickCb,
}

/// Entries of the tab context menu, in order (`TabRowControl` flyout).
pub const TAB_MENU: [&str; 6] = [
    "Close tab",
    "Close other tabs",
    "Close tabs to the right",
    "Duplicate tab",
    "Move tab to new window",
    "Rename tab",
];

/// Tab row: các tab chia đều chiều rộng + nút new-tab.
#[derive(IntoElement)]
pub struct TabStrip {
    model: TabStripModel,
    theme: Theme,
    callbacks: TabStripCallbacks,
}

impl TabStrip {
    pub fn new(model: TabStripModel, theme: Theme, callbacks: TabStripCallbacks) -> Self {
        Self {
            model,
            theme,
            callbacks,
        }
    }
}

impl RenderOnce for TabStrip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            model,
            theme,
            callbacks,
        } = self;
        let selected = model.selected();
        let mut row = div()
            .flex()
            .flex_row()
            .items_end()
            .h_full()
            .min_w(px(0.))
            .flex_shrink(1.)
            .overflow_hidden();
        for (i, item) in model.items().iter().enumerate() {
            let is_sel = selected == Some(i);
            let group = format!("tab-{i}");
            let on_select = callbacks.on_select.clone();
            let on_close = callbacks.on_close.clone();
            let on_drag_start = callbacks.on_drag_start.clone();
            let on_drag_over = callbacks.on_drag_over.clone();
            let on_context = callbacks.on_context.clone();
            let mut header = div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .flex_1()
                .min_w(px(0.));
            for g in item.status.glyphs() {
                header = header.child(icon(g, 12., theme.fg_dim));
            }
            header = header.child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(12.))
                    .child(item.display_title().to_string()),
            );
            let close = div()
                .id(("tab-close", i))
                .flex()
                .items_center()
                .justify_center()
                .size(px(20.))
                .rounded(px(4.))
                .hover(|s| {
                    s.bg(if is_sel {
                        theme.tab_row_bg
                    } else {
                        theme.tab_hover_bg
                    })
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_: &ClickEvent, w, cx| {
                    cx.stop_propagation();
                    on_close(i, w, cx);
                })
                .child(icon('\u{E711}', 10., theme.fg_dim));
            row = row.child(
                div()
                    .id(("tab", i))
                    .group(group)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .relative()
                    .h_full()
                    .w(px(metrics::TAB_MAX_W))
                    .flex_shrink(1.)
                    .min_w(px(metrics::TAB_MIN_W))
                    .pl(px(12.))
                    .pr(px(6.))
                    .text_color(if is_sel { theme.fg } else { theme.fg_dim })
                    .bg(if is_sel {
                        theme.tab_selected_bg
                    } else {
                        theme.tab_row_bg
                    })
                    .when(!is_sel, |d| d.hover(|s| s.bg(theme.tab_hover_bg)))
                    .on_click(move |_: &ClickEvent, w, cx| on_select(i, w, cx))
                    .on_mouse_down(MouseButton::Left, move |_, w, cx| on_drag_start(i, w, cx))
                    .on_mouse_down(MouseButton::Right, move |ev, w, cx| {
                        on_context(i, ev.position, w, cx)
                    })
                    .on_mouse_move(move |ev: &MouseMoveEvent, w, cx| {
                        if ev.pressed_button == Some(MouseButton::Left) {
                            on_drag_over(i, w, cx);
                        }
                    })
                    .child(header)
                    .child(close),
            );
        }
        let on_new = callbacks.on_new_tab;
        let new_tab = div()
            .id("new-tab")
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            .w(px(metrics::NEW_TAB_W))
            .h(px(metrics::NEW_TAB_H))
            .mb(px(4.))
            .ml(px(4.))
            .rounded(px(4.))
            .hover(|s| s.bg(theme.tab_hover_bg))
            .on_click(move |_: &ClickEvent, w, cx| on_new(w, cx))
            .child(icon('\u{E710}', 12., theme.fg));
        div()
            .flex()
            .flex_row()
            .items_end()
            .h_full()
            .min_w(px(0.))
            .child(row)
            .child(new_tab)
    }
}

/// Context menu of tab `tab`, drawn at the pointer position `pos` (window
/// coordinates). It lives at page level, outside the tab row's clipping, so
/// its entries stay clickable.
pub fn tab_context_menu(
    theme: Theme,
    tab: usize,
    pos: gpui::Point<gpui::Pixels>,
    on_pick: MenuPickCb,
) -> impl IntoElement {
    let mut list = div()
        .id("tab-context-menu")
        // Presses inside the menu must not reach the click-away backdrop below it.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .absolute()
        .left(pos.x)
        .top(pos.y)
        .min_w(px(220.))
        .flex()
        .flex_col()
        .p(px(4.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.stroke)
        .bg(theme.page_bg)
        .text_color(theme.fg)
        .text_size(px(13.));
    for (k, label) in TAB_MENU.iter().enumerate() {
        let on_pick = on_pick.clone();
        list = list.child(
            div()
                .id(("tab-menu-item", tab * 16 + k))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(4.))
                .hover(|s| s.bg(theme.tab_hover_bg))
                .on_click(move |_: &ClickEvent, w, cx| on_pick(tab, k, w, cx))
                .child(*label),
        );
    }
    deferred(list).with_priority(10)
}

/// Glyph icon từ Segoe MDL2 Assets.
fn icon(glyph: char, size: f32, color: gpui::Rgba) -> impl IntoElement {
    div()
        .font_family(ICON_FONT)
        .text_size(px(size))
        .text_color(color)
        .child(glyph.to_string())
}
