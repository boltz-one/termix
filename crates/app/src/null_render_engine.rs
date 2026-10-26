//! `NullRenderEngine` — no-op `RenderEngine` until the real GPUI engine lands.
//! Lets `TermControl` be constructed so the app window can open.

use renderer::grid::ShapedRow;
use renderer::{Cluster, CursorOptions, RenderData, RenderEngine, RenderFrameInfo};

#[derive(Default)]
pub struct NullRenderEngine;

impl RenderEngine for NullRenderEngine {
    fn start_paint(&mut self, _data: &dyn RenderData, _info: &RenderFrameInfo) -> bool {
        false
    }
    fn update_drawing_brushes(&mut self, _attr: &dyn std::any::Any) {}
    fn scroll_frame(&mut self) {}
    fn prepare_render_info(&mut self, _data: &dyn RenderData) {}
    fn paint_background(&mut self) {}
    fn paint_buffer_line(&mut self, _clusters: &[Cluster], _coord: (i32, i32), _trim: i32) {}
    fn paint_buffer_grid_lines(&mut self, _row: &ShapedRow, _lines: u32, _from: i32, _to: i32) {}
    fn paint_selection(&mut self, _rect: (i32, i32, i32, i32)) {}
    fn paint_cursor(&mut self, _options: &CursorOptions) {}
    fn update_title(&mut self, _title: &str) {}
    fn end_paint(&mut self) {}
    fn notify_paint_frame(&mut self) {}
    fn requires_continuous_redraw(&self) -> bool {
        false
    }
    fn invalidate(&mut self, _rect: (i32, i32, i32, i32)) {}
    fn invalidate_all(&mut self) {}
    fn get_proposed_font(&self) -> &dyn std::any::Any {
        &()
    }
    fn get_dirty_area(&self) -> (i32, i32, i32, i32) {
        (0, 0, 0, 0)
    }
    fn is_infinite_dirty(&self) -> bool {
        false
    }
    fn update_font(&mut self, _desired: &dyn std::any::Any, _actual: &dyn std::any::Any) -> bool {
        true
    }
    fn update_soft_font(&mut self, _cells: &[(u16, u16)], _bits: &[u8], _centering: bool) {}
    fn set_graphics_rendition(&mut self, _rendition: &dyn std::any::Any) {}
    fn set_selection_background(&mut self, _rgba: u32) {}
    fn prepare_line_transform(&mut self, _y: i32, _pitch: f32, _grid_lines: u32) {}
    fn paint_image_slice(&mut self, _coord: (i32, i32), _size: (u32, u32)) {}
}
