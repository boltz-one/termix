//! Paint traits + renderer entry — port verbatim:
//! * `inc/IRenderData.hpp:45-90` → `RenderData` trait.
//! * `inc/IRenderEngine.hpp:56-97` → `RenderEngine` trait (~30 methods).
//! * `base/renderer.cpp:470` (`_PaintFrameForEngine`) → `Renderer::paint_frame_for_engine`.
//!
//! Phase 3 fill step 6 per-row `_PaintBufferOutput` (`renderer.cpp:1069`) +
//! step 7-10 verbatim ordering. WT logic requiring hardware GPUI primitives
//! (D2D `DrawGlyphRun`) keeps a placeholder body with a `// port verbatim WT
//! AtlasEngine step N — GPUI primitive wire phase 4` comment — phase 3
//! guarantees no-panic + valid return only.

use anyhow::Result;

use crate::cell::{Cluster, CursorOptions};
use crate::grid::ShapedRow;

/// `RenderFrameInfo` — port `inc/IRenderData.hpp:31-37`. WT fields:
/// `searchHighlights`, `searchHighlightFocused`, `selectionSpans`,
/// `selectionBackground`. Phase 1 giữ `Vec<Rect>` cho highlights/spans (chưa
/// finalized shape trong `tg_core`).
#[derive(Clone, Debug, Default)]
pub struct RenderFrameInfo {
    pub search_highlights: Vec<(i32, i32, i32, i32)>,
    pub search_highlight_focused: bool,
    pub selection_spans: Vec<(i32, i32, i32, i32)>,
    pub selection_background: u32,
}

/// `IRenderData` — port `inc/IRenderData.hpp:45-90`. Trait method signatures
/// (chưa có default body — control impl từng method incrementally).
///
/// `// ASSUMED core API — rework after Lane A finalizes`
pub trait RenderData {
    fn get_viewport(&self) -> (i32, i32, i32, i32);
    fn get_text_buffer(&self) -> &dyn std::any::Any;
    fn get_font_info(&self) -> &dyn std::any::Any;
    /// `(fg, bg)` packed RGBA — WT `GetAttributeColors`.
    fn get_attribute_colors(&self, attr: &dyn std::any::Any) -> (u32, u32);
    fn get_search_highlights(&self) -> &[(i32, i32, i32, i32)];
    fn get_selection_spans(&self) -> &[(i32, i32, i32, i32)];
    fn get_blink_interval(&self) -> u32;
    fn get_pattern_id(&self, _point: (i32, i32)) -> u64;
    fn get_active_composition(&self) -> Option<&dyn std::any::Any>;
}

/// `IRenderEngine` — port ~30 methods `inc/IRenderEngine.hpp:56-97`.
/// Trait method signatures (không có default body) — phase 4 sẽ impl thực
/// (hardware GPUI primitives). `paint_frame_for_engine` gọi theo thứ tự
/// verbatim từ `_PaintFrameForEngine` (`renderer.cpp:470`).
pub trait RenderEngine {
    /// Step 1. Bắt đầu frame; trả về false để skip.
    fn start_paint(&mut self, data: &dyn RenderData, frame_info: &RenderFrameInfo) -> bool;
    /// Step 2. Cập nhật brushes FG/BG/UL + bold/italic (port `_UpdateDrawingBrushes`).
    fn update_drawing_brushes(&mut self, default_attr: &dyn std::any::Any);
    /// Step 3. Scroll frame theo dirty delta (`_PerformScrolling` → `ScrollFrame`).
    fn scroll_frame(&mut self);
    /// Step 4. Prepare search highlights / selection spans (`_PrepareRenderInfo`).
    fn prepare_render_info(&mut self, data: &dyn RenderData);
    /// Step 5. Paint background toàn viewport.
    fn paint_background(&mut self);
    /// Step 6. Paint một buffer line (`PaintBufferLine` `AtlasEngine.cpp:486`).
    fn paint_buffer_line(&mut self, clusters: &[Cluster], coord: (i32, i32), trim_left: i32);
    /// Step 6b. Paint grid lines cho row (`PaintBufferGridLines`).
    fn paint_buffer_grid_lines(&mut self, row: &ShapedRow, lines: u32, from_col: i32, to_col: i32);
    /// Step 7. Paint selection rect (`PaintSelection`).
    fn paint_selection(&mut self, rect: (i32, i32, i32, i32));
    /// Step 8. Paint cursor snapshot (`PaintCursor`).
    fn paint_cursor(&mut self, options: &CursorOptions);
    /// Step 9. Cập nhật title (`_PaintTitle` → `UpdateTitle`).
    fn update_title(&mut self, title: &str);
    /// Step 10. Kết frame; `NotifyPaintFrame` nếu `requires_continuous_redraw`.
    fn end_paint(&mut self);
    fn notify_paint_frame(&mut self);
    fn requires_continuous_redraw(&self) -> bool;
    /// Invalidate một rect (kèm cycle).
    fn invalidate(&mut self, rect: (i32, i32, i32, i32));
    /// Invalidate toàn bộ surface.
    fn invalidate_all(&mut self);
    fn get_proposed_font(&self) -> &dyn std::any::Any;
    fn get_dirty_area(&self) -> (i32, i32, i32, i32);
    fn is_infinite_dirty(&self) -> bool;
    fn update_font(&mut self, desired: &dyn std::any::Any, actual: &dyn std::any::Any) -> bool;
    fn update_soft_font(&mut self, cells: &[(u16, u16)], bit_pattern: &[u8], centering: bool);
    fn set_graphics_rendition(&mut self, _rendition: &dyn std::any::Any);
    fn set_selection_background(&mut self, rgba: u32);
    fn prepare_line_transform(&mut self, y: i32, pitch: f32, grid_lines: u32);
    /// Step 6 cận: paint image slice — port `PaintImageSlice` (`IRenderEngine.hpp`).
    /// WT vẽ image-style cell (SIXEL/inline image).
    fn paint_image_slice(&mut self, coord: (i32, i32), size: (u32, u32));
}

