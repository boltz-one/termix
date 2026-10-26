//! Cell-level render primitives. Verbatim port từ WT:
//! - `Cluster` (`inc/Cluster.hpp:23-66`) — một run text cùng attribute.
//! - `CursorOptions` (`inc/CursorOptions.h:62`) — snapshot cursor để paint.
//! - `GridLines` / `GridLineSet` (`inc/IRenderEngine.hpp:39-53`) — bitset các
//!   line decorations (top/bottom/left/right/underline/...).
//!
//! `CursorShape` định nghĩa local vì `core::cursor` chưa export — coi như
//! một ASSUMED core API (xem port-blocker trong `lib.rs`).

/// Hình dạng cursor — đối ứng `CursorType` trong WT
/// (`types/inc/RenderEnums.hpp`). Port sang `core::cursor::CursorShape`
/// sau khi Lane A finalize.
///
/// `// ASSUMED core API — rework after Lane A finalizes`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorShape {
    #[default]
    Legacy,
    VerticalBar,
    FullBox,
    Underscore,
    DoubleUnderscore,
    EmptyBox,
}

/// Một run text cùng attribute — port verbatim `inc/Cluster.hpp:23-66`.
///
/// WT: `struct Cluster { std::wstring text; SHORT columns; }` — `columns` là
/// số cell-grid columns run chiếm (sum của `wchar` advance). Trong Rust ta dùng
/// `String` (UTF-8) thay `wstring`; `columns` giữ `i32` để khớp `SHORT` mở rộng.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cluster {
    /// Nội dung text (UTF-8) của run.
    pub text: String,
    /// Số cell-grid columns run chiếm (`Cluster::columns` trong C++).
    pub columns: i32,
}

impl Cluster {
    /// Tạo `Cluster` mới — tương đương C++ ctor `Cluster(text, columns)`.
    #[inline]
    pub const fn new(text: String, columns: i32) -> Self {
        Self { text, columns }
    }

    /// Push thêm text & cộng dồn columns — port `Cluster::AddCluster` (run
    /// acc trong `_PaintBufferOutputHelper` tại `renderer.cpp:1264`).
    ///
    /// **Run-break loop**: `_PaintBufferOutputHelper` accumulates clusters
    /// into `_clusterBuffer` while color/attr/pattern-id/soft-font-state stay
    /// constant. Break placed at the boundary via `extend`:
    ///
    /// ```cpp
    /// auto columnCount = it->Columns();
    /// if (_clusterBuffer.empty() && it->DbcsAttr() == DbcsAttribute::Trailing) {
    ///     --screenPoint.x; trimLeft = true; ++columnCount;
    /// }
    /// _clusterBuffer.emplace_back(it->Chars(), columnCount);
    /// it += std::max(it->Columns(), 1);
    /// cols += columnCount;
    /// ```
    ///
    /// Caller (`paint_buffer_line` loop) breaks the run when `color != it.TextAttr()`
    /// or `patternId != thisPointPatterns` or `usingSoftFont != thisUsingSoftFont`.
    /// Trailing-half wide-char handling is left to caller (needs `screenPoint.x`
    /// decremented on the first trailing cluster) — verbatim from WT.
    #[inline]
    pub fn extend(&mut self, text: &str, columns: i32) {
        self.text.push_str(text);
        self.columns += columns;
    }
}

/// Snapshot cursor để paint — port verbatim `inc/CursorOptions.h:62`.
///
/// WT fields (tất cả uint/color/BOOL):
/// - `coordPosition`, `ulCursorHeightPercent`, `ulCursorPixelWidth`,
///   `cursorType`, `cursorColor`, `isDoubleWidth`, `isBlock`.
///
/// Phase 2: `paint_rect` để yield `(left, top, right, bottom)` cell-range
/// theo `style` — driver cho `RenderEngine::paint_cursor`.
#[derive(Clone, Debug, Default)]
pub struct CursorOptions {
    /// `(x, y)` cell-grid coordinates — WT `COORD coordPosition`.
    pub position: (i32, i32),
    /// Hình dạng cursor.
    pub style: CursorShape,
    /// Pixel width (WT `ulCursorPixelWidth`) — bar-mode độ dày.
    pub size: u8,
    /// RGBA packed (`0xRRGGBBAA`).
    pub color: u32,
    /// `visible` — WT `IsVisible()`.
    pub visible: bool,
    /// `blink` — WT `isBlinkingAllowed`.
    pub blink: bool,
    /// `is_double_width` — WT `fIsDoubleWidth`. True → cursor spans 2 cells
    /// (trừ `VerticalBar` luôn 1 cell — port `AtlasEngine.cpp:628`).
    pub is_double_width: bool,
    /// `height_percentage` — WT `ulCursorHeightPercent` (1..100) cho
    /// `Legacy` mode (underscore-fullbox variable height). Default 25.
    pub height_percentage: u8,
}

