//! Resolve a cell's `TextAttribute` into concrete colours/flags for painting
//! (`RenderSettings::GetAttributeColors`, `renderSettings.cpp`): palette lookup,
//! intense → bright, reverse video, faint, invisible.

use core::attrs::{ColorKind, TextAttribute};
use core::color_table::{campbell_color_table, xterm256, Color};

/// Campbell default foreground / background (WT defaults).
pub const DEFAULT_FG: u32 = 0xCCCCCC;
pub const DEFAULT_BG: u32 = 0x0C0C0C;

/// Fully resolved look of one cell. Colours are `0xRRGGBB`; `bg == None`
/// means "terminal background" so no quad needs painting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: u32,
    pub bg: Option<u32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub faint: bool,
    pub invisible: bool,
}

fn rgb(c: Color) -> u32 {
    ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32
}

/// `0x00BBGGRR` COLORREF (as stored by SGR 38/48;2) → `0xRRGGBB`.
fn colorref(c: u32) -> u32 {
    ((c & 0xFF) << 16) | (c & 0xFF00) | ((c >> 16) & 0xFF)
}

pub struct Palette {
    table: [Color; 16],
}

impl Default for Palette {
    fn default() -> Self {
        Self { table: campbell_color_table() }
    }
}

impl Palette {
    fn color(&self, kind: ColorKind, index: u8, packed: u32, intense: bool) -> Option<u32> {
        match kind {
            ColorKind::Default => None,
            ColorKind::Rgb => Some(colorref(packed)),
            ColorKind::Indexed256 => Some(rgb(xterm256(index, &self.table))),
            ColorKind::Indexed16 => {
                // `intenseTextStyle: bright` — bold ANSI 0-7 shows as 8-15.
                let i = if intense && index < 8 { index + 8 } else { index };
                Some(rgb(self.table[(i & 15) as usize]))
            }
        }
    }

    pub fn resolve(&self, a: &TextAttribute) -> CellStyle {
        let fg = self.color(a.fg_kind, a.fg_index, a.foreground, a.intense);
        let bg = self.color(a.bg_kind, a.bg_index, a.background, false);
        let (mut fg, mut bg) = (fg.unwrap_or(DEFAULT_FG), bg);
        if a.reverse_video {
            let new_fg = bg.unwrap_or(DEFAULT_BG);
            bg = Some(fg);
            fg = new_fg;
        }
        // Bold weight only when "bright" could not express intensity.
        let bold = a.intense && !(a.fg_kind == ColorKind::Indexed16 && a.fg_index < 8);
        CellStyle {
            fg,
            bg,
            bold,
            italic: a.italic,
            underline: a.is_underlined(),
            strike: a.crossed_out,
            faint: a.faint,
            invisible: a.invisible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(f: impl FnOnce(&mut TextAttribute)) -> CellStyle {
        let mut a = TextAttribute::default();
        f(&mut a);
        Palette::default().resolve(&a)
    }

    #[test]
    fn default_is_campbell_fg_and_no_bg() {
        let s = attr(|_| {});
        assert_eq!((s.fg, s.bg), (DEFAULT_FG, None));
    }

    #[test]
    fn indexed_fg_does_not_leak_into_default_bg() {
        // Regression: a legacy flag shared by fg and bg turned the default bg black.
        let s = attr(|a| a.set_indexed_foreground(1));
        assert_eq!(s.fg, 0xC50F1F);
        assert_eq!(s.bg, None);
        let s = attr(|a| a.set_indexed_background(4));
        assert_eq!(s.fg, DEFAULT_FG);
        assert_eq!(s.bg, Some(0x0037DA));
    }

    #[test]
    fn intense_brightens_ansi_and_bolds_others() {
        let s = attr(|a| {
            a.set_indexed_foreground(1);
            a.set_intense(true);
        });
        assert_eq!((s.fg, s.bold), (0xE74856, false));
        let s = attr(|a| a.set_intense(true));
        assert!(s.bold);
    }

    #[test]
    fn rgb_256_and_reverse() {
        let s = attr(|a| a.set_color(0x0000FF, true)); // COLORREF red
        assert_eq!(s.fg, 0xFF0000);
        let s = attr(|a| a.set_indexed_foreground_256(196));
        assert_eq!(s.fg, 0xFF0000);
        let s = attr(|a| a.set_reverse_video(true));
        assert_eq!((s.fg, s.bg), (DEFAULT_BG, Some(DEFAULT_FG)));
    }
}
