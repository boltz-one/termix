//! Paints the `TextBuffer` of a `ControlCore` into a GPUI canvas and keeps the
//! buffer/ConPTY size in sync with the available pixels. Interim stand-in for
//! the `renderer` engine (P10): fixed cell grid, per-cell colours,
//! scrollback, selection / search highlights and the cursor.

use std::cell::Cell as StdCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    canvas, fill, outline, point, px, rgb, size, BorderStyle, Bounds, ElementInputHandler, Entity, FocusHandle, Font, Hsla,
    IntoElement, Pixels, Point, Styled, Window,
};
use parking_lot::Mutex;
use core::CursorShape;

use crate::control::TermControl;
use crate::core::ControlCore;
use crate::row_paint::{match_span, paint_row, selection_span, Grid, Span};
use crate::term_style::Palette;
use crate::view_state::ViewState;

const LINE_HEIGHT_FACTOR: f32 = 1.25;
pub use crate::term_style::DEFAULT_BG as BG;

/// Geometry of the last painted frame, used to map mouse positions to cells.
#[derive(Clone, Copy, Default)]
pub struct Metrics {
    pub origin: Point<Pixels>,
    pub cell_w: Pixels,
    pub line_h: Pixels,
    /// Row id shown on the first screen row.
    pub top_id: usize,
    pub cols: u16,
    pub rows: u16,
}

pub type SharedMetrics = Rc<StdCell<Metrics>>;

/// Everything one frame needs besides the core.
pub struct PaintInput {
    pub core: Arc<Mutex<ControlCore>>,
    pub entity: Entity<TermControl>,
    pub focus: FocusHandle,
    pub focused: bool,
    pub view: ViewState,
    pub metrics: SharedMetrics,
    /// Profile font (`font.face`) and effective size in pixels.
    pub font_family: String,
    pub font_px: f32,
    /// IME composition drawn at the cursor.
    pub preedit: Option<String>,
}

fn pick_font(window: &Window, wanted: &str) -> Font {
    pick_font_in(window.text_system(), wanted)
}

/// Cell size (width, line height) in px for the profile font at `font_pt`,
/// as the canvas will measure it; used to size the window from rows × cols.
pub fn cell_size(cx: &gpui::App, face: &str, font_pt: f32) -> (f32, f32) {
    let ts = cx.text_system();
    let font_px = font_pt * 96.0 / 72.0;
    let font = pick_font_in(ts, face);
    let w = ts
        .advance(ts.resolve_font(&font), px(font_px), 'M')
        .map(|s| f32::from(s.width))
        .unwrap_or(font_px * 0.6);
    (w, (font_px * LINE_HEIGHT_FACTOR).round())
}

fn pick_font_in(ts: &gpui::TextSystem, wanted: &str) -> Font {
    let names = ts.all_font_names();
    let family = [wanted, "Cascadia Mono", "Consolas", "Courier New"]
        .into_iter()
        .find(|f| names.iter().any(|n| n == f))
        .unwrap_or("Courier New")
        .to_string();
    let mut font = gpui::font(family);
    // CJK / emoji / symbols the terminal font lacks (WT `font.fallbacks` defaults).
    let fallbacks: Vec<String> = ["Segoe UI Emoji", "Segoe UI Symbol", "Microsoft YaHei UI", "Yu Gothic UI", "Malgun Gothic"]
        .into_iter()
        .filter(|f| names.iter().any(|n| n == f))
        .map(String::from)
        .collect();
    if !fallbacks.is_empty() {
        font.fallbacks = Some(gpui::FontFallbacks::from_fonts(fallbacks));
    }
    font
}