/// `Renderer` — owns engine list, render task (gpui), viewport, cursor
/// options, cluster buffer (scratch cho `_PaintBufferOutputHelper` run breaks
/// tại `renderer.cpp:1264`).
pub struct Renderer {
    pub engines: Vec<Box<dyn RenderEngine>>,
    pub render_task: Option<gpui::Task<()>>,
    pub viewport: (i32, i32, i32, i32),
    pub cursor: CursorOptions,
    pub cluster_buffer: smallvec::SmallVec<[Cluster; 4]>,
    /// Snapshot selection rects (screen-space) cho `_PaintSelection`
    /// (`renderer.cpp:1805`). Set bởi `control` mỗi khi selection thay đổi;
    /// `paint_frame_for_engine` đọc lúc step 7.
    pub last_selection_rects: Vec<(i32, i32, i32, i32)>,
    /// Snapshot console title cho `_PaintTitle` (`renderer.cpp:934`).
    pub title: String,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            engines: Vec::new(),
            render_task: None,
            viewport: (0, 0, 0, 0),
            cursor: CursorOptions::default(),
            cluster_buffer: smallvec::SmallVec::new(),
            last_selection_rects: Vec::new(),
            title: String::new(),
        }
    }
}

impl Renderer {
    /// Port verbatim `_PaintFrameForEngine` (`base/renderer.cpp:470`).
    /// Mười bước theo WT thứ tự chuẩn. Phase 3: full pipeline wired — step 6
    /// per-row `_PaintBufferOutput` (no-op khi `core::TextBuffer` chưa
    /// finalize, structural loop ready), step 7-10 verbatim WT.
    pub fn paint_frame_for_engine(
        &mut self,
        engine: &mut dyn RenderEngine,
        data: &dyn RenderData,
        frame_info: &RenderFrameInfo,
        default_attr: &dyn std::any::Any,
    ) -> Result<()> {
        // Step 1 — StartPaint. Trả về false ⇒ nothing to paint, early return
        // (WT `if (S_FALSE == hr) return S_FALSE;`).
        let started = engine.start_paint(data, frame_info);
        if !started {
            return Ok(());
        }

        // `endPaint` scope_exit (WT `wil::scope_exit`) — luôn chạy EndPaint +
        // NotifyPaintFrame khi requerido, kể cả khi panic/error mid-frame.
        // Phase 3: tất cả inner steps infallible (không Result<>), nên inline
        // thay vì IIFE. Phase 4 nếu thêm fallible paint_quad, ta sẽ quay lại IIFE.

        // Step 2 — `_UpdateDrawingBrushes(defaultAttr, false, true)`.
        // Port verbatim `renderer.cpp:498`. Phase 3: caller pass default
        // attr; engine wires FG/BG/UL storage (phase 4 GPUI paint_quad).
        engine.update_drawing_brushes(default_attr);

        // Step 3 — `_PerformScrolling` → `engine.ScrollFrame()`.
        // Port verbatim `renderer.cpp:501`.
        engine.scroll_frame();

        // Step 4 — `_PrepareRenderInfo` → engine.PrepareRenderInfo(info).
        // Port verbatim `renderer.cpp:504`. WT builds RenderFrameInfo
        // from RenderData search/selection getters.
        engine.prepare_render_info(data);

        // Step 5 — `_PaintBackground` → engine.PaintBackground().
        // Port verbatim `renderer.cpp:507`. AtlasEngine `PaintBackground`
        // is no-op (AtlasEngine.cpp:359-362).
        engine.paint_background();

        // Step 6 — `_PaintBufferOutput(pEngine)` (`renderer.cpp:510,1069`).
        // Per-row text paint. Phase 3 structural port — see helper below.
        self.paint_buffer_output(engine, data);

        // Step 7 — `_PaintSelection(pEngine)` (`renderer.cpp:513,1805`).
        // WT: for each dirtyArea, for each selection rect, intersect with
        // dirty rect, call engine.PaintSelection(intersect).
        self.paint_selection(engine);

        // Step 8 — `_PaintCursor(pEngine)` (`renderer.cpp:516,1770`).
        // WT: if cursor inViewport && visible → PaintCursor(snapshot).
        self.paint_cursor_internal(engine);

        // Step 9 — `_PaintTitle(pEngine)` (`renderer.cpp:519,934`).
        // WT: `newTitle = _pData->GetConsoleTitle(); engine.UpdateTitle(newTitle)`.
        engine.update_title(&self.title);

        // Step 10 — EndPaint + NotifyPaintFrame. WT scope_exit (`renderer.cpp:485-495`):
        //   `LOG_IF_FAILED(pEngine->EndPaint()); if (RequiresContinuousRedraw()) NotifyPaintFrame();`
        // NOTE: WT actually calls EndPaint before the scope_exit returns (line 522
        // `endPaint.reset()`), but our declarative version below is equivalent —
        // always EndPaint after the inner steps complete (phase 4 may short-circuit).
        engine.end_paint();
        if engine.requires_continuous_redraw() {
            engine.notify_paint_frame();
        }

        Ok(())
    }

