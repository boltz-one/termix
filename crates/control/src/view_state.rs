//! View-only state of a `TermControl`: scroll position, selection, cursor
//! blink phase and the find box. Pure (no GPUI) so the scroll/search logic is
//! unit-testable.

use core::TextBuffer;

use crate::search::{self, Match};
use crate::selection::Selection;

/// Find box contents and results.
#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub query: String,
    pub matches: Vec<Match>,
    pub current: Option<usize>,
    pub case_sensitive: bool,
}

#[derive(Clone, Debug)]
pub struct ViewState {
    /// Row id of the first visible row, `None` while following the output.
    pub scroll_top: Option<usize>,
    pub selection: Option<Selection>,
    /// Cursor blink phase (true = drawn).
    pub cursor_on: bool,
    pub search: Option<SearchState>,
    /// Font size change in points from Ctrl+Plus/Minus/wheel (0 = profile size).
    pub font_zoom_pt: f32,
    /// Multi-line paste waiting for the user's confirmation.
    pub pending_paste: Option<String>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            scroll_top: None,
            selection: None,
            cursor_on: true,
            search: None,
            font_zoom_pt: 0.0,
            pending_paste: None,
        }
    }
}

/// Row id of the first row when showing the live page.
fn live_top(buf: &TextBuffer) -> usize {
    buf.scrollback_dropped + buf.scrollback.len()
}

impl ViewState {
    /// First visible row as an absolute index into scrollback + page.
    pub fn top_abs(&self, buf: &TextBuffer) -> usize {
        let live = live_top(buf);
        let id = self
            .scroll_top
            .unwrap_or(live)
            .clamp(buf.scrollback_dropped, live);
        id - buf.scrollback_dropped
    }

    pub fn is_live(&self) -> bool {
        self.scroll_top.is_none()
    }

    /// Scroll by `rows` (negative = towards older output).
    pub fn scroll_by(&mut self, buf: &TextBuffer, rows: isize) {
        let live = live_top(buf);
        let cur = self
            .scroll_top
            .unwrap_or(live)
            .clamp(buf.scrollback_dropped, live);
        let new =
            (cur as isize + rows).clamp(buf.scrollback_dropped as isize, live as isize) as usize;
        self.scroll_top = (new != live).then_some(new);
    }

    pub fn scroll_to_top(&mut self, buf: &TextBuffer) {
        let top = buf.scrollback_dropped;
        self.scroll_top = (top != live_top(buf)).then_some(top);
    }

    pub fn snap_to_bottom(&mut self) {
        self.scroll_top = None;
    }

    /// Re-run the search (query or buffer changed); keeps the current match
    /// index when still valid.
    pub fn refresh_search(&mut self, buf: &TextBuffer) {
        if let Some(s) = &mut self.search {
            s.matches = search::find_all(buf, &s.query, s.case_sensitive);
            if s.current
                .is_some_and(|i| i >= search::group_count(&s.matches))
            {
                s.current = None;
            }
        }
    }

    /// Move to the next/previous match and scroll it into view.
    pub fn step_search(&mut self, buf: &TextBuffer, forward: bool, page_rows: usize) {
        let Some(s) = &mut self.search else { return };
        s.current = search::step(&s.matches, s.current, forward);
        let Some(m) = s
            .current
            .and_then(|g| s.matches.iter().find(|m| m.group == g).copied())
        else {
            return;
        };
        let live = live_top(buf);
        let top = self.scroll_top.unwrap_or(live);
        // Keep the match inside the viewport, otherwise centre it.
        if m.id < top || m.id >= top + page_rows {
            let want = m.id.saturating_sub(page_rows / 2);
            self.scroll_top = (want < live).then_some(want.max(buf.scrollback_dropped));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::TextAttribute;

    fn buf_with_history(history: usize, page: u16) -> TextBuffer {
        let mut b = TextBuffer::new(10, page, TextAttribute::default());
        for i in 0..history {
            b.set_text(0, 0, &format!("h{i}"));
            b.increment_circular_buffer(&TextAttribute::default());
        }
        b
    }

    #[test]
    fn scroll_clamps_and_returns_to_live() {
        let b = buf_with_history(10, 4);
        let mut v = ViewState::default();
        assert!(v.is_live());
        v.scroll_by(&b, -3);
        assert_eq!(v.top_abs(&b), 7);
        v.scroll_by(&b, -100);
        assert_eq!(v.top_abs(&b), 0);
        v.scroll_by(&b, 100);
        assert!(v.is_live());
        assert_eq!(v.top_abs(&b), 10);
        v.scroll_to_top(&b);
        assert_eq!(v.top_abs(&b), 0);
        v.snap_to_bottom();
        assert!(v.is_live());
    }

    #[test]
    fn scroll_position_survives_history_trimming() {
        let mut b = buf_with_history(5, 2);
        b.scrollback_limit = 5;
        let mut v = ViewState::default();
        v.scroll_by(&b, -2); // showing abs 3 (= id 3)
        let before = b.row_abs(v.top_abs(&b)).unwrap().glyphs[0].0;
        let id_before = v.scroll_top.unwrap();
        b.set_text(0, 0, "zz");
        b.increment_circular_buffer(&TextAttribute::default()); // trims 1 row
        assert_eq!(v.scroll_top, Some(id_before));
        assert_eq!(b.row_abs(v.top_abs(&b)).unwrap().glyphs[0].0, before);
    }

    #[test]
    fn search_stepping_scrolls_match_into_view() {
        let b = buf_with_history(20, 4);
        let mut v = ViewState {
            search: Some(SearchState {
                query: "h2".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        v.refresh_search(&b);
        let n = search::group_count(&v.search.as_ref().unwrap().matches);
        assert!(n >= 1);
        v.step_search(&b, true, 4);
        let id = v.search.as_ref().unwrap().matches[0].id;
        let top = v.top_abs(&b) + b.scrollback_dropped;
        assert!(id >= top && id < top + 4, "match {id} visible from {top}");
    }
}