/// Canvas element showing `core`'s buffer; resizes the core to fit its bounds.
pub fn terminal_canvas(input: PaintInput) -> impl IntoElement {
    canvas(
        |bounds, _, _| bounds,
        move |bounds, _, window, cx| {
            // Text input (IME commit, Unicode, composed Vietnamese…) goes here.
            window.handle_input(&input.focus, ElementInputHandler::new(bounds, input.entity.clone()), cx);
            let font = pick_font(window, &input.font_family);
            let font_size = px(input.font_px);
            let line_h = px((input.font_px * LINE_HEIGHT_FACTOR).round());
            let ts = window.text_system().clone();
            let cell_w = ts
                .advance(ts.resolve_font(&font), font_size, 'M')
                .map(|s| s.width)
                .unwrap_or(px(input.font_px * 0.6));

            let cols = (f32::from(bounds.size.width) / f32::from(cell_w)).floor().max(1.0) as u16;
            let rows = (f32::from(bounds.size.height) / f32::from(line_h)).floor().max(1.0) as u16;

            let mut core = input.core.lock();
            if let Err(e) = core.size_changed(rows, cols) {
                log::warn!("terminal resize failed: {e}");
            }
            window.paint_quad(fill(bounds, rgb(BG)));
            let buf = &core.engine.dispatch.terminal;
            let top_abs = input.view.top_abs(buf);
            let top_id = buf.scrollback_dropped + top_abs;
            input.metrics.set(Metrics { origin: bounds.origin, cell_w, line_h, top_id, cols, rows });

            let grid = Grid { origin: bounds.origin, cell_w, line_h, font_size, cols: cols as usize };
            let palette = Palette::default();
            let selection = input.view.selection.map(|s| s.ordered(buf));
            let (matches, current) = match &input.view.search {
                Some(s) => (s.matches.as_slice(), s.current),
                None => (&[][..], None),
            };
            for i in 0..rows as usize {
                let abs = top_abs + i;
                let Some(row) = buf.row_abs(abs) else { continue };
                let id = buf.scrollback_dropped + abs;
                let mut spans: Vec<Span> = Vec::new();
                if let Some((a, b)) = selection {
                    if id >= a.id && id <= b.id {
                        let start = if id == a.id { a.col } else { 0 };
                        let end = if id == b.id { b.col.saturating_add(1) } else { usize::MAX };
                        spans.push(selection_span(start, end));
                    }
                }
                // Matches are sorted by row id: skip to this row.
                let first = matches.partition_point(|m| m.id < id);
                for m in matches[first..].iter().take_while(|m| m.id == id) {
                    spans.push(match_span(m.col, m.col + m.len, current == Some(m.group)));
                }
                paint_row(window, cx, &grid, i, row, &palette, &font, &spans);
            }

            // Scrollbar thumb (only when there is history to scroll through).
            let total = buf.total_rows();
            if total > rows as usize {
                let h = f32::from(bounds.size.height);
                let thumb_h = (h * rows as f32 / total as f32).max(20.0);
                let thumb_y = (h - thumb_h) * (top_abs as f32 / (total - rows as usize) as f32);
                let thumb = Bounds::new(
                    point(bounds.origin.x + bounds.size.width - px(6.), bounds.origin.y + px(thumb_y)),
                    size(px(6.), px(thumb_h)),
                );
                window.paint_quad(fill(thumb, Hsla { a: 0.28, ..rgb(0xFFFFFF).into() }));
            }

            // IME composition: boxed, underlined text starting at the cursor.
            if let Some(pre) = input.preedit.as_deref().filter(|p| !p.is_empty()) {
                let (x, y) = buf.get_cursor().get_position();
                draw_preedit(window, cx, &grid, &font, pre, x.max(0) as usize, y.max(0) as usize);
            }

            // Cursor: only on the live page, honouring blink and shape.
            let cursor = buf.get_cursor();
            if input.view.is_live() && cursor.is_visible() && (input.view.cursor_on || !input.focused || !cursor.is_blinking()) {
                let (cx_, cy) = cursor.get_position();
                draw_cursor(window, &grid, cx_.max(0) as usize, cy.max(0) as usize, cursor.get_type(), cursor.get_size(), input.focused);
            }
        },
    )
    .size_full()
}

/// Draw `text` cell by cell from `(col, row)`, over a lifted background and
/// underlined, so the user sees what the IME is composing.
fn draw_preedit(window: &mut Window, cx: &mut gpui::App, g: &Grid, font: &Font, text: &str, mut col: usize, row: usize) {
    use unicode_width::UnicodeWidthChar;
    let ts = window.text_system().clone();
    let fg: Hsla = rgb(0xFFFFFF).into();
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
        if col + w > g.cols {
            break;
        }
        let origin = point(g.origin.x + g.cell_w * col as f32, g.origin.y + g.line_h * row as f32);
        window.paint_quad(fill(Bounds::new(origin, size(g.cell_w * w as f32, g.line_h)), rgb(0x3A3A3A)));
        let run = gpui::TextRun {
            len: ch.len_utf8(),
            font: font.clone(),
            color: fg,
            background_color: None,
            underline: Some(gpui::UnderlineStyle { thickness: px(1.), color: Some(fg), wavy: false }),
            strikethrough: None,
        };
        let line = ts.shape_line(ch.to_string().into(), g.font_size, &[run], Some(g.cell_w * w as f32));
        let _ = line.paint(origin, g.line_h, gpui::TextAlign::Left, None, window, cx);
        col += w;
    }
}

fn draw_cursor(window: &mut Window, g: &Grid, col: usize, row: usize, shape: CursorShape, size_pct: u32, focused: bool) {
    let cell = Bounds::new(
        point(g.origin.x + g.cell_w * col as f32, g.origin.y + g.line_h * row as f32),
        size(g.cell_w, g.line_h),
    );
    let color: Hsla = rgb(0xFFFFFF).into();
    if !focused {
        // Unfocused windows show a hollow box regardless of the shape.
        window.paint_quad(outline(cell, color, BorderStyle::Solid));
        return;
    }
    let bar = |h: Pixels| Bounds::new(point(cell.origin.x, cell.origin.y + cell.size.height - h), size(cell.size.width, h));
    match shape {
        CursorShape::VerticalBar => {
            window.paint_quad(fill(Bounds::new(cell.origin, size(px(2.), cell.size.height)), color));
        }
        CursorShape::Underscore => window.paint_quad(fill(bar(px(2.)), color)),
        CursorShape::DoubleUnderscore => {
            window.paint_quad(fill(bar(px(1.)), color));
            window.paint_quad(fill(Bounds::new(point(cell.origin.x, cell.origin.y + cell.size.height - px(3.)), size(cell.size.width, px(1.))), color));
        }
        CursorShape::EmptyBox => window.paint_quad(outline(cell, color, BorderStyle::Solid)),
        CursorShape::FullBox => window.paint_quad(fill(cell, Hsla { a: 0.75, ..color })),
        CursorShape::Legacy => {
            let h = cell.size.height * (size_pct.clamp(1, 100) as f32 / 100.0);
            window.paint_quad(fill(bar(h), Hsla { a: 0.75, ..color }));
        }
    }
}
