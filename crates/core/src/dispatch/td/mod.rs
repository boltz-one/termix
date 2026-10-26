//! `td/` — `TerminalDispatch` impl hub (Phase 4 — Wave 1 foundation).
//!
//! Port verbatim `windows/src/terminal/adapter/adaptDispatch.{hpp,cpp}`:
//! C++ `AdaptDispatch` → Rust `TerminalDispatch`. W1 lands struct + ctor
//! + 14 cursor + 2 print + 2 private helper.
//!
//! File globs (W1 owns):
//! - `dispatch/td/mod.rs`      — struct + ctor + `Offset`/`ModeSet`/`CursorState` types.
//! - `dispatch/td/cursor.rs`   — 14 cursor method.
//! - `dispatch/td/print.rs`    — `print`/`print_string` + `_write_to_buffer` helper.
//!
//! Sibling Wave 1 worker files (`bell_tabs.rs`, `charset.rs`, `dsr.rs`,
//! `modes.rs`, `reset.rs`, `scroll.rs`) ship cùng directory nhưng chưa trong
//! `mod` tree (not declared here) — each wave's worker declares its own
//! module sau khi struct land. W1 build chỉ include `cursor`/`print`.
//!
//! Anchors:
//! - `adaptDispatch.hpp:33-359` — struct decl + member list.
//! - `adaptDispatch.hpp:202-213` — `enum class Mode` (private).
//! - `adaptDispatch.hpp:219-228` — `struct CursorState`.
//! - `adaptDispatch.hpp:229-238` — `struct Offset`.
//! - `adaptDispatch.cpp:40-49` — ctor `AdaptDispatch::AdaptDispatch`.

pub mod cursor;
pub mod print;
// W3 — TEMP integration for verification; will be re-enabled properly when
// W1 lands the integration hub (remove before commit if requested).
pub mod bell_tabs;
pub mod charset;
pub mod dsr;
pub mod erase;
pub mod modes;
pub mod reset;
pub mod scroll;
// Phase 5 W1 — SGR/color/free-fn hub (file existed, chưa wire vào trait).
pub mod graphics;
// Phase 5 W1 — color table (DECAC/Xterm resource). render_settings stub.
pub mod color_table;
// Phase 5 W2 — OSC sub-actions (SetWindowTitle, SetClipboard, WindowManipulation wired;
// 9 STUB chờ InteractDispatch trait extends).
pub mod osc;
// Phase 5 W4 — DCS sub-actions (DECRQSS responses, DECCARA/DECRARA rect area, sixel/macro,
// DECSACE, SetOptionalFeatures). select_attribute_range_extent port real; rest STUB.
pub mod dcs;

// Removed glob imports that don't re-export public items.

use crate::buffer::TextBuffer;
use crate::dispatch::interact::InteractDispatch;
use crate::dispatch::ITermDispatch;
use crate::screen::Viewport;
use crate::sgr_stack::SgrStack;

/// `Offset` — port `adaptDispatch.hpp:229-238`.
///
/// Verbatim semantics: VT origin is at 1,1; absolute positions subtract 1.
/// `Forward(n)`/`Backward(n)` produce relative signed deltas; `Absolute(v)`
/// produces value-1 stored with `IsAbsolute=true`; `Unchanged` = `Forward(0)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Offset {
    /// Signed delta from cursor (relative) or absolute (value-1).
    pub value: i32,
    /// `true` nếu đây là toạ độ absolute (relative page top / margin).
    pub is_absolute: bool,
}

impl Offset {
    /// `Offset::Absolute(value)` (`:234`) — subtract 1 từ VT 1-based.
    pub const fn Absolute(value: i32) -> Self {
        Self {
            value: value - 1,
            is_absolute: true,
        }
    }

    /// `Offset::Forward(value)` (`:235`).
    pub const fn Forward(value: i32) -> Self {
        Self {
            value,
            is_absolute: false,
        }
    }

    /// `Offset::Backward(value)` (`:236`) — negated relative delta.
    pub const fn Backward(value: i32) -> Self {
        Self {
            value: -value,
            is_absolute: false,
        }
    }

    /// `Offset::Unchanged()` (`:237`) — alias `Forward(0)`.
    pub const fn Unchanged() -> Self {
        Self::Forward(0)
    }
}

/// `Mode` — port `adaptDispatch.hpp:202-213` (private enum trong AdaptDispatch).
///
/// C++ src:
/// ```cpp
/// enum class Mode {
///     InsertReplace,
///     Origin,
///     Column,
///     AllowDECCOLM,
///     AllowDECSLRM,
///     SixelDisplay,
///     EraseColor,
///     RectangularChangeExtent,
///     PageCursorCoupling
/// };
/// ```
///
/// Vị trí bit (1 << variant ordinal) cho `ModeSet` mask.
///
/// **Wave 0 port-side deviation** (line_feed LNM): WT lưu LNM bit ở 2 bucket —
/// `_api.GetSystemMode(ITerminalApi::Mode::LineFeed)` (TerminalApi) +
/// `_terminalInput.GetInputMode(TerminalInput::Mode::LineFeed)` (TerminalInput),
/// sync lẫn nhau (`adaptDispatch.cpp:1773-1776` / `1949`). Port `core` gom 2
/// bucket thành 1 (`AdaptDispatch::Mode::LineFeed`) — single-process collapse
/// (xem `td/bell_tabs.rs::line_feed` body Wave 0). Deviation documented,
/// không break semantics cho port single-process (không cross-thread sync).
///
/// Drift so brief prose list: brief liệt kê mode khác (OriginMode, LineFeedMode,
/// AutoWrapMode, CursorKeyMode, ANSI8BitMode, ...) — đây là `Microsoft::Terminal::
/// Core::Mode` (`OutputStateMachineEngine.hpp`), không phải private `AdaptDispatch::Mode`.
/// Port verbatim theo `adaptDispatch.hpp:202-213` — khớp C++ source (9 variant
/// WT-native); `LineFeed = 9` là port-side add.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum Mode {
    /// `_modes.test(Mode::InsertReplace)` (`adaptDispatch.cpp:154`).
    InsertReplace = 0,
    /// `Mode::Origin` (`:345,354,470`).
    Origin = 1,
    /// `Mode::Column`.
    Column = 2,
    /// `Mode::AllowDECCOLM`.
    AllowDECCOLM = 3,
    /// `Mode::AllowDECSLRM`.
    AllowDECSLRM = 4,
    /// `Mode::SixelDisplay`.
    SixelDisplay = 5,
    /// `Mode::EraseColor` (`:529`).
    EraseColor = 6,
    /// `Mode::RectangularChangeExtent`.
    RectangularChangeExtent = 7,
    /// `Mode::PageCursorCoupling` — default (`adaptDispatch.hpp:335`).
    PageCursorCoupling = 8,
    /// `Mode::LineFeed` — LNM bit (Wave 0 port-side add). WT counterpart:
    /// `ITerminalApi::Mode::LineFeed` (`adaptDispatch.cpp:2519`) +
    /// `TerminalInput::Mode::LineFeed` (`adaptDispatch.cpp:1773-1776`).
    /// Single-process collapse — port `core` không có cross-thread api.
    LineFeed = 9,
}

