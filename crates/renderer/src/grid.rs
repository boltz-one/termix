//! Grid-level render data — port verbatim `atlas/common.h`. Phase 3 fill
//! shaping/glyph map per `_mapComplex` (advance clamping) via `gpui::TextSystem`.
//!
//! Mapping COM handles sang GPUI:
//! - `IDWriteFontFace2 *` → `gpui::Font` (resolved bởi `tg_font`).
//! - `Buffer<T, N>` (WIL constexpr-aligned) → `Vec<T>`.
//! - `til::generation_t` → `std::sync::atomic::AtomicU32` (xem `payload.rs`).
//!
//! `RenderingPayload` tách ra `payload.rs` để giữ file <200 LOC.

use crate::cell::GridLineSet;

/// Antialiasing — port `atlas/common.h:334-339`. 1:1 với
/// `settings::TextAntialiasingMode` nhưng giữ enum riêng (không phụ thuộc serde path).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AntialiasingMode {
    Cleartype,
    #[default]
    Grayscale,
    Aliased,
}

// `FontRelevantAttributes` — port `common.h:429-435`, WT bitset `Bold | Italic`.
// Dùng `bitflags!` 2.x (struct bitset trực tiếp).
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FontRelevantAttributes: u8 {
        /// WT `FontRelevantAttributes::Bold = 0x1`.
        const BOLD = 0x1;
        /// WT `FontRelevantAttributes::Italic = 0x2`.
        const ITALIC = 0x2;
    }
}

/// `FontMapping` — port `common.h:437`. WT:
/// `struct FontMapping { IDWriteFontFace2* font; u32 glyphsFrom; u32 glyphsTo; }`.
/// COM handle được thay bằng `gpui::Font`.
#[derive(Clone, Debug, Default)]
pub struct FontMapping {
    /// Font đã resolved (chính / fallback).
    pub font: Option<gpui::Font>,
    /// Index `glyphs_from` — range glyph trong `glyph_indices` thuộc mapping này.
    pub glyphs_from: u32,
    /// Index `glyphs_to` (exclusive).
    pub glyphs_to: u32,
}

/// `GridLineRange` — port `common.h:444`. WT:
/// `struct GridLineRange { GridLineSet lines; u16 from; u16 to; }`.
#[derive(Clone, Debug, Default)]
pub struct GridLineRange {
    /// Set các lines áp dụng.
    pub lines: GridLineSet,
    /// Cột bắt đầu (inclusive).
    pub from: u16,
    /// Cột kết thúc (exclusive).
    pub to: u16,
}

/// `Bitmap` — port `common.h:451`. WT: `{ u64 revision; ID2D1Bitmap2* source;
/// SizeI sourceSize; PointI targetOffset; SizeI targetWidth; bool active; }`.
/// Phase 1 `source` = `Option<u64>` (id placeholder); phase 2 → `gpui::RenderImage`.
#[derive(Clone, Debug, Default)]
pub struct Bitmap {
    pub revision: u64,
    /// Placeholder image-id; phase 2 thay bằng `gpui::RenderImage` handle.
    pub source: Option<u64>,
    pub source_size: (u16, u16),
    pub target_offset: (i16, i16),
    pub target_width: (u16, u16),
    pub active: bool,
}

/// `ShapedRow` — port field-for-field `atlas/common.h:466`. WT fields: `mappings`,
/// `glyph_indices`, `glyph_advances`, `glyph_offsets`, `colors`, `bitmap`,
/// `grid_line_ranges`, `line_rendition`, `dirty_top`, `dirty_bottom`.
/// `GlyphOffset` (WT `DWRITE_GLYPH_OFFSET`) port sang tuple `(f32, f32)`.
#[derive(Clone, Debug, Default)]
pub struct ShapedRow {
    /// Map glyph indices → font (chính + fallback).
    pub mappings: Vec<FontMapping>,
    /// Glyph indices (DWrite glyph IDs).
    pub glyph_indices: Vec<u16>,
    /// Advance per glyph trong px.
    pub glyph_advances: Vec<f32>,
    /// `(advance_offset, ascender_offset)` per glyph.
    pub glyph_offsets: Vec<(f32, f32)>,
    /// Color packed RGBA per glyph (fg straight trong WT color_bitmap slab 1).
    pub colors: Vec<u32>,
    /// Cached bitmap row (built-in/box-drawing glyphs phase 2).
    pub bitmap: Bitmap,
    /// GridLineRanges apply cho row này.
    pub grid_line_ranges: Vec<GridLineRange>,
    /// Line rendition (single/double-width/height) — ASSUMED `tg_core` enum.
    pub line_rendition: LineRendition,
    /// Top dirty y (px) — WT `dirtyTop`.
    pub dirty_top: u16,
    /// Bottom dirty y (px) — WT `dirtyBottom`.
    pub dirty_bottom: u16,
}

