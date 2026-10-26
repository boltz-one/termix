//! `Cursor` + `CursorShape` — port của `windows/src/buffer/out/cursor.h`
//! và `CursorType` (`windows/src/inc/conattrs.hpp:40`).
//!
//! Phase 1 skeleton: struct + getters/setters, bodies `todo!()` cho các
//! thao tác có tương tác với `TextBuffer` (EOL wrap, redraw). Position /
//! visibility / style có default impl thật.

use crate::attrs::TextAttribute;
use crate::buffer::TextBuffer;

/// `CursorState` — port `windows/src/terminal/adapter/adaptDispatch.hpp:219`.
///
/// WT actual `CursorState` (NOT in `cursor.h` — see note below) — used bởi
/// DECSC/DECRC (Save/Restore Cursor) bên trong `AdaptDispatch::_savedCursorState`.
/// Field set:
/// - `Row`, `Column`, `Page` (VTInt = 1-indexed theo VT origin).
/// - `IsDelayedEOLWrap` (bool).
/// - `IsOriginModeRelative` (bool) — DECOM state tại save time.
/// - `Attributes` (TextAttribute).
/// - `TermOutput` (TerminalOutput) — charset state snapshot.
///
/// NOTE về brief anchor: brief đề "port cursor.h:CursorState" — nhưng
/// `cursor.h` không có `CursorState` struct (chỉ có `Cursor` class private
/// fields). WT `CursorState` thật ở `adaptDispatch.hpp:219`. Brief skeleton
/// (column/row/is_visible/is_double_width/attributes) là guess — port từ
/// `adaptDispatch.hpp:219` cho accuracy. W1 worker assume signature dưới —
/// rework if drift.
///
/// `TerminalOutput` (charset translator) chưa port; ta dùng stub opaque
/// struct `TerminalOutputState` (zero-sized) — phase 2 nâng lên full port
/// của `terminalOutput.hpp:23`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalOutputState {
    /// `_grTranslationEnabled` (`terminalOutput.hpp`) — chỉ field tối thiểu
    /// cần cho DECSC/DECRC parity. Phase 2 nâng full.
    pub gr_translation_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CursorState {
    /// `Row = 1` (1-indexed) — port `adaptDispatch.hpp:220`.
    pub row: i32,
    /// `Column = 1` (1-indexed) — port `:221`.
    pub column: i32,
    /// `Page = 1` — port `:222`. Phase 1: không page → luôn 1.
    pub page: i32,
    /// `IsDelayedEOLWrap = false` — port `:223`.
    pub is_delayed_eol_wrap: bool,
    /// `IsOriginModeRelative = false` — port `:224` (DECOM state).
    pub is_origin_mode_relative: bool,
    /// `Attributes = {}` — port `:225` (default TextAttribute).
    pub attributes: TextAttribute,
    /// `TermOutput = {}` — port `:226`. Stub.phase 2 fill.
    pub term_output: TerminalOutputState,
}

impl CursorState {
    /// Default constructor — match WT defaults (Row/Column/Page = 1, rest
    /// bool false, default-constructed Attributes/TermOutput).
    pub const fn new() -> Self {
        Self {
            row: 1,
            column: 1,
            page: 1,
            is_delayed_eol_wrap: false,
            is_origin_mode_relative: false,
            attributes: TextAttribute::new(),
            term_output: TerminalOutputState {
                gr_translation_enabled: false,
            },
        }
    }
}

impl Default for CursorState {
    /// `Row/Column/Page = 1` (WT 1-indexed), rest false/default — port
    /// `CursorState{}` default-init in `adaptDispatch.cpp:45-48`. Delegates to
    /// `new()` to keep `default()`/`new()` parity (derive(Default) would zero
    /// the 1-based fields, breaking WT parity tests).
    fn default() -> Self {
        Self::new()
    }
}

/// `CursorType` — port `conattrs.hpp:40-48`. 6 variants.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum CursorShape {
    /// `Legacy = 0x0` — dùng height để render từ underscore → full box.
    #[default]
    Legacy = 0x0,
    /// `VerticalBar = 0x1` — `|`.
    VerticalBar = 0x1,
    /// `Underscore = 0x2` — nhỏ hơn min-height legacy.
    Underscore = 0x2,
    /// `EmptyBox = 0x3` — outline full box.
    EmptyBox = 0x3,
    /// `FullBox = 0x4` — full box, height=100%.
    FullBox = 0x4,
    /// `DoubleUnderscore = 0x5`.
    DoubleUnderscore = 0x5,
}