impl Mode {
    /// Bit mask cho mode này (`1 << ordinal`).
    pub const fn bit(self) -> u32 {
        1u32 << (self as u32)
    }
}

/// `ModeSet` — port `til::enumset<Mode>` (`adaptDispatch.hpp:335`).
///
/// W1 dùng plain `u32` bitmask (no `bitflags` crate). `bitflags` không có trong
/// `crates/core/Cargo.toml` — adding it là Cargo scope, không thuộc W1. API
/// surface (`test`/`set`) khớp `til::enumset` semantics.
/// Keyboard modes (`TerminalInput::Mode`): DECCKM, DECKPAM/DECNKM, DECBKM.
/// LNM lives in `TerminalDispatch::modes` as `Mode::LineFeed`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputModes {
    /// DECCKM: unmodified cursor keys use SS3 instead of CSI.
    pub cursor_key: bool,
    /// DECKPAM / DECNKM: keypad application mode.
    pub keypad: bool,
    /// DECBKM: Backspace sends BS instead of DEL (Ctrl inverts).
    pub backarrow: bool,
}

/// Mouse/focus reporting modes requested by the running program.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseModes {
    /// 1005: UTF-8 coordinate encoding.
    pub utf8: bool,
    /// 1000: report button press/release (and wheel).
    pub press_release: bool,
    /// 1002: also report motion while a button is held.
    pub button_motion: bool,
    /// 1003: report all motion.
    pub any_motion: bool,
    /// 1006: SGR encoding (`ESC[<b;x;yM`).
    pub sgr: bool,
    /// 1007: wheel sends arrow keys on the alternate screen.
    pub alternate_scroll: bool,
    /// 1004: report focus in/out (`ESC[I` / `ESC[O`).
    pub focus: bool,
}

impl Default for MouseModes {
    /// `ResetInputModes`: only AlternateScroll (1007) is on by default.
    fn default() -> Self {
        Self {
            utf8: false,
            press_release: false,
            button_motion: false,
            any_motion: false,
            sgr: false,
            alternate_scroll: true,
            focus: false,
        }
    }
}

impl MouseModes {
    /// `SetInputMode` for 1000/1002/1003: changing a tracking mode clears the others.
    pub fn set_tracking(&mut self, which: u16, enable: bool) {
        self.press_release = false;
        self.button_motion = false;
        self.any_motion = false;
        match which {
            1000 => self.press_release = enable,
            1002 => self.button_motion = enable,
            _ => self.any_motion = enable,
        }
    }

    /// `SetInputMode` for 1005/1006: enabling one clears the other encoding.
    pub fn set_encoding(&mut self, utf8: bool, enable: bool) {
        if enable {
            self.utf8 = false;
            self.sgr = false;
        }
        if utf8 {
            self.utf8 = enable;
        } else {
            self.sgr = enable;
        }
    }

    /// Is the program asking for mouse events at all?
    pub fn tracking(&self) -> bool {
        self.press_release || self.button_motion || self.any_motion
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct ModeSet(pub u32);

impl ModeSet {
    pub const EMPTY: Self = Self(0);
    /// `_modes{ Mode::PageCursorCoupling }` — default (`adaptDispatch.hpp:335`).
    pub const DEFAULT: Self = Self(Mode::PageCursorCoupling.bit());

    /// `_modes.test(mode)` — bit được set.
    pub const fn test(self, mode: Mode) -> bool {
        (self.0 & mode.bit()) != 0
    }

    /// `_modes.set(mode, value)` — set (true) hoặc clear (false) bit.
    pub fn set(&mut self, mode: Mode, value: bool) {
        if value {
            self.0 |= mode.bit();
        } else {
            self.0 &= !mode.bit();
        }
    }
}

/// `CursorState` — port `adaptDispatch.hpp:219-228`. Nested struct trong AdaptDispatch.
///
/// C++ src:
/// ```cpp
/// struct CursorState {
///     VTInt Row = 1;
///     VTInt Column = 1;
///     VTInt Page = 1;
///     bool IsDelayedEOLWrap = false;
///     bool IsOriginModeRelative = false;
///     TextAttribute Attributes = {};
///     TerminalOutput TermOutput = {};
/// };
/// ```
///
/// W1 bỏ `TerminalOutput TermOutput` — chưa có Phase 4 terminal_output port.
///
/// Drift về placement: brief nói re-export từ `crate::cursor::CursorState`. Nhưng
/// Phase 4.0-pre chưa land nên `crate::cursor` chưa có type này. Định nghĩa ở
/// `dispatch::td::CursorState` (C++ có nested struct trong AdaptDispatch) —
/// gần với C++ hơn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorState {
    pub row: i32,
    pub column: i32,
    pub page: i32,
    pub is_delayed_eol_wrap: bool,
    pub is_origin_mode_relative: bool,
    pub attributes: crate::attrs::TextAttribute,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            row: 1,
            column: 1,
            page: 1,
            is_delayed_eol_wrap: false,
            is_origin_mode_relative: false,
            attributes: crate::attrs::TextAttribute::default(),
        }
    }
}

