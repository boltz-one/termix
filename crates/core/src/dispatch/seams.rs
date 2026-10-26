//! `dispatch::seams` — frozen contract surface between phase boundaries.
//!
//! Phase 1 (P01 wave 0 enabler): trait SIGNATURES only — bodies marked
//! `// STUB`. Later phases implement concretely:
//! - P03 (`host`) — `ITerminalApi` glue impl + `HeadlessHost` ownership.
//! - P09 (`core`/terminal core) — Terminal writes `RenderState` via
//!   `RenderModeSink`, drives `InputModeSink` notify on interactivity/mode
//!   changes.
//! - P10 (`renderer`) — reads `IRenderData` + `RenderState` for paint.
//!
//! WT source anchors:
//! - `ITerminalApi.hpp`/`ITerminalApi.cpp` — `windows/src/terminal/adapter/`.
//! - `InteractivityFactory` / `InputMode` —
//!   `windows/src/terminal/interactivity/InteractivityFactory.cpp` +
//!   `terminalInput.hpp:18-58` (notify path).
//! - `RenderSettings` / `Mode` (cursor style/color, screen-reversed, palette overrides)
//!   — `windows/src/renderer/renderer.cpp:60-180` + `inc/RenderSettings.hpp`.
//!   Move of `IRenderData` from `renderer/src/paint.rs` (canonical home is
//!   here; renderer's `RenderData` is the local copy, with rework target P10).
//! - `IRenderData.hpp:31-90` — `RenderFrameInfo` + getters.

use crate::attrs::TextAttribute;
use crate::cursor::CursorShape;
// —— ASSOCIATED PLAIN STRUCTS —————————————————————————

/// `(row, col)` — port của WT `COORD`/`til::point`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CursorPosition {
    pub row: i32,
    pub col: i32,
}

/// `(top, left, bottom, right)` — port của WT `til::rect`/`SMALL_RECT`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

/// Color overrides for `RenderState` (VT DECSCUSR/OSC 4/12 settled at P09).
/// 16-entry palette (packed RGBA8888, BIG-endian `0xRRGGBBAA` — match WT
/// `ColorRef`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaletteOverrides {
    pub table: [u32; 16],
}

/// Renderer-facing state snapshot. Port của WT
/// `RenderSettings` (cursor style/color, screen-reversed flag, palette
/// overrides). Written P09 via `RenderModeSink`; read P10 via `IRenderData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderState {
    /// `screen_reversed` — SGR 5/DECSCNM (mode 5) — `RenderSettings::IsScreenReversed`.
    pub screen_reversed: bool,
    /// `cursor_style` — DECSCUSR cursor style — `RenderSettings::GetCursorStyle`.
    pub cursor_style: CursorShape,
    /// `cursor_color` — OSC 12 settled color — `RenderSettings::GetCursorColor`.
    pub cursor_color: u32,
    /// `palette_overrides` — OSC 4/10/11/12 entries — `RenderSettings::GetColorTableEntry`.
    pub palette_overrides: PaletteOverrides,
}

/// `RenderFrameInfo` — port `inc/IRenderData.hpp:31-37`. WT fields:
/// `searchHighlights`, `searchHighlightFocused`, `selectionSpans`,
/// `selectionBackground`. Phase 1 holds `Vec<Rect>` for highlight/span
/// lists (final shape finalized P10).
#[derive(Clone, Debug, Default)]
pub struct RenderFrameInfo {
    pub search_highlights: Vec<Rect>,
    pub search_highlight_focused: bool,
    pub selection_spans: Vec<Rect>,
    pub selection_background: u32,
}

// —— TRAIT: `InputModeSink` ——————————————————————

/// `InputModeSink` — port của WT `InteractivityFactory`/`InputMode` notify
/// path (`terminalInput.hpp:18-58` + `InteractivityFactory.cpp`). Cung cấp
/// thông báo về interactivity sign-off / mode transitions từ phía Terminal
/// (P09) xuống input subsystem (P04/P11). Phase 1 contract-stub.
pub trait InputModeSink {
    /// Notify input subsystem khi `InteractivityState` (focus, conpty input
    /// mode, mouse mode) thay đổi — port của
    /// `RenderSettings::SetInputMode` / `InteractivityFactory::Notify`.
    fn notify_input_mode(&mut self, mode: InputMode);
    /// Notify focus state change — `InteractivityFactory::FocusChanged`.
    fn notify_focus(&mut self, focused: bool);
}

