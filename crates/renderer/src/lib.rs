//! `renderer` — GPUI renderer port (phase 1 skeleton) thay thế WT Atlas
//! (DirectWrite/D2D/D3D). Verbatim port từ `windows/src/renderer/{base,atlas,inc}/`.
//!
//! Phase 1: traits `RenderData`/`RenderEngine` + structs `Cluster`/
//! `CursorOptions`/`RenderFrameInfo`/`ShapedRow`/`GridLineRange` skeleton;
//! phase 3 fill `gpui::TextSystem` shaping + step 6-10 paint pipeline.
//!
//! ## Port-blockers
//! - `core` chưa export `TextBuffer`/`Row`/`TextAttribute`/`Cursor`/`Viewport`
//!   (Lane A đang port song song). Mọi signature `core::*` trong crate này
//!   là **GUESS** — rework khi Lane A finalize (xem comments `// ASSUMED core
//!   API`).
//! - `_resolveFontMetrics` đã port sang `font::resolve_font_metrics()` —
//!   phase 1 re-export, không re-implement (`metrics.rs` không tồn tại).
//! - Builtin glyphs (box-drawing U+2500-259F, Powerline U+E0B0-E0BF,
//!   LegacyComputing U+1FB00-1FB94) defer phase 2 (Path shapes GPUI).
//!
//! `colorfix.rs` port OKlab color-appearance math (`linear_srgb ↔ oklab`,
//! `GetPerceivableColor`, `AdjustLightness`) từ WT `windows/src/types/
//! ColorFix.cpp`. WT C++ source dùng hex-float literal (`0x1.3e4568p-12f`)
//! cho 256-entry sRGB→linear LUT; Rust stable **chưa** support hex float
//! literal (nightly `hex_float_literal` only) → convert sang decimal f32 with
//! full round-trip precision (bit-identical giá trị với C++ origin). OKlab
//! transform coeff (`2.6097574011f32` etc.) giữ nguyên verbatim.
//! `clippy::excessive_precision` (rust 1.99) flag literal này — truncate theo
//! suggest không đổi semantic f32 nhưng break port-verbatim intent → allow
//! crate-wide. Doc Tiếng Việt allow `doc_lazy_continuation` cùng reason như `core`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::doc_lazy_continuation)]

pub mod cell;
pub mod colorfix;
pub mod grid;
pub mod paint;
pub mod payload;
pub mod settings;

pub use cell::{Cluster, CursorOptions, CursorShape, GridLineSet, GridLines};
pub use grid::{
    AntialiasingMode, Bitmap, FontMapping, FontRelevantAttributes, GridLineRange, RenderingPayload,
    ShapedRow,
};
pub use paint::{RenderData, RenderEngine, RenderFrameInfo, Renderer};
pub use settings::{RenderModeFlags, RenderSettings};
pub use colorfix::{adjust_lightness, get_lightness, get_perceivable_color};
