//! Re-export bảng màu từ `settings` — KHÔNG duplicate.
//!
//! `campbell_color_table()` port `windows/src/types/colorTable.cpp:11`
//! (16 màu default Campbell palette). `xterm256(index, named)` port
//! `InitializeExtendedColorTable` (256-color cube + grayscale ramp).

pub use settings::color::{campbell_color_table, xterm256, Color};
