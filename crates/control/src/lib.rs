//! `control` — port `windows/src/cascadia/TerminalControl/` sang Rust+GPUI.
//! 4-layer WT architecture:
//!   Terminal (core) → ControlCore → ControlInteractivity → TermControl
//!   (XAML UserControl → GPUI `Render` View).
//!
//! Phase 2: đã thay `assumed.rs` shim bằng real Lane A API (`tg_core`,
//! `tg_connection`, `tg_renderer`). Còn `todo!()` ở nút chưa finalize:
//! - `core::parser::OutputStateMachineEngine` thay `VteAdapter` (brief name
//!   không match real export — port-blocker đã flag).
//! - `renderer::RenderEngine` chưa có method trả `gpui::IntoElement`
//!   (port-blocker phase 3).
//!
//! Anchors:
//! - `ControlCore.cpp` ~3500 LOC.
//! - `ControlInteractivity.cpp` ~900 LOC.
//! - `TermControl.cpp` ~4800 LOC.

pub mod control;
pub mod core;
pub mod interactivity;
pub mod key_encode;
pub mod shortcuts;
pub mod keymap;
pub mod mouse_report;
mod row_paint;
pub mod search;
pub mod selection;
mod term_input;
pub mod term_style;
pub mod term_view;
pub mod view_state;

pub use control::TermControl;
pub use core::ControlCore;
pub use interactivity::ControlInteractivity;
pub use keymap::{KeyAction, KeyBinding, KeyMapping};

// Phase 4 lane: pane tiling — giữ stub `pane.rs` chưa wire (Lane app port).
#[allow(dead_code)]
mod pane;
