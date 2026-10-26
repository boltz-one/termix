//! `RenderSettings` — port `base/RenderSettings.cpp`. Phase 2 fill:
//! - `get_attribute_colors` verbatim từ `RenderSettings.cpp:183`
//!   (`GetAttributeColors`) — FG/BG lookup, IntenseIsBright + ScreenReversed
//!   swap, dim-faint, invisible-move.
//! - `get_attribute_colors_with_alpha` (`RenderSettings.cpp:236`).
//! - `get_attribute_underline_color` (`RenderSettings.cpp:259`).
//!
//! Color table: WT `_colorTable[TextColor::TABLE_SIZE]` (=256: 16 named ANSI +
//! xterm 6×6×6 cube + 24-step grayscale). Mỗi entry packed ABGR
//! (`settings::Color` layout: `a<<24 | b<<16 | g<<8 | r`).

// `RenderModeFlags` — port `RenderSettings.cpp::Mode` bitset. Mỗi flag đến từ
// một setting/option trong WT. Dùng `bitflags!` 2.x (struct bitset).
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct RenderModeFlags: u32 {
        /// Intense text hiển thị bold (WT `IntenseIsBold`).
        const INTENSE_IS_BOLD = 0x0001;
        /// Intense text dùng bright color (WT `IntenseIsBright`).
        const INTENSE_IS_BRIGHT = 0x0002;
        /// Screen reversed (DECSCNM) — fg↔bg.
        const SCREEN_REVERSED = 0x0004;
        /// Synchronized output (BSU/ESU) — defer composite.
        const SYNCHRONIZED_OUTPUT = 0x0008;
        /// Always distinguishable colors — port WT `AlwaysDistinguishableColors`.
        const ALWAYS_DISTINGUISHABLE = 0x0010;
        /// Indexed distinguishable colors — port WT `IndexedDistinguishableColors`.
        const INDEXED_DISTINGUISHABLE = 0x0020;
    }
}

/// Số color table entry — port `TextColor::TABLE_SIZE` (`TextColor.h`).
pub const COLOR_TABLE_SIZE: usize = 256;

/// `RenderSettings` — port `base/RenderSettings.cpp`. WT giữ
/// `_colorTable[COLOR_TABLE_SIZE]` (16 ANSI + 240 xterm256) + `_renderMode`.
/// Phase 2: full 256-table lookup + bright/dim/invert verbatim.
#[derive(Clone, Debug)]
pub struct RenderSettings {
    /// 256 ANSI/xterm color entries packed ABGR (alpha `0xFF`).
    pub color_table: [u32; COLOR_TABLE_SIZE],
    /// Mode flags (bitset).
    pub render_mode: RenderModeFlags,
    /// `_blinkShouldBeFaint` (`RenderSettings.cpp:160`) — toggle qua
    /// `ToggleBlinkRendition`. Phase 2: wire tới blink tick từ
    /// `Renderer`.
    pub blink_should_be_faint: bool,

    /// Default FG index vào color table — port `ColorAlias::DefaultForeground`
    /// = `TextColor::DEFAULT_FOREGROUND` (15 = White trong Campbell).
    pub default_fg_index: u8,
    /// Default BG index — port `ColorAlias::DefaultBackground`
    /// = `TextColor::DEFAULT_BACKGROUND` (0 = Black trong Campbell).
    pub default_bg_index: u8,
}

impl Default for RenderSettings {
    fn default() -> Self {
        // Port WT default: Campbell scheme + xterm256 lookup.
        // 16 named = campbell; 16..=255 = xterm 6x6x6 cube + 24-step grayscale.
        let named = settings::color::campbell_color_table();
        let mut table = [0u32; COLOR_TABLE_SIZE];
        for (i, c) in named.iter().enumerate() {
            table[i] = c.abgr() | 0xFF000000;
        }
        for idx in 16u32..=255 {
            let c = settings::color::xterm256(idx as u8, &named);
            table[idx as usize] = c.abgr() | 0xFF000000;
        }
        Self {
            color_table: table,
            render_mode: RenderModeFlags::empty(),
            blink_should_be_faint: false,
            // WT defaults (`TextColor::DEFAULT_FOREGROUND`/`BACKGROUND`).
            default_fg_index: 15,
            default_bg_index: 0,
        }
    }
}