/// `LineRendition` — port enum từ `LineRendition.hpp:16` (đã có trong Lane A).
/// `// ASSUMED core API — rework after Lane A finalizes`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineRendition {
    #[default]
    SingleWidthSingleHeight,
    DoubleWidthSingleHeight,
    SingleWidthDoubleHeightTop,
    SingleWidthDoubleHeightBottom,
}

/// `ShapeCtx` — bundle các input shape-only (font + sizing + text-system)
/// trong `_mapComplex` (`AtlasEngine.cpp:1040-1173`). Tách thành struct để
/// `ShapedRow::shape_row` giữ ≤7 args (clippy:too_many_arguments) và để caller
/// reuse một ctx qua nhiều row (DRY).
#[derive(Clone)]
pub struct ShapeCtx<'a> {
    /// Resolved `gpui::Font` từ `font::FontRuntime`.
    pub font: gpui::Font,
    /// Pixel size (`FontRuntime::font_size`).
    pub font_size: f32,
    /// WT `_p.s->font->cellSize.x`.
    pub cell_size_x: f32,
    /// gpui text system handle.
    pub text_system: &'a gpui::WindowTextSystem,
}

impl ShapedRow {
    /// Reset row về trạng thái empty tại `y`, height `cell_height` — port
    /// verbatim `ShapedRow::Clear(y, cellHeight)` tại `common.h:466`. WT clear
    /// `glyph_*`/`colors`/`grid_line_ranges`/`mappings`, set `bitmap.active=false`,
    /// `line_rendition=Single...`, `dirty_top=y`, `dirty_bottom=y+cellHeight`.
    pub fn clear(&mut self, y: u16, cell_height: u16) -> &mut Self {
        self.mappings.clear();
        self.glyph_indices.clear();
        self.glyph_advances.clear();
        self.glyph_offsets.clear();
        self.colors.clear();
        self.grid_line_ranges.clear();
        self.bitmap.active = false;
        self.line_rendition = LineRendition::SingleWidthSingleHeight;
        self.dirty_top = y;
        self.dirty_bottom = y.saturating_add(cell_height);
        self
    }