impl CursorShape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::VerticalBar => "vertical-bar",
            Self::Underscore => "underscore",
            Self::EmptyBox => "empty-box",
            Self::FullBox => "full-box",
            Self::DoubleUnderscore => "double-underscore",
        }
    }
}

/// `Cursor` — port `cursor.h:24-87`. Phase 1 skeleton — bỏ backref
/// `TextBuffer&` (Rust ownership) thay bằng poem governor ở phase 2.
///
/// `CURSOR_SMALL_SIZE = 25` (`cursor.h:28`).
pub const CURSOR_SMALL_SIZE: u32 = 25;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Cursor {
    /// `_cPosition` (`cursor.h:80`) — (x, y) trong buffer coords.
    pub position: (i32, i32),
    /// `_isVisible = true` (`:84`).
    pub visible: bool,
    /// `_isBlinking = true` (`:85`).
    pub blink: bool,
    /// `_fIsDouble = false` (`:86`) — double cursor size.
    pub is_double: bool,
    /// `_ulSize` (`:82`) — % height (1..100) cho `Legacy`.
    pub size: u32,
    /// `_cursorType = CursorType::Legacy` (`:83`).
    pub style: CursorShape,
    /// Color override (COLORREF packed). `0xffffffff` = inherit text color.
    pub color: u32,
    /// `_mutationId` (`:79`) — phase 2 dùng cho incremental repaint.
    pub mutation_id: u64,
    /// `_delayEOLWrap` (`cursor.h:87`) — last column filled; wrap on next print.
    pub is_delayed_eol_wrap: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            position: (0, 0),
            visible: true,
            blink: true,
            is_double: false,
            size: CURSOR_SMALL_SIZE,
            style: CursorShape::Legacy,
            color: 0xffff_ffff,
            mutation_id: 0,
            is_delayed_eol_wrap: false,
        }
    }
}

impl Cursor {
    pub fn new(size: u32) -> Self {
        Self {
            size,
            ..Self::default()
        }
    }

    /// `IsVisible()` (`cursor.h:40`).
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// `SetIsVisible(bool)` (`:47`).
    pub fn set_visibility(&mut self, v: bool) {
        self.visible = v;
    }

    /// `IsBlinking()` (`:41`).
    pub fn is_blinking(&self) -> bool {
        self.blink
    }

    /// `SetIsBlinking(bool)` (`:48`).
    pub fn set_blinking(&mut self, v: bool) {
        self.blink = v;
    }

    /// `IsDouble()` (`:42`).
    pub fn is_double(&self) -> bool {
        self.is_double
    }

    /// `SetIsDouble(bool)` (`:49`).
    pub fn set_double(&mut self, v: bool) {
        self.is_double = v;
    }

    /// `GetSize()` (`:43`).
    pub fn get_size(&self) -> u32 {
        self.size
    }

    /// `SetSize(ULONG)` (`:50`).
    pub fn set_size(&mut self, s: u32) {
        self.size = s;
    }

    /// `GetPosition()` (`:44`) — trả (x, y).
    pub fn get_position(&self) -> (i32, i32) {
        self.position
    }

