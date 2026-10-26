//! Color table port.
//!
//! Ports:
//! - `til::color` (`windows/src/inc/til/color.h`) as a packed-`u32` RGBA value
//!   matching WT's memory layout (`abgr` union).
//! - `Microsoft::Console::Utils::CampbellColorTable` (`windows/src/types/colorTable.cpp`)
//!   — the default 16-color palette WT ships in-box.
//! - The xterm 256-color cube + grayscale ramp (`InitializeExtendedColorTable`).
//!
//! We model `Color` as a plain `u32` (ABGR order, alpha in the high byte) so
//! it round-trips byte-identically with `til::color::abgr` and converts to
//! GPUI's `Rgba` only at the render boundary.

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

/// Packed RGBA color, ABGR byte order (matches `til::color::abgr`). Alpha is
/// in the high byte. This is the wire format WT uses end-to-end.
///
/// Serde uses the WT JSON wire form: a `"#rrggbb"` (or 3-digit `"#rgb"`) hex
/// string, alpha forced to 0xFF on parse and emitted as 0xFF on serialize
/// (matching `JsonUtils::GetValueForKey` / `SetValueForKey` for
/// `Core::Color`, `TerminalSettingsModel/ColorScheme.cpp:81`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct Color(pub u32);

impl Color {
    /// Construct from `r, g, b` (alpha 255). Mirrors
    /// `til::color(uint8_t r, uint8_t g, uint8_t b)` (`til/color.h:50`).
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(Self::pack(r, g, b, 255))
    }

    /// Construct from `r, g, b, a`. Mirrors
    /// `til::color(uint8_t r, uint8_t g, uint8_t b, uint8_t a)`
    /// (`til/color.h:56`).
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(Self::pack(r, g, b, a))
    }

    /// Construct from a `COLORREF` (RGB in low 24 bits, alpha forced 0xFF).
    /// Mirrors `til::color(COLORREF c)` (`til/color.h:69`). The param name is
    /// deliberately not `colorref` to avoid colliding with the Win32 macro.
    pub const fn from_colorref(colorref_rgb_with_0_in_high_byte: u32) -> Self {
        Self(colorref_rgb_with_0_in_high_byte | 0xff000000)
    }

    const fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
        (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32)
    }

    pub const fn r(self) -> u8 {
        (self.0 & 0xff) as u8
    }
    pub const fn g(self) -> u8 {
        ((self.0 >> 8) & 0xff) as u8
    }
    pub const fn b(self) -> u8 {
        ((self.0 >> 16) & 0xff) as u8
    }
    pub const fn a(self) -> u8 {
        ((self.0 >> 24) & 0xff) as u8
    }

    /// Re-encode as GPUI RGBA. `gpui::Rgba` stores channels as `f32` 0..1; we
    /// pack WT's bytes into a `0xRRGGBBAA` u32 and use `gpui::rgba`, the
    /// boundary helper.
    pub fn to_gpui(self) -> gpui::Rgba {
        let packed = ((self.r() as u32) << 24)
            | ((self.g() as u32) << 16)
            | ((self.b() as u32) << 8)
            | (self.a() as u32);
        gpui::rgba(packed)
    }

    /// `til::color::abgr` — direct access to the packed store. Useful where
    /// callers need the exact WT raw value (e.g. parity tests).
    pub const fn abgr(self) -> u32 {
        self.0
    }
}

impl From<gpui::Rgba> for Color {
    fn from(value: gpui::Rgba) -> Self {
        // GPUI stores channels as `f32` 0..1; scale back to u8 then repack
        // into WT's ABGR byte order. We avoid the `u32::from(Rgba)` impl
        // (which packs as 0xRRGGBBAA) and rebuild directly so the order
        // matches `Color::from_rgba`.
        let r = (value.r * 255.0).round() as u8;
        let g = (value.g * 255.0).round() as u8;
        let b = (value.b * 255.0).round() as u8;
        let a = (value.a * 255.0).round() as u8;
        Self::from_rgba(r, g, b, a)
    }
}