/// `TerminalDispatch` — port `adaptDispatch.hpp:33-359` (member list).
///
/// Member fields port verbatim (skip PageManager/Sixel/Font/Macro theo brief).
/// `_pages` (PageManager) collapse thành `terminal: TextBuffer` host (simplification
/// for Phase 4 phase 1 — owns cursor + buffer). `_api`/`_renderer`/
/// `_renderSettings`/`_terminalInput` collapse thành `interact: Box<dyn
/// InteractDispatch>` host bridge.
///
/// Drift so sibling workers (W2/W3/W4) trong `dispatch/td/`: siblings sử dụng
/// `this.pages.active_page().cursor()`, `this.api`, `this.terminal_input`,
/// `this.render_settings` — đấy là_struct shape khác_, mô tả C++ faithful.
/// Brief W1 spec đơn giản hơn (single TextBuffer host). Sibling files not trong
/// mod tree (not declared trong `dispatch/td/mod.rs`) — Land Wave sau sẽ sync.
pub struct TerminalDispatch {
    /// Host — owns cursor + buffer. Port `_pages` page.Buffer() / page.Cursor()
    /// (simplification for Phase 4 phase 1).
    pub terminal: TextBuffer,
    /// Host bridge — title/clipboard/bell/resize/write_file. Port `_api`+部分的
    /// `_renderer`/`_renderSettings`/`_terminalInput` interface.
    pub interact: Box<dyn InteractDispatch>,
    /// `_pages.ActivePage().GetViewport()` cache — port `_pages` viewport.
    pub viewport: Viewport,
    /// `_scrollMargins` (`adaptDispatch.hpp:333`) — top, bottom, left, right.
    /// `til::inclusive_rect` dạng inclusive coords.
    pub scroll_margins: (i32, i32, i32, i32),
    /// `_usingAltBuffer` (`:331`) — index vào `_savedCursorState[2]`.
    pub using_alt_buffer: bool,
    /// `_savedCursorState[2]` (`:330`) — main buffer index 0, alt 1.
    pub saved_cursor_state: [CursorState; 2],
    /// `_tabStopColumns` (`:309`) — `std::vector<uint8_t>`.
    pub tab_stop_columns: Vec<u8>,
    /// `_initDefaultTabStops` (`:310`).
    pub init_default_tab_stops: bool,
    /// `_modes` (`:335`) — `til::enumset<Mode>`.
    pub modes: ModeSet,
    /// `_sgrStack` (`:337`) — Phase 1 đã có `sgr_stack::SgrStack`.
    pub sgr_stack: SgrStack,
    /// `_optionalFeatures` (`:323`) — `til::enumset<OptionalFeature>`.
    pub optional_features: u32,
    /// `Mode::BracketedPaste` (DECSET 2004), read by the control when pasting.
    pub bracketed_paste: bool,
    /// `Mode::AutoWrap` (DECAWM, default on).
    pub autowrap: bool,
    /// DECSET mouse / focus reporting state (`TerminalInput` input modes).
    pub mouse: MouseModes,
    /// Keyboard input modes read when translating keys (`TerminalInput`).
    pub input: InputModes,
}

impl TerminalDispatch {
    /// `AdaptDispatch::AdaptDispatch(...)` ctor (`adaptDispatch.cpp:40-49`).
    ///
    /// W1 simplification: thay vì inject 4 deps C++ (api/renderer/renderSettings/
    /// terminalInput), inject `terminal: TextBuffer` (host) + `interact` bridge.
    ///
    /// Init theo plan §W1:
    /// - `viewport = terminal.get_size()`.
    /// - `scroll_margins = (0, h-1, 0, w-1)`.
    /// - `tab_stop_columns` prefilled every 8 cols (`_initDefaultTabStops=true`).
    /// - `modes = ModeSet::DEFAULT` (= `Mode::PageCursorCoupling`).
    pub fn new(terminal: TextBuffer, interact: Box<dyn InteractDispatch>) -> Self {
        let viewport = terminal.get_size();
        let width = viewport.width();
        let height = viewport.height();
        let mut tab_stop_columns: Vec<u8> = Vec::with_capacity(width as usize);
        // `_InitTabStopsForWidth(width)` (`adaptDispatch.hpp:287`) — fill every 8 col.
        if width > 0 {
            let mut col: i32 = 0;
            while col < width {
                tab_stop_columns.push(if col % 8 == 0 { 1 } else { 0 });
                col += 1;
            }
        }
        Self {
            terminal,
            interact,
            viewport,
            scroll_margins: (0, height - 1, 0, width - 1),
            using_alt_buffer: false,
            saved_cursor_state: [CursorState::default(), CursorState::default()],
            tab_stop_columns,
            init_default_tab_stops: true,
            modes: ModeSet::DEFAULT,
            sgr_stack: SgrStack::new(),
            optional_features: 0,
            bracketed_paste: false,
            autowrap: true,
            mouse: MouseModes::default(),
            input: InputModes::default(),
        }
    }

    /// `_GetVerticalMargins(page, absolute)` (`adaptDispatch.cpp:276-296`).
    ///
    /// W1 simplification: bỏ `Page` abstraction — pass `absolute: bool`,
    /// dùng `viewport.top` làm `page.Top()` (0 cho single-page port phase 1).
    pub fn get_vertical_margins(&mut self, absolute: bool) -> (i32, i32) {
        let bottommost_row = self.viewport.height() - 1;
        let (mut top, mut bottom, _, _) = self.scroll_margins;
        // If top out of range, reset margins completely (`:281`).
        if top >= bottommost_row {
            top = 0;
            bottom = 0;
            self.scroll_margins.0 = 0;
            self.scroll_margins.1 = 0;
        }
        // If margins aren't set, use full extent of the page (`:285`).
        let margins_set = top < bottom;
        let mut top_margin = if margins_set { top } else { 0 };
        let mut bottom_margin = if margins_set { bottom } else { bottommost_row };
        // Clamp bottom into range (`:289`).
        bottom_margin = bottom_margin.min(bottommost_row);
        if absolute {
            top_margin += self.viewport.top;
            bottom_margin += self.viewport.top;
        }
        (top_margin, bottom_margin)
    }

    /// `_GetHorizontalMargins(pageWidth)` (`adaptDispatch.cpp:304-319`).
    pub fn get_horizontal_margins(&self, page_width: i32) -> (i32, i32) {
        let rightmost_column = page_width - 1;
        let (mut left, mut right) = (self.scroll_margins.2, self.scroll_margins.3);
        if left >= rightmost_column {
            left = 0;
            right = 0;
        }
        let margins_set = left < right;
        let left_margin = if margins_set { left } else { 0 };
        let mut right_margin = if margins_set { right } else { rightmost_column };
        right_margin = right_margin.min(rightmost_column);
        (left_margin, right_margin)
    }
}

