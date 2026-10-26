//! `TextAttribute` + `LineRendition` — port của
//! `windows/src/buffer/out/TextAttribute.hpp` và `LineRendition.hpp`.
//!
//! Phase 1 skeleton: struct + getter/setter stubs trả default (`false`/`0`).
//! C++ field set (`_attrs`, `_foreground`, `_background`, `_hyperlinkId`,
//! `_underlineColor`, `_markKind`) được mo-dell hoá thành flat bool/u8/u32
//! để renderer/control compile được mà không cần `TextColor` full impl.
//!
//! Anchors:
//! - `TextAttribute.hpp:50-235` — class TextAttribute.
//! - `LineRendition.hpp:16` — enum (4 variants) + `ScreenToBufferLine` /
//!   `BufferToScreenLine` helpers.

use core::fmt;

/// Kiểu underline — port `UnderlineStyle` (`TextAttribute.hpp:30-39`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum UnderlineStyle {
    #[default]
    NoUnderline = 0,
    SinglyUnderlined = 1,
    DoublyUnderlined = 2,
    CurlyUnderlined = 3,
    DottedUnderlined = 4,
    DashedUnderlined = 5,
}

/// Đánh dấu prompt/command/output — port `MarkKind` (`TextAttribute.hpp:42-48`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(u16)]
pub enum MarkKind {
    #[default]
    None = 0,
    Prompt = 1,
    Command = 2,
    Output = 3,
}

/// How a foreground/background colour is specified (`TextColor`'s type tag).
/// `foreground`/`background` hold the RGB for `Rgb`; `fg_index`/`bg_index` the
/// palette index for the indexed kinds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ColorKind {
    /// Terminal default colour.
    #[default]
    Default,
    /// One of the 16 legacy ANSI colours (SGR 30-37/40-47/90-97/100-107).
    Indexed16,
    /// `38;5;n` / `48;5;n` — index into the 256-colour table.
    Indexed256,
    /// `38;2;r;g;b` / `48;2;r;g;b`.
    Rgb,
}

/// `TextAttribute` — port verbatim từ `TextAttribute.hpp:50`.
///
/// Phase 1: FG/BG mo-dell hoá thành packed `u32` (ABGR, khớp
/// `settings::Color`). Khi phase 2 cần `TextColor` full (legacy +
/// 256 + RGB index), nâng cấp field type ở đây.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TextAttribute {
    /// `CharacterAttributes` bitset xả ra thành flat bools — đủ cho renderer.
    pub intense: bool,
    pub faint: bool,
    pub italic: bool,
    pub blink: bool,
    pub invisible: bool,
    pub crossed_out: bool,
    pub reverse_video: bool,
    pub protected: bool,
    pub top_gridline: bool,
    pub bottom_gridline: bool,
    pub left_gridline: bool,
    pub right_gridline: bool,
    pub overlined: bool,

    /// Packed ABGR (`settings::Color`). 0 = default.
    pub foreground: u32,
    pub background: u32,
    /// Which of the colour fields above is meaningful (see [`ColorKind`]).
    pub fg_kind: ColorKind,
    pub bg_kind: ColorKind,
    pub underline_color: u32,
    pub underline_style: UnderlineStyle,

    /// Index 0..256 cho indexed colors; (`is_legacy` true → dùng legacy 16).
    pub is_legacy: bool,
    pub fg_index: u8,
    pub bg_index: u8,

    pub hyperlink_id: u16,
    pub mark_kind: MarkKind,
}

impl TextAttribute {
    /// `TextAttribute()` default ctor (`TextAttribute.hpp:53`).
    pub const fn new() -> Self {
        Self {
            intense: false,
            faint: false,
            italic: false,
            blink: false,
            invisible: false,
            crossed_out: false,
            reverse_video: false,
            protected: false,
            top_gridline: false,
            bottom_gridline: false,
            left_gridline: false,
            right_gridline: false,
            overlined: false,
            foreground: 0,
            background: 0,
            fg_kind: ColorKind::Default,
            bg_kind: ColorKind::Default,
            underline_color: 0,
            underline_style: UnderlineStyle::NoUnderline,
            is_legacy: false,
            fg_index: 0,
            bg_index: 0,
            hyperlink_id: 0,
            mark_kind: MarkKind::None,
        }
    }