impl Color {
    /// Parse a WT JSON color hex string (`"#rrggbb"` or `"#rgb"`). Alpha is
    /// forced to 0xFF — matches `JsonUtils::GetValueForKey<Color>` which
    /// discards any leading `#` and the alpha channel on parse. Returns
    /// `None` on malformed input (caller decides whether to treat as a
    /// parse error vs. leave-existing).
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#')?;
        match s.len() {
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::from_rgb(r, g, b))
            }
            3 => {
                // `"#rgb"` expands by duplicating each nibble, e.g. `"#f0a"`
                // → `"#ff00aa"`. Mirrors WT's `Utils::ColorFromHexDrawable`.
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                Some(Self::from_rgb(r * 17, g * 17, b * 17))
            }
            _ => None,
        }
    }

    /// Render as `"#rrggbb"` (alpha always emitted as FF on the wire —
    /// matches `JsonUtils::SetValueForKey<Color>` /
    /// `Core::Color::ToString()`).
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r(), self.g(), self.b())
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct HexVisitor;
        impl Visitor<'_> for HexVisitor {
            type Value = Color;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a #rrggbb / #rgb hex color string")
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Color::from_hex(s).ok_or_else(|| E::custom(format!("bad color: {s}")))
            }
        }
        de.deserialize_str(HexVisitor)
    }
}

/// Returns the WT Campbell color table (`windows/src/types/colorTable.cpp:11`).
/// Index ordering is VT standard: 0 black .. 7 gray, 8 darkgray .. 15 white.
pub fn campbell_color_table() -> [Color; 16] {
    [
        Color::from_rgb(0x0c, 0x0c, 0x0c), // 0  Black
        Color::from_rgb(0xc5, 0x0f, 0x1f), // 1  DarkRed
        Color::from_rgb(0x13, 0xa1, 0x0e), // 2  DarkGreen
        Color::from_rgb(0xc1, 0x9c, 0x00), // 3  DarkYellow
        Color::from_rgb(0x00, 0x37, 0xda), // 4  DarkBlue
        Color::from_rgb(0x88, 0x17, 0x98), // 5  DarkMagenta
        Color::from_rgb(0x3a, 0x96, 0xdd), // 6  DarkCyan
        Color::from_rgb(0xcc, 0xcc, 0xcc), // 7  Gray
        Color::from_rgb(0x76, 0x76, 0x76), // 8  DarkGray
        Color::from_rgb(0xe7, 0x48, 0x56), // 9  Red
        Color::from_rgb(0x16, 0xc6, 0x0c), // 10 Green
        Color::from_rgb(0xf9, 0xf1, 0xa5), // 11 Yellow
        Color::from_rgb(0x3b, 0x78, 0xff), // 12 Blue
        Color::from_rgb(0xb4, 0x00, 0x9e), // 13 Magenta
        Color::from_rgb(0x61, 0xd6, 0xd6), // 14 Cyan
        Color::from_rgb(0xf2, 0xf2, 0xf2), // 15 White
    ]
}

/// Back-compat: previous callers expect `[gpui::Rgba; 16]`. Forwards to
/// [`campbell_color_table`] then converts at the boundary.
pub fn campbell_palette() -> [gpui::Rgba; 16] {
    let mut out = [gpui::Rgba::default(); 16];
    let tbl = campbell_color_table();
    for (i, c) in tbl.into_iter().enumerate() {
        out[i] = c.to_gpui();
    }
    out
}