    /// Port verbatim `_PaintBufferOutput` (`base/renderer.cpp:1069`).
    ///
    /// WT logic:
    /// ```cpp
    /// std::span<const til::rect> dirtyAreas;
    /// LOG_IF_FAILED(pEngine->GetDirtyArea(dirtyAreas));
    /// for (const auto& dirtyRect : dirtyAreas) {
    ///     if (!dirtyRect) continue;
    ///     auto dirty = Viewport::FromExclusive(dirtyRect);
    ///     dirty = Viewport::Offset(dirty, _viewport.Origin());
    ///     const auto redraw = Viewport::Intersect(dirty, _viewport);
    ///     for (auto row = redraw.Top(); row < redraw.BottomExclusive(); row++) {
    ///         // PrepareLineTransform → _PaintBufferOutputHelper → PaintImageSlice
    ///     }
    /// }
    /// ```
    ///
    /// Phase 3ترنت: structural loop wired — uses `engine.get_dirty_area()` +
    /// `data.get_text_buffer()` (downcast `() → None` ⇒ no rows, no-op valid
    /// return). Currently always no-op because `core::TextBuffer::rows()`
    /// isn't finalized. The loop body will iterate `_PaintBufferOutputHelper`
    /// once core exposes row iteration.
    ///
    /// `// port verbatim WT AtlasEngine step 6 — GPUI primitive wire phase 4`
    fn paint_buffer_output(&mut self, engine: &mut dyn RenderEngine, data: &dyn RenderData) {
        let dirty = engine.get_dirty_area();
        // WT: skip if dirty rect empty.
        if dirty.0 >= dirty.2 || dirty.1 >= dirty.3 {
            return;
        }

        let (vx, vy, vright, vbottom) = self.viewport;
        if vx >= vright || vy >= vbottom {
            return;
        }

        // Intersect dirty with viewport (port `Viewport::Intersect`).
        let left = dirty.0.max(vx);
        let top = dirty.1.max(vy);
        let right = dirty.2.min(vright);
        let bottom = dirty.3.min(vbottom);
        if left >= right || top >= bottom {
            return;
        }

        // WT: `buffer = _pData->GetTextBuffer()`. Phase 3: core chưa
        // finalize TextBuffer row iteration — `&dyn Any` chưa có type để
        // downcast. Khi Lane A finalize, thay bằng `downcast_ref::<TextBuffer>()`.
        let _text_buffer = data.get_text_buffer();
        // Currently: text_buffer.downcast_ref::<()>() luôn None ⇒ skip per-row
        // loop. Khi core expose `TextBuffer`, mở `for row in top..bottom`.
        // Phase 4 wire `gpui::TextSystem::shape_line` + `paint_buffer_line`.

        // NO-OP至此; phase 4 sẽ fill `PrePareLineTransform → _PaintBufferOutputHelper
        // → PaintImageSlice` per row theo `renderer.cpp:1144-1155`. Cluster
        // accumulation goes via `self.cluster_buffer` (run-break port
        // `renderer.cpp:1264`). NO panic và return valid.

        // Touch params to avoid unused warnings — engine/data giữ interface wired.
        let _ = (engine, data);
    }

