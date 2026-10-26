//! `PalettePanel` — visual of `CommandPalette.xaml`: search box on top, list of
//! filtered commands below with fzf highlights and key-chord text.

use std::rc::Rc;

use gpui::{
    div, px, App, ClickEvent, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window,
};

use super::filtered_command::FilteredCommand;
use super::fzf::TextRun;
use super::state::PaletteState;
use crate::ui::theme::Theme;

/// Click on row `i` of the visible list.
pub type RowCb = Rc<dyn Fn(usize, &mut Window, &mut App)>;

pub struct PaletteCallbacks {
    pub on_pick: RowCb,
}

/// One visible row, copied out of the state so the panel owns its data.
struct Row {
    name: String,
    highlights: Vec<TextRun>,
    chord: String,
}

#[derive(IntoElement)]
pub struct PalettePanel {
    search: String,
    rows: Vec<Row>,
    selected: usize,
    theme: Theme,
    callbacks: PaletteCallbacks,
}

impl PalettePanel {
    pub fn new<A>(state: &PaletteState<A>, theme: Theme, callbacks: PaletteCallbacks) -> Self {
        let rows = state
            .visible()
            .map(|f: &FilteredCommand| Row {
                name: f.item().name.clone(),
                highlights: f.name_highlights().map(<[_]>::to_vec).unwrap_or_default(),
                chord: f.item().key_chord_text.clone(),
            })
            .collect();
        Self {
            search: state.search().to_string(),
            rows,
            selected: state.selected(),
            theme,
            callbacks,
        }
    }
}

/// Split `text` into `(segment, highlighted)` using inclusive UTF-16 runs.
pub fn split_highlights(text: &str, runs: &[TextRun]) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut u16_off = 0;
    for c in text.chars() {
        let hl = runs.iter().any(|r| u16_off >= r.start && u16_off <= r.end);
        match out.last_mut() {
            Some((s, h)) if *h == hl => s.push(c),
            _ => out.push((c.to_string(), hl)),
        }
        u16_off += c.len_utf16();
    }
    out
}

impl RenderOnce for PalettePanel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.theme;
        let search = if self.search.is_empty() {
            div().text_color(t.fg_dim).child("Type a command name…")
        } else {
            div().child(format!("{}▏", self.search))
        };
        let mut list = div().flex().flex_col().max_h(px(300.)).overflow_hidden();
        for (i, row) in self.rows.iter().enumerate() {
            let on_pick = self.callbacks.on_pick.clone();
            let mut name = div().flex().flex_row().flex_1();
            for (seg, hl) in split_highlights(&row.name, &row.highlights) {
                name = name.child(if hl {
                    div().font_weight(FontWeight::BOLD).child(seg)
                } else {
                    div().child(seg)
                });
            }
            list = list.child(
                div()
                    .id(("palette-row", i))
                    .flex()
                    .flex_row()
                    .items_center()
                    .h(px(32.))
                    .px(px(12.))
                    .rounded(px(4.))
                    .text_size(px(14.))
                    .bg(if i == self.selected {
                        t.tab_hover_bg
                    } else {
                        t.page_bg
                    })
                    .hover(|s| s.bg(t.tab_hover_bg))
                    .on_click(move |_: &ClickEvent, w, cx| on_pick(i, w, cx))
                    .child(name)
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.fg_dim)
                            .child(row.chord.clone()),
                    ),
            );
        }
        // Top-centre like WT (`HorizontalAlignment=Center`, width 600).
        div().size_full().flex().justify_center().child(
            div()
                .mt(px(8.))
                .w(px(600.))
                .max_w_full()
                .h_auto()
                .flex_none()
                .self_start()
                .flex()
                .flex_col()
                .gap(px(4.))
                .p(px(8.))
                .rounded(px(8.))
                .border_1()
                .border_color(t.stroke)
                .bg(t.page_bg)
                .text_color(t.fg)
                .child(
                    div()
                        .h(px(32.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .text_size(px(14.))
                        .bg(t.tab_row_bg)
                        .rounded(px(4.))
                        .child(search),
                )
                .child(list),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn wt_ui__palette_split_highlights() {
        let segs = split_highlights(
            "abcdef",
            &[TextRun { start: 1, end: 2 }, TextRun { start: 5, end: 5 }],
        );
        assert_eq!(
            segs,
            vec![
                ("a".into(), false),
                ("bc".into(), true),
                ("de".into(), false),
                ("f".into(), true)
            ]
        );
        assert_eq!(
            split_highlights("abc", &[]),
            vec![("abc".to_string(), false)]
        );
    }
}
