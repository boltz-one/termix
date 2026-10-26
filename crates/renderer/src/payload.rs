//! `RenderingPayload` — extracted from `grid.rs` để giữ file <200 LOC. Port
//! field-for-field `atlas/common.h:501`. Phase 2 thêm `_fillColorBitmap`
//! (`AtlasEngine.cpp:364`) 3-slab write + generations gate.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::grid::{LineRendition, ShapedRow};

/// `RenderingPayload` — port field-for-field `atlas/common.h:501`. WT có
/// `settings`, `unordered_rows`, `rows_scratch`, `rows`, `color_bitmap` (3
/// slabs bg/fg/ul với row_stride + depth_stride), `color_bitmap_generations`
/// (3 × generation_t), `dirty_rect_in_px`, `invalidated_rows`,
/// `cursor_rect`, `scroll_offset_x`, `scroll_delta_y`.
///
/// Phase 2: `color_bitmap` là `Vec<u32>` phẳng với layout
/// `[row][depth 0..3][col]`. `color_bitmap_depth_stride` = `cell_count_x`
/// (hoặc `2 * cell_count_x` cho double-width rows). `row_stride` =
/// `depth_stride * 3`. Generations bumped khi buffer thay đổi — gate GPU
/// upload (`common.h:501`).
#[derive(Debug, Default)]
pub struct RenderingPayload {
    /// FontRuntime snapshot (cell_size, baseline, …) — port `GenerationalSettings`.
    /// `Option` vì `FontRuntime` chưa impl `Default` (port `tg_font`); phase 2
    /// sẽ set ngay khi `_resolveFontMetrics` chạy.
    pub settings: Option<font::FontRuntime>,
    /// Shaped rows chưa sort theo dirty order.
    pub unordered_rows: Vec<ShapedRow>,
    /// Scratch buffer cho sort/swap.
    pub rows_scratch: Vec<ShapedRow>,
    /// Trỏ đến shaped rows đã sort (WT `Buffer<ShapedRow*>`).
    pub rows: Vec<*const ShapedRow>,
    /// 3-slab color bitmap phẳng: bg (premultiplied) | fg (straight) | ul
    /// (straight). WT `_fillColorBitmap` tại `AtlasEngine.cpp:364`.
    pub color_bitmap: Vec<u32>,
    /// Số u32 mỗi row trên mỗi slab.
    pub color_bitmap_row_stride: usize,
    /// Số u32 giữa cùng row nhưng slab khác (bg→fg→ul).
    pub color_bitmap_depth_stride: usize,
    /// `[bg_gen, fg_gen, ul_gen]` — port `generation_t[3]`.
    pub color_bitmap_generations: [AtomicU32; 3],
    /// Dirty rect (x, y, w, h) trong px.
    pub dirty_rect_in_px: (i32, i32, i32, i32),
    /// Marked-dirty rows (y, count).
    pub invalidated_rows: (u16, u16),
    /// Cursor rect (x, y, w, h) trong px.
    pub cursor_rect: (i32, i32, i32, i32),
    /// Horizontal scroll offset (px).
    pub scroll_offset_x: i32,
    /// Delta y scroll so với frame trước (px, row*cellHeight).
    pub scroll_delta_y: i32,
}

