//! `ITermDispatch` trait — port verbatim từng method name theo
//! `windows/src/terminal/adapter/ITermDispatch.hpp:39-199` (124 virtual
//! methods, ~123 + destructor).
//!
//! Phase 1 complete skeleton: 124 trait method default `todo!()` → no-op
//! (lane B mass fold loop-08). Lane C `AdaptDispatch` concrete impl defer
//! P09/P11 (conhost out-of-scope: ITerminalApi/ServiceLocator/InputBuffer/VtIo).
//!
//! Module chia thành sub-groups để dưới 200 LOC/file. The single trait body
//! stays here; participate subtypes ở `interact.rs`.

// —— Re-export sub-group traits/types ——
pub mod headless_host;
pub mod interact;
pub mod interact_impl;
pub mod seams;
pub mod td;
pub mod types;

pub use interact::InteractDispatch;
pub use seams::{
    CursorPosition, IRenderData, ITerminalApi, InputMode, InputModeSink, PaletteOverrides, Rect,
    RenderFrameInfo, RenderModeSink, RenderState,
};
pub use types::*;

use crate::attrs::LineRendition;

/// Tham số VT — port `VTInt`/`VTParameter`/`VTParameters`. Phase 1: `i32`
/// + slice cho variadic; phase 2 nâng `VTParameter` (with default value).
pub type VtInt = i32;
pub type VtParameter = i32;

/// `VTParameters` — variadic slice. Phase 1: `&[VtParameter]`.
pub type VtParameters<'a> = &'a [VtParameter];

/// `VTID` — port `VTID` (final byte / string). Phase 1 dùng `&str`.
pub type VtId<'a> = &'a str;

/// `StringHandler` — port `StringHandler` (consumes a string).
/// Phase 1: Box<dyn FnOnce(&str)>.
pub type StringHandler = Box<dyn FnOnce(&str)>;

