//! Find in scrollback + page (`TerminalControl` search box, `Search.cpp`):
//! case-insensitive unless asked otherwise; a match may continue across
//! soft-wrapped rows and is then reported as one part per row (same `group`).

use core::TextBuffer;

/// One highlighted part of a match; `id` is the stable row id (see
/// `selection::Cell`), `col`/`len` are in columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub id: usize,
    pub col: usize,
    pub len: usize,
    /// Index of the occurrence this part belongs to.
    pub group: usize,
}

fn fold(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

/// One searchable cell: character plus where it sits on screen.
struct Cell {
    ch: char,
    id: usize,
    col: usize,
    width: usize,
}

/// All non-overlapping occurrences, top to bottom, left to right.
pub fn find_all(buf: &TextBuffer, query: &str, case_sensitive: bool) -> Vec<Match> {
    let norm = |c: char| if case_sensitive { c } else { fold(c) };
    let needle: Vec<char> = query.chars().map(norm).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut group = 0;
    let total = buf.total_rows();
    let mut abs = 0;
    while abs < total {
        // Logical line: rows joined while the row wraps into the next one.
        let mut cells: Vec<Cell> = Vec::new();
        loop {
            let Some(row) = buf.row_abs(abs) else { break };
            let id = buf.scrollback_dropped + abs;
            for (col, &(ch, w)) in row.glyphs().iter().enumerate() {
                if w != 0 {
                    cells.push(Cell { ch, id, col, width: w as usize });
                }
            }
            abs += 1;
            if !(row.wrap_forced && abs < total) {
                break;
            }
        }
        let hay: Vec<char> = cells.iter().map(|c| norm(c.ch)).collect();
        let mut i = 0;
        while i + needle.len() <= hay.len() {
            if hay[i..i + needle.len()] == needle[..] {
                let mut part: Option<(usize, usize, usize)> = None; // (id, col, end)
                for c in &cells[i..i + needle.len()] {
                    match &mut part {
                        Some((id, _, end)) if *id == c.id => *end = c.col + c.width,
                        _ => {
                            if let Some((id, col, end)) = part.take() {
                                out.push(Match { id, col, len: end - col, group });
                            }
                            part = Some((c.id, c.col, c.col + c.width));
                        }
                    }
                }
                if let Some((id, col, end)) = part {
                    out.push(Match { id, col, len: end - col, group });
                }
                group += 1;
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Number of occurrences (a match wrapped over two rows counts once).
pub fn group_count(matches: &[Match]) -> usize {
    matches.last().map_or(0, |m| m.group + 1)
}

/// Occurrence after/before `current` (wrapping); `None` if there are none.
pub fn step(matches: &[Match], current: Option<usize>, forward: bool) -> Option<usize> {
    let n = group_count(matches);
    if n == 0 {
        return None;
    }
    Some(match (current, forward) {
        (None, true) => 0,
        (None, false) => n - 1,
        (Some(i), true) => (i + 1) % n,
        (Some(i), false) => (i + n - 1) % n,
    })
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

    fn part(id: usize, col: usize, len: usize, group: usize) -> Match {
        Match { id, col, len, group }
    }

    #[test]
    fn finds_case_insensitively_across_rows_and_history() {
        let mut b = buf(&["Foo foo", "none", "FOO"]);
        b.increment_circular_buffer(&TextAttribute::default()); // "Foo foo" -> history
        assert_eq!(find_all(&b, "foo", false), vec![part(0, 0, 3, 0), part(0, 4, 3, 1), part(2, 0, 3, 2)]);
        assert_eq!(group_count(&find_all(&b, "foo", true)), 1, "case sensitive");
        assert!(find_all(&b, "", false).is_empty());
    }

    #[test]
    fn match_continues_across_a_soft_wrap() {
        let mut b = TextBuffer::new(6, 3, TextAttribute::default());
        b.set_text(0, 0, "abcdef");
        b.rows_mut()[0].wrap_forced = true;
        b.set_text(1, 0, "ghi");
        let m = find_all(&b, "efgh", false);
        assert_eq!(m, vec![part(0, 4, 2, 0), part(1, 0, 2, 0)]);
        assert_eq!(group_count(&m), 1);
        assert!(find_all(&b, "efgh", false).len() == 2 && find_all(&buf(&["abcdef", "ghi"]), "efgh", false).is_empty());
    }

    #[test]
    fn wide_characters_report_columns() {
        let mut b = TextBuffer::new(10, 1, TextAttribute::default());
        b.set_text(0, 0, "a\u{65E5}\u{672C}b");
        let m = find_all(&b, "\u{672C}b", false);
        assert_eq!(m, vec![part(0, 3, 3, 0)]);
    }

    #[test]
    fn stepping_wraps_over_occurrences() {
        let m = [part(0, 0, 1, 0), part(1, 0, 1, 0), part(2, 0, 1, 1)];
        assert_eq!(step(&m, None, true), Some(0));
        assert_eq!(step(&m, Some(1), true), Some(0));
        assert_eq!(step(&m, Some(0), false), Some(1));
        assert_eq!(step(&[], None, true), None);
    }
}
