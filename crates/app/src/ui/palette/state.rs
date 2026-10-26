//! `PaletteState` — pure filter/selection state of `CommandPalette`
//! (action mode): `_updateFilteredActions`, `_selectNextItem`.

use super::filtered_command::{FilteredCommand, PaletteItem};
use super::fzf::parse_pattern;

pub struct PaletteState<A> {
    all: Vec<(FilteredCommand, A)>,
    search: String,
    /// Indices into `all`, in display order.
    visible: Vec<usize>,
    selected: usize,
}

impl<A> PaletteState<A> {
    pub fn new(commands: Vec<(PaletteItem, A)>) -> Self {
        let all: Vec<_> = commands.into_iter().map(|(i, a)| (FilteredCommand::new(i), a)).collect();
        let visible = (0..all.len()).collect();
        Self { all, search: String::new(), visible, selected: 0 }
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn visible(&self) -> impl Iterator<Item = &FilteredCommand> {
        self.visible.iter().map(|&i| &self.all[i].0)
    }

    pub fn len(&self) -> usize {
        self.visible.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }

    /// Replace the search text: empty keeps source order; otherwise drop
    /// non-matching items and sort by `FilteredCommand::compare`. Selection
    /// resets to the top (WT `_searchTextChanged`).
    pub fn set_search(&mut self, text: &str) {
        self.search = text.to_string();
        let pattern = parse_pattern(text);
        let has_filter = !pattern.terms.is_empty();
        for (fc, _) in &mut self.all {
            fc.update_filter(Some(pattern.clone()));
        }
        let mut idx: Vec<usize> = (0..self.all.len())
            .filter(|&i| !has_filter || self.all[i].0.weight() > 0)
            .collect();
        if has_filter {
            idx.sort_by(|&a, &b| {
                let (x, y) = (&self.all[a].0, &self.all[b].0);
                if FilteredCommand::compare(x, y) {
                    std::cmp::Ordering::Less
                } else if FilteredCommand::compare(y, x) {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }
        self.visible = idx;
        self.selected = 0;
    }

    /// Move selection by `delta`, wrapping at both ends.
    pub fn move_selection(&mut self, delta: isize) {
        let n = self.visible.len() as isize;
        if n > 0 {
            self.selected = (self.selected as isize + delta).rem_euclid(n) as usize;
        }
    }

    pub fn selected_action(&self) -> Option<&A> {
        self.visible.get(self.selected).map(|&i| &self.all[i].1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PaletteState<u8> {
        PaletteState::new(
            ["New tab", "Close tab", "Next tab", "Split pane"]
                .iter()
                .enumerate()
                .map(|(i, n)| (PaletteItem::new(*n), i as u8))
                .collect(),
        )
    }

    fn names(s: &PaletteState<u8>) -> Vec<String> {
        s.visible().map(|f| f.item().name.clone()).collect()
    }

    #[test]
    fn wt_ui__palette_empty_search_keeps_order() {
        let s = state();
        assert_eq!(names(&s), ["New tab", "Close tab", "Next tab", "Split pane"]);
        assert_eq!(s.selected_action(), Some(&0));
    }

    #[test]
    fn wt_ui__palette_filter_drops_nonmatching_and_ranks() {
        let mut s = state();
        s.set_search("split");
        assert_eq!(names(&s), ["Split pane"]);
        s.set_search("nt");
        // "New tab": both letters on word boundaries; "Next tab" matches the inner `t` of "Next".
        assert_eq!(names(&s)[0], "New tab");
        s.set_search("zzz");
        assert!(s.is_empty());
        assert_eq!(s.selected_action(), None);
    }

    #[test]
    fn wt_ui__palette_selection_wraps_and_resets_on_search() {
        let mut s = state();
        s.move_selection(-1);
        assert_eq!(s.selected(), 3);
        s.move_selection(1);
        assert_eq!(s.selected(), 0);
        s.move_selection(2);
        s.set_search("tab");
        assert_eq!(s.selected(), 0);
    }
}