    /// Shape row bằng `gpui::TextSystem::shape_line` rồi **reproduce
    /// advance clamping** từ WT `_mapComplex` (`AtlasEngine.cpp:1040-1173`).
    ///
    /// Cấp 1: gpui shaping theo layout thật của platform (DirectWrite hay
    /// core-text). Cấp 2: ép cluster advances → `(col2 - col1) *
    /// cell_size.x` để mỗi cell-grid column chiếm exactly `cell_size.x` px
    /// (mimic DWrite monospace cell-grid).
    ///
    /// **Port verbatim `_mapComplex` advance clamping**:
    /// ```cpp
    /// const auto expectedAdvance = (col2 - col1) * _p.s->font->cellSize.x;
    /// f32 actualAdvance = 0;
    /// for (j = prevCluster; j < nextCluster; ++j)
    ///     actualAdvance += _api.glyphAdvances[j];
    /// _api.glyphAdvances[nextCluster - 1] += expectedAdvance - actualAdvance;
    /// ```
    ///
    /// Inputs:
    /// - `text`: text content (UTF-8) của row.
    /// - `col_map`: `[usize; text.len() + 1]` — column index tại mỗi byte
    ///   boundary, computed from `Row::glyphs().columns`.
    /// - `fg_color`: packed ABGR per-glyph color (same for whole row;
    ///   per-cluster colors done by caller splitting the run).
    /// - `font`: resolved `gpui::Font` từ `font::FontRuntime`.
    /// - `font_size`: pixel size (`FontRuntime::font_size`).
    /// - `cell_size_x`: WT `_p.s->font->cellSize.x`.
    /// - `text_system`: gpui text system handle.
    ///
    /// Caller (`Renderer::paint_frame_for_engine`) must `clear()` trước khi
    /// shape successive rows,或缺省 reuse pattern.
    pub fn shape_row(
        &mut self,
        text: &str,
        col_map: &[usize],
        fg_color: u32,
        ctx: &ShapeCtx,
    ) -> anyhow::Result<()> {
        use gpui::{Pixels, SharedString, TextRun};
        let font = &ctx.font;
        let font_size = ctx.font_size;
        let cell_size_x = ctx.cell_size_x;
        let text_system = &ctx.text_system;

        // Clear writes; preserves `y`/`dirty_*` of caller set.
        self.glyph_indices.clear();
        self.glyph_advances.clear();
        self.glyph_offsets.clear();
        self.colors.clear();
        self.mappings.clear();

        let text_string: SharedString = text.into();
        let run = TextRun {
            len: text.len(),
            font: font.clone(),
            color: gpui::hsla(0.0, 0.0, 0.0, 0.0), // actual color applied via `colors[]` array.
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = text_system.shape_line(
            text_string,
            Pixels::from(font_size),
            &[run],
            // Force width: leave to clamping (port verbatim _mapComplex).
            None,
        );

        // Walk layout runs → gather glyphs with their (pos_x, byte_index).
        // Compute per-glyph advance by position.x deltas within a run; for
        // the last glyph in a run we fall back to run width minus its
        // position; safer approach used here: track next glyph's position.
        struct LaidGlyph {
            id: u32,
            offset: f32, // advance_offset (offset_x from shaping)
            ascender_offset: f32,
            advance: f32,
            byte_index: usize,
        }
        let mut laid: Vec<LaidGlyph> = Vec::new();
        for run in shaped.runs.iter() {
            let n = run.glyphs.len();
            for (i, g) in run.glyphs.iter().enumerate() {
                let next_x = if i + 1 < n {
                    run.glyphs[i + 1].position.x
                } else {
                    run.glyphs
                        .get(i)
                        .map(|g2| g2.position.x)
                        .unwrap_or_else(|| g.position.x)
                };
                let advance = f32::from((next_x - g.position.x).max(Pixels::from(0.0)));
                laid.push(LaidGlyph {
                    id: g.id.0,
                    offset: f32::from(g.position.x),
                    ascender_offset: f32::from(g.position.y),
                    advance,
                    byte_index: g.index,
                });
            }
            // Run width is pre-computed; if our last-glyph advance is 0
            // (no next glyph), use run width minus last position.
            if let Some(last) = run.glyphs.last() {
                let advance = f32::from((shaped.width - last.position.x).max(Pixels::from(0.0)));
                if let Some(g) = laid.last_mut() {
                    if g.advance == 0.0 {
                        g.advance = advance;
                    }
                }
            }
        }

        // Group glyphs by byte_index (= cluster boundary). Within a group,
        // sum actualAdvance; expectedAdvance = (col2 - col1) * cell_size_x;
        // clamp last glyph's advance: `+= expectedAdvance - actualAdvance`.
        // Port verbatim `AtlasEngine.cpp:1167-1173`.
        let mut i = 0usize;
        let zero_offset = (0.0f32, 0.0f32);
        while i < laid.len() {
            let cluster_byte_start = laid[i].byte_index;
            let mut j = i;
            while j < laid.len() && laid[j].byte_index == cluster_byte_start {
                j += 1;
            }
            let cluster_end_byte = if j < laid.len() {
                laid[j].byte_index
            } else {
                text.len()
            };
            // Column lookup — col_map indexed by byte position.
            let col1 = col_map.get(cluster_byte_start).copied().unwrap_or(0);
            let col2 = col_map.get(cluster_end_byte).copied().unwrap_or(col1);
            let expected_advance = (col2.saturating_sub(col1)) as f32 * cell_size_x;

            // Compute actual advance for cluster [i..j).
            let mut actual_advance = 0.0f32;
            for g in &laid[i..j] {
                actual_advance += g.advance;
            }
            // Clamp last-glyph advance so cluster sum = expectedAdvance.
            // Port WT: `_api.glyphAdvances[nextCluster - 1] += expectedAdvance - actualAdvance`.
            if j > i {
                let delta = expected_advance - actual_advance;
                laid[j - 1].advance += delta;
            }

            // Emit glyphs of this cluster.
            for (k, g) in laid[i..j].iter().enumerate() {
                self.glyph_indices.push(g.id as u16);
                self.glyph_advances.push(g.advance);
                // offset = relative to baseline start of cluster. GPUI
                // positions are absolute; shift to relative-advance form
                // expected by WT `DWRITE_GLYPH_OFFSET`. Use advanceOffset
                // per-glyph position relative to cluster prev- glyph.
                let off_x = if k == 0 {
                    g.offset
                } else {
                    let prev = &laid[i + k - 1];
                    g.offset - prev.offset - prev.advance
                };
                self.glyph_offsets.push((off_x, g.ascender_offset));
                self.colors.push(fg_color);
            }

            i = j;
        }

        // Single FontMapping covering whole row — phase 3 split when
        // multiple fonts/fallbacks involved (`BuiltinGlyphs`/Powerline).
        self.mappings.push(FontMapping {
            font: Some(font.clone()),
            glyphs_from: 0,
            glyphs_to: self.glyph_indices.len() as u32,
        });

        // Default offset baseline for whole row (no shaping offset).
        let _ = zero_offset;
        Ok(())
    }
}

/// `RenderingPayload` đã tách ra `payload.rs` (port `atlas/common.h:501`).
pub use crate::payload::RenderingPayload;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::GridLines;