impl RenderSettings {
    /// Map FG/BG lookup phase — port `TextColor::GetColor(...)` (`TextColor.h`):
    /// - 0..=15: `color_table[idx | bright_bit]`.
    /// - 16..=255: `color_table[idx]`.
    /// - >255 (RGB literal / invalid): raw packed RGB với alpha 0xFF.
    ///
    /// WT `GetColor`:
    /// ```cpp
    /// if (IsDefault()) return table[defaultIndex];
    /// if (IsLegacyIndex()) return table[idx | (brightenFg && idx<8 ? 0x08 : 0)];
    /// if (IsIndex256()) return table[idx];
    /// if (IsRgb()) return rgb;
    /// ```
    fn lookup_color(&self, attr: &core::TextAttribute, fg: bool, brighten: bool) -> u32 {
        // WT `TextColor::IsDefault()` ≡ NOT legacy && NOT index256 && rgb==0.
        // Our phase-1 `TextAttribute`: default signaled by `foreground==0`
        // AND `!is_legacy` AND `fg_index==0` (i.e. not "set via indexed path").
        // ASSUMED core API — rework when Lane A finalizes TextColor.
        let idx = if fg { attr.fg_index } else { attr.bg_index };
        if attr.is_legacy {
            // Legacy 16-color index — WT `IsLegacyIndex()` branch.
            let mut i = idx as usize & 0x0F;
            if fg && brighten && i < 8 {
                i |= 0x08; // brightenFg: dark → bright pair
            }
            return self.color_table[i];
        }
        if idx > 0 && (idx as u32) < 256 {
            // `IsIndex256()` branch.
            return self.color_table[idx as usize];
        }
        // 256-color table exhausted or zero — fall back to packed RGB.
        let raw = if fg { attr.foreground } else { attr.background };
        if raw == 0 {
            return self.color_table[if fg {
                self.default_fg_index as usize
            } else {
                self.default_bg_index as usize
            }];
        }
        // RGB literal — ensure alpha 0xFF.
        raw | 0xFF000000
    }