    /// `GetForeground()` (`TextAttribute.hpp:162`). Phase 2: trả `TextColor`.
    pub fn get_foreground(&self) -> u32 {
        self.foreground
    }

    /// `SetForeground(TextColor)` / `SetForeground(COLORREF)` (`:167,170`).
    pub fn set_foreground(&mut self, color: u32) {
        self.foreground = color;
        self.fg_kind = ColorKind::Rgb;
        self.is_legacy = false;
    }

    /// `GetBackground()` (`:163`).
    pub fn get_background(&self) -> u32 {
        self.background
    }

    /// `SetBackground(TextColor)` / `SetBackground(COLORREF)` (`:168,171`).
    pub fn set_background(&mut self, color: u32) {
        self.background = color;
        self.bg_kind = ColorKind::Rgb;
        self.is_legacy = false;
    }

    /// `SetIndexedForeground(BYTE)` (`:172`).
    pub fn set_indexed_foreground(&mut self, idx: u8) {
        self.fg_index = idx;
        self.fg_kind = ColorKind::Indexed16;
        self.is_legacy = true;
    }

    /// `SetIndexedBackground(BYTE)` (`:173`).
    pub fn set_indexed_background(&mut self, idx: u8) {
        self.bg_index = idx;
        self.bg_kind = ColorKind::Indexed16;
        self.is_legacy = true;
    }

    /// `SetIndexedForeground256(BYTE)` (`TextAttribute.hpp:174`). 256-color
    /// table (NOT legacy 16) — turn off `is_legacy` để renderer dùng 256-table.
    pub fn set_indexed_foreground_256(&mut self, idx: u8) {
        self.fg_index = idx;
        self.fg_kind = ColorKind::Indexed256;
        self.is_legacy = false;
    }

    /// `SetIndexedBackground256(BYTE)` (`:175`).
    pub fn set_indexed_background_256(&mut self, idx: u8) {
        self.bg_index = idx;
        self.bg_kind = ColorKind::Indexed256;
        self.is_legacy = false;
    }

    /// `SetColor(COLORREF, bool isForeground)` (`TextAttribute.hpp:176-178`).
    /// Phase 1: packed RGB → u32 foreground/background; reset `is_legacy`.
    pub fn set_color(&mut self, rgb: u32, is_foreground: bool) {
        if is_foreground {
            self.foreground = rgb;
            self.fg_kind = ColorKind::Rgb;
        } else {
            self.background = rgb;
            self.bg_kind = ColorKind::Rgb;
        }
        self.is_legacy = false;
    }

    pub fn get_underline_color(&self) -> u32 {
        self.underline_color
    }

    pub fn set_underline_color(&mut self, color: u32) {
        self.underline_color = color;
    }

    pub fn get_underline_style(&self) -> UnderlineStyle {
        self.underline_style
    }

    pub fn set_underline_style(&mut self, style: UnderlineStyle) {
        self.underline_style = style;
    }

    pub fn get_hyperlink_id(&self) -> u16 {
        self.hyperlink_id
    }

    pub fn set_hyperlink_id(&mut self, id: u16) {
        self.hyperlink_id = id;
    }

    pub fn get_mark_attributes(&self) -> MarkKind {
        self.mark_kind
    }

    pub fn set_mark_attributes(&mut self, kind: MarkKind) {
        self.mark_kind = kind;
    }

    // — Boolean getters — port `Is*()` (`TextAttribute.hpp:118-129`).

    pub fn is_bold(&self, _intense_is_bold: bool) -> bool {
        self.intense
    }

    pub fn is_legacy(&self) -> bool {
        self.is_legacy
    }

    pub fn is_intense(&self) -> bool {
        self.intense
    }

    pub fn is_faint(&self) -> bool {
        self.faint
    }

    pub fn is_italic(&self) -> bool {
        self.italic
    }

    pub fn is_blinking(&self) -> bool {
        self.blink
    }

    pub fn is_invisible(&self) -> bool {
        self.invisible
    }

    pub fn is_crossed_out(&self) -> bool {
        self.crossed_out
    }

    pub fn is_underlined(&self) -> bool {
        self.underline_style != UnderlineStyle::NoUnderline
    }

    pub fn is_overlined(&self) -> bool {
        self.overlined
    }