/// Resolve a standard xterm 256-color palette index. `16..=231` = 6×6×6 cube,
/// `232..=255` = grayscale ramp. `0..=15` is supplied by `named`. Mirrors
/// `Microsoft::Console::Utils::InitializeExtendedColorTable`
/// (`windows/src/types/colorTable.cpp:242` onward).
pub fn xterm256(index: u8, named: &[Color; 16]) -> Color {
    match index {
        0..=15 => named[index as usize],
        16..=231 => {
            let i = (index - 16) as usize;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            // WT uses these exact levels (from `InitializeExtendedColorTable`):
            // 0, 95, 135, 175, 215, 255.
            let levels = [0u8, 95, 135, 175, 215, 255];
            Color::from_rgb(levels[r], levels[g], levels[b])
        }
        232..=255 => {
            // WT: `8 + (index - 232) * 10` across all channels, 24 steps.
            let v = 8u8.saturating_add((index - 232).wrapping_mul(10));
            Color::from_rgb(v, v, v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity: Campbell table matches `colorTable.cpp:11` byte-for-byte.
    #[test]
    fn campbell_table_matches_color_table_cpp() {
        let tbl = campbell_color_table();
        // spot-check a few entries against the C++ source.
        assert_eq!(tbl[0].abgr() & 0xffffff, 0x0c0c0c); // Black
        assert_eq!(tbl[1].abgr() & 0xffffff, 0x1f0fc5); // DarkRed (ABGR)
        assert_eq!(tbl[4].abgr() & 0xffffff, 0xda3700); // DarkBlue
        assert_eq!(tbl[15].abgr() & 0xffffff, 0xf2f2f2); // White
                                                         // Alpha forced to 0xFF for every Campbell entry (constructor that
                                                         // omits alpha defaults a to 255 in `til/color.h:54`).
        for c in tbl {
            assert_eq!(c.a(), 0xff);
        }
    }

    /// Parity: `from_colorref` masks RGB into the low 24 bits and forces 0xFF
    /// alpha, matching `til::color(COLORREF c)` (`til/color.h:69`).
    #[test]
    fn colorref_round_trip_matches_til_color() {
        // COLORREF packs as 0x00BBGGRR (Win32 `RGB()` macro). For `0x0037da`:
        // R = 0xda, G = 0x37, B = 0x00 — i.e. the WT "Dark Blue" COLORREF.
        let c = Color::from_colorref(0x0037da);
        assert_eq!(c.r(), 0xda);
        assert_eq!(c.g(), 0x37);
        assert_eq!(c.b(), 0x00);
        assert_eq!(c.a(), 0xff);
        // ABGR layout: alpha high byte, then B, G, R in the next three bytes.
        // For R=0xda, G=0x37, B=0x00, A=0xff: 0xff_00_37_da.
        assert_eq!(c.abgr(), 0xff0037da);
    }

    /// Parity: xterm cube entry `[5,0,0]` (index 16+5*36 = 196) = pure red
    /// 255,0,0.
    #[test]
    fn xterm256_cube_red_max_is_255() {
        let tbl = campbell_color_table();
        let c = xterm256(196, &tbl);
        assert_eq!(c.r(), 255);
        assert_eq!(c.g(), 0);
        assert_eq!(c.b(), 0);
    }

    /// Parity: grayscale entry at index 232 starts at `8` and ramps by 10.
    #[test]
    fn xterm256_grayscale_ramp_starts_at_8() {
        let tbl = campbell_color_table();
        assert_eq!(xterm256(232, &tbl).r(), 8);
        assert_eq!(xterm256(232, &tbl).g(), 8);
        assert_eq!(xterm256(232, &tbl).b(), 8);
        // 10 steps later (index 242): 8 + 10*10 = 108
        assert_eq!(xterm256(242, &tbl).r(), 108);
    }

    /// `to_gpui` / `From<Rgba>` round-trip a WT color byte-identically.
    /// `gpui::Rgba` stores channels as `f32 0..1`, so we verify the scaling
    /// path (u8 → f32 → u8) loses nothing for Campbell entries (alpha 0xFF).
    #[test]
    fn gpui_round_trip_preserves_components() {
        for c in campbell_color_table() {
            let g = c.to_gpui();
            let back = Color::from(g);
            assert_eq!(back, c, "round-trip failed for {:?}", c);
        }
        // Also exercise a non-opaque alpha.
        let c = Color::from_rgba(0x12, 0x34, 0x56, 0x80);
        let back = Color::from(c.to_gpui());
        assert_eq!(back, c);
    }

    /// `campbell_palette` produces the GPUI view of the Campbell table and
    /// matches the per-entry `to_gpui` conversion.
    #[test]
    fn campbell_palette_matches_to_gpui() {
        let tbl = campbell_color_table();
        let pal = campbell_palette();
        for (i, c) in tbl.into_iter().enumerate() {
            assert_eq!(pal[i], c.to_gpui());
        }
    }

    /// Parity: `from_hex`/`to_hex` round-trip WT's `#rrggbb` wire form, and
    /// `#rgb` short form expands to the doubled-nibble form WT accepts.
    #[test]
    fn hex_round_trip_matches_json_wire_form() {
        let c = Color::from_rgb(0x0c, 0x0c, 0x0c);
        assert_eq!(c.to_hex(), "#0c0c0c");
        let back = Color::from_hex("#0c0c0c").unwrap();
        assert_eq!(back, c);
        assert_eq!(back.a(), 0xff);
        // Short form `"#rgb"` expands to double-nibble (e.g. `#f0a` -> #ff00aa).
        assert_eq!(
            Color::from_hex("#f0a").unwrap(),
            Color::from_rgb(0xff, 0x00, 0xaa)
        );
        // Malformed input yields None (mirrors WT silently skipping bad values).
        assert!(Color::from_hex("nothex").is_none());
        assert!(Color::from_hex("#12").is_none());
    }

    /// Parity: serde uses the `#rrggbb` wire form, matching WT's
    /// `JsonUtils` (de)serialization for `Core::Color`.
    #[test]
    fn serde_color_uses_hex_wire_form() {
        let c = Color::from_rgb(0xc5, 0x0f, 0x1f); // Campbell DarkRed
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, "\"#c50f1f\"");
        let back: Color = serde_json::from_str(&j).unwrap();
        assert_eq!(back, c);
    }
}