/// `impl ITermDispatch for TerminalDispatch` — single delegation block.
///
/// This block overrides every method of the `ITermDispatch` trait (124
/// methods total) so that the trait's default `todo!()` bodies in
/// `dispatch/mod.rs` are never reachable at runtime when the e2e parser
/// calls through `Box<dyn ITermDispatch>`. The bodies live as free
/// functions split across `cursor.rs` / `print.rs` / `erase.rs` /
/// `scroll.rs` / `bell_tabs.rs` / `modes.rs` / `dsr.rs` / `charset.rs` /
/// `reset.rs` / `graphics.rs` / `color_table.rs` / `osc.rs` / `dcs.rs`;
/// this block delegates each trait method to its free-fn counterpart. Only
/// ONE `impl Trait for Type` is allowed per type/trait pair (Rust E0119),
/// so all waves funnel through this hub.
///
/// Per Rust E0119: cannot have two `impl ITermDispatch for TerminalDispatch`
/// blocks (cursor.rs / print.rs were both `impl ITermDispatch` → fixed by
/// converting those to inherent `impl TerminalDispatch`).
impl ITermDispatch for TerminalDispatch {
    // —— W1: Unknown / Print + Cursor (17 methods) ——
    fn unknown_sequence(&mut self) {
        TerminalDispatch::unknown_sequence(self);
    }
    fn print(&mut self, c: char) {
        TerminalDispatch::print(self, c);
    }
    fn print_string(&mut self, s: &str) {
        TerminalDispatch::print_string(self, s);
    }
    fn cursor_up(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_up(self, distance);
    }
    fn cursor_down(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_down(self, distance);
    }
    fn cursor_forward(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_forward(self, distance);
    }
    fn cursor_backward(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_backward(self, distance);
    }
    fn cursor_next_line(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_next_line(self, distance);
    }
    fn cursor_prev_line(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_prev_line(self, distance);
    }
    fn cursor_horizontal_position_absolute(&mut self, column: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_horizontal_position_absolute(self, column);
    }
    fn vertical_line_position_absolute(&mut self, line: crate::dispatch::VtInt) {
        TerminalDispatch::vertical_line_position_absolute(self, line);
    }
    fn horizontal_position_relative(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::horizontal_position_relative(self, distance);
    }
    fn vertical_position_relative(&mut self, distance: crate::dispatch::VtInt) {
        TerminalDispatch::vertical_position_relative(self, distance);
    }
    fn cursor_position(&mut self, line: crate::dispatch::VtInt, column: crate::dispatch::VtInt) {
        TerminalDispatch::cursor_position(self, line, column);
    }
    fn cursor_save_state(&mut self) {
        TerminalDispatch::cursor_save_state(self);
    }
    fn cursor_restore_state(&mut self) {
        TerminalDispatch::cursor_restore_state(self);
    }

    // —— W2 Erase / Insert-Delete characters & lines ——
    fn insert_character(&mut self, count: crate::dispatch::VtInt) {
        erase::insert_character(self, count);
    }
    fn delete_character(&mut self, count: crate::dispatch::VtInt) {
        erase::delete_character(self, count);
    }
    fn erase_in_display(&mut self, erase_type: crate::dispatch::EraseType) {
        erase::erase_in_display(self, erase_type);
    }
    fn erase_in_line(&mut self, erase_type: crate::dispatch::EraseType) {
        erase::erase_in_line(self, erase_type);
    }
    fn erase_characters(&mut self, num_chars: crate::dispatch::VtInt) {
        erase::erase_characters(self, num_chars);
    }
    fn selective_erase_in_display(&mut self, erase_type: crate::dispatch::EraseType) {
        erase::selective_erase_in_display(self, erase_type);
    }
    fn selective_erase_in_line(&mut self, erase_type: crate::dispatch::EraseType) {
        erase::selective_erase_in_line(self, erase_type);
    }

    // —— W3 Scroll / Page ——
    fn scroll_up(&mut self, distance: crate::dispatch::VtInt) {
        scroll::scroll_up(self, distance);
    }
    fn scroll_down(&mut self, distance: crate::dispatch::VtInt) {
        scroll::scroll_down(self, distance);
    }
    fn next_page(&mut self, page_count: crate::dispatch::VtInt) {
        scroll::next_page(self, page_count);
    }
    fn preceding_page(&mut self, page_count: crate::dispatch::VtInt) {
        scroll::preceding_page(self, page_count);
    }
    fn page_position_absolute(&mut self, page: crate::dispatch::VtInt) {
        scroll::page_position_absolute(self, page);
    }
    fn page_position_relative(&mut self, page_count: crate::dispatch::VtInt) {
        scroll::page_position_relative(self, page_count);
    }
    fn page_position_back(&mut self, page_count: crate::dispatch::VtInt) {
        scroll::page_position_back(self, page_count);
    }
    fn request_displayed_extent(&mut self) {
        scroll::request_displayed_extent(self);
    }

    // —— W4 Bell / CR / LF / RI / Tabs / Lines / Margins ——
    fn insert_line(&mut self, distance: crate::dispatch::VtInt) {
        bell_tabs::insert_line(self, distance);
    }
    fn delete_line(&mut self, distance: crate::dispatch::VtInt) {
        bell_tabs::delete_line(self, distance);
    }
    fn insert_column(&mut self, distance: crate::dispatch::VtInt) {
        bell_tabs::insert_column(self, distance);
    }
    fn delete_column(&mut self, distance: crate::dispatch::VtInt) {
        bell_tabs::delete_column(self, distance);
    }
    fn set_top_bottom_scrolling_margins(
        &mut self,
        top: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
    ) {
        bell_tabs::set_top_bottom_scrolling_margins(self, top, bottom);
    }
    fn set_left_right_scrolling_margins(
        &mut self,
        left: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
    ) {
        bell_tabs::set_left_right_scrolling_margins(self, left, right);
    }
    fn enquire_answerback(&mut self) {
        bell_tabs::enquire_answerback(self);
    }
    fn warning_bell(&mut self) {
        bell_tabs::warning_bell(self);
    }
    fn carriage_return(&mut self) {
        bell_tabs::carriage_return(self);
    }
    fn line_feed(&mut self, line_feed_type: crate::dispatch::LineFeedType) {
        bell_tabs::line_feed(self, line_feed_type);
    }
    fn reverse_line_feed(&mut self) {
        bell_tabs::reverse_line_feed(self);
    }
    fn back_index(&mut self) {
        bell_tabs::back_index(self);
    }
    fn forward_index(&mut self) {
        bell_tabs::forward_index(self);
    }
    fn horizontal_tab_set(&mut self) {
        bell_tabs::horizontal_tab_set(self);
    }
    fn forward_tab(&mut self, num_tabs: crate::dispatch::VtInt) {
        bell_tabs::forward_tab(self, num_tabs);
    }
    fn backwards_tab(&mut self, num_tabs: crate::dispatch::VtInt) {
        bell_tabs::backwards_tab(self, num_tabs);
    }
    fn tab_clear(&mut self, clear_type: crate::dispatch::TabClearType) {
        bell_tabs::tab_clear(self, clear_type);
    }
    fn tab_set(&mut self, set_type: crate::dispatch::VtParameter) {
        bell_tabs::tab_set(self, set_type);
    }