    /// `SetPosition(point)` (`:53`).
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position = (x, y);
        self.is_delayed_eol_wrap = false;
    }

    /// `SetXPosition(CoordType)` (`:54`).
    pub fn set_x_position(&mut self, x: i32) {
        self.position.0 = x;
        self.is_delayed_eol_wrap = false;
    }

    /// `SetYPosition(CoordType)` (`:55`).
    pub fn set_y_position(&mut self, y: i32) {
        self.position.1 = y;
        self.is_delayed_eol_wrap = false;
    }

    /// `IncrementXPosition(CoordType)` (`:56`).
    pub fn increment_x(&mut self, dx: i32) {
        self.position.0 += dx;
    }

    /// `IncrementYPosition(CoordType)` (`:57`).
    pub fn increment_y(&mut self, dy: i32) {
        self.position.1 += dy;
    }

    /// `DecrementXPosition(CoordType)` (`:58`).
    pub fn decrement_x(&mut self, dx: i32) {
        self.position.0 -= dx;
    }

    /// `DecrementYPosition(CoordType)` (`:59`).
    pub fn decrement_y(&mut self, dy: i32) {
        self.position.1 -= dy;
    }

    /// `GetType()` (`:45`).
    pub fn get_type(&self) -> CursorShape {
        self.style
    }

    /// `SetType(CursorType)` (`:67`).
    pub fn set_style(&mut self, t: CursorShape) {
        self.style = t;
    }

    /// `SetStyle(ULONG, CursorType)` (`:51`).
    pub fn set_style_with_size(&mut self, size: u32, t: CursorShape) {
        self.size = size;
        self.style = t;
    }

    /// Clamp row vào buffer height — port `Cursor`'s implicit clamp.
    /// Phase 1: simple clamp.
    pub fn clamp_row(&mut self, max_row: i32) {
        if self.position.1 < 0 {
            self.position.1 = 0;
        } else if self.position.1 > max_row {
            self.position.1 = max_row;
        }
    }

    /// `DelayEOLWrap()` (`:63`) — phase 2: tích hợp với `TextBuffer`. Stub.
    pub fn delay_eol_wrap(&mut self) {
        self.is_delayed_eol_wrap = true;
        self.mutation_id = self.mutation_id.wrapping_add(1);
    }

    /// `ResetDelayEOLWrap()` (`:64`).
    pub fn reset_delay_eol_wrap(&mut self) {
        self.is_delayed_eol_wrap = false;
        self.mutation_id = self.mutation_id.wrapping_add(1);
    }

    /// `CopyProperties(const Cursor&)` (`:61`) — copy all fields trừ mutationId.
    pub fn copy_properties(&mut self, other: &Cursor) {
        self.position = other.position;
        self.visible = other.visible;
        self.blink = other.blink;
        self.is_double = other.is_double;
        self.size = other.size;
        self.style = other.style;
        self.color = other.color;
    }

    /// `GetLastMutationId()` (`:39`).
    pub fn get_last_mutation_id(&self) -> u64 {
        self.mutation_id
    }

    /// Bump mutation id — phase 2 renderer dùng để invalidate.
    pub fn bump_mutation(&mut self) {
        self.mutation_id = self.mutation_id.wrapping_add(1);
    }

    /// Phase 2 — re-inst backref tới `TextBuffer` cho `_redrawIfVisible()`.
    /// Phase 1: no-op để skeleton compile được mà không cần cycle.
    pub fn notify_redraw(_buffer: &TextBuffer) {
        // no-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cursor_legacy_25_visible_blinking() {
        let c = Cursor::default();
        assert_eq!(c.get_type(), CursorShape::Legacy);
        assert_eq!(c.get_size(), CURSOR_SMALL_SIZE);
        assert!(c.is_visible());
        assert!(c.is_blinking());
        assert!(!c.is_double());
        assert_eq!(c.get_position(), (0, 0));
        assert_eq!(c.color, 0xffff_ffff);
    }

    #[test]
    fn set_position_and_clamp() {
        let mut c = Cursor::default();
        c.set_position(10, 20);
        assert_eq!(c.get_position(), (10, 20));
        c.set_x_position(5);
        c.increment_y(3);
        assert_eq!(c.get_position(), (5, 23));
        c.clamp_row(15);
        assert_eq!(c.get_position(), (5, 15));
        c.clamp_row(100);
        assert_eq!(c.get_position(), (5, 15));
    }

    #[test]
    fn set_style_with_size() {
        let mut c = Cursor::default();
        c.set_style_with_size(50, CursorShape::VerticalBar);
        assert_eq!(c.get_size(), 50);
        assert_eq!(c.get_type(), CursorShape::VerticalBar);
    }

    #[test]
    fn copy_properties_overlays_rendering_state() {
        let mut a = Cursor::default();
        let mut b = Cursor::default();
        b.set_position(7, 8);
        b.set_style(CursorShape::FullBox);
        b.set_visibility(false);
        b.bump_mutation();
        a.copy_properties(&b);
        assert_eq!(a.get_position(), (7, 8));
        assert_eq!(a.get_type(), CursorShape::FullBox);
        assert!(!a.is_visible());
    }

    // ===== Phase 4.0-pre — CursorState port adaptDispatch.hpp:219 =====

    #[test]
    fn cursor_state_default_matches_wt_defaults() {
        let s = CursorState::default();
        assert_eq!(s.row, 1);
        assert_eq!(s.column, 1);
        assert_eq!(s.page, 1);
        assert!(!s.is_delayed_eol_wrap);
        assert!(!s.is_origin_mode_relative);
        // attributes / term_output default
    }

    #[test]
    fn cursor_state_new_is_const_constructible() {
        const S: CursorState = CursorState::new();
        assert_eq!(S.row, 1);
        assert_eq!(S.column, 1);
    }

    #[test]
    fn cursor_state_clone_preserves_fields() {
        let mut a = CursorState::new();
        a.row = 10;
        a.column = 20;
        a.is_origin_mode_relative = true;
        let b = a.clone();
        assert_eq!(b.row, 10);
        assert_eq!(b.column, 20);
        assert!(b.is_origin_mode_relative);
    }
}