/// `InputMode` flag set — port của WT `InputMode` (keyboard / mouse /
/// quick-edit bitmask). Phase 1 dùng `u32` (bitflags land P04).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputMode(pub u32);
// —— TRAIT: `RenderModeSink` ———————————————————————

/// `RenderModeSink` — port của WT `RenderSettings`/mode notify path.
/// Terminal (P09) writes `RenderState` qua trait này khi SGR/DECSCUSR/OSC
/// mutations settle; P10 renderer subscribes để refresh paint brushes.
pub trait RenderModeSink {
    /// Apply full `RenderState` snapshot — port của
    /// `RenderSettings::SetSettings` (full replace).
    fn set_render_state(&mut self, state: &RenderState);
    /// Toggle screen-reversed (SGR 5 / DECSCNM mode 5) —
    /// `RenderSettings::SetScreenReversed`.
    fn set_screen_reversed(&mut self, reversed: bool);
    /// Settle cursor style (DECSCUSR) — `RenderSettings::SetCursorStyle`.
    fn set_cursor_style(&mut self, style: CursorShape);
    /// Settle cursor color (OSC 12) — `RenderSettings::SetCursorColor`.
    fn set_cursor_color(&mut self, color: u32);
    /// Settle palette override at `index` (OSC 4) —
    /// `RenderSettings::SetColorTableEntry`.
    fn set_palette_entry(&mut self, index: usize, color: u32);
    /// Reset a single palette entry to default (`RenderSettings::ResetColorTableEntry`).
    fn reset_palette_entry(&mut self, index: usize);
}

// —— TRAIT: `ITerminalApi` ————————————————————————

/// `ITerminalApi` — port verbatim của `windows/src/terminal/adapter/ITerminalApi.hpp`
/// full method set. Adapter-facing host API (programmatic/test + conhost-like
/// impl). Phase 1: signatures only, bodies impl P03 (`host`/`HeadlessHost`).
///
/// Modeled after WT virtual surface — every method trên `ITermDispatch` tuân
/// theo phương thức `ITerminal*` runtime API được conhost áp dụng phía chân
/// adapter. Group theo WT source order.
#[allow(unused_variables)]
pub trait ITerminalApi {
    // —— Cursor positions — port `ITerminalApi.hpp:24-40`.
    /// Get current cursor position (row, col) — `GetCursorPosition`.
    fn get_cursor_position(&self) -> CursorPosition;
    /// Get viewport top-row offset — `GetViewportTop`.
    fn get_viewport_top(&self) -> i32;
    /// Get text buffer reference — `GetTextBuffer`.
    fn get_text_buffer(&self) -> &dyn std::any::Any;
    /// Get text buffer mutable reference — `GetTextBuffer` (mutable).
    fn get_text_buffer_mut(&mut self) -> &mut dyn std::any::Any;
    /// Get screen lines (restrict-to-visible) — `GetScreenLines`.
    fn get_screen_lines(&self) -> &dyn std::any::Any;
    /// Get active buffer row/column — WT internal helper cho
    /// `GetActiveBuffer` virtual row/col returns.
    fn get_active_row(&self) -> i32;
    fn get_active_col(&self) -> i32;

    // —— Mode/state — port `:42-78`.
    /// Returns whether screen-reversed (DECSCNM) is set — `IsScreenReversed`.
    fn is_screen_reversed(&self) -> bool;
    /// Returns current `RenderState` snapshot — `GetRenderSettings`.
    fn get_render_state(&self) -> RenderState;
    /// Returns whether VT input mode (`ansi_mode`) is on — `IsVtInputMode`.
    fn is_vt_input_mode(&self) -> bool;
    /// Get the `TextAttribute` at cursor — `GetTextAttributes`.
    fn get_text_attributes(&self) -> TextAttribute;
    /// Set the `TextAttribute` at cursor — `SetTextAttributes`.
    fn set_text_attributes(&mut self, attrs: TextAttribute);
    /// Returns the current `InputMode` flags — `GetInputMode`.
    fn get_input_mode(&self) -> InputMode;
    /// Set the `InputMode` flags — `SetInputMode`.
    fn set_input_mode(&mut self, mode: InputMode);

