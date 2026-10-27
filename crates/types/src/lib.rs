//! `types` — Geometric primitives, color ops, Unicode width detection
//!
//! Port of Windows Terminal `til/*` and `types/*` utilities.
//! Phase P05 (types-til), Wave 1.

pub mod primitives;
pub mod rle;
pub mod viewport;

pub use primitives::{Point, Rect, Size};
pub use rle::Rle;
pub use viewport::Viewport;