    // —— W3 Modes (set/reset/request + keypad/ansi/kitty) ——
    fn set_keypad_mode(&mut self, application_mode: bool) {
        modes::set_keypad_mode(self, application_mode);
    }
    fn set_ansi_mode(&mut self, ansi_mode: bool) {
        modes::set_ansi_mode(self, ansi_mode);
    }
    fn set_kitty_keyboard_protocol(
        &mut self,
        flags: crate::dispatch::VtParameter,
        mode: crate::dispatch::VtParameter,
    ) {
        modes::set_kitty_keyboard_protocol(self, flags, mode);
    }
    fn query_kitty_keyboard_protocol(&mut self) {
        modes::query_kitty_keyboard_protocol(self);
    }
    fn push_kitty_keyboard_protocol(&mut self, flags: crate::dispatch::VtParameter) {
        modes::push_kitty_keyboard_protocol(self, flags);
    }
    fn pop_kitty_keyboard_protocol(&mut self, count: crate::dispatch::VtParameter) {
        modes::pop_kitty_keyboard_protocol(self, count);
    }
    fn set_mode(&mut self, param: crate::dispatch::ModeParams) {
        modes::set_mode(self, param);
    }
    fn reset_mode(&mut self, param: crate::dispatch::ModeParams) {
        modes::reset_mode(self, param);
    }
    fn request_mode(&mut self, param: crate::dispatch::ModeParams) {
        modes::request_mode(self, param);
    }

    // —— W3 DSR / DA / DECREQTPARM ——
    fn device_status_report(
        &mut self,
        status_type: crate::dispatch::StatusType,
        id: crate::dispatch::VtParameter,
    ) {
        dsr::device_status_report(self, status_type, id);
    }
    fn device_attributes(&mut self) {
        dsr::device_attributes(self);
    }
    fn secondary_device_attributes(&mut self) {
        dsr::secondary_device_attributes(self);
    }
    fn tertiary_device_attributes(&mut self) {
        dsr::tertiary_device_attributes(self);
    }
    fn vt52_device_attributes(&mut self) {
        dsr::vt52_device_attributes(self);
    }
    fn request_terminal_parameters(&mut self, permission: crate::dispatch::ReportingPermission) {
        dsr::request_terminal_parameters(self, permission);
    }

    // —— W3 Charsets ——
    fn designate_coding_system(&mut self, coding_system: crate::dispatch::VtId) {
        charset::designate_coding_system(self, coding_system);
    }
    fn designate94_charset(
        &mut self,
        gset_number: crate::dispatch::VtInt,
        charset: crate::dispatch::VtId,
    ) {
        charset::designate94_charset(self, gset_number, charset);
    }
    fn designate96_charset(
        &mut self,
        gset_number: crate::dispatch::VtInt,
        charset: crate::dispatch::VtId,
    ) {
        charset::designate96_charset(self, gset_number, charset);
    }
    fn locking_shift(&mut self, gset_number: crate::dispatch::VtInt) {
        charset::locking_shift(self, gset_number);
    }
    fn locking_shift_right(&mut self, gset_number: crate::dispatch::VtInt) {
        charset::locking_shift_right(self, gset_number);
    }
    fn single_shift(&mut self, gset_number: crate::dispatch::VtInt) {
        charset::single_shift(self, gset_number);
    }
    fn accept_c1_controls(&mut self, enabled: bool) {
        charset::accept_c1_controls(self, enabled);
    }
    fn send_c1_controls(&mut self, enabled: bool) {
        charset::send_c1_controls(self, enabled);
    }
    fn announce_code_structure(&mut self, ansi_level: crate::dispatch::VtInt) {
        charset::announce_code_structure(self, ansi_level);
    }

    // —— W4 Reset / Screen alignment / Cursor style ——
    fn soft_reset(&mut self) {
        reset::soft_reset(self);
    }
    fn hard_reset(&mut self, erase: bool) {
        reset::hard_reset(self, erase);
    }
    fn screen_alignment_pattern(&mut self) {
        reset::screen_alignment_pattern(self);
    }
    fn set_cursor_style(&mut self, cursor_style: crate::dispatch::CursorStyle) {
        reset::set_cursor_style(self, cursor_style);
    }

    // —— Phase 5 W1: SGR / line rendition / DECSCA / push-pop (5 methods) ——
    // graphics.rs đã port free-fn `_ApplyGraphicsOption(s)` + `push/pop/line/protection`
    // verbatim từ adaptDispatchGraphics.cpp. Wire vào trait impl tại đây.
    fn set_graphics_rendition(&mut self, options: crate::dispatch::VtParameters) {
        graphics::set_graphics_rendition(self, options);
    }
    fn set_line_rendition(&mut self, rendition: crate::attrs::LineRendition) {
        graphics::set_line_rendition(self, rendition);
    }
    fn set_character_protection_attribute(&mut self, options: crate::dispatch::VtParameters) {
        graphics::set_character_protection_attribute(self, options);
    }
    fn push_graphics_rendition(&mut self, options: crate::dispatch::VtParameters) {
        graphics::push_graphics_rendition(self, options);
    }
    fn pop_graphics_rendition(&mut self) {
        graphics::pop_graphics_rendition(self);
    }

