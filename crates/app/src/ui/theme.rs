//! `Theme` — token màu cho chrome. Port rút gọn của các `ThemeResource` WT
//! (`TabViewBackground`, `CardStrokeColorDefaultBrush`, `CaptionButton*`,
//! `CloseButton*`) cho dark/light. Giá trị lấy từ WinUI 2 default palette.

use gpui::{rgb, rgba, Rgba};

/// Bảng màu chrome. Mọi field là màu đã resolve (không còn brush lookup).
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// `ApplicationPageBackgroundThemeBrush` — nền page / infobar.
    pub page_bg: Rgba,
    /// `TabViewBackground` — nền tab row + titlebar.
    pub tab_row_bg: Rgba,
    /// Tab đang chọn (`TabViewItemHeaderBackgroundSelected`).
    pub tab_selected_bg: Rgba,
    /// Tab hover (`TabViewItemHeaderBackgroundPointerOver`).
    pub tab_hover_bg: Rgba,
    /// `SystemControlForegroundBaseHighBrush`.
    pub fg: Rgba,
    /// `SystemControlForegroundBaseMediumBrush` — tab không chọn, glyph phụ.
    pub fg_dim: Rgba,
    /// `CardStrokeColorDefaultBrush` — đường kẻ chân titlebar.
    pub stroke: Rgba,
    /// `CaptionButtonBackgroundPointerOver`.
    pub caption_hover_bg: Rgba,
    /// `CaptionButtonBackgroundPressed`.
    pub caption_pressed_bg: Rgba,
    /// `CloseButtonBackgroundPointerOver` (#C42B1C).
    pub close_hover_bg: Rgba,
    /// `CloseButtonBackgroundPressed`.
    pub close_pressed_bg: Rgba,
}

impl Theme {
    /// Dark theme (mặc định WT).
    pub fn dark() -> Self {
        Self {
            page_bg: rgb(0x202020),
            tab_row_bg: rgb(0x202020),
            tab_selected_bg: rgb(0x0C0C0C),
            tab_hover_bg: rgb(0x2D2D2D),
            fg: rgb(0xFFFFFF),
            fg_dim: rgba(0xFFFFFFC5),
            stroke: rgb(0x3A3A3A),
            caption_hover_bg: rgba(0xFFFFFF1A),
            caption_pressed_bg: rgba(0xFFFFFF12),
            close_hover_bg: rgb(0xC42B1C),
            close_pressed_bg: rgba(0xC42B1CE6),
        }
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            page_bg: rgb(0xF3F3F3),
            tab_row_bg: rgb(0xE9E9E9),
            tab_selected_bg: rgb(0xFFFFFF),
            tab_hover_bg: rgb(0xDADADA),
            fg: rgb(0x000000),
            fg_dim: rgba(0x000000A0),
            stroke: rgb(0xD0D0D0),
            caption_hover_bg: rgba(0x0000001A),
            caption_pressed_bg: rgba(0x00000012),
            close_hover_bg: rgb(0xC42B1C),
            close_pressed_bg: rgba(0xC42B1CE6),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Font glyph (Segoe MDL2 Assets) dùng cho icon chrome.
pub const ICON_FONT: &str = "Segoe MDL2 Assets";

/// Kích thước layout (px) — đọc từ XAML (`MinMaxCloseControl` 46×40, tab 32…).
pub mod metrics {
    /// Chiều cao titlebar (`MinMaxCloseControl d:DesignHeight=40`).
    pub const TITLEBAR_H: f32 = 30.0;
    /// Chiều rộng 1 caption button.
    pub const CAPTION_W: f32 = 46.0;
    /// Chiều cao tab item.
    pub const TAB_H: f32 = 32.0;
    /// Biên chiều rộng tab (`TabWidthMode=Equal`).
    pub const TAB_MIN_W: f32 = 80.0;
    pub const TAB_MAX_W: f32 = 240.0;
    /// Nút new-tab (`SplitButton Height=24`, primary 31).
    pub const NEW_TAB_W: f32 = 31.0;
    pub const NEW_TAB_H: f32 = 22.0;
    /// `DragBar MinWidth`.
    pub const DRAG_MIN_W: f32 = 45.0;
}
