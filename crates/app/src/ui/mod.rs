//! `ui` — layout chrome của `TerminalApp` (phase 14, phần layout trước).
//!
//! Port của `TerminalPage.xaml` (3 hàng: tab row / infobar / content),
//! `TitlebarControl.xaml`, `MinMaxCloseControl.xaml`, `TabRowControl.xaml` và
//! `TabHeaderControl.xaml`. Behaviour = state machine thuần (`tab_strip_model`),
//! hình ảnh = GPUI `RenderOnce` (`tab_strip`, `titlebar`, `page_layout`).

pub mod page_layout;
pub mod palette;
pub mod tab_strip;
pub mod tab_strip_model;
pub mod theme;
pub mod titlebar;

pub use page_layout::PageLayout;
pub use tab_strip::{TabStrip, TabStripCallbacks};
pub use tab_strip_model::{TabItem, TabStatus, TabStripModel};
pub use theme::Theme;
pub use titlebar::Titlebar;