    // —— Phase 5 W1: Color table (9 methods) — verbatim forward sang
    // render_settings/interact. RenderSettings chưa port → // STUB marker.
    fn set_color_table_entry(&mut self, table_index: usize, color: u32) {
        color_table::set_color_table_entry(self, table_index, color);
    }
    fn request_color_table_entry(&mut self, table_index: usize) {
        color_table::request_color_table_entry(self, table_index);
    }
    fn reset_color_table(&mut self) {
        color_table::reset_color_table(self);
    }
    fn reset_color_table_entry(&mut self, table_index: usize) {
        color_table::reset_color_table_entry(self, table_index);
    }
    fn set_xterm_color_resource(&mut self, resource: usize, color: u32) {
        color_table::set_xterm_color_resource(self, resource, color);
    }
    fn request_xterm_color_resource(&mut self, resource: usize) {
        color_table::request_xterm_color_resource(self, resource);
    }
    fn reset_xterm_color_resource(&mut self, resource: usize) {
        color_table::reset_xterm_color_resource(self, resource);
    }
    fn assign_color(
        &mut self,
        item: crate::dispatch::types::ColorItem,
        fg_index: crate::dispatch::VtInt,
        bg_index: crate::dispatch::VtInt,
    ) {
        color_table::assign_color(self, item, fg_index, bg_index);
    }

    // —— Phase 5 W2: OSC sub-actions (12 methods) — free-fn bodies ở `osc.rs`
    // đã forward sang `this.interact.<method>(...)` (InteractDispatch trait
    // method port ở Wave 2). Wire vào trait impl tại đây để trait default
    // `todo!()` (dispatch/mod.rs:233+) KHÔNG còn reachable khi e2e parser
    // gọi qua `Box<dyn ITermDispatch>`.
    fn set_window_title(&mut self, title: &str) {
        osc::set_window_title(self, title);
    }
    fn set_current_working_directory(&mut self, uri: &str) {
        osc::set_current_working_directory(self, uri);
    }
    fn set_clipboard(&mut self, content: &str) {
        osc::set_clipboard(self, content);
    }
    fn window_manipulation(
        &mut self,
        function: crate::dispatch::types::WindowManipulationType,
        params: crate::dispatch::VtParameters,
    ) {
        osc::window_manipulation(self, function, params);
    }
    fn add_hyperlink(&mut self, uri: &str, params: &str) {
        osc::add_hyperlink(self, uri, params);
    }
    fn end_hyperlink(&mut self) {
        osc::end_hyperlink(self);
    }
    fn do_con_emu_action(&mut self, s: &str) {
        osc::do_con_emu_action(self, s);
    }
    fn do_iterm2_action(&mut self, s: &str) {
        osc::do_iterm2_action(self, s);
    }
    fn do_final_term_action(&mut self, s: &str) {
        osc::do_final_term_action(self, s);
    }
    fn do_vs_code_action(&mut self, s: &str) {
        osc::do_vs_code_action(self, s);
    }
    fn do_wt_action(&mut self, s: &str) {
        osc::do_wt_action(self, s);
    }
    fn do_urxvt_action(&mut self, s: &str) {
        osc::do_urxvt_action(self, s);
    }