    // —— Mutations / output — port `:80-128`.
    /// Print a single char at cursor, advancing it — `PrintChar`.
    fn print_char(&mut self, c: char);
    /// Print a string at cursor, advancing it — `PrintString`.
    fn print_string(&mut self, s: &str);
    /// Execute newline/CR at cursor — `Newline`/`CarriageReturn` combo.
    fn execute_crlf(&mut self);
    /// Resize buffer to new width/height (cols/rows) — `ResizeScreenBuffer`.
    fn resize_screen_buffer(&mut self, width: i32, height: i32);
    /// Set cursor position — `SetCursorPosition`.
    fn set_cursor_position(&mut self, row: i32, col: i32);
    /// Set viewport top offset — `SetViewportTop`.
    fn set_viewport_top(&mut self, top: i32);
    /// Set screen-reversed mode — `SetScreenReversed`.
    fn set_screen_reversed(&mut self, reversed: bool);
    /// Set window title (OSC 0/2) — `SetWindowTitle`.
    fn set_window_title(&mut self, title: &str);
    /// Set current working directory (OSC 7) — `SetCurrentWorkingDirectory`.
    fn set_current_working_directory(&mut self, uri: &str);
    /// Set clipboard content (OSC 52) — `SetClipboard`.
    fn set_clipboard(&mut self, content: &str);
    /// Add outbound bytes (DSR/DA reports) back to host stdin — `WriteFile`.
    fn write_input(&mut self, bytes: &[u8]);
    /// Set color table entry (OSC 4) — `SetColorTableEntry`.
    fn set_color_table_entry(&mut self, index: usize, color: u32);
    /// Reset color table entry (OSC 104) — `ResetColorTableEntry`.
    fn reset_color_table_entry(&mut self, index: usize);
    /// Trigger warning bell — `WarningBell`.
    fn warning_bell(&mut self);

    // —— Hard/soft reset — port `:130-145`.
    /// Soft reset (DECSTR) — `SoftReset`.
    fn soft_reset(&mut self);
    /// Hard reset (RIS) — `HardReset`.
    fn hard_reset(&mut self, erase: bool);
    /// Screen alignment pattern (DECALN) — `ScreenAlignmentPattern`.
    fn screen_alignment_pattern(&mut self);
}

// —— TRAIT: `IRenderData` ————————————————————————

/// `IRenderData` — port của `inc/IRenderData.hpp:45-90`. Renderer (P10)
/// consumes this surface; adapter (P09/P10) provides it. Methods return
/// rows/cursor/selection/accessor for paint — moved from
/// `renderer/src/paint.rs` (canonical home).
///
/// Phase 1 contract-stub; P10 wires concrete `RenderData` impl against
/// `TextBuffer`/`RenderState`/.
#[allow(unused_variables)]
pub trait IRenderData {
    /// Viewport `Rect` (top/left/bottom/right) — `GetViewport`.
    fn get_viewport(&self) -> Rect;
    /// `TextBuffer` đọc-only — `GetTextBuffer`.
    fn get_text_buffer(&self) -> &dyn std::any::Any;
    /// `FontInfo` đọc-only — `GetFontInfo`.
    fn get_font_info(&self) -> &dyn std::any::Any;
    /// `(fg, bg)` packed RGBA cho một `TextAttribute` — `GetAttributeColors`.
    fn get_attribute_colors(&self, attr: &dyn std::any::Any) -> (u32, u32);
    /// Search highlight rectangles — `GetSearchHighlights`.
    fn get_search_highlights(&self) -> &[Rect];
    /// Selection span rectangles — `GetSelectionSpans`.
    fn get_selection_spans(&self) -> &[Rect];
    /// Cursor blink interval (ms) — `GetBlinkInterval`.
    fn get_blink_interval(&self) -> u32;
    /// Pattern renderer id tại một point — `GetPatternId`.
    fn get_pattern_id(&self, point: (i32, i32)) -> u64;
    /// Active composition (IME) state — `GetActiveComposition`.
    fn get_active_composition(&self) -> Option<&dyn std::any::Any>;
    /// Current cursor position (row, col) — `GetCursorPosition`.
    fn get_cursor_position(&self) -> CursorPosition;
    /// Current `RenderState` snapshot — `GetRenderState`.
    fn get_render_state(&self) -> &RenderState;
    /// `RenderFrameInfo` snapshot — `GetRenderFrameInfo`.
    fn get_render_frame_info(&self) -> RenderFrameInfo;
}

// —— Re-exports ————————————————————————————————

/// `TextAttribute` alias exported cho paint path users.
pub use crate::attrs::TextAttribute as RenderTextAttribute;