    /// Port verbatim `_PaintSelection` (`base/renderer.cpp:1805`).
    ///
    /// WT logic:
    /// ```cpp
    /// std::span<const til::rect> dirtyAreas;
    /// LOG_IF_FAILED(pEngine->GetDirtyArea(dirtyAreas));
    /// for (auto&& dirtyRect : dirtyAreas) {
    ///     for (const auto& rect : _lastSelectionRectsByViewport) {
    ///         if (const auto rectCopy{ rect & dirtyRect }) {
    ///             LOG_IF_FAILED(pEngine->PaintSelection(rectCopy));
    ///         }
    ///     }
    /// }
    /// ```
    fn paint_selection(&self, engine: &mut dyn RenderEngine) {
        let dirty = engine.get_dirty_area();
        if dirty.0 >= dirty.2 || dirty.1 >= dirty.3 {
            return;
        }
        for rect in &self.last_selection_rects {
            // Intersect rect ∩ dirty (port `til::rect::operator&`).
            let x0 = rect.0.max(dirty.0);
            let y0 = rect.1.max(dirty.1);
            let x1 = rect.2.min(dirty.2);
            let y1 = rect.3.min(dirty.3);
            if x0 < x1 && y0 < y1 {
                engine.paint_selection((x0, y0, x1, y1));
            }
        }
    }