impl RenderingPayload {
    /// `_fillColorBitmap` — port verbatim `AtlasEngine.cpp:364`. Viết 3 slabs
    /// (bg premul / fg straight / ul straight) cho row `y`, cột `x1..x2`.
    ///
    /// WT:
    /// ```cpp
    /// const auto bitmap = _p.colorBitmap.begin() + _p.colorBitmapRowStride * y;
    /// const auto shift = (row.lineRendition != LineRendition::SingleWidth);
    /// auto beg = bitmap + (x1 << shift);
    /// auto end = bitmap + (x2 << shift);
    /// const u32 colors[] = { premul(bgColor), fgColor, ulColor };
    /// for (i in 0..3) {
    ///     const auto color = colors[i];
    ///     for (it = beg; it != end; ++it) {
    ///         if (*it != color) {
    ///             _p.colorBitmapGenerations[i].bump();
    ///             std::fill(it, end, color);
    ///             break;
    ///         }
    ///     }
    ///     beg += _p.colorBitmapDepthStride;
    ///     end += _p.colorBitmapDepthStride;
    /// }
    /// ```
    ///
    /// **Reproduction bắt buộc**: bump generation chỉ khi cell thực sự
    /// đổi (`*it != color`) — port `generation_t::bump` (`til/generations.h`).
    /// Cell width double cho `LineRendition::DoubleWidthSingleHeight` (ù
    /// applies cho `shift = 1`).
    ///
    /// Returns `true` nếu bất kỳ slab nào bị bumped — caller dùng để gate
    /// GPU upload.
    #[allow(clippy::too_many_arguments)] // Port WT `_FillColorBitmap` 8-arg signature verbatim.
    pub fn fill_color_bitmap(
        &mut self,
        y: usize,
        x1: usize,
        x2: usize,
        fg_color: u32,
        bg_color: u32,
        ul_color: u32,
        line_rendition: LineRendition,
    ) -> bool {
        // Port WT::Allocation guards. Phase 2: caller must ensure bitmap
        // allocated; we no-op silently if not (matches WT `_p.colorBitmap`
        // lazy init returning zero-size).
        if self.color_bitmap_row_stride == 0 || self.color_bitmap_depth_stride == 0 {
            return false;
        }
        let row_stride = self.color_bitmap_row_stride;
        let depth_stride = self.color_bitmap_depth_stride;
        let row_base = y.wrapping_mul(row_stride);
        if row_base >= self.color_bitmap.len() {
            return false;
        }

        let shift = if line_rendition != LineRendition::SingleWidthSingleHeight {
            1usize
        } else {
            0
        };
        let cell_x1 = x1 << shift;
        let cell_x2 = x2 << shift;
        // Guard against out-of-range writes — caller passes raw row index
        // and column span; we clamp to slab bounds.
        let cells_per_slab = depth_stride;
        let end_x = cell_x2.min(cells_per_slab);
        if cell_x1 >= end_x || row_base.saturating_add(end_x) > self.color_bitmap.len() {
            return false;
        }

        let colors = [u32_color_premultiply(bg_color), fg_color, ul_color];
        let mut any_bumped = false;
        // Each slab iteration bumps both `beg` and `end` by `depth_stride`,
        // matching WT `beg += depth_stride; end += depth_stride;`.
        let mut slab_beg = row_base + cell_x1;
        let mut slab_end = row_base + end_x;
        for (i, &color) in colors.iter().enumerate() {
            // Guard against out-of-bounds.
            if slab_beg >= self.color_bitmap.len() || slab_end > self.color_bitmap.len() {
                break;
            }
            // Walk [slab_beg..slab_end) — bump gen on first diff, fill rest.
            // Matches WT inline loop:
            //   for (it = beg; it != end; ++it) {
            //       if (*it != color) { bump(); std::fill(it, end, color); break; }
            //   }
            for it in slab_beg..slab_end {
                if self.color_bitmap[it] != color {
                    bump_generation(&self.color_bitmap_generations[i]);
                    for j in it..slab_end {
                        self.color_bitmap[j] = color;
                    }
                    any_bumped = true;
                    break;
                }
            }
            // Advance to next slab (same row, same x-range, next depth).
            slab_beg = slab_beg.saturating_add(depth_stride);
            slab_end = slab_end.saturating_add(depth_stride);
        }
        any_bumped
    }
}

/// Bump `generation_t` — port `til/generations.h::generation_t::bump`.
/// Wrap-on-overflow using `fetch_add(1, Relaxed)`. Phase 1: start at 1 (so
/// `0` is the "never bumped" sentinel).
#[inline]
fn bump_generation(gen: &AtomicU32) {
    // Wrap-on-overflow using fetch_add; skip 0 reserved sentinel.
    let prev = gen.fetch_add(1, Ordering::Relaxed);
    if prev.wrapping_add(1) == 0 {
        //極 rare: skip over the 0 sentinel.
        let _ = gen.fetch_add(1, Ordering::Relaxed);
    }
}

