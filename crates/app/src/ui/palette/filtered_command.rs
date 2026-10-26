//! Port of `TerminalApp/FilteredCommand.cpp`: a palette item plus the
//! highlight spans and weight computed from the current fzf pattern.

use std::cmp::Ordering;

use super::fzf::{match_text, MatchResult, Pattern, TextRun};

/// `IPaletteItem` data used for display + filtering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteItem {
    pub name: String,
    pub subtitle: String,
    pub key_chord_text: String,
}

impl PaletteItem {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), subtitle: String::new(), key_chord_text: String::new() }
    }
}

#[derive(Clone, Debug)]
pub struct FilteredCommand {
    item: PaletteItem,
    pattern: Option<Pattern>,
    weight: i32,
    name_highlights: Option<Vec<TextRun>>,
    subtitle_highlights: Option<Vec<TextRun>>,
}

/// `_matchedSegmentsAndWeight`.
fn segments_and_weight(pattern: &Option<Pattern>, haystack: &str) -> (Option<Vec<TextRun>>, i32) {
    match pattern {
        Some(p) if !p.terms.is_empty() => match match_text(haystack, p) {
            Some(MatchResult { score, runs }) => ((!runs.is_empty()).then_some(runs), score),
            None => (None, 0),
        },
        _ => (None, 0),
    }
}

impl FilteredCommand {
    pub fn new(item: PaletteItem) -> Self {
        Self { item, pattern: None, weight: 0, name_highlights: None, subtitle_highlights: None }
    }

    pub fn item(&self) -> &PaletteItem {
        &self.item
    }

    pub fn weight(&self) -> i32 {
        self.weight
    }

    pub fn name_highlights(&self) -> Option<&[TextRun]> {
        self.name_highlights.as_deref()
    }

    pub fn subtitle_highlights(&self) -> Option<&[TextRun]> {
        self.subtitle_highlights.as_deref()
    }

    /// `UpdateFilter`; no-op if the pattern is unchanged.
    pub fn update_filter(&mut self, pattern: Option<Pattern>) {
        if pattern != self.pattern {
            self.pattern = pattern;
            self.update();
        }
    }

    fn update(&mut self) {
        let (name_segs, mut weight) = segments_and_weight(&self.pattern, &self.item.name);
        let mut sub_segs = None;
        if !self.item.subtitle.is_empty() {
            let (s, w) = segments_and_weight(&self.pattern, &self.item.subtitle);
            sub_segs = s;
            weight = weight.max(w);
        }
        self.name_highlights = name_segs;
        self.subtitle_highlights = sub_segs;
        self.weight = weight;
    }

    /// `FilteredCommand::Compare` — true when `first` should appear before
    /// `second`: higher weight wins, ties by case-insensitive name.
    /// (Deviation: `til::compare_linguistic_insensitive` → lowercase compare.)
    pub fn compare(first: &Self, second: &Self) -> bool {
        match first.weight.cmp(&second.weight) {
            Ordering::Equal => {
                first.item.name.to_lowercase().cmp(&second.item.name.to_lowercase()) == Ordering::Less
            }
            o => o == Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `LocalTests_TerminalApp/FilteredCommandTests.cpp`.
    use super::super::fzf::parse_pattern;
    use super::*;

    fn fc(name: &str) -> FilteredCommand {
        FilteredCommand::new(PaletteItem::new(name))
    }

    fn filter(c: &mut FilteredCommand, s: &str) {
        c.update_filter(Some(parse_pattern(s)));
    }

    fn runs(c: &FilteredCommand) -> Option<Vec<(usize, usize)>> {
        c.name_highlights().map(|r| r.iter().map(|t| (t.start, t.end)).collect())
    }

    #[test]
    fn wt_ui__filteredcommand_verify_highlighting() {
        let mut c = fc("AAAAAABBBBBBCCC");
        assert_eq!(runs(&c), None);
        filter(&mut c, "");
        assert_eq!(runs(&c), None);
        filter(&mut c, "AAAAAABBBBBBCCC");
        assert_eq!(runs(&c), Some(vec![(0, 14)]));
        filter(&mut c, "A");
        assert_eq!(runs(&c), Some(vec![(0, 0)]));
        filter(&mut c, "a");
        assert_eq!(runs(&c), Some(vec![(0, 0)]));
        filter(&mut c, "ab");
        assert_eq!(runs(&c), Some(vec![(5, 6)]));
        filter(&mut c, "abcc");
        assert_eq!(runs(&c), Some(vec![(5, 6), (12, 13)]));
        filter(&mut c, "abcd");
        assert_eq!(runs(&c), None);
    }

    #[test]
    fn wt_ui__filteredcommand_verify_weight() {
        let mut c = fc("AAAAAABBBBBBCCC");
        let mut weigh = |s: Option<&str>| {
            c.update_filter(s.map(parse_pattern));
            c.weight()
        };
        let null = weigh(None);
        let empty = weigh(Some(""));
        let full = weigh(Some("AAAAAABBBBBBCCC"));
        let first = weigh(Some("A"));
        let other_case = weigh(Some("a"));
        let several = weigh(Some("ab"));
        assert_eq!((null, empty), (0, 0));
        assert!(full > 100);
        assert!(first > 0 && first < full);
        assert!(other_case > 0 && other_case < full);
        assert!(several > other_case && several < full);
    }

    #[test]
    fn wt_ui__filteredcommand_verify_compare() {
        let (mut a, mut b) = (fc("AAAAAABBBBBBCCC"), fc("BBBBBCCC"));
        assert_eq!(a.weight(), b.weight());
        assert!(FilteredCommand::compare(&a, &b));
        filter(&mut a, "");
        filter(&mut b, "");
        assert_eq!(a.weight(), b.weight());
        assert!(FilteredCommand::compare(&a, &b));
        filter(&mut a, "B");
        filter(&mut b, "B");
        assert!(a.weight() < b.weight());
        assert!(!FilteredCommand::compare(&a, &b));
    }

    #[test]
    fn wt_ui__filteredcommand_verify_compare_ignore_case() {
        let (a, b) = (fc("a"), fc("B"));
        assert_eq!(a.weight(), b.weight());
        assert!(FilteredCommand::compare(&a, &b));
    }
}