    // —— Phase 5 W4: DCS / rect-area / Sixel / Macro / Presentation / Sound /
    // Optional-features (21 methods) — free-fn bodies ở `dcs.rs`. Đa số STUB
    // no-op (chờ P08 TextBuffer rect API / P10 RenderSettings / host bridge),
    // nhưng wire vào trait impl để trait default `todo!()` không còn reachable
    // runtime — tương đương WT no-op, không panic khi parser gọi.
    fn change_attributes_rectangular_area(
        &mut self,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
        attrs: crate::dispatch::VtParameters,
    ) {
        dcs::change_attributes_rectangular_area(self, top, left, bottom, right, attrs);
    }
    fn reverse_attributes_rectangular_area(
        &mut self,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
        attrs: crate::dispatch::VtParameters,
    ) {
        dcs::reverse_attributes_rectangular_area(self, top, left, bottom, right, attrs);
    }
    #[allow(clippy::too_many_arguments)]
    fn copy_rectangular_area(
        &mut self,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
        page: crate::dispatch::VtInt,
        dst_top: crate::dispatch::VtInt,
        dst_left: crate::dispatch::VtInt,
        dst_page: crate::dispatch::VtInt,
    ) {
        dcs::copy_rectangular_area(
            self, top, left, bottom, right, page, dst_top, dst_left, dst_page,
        );
    }
    fn fill_rectangular_area(
        &mut self,
        ch: crate::dispatch::VtParameter,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
    ) {
        dcs::fill_rectangular_area(self, ch, top, left, bottom, right);
    }
    fn erase_rectangular_area(
        &mut self,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
    ) {
        dcs::erase_rectangular_area(self, top, left, bottom, right);
    }
    fn selective_erase_rectangular_area(
        &mut self,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
    ) {
        dcs::selective_erase_rectangular_area(self, top, left, bottom, right);
    }
    fn select_attribute_change_extent(&mut self, extent: crate::dispatch::types::ChangeExtent) {
        dcs::select_attribute_change_extent(self, extent);
    }
    fn request_checksum_rectangular_area(
        &mut self,
        id: crate::dispatch::VtInt,
        page: crate::dispatch::VtInt,
        top: crate::dispatch::VtInt,
        left: crate::dispatch::VtInt,
        bottom: crate::dispatch::VtInt,
        right: crate::dispatch::VtInt,
    ) {
        dcs::request_checksum_rectangular_area(self, id, page, top, left, bottom, right);
    }
    fn define_sixel_image(
        &mut self,
        macro_parameter: crate::dispatch::VtInt,
        background_color: crate::dispatch::VtInt,
        private_color_registers: bool,
        handler: crate::dispatch::StringHandler,
    ) {
        dcs::define_sixel_image(
            self,
            macro_parameter,
            background_color,
            private_color_registers,
            handler,
        );
    }
    fn download_drcs(
        &mut self,
        font_number: crate::dispatch::VtInt,
        start_char: crate::dispatch::VtInt,
        cleanup: bool,
        cell_matrix: crate::dispatch::VtInt,
        font_sets: crate::dispatch::VtInt,
        handler: crate::dispatch::StringHandler,
    ) {
        dcs::download_drcs(
            self,
            font_number,
            start_char,
            cleanup,
            cell_matrix,
            font_sets,
            handler,
        );
    }
    fn request_user_preference_charset(&mut self) {
        dcs::request_user_preference_charset(self);
    }
    fn assign_user_preference_charset(
        &mut self,
        charset_size: crate::dispatch::types::CharsetSize,
    ) {
        dcs::assign_user_preference_charset(self, charset_size);
    }
    fn define_macro(
        &mut self,
        macro_id: crate::dispatch::VtInt,
        private_macro: bool,
        handler: crate::dispatch::StringHandler,
    ) {
        dcs::define_macro(self, macro_id, private_macro, handler);
    }
    fn invoke_macro(&mut self, macro_id: crate::dispatch::VtInt) {
        dcs::invoke_macro(self, macro_id);
    }
    fn request_terminal_state_report(
        &mut self,
        format: crate::dispatch::types::ReportFormat,
        format_option: crate::dispatch::VtParameter,
    ) {
        dcs::request_terminal_state_report(self, format, format_option);
    }
    fn restore_terminal_state(&mut self, format: crate::dispatch::types::ReportFormat) {
        dcs::restore_terminal_state(self, format);
    }
    fn request_setting(&mut self) {
        dcs::request_setting(self);
    }
    fn request_presentation_state_report(
        &mut self,
        format: crate::dispatch::types::PresentationReportFormat,
    ) {
        dcs::request_presentation_state_report(self, format);
    }
    fn restore_presentation_state(
        &mut self,
        format: crate::dispatch::types::PresentationReportFormat,
    ) {
        dcs::restore_presentation_state(self, format);
    }
    fn play_sounds(&mut self, parameters: crate::dispatch::VtParameters) {
        dcs::play_sounds(self, parameters);
    }
    fn set_optional_features(&mut self, features: u32) {
        dcs::set_optional_features(self, features);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::interact::InteractDispatch;
    use crate::dispatch::VtInt;

    struct Stub;
    impl InteractDispatch for Stub {
        // Override 7 phương thức có default `todo!()` để free-fn forward test
        // không panic khi gọi qua `Box<dyn InteractDispatch>`.
        fn write_file(&mut self, _s: &str) {}
        fn notify_cursor_change(&mut self) {}
        fn set_window_title(&mut self, _title: &str) {}
        fn set_clipboard(&mut self, _content: &str) {}
        fn resize_window(&mut self, _width: VtInt, _height: VtInt) {}
        fn notify_buffer_change(&mut self) {}
        fn warning_bell(&mut self) {}
        // Phase 4.0-pre — 8 ext methods cũng default `todo!()`, override no-op
        // để test qua `Box<dyn ITermDispatch>` không panic.
        fn is_vt_input_enabled(&self) -> bool {
            true
        }
        fn set_device_attributes(
            &mut self,
            _attributes: crate::dispatch::interact::DeviceAttributeSet,
        ) {
        }
        fn write_input(&mut self, _input_events: &[crate::dispatch::interact::InputRecord]) {}
        fn write_ctrl_key(&mut self, _event: &crate::dispatch::interact::InputRecord) {}
        fn write_string(&mut self, _string: &str) {}
        fn write_string_raw(&mut self, _string: &str) {}
        fn window_manipulation(
            &mut self,
            _function: crate::dispatch::types::WindowManipulationType,
            _parameter1: VtInt,
            _parameter2: VtInt,
        ) {
        }
        fn move_cursor(&mut self, _row: VtInt, _col: VtInt) {}
        // Phase 5 Wave 2: 33 method mode/get/OSC forward — no-op default
        // override để parser gọi không panic khi test free-fn forward path.
        fn set_cursor_key_mode(&mut self, _enable: bool) {}
        fn set_auto_repeat_mode(&mut self, _enable: bool) {}
        fn set_screen_reversed(&mut self, _enable: bool) {}
        fn set_default_mouse_tracking(&mut self, _enable: bool) {}
        fn set_any_event_mouse_tracking(&mut self, _enable: bool) {}
        fn set_focus_event(&mut self, _enable: bool) {}
        fn set_sgr_mouse_encoding(&mut self, _enable: bool) {}
        fn set_alternate_scroll(&mut self, _enable: bool) {}
        fn set_keypad_mode(&mut self, _application_mode: bool) {}
        fn set_ansi_mode(&mut self, _ansi_mode: bool) {}
        fn get_cursor_key_mode(&self) -> bool {
            false
        }
        fn get_screen_reversed(&self) -> bool {
            false
        }
        fn get_autowrap_mode(&self) -> bool {
            false
        }
        fn get_auto_repeat_mode(&self) -> bool {
            false
        }
        fn get_default_mouse_tracking(&self) -> bool {
            false
        }
        fn get_any_event_mouse_tracking(&self) -> bool {
            false
        }
        fn get_focus_event(&self) -> bool {
            false
        }
        fn get_sgr_mouse_encoding(&self) -> bool {
            false
        }
        fn get_alternate_scroll(&self) -> bool {
            false
        }
        fn get_bracketed_paste(&self) -> bool {
            false
        }
        fn set_kitty_keyboard_protocol(&mut self, _flags: u32, _mode: u8) {}
        fn get_kitty_flags(&self) -> u32 {
            0
        }
        fn push_kitty_flags(&mut self, _flags: u32) {}
        fn pop_kitty_flags(&mut self, _count: usize) {}
        fn set_current_working_directory(&mut self, _uri: &str) {}
        fn add_hyperlink(&mut self, _uri: &str, _params: &str) {}
        fn end_hyperlink(&mut self) {}
        fn do_con_emu_action(&mut self, _s: &str) {}
        fn do_iterm2_action(&mut self, _s: &str) {}
        fn do_final_term_action(&mut self, _s: &str) {}
        fn do_vs_code_action(&mut self, _s: &str) {}
        fn do_wt_action(&mut self, _s: &str) {}
        fn do_urxvt_action(&mut self, _s: &str) {}
    }

    #[test]
    fn offset_absolute_subtracts_one() {
        let o = Offset::Absolute(5);
        assert_eq!(o.value, 4);
        assert!(o.is_absolute);
    }

    #[test]
    fn offset_forward_keeps_value() {
        let o = Offset::Forward(7);
        assert_eq!(o.value, 7);
        assert!(!o.is_absolute);
    }

    #[test]
    fn offset_backward_negates() {
        let o = Offset::Backward(3);
        assert_eq!(o.value, -3);
    }

    #[test]
    fn offset_unchanged_is_forward_zero() {
        assert_eq!(Offset::Unchanged(), Offset::Forward(0));
    }

    #[test]
    fn modeset_default_has_page_cursor_coupling() {
        let m = ModeSet::DEFAULT;
        assert!(m.test(Mode::PageCursorCoupling));
        assert!(!m.test(Mode::InsertReplace));
    }

    #[test]
    fn modeset_set_clear_bits() {
        let mut m = ModeSet::EMPTY;
        m.set(Mode::Origin, true);
        assert!(m.test(Mode::Origin));
        m.set(Mode::Origin, false);
        assert!(!m.test(Mode::Origin));
    }

    #[test]
    fn cursor_state_defaults_to_one() {
        let c = CursorState::default();
        assert_eq!((c.row, c.column, c.page), (1, 1, 1));
        assert!(!c.is_delayed_eol_wrap);
        assert!(!c.is_origin_mode_relative);
    }

    #[test]
    fn mode_bit_disjoint() {
        assert_eq!(Mode::InsertReplace.bit(), 1);
        assert_eq!(Mode::Origin.bit(), 2);
        assert_eq!(Mode::PageCursorCoupling.bit(), 256);
    }

    #[test]
    fn ctor_initializes_viewport_and_margins() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let d = TerminalDispatch::new(terminal, Box::new(Stub));
        assert_eq!(d.viewport.width(), 80);
        assert_eq!(d.viewport.height(), 24);
        assert_eq!(d.scroll_margins, (0, 23, 0, 79));
        assert!(!d.using_alt_buffer);
        assert!(d.modes.test(Mode::PageCursorCoupling));
        // Default tab stops every 8 columns.
        assert_eq!(d.tab_stop_columns.len(), 80);
        assert_eq!(d.tab_stop_columns[0], 1);
        assert_eq!(d.tab_stop_columns[8], 1);
        assert_eq!(d.tab_stop_columns[1], 0);
    }

