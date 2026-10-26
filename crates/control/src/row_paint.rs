//! Paints one buffer row: attribute backgrounds, selection / search highlights,
//! then the text with per-run colour, weight, slant, underline, strikethrough.

use core::Row;
use gpui::{
    fill, point, px, rgb, size, Bounds, Font, FontStyle, FontWeight, Hsla, Pixels, Point,
    TextAlign, TextRun, Window,
};

use crate::term_style::{CellStyle, Palette};

/// Highlight colours drawn under the text.
const SELECTION: u32 = 0x264F78;
const MATCH: u32 = 0x6B5A00;
const CURRENT_MATCH: u32 = 0xB35C00;

/// Cell grid geometry of a frame.
#[derive(Clone, Copy)]
pub struct Grid {
    pub origin: Point<Pixels>,
    pub cell_w: Pixels,
    pub line_h: Pixels,
    pub font_size: Pixels,
    pub cols: usize,
}

/// A column span `[start, end)` to highlight.
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub color: u32,
}

pub fn selection_span(start: usize, end: usize) -> Span {
    Span {
        start,
        end,
        color: SELECTION,
    }
}

pub fn match_span(start: usize, end: usize, current: bool) -> Span {
    Span {
        start,
        end,
        color: if current { CURRENT_MATCH } else { MATCH },
    }
}

fn hsla(c: u32) -> Hsla {
    rgb(c).into()
}

fn cell_bounds(g: &Grid, row_i: usize, col: usize, cols: usize) -> Bounds<Pixels> {
    Bounds::new(
        point(
            g.origin.x + g.cell_w * col as f32,
            g.origin.y + g.line_h * row_i as f32,
        ),
        size(g.cell_w * cols as f32, g.line_h),
    )
}

fn text_run(base: &Font, len: usize, st: &CellStyle) -> TextRun {
    let mut font = base.clone();
    if st.bold {
        font.weight = FontWeight::BOLD;
    }
    if st.italic {
        font.style = FontStyle::Italic;
    }
    let mut color = hsla(st.fg);
    if st.faint {
        color.a = 0.6;
    }
    TextRun {
        len,
        font,
        color,
        background_color: None,
        underline: st.underline.then(|| gpui::UnderlineStyle {
            thickness: px(1.),
            color: Some(color),
            wavy: false,
        }),
        strikethrough: st.strike.then(|| gpui::StrikethroughStyle {
            thickness: px(1.),
            color: Some(color),
        }),
    }
}

/// Draw `row` at screen row `row_i`.
#[allow(clippy::too_many_arguments)]
pub fn paint_row(
    window: &mut Window,
    cx: &mut gpui::App,
    g: &Grid,
    row_i: usize,
    row: &Row,
    palette: &Palette,
    base_font: &Font,
    highlights: &[Span],
) {
    let styles: Vec<CellStyle> = (0..g.cols)
        .map(|c| palette.resolve(row.attr_at(c)))
        .collect();

    // Attribute backgrounds (runs of equal colour).
    let mut c = 0;
    while c < g.cols {
        let bg = styles[c].bg;
        let mut e = c + 1;
        while e < g.cols && styles[e].bg == bg {
            e += 1;
        }
        if let Some(b) = bg {
            window.paint_quad(fill(cell_bounds(g, row_i, c, e - c), rgb(b)));
        }
        c = e;
    }
    // Selection / search highlights.
    for h in highlights {
        let (s, e) = (h.start.min(g.cols), h.end.min(g.cols));
        if s < e {
            window.paint_quad(fill(cell_bounds(g, row_i, s, e - s), rgb(h.color)));
        }
    }

    // Text: one shaped line per stretch of single-width glyphs, with one run
    // per style change; wide glyphs are placed individually to keep the grid.
    let ts = window.text_system().clone();
    let mut text = String::new();
    let mut runs: Vec<TextRun> = Vec::new();
    let mut start_col = 0usize;
    let mut col;
    let flush = |window: &mut Window,
                 cx: &mut gpui::App,
                 text: &mut String,
                 runs: &mut Vec<TextRun>,
                 start_col: usize| {
        if text.is_empty() {
            return;
        }
        let line = ts.shape_line(
            std::mem::take(text).into(),
            g.font_size,
            runs,
            Some(g.cell_w),
        );
        runs.clear();
        let origin = point(
            g.origin.x + g.cell_w * start_col as f32,
            g.origin.y + g.line_h * row_i as f32,
        );
        let _ = line.paint(origin, g.line_h, TextAlign::Left, None, window, cx);
    };
    for (idx, &(ch, width)) in row.glyphs().iter().enumerate() {
        col = idx;
        if col >= g.cols {
            break;
        }
        let st = styles[col];
        if width == 0 {
            // Trailer of a wide glyph: already covered by its lead.
            flush(window, cx, &mut text, &mut runs, start_col);
            continue;
        }
        if width == 1 {
            if st.invisible {
                flush(window, cx, &mut text, &mut runs, start_col);
            } else {
                if text.is_empty() {
                    start_col = col;
                }
                match runs.last_mut() {
                    Some(r) if same_look(r, &text_run(base_font, 0, &st)) => r.len += ch.len_utf8(),
                    _ => runs.push(text_run(base_font, ch.len_utf8(), &st)),
                }
                text.push(ch);
            }
        } else {
            flush(window, cx, &mut text, &mut runs, start_col);
            if !st.invisible {
                let r = [text_run(base_font, ch.len_utf8(), &st)];
                let line = ts.shape_line(
                    ch.to_string().into(),
                    g.font_size,
                    &r,
                    Some(g.cell_w * width as f32),
                );
                let origin = point(
                    g.origin.x + g.cell_w * col as f32,
                    g.origin.y + g.line_h * row_i as f32,
                );
                let _ = line.paint(origin, g.line_h, TextAlign::Left, None, window, cx);
            }
        }
    }
    flush(window, cx, &mut text, &mut runs, start_col);
}

/// Runs are mergeable when everything but `len` matches.
fn same_look(a: &TextRun, b: &TextRun) -> bool {
    a.font == b.font
        && a.color == b.color
        && a.underline == b.underline
        && a.strikethrough == b.strikethrough
}