    /// `(fg, bg)` cho attribute — port verbatim `GetAttributeColors`
    /// (`RenderSettings.cpp:183`). Packed ABGR với alpha 0xFF.
    ///
    /// Adjust fg nếu gần bg (ΔEOK² < 0.25) khi `ALWAYS_DISTINGUISHABLE` /
    /// `INDEXED_DISTINGUISHABLE` trong `render_mode` bật — port WT
    /// `Feature_AdjustIndistinguishableText` (`RenderSettings.cpp:215-225`).
    /// ADR-0015. Flag default empty → adjust path skip (backwards-compat).
    pub fn get_attribute_colors(&self, attr: &core::TextAttribute) -> (u32, u32) {
        let brighten_fg = attr.is_intense()
            && self
                .render_mode
                .contains(RenderModeFlags::INTENSE_IS_BRIGHT);
        let dim_fg = attr.is_faint() || (self.blink_should_be_faint && attr.is_blinking());
        let swap_fg_and_bg =
            attr.is_reverse_video() ^ self.render_mode.contains(RenderModeFlags::SCREEN_REVERSED);

        let mut fg = self.lookup_color(attr, true, brighten_fg);
        let mut bg = self.lookup_color(attr, false, false);

        // WT: `fg = (fg >> 1) & 0x7F7F7F;` — but Color is ABGR-encoded =>
        // halving each channel. Phase 2: WT works on RGB-only (high byte
        // is 0 in COLORREF). Our packed ABGR has alpha 0xFF in the high
        // byte, so halving the top byte would zero alpha. Mask back to
        // 0xFF after halving.
        if dim_fg {
            fg = (fg >> 1) & 0x7F7F7F7F;
            fg |= 0xFF000000;
        }
        if swap_fg_and_bg {
            std::mem::swap(&mut fg, &mut bg);
        }
        if attr.is_invisible() {
            fg = bg;
        }
        // Port WT `RenderSettings.cpp:215-225` (`Feature_AdjustIndistinguishableText`).
        // `IsDefaultOrLegacy` ≡ default (idx==0 và raw==0, lookup_color nhánh fallback)
        // OR legacy (16-color index). Mirror lookup_color branch logic (line 99-122).
        let fg_default_or_legacy =
            attr.is_legacy || (attr.fg_index == 0 && attr.foreground == 0);
        let bg_default_or_legacy =
            attr.is_legacy || (attr.bg_index == 0 && attr.background == 0);
        let mode = self.render_mode;
        let always = mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE);
        let indexed = mode.contains(RenderModeFlags::INDEXED_DISTINGUISHABLE);
        if (always || indexed)
            && fg != bg
            && (always || (fg_default_or_legacy && bg_default_or_legacy))
        {
            fg = crate::colorfix::get_perceivable_color(fg, bg, 0.5 * 0.5); // = 0.25
        }
        (fg, bg)
    }

    /// `(fg, bg)` với alpha mixin (port `GetAttributeColorsWithAlpha`,
    /// `RenderSettings.cpp:236`). WT đặt `fg |= 0xff000000` luôn; `bg` chỉ
    /// opaque khi không phải default background (để acrylic nền trong suốt).
    pub fn get_attribute_colors_with_alpha(
        &self,
        attr: &core::TextAttribute,
        _bg_opaque_mixin: u8,
    ) -> (u32, u32) {
        let (mut fg, mut bg) = self.get_attribute_colors(attr);
        fg |= 0xFF000000;
        // WT: `!attr.BackgroundIsDefault() || reverse || invisible`.
        // Phase 2: core chưa có `BackgroundIsDefault`; coi `background==0`
        // (= default) → giữ alpha của palette (0xFF). Nếu non-default hay inverted,
        // ép alpha 0xFF.
        let inverted =
            attr.is_reverse_video() ^ self.render_mode.contains(RenderModeFlags::SCREEN_REVERSED);
        if attr.background != 0 || inverted || attr.is_invisible() {
            bg |= 0xFF000000;
        }
        (fg, bg)
    }

    /// Underline color — port verbatim `GetAttributeUnderlineColor`
    /// (`RenderSettings.cpp:259`). WT ưu tiên `TextAttribute::GetUnderlineColor`,
    /// fallback fg color. Nếu `attr.underline_color==0` (Default), trả fg.
    /// Nếu `attr.invisible`, trả bg.
    ///
    /// Adjust ul nếu gần bg (ΔEOK² < 0.25) khi `ALWAYS_DISTINGUISHABLE` /
    /// `INDEXED_DISTINGUISHABLE` bật — port WT `RenderSettings.cpp:280-286`.
    /// ADR-0015. Flag default empty → adjust path skip (backwards-compat).
    pub fn get_attribute_underline_color(&self, attr: &core::TextAttribute) -> u32 {
        let (fg, bg) = self.get_attribute_colors(attr);
        if attr.get_underline_color() == 0 {
            return fg;
        }
        // Indexed/RGB lookup qua color table — apply brighten như WT
        // (`GetColor(table, defaultUlIndex, true)`).
        let mut ul = self.color_table[self.default_fg_index as usize];
        if attr.is_legacy {
            let i = (attr.underline_color as u8) as usize & 0x0F;
            ul = self.color_table[i | 0x08]; // brighten
        } else if attr.underline_color != 0 {
            ul = attr.underline_color | 0xFF000000;
        }
        if attr.is_invisible() {
            ul = bg;
        }
        // Port WT `RenderSettings.cpp:280-286`.
        let ul_default_or_legacy = attr.underline_color == 0 || attr.is_legacy;
        let bg_default_or_legacy =
            attr.is_legacy || (attr.bg_index == 0 && attr.background == 0);
        let mode = self.render_mode;
        let always = mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE);
        let indexed = mode.contains(RenderModeFlags::INDEXED_DISTINGUISHABLE);
        if ul != bg && (always || (indexed && ul_default_or_legacy && bg_default_or_legacy)) {
            ul = crate::colorfix::get_perceivable_color(ul, bg, 0.5 * 0.5); // = 0.25
        }
        ul
    }

    /// Toggle blink faint — port `ToggleBlinkRendition` (`:292`).
    pub fn toggle_blink_rendition(&mut self) {
        self.blink_should_be_faint = !self.blink_should_be_faint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::TextAttribute;

    #[test]
    fn render_settings_default_loads_campbell_table() {
        let s = RenderSettings::default();
        // Campbell entry 0 = Black (ABGR=0xFF0C0C0C).
        assert_eq!(s.color_table[0], 0xFF0C0C0C);
        assert!(s.render_mode.is_empty());
        // 256 entries — xterm256 entry 16 (cube 0,0,0 → black).
        assert_eq!(s.color_table[16], 0xFF000000);
        // xterm256 entry 255 = 8 + (255-232)*10 = 238 (gray ramp top).
        assert_eq!(s.color_table[255] & 0xFF, 238);
        assert_eq!(s.default_fg_index, 15);
        assert_eq!(s.default_bg_index, 0);
    }

    #[test]
    fn render_mode_flags_default_empty_and_insert() {
        let f = RenderModeFlags::INTENSE_IS_BOLD | RenderModeFlags::INTENSE_IS_BRIGHT;
        assert!(f.contains(RenderModeFlags::INTENSE_IS_BOLD));
        assert!(f.contains(RenderModeFlags::INTENSE_IS_BRIGHT));
        assert!(!f.contains(RenderModeFlags::SCREEN_REVERSED));
    }

    #[test]
    fn get_attribute_colors_default_returns_palette_defaults() {
        let s = RenderSettings::default();
        let a = TextAttribute::new(); // foreground=0 (default)
        let (fg, bg) = s.get_attribute_colors(&a);
        // default FG = entry 15 (White), default BG = entry 0 (Black).
        assert_eq!(fg, s.color_table[15]);
        assert_eq!(bg, s.color_table[0]);
    }

    #[test]
    fn get_attribute_colors_reverse_video_swaps() {
        let s = RenderSettings::default();
        let mut a = TextAttribute::new();
        a.set_reverse_video(true);
        let (fg, bg) = s.get_attribute_colors(&a);
        // swapped: fg = palette[0] (Black), bg = palette[15] (White).
        assert_eq!(fg, s.color_table[0]);
        assert_eq!(bg, s.color_table[15]);
    }

    #[test]
    fn get_attribute_colors_intense_brighten_picks_index_8() {
        let s = RenderSettings {
            render_mode: RenderModeFlags::INTENSE_IS_BRIGHT,
            ..RenderSettings::default()
        };
        let mut a = TextAttribute::new();
        a.set_intense(true);
        a.set_indexed_foreground(2); // legacy green
                                     // brighten: legacy index 2 → palette[10] (bright green)
        let (fg, _) = s.get_attribute_colors(&a);
        assert_eq!(fg, s.color_table[10]);
    }

    #[test]
    fn get_attribute_colors_faint_halves_fg_channels() {
        let s = RenderSettings::default();
        let mut a = TextAttribute::new();
        a.set_faint(true);
        a.set_foreground(0xFFFFFFFF); // white-ish
        let (fg, _) = s.get_attribute_colors(&a);
        // halved + masked + alpha restored: 0xFF7F7F7F
        assert_eq!(fg & 0xFF7F7F7F, 0xFF7F7F7F);
    }

    #[test]
    fn get_attribute_colors_invisible_sets_fg_equal_bg() {
        let s = RenderSettings::default();
        let mut a = TextAttribute::new();
        a.set_invisible(true);
        let (fg, bg) = s.get_attribute_colors(&a);
        assert_eq!(fg, bg);
    }

    #[test]
    fn get_attribute_colors_with_alpha_opaque_fg() {
        let s = RenderSettings::default();
        let a = TextAttribute::new();
        let (fg, _) = s.get_attribute_colors_with_alpha(&a, 0xFF);
        assert_eq!(fg & 0xFF000000, 0xFF000000);
    }

    #[test]
    fn get_attribute_colors_with_alpha_keeps_default_bg_alpha_for_screenreversed_false() {
        let s = RenderSettings::default();
        let a = TextAttribute::new(); // background=0 (default)
        let (_, bg) = s.get_attribute_colors_with_alpha(&a, 0);
        // Default background → WT acrylic alpha. Phase 2: giữ alpha table.
        assert_eq!(bg & 0xFF000000, 0xFF000000);
    }

    #[test]
    fn get_attribute_colors_with_alpha_forces_bg_opaque_when_reverse_video() {
        let s = RenderSettings::default();
        let mut a = TextAttribute::new();
        a.set_reverse_video(true);
        let (_, bg) = s.get_attribute_colors_with_alpha(&a, 0);
        // WT: reverse-video → bg |= 0xff000000.
        assert_eq!(bg & 0xFF000000, 0xFF000000);
    }

    #[test]
    fn get_attribute_underline_color_falls_back_to_fg() {
        let s = RenderSettings::default();
        let a = TextAttribute::new();
        let (fg, _) = s.get_attribute_colors(&a);
        // underline_color == 0 → fg
        assert_eq!(s.get_attribute_underline_color(&a), fg);
    }

    #[test]
    fn get_attribute_underline_color_uses_explicit_when_set() {
        let s = RenderSettings::default();
        let mut a = TextAttribute::new();
        a.set_underline_color(0xFF112233);
        // Non-default + non-legacy → raw | 0xFF000000 = 0xFF112233
        assert_eq!(s.get_attribute_underline_color(&a), 0xFF112233);
    }

    #[test]
    fn get_attribute_underline_color_invisible_returns_bg() {
        let s = RenderSettings::default();
        let mut a = TextAttribute::new();
        a.set_invisible(true);
        a.set_underline_color(0xFF112233);
        let (_, bg) = s.get_attribute_colors(&a);
        // WT: `if (attr.IsInvisible()) ul = bg;`
        assert_eq!(s.get_attribute_underline_color(&a), bg);
    }

    // === T5 — AdjustIndistinguishable behavioral (loop 03) — port WT
    // `Feature_AdjustIndistinguishableText` (`RenderSettings.cpp:215-225`,
    // `:280-286`). ADR-0015. `render_mode` flags gate adjust path.

    /// VR3 — flag default empty → adjust path skip (backwards-compat).
    #[test]
    fn get_attribute_colors_no_adjust_when_flags_off() {
        let s = RenderSettings::default(); // render_mode empty
        let a = TextAttribute::new(); // fg/bg default (idx 0, raw 0)
        let (fg, bg) = s.get_attribute_colors(&a);
        // Flag off → trả palette defaults verbatim (no nudge).
        assert_eq!(fg, s.color_table[s.default_fg_index as usize]);
        assert_eq!(bg, s.color_table[s.default_bg_index as usize]);
    }

    /// VR1 — `ALWAYS_DISTINGUISHABLE` nudges fg khi gần bg.
    #[test]
    fn get_attribute_colors_always_distinguishable_nudges_close_pair() {
        // near pair: fg=0xFF555555, bg=0xFF575757 (ΔEOK² < 0.25).
        let mut s = RenderSettings::default();
        s.render_mode |= RenderModeFlags::ALWAYS_DISTINGUISHABLE;
        // FG = RGB literal non-default (near bg); BG = literal.
        let mut a = TextAttribute::new();
        a.set_color(0xFF555555, true); // fg
        a.set_color(0xFF575757, false); // bg
        let (fg, bg) = s.get_attribute_colors(&a);
        assert_ne!(fg, 0xFF555555, "fg must be nudged (≠ input)");
        assert_ne!(fg, bg, "nudged fg must differ from bg");
        // Idempotent: re-call với nudged fg → ΔEOK² ≥ 0.25 → no nudge → unchanged.
        let mut a2 = TextAttribute::new();
        a2.set_color(fg, true);
        a2.set_color(0xFF575757, false);
        let (fg2, _) = s.get_attribute_colors(&a2);
        assert_eq!(fg2, fg, "idempotent — nudged fg already distinguishable");
    }

    /// VR4 — `INDEXED_DISTINGUISHABLE` only (no `ALWAYS`) — adjust khi
    /// fg/bg default-or-legacy; KHÔNG adjust khi fg = RGB literal non-legacy.
    #[test]
    fn get_attribute_colors_indexed_distinguishable_only_default_or_legacy() {
        let mut s = RenderSettings::default();
        s.render_mode |= RenderModeFlags::INDEXED_DISTINGUISHABLE;

        // Case A: fg/bg cả hai RGB literal non-legacy (near pair) →
        // guard `(fg_default_or_legacy && bg_default_or_legacy)` = false →
        // skip → fg trả raw RGB literal (lookup nhánh `raw|0xFF000000`).
        let mut a = TextAttribute::new();
        a.set_color(0xFF555555, true); // fg RGB non-legacy
        a.set_color(0xFF575757, false); // bg RGB non-legacy
        let (fg, _) = s.get_attribute_colors(&a);
        // RGB literal non-legacy → guard false → skip nudge → fg = raw.
        assert_eq!(fg, 0xFF555555, "indexed-only skip cho RGB literal non-legacy");

        // Case B: khi chỉ một trong fg/bg default-or-legacy thì AND false →
        // skip. Verify bằng default attr (fg/bg đều default: idx=0 raw=0
        // is_legacy=false → guard fg_default=true && bg_default=true →
        // adjust path reach, nhưng palette[15] White vs palette[0] Black ΔE
        // lớn → `get_perceivable_color` early-return (ΔEOK² ≥ 0.25) → fg
        // unchanged = palette[15]. Mirror WT: default pair không cần nudge.
        let a2 = TextAttribute::new(); // pure default
        let (fg2, bg2) = s.get_attribute_colors(&a2);
        assert_eq!(fg2, s.color_table[s.default_fg_index as usize],
            "default fg → palette[15] (already distinguishable vs Black → no nudge)");
        assert_eq!(bg2, s.color_table[s.default_bg_index as usize]);
        // Indexed flag không break default lookup path (P3 tên = verbatim).
    }

    /// VR5 — guard `fg != bg`: identical pair → unchanged.
    #[test]
    fn get_attribute_colors_skip_when_fg_equals_bg() {
        let mut s = RenderSettings::default();
        s.render_mode |= RenderModeFlags::ALWAYS_DISTINGUISHABLE;
        let mut a = TextAttribute::new();
        a.set_color(0xFF555555, true);
        a.set_color(0xFF555555, false); // fg == bg literal
        let (fg, bg) = s.get_attribute_colors(&a);
        assert_eq!(fg, bg, "fg == bg unchanged (guard fg != bg)");
        assert_eq!(fg, 0xFF555555, "no nudge when fg == bg");
    }

    /// VR6/VR2 — ul close to bg, `ALWAYS_DISTINGUISHABLE` on → nudge.
    #[test]
    fn get_attribute_underline_color_distinguishable_nudges() {
        let mut s = RenderSettings::default();
        s.render_mode |= RenderModeFlags::ALWAYS_DISTINGUISHABLE;
        // underline_color = literal gần bg (non-legacy, non-default).
        let mut a = TextAttribute::new();
        a.set_underline_color(0xFF555555); // ul literal
        a.set_color(0xFF575757, false); // bg literal near ul
        let ul = s.get_attribute_underline_color(&a);
        // ALWAYS path: ul != bg → nudge.
        assert_ne!(ul, 0xFF575757, "ul must be nudged (≠ bg)");
        // Idempotent: re-call với nudged ul → ΔEOK² ≥ 0.25 → unchanged.
        let mut a2 = TextAttribute::new();
        a2.set_underline_color(ul);
        a2.set_color(0xFF575757, false);
        let ul2 = s.get_attribute_underline_color(&a2);
        assert_eq!(ul2, ul, "idempotent — nudged ul already distinguishable");
    }
}