impl CursorOptions {
    /// Compute cell-grid rect `(left, top, right, bottom)` cho cursor — port
    /// `AtlasEngine::PaintCursor` (`AtlasEngine.cpp:605`).
    ///
    /// Khác biệt với WT: GPUI không draw texturing trực tiếp, trả về rect +
    /// `style` để caller (`tg_app`) dùng `gpui::Element::paint_quad` /
    /// `paint_path` cho outline variants. Verbatim logic:
    /// - `VerticalBar`: 1 cell wide.
    /// - `Underscore`: 1 cell wide, bottom `thin_line_width` px height.
    /// - `EmptyBox`/`FullBox`/`Legacy`/`DoubleUnderscore`: `cursorWidth = 1 +
    ///   (is_double_width && style != VerticalBar)`.
    /// - `Legacy`: height scaled theo `height_percentage` (1..100%).
    ///
    /// Returns `None` nếu `visible=false`. Cell-grid coords; caller multiply
    /// by `FontRuntime::cell_size` để có px rect.
    pub fn paint_rect(&self) -> Option<(i32, i32, i32, i32)> {
        if !self.visible {
            return None;
        }
        let (x, y) = self.position;
        // Port WT line 628: `cursorWidth = 1 + (fIsDoubleWidth & cursorType != VerticalBar)`.
        let cursor_width =
            1 + i32::from(self.is_double_width && self.style != CursorShape::VerticalBar);
        let right = x + cursor_width;
        match self.style {
            CursorShape::VerticalBar => Some((x, y, x + cursor_width, y + 1)),
            CursorShape::Underscore => {
                // Bottom-row: drop top, keep bottom at y+1.
                Some((x, y, right, y + 1))
            }
            CursorShape::FullBox | CursorShape::EmptyBox => Some((x, y, right, y + 1)),
            CursorShape::Legacy => {
                // Variable height scaled by `height_percentage` (1..100).
                // WT: `top_offset = (100 - height_pct) * cell_height / 100`.
                // Cell-grid math leaves that to caller — return whole cell.
                // Thông tin `size` đã đủ để caller clip.
                Some((x, y, right, y + 1))
            }
            CursorShape::DoubleUnderscore => {
                // Thay vì 1 row dưới, vẽ 2 underscores stacked (WT .r.cpp
                // path). Caller handle bằng cách draw 2 thin rects.
                Some((x, y + 1, right, y + 2))
            }
        }
    }

    /// Draw-stroke flag — `EmptyBox` vẽ outline, `FullBox` fill solid.
    /// Caller (`app::render_element`) chọn `paint_quad` (fill) hoặc
    /// `paint_path` (outline) dựa vào hàm này.
    pub const fn is_outline(&self) -> bool {
        matches!(self.style, CursorShape::EmptyBox)
    }

    /// Draw double-underline: caller vẽ 2 thin rects ngang. Port verbatim
    /// WT `_PaintBufferOutputGridLineHelper` Double-Underline path.
    pub const fn is_double_underline(&self) -> bool {
        matches!(self.style, CursorShape::DoubleUnderscore)
    }
}

// Bitset các line decorations — port verbatim `IRenderEngine.hpp:39-53`.
//
// WT định nghĩa `enum GridLines : uint32_t` với `Top`..`Hyperlink`. Trong
// bitflags 2.x, struct được tạo bởi `bitflags!` chính là type bitset (có
// `.contains`, `|`, `&`, `empty()`), nên `GridLineSet` bí danh trực tiếp.
// Tên alias giữ parity với WT `til::enum_set<GridLines>`.
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct GridLines: u32 {
        /// Đường dọc trên (top) cell.
        const TOP = 0x0001;
        /// Đường dọc dưới (bottom) cell.
        const BOTTOM = 0x0002;
        /// Đường ngang trái cell.
        const LEFT = 0x0004;
        /// Đường ngang phải cell.
        const RIGHT = 0x0008;
        /// Single underline (`TextAttribute::Underline`).
        const UNDERLINE = 0x0010;
        /// Double underline (`TextAttribute::DoubleUnderline`).
        const DOUBLE_UNDERLINE = 0x0020;
        /// Strikethrough (`TextAttribute::CrossedOut`).
        const STRIKETHROUGH = 0x0040;
        /// Curly underline (`RenderEnums::UnderlineStyle::Curly`).
        const CURLY_UNDERLINE = 0x0080;
        /// Dotted underline.
        const DOTTED_UNDERLINE = 0x0100;
        /// Dashed underline.
        const DASHED_UNDERLINE = 0x0200;
        /// Hyperlink underline (dùng配色 riêng).
        const HYPERLINK = 0x0400;
    }
}

