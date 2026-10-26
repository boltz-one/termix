//! `RendererState` — port WT `renderer/base/renderer.cpp` invalidation/teardown state.
//!
//! Loop 04 skeleton (P10 step 2a): trigger methods + state getters, KHÔNG paint
//! driver, KHÔNG engine backend (ADR-0003 — GPUI owns scene). Loop sau wire
//! `cx.notify()` + scroll/cursor/paint. Port slice ban đầu từ `renderer.cpp:656-716`
//! (`TriggerRedraw`/`TriggerRedrawAll`/`TriggerTeardown`) + `renderer.cpp:110`
//! (`TriggerTeardown`) + `renderer.hpp:181-185` (`_fInvalidRectUsed`/`_fAllInvalidated`
//! /`_fTeardown`/`_fInvalidRect`).
//!
//! **Naming**: WT class tên `Renderer`; crate `paint::Renderer` (`paint.rs`) đã port
//! paint driver (`paint_frame_for_engine`/`paint_selection`/`paint_cursor_internal`)
//! — cùng class WT, slice khác. Để tránh xung đột top-level `Renderer` (xem
//! `lib.rs:44 pub use paint::Renderer`), loop-04 state slice đặt `RendererState`.
//! Loop sau khi gộp state vào `paint::Renderer` (P10 step 6+), rename/squash lại.
//!
//! **`InvalidRect` convention**: WT `_fInvalidRect` là `til::rect` (exclusive
//! bottom-right). `trigger_redraw(&Viewport)` port verbatim WT
//! `TriggerRedraw(Viewport region)` — pass field-by-field `Viewport` (inclusive
//! `right/bottom`) → `InvalidRect` (giữ nguyên tọa độ, không +1 convert);
//! `trigger_redraw_point` xây `InvalidRect` 1×1 exclusive trực tiếp
//! (`right = left + 1`, `bottom = top + 1`) — mirror WT
//! `Viewport::FromDimensions(p, {1,1})` truyền cho `til::rect` exclusive. Hai
//! path có convention hơi khác (VR2 pass-through inclusive, VR3 point exclusive)
//! — đây là re-expression verbatim theo VR của D_4, không phải WT-faithful
//! convention thống nhất; loop sau nếu cần exclusive thống nhất sẽ refactor.

use core::Viewport;

/// Invalidation region — port WT `_fInvalidRect` (`til::rect`, renderer.hpp:181).
/// `None` = chưa có invalid region accumulated (state `_fInvalidRectUsed=false`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvalidRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// `RendererState` skeleton — port WT `renderer/base/renderer.cpp:110,656-716`.
///
/// State observable qua getters; trigger methods mutate state, không gọi engine
/// backend (ADR-0003 behavioral parity — GPUI owns scene/paint scheduler ở loop
/// sau). WT `_pEngine->Invalidate(&region)` / `_pEngine->InvalidateAll()` được
/// bỏ — scheduler downstream (GPUI `cx.notify()`) loop sau.
#[derive(Clone, Debug, Default)]
pub struct RendererState {
    /// `_fInvalidRectUsed` (renderer.hpp:181) — true nếu có invalid rect accumulated.
    invalid_rect_used: bool,
    /// `_fInvalidRect` — accumulated invalid region (None when !used).
    invalid_rect: Option<InvalidRect>,
    /// `_fAllInvalidated` (renderer.hpp:181) — true sau `TriggerRedrawAll`.
    all_invalidated: bool,
    /// `_fTeardown` (renderer.cpp:110) — true sau `TriggerTeardown`.
    teardown_requested: bool,
    /// `_lastFrameType |= Frame::Background` — port `TriggerRedrawAll(bgChanged=true)`.
    /// Sticky flag (mirror WT `|=` không clear).
    background_changed: bool,
    /// `_pfnFrameCompiledCallback` trigger — port `TriggerRedrawAll(frameChanged=true)`.
    /// Sticky flag.
    frame_changed: bool,
}

