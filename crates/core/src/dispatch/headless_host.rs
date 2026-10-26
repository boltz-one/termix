//! `HeadlessHost` — minimal programmatic `ITerminalApi`/`InputModeSink`/
//! `RenderModeSink` impl, kept in plain fields. Port của WT `TestGetSet`
//! semantics (`windows/src/terminal/adapter/TestGetSet.hpp/.cpp`).
//!
//! Phase 1 (P01 wave 0): struct complete, bodies marked defer implementation (return
//! default), no real buffer/state behind it. Owned by P01 (created); P03
//! owns afterwards and wires concrete `TextBuffer`/`Viewport`/`RenderState`.
//!
//! `TestGetSet` là test double tách simple-getter/setter surface — chỉ
//! mirror field state, không có real coninteraction. Here we keep the
//! same shape: field-per-ITerminalApi getter + setters simple round-trip.

use super::seams::{
    CursorPosition, ITerminalApi, InputMode, InputModeSink, PaletteOverrides, Rect, RenderModeSink,
    RenderState,
};
use crate::attrs::TextAttribute;
use crate::cursor::CursorShape;

/// `HeadlessHost` — programmatic test host. Stores host state in plain
/// fields. External `ITermDispatch` tests + P03 host wiring mutate qua
/// `ITerminalApi`/`InputModeSink`/`RenderModeSink` methods.
#[derive(Clone, Debug, Default)]
pub struct HeadlessHost {
    // —— Cursor / position — port `TestGetSet` + `ITerminalApi.hpp:24-40`.
    pub cursor_position: CursorPosition,
    pub viewport_top: i32,
    pub active_row: i32,
    pub active_col: i32,

    // —— Viewport / screen — port `:30-46`.
    pub viewport: Rect,
    pub screen_lines_buf: Vec<String>,

    // —— Mode / state — port `:42-78`.
    pub screen_reversed: bool,
    pub render_state: RenderState,
    pub vt_input_mode: bool,
    pub ansi_mode: bool,
    pub input_mode: InputMode,
    pub text_attributes: TextAttribute,
    pub cursor_shape: CursorShape,

    // —— Palette — port `OSC 4/12` (TestGetSet `_colorTable`).
    pub color_table: [u32; 16],
    pub palette_overrides: PaletteOverrides,

    // —— Output side-effects — TestGetSet records outbound host bytes.
    pub window_title: String,
    pub current_working_directory: String,
    pub clipboard: String,
    pub write_buffer: Vec<u8>,

    // —— Bell/sizing.
    pub bell_rang: bool,
    pub resize_history: Vec<(i32, i32)>,

    // —— Reset sentinel.
    pub soft_reset_count: u32,
    pub hard_reset_count: u32,
}

impl HeadlessHost {
    /// New default host — empty state. Mirrors `TestGetSet::TestGetSet`.
    pub fn new() -> Self {
        Self::default()
    }
}

// —— `ITerminalApi` impl — port verbatim `ITerminalApi.hpp` virtual surface.
#[allow(unused_variables)]
impl ITerminalApi for HeadlessHost {
    fn get_cursor_position(&self) -> CursorPosition {
        self.cursor_position
    }
    fn get_viewport_top(&self) -> i32 {
        self.viewport_top
    }
    fn get_text_buffer(&self) -> &dyn std::any::Any {
        &self.screen_lines_buf as &dyn std::any::Any
    }
    fn get_text_buffer_mut(&mut self) -> &mut dyn std::any::Any {
        &mut self.screen_lines_buf as &mut dyn std::any::Any
    }
    fn get_screen_lines(&self) -> &dyn std::any::Any {
        &self.screen_lines_buf as &dyn std::any::Any
    }
    fn get_active_row(&self) -> i32 {
        self.active_row
    }
    fn get_active_col(&self) -> i32 {
        self.active_col
    }