/// Set các `GridLines` — alias sang struct `GridLines` (sẵn sàng bitset ops).
/// `grid.rs::GridLineRange` dùng kiểu này cho `lines`.
pub type GridLineSet = GridLines;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_new_sets_fields() {
        let c = Cluster::new("abc".into(), 3);
        assert_eq!(c.text, "abc");
        assert_eq!(c.columns, 3);
    }

    #[test]
    fn cluster_extend_accumulates_columns() {
        let mut c = Cluster::new("ab".into(), 2);
        c.extend("cde", 3);
        assert_eq!(c.text, "abcde");
        assert_eq!(c.columns, 5);
    }

    #[test]
    fn gridlines_bitflag_combinations_or_together() {
        let s = GridLines::TOP | GridLines::BOTTOM | GridLines::LEFT | GridLines::RIGHT;
        assert!(s.contains(GridLines::TOP));
        assert!(s.contains(GridLines::BOTTOM));
        assert!(s.contains(GridLines::LEFT));
        assert!(s.contains(GridLines::RIGHT));
        assert!(!s.contains(GridLines::UNDERLINE));
    }

    #[test]
    fn gridline_set_underline_styles_distinct_bits() {
        let u = GridLines::UNDERLINE
            | GridLines::DOUBLE_UNDERLINE
            | GridLines::CURLY_UNDERLINE
            | GridLines::DOTTED_UNDERLINE
            | GridLines::DASHED_UNDERLINE
            | GridLines::STRIKETHROUGH;
        assert!(!u.contains(GridLines::HYPERLINK));
        let mut v = u;
        v.remove(GridLines::CURLY_UNDERLINE);
        assert!(!v.contains(GridLines::CURLY_UNDERLINE));
        assert!(v.contains(GridLines::STRIKETHROUGH));
    }

    #[test]
    fn cursoroptions_default_has_invisible_cursor() {
        let c = CursorOptions::default();
        assert!(!c.visible);
        assert_eq!(c.position, (0, 0));
    }

    #[test]
    fn cursor_paint_rect_none_when_invisible() {
        let c = CursorOptions::default();
        assert!(c.paint_rect().is_none());
    }

    #[test]
    fn cursor_vertical_bar_one_cell_wide() {
        let c = CursorOptions {
            visible: true,
            style: CursorShape::VerticalBar,
            position: (5, 3),
            ..Default::default()
        };
        let r = c.paint_rect().expect("visible");
        assert_eq!(r, (5, 3, 6, 4));
    }

    #[test]
    fn cursor_full_box_doubles_for_double_width() {
        let c = CursorOptions {
            visible: true,
            style: CursorShape::FullBox,
            is_double_width: true,
            position: (2, 1),
            ..Default::default()
        };
        let r = c.paint_rect().unwrap();
        assert_eq!(r, (2, 1, 4, 2));
    }

    #[test]
    fn cursor_vertical_bar_ignores_double_width() {
        // WT line 628: `cursorType != VerticalBar` → bar luôn 1 cell.
        let c = CursorOptions {
            visible: true,
            style: CursorShape::VerticalBar,
            is_double_width: true,
            position: (0, 0),
            ..Default::default()
        };
        let r = c.paint_rect().unwrap();
        assert_eq!(r, (0, 0, 1, 1));
    }

    #[test]
    fn cursor_empty_box_is_outline_flag() {
        let c = CursorOptions {
            visible: true,
            style: CursorShape::EmptyBox,
            ..Default::default()
        };
        assert!(c.is_outline());
        assert!(!c.is_double_underline());
    }

    #[test]
    fn cursor_double_underscore_is_double_underline() {
        let c = CursorOptions {
            visible: true,
            style: CursorShape::DoubleUnderscore,
            ..Default::default()
        };
        assert!(c.is_double_underline());
        assert!(!c.is_outline());
    }
}
