//! `font` — font resolution, metrics and DirectWrite binding for
//! termix.
//!
//! This crate is the Rust counterpart of Windows Terminal's
//! `windows/src/renderer/atlas/AtlasEngine.api.cpp::_resolveFontMetrics` and
//! the supporting `FontInfoBase` / `FontInfoDesired` / `FontInfo` types
//! (`windows/src/renderer/inc/FontInfo*.hpp`), plus the CSS-like cell-size
//! parser `CSSLengthPercentage` (`css_length` module).
//!
//! The metrics algorithm is reproduced **verbatim** from the C++ source so
//! that a profile with `("Cascadia Mono", 12pt, weight=400, dpi=96)` produces
//! the *exact* `cellWidth` / `cellHeight` / `baseline` / `advanceWidth` that
//! WT's AtlasEngine yields on the same machine.

pub mod css_length;
pub mod families;
pub mod font_info;
pub mod metrics;

#[cfg(all(target_os = "windows", feature = "directwrite"))]
pub mod dwrite;

pub use families::iterate_font_families;
pub use font_info::{FontInfo, FontInfoBase, FontInfoDesired};
pub use metrics::{
    resolve_font_metrics, resolve_font_metrics_with_collection, FontDecorationPosition,
    FontRuntime, FontSettingsSummary, ResolveFontResult,
};

#[cfg(all(target_os = "windows", feature = "directwrite"))]
pub use dwrite::DWriteFontCollection;