    pub fn is_reverse_video(&self) -> bool {
        self.reverse_video
    }

    pub fn is_protected(&self) -> bool {
        self.protected
    }

    pub fn is_hyperlink(&self) -> bool {
        self.hyperlink_id != 0
    }

    pub fn is_top_horizontal_displayed(&self) -> bool {
        self.top_gridline
    }

    pub fn is_bottom_horizontal_displayed(&self) -> bool {
        self.bottom_gridline
    }

    pub fn is_left_vertical_displayed(&self) -> bool {
        self.left_gridline
    }

    pub fn is_right_vertical_displayed(&self) -> bool {
        self.right_gridline
    }

    pub fn is_any_grid_line_enabled(&self) -> bool {
        self.top_gridline || self.bottom_gridline || self.left_gridline || self.right_gridline
    }

    pub fn has_any_visual_attributes(&self) -> bool {
        self.intense
            || self.faint
            || self.italic
            || self.blink
            || self.invisible
            || self.crossed_out
            || self.reverse_video
            || self.protected
            || self.is_any_grid_line_enabled()
            || self.overlined
            || self.is_underlined()
            || self.is_hyperlink()
    }

    // — Boolean setters — port `Set*()` (`TextAttribute.hpp:131-140`).

    pub fn set_intense(&mut self, v: bool) {
        self.intense = v;
    }

    pub fn set_faint(&mut self, v: bool) {
        self.faint = v;
    }

    pub fn set_italic(&mut self, v: bool) {
        self.italic = v;
    }

    pub fn set_blinking(&mut self, v: bool) {
        self.blink = v;
    }

    pub fn set_invisible(&mut self, v: bool) {
        self.invisible = v;
    }

    pub fn set_crossed_out(&mut self, v: bool) {
        self.crossed_out = v;
    }

    pub fn set_overlined(&mut self, v: bool) {
        self.overlined = v;
    }

    pub fn set_reverse_video(&mut self, v: bool) {
        self.reverse_video = v;
    }

    pub fn set_protected(&mut self, v: bool) {
        self.protected = v;
    }

    pub fn set_left_vertical_displayed(&mut self, v: bool) {
        self.left_gridline = v;
    }

    pub fn set_right_vertical_displayed(&mut self, v: bool) {
        self.right_gridline = v;
    }

    /// `Invert()` (`TextAttribute.hpp:106`).
    pub fn invert(&mut self) {
        self.reverse_video = !self.reverse_video;
    }

    /// `SetStandardErase()` (`:186`) — reset rendition + reverse, giữ màu.
    pub fn set_standard_erase(&mut self) {
        self.intense = false;
        self.faint = false;
        self.italic = false;
        self.blink = false;
        self.invisible = false;
        self.crossed_out = false;
        self.reverse_video = false;
        self.overlined = false;
        self.underline_style = UnderlineStyle::NoUnderline;
        self.top_gridline = false;
        self.bottom_gridline = false;
        self.left_gridline = false;
        self.right_gridline = false;
    }

    /// `SetDefaultRenditionAttributes()` (`:182`).
    pub fn set_default_rendition_attributes(&mut self) {
        self.set_standard_erase();
    }

    /// `SetDefaultForeground()` (`:179`).
    pub fn set_default_foreground(&mut self) {
        self.foreground = 0;
        self.fg_index = 0;
        self.fg_kind = ColorKind::Default;
    }

    /// `SetDefaultBackground()` (`:180`).
    pub fn set_default_background(&mut self) {
        self.background = 0;
        self.bg_index = 0;
        self.bg_kind = ColorKind::Default;
    }

    /// `SetDefaultUnderlineColor()` (`:181`).
    pub fn set_default_underline_color(&mut self) {
        self.underline_color = 0;
    }

    /// `BackgroundIsDefault()` (`:184`).
    pub fn background_is_default(&self) -> bool {
        self.bg_kind == ColorKind::Default
    }
}

impl Default for TextAttribute {
    fn default() -> Self {
        Self::new()
    }
}

/// `LineRendition` — port `LineRendition.hpp:16`. DECSWL/DECDWL/DECDHL.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum LineRendition {
    #[default]
    SingleWidth = 0,
    DoubleWidth = 1,
    DoubleHeightTop = 2,
    DoubleHeightBottom = 3,
}