    #[test]
    fn get_vertical_margins_full_when_unset() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let mut d = TerminalDispatch::new(terminal, Box::new(Stub));
        let (top, bottom) = d.get_vertical_margins(false);
        assert_eq!(top, 0);
        assert_eq!(bottom, 23);
    }

    #[test]
    fn get_horizontal_margins_full_when_unset() {
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let d = TerminalDispatch::new(terminal, Box::new(Stub));
        let (left, right) = d.get_horizontal_margins(80);
        assert_eq!(left, 0);
        assert_eq!(right, 79);
    }

    /// E2E-path check: 12 OSC + `set_optional_features` được wire vào
    /// `impl ITermDispatch for TerminalDispatch`, gọi qua `Box<dyn
    /// ITermDispatch>` (path parser thật) không panic. Trước khi wire, các
    /// method này fall through về trait default `todo!()` (`dispatch/mod.rs`)
    /// và panic runtime khi e2e parser gọi.
    #[test]
    fn osc_trait_methods_no_panic_via_box_dyn() {
        use crate::dispatch::types::WindowManipulationType;
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let d: Box<dyn ITermDispatch> = Box::new(TerminalDispatch::new(terminal, Box::new(Stub)));
        let mut d = d;
        d.set_window_title("t");
        d.set_current_working_directory("file:///x");
        d.set_clipboard("c");
        d.window_manipulation(WindowManipulationType::RefreshWindow, &[]);
        d.add_hyperlink("http://x", "");
        d.end_hyperlink();
        d.do_con_emu_action("s");
        d.do_iterm2_action("s");
        d.do_final_term_action("s");
        d.do_vs_code_action("s");
        d.do_wt_action("s");
        d.do_urxvt_action("s");
        d.set_optional_features(0);
        // Pass = trait default `todo!()` unreachable — wire omit runtime panic.
    }

    /// E2E-path check: 20 DCS rect/sixel/macro/report method được wire vào
    /// `impl ITermDispatch for TerminalDispatch`. Body STUB no-op (chờ P08
    /// TextBuffer rect API / P10 / host bridge) nhưng gọi qua `Box<dyn
    /// ITermDispatch>` không panic — tương đương WT no-op.
    #[test]
    fn dcs_trait_methods_no_panic_via_box_dyn() {
        use crate::dispatch::types::{
            ChangeExtent, CharsetSize, PresentationReportFormat, ReportFormat,
        };
        let terminal = TextBuffer::new(80, 24, crate::attrs::TextAttribute::default());
        let d: Box<dyn ITermDispatch> = Box::new(TerminalDispatch::new(terminal, Box::new(Stub)));
        let mut d = d;
        let handler = Box::new(|_: &str| {});
        d.change_attributes_rectangular_area(1, 1, 2, 2, &[]);
        d.reverse_attributes_rectangular_area(1, 1, 2, 2, &[]);
        d.copy_rectangular_area(1, 1, 2, 2, 0, 3, 3, 0);
        d.fill_rectangular_area(0x20, 1, 1, 2, 2);
        d.erase_rectangular_area(1, 1, 2, 2);
        d.selective_erase_rectangular_area(1, 1, 2, 2);
        d.select_attribute_change_extent(ChangeExtent::All);
        d.request_checksum_rectangular_area(0, 0, 1, 1, 2, 2);
        d.define_sixel_image(0, 0, false, handler);
        let handler2 = Box::new(|_: &str| {});
        d.download_drcs(0, 0, false, 0, 0, handler2);
        d.request_user_preference_charset();
        d.assign_user_preference_charset(CharsetSize::Size96);
        let handler3 = Box::new(|_: &str| {});
        d.define_macro(0, false, handler3);
        d.invoke_macro(0);
        d.request_terminal_state_report(ReportFormat::ColorTable, 0);
        d.restore_terminal_state(ReportFormat::ColorTable);
        d.request_setting();
        d.request_presentation_state_report(PresentationReportFormat::CursorInformation);
        d.restore_presentation_state(PresentationReportFormat::CursorInformation);
        d.play_sounds(&[]);
        // Pass = 20 DCS wire reachable, no-op body không panic.
    }
}