/// `u32ColorPremultiply` — port `til/color.h:u32ColorPremultiply`. ABGR packed:
/// alpha <<= 24 | b <<= 16 | g <<= 8 | r. Premultiplied: rgb channels được
/// nhân với `a / 255`.
///
/// WT dùng `DirectX::XMUNibble4`-style pack: `*((DirectX::XMUNIBBLE4*)&bg)`.
/// `premul.r = (color.r * color.a + 255) >> 8`. Alpha không đổi.
const fn u32_color_premultiply(abgr: u32) -> u32 {
    let r = abgr & 0xFF;
    let g = (abgr >> 8) & 0xFF;
    let b = (abgr >> 16) & 0xFF;
    let a = (abgr >> 24) & 0xFF;
    // WT: `(c * a + 255) >> 8` (round-up).
    let pr = (r * a + 255) >> 8;
    let pg = (g * a + 255) >> 8;
    let pb = (b * a + 255) >> 8;
    (a << 24) | (pb << 16) | (pg << 8) | pr
}

// Safety: `RenderingPayload` hold raw ptr để `ShapedRow` — phase 1 không
// gửi cross-thread (single-threaded paint). Phase 2 có thể cần rework.
unsafe impl Send for RenderingPayload {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tạo một `RenderingPayload` với layout 3 slabs × 8 cells × 4 rows
    /// để test `_fillColorBitmap`.
    fn make_payload(depth_stride: usize, rows: usize) -> RenderingPayload {
        let row_stride = depth_stride * 3;
        let total = row_stride * rows;
        RenderingPayload {
            color_bitmap: vec![0u32; total],
            color_bitmap_row_stride: row_stride,
            color_bitmap_depth_stride: depth_stride,
            ..RenderingPayload::default()
        }
    }

    #[test]
    fn fill_color_bitmap_writes_all_3_slabs_at_row_y() {
        let mut p = make_payload(8, 4);
        let y = 1;
        let x1 = 2;
        let x2 = 6;
        let bg = 0xFF112233;
        let fg = 0xFF445566;
        let ul = 0xFF778899;
        let bumped = p.fill_color_bitmap(
            y,
            x1,
            x2,
            fg,
            bg,
            ul,
            LineRendition::SingleWidthSingleHeight,
        );
        assert!(bumped);
        // Compute expected offsets: row 1, slab 0 (bg) — premultiplied.
        let premul_bg = u32_color_premultiply(bg);
        let row_stride = p.color_bitmap_row_stride;
        let depth_stride = p.color_bitmap_depth_stride;
        let row_base = y * row_stride;
        // Cells [x1..x2) of slab 0 should be premul_bg; cells outside unchanged.
        for x in 0..depth_stride {
            let expected = if (x1..x2).contains(&x) { premul_bg } else { 0 };
            assert_eq!(p.color_bitmap[row_base + x], expected, "bg slab @{}", x);
            assert_eq!(
                p.color_bitmap[row_base + depth_stride + x],
                if (x1..x2).contains(&x) { fg } else { 0 },
                "fg slab @{}",
                x
            );
            assert_eq!(
                p.color_bitmap[row_base + 2 * depth_stride + x],
                if (x1..x2).contains(&x) { ul } else { 0 },
                "ul slab @{}",
                x
            );
        }
    }

    #[test]
    fn fill_color_bitmap_bumps_generation_on_change_only() {
        let mut p = make_payload(8, 2);
        // First write changes from 0 → non-zero, so all 3 gens bump.
        let bg = 0xFF112233;
        let fg = 0xFF445566;
        let ul = 0xFF778899;
        p.fill_color_bitmap(0, 0, 4, fg, bg, ul, LineRendition::SingleWidthSingleHeight);
        let g_after_first = [
            p.color_bitmap_generations[0].load(Ordering::Relaxed),
            p.color_bitmap_generations[1].load(Ordering::Relaxed),
            p.color_bitmap_generations[2].load(Ordering::Relaxed),
        ];
        assert!(g_after_first[0] >= 1, "bg gen bumped");
        assert!(g_after_first[1] >= 1, "fg gen bumped");
        assert!(g_after_first[2] >= 1, "ul gen bumped");
        // Second write với cùng colors, cùng range — no bump.
        p.fill_color_bitmap(0, 0, 4, fg, bg, ul, LineRendition::SingleWidthSingleHeight);
        let g_after_second = [
            p.color_bitmap_generations[0].load(Ordering::Relaxed),
            p.color_bitmap_generations[1].load(Ordering::Relaxed),
            p.color_bitmap_generations[2].load(Ordering::Relaxed),
        ];
        assert_eq!(
            g_after_first, g_after_second,
            "no repeat bump for identical writes"
        );
    }

