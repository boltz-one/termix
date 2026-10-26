//! Port-of-intent: WT `ColorFix::GetPerceivableColor` (`types/ColorFix.cpp:167`)
//! + `RenderSettings::GetAttributeColors` AdjustIndistinguishable block
//! (`base/RenderSettings.cpp:215-225`, `:280-286`).
//!
//! WT-named integration test suite cho phase P10 step 1.2 — verify
//! ColorFix port (loop 02) + AdjustIndistinguishable wire (loop 03) deliver
//! observable distinguishable-color behavior at the `RenderSettings` API
//! boundary. Convention `wt_<suite>__<behavior>` (double underscore, mirror
//! loop-01 `wt_terminalcore__`). ADR-0015.

#![allow(non_snake_case)]
#![allow(clippy::doc_lazy_continuation)]

use core::TextAttribute;
use renderer::colorfix::get_perceivable_color;
use renderer::settings::{RenderModeFlags, RenderSettings};

/// `wt_colorfix__perceivable_color_distinguishable` — VR1 white-box: identical
/// FG/BG nudge → output ≠ input, alpha byte preserved. Port WT
/// `ColorFix::GetPerceivableColor` (`ColorFix.cpp:167`).
#[test]
fn wt_colorfix__perceivable_color_distinguishable() {
    // Identical color vs reference → ΔEOK² = 0 < 0.25 → nudge.
    let color = 0xFF555555u32;
    let reference = 0xFF555555u32;
    let out = get_perceivable_color(color, reference, 0.5 * 0.5); // min_sq = 0.25
    assert_ne!(out, color, "identical pair must be nudged");
    assert_eq!(out & 0xFF000000, 0xFF000000, "alpha byte preserved");
    // Idempotent endpoint: re-nudge `out` vs `reference` → ΔEOK² ≥ min_sq
    // → `get_perceivable_color` early-return → out unchanged (`ColorFix.cpp:172`).
    let out2 = get_perceivable_color(out, reference, 0.5 * 0.5);
    assert_eq!(
        out2, out,
        "idempotent — after nudge ΔEOK² ≥ min_sq (≥0.25 full ΔE)"
    );
}

/// `wt_renderer__always_distinguishable_nudges_close_pair` — VR3 wired: near
/// FG/BG pair (`0xFF555555` / `0xFF575757`, ΔEOK² < 0.25) + flag
/// `ALWAYS_DISTINGUISHABLE` → `get_attribute_colors` nudges fg ≠ bg. Port WT
/// `Feature_AdjustIndistinguishableText` (`RenderSettings.cpp:215-225`).
#[test]
fn wt_renderer__always_distinguishable_nudges_close_pair() {
    let mut s = RenderSettings::default();
    s.render_mode |= RenderModeFlags::ALWAYS_DISTINGUISHABLE;
    let mut a = TextAttribute::new();
    a.set_color(0xFF555555, true); // fg literal
    a.set_color(0xFF575757, false); // bg literal near fg
    let (fg, bg) = s.get_attribute_colors(&a);
    assert_ne!(fg, 0xFF555555, "fg nudged (≠ input literal)");
    assert_ne!(fg, bg, "nudged fg distinguishes from bg");
    // Idempotent: re-call `get_attribute_colors` với nudged fg wrap attr —
    // ΔEOK²(fg,bg) ≥ 0.25 → adjust early-return → fg unchanged (distinguishable).
    let mut a2 = TextAttribute::new();
    a2.set_color(fg, true);
    a2.set_color(0xFF575757, false);
    let (fg2, _) = s.get_attribute_colors(&a2);
    assert_eq!(
        fg2, fg,
        "idempotent — nudged fg already distinguishable from bg"
    );
}

/// `wt_renderer__indexed_distinguishable_requires_default_or_legacy` — VR4
/// wired: `INDEXED_DISTINGUISHABLE` only skip cho RGB literal non-legacy
/// (guard `fg.IsDefaultOrLegacy && bg.IsDefaultOrLegacy`).
#[test]
fn wt_renderer__indexed_distinguishable_requires_default_or_legacy() {
    let mut s = RenderSettings::default();
    s.render_mode |= RenderModeFlags::INDEXED_DISTINGUISHABLE;
    // fg/bg both RGB literal non-legacy near pair → guard false → skip nudge.
    let mut a = TextAttribute::new();
    a.set_color(0xFF555555, true);
    a.set_color(0xFF575757, false);
    let (fg, _) = s.get_attribute_colors(&a);
    assert_eq!(
        fg, 0xFF555555,
        "indexed-only skip cho RGB literal non-legacy"
    );
    // Default pair (idx 0 raw 0) → guard true → adjust reach, nhưng palette
    // White/Black ΔE lớn → get_perceivable_color early-return → fg unchanged.
    let a2 = TextAttribute::new();
    let (fg2, _) = s.get_attribute_colors(&a2);
    assert_eq!(
        fg2, s.color_table[s.default_fg_index as usize],
        "default pair → palette[15] (already distinguishable → no nudge)"
    );
}