    #[test]
    fn font_relevant_attributes_bold_italic_combos() {
        let none = FontRelevantAttributes::empty();
        assert!(none.is_empty());
        let both = FontRelevantAttributes::BOLD | FontRelevantAttributes::ITALIC;
        assert!(both.contains(FontRelevantAttributes::BOLD));
        assert!(both.contains(FontRelevantAttributes::ITALIC));
    }

    #[test]
    fn shaped_row_clear_resets_all_fields_and_sets_dirty_range() {
        let mut row = ShapedRow {
            glyph_indices: vec![1, 2, 3],
            glyph_advances: vec![10.0, 20.0],
            glyph_offsets: vec![(0.5, 0.5)],
            colors: vec![0xFFFFFFFF; 3],
            grid_line_ranges: vec![GridLineRange {
                lines: GridLines::TOP,
                from: 0,
                to: 80,
            }],
            dirty_top: 5,
            dirty_bottom: 25,
            ..ShapedRow::default()
        };
        row.clear(100, 20);
        assert!(row.glyph_indices.is_empty());
        assert!(row.glyph_advances.is_empty());
        assert!(row.glyph_offsets.is_empty());
        assert!(row.colors.is_empty());
        assert!(row.grid_line_ranges.is_empty());
        assert!(!row.bitmap.active);
        assert_eq!(row.line_rendition, LineRendition::SingleWidthSingleHeight);
        assert_eq!(row.dirty_top, 100);
        assert_eq!(row.dirty_bottom, 120);
    }

    #[test]
    fn grid_line_range_default_is_empty() {
        let r = GridLineRange::default();
        assert!(r.lines.is_empty());
        assert_eq!(r.from, 0);
        assert_eq!(r.to, 0);
    }

    #[test]
    fn antialiasing_default_is_grayscale() {
        assert_eq!(AntialiasingMode::default(), AntialiasingMode::Grayscale);
    }

    #[test]
    fn line_rendition_default_single() {
        assert_eq!(
            LineRendition::default(),
            LineRendition::SingleWidthSingleHeight
        );
    }
}