    #[test]
    fn fill_color_bitmap_double_width_shifts_x_range() {
        let mut p = make_payload(8, 1);
        let bg = 0xFF112233;
        let fg = 0xFF445566;
        let ul = 0xFF778899;
        p.fill_color_bitmap(0, 1, 3, fg, bg, ul, LineRendition::DoubleWidthSingleHeight);
        // shift=1: x1=2, x2=6 — fills 4 cells.
        let depth_stride = p.color_bitmap_depth_stride;
        let row_base = 0;
        for x in 0..depth_stride {
            let expected = if (2..6).contains(&x) { fg } else { 0 };
            assert_eq!(
                p.color_bitmap[row_base + depth_stride + x],
                expected,
                "fg slab double-width @{}",
                x
            );
        }
    }

    #[test]
    fn fill_color_bitmap_no_op_when_bitmap_unallocated() {
        let mut p = RenderingPayload::default();
        let bumped = p.fill_color_bitmap(
            0,
            0,
            4,
            0xFF445566,
            0xFF112233,
            0xFF778899,
            LineRendition::SingleWidthSingleHeight,
        );
        assert!(!bumped);
    }

    #[test]
    fn fill_color_bitmap_clamps_out_of_range_x() {
        let mut p = make_payload(4, 1);
        // x2 = 10 > depth_stride (4) — clamp to 4.
        let bumped = p.fill_color_bitmap(
            0,
            2,
            10,
            0xFF445566,
            0xFF112233,
            0xFF778899,
            LineRendition::SingleWidthSingleHeight,
        );
        assert!(bumped);
        // Only cells [2..4) được fill, rest unchanged.
        let row_base = 0;
        let depth_stride = 4;
        for x in 0..depth_stride {
            let expected_fg = if (2..4).contains(&x) { 0xFF445566 } else { 0 };
            assert_eq!(p.color_bitmap[row_base + depth_stride + x], expected_fg);
        }
    }

    #[test]
    fn u32_color_premultiply_matches_wt_formula() {
        // WT: pr = (r*a + 255) >> 8. For r=0xAA, a=0x80: pr = (170*128 + 255) >> 8 = 21815 >> 8 = 85 (=0x55).
        let abgr = 0xFF88_00AA; // r=0xAA, g=0, b=0x88, a=0xFF
        let premul = u32_color_premultiply(abgr);
        // Since a=255, premul = identity (no premultiplicative change).
        assert_eq!(premul, abgr);
        // Half-alpha: r=0xAA (170), a=0x80 (128): pr = (170*128 + 255) >> 8 = 21790+255 >> 8 = 22045 >> 8 = 86
        let abgr_half = 0x8088_00AA; // a=0x80
        let premul_half = u32_color_premultiply(abgr_half);
        let pr = premul_half & 0xFF;
        let pg = (premul_half >> 8) & 0xFF;
        let pb = (premul_half >> 16) & 0xFF;
        let pa = (premul_half >> 24) & 0xFF;
        // r=0xAA (170), a=0x80 (128): pr = (170*128 + 255) >> 8 = 22015 / 256 = 85 (floor).
        assert_eq!(pr, 85);
        assert_eq!(pg, 0);
        // b=0x88 (136), a=0x80 (128): pb = (136*128 + 255) >> 8 = 17663 / 256 = 68 (floor).
        assert_eq!(pb, 68);
        assert_eq!(pa, 0x80); // alpha unchanged
    }
}
