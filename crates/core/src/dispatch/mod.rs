//! `ITermDispatch` trait — port verbatim từng method name theo
//! `windows/src/terminal/adapter/ITermDispatch.hpp:39-199` (124 virtual
//! methods, ~123 + destructor).
//!
//! Phase 1 skeleton: 123 trait methods với default `todo!()` body. Lane B
//! (parser) build trên top của trait này; Lane C (adapter) implement.
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

/// `ITermDispatch` — port của `ITermDispatch.hpp:35-200`. Group theo nhóm
/// để dễ đọc. Tất cả method mặc định `todo!()` — nàng/he phải implement
/// trait này ở phase 2.
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
        todo!("ITermDispatch::UnknownSequence")
    }
    fn print(&mut self, c: char) {
        todo!("ITermDispatch::Print")
    }
    fn print_string(&mut self, s: &str) {
        todo!("ITermDispatch::PrintString")
    }

    // —— Cursor — port `:43-55`.
    fn cursor_up(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::CursorUp")
    }
    fn cursor_down(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::CursorDown")
    }
    fn cursor_forward(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::CursorForward")
    }
    fn cursor_backward(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::CursorBackward")
    }
    fn cursor_next_line(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::CursorNextLine")
    }
    fn cursor_prev_line(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::CursorPrevLine")
    }
    fn cursor_horizontal_position_absolute(&mut self, column: VtInt) {
        let _ = column;
        todo!("ITermDispatch::CursorHorizontalPositionAbsolute")
    }
    fn vertical_line_position_absolute(&mut self, line: VtInt) {
        let _ = line;
        todo!("ITermDispatch::VerticalLinePositionAbsolute")
    }
    fn horizontal_position_relative(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::HorizontalPositionRelative")
    }
    fn vertical_position_relative(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::VerticalPositionRelative")
    }
    fn cursor_position(&mut self, line: VtInt, column: VtInt) {
        let _ = (line, column);
        todo!("ITermDispatch::CursorPosition")
    }
    fn cursor_save_state(&mut self) {
        todo!("ITermDispatch::CursorSaveState")
    }
    fn cursor_restore_state(&mut self) {
        todo!("ITermDispatch::CursorRestoreState")
    }

    // —— Insert/Delete (chars + lines) — port `:56-69`.
    fn insert_character(&mut self, count: VtInt) {
        let _ = count;
        todo!("ITermDispatch::InsertCharacter")
    }
    fn delete_character(&mut self, count: VtInt) {
        let _ = count;
        todo!("ITermDispatch::DeleteCharacter")
    }
    fn scroll_up(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::ScrollUp")
    }
    fn scroll_down(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::ScrollDown")
    }
    fn next_page(&mut self, page_count: VtInt) {
        let _ = page_count;
        todo!("ITermDispatch::NextPage")
    }
    fn preceding_page(&mut self, page_count: VtInt) {
        let _ = page_count;
        todo!("ITermDispatch::PrecedingPage")
    }
    fn page_position_absolute(&mut self, page: VtInt) {
        let _ = page;
        todo!("ITermDispatch::PagePositionAbsolute")
    }
    fn page_position_relative(&mut self, page_count: VtInt) {
        let _ = page_count;
        todo!("ITermDispatch::PagePositionRelative")
    }
    fn page_position_back(&mut self, page_count: VtInt) {
        let _ = page_count;
        todo!("ITermDispatch::PagePositionBack")
    }
    fn request_displayed_extent(&mut self) {
        todo!("ITermDispatch::RequestDisplayedExtent")
    }
    fn insert_line(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::InsertLine")
    }
    fn delete_line(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::DeleteLine")
    }
    fn insert_column(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::InsertColumn")
    }
    fn delete_column(&mut self, distance: VtInt) {
        let _ = distance;
        todo!("ITermDispatch::DeleteColumn")
    }

    // —— Modes — port `:70-78`.
    fn set_keypad_mode(&mut self, application_mode: bool) {
        let _ = application_mode;
        todo!("ITermDispatch::SetKeypadMode")
    }
    fn set_ansi_mode(&mut self, ansi_mode: bool) {
        let _ = ansi_mode;
        todo!("ITermDispatch::SetAnsiMode")
    }
    fn set_kitty_keyboard_protocol(&mut self, flags: VtParameter, mode: VtParameter) {
        let _ = (flags, mode);
        todo!("ITermDispatch::SetKittyKeyboardProtocol")
    }
    fn query_kitty_keyboard_protocol(&mut self) {
        todo!("ITermDispatch::QueryKittyKeyboardProtocol")
    }
    fn push_kitty_keyboard_protocol(&mut self, flags: VtParameter) {
        let _ = flags;
        todo!("ITermDispatch::PushKittyKeyboardProtocol")
    }
    fn pop_kitty_keyboard_protocol(&mut self, count: VtParameter) {
        let _ = count;
        todo!("ITermDispatch::PopKittyKeyboardProtocol")
    }
    fn set_top_bottom_scrolling_margins(&mut self, top: VtInt, bottom: VtInt) {
        let _ = (top, bottom);
        todo!("ITermDispatch::SetTopBottomScrollingMargins")
    }
    fn set_left_right_scrolling_margins(&mut self, left: VtInt, right: VtInt) {
        let _ = (left, right);
        todo!("ITermDispatch::SetLeftRightScrollingMargins")
    }
    fn enquire_answerback(&mut self) {
        todo!("ITermDispatch::EnquireAnswerback")
    }

    // —— Bell / Cr / Lf / Ri — port `:79-84`.
    fn warning_bell(&mut self) {
        todo!("ITermDispatch::WarningBell")
    }
    fn carriage_return(&mut self) {
        todo!("ITermDispatch::CarriageReturn")
    }
    fn line_feed(&mut self, line_feed_type: LineFeedType) {
        let _ = line_feed_type;
        todo!("ITermDispatch::LineFeed")
    }
    fn reverse_line_feed(&mut self) {
        todo!("ITermDispatch::ReverseLineFeed")
    }
    fn back_index(&mut self) {
        todo!("ITermDispatch::BackIndex")
    }
    fn forward_index(&mut self) {
        todo!("ITermDispatch::ForwardIndex")
    }

    // —— Window / OSC — port `:85-86,157-166`.
    fn set_window_title(&mut self, title: &str) {
        let _ = title;
        todo!("ITermDispatch::SetWindowTitle")
    }
    fn set_current_working_directory(&mut self, uri: &str) {
        let _ = uri;
        todo!("ITermDispatch::SetCurrentWorkingDirectory")
    }
    fn set_clipboard(&mut self, content: &str) {
        let _ = content;
        todo!("ITermDispatch::SetClipboard")
    }
    fn window_manipulation(&mut self, function: WindowManipulationType, params: VtParameters) {
        let _ = (function, params);
        todo!("ITermDispatch::WindowManipulation")
    }
    fn add_hyperlink(&mut self, uri: &str, params: &str) {
        let _ = (uri, params);
        todo!("ITermDispatch::AddHyperlink")
    }
    fn end_hyperlink(&mut self) {
        todo!("ITermDispatch::EndHyperlink")
    }
    fn do_con_emu_action(&mut self, s: &str) {
        let _ = s;
        todo!("ITermDispatch::DoConEmuAction")
    }
    fn do_iterm2_action(&mut self, s: &str) {
        let _ = s;
        todo!("ITermDispatch::DoITerm2Action")
    }
    fn do_final_term_action(&mut self, s: &str) {
        let _ = s;
        todo!("ITermDispatch::DoFinalTermAction")
    }
    fn do_vs_code_action(&mut self, s: &str) {
        let _ = s;
        todo!("ITermDispatch::DoVsCodeAction")
    }
    fn do_wt_action(&mut self, s: &str) {
        let _ = s;
        todo!("ITermDispatch::DoWTAction")
    }
    fn do_urxvt_action(&mut self, s: &str) {
        let _ = s;
        todo!("ITermDispatch::DoUrxvtAction")
    }

    // —— Tabs — port `:87-91`.
    fn horizontal_tab_set(&mut self) {
        todo!("ITermDispatch::HorizontalTabSet")
    }
    fn forward_tab(&mut self, num_tabs: VtInt) {
        let _ = num_tabs;
        todo!("ITermDispatch::ForwardTab")
    }
    fn backwards_tab(&mut self, num_tabs: VtInt) {
        let _ = num_tabs;
        todo!("ITermDispatch::BackwardsTab")
    }
    fn tab_clear(&mut self, clear_type: TabClearType) {
        let _ = clear_type;
        todo!("ITermDispatch::TabClear")
    }
    fn tab_set(&mut self, set_type: VtParameter) {
        let _ = set_type;
        todo!("ITermDispatch::TabSet")
    }

    // —— Color table — port `:92-99`.
    fn set_color_table_entry(&mut self, table_index: usize, color: u32) {
        let _ = (table_index, color);
        todo!("ITermDispatch::SetColorTableEntry")
    }
    fn request_color_table_entry(&mut self, table_index: usize) {
        let _ = table_index;
        todo!("ITermDispatch::RequestColorTableEntry")
    }
    fn reset_color_table(&mut self) {
        todo!("ITermDispatch::ResetColorTable")
    }
    fn reset_color_table_entry(&mut self, table_index: usize) {
        let _ = table_index;
        todo!("ITermDispatch::ResetColorTableEntry")
    }
    fn set_xterm_color_resource(&mut self, resource: usize, color: u32) {
        let _ = (resource, color);
        todo!("ITermDispatch::SetXtermColorResource")
    }
    fn request_xterm_color_resource(&mut self, resource: usize) {
        let _ = resource;
        todo!("ITermDispatch::RequestXtermColorResource")
    }
    fn reset_xterm_color_resource(&mut self, resource: usize) {
        let _ = resource;
        todo!("ITermDispatch::ResetXtermColorResource")
    }
    fn assign_color(&mut self, item: ColorItem, fg_index: VtInt, bg_index: VtInt) {
        let _ = (item, fg_index, bg_index);
        todo!("ITermDispatch::AssignColor")
    }

    // —— Erase — port `:101-113`.
    fn erase_in_display(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        todo!("ITermDispatch::EraseInDisplay")
    }
    fn erase_in_line(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        todo!("ITermDispatch::EraseInLine")
    }
    fn erase_characters(&mut self, num_chars: VtInt) {
        let _ = num_chars;
        todo!("ITermDispatch::EraseCharacters")
    }
    fn selective_erase_in_display(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        todo!("ITermDispatch::SelectiveEraseInDisplay")
    }
    fn selective_erase_in_line(&mut self, erase_type: EraseType) {
        let _ = erase_type;
        todo!("ITermDispatch::SelectiveEraseInLine")
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
        todo!("ITermDispatch::ChangeAttributesRectangularArea")
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
        todo!("ITermDispatch::ReverseAttributesRectangularArea")
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
        todo!("ITermDispatch::CopyRectangularArea")
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
        todo!("ITermDispatch::FillRectangularArea")
    }
    fn erase_rectangular_area(&mut self, top: VtInt, left: VtInt, bottom: VtInt, right: VtInt) {
        let _ = (top, left, bottom, right);
        todo!("ITermDispatch::EraseRectangularArea")
    }
    fn selective_erase_rectangular_area(
        &mut self,
        top: VtInt,
        left: VtInt,
        bottom: VtInt,
        right: VtInt,
    ) {
        let _ = (top, left, bottom, right);
        todo!("ITermDispatch::SelectiveEraseRectangularArea")
    }
    fn select_attribute_change_extent(&mut self, extent: ChangeExtent) {
        let _ = extent;
        todo!("ITermDispatch::SelectAttributeChangeExtent")
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
        todo!("ITermDispatch::RequestChecksumRectangularArea")
    }

    // —— SGR — port `:116-122`.
    fn set_graphics_rendition(&mut self, options: VtParameters) {
        let _ = options;
        todo!("ITermDispatch::SetGraphicsRendition")
    }
    fn set_line_rendition(&mut self, rendition: LineRendition) {
        let _ = rendition;
        todo!("ITermDispatch::SetLineRendition")
    }
    fn set_character_protection_attribute(&mut self, options: VtParameters) {
        let _ = options;
        todo!("ITermDispatch::SetCharacterProtectionAttribute")
    }
    fn push_graphics_rendition(&mut self, options: VtParameters) {
        let _ = options;
        todo!("ITermDispatch::PushGraphicsRendition")
    }
    fn pop_graphics_rendition(&mut self) {
        todo!("ITermDispatch::PopGraphicsRendition")
    }

    // —— Modes (set / reset / request) — port `:123-131`.
    fn set_mode(&mut self, param: ModeParams) {
        let _ = param;
        todo!("ITermDispatch::SetMode")
    }
    fn reset_mode(&mut self, param: ModeParams) {
        let _ = param;
        todo!("ITermDispatch::ResetMode")
    }
    fn request_mode(&mut self, param: ModeParams) {
        let _ = param;
        todo!("ITermDispatch::RequestMode")
    }
    fn device_status_report(&mut self, status_type: StatusType, id: VtParameter) {
        let _ = (status_type, id);
        todo!("ITermDispatch::DeviceStatusReport")
    }
    fn device_attributes(&mut self) {
        todo!("ITermDispatch::DeviceAttributes")
    }
    fn secondary_device_attributes(&mut self) {
        todo!("ITermDispatch::SecondaryDeviceAttributes")
    }
    fn tertiary_device_attributes(&mut self) {
        todo!("ITermDispatch::TertiaryDeviceAttributes")
    }
    fn vt52_device_attributes(&mut self) {
        todo!("ITermDispatch::Vt52DeviceAttributes")
    }
    fn request_terminal_parameters(&mut self, permission: ReportingPermission) {
        let _ = permission;
        todo!("ITermDispatch::RequestTerminalParameters")
    }

    // —— Charsets — port `:134-143`.
    fn designate_coding_system(&mut self, coding_system: VtId) {
        let _ = coding_system;
        todo!("ITermDispatch::DesignateCodingSystem")
    }
    fn designate94_charset(&mut self, gset_number: VtInt, charset: VtId) {
        let _ = (gset_number, charset);
        todo!("ITermDispatch::Designate94Charset")
    }
    fn designate96_charset(&mut self, gset_number: VtInt, charset: VtId) {
        let _ = (gset_number, charset);
        todo!("ITermDispatch::Designate96Charset")
    }
    fn locking_shift(&mut self, gset_number: VtInt) {
        let _ = gset_number;
        todo!("ITermDispatch::LockingShift")
    }
    fn locking_shift_right(&mut self, gset_number: VtInt) {
        let _ = gset_number;
        todo!("ITermDispatch::LockingShiftRight")
    }
    fn single_shift(&mut self, gset_number: VtInt) {
        let _ = gset_number;
        todo!("ITermDispatch::SingleShift")
    }
    fn accept_c1_controls(&mut self, enabled: bool) {
        let _ = enabled;
        todo!("ITermDispatch::AcceptC1Controls")
    }
    fn send_c1_controls(&mut self, enabled: bool) {
        let _ = enabled;
        todo!("ITermDispatch::SendC1Controls")
    }
    fn announce_code_structure(&mut self, ansi_level: VtInt) {
        let _ = ansi_level;
        todo!("ITermDispatch::AnnounceCodeStructure")
    }

    // —— Reset — port `:144-146`.
    fn soft_reset(&mut self) {
        todo!("ITermDispatch::SoftReset")
    }
    fn hard_reset(&mut self, erase: bool) {
        let _ = erase;
        todo!("ITermDispatch::HardReset")
    }
    fn screen_alignment_pattern(&mut self) {
        todo!("ITermDispatch::ScreenAlignmentPattern")
    }

    // —— Cursor style — port `:148`.
    fn set_cursor_style(&mut self, cursor_style: CursorStyle) {
        let _ = cursor_style;
        todo!("ITermDispatch::SetCursorStyle")
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
        todo!("ITermDispatch::DefineSixelImage")
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
        todo!("ITermDispatch::DownloadDRCS")
    }
    fn request_user_preference_charset(&mut self) {
        todo!("ITermDispatch::RequestUserPreferenceCharset")
    }
    fn assign_user_preference_charset(&mut self, charset_size: CharsetSize) {
        let _ = charset_size;
        todo!("ITermDispatch::AssignUserPreferenceCharset")
    }
    fn define_macro(&mut self, macro_id: VtInt, private_macro: bool, handler: StringHandler) {
        let _ = (macro_id, private_macro, handler);
        todo!("ITermDispatch::DefineMacro")
    }
    fn invoke_macro(&mut self, macro_id: VtInt) {
        let _ = macro_id;
        todo!("ITermDispatch::InvokeMacro")
    }
    fn request_terminal_state_report(&mut self, format: ReportFormat, format_option: VtParameter) {
        let _ = (format, format_option);
        todo!("ITermDispatch::RequestTerminalStateReport")
    }
    fn restore_terminal_state(&mut self, format: ReportFormat) {
        let _ = format;
        todo!("ITermDispatch::RestoreTerminalState")
    }
    fn request_setting(&mut self) {
        todo!("ITermDispatch::RequestSetting")
    }
    fn request_presentation_state_report(&mut self, format: PresentationReportFormat) {
        let _ = format;
        todo!("ITermDispatch::RequestPresentationStateReport")
    }
    fn restore_presentation_state(&mut self, format: PresentationReportFormat) {
        let _ = format;
        todo!("ITermDispatch::RestorePresentationState")
    }
    fn play_sounds(&mut self, parameters: VtParameters) {
        let _ = parameters;
        todo!("ITermDispatch::PlaySounds")
    }
    fn set_optional_features(&mut self, features: u32) {
        let _ = features;
        todo!("ITermDispatch::SetOptionalFeatures")
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
        // Default body là `todo!()` — không gọi được nên chỉ check struct
        // impl được compile (assert presence bằng type cast).
        fn _assert_impl<T: ITermDispatch>() {}
        _assert_impl::<Stub>();
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
}