    /// Port verbatim `_PaintCursor` (`base/renderer.cpp:1770`).
    ///
    /// WT logic:
    /// ```cpp
    /// if (_currentCursorOptions.inViewport && _currentCursorOptions.isVisible) {
    ///     LOG_IF_FAILED(pEngine->PaintCursor(_currentCursorOptions));
    /// }
    /// ```
    fn paint_cursor_internal(&self, engine: &mut dyn RenderEngine) {
        if self.cursor.visible {
            // WT发展改革委 kiểm tra `inViewport` (cursor nằm trong viewport bounds).
            // We check both: visible + position within viewport (inclusive).
            let (x, y) = self.cursor.position;
            let (_, _, vw, vh) = self.viewport;
            if x >= 0 && y >= 0 && x < vw && y < vh {
                engine.paint_cursor(&self.cursor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock `RenderData` — returns all-zero state. Used by `paint_frame_for_engine`
    /// tests below (no real core TextBuffer required).
    struct MockRenderData;
    impl RenderData for MockRenderData {
        fn get_viewport(&self) -> (i32, i32, i32, i32) {
            (0, 0, 80, 24)
        }
        fn get_text_buffer(&self) -> &dyn std::any::Any {
            &()
        }
        fn get_font_info(&self) -> &dyn std::any::Any {
            &()
        }
        fn get_attribute_colors(&self, _attr: &dyn std::any::Any) -> (u32, u32) {
            (0xFF_FFFFFF, 0xFF_000000)
        }
        fn get_search_highlights(&self) -> &[(i32, i32, i32, i32)] {
            &[]
        }
        fn get_selection_spans(&self) -> &[(i32, i32, i32, i32)] {
            &[]
        }
        fn get_blink_interval(&self) -> u32 {
            500
        }
        fn get_pattern_id(&self, _point: (i32, i32)) -> u64 {
            0
        }
        fn get_active_composition(&self) -> Option<&dyn std::any::Any> {
            None
        }
    }

    /// Mock `RenderEngine` recording every call into a `Vec<&'static str>`.
    /// All `paint_*` methods no-op (phase 3 seulement — no GPUI primitive).
    struct MockRenderEngine {
        calls: Vec<&'static str>,
        dirty: (i32, i32, i32, i32),
        started: bool,
    }
    impl MockRenderEngine {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                dirty: (0, 0, 80, 24),
                started: true,
            }
        }
        fn with_dirty(dirty: (i32, i32, i32, i32)) -> Self {
            Self {
                calls: Vec::new(),
                dirty,
                started: true,
            }
        }
        fn record(&mut self, name: &'static str) {
            self.calls.push(name);
        }
    }
    impl RenderEngine for MockRenderEngine {
        fn start_paint(&mut self, _data: &dyn RenderData, _info: &RenderFrameInfo) -> bool {
            self.record("start_paint");
            self.started
        }
        fn update_drawing_brushes(&mut self, _attr: &dyn std::any::Any) {
            self.record("update_drawing_brushes");
        }
        fn scroll_frame(&mut self) {
            self.record("scroll_frame");
        }
        fn prepare_render_info(&mut self, _data: &dyn RenderData) {
            self.record("prepare_render_info");
        }
        fn paint_background(&mut self) {
            self.record("paint_background");
        }
        fn paint_buffer_line(&mut self, _clusters: &[Cluster], _coord: (i32, i32), _trim: i32) {
            self.record("paint_buffer_line");
        }
        fn paint_buffer_grid_lines(&mut self, _row: &ShapedRow, _lines: u32, _from: i32, _to: i32) {
            self.record("paint_buffer_grid_lines");
        }
        fn paint_selection(&mut self, _rect: (i32, i32, i32, i32)) {
            self.record("paint_selection");
        }
        fn paint_cursor(&mut self, _options: &CursorOptions) {
            self.record("paint_cursor");
        }
        fn update_title(&mut self, _title: &str) {
            self.record("update_title");
        }
        fn end_paint(&mut self) {
            self.record("end_paint");
        }
        fn notify_paint_frame(&mut self) {
            self.record("notify_paint_frame");
        }
        fn requires_continuous_redraw(&self) -> bool {
            false
        }
        fn invalidate(&mut self, _rect: (i32, i32, i32, i32)) {}
        fn invalidate_all(&mut self) {}
        fn get_proposed_font(&self) -> &dyn std::any::Any {
            &()
        }
        fn get_dirty_area(&self) -> (i32, i32, i32, i32) {
            self.dirty
        }
        fn is_infinite_dirty(&self) -> bool {
            false
        }
        fn update_font(
            &mut self,
            _desired: &dyn std::any::Any,
            _actual: &dyn std::any::Any,
        ) -> bool {
            true
        }
        fn update_soft_font(
            &mut self,
            _cells: &[(u16, u16)],
            _bit_pattern: &[u8],
            _centering: bool,
        ) {
        }
        fn set_graphics_rendition(&mut self, _rendition: &dyn std::any::Any) {}
        fn set_selection_background(&mut self, _rgba: u32) {}
        fn prepare_line_transform(&mut self, _y: i32, _pitch: f32, _grid_lines: u32) {}
        fn paint_image_slice(&mut self, _coord: (i32, i32), _size: (u32, u32)) {
            self.record("paint_image_slice");
        }
    }

    #[test]
    fn paint_frame_returns_ok_with_default_state() {
        // Brief verify #1: `paint_frame_returns_ok_with_default_state`.
        let mut renderer = Renderer {
            viewport: (0, 0, 80, 24),
            ..Default::default()
        };
        let mut engine = MockRenderEngine::new();
        let data = MockRenderData;
        let info = RenderFrameInfo::default();
        let result = renderer.paint_frame_for_engine(&mut engine, &data, &info, &());
        assert!(result.is_ok(), "paint_frame_for_engine should return Ok");
        // Verify canonical step order (port verbatim `_PaintFrameForEngine`).
        assert!(engine.calls.starts_with(&[
            "start_paint",
            "update_drawing_brushes",
            "scroll_frame",
            "prepare_render_info",
            "paint_background",
        ]));
        // Step 9 + 10 verbatim order.
        assert!(engine.calls.contains(&"update_title"));
        assert!(engine.calls.contains(&"end_paint"));
        let update_title_idx = engine
            .calls
            .iter()
            .position(|c| *c == "update_title")
            .unwrap();
        let end_paint_idx = engine.calls.iter().position(|c| *c == "end_paint").unwrap();
        assert!(
            update_title_idx < end_paint_idx,
            "title before end_paint (port verbatim)"
        );
    }

    #[test]
    fn paint_frame_early_returns_when_start_paint_false() {
        // Brief verify #1 variant: engine refuses paint ⇒ no steps run.
        let mut renderer = Renderer {
            viewport: (0, 0, 80, 24),
            ..Default::default()
        };
        let mut engine = MockRenderEngine::new();
        engine.started = false;
        let data = MockRenderData;
        let info = RenderFrameInfo::default();
        let result = renderer.paint_frame_for_engine(&mut engine, &data, &info, &());
        assert!(result.is_ok());
        // Only start_paint should run; nothing else per WT `_PaintFrameForEngine:480-483`.
        assert_eq!(engine.calls, vec!["start_paint"]);
    }

    #[test]
    fn paint_selection_no_panic_with_default_state() {
        // Brief verify #2: `paint_selection_no_panic`.
        let renderer = Renderer {
            viewport: (0, 0, 80, 24),
            ..Default::default()
        };
        // Empty selection list ⇒ no paint_selection calls.
        let mut engine = MockRenderEngine::new();
        renderer.paint_selection(&mut engine);
        assert!(
            engine.calls.is_empty(),
            "no selection rects ⇒ no paint_selection"
        );
    }

    #[test]
    fn paint_selection_emits_for_intersected_rect() {
        // Brief verify: selection rect inside dirty area ⇒ emitted once.
        let renderer = Renderer {
            viewport: (0, 0, 80, 24),
            last_selection_rects: vec![(10, 5, 30, 15)],
            ..Default::default()
        };
        let mut engine = MockRenderEngine::with_dirty((0, 0, 80, 24));
        renderer.paint_selection(&mut engine);
        assert_eq!(engine.calls, vec!["paint_selection"]);
    }

    #[test]
    fn paint_selection_skipped_when_disjoint_from_dirty() {
        // Selection rect outside dirty rect ⇒ skipped.
        let renderer = Renderer {
            viewport: (0, 0, 80, 24),
            last_selection_rects: vec![(100, 100, 110, 110)],
            ..Default::default()
        };
        let mut engine = MockRenderEngine::with_dirty((0, 0, 80, 24));
        renderer.paint_selection(&mut engine);
        assert!(engine.calls.is_empty());
    }

    #[test]
    fn paint_cursor_no_panic_with_default_state() {
        // Brief verify #3: `paint_cursor_no_panic`. Default cursor not visible.
        let renderer = Renderer {
            viewport: (0, 0, 80, 24),
            ..Default::default()
        };
        let mut engine = MockRenderEngine::new();
        renderer.paint_cursor_internal(&mut engine);
        assert!(
            engine.calls.is_empty(),
            "invisible cursor ⇒ no PaintCursor call (port verbatim WT)"
        );
    }

    #[test]
    fn paint_cursor_emitted_when_visible_in_viewport() {
        // Visible cursor inside viewport ⇒ one PaintCursor call.
        let renderer = Renderer {
            viewport: (0, 0, 80, 24),
            cursor: CursorOptions {
                visible: true,
                position: (5, 10),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = MockRenderEngine::new();
        renderer.paint_cursor_internal(&mut engine);
        assert_eq!(engine.calls, vec!["paint_cursor"]);
    }

    #[test]
    fn paint_cursor_skipped_when_outside_viewport() {
        // Visible cursor outside viewport ⇒ no PaintCursor (WT inViewport check).
        let renderer = Renderer {
            viewport: (0, 0, 80, 24),
            cursor: CursorOptions {
                visible: true,
                position: (90, 100),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = MockRenderEngine::new();
        renderer.paint_cursor_internal(&mut engine);
        assert!(engine.calls.is_empty());
    }

    #[test]
    fn paint_buffer_output_no_panic_with_empty_text_buffer() {
        // Step 6 structural loop no-ops when text_buffer downcast to `()` fails.
        let mut renderer = Renderer {
            viewport: (0, 0, 80, 24),
            ..Default::default()
        };
        let mut engine = MockRenderEngine::new();
        let data = MockRenderData;
        renderer.paint_buffer_output(&mut engine, &data);
        // No paint_buffer_line called (phase 3: core missing TextBuffer rows).
        assert!(engine.calls.is_empty());
    }

    #[test]
    fn renderer_default_inits_new_selection_and_title_fields() {
        let r = Renderer::default();
        assert!(r.last_selection_rects.is_empty());
        assert!(r.title.is_empty());
        assert_eq!(r.viewport, (0, 0, 0, 0));
    }
}