    fn is_screen_reversed(&self) -> bool {
        self.screen_reversed
    }
    fn get_render_state(&self) -> RenderState {
        self.render_state.clone()
    }
    fn is_vt_input_mode(&self) -> bool {
        self.vt_input_mode
    }
    fn get_text_attributes(&self) -> TextAttribute {
        self.text_attributes
    }
    fn set_text_attributes(&mut self, attrs: TextAttribute) {
        self.text_attributes = attrs;
    }
    fn get_input_mode(&self) -> InputMode {
        self.input_mode
    }
    fn set_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
    }

    fn print_char(&mut self, c: char) {
        let _ = c;
    }
    fn print_string(&mut self, s: &str) {
        let _ = s;
    }
    fn execute_crlf(&mut self) {}
    fn resize_screen_buffer(&mut self, width: i32, height: i32) {
        self.resize_history.push((width, height));
    }
    fn set_cursor_position(&mut self, row: i32, col: i32) {
        self.cursor_position = CursorPosition { row, col };
    }
    fn set_viewport_top(&mut self, top: i32) {
        self.viewport_top = top;
    }
    fn set_screen_reversed(&mut self, reversed: bool) {
        self.screen_reversed = reversed;
        self.render_state.screen_reversed = reversed;
    }
    fn set_window_title(&mut self, title: &str) {
        self.window_title = title.to_string();
    }
    fn set_current_working_directory(&mut self, uri: &str) {
        self.current_working_directory = uri.to_string();
    }
    fn set_clipboard(&mut self, content: &str) {
        self.clipboard = content.to_string();
    }
    fn write_input(&mut self, bytes: &[u8]) {
        self.write_buffer.extend_from_slice(bytes);
    }
    fn set_color_table_entry(&mut self, index: usize, color: u32) {
        if let Some(slot) = self.color_table.get_mut(index) {
            *slot = color;
        }
        if let Some(slot) = self.palette_overrides.table.get_mut(index) {
            *slot = color;
        }
    }
    fn reset_color_table_entry(&mut self, index: usize) {
        if let Some(slot) = self.color_table.get_mut(index) {
            *slot = 0;
        }
        if let Some(slot) = self.palette_overrides.table.get_mut(index) {
            *slot = 0;
        }
    }
    fn warning_bell(&mut self) {
        self.bell_rang = true;
    }
    fn soft_reset(&mut self) {
        self.soft_reset_count += 1;
    }
    fn hard_reset(&mut self, erase: bool) {
        let _ = erase;
        self.hard_reset_count += 1;
    }
    fn screen_alignment_pattern(&mut self) {}
}

// —— `InputModeSink` impl — port `InteractivityFactory` notify path.
impl InputModeSink for HeadlessHost {
    fn notify_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
    }
    fn notify_focus(&mut self, focused: bool) {
        let _ = focused;
    }
}

// —— `RenderModeSink` impl — port `RenderSettings` write-side notify.
impl RenderModeSink for HeadlessHost {
    fn set_render_state(&mut self, state: &RenderState) {
        self.render_state = state.clone();
        self.screen_reversed = state.screen_reversed;
    }
    fn set_screen_reversed(&mut self, reversed: bool) {
        self.render_state.screen_reversed = reversed;
        self.screen_reversed = reversed;
    }
    fn set_cursor_style(&mut self, style: CursorShape) {
        self.render_state.cursor_style = style;
        self.cursor_shape = style;
    }
    fn set_cursor_color(&mut self, color: u32) {
        self.render_state.cursor_color = color;
    }
    fn set_palette_entry(&mut self, index: usize, color: u32) {
        if let Some(slot) = self.render_state.palette_overrides.table.get_mut(index) {
            *slot = color;
        }
        if let Some(slot) = self.color_table.get_mut(index) {
            *slot = color;
        }
    }
    fn reset_palette_entry(&mut self, index: usize) {
        if let Some(slot) = self.render_state.palette_overrides.table.get_mut(index) {
            *slot = 0;
        }
        if let Some(slot) = self.color_table.get_mut(index) {
            *slot = 0;
        }
    }
}

// —— `IRenderData` impl gone here — concrete buffer for rendering is a P10
// concern; `HeadlessHost` isn't expected to be a paint side.

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 1: do trivial round-trip on getters/setters to assert trait
    /// surface exists + struct is default-constructible. P03/+ will swap
    /// these for real-state tests.
    #[test]
    fn headless_host_default_constructs() {
        let host = HeadlessHost::new();
        assert_eq!(host.cursor_position, CursorPosition::default());
        assert_eq!(host.viewport_top, 0);
        assert_eq!(host.render_state, RenderState::default());
        assert!(!host.screen_reversed);
    }

    #[test]
    fn headless_host_setter_round_trips_via_traits() {
        let mut host = HeadlessHost::new();
        ITerminalApi::set_cursor_position(&mut host, 3, 7);
        assert_eq!(ITerminalApi::get_cursor_position(&host).row, 3);
        assert_eq!(ITerminalApi::get_cursor_position(&host).col, 7);

        RenderModeSink::set_screen_reversed(&mut host, true);
        assert!(ITerminalApi::is_screen_reversed(&host));

        InputModeSink::notify_input_mode(&mut host, InputMode(0xDEAD_BEEF));
        assert_eq!(ITerminalApi::get_input_mode(&host).0, 0xDEAD_BEEF);
    }
}