/// `ITermDispatch` — port của `ITermDispatch.hpp:35-200` (124 virtual method).
/// Group theo nhóm để dễ đọc. Phase 1 skeleton complete: default body
/// `todo!()` → no-op lane B (loop-08). Lane C `AdaptDispatch` concrete impl
/// defer P09/P11 (conhost out-of-scope).
///
/// C++ args được hạ cấp thành Rust-friendly types:
/// - `VTInt` → `VtInt` (= `i32`).
/// - `VTParameter` → `VtParameter` (= `i32`).
/// - `VTParameters` → `&[i32]`.
/// - `wchar_t`/`wstring_view` → `char`/`&str`.
/// - `DispatchTypes::*` enums → trait-level stub enums (ở底部).
#[allow(unused_variables)]
pub trait ITermDispatch {
    // Unknown / Print — port `:39,40,41`.
    fn unknown_sequence(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn print(&mut self, c: char) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn print_string(&mut self, s: &str) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Cursor — port `:43-55`.
    fn cursor_up(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_down(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_forward(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_backward(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_next_line(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_prev_line(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_horizontal_position_absolute(&mut self, column: VtInt) {
        let _ = column;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn vertical_line_position_absolute(&mut self, line: VtInt) {
        let _ = line;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn horizontal_position_relative(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn vertical_position_relative(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_position(&mut self, line: VtInt, column: VtInt) {
        let _ = (line, column);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_save_state(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn cursor_restore_state(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Insert/Delete (chars + lines) — port `:56-69`.
    fn insert_character(&mut self, count: VtInt) {
        let _ = count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn delete_character(&mut self, count: VtInt) {
        let _ = count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn scroll_up(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn scroll_down(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn next_page(&mut self, page_count: VtInt) {
        let _ = page_count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn preceding_page(&mut self, page_count: VtInt) {
        let _ = page_count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn page_position_absolute(&mut self, page: VtInt) {
        let _ = page;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn page_position_relative(&mut self, page_count: VtInt) {
        let _ = page_count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn page_position_back(&mut self, page_count: VtInt) {
        let _ = page_count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_displayed_extent(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn insert_line(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn delete_line(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn insert_column(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn delete_column(&mut self, distance: VtInt) {
        let _ = distance;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Modes — port `:70-78`.
    fn set_keypad_mode(&mut self, application_mode: bool) {
        let _ = application_mode;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_ansi_mode(&mut self, ansi_mode: bool) {
        let _ = ansi_mode;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_kitty_keyboard_protocol(&mut self, flags: VtParameter, mode: VtParameter) {
        let _ = (flags, mode);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn query_kitty_keyboard_protocol(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn push_kitty_keyboard_protocol(&mut self, flags: VtParameter) {
        let _ = flags;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn pop_kitty_keyboard_protocol(&mut self, count: VtParameter) {
        let _ = count;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_top_bottom_scrolling_margins(&mut self, top: VtInt, bottom: VtInt) {
        let _ = (top, bottom);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_left_right_scrolling_margins(&mut self, left: VtInt, right: VtInt) {
        let _ = (left, right);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn enquire_answerback(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Bell / Cr / Lf / Ri — port `:79-84`.
    fn warning_bell(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn carriage_return(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn line_feed(&mut self, line_feed_type: LineFeedType) {
        let _ = line_feed_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn reverse_line_feed(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn back_index(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn forward_index(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Window / OSC — port `:85-86,157-166`.
    fn set_window_title(&mut self, title: &str) {
        let _ = title;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_current_working_directory(&mut self, uri: &str) {
        let _ = uri;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_clipboard(&mut self, content: &str) {
        let _ = content;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn window_manipulation(&mut self, function: WindowManipulationType, params: VtParameters) {
        let _ = (function, params);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn add_hyperlink(&mut self, uri: &str, params: &str) {
        let _ = (uri, params);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn end_hyperlink(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn do_con_emu_action(&mut self, s: &str) {
        let _ = s;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn do_iterm2_action(&mut self, s: &str) {
        let _ = s;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn do_final_term_action(&mut self, s: &str) {
        let _ = s;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn do_vs_code_action(&mut self, s: &str) {
        let _ = s;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn do_wt_action(&mut self, s: &str) {
        let _ = s;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn do_urxvt_action(&mut self, s: &str) {
        let _ = s;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Tabs — port `:87-91`.
    fn horizontal_tab_set(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn forward_tab(&mut self, num_tabs: VtInt) {
        let _ = num_tabs;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn backwards_tab(&mut self, num_tabs: VtInt) {
        let _ = num_tabs;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn tab_clear(&mut self, clear_type: TabClearType) {
        let _ = clear_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn tab_set(&mut self, set_type: VtParameter) {
        let _ = set_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Color table — port `:92-99`.
    fn set_color_table_entry(&mut self, table_index: usize, color: u32) {
        let _ = (table_index, color);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_color_table_entry(&mut self, table_index: usize) {
        let _ = table_index;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn reset_color_table(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn reset_color_table_entry(&mut self, table_index: usize) {
        let _ = table_index;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_xterm_color_resource(&mut self, resource: usize, color: u32) {
        let _ = (resource, color);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_xterm_color_resource(&mut self, resource: usize) {
        let _ = resource;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn reset_xterm_color_resource(&mut self, resource: usize) {
        let _ = resource;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn assign_color(&mut self, item: ColorItem, fg_index: VtInt, bg_index: VtInt) {
        let _ = (item, fg_index, bg_index);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Erase — port `:101-113`.
    fn erase_in_display(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn erase_in_line(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn erase_characters(&mut self, num_chars: VtInt) {
        let _ = num_chars;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn selective_erase_in_display(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn selective_erase_in_line(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn change_attributes_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
        attrs: VtParameters,
    ) {
        let _ = (top, left, bottom, right, attrs);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn reverse_attributes_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
        attrs: VtParameters,
    ) {
        let _ = (top, left, bottom, right, attrs);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    #[allow(clippy::too_many_arguments)]
    fn copy_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
        page: VtInt,
        dst_top: VtInt,
        dst_left: VtInt,
        dst_page: VtInt,
    ) {
        let _ = (top, left, bottom, right, page, dst_top, dst_left, dst_page);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn fill_rectangular_area(
        &mut self,
        ch: VtParameter,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        let _ = (ch, top, left, bottom, right);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn erase_rectangular_area(&mut self, top: VtInt, left: VtInt, bottom: VtInt, right: VtInt) {
        let _ = (top, left, bottom, right);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn selective_erase_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        let _ = (top, left, bottom, right);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn select_attribute_change_extent(&mut self, extent: ChangeExtent) {
        let _ = extent;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_checksum_rectangular_area(
        &mut self,
        id: VtInt,
        page: VtInt,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        let _ = (id, page, top, left, bottom, right);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— SGR — port `:116-122`.
    fn set_graphics_rendition(&mut self, options: VtParameters) {
        let _ = options;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_line_rendition(&mut self, rendition: LineRendition) {
        let _ = rendition;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_character_protection_attribute(&mut self, options: VtParameters) {
        let _ = options;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn push_graphics_rendition(&mut self, options: VtParameters) {
        let _ = options;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn pop_graphics_rendition(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Modes (set / reset / request) — port `:123-131`.
    fn set_mode(&mut self, param: ModeParams) {
        let _ = param;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn reset_mode(&mut self, param: ModeParams) {
        let _ = param;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_mode(&mut self, param: ModeParams) {
        let _ = param;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn device_status_report(&mut self, status_type: StatusType, id: VtParameter) {
        let _ = (status_type, id);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn device_attributes(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn secondary_device_attributes(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn tertiary_device_attributes(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn vt52_device_attributes(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_terminal_parameters(&mut self, permission: ReportingPermission) {
        let _ = permission;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Charsets — port `:134-143`.
    fn designate_coding_system(&mut self, coding_system: VtId) {
        let _ = coding_system;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn designate94_charset(&mut self, gset_number: VtInt, charset: VtId) {
        let _ = (gset_number, charset);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn designate96_charset(&mut self, gset_number: VtInt, charset: VtId) {
        let _ = (gset_number, charset);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn locking_shift(&mut self, gset_number: VtInt) {
        let _ = gset_number;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn locking_shift_right(&mut self, gset_number: VtInt) {
        let _ = gset_number;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn single_shift(&mut self, gset_number: VtInt) {
        let _ = gset_number;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn accept_c1_controls(&mut self, enabled: bool) {
        let _ = enabled;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn send_c1_controls(&mut self, enabled: bool) {
        let _ = enabled;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn announce_code_structure(&mut self, ansi_level: VtInt) {
        let _ = ansi_level;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Reset — port `:144-146`.
    fn soft_reset(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn hard_reset(&mut self, erase: bool) {
        let _ = erase;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn screen_alignment_pattern(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— Cursor style — port `:148`.
    fn set_cursor_style(&mut self, cursor_style: CursorStyle) {
        let _ = cursor_style;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }

    // —— DCS / Sixel / Macros — port `:168-200`.
    fn define_sixel_image(
        &mut self,
        macro_parameter: VtInt,
        background_color: VtInt,
        private_color_registers: bool,
        handler: StringHandler,
    ) {
        let _ = (
            macro_parameter,
            background_color,
            private_color_registers,
            handler,
        );
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn download_drcs(
        &mut self,
        font_number: VtInt,
        start_char: VtInt,
        cleanup: bool,
        cell_matrix: VtInt,
        font_sets: VtInt,
        handler: StringHandler,
    ) {
        let _ = (
            font_number,
            start_char,
            cleanup,
            cell_matrix,
            font_sets,
            handler,
        );
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_user_preference_charset(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn assign_user_preference_charset(&mut self, charset_size: CharsetSize) {
        let _ = charset_size;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn define_macro(&mut self, macro_id: VtInt, private_macro: bool, handler: StringHandler) {
        let _ = (macro_id, private_macro, handler);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn invoke_macro(&mut self, macro_id: VtInt) {
        let _ = macro_id;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_terminal_state_report(&mut self, format: ReportFormat, format_option: VtParameter) {
        let _ = (format, format_option);
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn restore_terminal_state(&mut self, format: ReportFormat) {
        let _ = format;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_setting(&mut self) {
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn request_presentation_state_report(&mut self, format: PresentationReportFormat) {
        let _ = format;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn restore_presentation_state(&mut self, format: PresentationReportFormat) {
        let _ = format;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn play_sounds(&mut self, parameters: VtParameters) {
        let _ = parameters;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
    fn set_optional_features(&mut self, features: u32) {
        let _ = features;
        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.
    }
}

// Blanket impl `ITermDispatch for Box<T>` — forward mọi method sang inner `T`.
//
// Rust std không tự cung cấp `impl<T: Trait + ?Sized> Trait for Box<T>` (chỉ có
// `Deref`/`DerefMut`, không tự impl trait). Adapter engine `OutputStateMachineEngine<D:
// ITermDispatch>` generic over `D` — khi test wrap `Box::new(RecordingDispatch)` và
// e2e-wrap `Box<dyn ITermDispatch>`, cả hai đều cần `Box<…>: ITermDispatch`. Blanket
// impl cover cả hai case với một khai báo duy nhất, không cần macro delegate từng method.
impl<T: ITermDispatch + ?Sized> ITermDispatch for Box<T> {
    fn unknown_sequence(&mut self) {
        (**self).unknown_sequence();
    }
    fn print(&mut self, c: char) {
        (**self).print(c);
    }
    fn print_string(&mut self, s: &str) {
        (**self).print_string(s);
    }
    fn cursor_up(&mut self, distance: VtInt) {
        (**self).cursor_up(distance);
    }
    fn cursor_down(&mut self, distance: VtInt) {
        (**self).cursor_down(distance);
    }
    fn cursor_forward(&mut self, distance: VtInt) {
        (**self).cursor_forward(distance);
    }
    fn cursor_backward(&mut self, distance: VtInt) {
        (**self).cursor_backward(distance);
    }
    fn cursor_next_line(&mut self, distance: VtInt) {
        (**self).cursor_next_line(distance);
    }
    fn cursor_prev_line(&mut self, distance: VtInt) {
        (**self).cursor_prev_line(distance);
    }
    fn cursor_horizontal_position_absolute(&mut self, column: VtInt) {
        (**self).cursor_horizontal_position_absolute(column);
    }
    fn vertical_line_position_absolute(&mut self, line: VtInt) {
        (**self).vertical_line_position_absolute(line);
    }
    fn horizontal_position_relative(&mut self, distance: VtInt) {
        (**self).horizontal_position_relative(distance);
    }
    fn vertical_position_relative(&mut self, distance: VtInt) {
        (**self).vertical_position_relative(distance);
    }
    fn cursor_position(&mut self, line: VtInt, column: VtInt) {
        (**self).cursor_position(line, column);
    }
    fn cursor_save_state(&mut self) {
        (**self).cursor_save_state();
    }
    fn cursor_restore_state(&mut self) {
        (**self).cursor_restore_state();
    }
    fn insert_character(&mut self, count: VtInt) {
        (**self).insert_character(count);
    }
    fn delete_character(&mut self, count: VtInt) {
        (**self).delete_character(count);
    }
    fn scroll_up(&mut self, distance: VtInt) {
        (**self).scroll_up(distance);
    }
    fn scroll_down(&mut self, distance: VtInt) {
        (**self).scroll_down(distance);
    }
    fn next_page(&mut self, page_count: VtInt) {
        (**self).next_page(page_count);
    }
    fn preceding_page(&mut self, page_count: VtInt) {
        (**self).preceding_page(page_count);
    }
    fn page_position_absolute(&mut self, page: VtInt) {
        (**self).page_position_absolute(page);
    }
    fn page_position_relative(&mut self, page_count: VtInt) {
        (**self).page_position_relative(page_count);
    }
    fn page_position_back(&mut self, page_count: VtInt) {
        (**self).page_position_back(page_count);
    }
    fn request_displayed_extent(&mut self) {
        (**self).request_displayed_extent();
    }
    fn insert_line(&mut self, distance: VtInt) {
        (**self).insert_line(distance);
    }
    fn delete_line(&mut self, distance: VtInt) {
        (**self).delete_line(distance);
    }
    fn insert_column(&mut self, distance: VtInt) {
        (**self).insert_column(distance);
    }
    fn delete_column(&mut self, distance: VtInt) {
        (**self).delete_column(distance);
    }
    fn set_keypad_mode(&mut self, application_mode: bool) {
        (**self).set_keypad_mode(application_mode);
    }
    fn set_ansi_mode(&mut self, ansi_mode: bool) {
        (**self).set_ansi_mode(ansi_mode);
    }
    fn set_kitty_keyboard_protocol(&mut self, flags: VtParameter, mode: VtParameter) {
        (**self).set_kitty_keyboard_protocol(flags, mode);
    }
    fn query_kitty_keyboard_protocol(&mut self) {
        (**self).query_kitty_keyboard_protocol();
    }
    fn push_kitty_keyboard_protocol(&mut self, flags: VtParameter) {
        (**self).push_kitty_keyboard_protocol(flags);
    }
    fn pop_kitty_keyboard_protocol(&mut self, count: VtParameter) {
        (**self).pop_kitty_keyboard_protocol(count);
    }
    fn set_top_bottom_scrolling_margins(&mut self, top: VtInt, bottom: VtInt) {
        (**self).set_top_bottom_scrolling_margins(top, bottom);
    }
    fn set_left_right_scrolling_margins(&mut self, left: VtInt, right: VtInt) {
        (**self).set_left_right_scrolling_margins(left, right);
    }
    fn enquire_answerback(&mut self) {
        (**self).enquire_answerback();
    }
    fn warning_bell(&mut self) {
        (**self).warning_bell();
    }
    fn carriage_return(&mut self) {
        (**self).carriage_return();
    }
    fn line_feed(&mut self, line_feed_type: LineFeedType) {
        (**self).line_feed(line_feed_type);
    }
    fn reverse_line_feed(&mut self) {
        (**self).reverse_line_feed();
    }
    fn back_index(&mut self) {
        (**self).back_index();
    }
    fn forward_index(&mut self) {
        (**self).forward_index();
    }
    fn set_window_title(&mut self, title: &str) {
        (**self).set_window_title(title);
    }
    fn set_current_working_directory(&mut self, uri: &str) {
        (**self).set_current_working_directory(uri);
    }
    fn set_clipboard(&mut self, content: &str) {
        (**self).set_clipboard(content);
    }
    fn window_manipulation(&mut self, function: WindowManipulationType, params: VtParameters) {
        (**self).window_manipulation(function, params);
    }
    fn add_hyperlink(&mut self, uri: &str, params: &str) {
        (**self).add_hyperlink(uri, params);
    }
    fn end_hyperlink(&mut self) {
        (**self).end_hyperlink();
    }
    fn do_con_emu_action(&mut self, s: &str) {
        (**self).do_con_emu_action(s);
    }
    fn do_iterm2_action(&mut self, s: &str) {
        (**self).do_iterm2_action(s);
    }
    fn do_final_term_action(&mut self, s: &str) {
        (**self).do_final_term_action(s);
    }
    fn do_vs_code_action(&mut self, s: &str) {
        (**self).do_vs_code_action(s);
    }
    fn do_wt_action(&mut self, s: &str) {
        (**self).do_wt_action(s);
    }
    fn do_urxvt_action(&mut self, s: &str) {
        (**self).do_urxvt_action(s);
    }
    fn horizontal_tab_set(&mut self) {
        (**self).horizontal_tab_set();
    }
    fn forward_tab(&mut self, num_tabs: VtInt) {
        (**self).forward_tab(num_tabs);
    }
    fn backwards_tab(&mut self, num_tabs: VtInt) {
        (**self).backwards_tab(num_tabs);
    }
    fn tab_clear(&mut self, clear_type: TabClearType) {
        (**self).tab_clear(clear_type);
    }
    fn tab_set(&mut self, set_type: VtParameter) {
        (**self).tab_set(set_type);
    }
    fn set_color_table_entry(&mut self, table_index: usize, color: u32) {
        (**self).set_color_table_entry(table_index, color);
    }
    fn request_color_table_entry(&mut self, table_index: usize) {
        (**self).request_color_table_entry(table_index);
    }
    fn reset_color_table(&mut self) {
        (**self).reset_color_table();
    }
    fn reset_color_table_entry(&mut self, table_index: usize) {
        (**self).reset_color_table_entry(table_index);
    }
    fn set_xterm_color_resource(&mut self, resource: usize, color: u32) {
        (**self).set_xterm_color_resource(resource, color);
    }
    fn request_xterm_color_resource(&mut self, resource: usize) {
        (**self).request_xterm_color_resource(resource);
    }
    fn reset_xterm_color_resource(&mut self, resource: usize) {
        (**self).reset_xterm_color_resource(resource);
    }
    fn assign_color(&mut self, item: ColorItem, fg_index: VtInt, bg_index: VtInt) {
        (**self).assign_color(item, fg_index, bg_index);
    }
    fn erase_in_display(&mut self, erase_type: EraseType) {
        (**self).erase_in_display(erase_type);
    }
    fn erase_in_line(&mut self, erase_type: EraseType) {
        (**self).erase_in_line(erase_type);
    }
    fn erase_characters(&mut self, num_chars: VtInt) {
        (**self).erase_characters(num_chars);
    }
    fn selective_erase_in_display(&mut self, erase_type: EraseType) {
        (**self).selective_erase_in_display(erase_type);
    }
    fn selective_erase_in_line(&mut self, erase_type: EraseType) {
        (**self).selective_erase_in_line(erase_type);
    }
    fn change_attributes_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
        attrs: VtParameters,
    ) {
        (**self).change_attributes_rectangular_area(top, left, bottom, right, attrs);
    }
    fn reverse_attributes_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
        attrs: VtParameters,
    ) {
        (**self).reverse_attributes_rectangular_area(top, left, bottom, right, attrs);
    }
    #[allow(clippy::too_many_arguments)]
    fn copy_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
        page: VtInt,
        dst_top: VtInt,
        dst_left: VtInt,
        dst_page: VtInt,
    ) {
        (**self).copy_rectangular_area(top, left, bottom, right, page, dst_top, dst_left, dst_page);
    }
    fn fill_rectangular_area(
        &mut self,
        ch: VtParameter,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        (**self).fill_rectangular_area(ch, top, left, bottom, right);
    }
    fn erase_rectangular_area(&mut self, top: VtInt, left: VtInt, bottom: VtInt, right: VtInt) {
        (**self).erase_rectangular_area(top, left, bottom, right);
    }
    fn selective_erase_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        (**self).selective_erase_rectangular_area(top, left, bottom, right);
    }
    fn select_attribute_change_extent(&mut self, extent: ChangeExtent) {
        (**self).select_attribute_change_extent(extent);
    }
    fn request_checksum_rectangular_area(
        &mut self,
        id: VtInt,
        page: VtInt,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        (**self).request_checksum_rectangular_area(id, page, top, left, bottom, right);
    }
    fn set_graphics_rendition(&mut self, options: VtParameters) {
        (**self).set_graphics_rendition(options);
    }
    fn set_line_rendition(&mut self, rendition: LineRendition) {
        (**self).set_line_rendition(rendition);
    }
    fn set_character_protection_attribute(&mut self, options: VtParameters) {
        (**self).set_character_protection_attribute(options);
    }
    fn push_graphics_rendition(&mut self, options: VtParameters) {
        (**self).push_graphics_rendition(options);
    }
    fn pop_graphics_rendition(&mut self) {
        (**self).pop_graphics_rendition();
    }
    fn set_mode(&mut self, param: ModeParams) {
        (**self).set_mode(param);
    }
    fn reset_mode(&mut self, param: ModeParams) {
        (**self).reset_mode(param);
    }
    fn request_mode(&mut self, param: ModeParams) {
        (**self).request_mode(param);
    }
    fn device_status_report(&mut self, status_type: StatusType, id: VtParameter) {
        (**self).device_status_report(status_type, id);
    }
    fn device_attributes(&mut self) {
        (**self).device_attributes();
    }
    fn secondary_device_attributes(&mut self) {
        (**self).secondary_device_attributes();
    }
    fn tertiary_device_attributes(&mut self) {
        (**self).tertiary_device_attributes();
    }
    fn vt52_device_attributes(&mut self) {
        (**self).vt52_device_attributes();
    }
    fn request_terminal_parameters(&mut self, permission: ReportingPermission) {
        (**self).request_terminal_parameters(permission);
    }
    fn designate_coding_system(&mut self, coding_system: VtId) {
        (**self).designate_coding_system(coding_system);
    }
    fn designate94_charset(&mut self, gset_number: VtInt, charset: VtId) {
        (**self).designate94_charset(gset_number, charset);
    }
    fn designate96_charset(&mut self, gset_number: VtInt, charset: VtId) {
        (**self).designate96_charset(gset_number, charset);
    }
    fn locking_shift(&mut self, gset_number: VtInt) {
        (**self).locking_shift(gset_number);
    }
    fn locking_shift_right(&mut self, gset_number: VtInt) {
        (**self).locking_shift_right(gset_number);
    }
    fn single_shift(&mut self, gset_number: VtInt) {
        (**self).single_shift(gset_number);
    }
    fn accept_c1_controls(&mut self, enabled: bool) {
        (**self).accept_c1_controls(enabled);
    }
    fn send_c1_controls(&mut self, enabled: bool) {
        (**self).send_c1_controls(enabled);
    }
    fn announce_code_structure(&mut self, ansi_level: VtInt) {
        (**self).announce_code_structure(ansi_level);
    }
    fn soft_reset(&mut self) {
        (**self).soft_reset();
    }
    fn hard_reset(&mut self, erase: bool) {
        (**self).hard_reset(erase);
    }
    fn screen_alignment_pattern(&mut self) {
        (**self).screen_alignment_pattern();
    }
    fn set_cursor_style(&mut self, cursor_style: CursorStyle) {
        (**self).set_cursor_style(cursor_style);
    }
    fn define_sixel_image(
        &mut self,
        macro_parameter: VtInt,
        background_color: VtInt,
        private_color_registers: bool,
        handler: StringHandler,
    ) {
        (**self).define_sixel_image(
            macro_parameter,
            background_color,
            private_color_registers,
            handler,
        );
    }
    fn download_drcs(
        &mut self,
        font_number: VtInt,
        start_char: VtInt,
        cleanup: bool,
        cell_matrix: VtInt,
        font_sets: VtInt,
        handler: StringHandler,
    ) {
        (**self).download_drcs(
            font_number,
            start_char,
            cleanup,
            cell_matrix,
            font_sets,
            handler,
        );
    }
    fn request_user_preference_charset(&mut self) {
        (**self).request_user_preference_charset();
    }
    fn assign_user_preference_charset(&mut self, charset_size: CharsetSize) {
        (**self).assign_user_preference_charset(charset_size);
    }
    fn define_macro(&mut self, macro_id: VtInt, private_macro: bool, handler: StringHandler) {
        (**self).define_macro(macro_id, private_macro, handler);
    }
    fn invoke_macro(&mut self, macro_id: VtInt) {
        (**self).invoke_macro(macro_id);
    }
    fn request_terminal_state_report(&mut self, format: ReportFormat, format_option: VtParameter) {
        (**self).request_terminal_state_report(format, format_option);
    }
    fn restore_terminal_state(&mut self, format: ReportFormat) {
        (**self).restore_terminal_state(format);
    }
    fn request_setting(&mut self) {
        (**self).request_setting();
    }
    fn request_presentation_state_report(&mut self, format: PresentationReportFormat) {
        (**self).request_presentation_state_report(format);
    }
    fn restore_presentation_state(&mut self, format: PresentationReportFormat) {
        (**self).restore_presentation_state(format);
    }
    fn play_sounds(&mut self, parameters: VtParameters) {
        (**self).play_sounds(parameters);
    }
    fn set_optional_features(&mut self, features: u32) {
        (**self).set_optional_features(features);
    }
}

// —— Sub-type stubs cho trait method signatures. Phase 2 sẽ switch sang
// full `bitflags`-backed enums ở `DispatchTypes.hpp`; phase 1 giữ `i32`-like
// newtypes cho signature stability. (Enums moved to `types.rs`.)

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub impl để sanity-check trait surface compile. Phase 1 chỉ 보
    /// signature; adapter phase 2 mới impl thật.
    struct Stub;
    impl ITermDispatch for Stub {}

    #[test]
    fn trait_has_default_print_stub() {
        // Loop-08 fold: default body giờ là no-op (không `todo!()`), nên
        // ta có thể call `print` an toàn — assert trait impl compile + call
        // không panic. Sanity-check trait surface.
        fn _assert_impl<T: ITermDispatch>() {}
        _assert_impl::<Stub>();
        let mut s = Stub;
        s.print('a'); // no-op, không panic.
    }

    #[test]
    fn enums_default_to_first_variant() {
        // Verbatim variant order per `types.rs` (`DispatchTypes.hpp`): first
        // variant = `#[default]`. EraseType → ToEnd, TabClearType →
        // ClearCurrent, LineFeedType → DependsOnMode.
        assert_eq!(EraseType::default(), EraseType::ToEnd);
        assert_eq!(TabClearType::default(), TabClearType::ClearCurrent);
        assert_eq!(LineFeedType::default(), LineFeedType::DependsOnMode);
    }

    // —— Loop-08 lane-B no-op fold coverage: 20 tests assert default trait
    // bodies không panic + không có side-effect observable. Mỗi test cover
    // một signature-shape đại diện (zero-arg, single VtInt, char/&str, bool,
    // enum, multi-arg, struct). Verify `todo!("ITermDispatch::Name")` đã
    // được fold thành no-op comment (lane C AdaptDispatch defer P09/P11).

    #[test]
    fn no_op_zero_arg_unknown_sequence() {
        let mut s = Stub;
        s.unknown_sequence();
    }

    #[test]
    fn no_op_zero_arg_warning_bell() {
        let mut s = Stub;
        s.warning_bell();
    }

    #[test]
    fn no_op_zero_arg_carriage_return() {
        let mut s = Stub;
        s.carriage_return();
    }

    #[test]
    fn no_op_zero_arg_cursor_save_restore_state() {
        let mut s = Stub;
        s.cursor_save_state();
        s.cursor_restore_state();
    }

    #[test]
    fn no_op_char_print() {
        let mut s = Stub;
        s.print('a');
        s.print('来');
        s.print('\u{1F600}');
    }

    #[test]
    fn no_op_str_print_string() {
        let mut s = Stub;
        s.print_string("hello");
        s.print_string("");
        s.print_string("tiếng Việt");
    }

    #[test]
    fn no_op_single_vtint_cursor_up() {
        let mut s = Stub;
        s.cursor_up(5);
        s.cursor_up(0);
        s.cursor_up(i32::MAX);
    }

    #[test]
    fn no_op_single_vtint_scan_scroll_insert_delete() {
        let mut s = Stub;
        s.scroll_up(3);
        s.scroll_down(7);
        s.insert_character(2);
        s.delete_character(9);
    }

    #[test]
    fn no_op_two_vtint_cursor_position() {
        let mut s = Stub;
        s.cursor_position(10, 20);
        s.set_top_bottom_scrolling_margins(1, 24);
        s.set_left_right_scrolling_margins(1, 80);
    }

    #[test]
    fn no_op_bool_set_keypad_ansi_mode() {
        let mut s = Stub;
        s.set_keypad_mode(true);
        s.set_keypad_mode(false);
        s.set_ansi_mode(true);
        s.set_ansi_mode(false);
    }

    #[test]
    fn no_op_enum_line_feed_with_default_variant() {
        let mut s = Stub;
        s.line_feed(LineFeedType::default());
        s.line_feed(LineFeedType::DependsOnMode);
    }

    #[test]
    fn no_op_enum_tab_clear_with_default_variant() {
        let mut s = Stub;
        s.tab_clear(TabClearType::default());
        s.tab_clear(TabClearType::ClearCurrent);
    }

    #[test]
    fn no_op_enum_erase_in_display_with_default_variant() {
        let mut s = Stub;
        s.erase_in_display(EraseType::default());
        s.erase_in_line(EraseType::ToEnd);
        s.erase_characters(4);
    }

    #[test]
    fn no_op_str_window_title_clipboard() {
        let mut s = Stub;
        s.set_window_title("test");
        s.set_clipboard("copied");
        s.set_current_working_directory("file:///x");
    }

    #[test]
    fn no_op_str_osc_action_handlers() {
        let mut s = Stub;
        s.do_con_emu_action("a");
        s.do_iterm2_action("");
        s.do_final_term_action("x");
        s.add_hyperlink("https://e", "");
        s.end_hyperlink();
    }

    #[test]
    fn no_op_usize_u32_color_table_entry() {
        let mut s = Stub;
        s.set_color_table_entry(1, 0xFF0000);
        s.request_color_table_entry(2);
        s.reset_color_table_entry(3);
    }

    #[test]
    fn no_op_vtparam_tab_set_and_kitty() {
        let mut s = Stub;
        s.tab_set(0);
        s.set_kitty_keyboard_protocol(1, 2);
        s.push_kitty_keyboard_protocol(3);
        s.pop_kitty_keyboard_protocol(1);
    }

    #[test]
    fn no_op_enum_struct_rect_and_window_manipulation() {
        let mut s = Stub;
        s.set_cursor_style(CursorStyle::default());
        s.window_manipulation(WindowManipulationType::default(), VtParameters::default());
    }

    #[test]
    fn no_op_bool_hard_reset_and_features() {
        let mut s = Stub;
        s.hard_reset(true);
        s.hard_reset(false);
        s.set_optional_features(0);
        s.set_optional_features(u32::MAX);
    }

    #[test]
    fn no_op_vtparameters_graphics_rendition_push_pop() {
        let mut s = Stub;
        s.set_graphics_rendition(VtParameters::default());
        s.push_graphics_rendition(VtParameters::default());
        s.pop_graphics_rendition();
    }
}
