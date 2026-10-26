//! `PageLayout` — port `TerminalPage.xaml` root `Grid` (3 hàng):
//! row 0 tab row/titlebar (Auto), row 1 infobar (Auto), row 2 content (`*`).
//! Dialog/palette/suggestions nằm trong row 2 dưới dạng overlay (`Grid.Row=2`).

use gpui::{
    div, px, AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window,
};

use super::theme::Theme;

/// Bố cục page: titlebar, infobar tuỳ chọn, content + overlays.
#[derive(IntoElement)]
pub struct PageLayout {
    theme: Theme,
    titlebar: AnyElement,
    infobar: Option<AnyElement>,
    content: AnyElement,
    overlays: Vec<AnyElement>,
}

impl PageLayout {
    pub fn new(theme: Theme, titlebar: impl IntoElement, content: impl IntoElement) -> Self {
        Self {
            theme,
            titlebar: titlebar.into_any_element(),
            infobar: None,
            content: content.into_any_element(),
            overlays: Vec::new(),
        }
    }

    /// `CloseOnExitInfoBar` slot (row 1).
    pub fn infobar(mut self, infobar: impl IntoElement) -> Self {
        self.infobar = Some(infobar.into_any_element());
        self
    }

    /// Phủ lên content (row 2): palette, dialog, suggestions…
    pub fn overlay(mut self, overlay: impl IntoElement) -> Self {
        self.overlays.push(overlay.into_any_element());
        self
    }
}

impl RenderOnce for PageLayout {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(self.theme.page_bg)
            .child(self.titlebar);
        if let Some(infobar) = self.infobar {
            root = root.child(div().flex_none().w_full().bg(self.theme.page_bg).child(infobar));
        }
        let mut content = div()
            .relative()
            .flex_1()
            .w_full()
            .min_h(px(0.))
            .overflow_hidden()
            .child(div().size_full().child(self.content));
        for overlay in self.overlays {
            content = content.child(div().absolute().inset_0().child(overlay));
        }
        root.child(content)
    }
}