impl RendererState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Port `TriggerRedraw(Viewport region)` (renderer.cpp:656). Union region vào
    /// `_fInvalidRect`, set `_fInvalidRectUsed=true`, clear `_fAllInvalidated`.
    /// WT cũng gọi `_pEngine->Invalidate(&region)` — bỏ (no engine; GPUI scene).
    pub fn trigger_redraw(&mut self, region: &Viewport) {
        let r = InvalidRect {
            left: region.left,
            top: region.top,
            right: region.right,
            bottom: region.bottom,
        };
        self.union_rect(r);
        self.all_invalidated = false;
    }

    /// Port `TriggerRedraw(point*)` (renderer.cpp:691) — WT delegate
    /// `TriggerRedraw(Viewport::FromDimensions(*pcoord, {1,1}))`. Xây `InvalidRect`
    /// 1×1 exclusive (`right = left + 1`, `bottom = top + 1`) trực tiếp rồi union.
    pub fn trigger_redraw_point(&mut self, point: (i32, i32)) {
        let r = InvalidRect {
            left: point.0,
            top: point.1,
            right: point.0 + 1,
            bottom: point.1 + 1,
        };
        self.union_rect(r);
        self.all_invalidated = false;
    }

    /// Port `TriggerRedrawAll(bgChanged, frameChanged)` (renderer.cpp:704).
    /// `_fAllInvalidated=true`; clear `_fInvalidRect*` (all = superset). Set
    /// bg/frame flags (sticky — mirror WT `_lastFrameType |=` / callback set).
    /// WT `_pEngine->InvalidateAll()` — bỏ (no engine).
    pub fn trigger_redraw_all(&mut self, background_changed: bool, frame_changed: bool) {
        self.all_invalidated = true;
        self.invalid_rect = None;
        self.invalid_rect_used = false;
        if background_changed {
            self.background_changed = true;
        }
        if frame_changed {
            self.frame_changed = true;
        }
    }

    /// Port `TriggerTeardown()` (renderer.cpp:110). `_fTeardown=true`. Teardown
    /// không invalidate (không set invalid_rect), mirror WT line 110.
    pub fn trigger_teardown(&mut self) {
        self.teardown_requested = true;
    }

    /// Helper — union `r` vào `invalid_rect` (bbox: min left/top, max right/bottom).
    /// Set `invalid_rect_used=true`. Dùng chung cho `trigger_redraw` (Viewport)
    /// và `trigger_redraw_point` (1×1 exclusive).
    fn union_rect(&mut self, r: InvalidRect) {
        self.invalid_rect = Some(match self.invalid_rect.take() {
            None => r,
            Some(existing) => InvalidRect {
                left: existing.left.min(r.left),
                top: existing.top.min(r.top),
                right: existing.right.max(r.right),
                bottom: existing.bottom.max(r.bottom),
            },
        });
        self.invalid_rect_used = true;
    }

    /// Invalidation pending (rect used OR all invalidated).
    pub fn is_invalidated(&self) -> bool {
        self.invalid_rect_used || self.all_invalidated
    }
    pub fn is_all_invalidated(&self) -> bool {
        self.all_invalidated
    }
    pub fn invalid_rect(&self) -> Option<&InvalidRect> {
        self.invalid_rect.as_ref()
    }
    pub fn is_teardown_requested(&self) -> bool {
        self.teardown_requested
    }
    pub fn background_changed(&self) -> bool {
        self.background_changed
    }
    pub fn frame_changed(&self) -> bool {
        self.frame_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::Viewport;

    /// VR2 — `trigger_redraw(Viewport)` sets invalid_rect + used; clears all_invalidated.
    #[test]
    fn test_trigger_redraw_sets_invalid_rect_and_used() {
        let mut r = RendererState::new();
        assert!(!r.is_invalidated(), "clean state: not invalidated");
        r.trigger_redraw(&Viewport::new(5, 7, 20, 15));
        assert!(r.is_invalidated());
        assert_eq!(
            r.invalid_rect(),
            Some(&InvalidRect {
                left: 5,
                top: 7,
                right: 20,
                bottom: 15
            })
        );
        assert!(!r.is_all_invalidated());
        assert!(!r.is_teardown_requested());
    }

    /// VR3 — `trigger_redraw_point((5,7))` → 1×1 exclusive (right6, bottom8).
    #[test]
    fn test_trigger_redraw_point_one_by_one() {
        let mut r = RendererState::new();
        r.trigger_redraw_point((5, 7));
        assert_eq!(
            r.invalid_rect(),
            Some(&InvalidRect {
                left: 5,
                top: 7,
                right: 6,
                bottom: 8
            })
        );
    }

    /// VR4 — bbox union accumulate (min left/top, max right/bottom).
    #[test]
    fn test_trigger_redraw_accumulates_bbox_union() {
        let mut r = RendererState::new();
        r.trigger_redraw(&Viewport::new(5, 7, 10, 12));
        r.trigger_redraw(&Viewport::new(20, 3, 25, 8));
        assert_eq!(
            r.invalid_rect(),
            Some(&InvalidRect {
                left: 5,
                top: 3,
                right: 25,
                bottom: 12
            })
        );
    }

    /// VR5 part 1 — `trigger_redraw_all` clears rect, sets all_invalidated; sau
    /// `trigger_redraw` clears all_invalidated (mirror WT line 708).
    #[test]
    fn test_trigger_redraw_all_clears_rect_sets_all_invalidated() {
        let mut r = RendererState::new();
        // Accumulate rect trước.
        r.trigger_redraw(&Viewport::new(5, 7, 10, 12));
        assert!(r.invalid_rect().is_some());
        // trigger_redraw_all superset → clear rect, set all.
        r.trigger_redraw_all(false, false);
        assert!(r.is_all_invalidated());
        assert!(r.invalid_rect().is_none(), "all-invalidated clears rect");
        assert!(!r.invalid_rect_used);
        assert!(r.is_invalidated(), "all_invalidated ⇒ is_invalidated");
        // Sau trigger_redraw(region) → all_invalidated cleared (mirror WT
        // `TriggerRedraw` set `_fAllInvalidated=false`).
        r.trigger_redraw(&Viewport::new(0, 0, 3, 3));
        assert!(!r.is_all_invalidated(), "TriggerRedraw clears all_invalidated");
        assert!(r.is_invalidated(), "rect used lại");
        assert!(r.invalid_rect().is_some());
    }

    /// VR5 part 2 — `trigger_redraw_all(true,true)` records bg/frame flags; sticky
    /// (sau `trigger_redraw_all(false,false)` không clear, mirror WT `|=`).
    #[test]
    fn test_trigger_redraw_all_records_bg_frame_flags() {
        let mut r = RendererState::new();
        r.trigger_redraw_all(true, true);
        assert!(r.background_changed());
        assert!(r.frame_changed());
        // Sticky — `trigger_redraw_all(false,false)` không clear.
        r.trigger_redraw_all(false, false);
        assert!(r.background_changed(), "bg flag sticky");
        assert!(r.frame_changed(), "frame flag sticky");
    }

    /// VR6 — `trigger_teardown()` sets teardown_requested; không invalidate.
    #[test]
    fn test_trigger_teardown_sets_flag() {
        let mut r = RendererState::new();
        r.trigger_teardown();
        assert!(r.is_teardown_requested());
        assert!(!r.is_invalidated(), "teardown không invalidate");
        assert!(r.invalid_rect().is_none());
    }
}