impl LineRendition {
    /// Tên chuẩn cho log/JSON — khớp naming WT internals.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleWidth => "single-width",
            Self::DoubleWidth => "double-width",
            Self::DoubleHeightTop => "double-height-top",
            Self::DoubleHeightBottom => "double-height-bottom",
        }
    }

    /// `IsDoubleHeight()` helper (`LineRendition.hpp`).
    pub fn is_double_height(self) -> bool {
        matches!(self, Self::DoubleHeightTop | Self::DoubleHeightBottom)
    }

    /// `IsDoubleWidth()` helper.
    pub fn is_double_width(self) -> bool {
        !matches!(self, Self::SingleWidth)
    }

    /// `ScreenToBufferLine` (`LineRendition.hpp`) — map screen line ↔ buffer
    /// line cho double-height. Phase 1: identity stub.
    pub fn screen_to_buffer_line(self, screen_line: i32) -> i32 {
        match self {
            Self::DoubleHeightTop => screen_line * 2,
            Self::DoubleHeightBottom => screen_line * 2 + 1,
            _ => screen_line,
        }
    }

    /// `BufferToScreenLine` — reciprocity của `screen_to_buffer_line`.
    pub fn buffer_to_screen_line(self, buffer_line: i32) -> i32 {
        match self {
            Self::DoubleHeightTop => buffer_line / 2,
            Self::DoubleHeightBottom => buffer_line / 2,
            _ => buffer_line,
        }
    }
}

impl fmt::Display for LineRendition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_attribute_is_blank() {
        let a = TextAttribute::default();
        assert_eq!(a.get_foreground(), 0);
        assert_eq!(a.get_background(), 0);
        assert!(!a.is_italic());
        assert!(!a.is_bold(false));
        assert!(!a.is_reverse_video());
        assert!(!a.is_underlined());
        assert!(!a.is_any_grid_line_enabled());
        assert!(!a.has_any_visual_attributes());
        assert!(!a.is_legacy());
    }

    #[test]
    fn setters_mutate_flags() {
        let mut a = TextAttribute::default();
        a.set_italic(true);
        a.set_reverse_video(true);
        a.set_foreground(0xff000000);
        assert!(a.is_italic());
        assert!(a.is_reverse_video());
        assert_eq!(a.get_foreground(), 0xff000000);
        assert!(!a.is_legacy());
    }

    #[test]
    fn indexed_colors_set_legacy_flag() {
        let mut a = TextAttribute::default();
        a.set_indexed_foreground(7);
        a.set_indexed_background(1);
        assert!(a.is_legacy());
        assert_eq!(a.fg_index, 7);
        assert_eq!(a.bg_index, 1);
    }

    #[test]
    fn standard_erase_resets_rendition_only() {
        let mut a = TextAttribute::default();
        a.set_italic(true);
        a.set_reverse_video(true);
        a.set_foreground(0x11223344);
        a.set_standard_erase();
        assert!(!a.is_italic());
        assert!(!a.is_reverse_video());
        assert_eq!(a.get_foreground(), 0x11223344, "color preserved");
    }

    #[test]
    fn invert_toggles_reverse() {
        let mut a = TextAttribute::default();
        assert!(!a.is_reverse_video());
        a.invert();
        assert!(a.is_reverse_video());
        a.invert();
        assert!(!a.is_reverse_video());
    }

    #[test]
    fn line_rendition_variants_and_helpers() {
        assert_eq!(LineRendition::default(), LineRendition::SingleWidth);
        assert!(!LineRendition::SingleWidth.is_double_width());
        assert!(LineRendition::DoubleWidth.is_double_width());
        assert!(LineRendition::DoubleHeightTop.is_double_height());
        assert!(LineRendition::DoubleHeightBottom.is_double_height());
        assert_eq!(LineRendition::DoubleHeightTop.as_str(), "double-height-top");
        assert_eq!(LineRendition::DoubleHeightTop.screen_to_buffer_line(3), 6);
        assert_eq!(
            LineRendition::DoubleHeightBottom.screen_to_buffer_line(3),
            7
        );
        assert_eq!(LineRendition::SingleWidth.screen_to_buffer_line(9), 9);
        assert_eq!(LineRendition::DoubleHeightTop.buffer_to_screen_line(7), 3);
    }
}
