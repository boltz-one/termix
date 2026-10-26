//! Text selection over the scrollback + page (`Terminal::SetSelectionAnchor`,
//! `TerminalSelection.cpp`): linear selection by character, word or line, and
//! text extraction for copy. Positions use stable row ids so a selection
//! survives scrolling and history trimming.

use core::TextBuffer;

/// WT default `wordDelimiters`.
const WORD_DELIMITERS: &str = " /\\()\"'-.,:;<>~!@#$%^&*|+=[]{}~?\u{2502}";

/// A cell: column + stable row id (`scrollback_dropped + absolute index`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell {
    pub id: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granularity {
    Char,
    Word,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    anchor: Cell,
    head: Cell,
    granularity: Granularity,
}

/// Cells of a row as `(char, width)`; width 0 marks a wide glyph's trailer.
fn row_cells(buf: &TextBuffer, id: usize) -> Option<Vec<(char, u8)>> {
    let abs = id.checked_sub(buf.scrollback_dropped)?;
    Some(buf.row_abs(abs)?.glyphs().to_vec())
}

fn row_text(buf: &TextBuffer, id: usize) -> Option<Vec<char>> {
    Some(row_cells(buf, id)?.into_iter().map(|(c, _)| c).collect())
}

/// `[start, end]` columns of the word (or delimiter run) containing `col`.
fn word_bounds(chars: &[char], col: usize) -> (usize, usize) {
    if chars.is_empty() {
        return (col, col);
    }
    let col = col.min(chars.len() - 1);
    let is_delim = |c: char| WORD_DELIMITERS.contains(c);
    let kind = is_delim(chars[col]);
    let (mut s, mut e) = (col, col);
    while s > 0 && is_delim(chars[s - 1]) == kind {
        s -= 1;
    }
    while e + 1 < chars.len() && is_delim(chars[e + 1]) == kind {
        e += 1;
    }
    (s, e)
}

impl Selection {
    pub fn new(at: Cell, granularity: Granularity) -> Self {
        Self {
            anchor: at,
            head: at,
            granularity,
        }
    }

    pub fn granularity(&self) -> Granularity {
        self.granularity
    }

    /// Move the head to `to`; word/line snapping is applied by `ordered`.
    pub fn extend(&mut self, to: Cell) {
        self.head = to;
    }

    /// Normalised `(start, end)` inclusive, expanded for word/line modes.
    pub fn ordered(&self, buf: &TextBuffer) -> (Cell, Cell) {
        let (mut a, mut b) = if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        };
        match self.granularity {
            Granularity::Char => {}
            Granularity::Word => {
                if let Some(t) = row_text(buf, a.id) {
                    a.col = word_bounds(&t, a.col).0;
                }
                if let Some(t) = row_text(buf, b.id) {
                    b.col = word_bounds(&t, b.col).1;
                }
            }
            Granularity::Line => {
                a.col = 0;
                b.col = usize::MAX;
            }
        }
        (a, b)
    }

    pub fn contains(&self, buf: &TextBuffer, id: usize, col: usize) -> bool {
        let (a, b) = self.ordered(buf);
        let c = Cell { id, col };
        c >= a && c <= b
    }

    /// Selected text, rows joined with `\r\n` (soft-wrapped rows are joined
    /// without a break), trailing blanks trimmed per row.
    pub fn text(&self, buf: &TextBuffer) -> String {
        let (a, b) = self.ordered(buf);
        let mut out = String::new();
        for id in a.id..=b.id {
            let Some(cells) = row_cells(buf, id) else {
                continue;
            };
            let from = if id == a.id { a.col } else { 0 };
            let to = if id == b.id {
                b.col.saturating_add(1).min(cells.len())
            } else {
                cells.len()
            };
            let mut line: String = cells
                .get(from..to.max(from))
                .unwrap_or(&[])
                .iter()
                .filter(|(_, w)| *w != 0)
                .map(|(c, _)| *c)
                .collect();
            let wrapped = buf
                .row_abs(id - buf.scrollback_dropped)
                .is_some_and(|r| r.wrap_forced);
            if !wrapped || id == b.id {
                line = line.trim_end().to_string();
            }
            out.push_str(&line);
            if id != b.id && !wrapped {
                out.push_str("\r\n");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::TextAttribute;

    fn buf(lines: &[&str]) -> TextBuffer {
        let mut b = TextBuffer::new(20, lines.len() as u16, TextAttribute::default());
        for (i, l) in lines.iter().enumerate() {
            b.set_text(i, 0, l);
        }
        b
    }

    fn cell(id: usize, col: usize) -> Cell {
        Cell { id, col }
    }

    #[test]
    fn char_selection_across_rows() {
        let b = buf(&["hello world", "second line"]);
        let mut s = Selection::new(cell(0, 6), Granularity::Char);
        s.extend(cell(1, 5));
        assert_eq!(s.text(&b), "world\r\nsecond");
        assert!(s.contains(&b, 0, 8) && !s.contains(&b, 0, 5) && !s.contains(&b, 1, 6));
    }

    #[test]
    fn selection_is_direction_independent() {
        let b = buf(&["abcdef"]);
        let mut s = Selection::new(cell(0, 4), Granularity::Char);
        s.extend(cell(0, 1));
        assert_eq!(s.text(&b), "bcde");
    }

    #[test]
    fn word_and_line_granularity() {
        let b = buf(&["foo bar-baz qux"]);
        let s = Selection::new(cell(0, 5), Granularity::Word);
        assert_eq!(s.text(&b), "bar");
        let mut s = Selection::new(cell(0, 5), Granularity::Word);
        s.extend(cell(0, 13));
        assert_eq!(s.text(&b), "bar-baz qux");
        let s = Selection::new(cell(0, 5), Granularity::Line);
        assert_eq!(s.text(&b), "foo bar-baz qux");
    }

    #[test]
    fn scrollback_rows_use_stable_ids() {
        let mut b = buf(&["one", "two"]);
        b.increment_circular_buffer(&TextAttribute::default()); // "one" -> history
        b.set_text(1, 0, "three");
        let s = Selection::new(cell(0, 0), Granularity::Line);
        assert_eq!(s.text(&b), "one");
        b.scrollback_limit = 1;
        b.increment_circular_buffer(&TextAttribute::default()); // "two" -> history, "one" dropped
        let s = Selection::new(cell(1, 0), Granularity::Line);
        assert_eq!(
            s.text(&b),
            "two",
            "id 1 still names the same row after trimming"
        );
    }
}
