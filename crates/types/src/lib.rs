//! `types` — Geometric primitives, color ops, Unicode width detection
//!
//! Port of Windows Terminal `til/*` and `types/*` utilities.
//! Phase P05 (types-til), Wave 1.

pub mod color;
pub mod primitives;
pub mod rle;
pub mod unicode;
pub mod utils;
pub mod viewport;
pub mod width;

pub use color::{Color, Oklab};
pub use primitives::{Point, Rect, Size};
pub use rle::Rle;
pub use unicode::{decode_utf8, grapheme_clusters, utf16_to_utf8, utf8_to_utf16};
pub use utils::{console, Guid};
pub use viewport::Viewport;
pub use width::{codepoint_width, string_width};

// Re-export bitflags for EnumSet-style usage
pub use bitflags::bitflags;
