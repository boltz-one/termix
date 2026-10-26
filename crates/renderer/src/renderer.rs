//! `RendererState` — port WT `renderer/base/renderer.cpp:110,656-722`
//! `TriggerRedraw/All/Teardown` + `renderer.hpp:175-185` state members
//! (`_threadKeepRunning` / `_pfnBackgroundColorChanged` / `_pfnFrameColorChanged`
//! / `_engines`).
//!
//! Loop 05 rework (close L04-G01/G02/G03): port-faithful per-engine dispatch
//! (`Invalidate(&region)` / `InvalidateAll()`) + thread lifecycle state.
//! KHÔNG còn GdiEngine accumulation pattern (`_rcInvalid` / `_fInvalidRectUsed`
//! thuộc `renderer/gdi/gdirenderer.hpp:159-160`, out-of-scope ADR-0001). WT
//! `renderer/base Renderer` không có `_fInvalidRect` accumulator, không có
//! `_lastFrameType` enum hay `_pfnFrameCompiledCallback` (zero hit fabricated
//! loop-04). `_pfnBackgroundColorChanged` / `_pfnFrameColorChanged` là function
//! pointer fire-and-forget (renderer.cpp:715,720), không sticky state.
//!
//! Re-expression trên Rust+GPUI (ADR-0003 behavioral parity): GPUI owns
//! swapchain/scene → `_pEngine->Invalidate*` direct call替换 thành trait dispatch
//! `RenderEngine::invalidate(*)` trên `Vec<Box<dyn RenderEngine>>` (loop sau
//! GPUI engine impl); `NotifyPaintFrame()` wire sang `cx.notify()` loop sau;
//! `_threadKeepRunning` (atomic) + `_thread` handle presence → `AtomicBool`
//! + `thread_handle_active` bool. Loop 05 chỉ port dispatch + state observable.

use core::Viewport;
use std::sync::atomic::{AtomicBool, Ordering};

/// Invalidation rect — port `til::rect` (TIL primitive). GdiEngine uses
/// `til::rect` for accumulation (`_rcInvalid`), nhưng WT `renderer/base` KHÔNG
/// accumulate. Loop 05 giữ `InvalidRect` cho `RenderEngine` trait API +
/// future scroll/paint; không longer used as `RendererState` accumulator field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvalidRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// `RenderEngine` trait — port slice WT `RenderEngineBase` / `IRenderEngine`
/// (`renderer/inc/IRenderEngine.hpp`, `renderer/base/RenderEngineBase.hpp`) API
/// cần cho `TriggerRedraw` / `TriggerRedrawAll` dispatch (renderer.cpp:656-722).
/// Full trait port loop sau (P10 step 5+); loop 05 chỉ `invalidate` + `invalidate_all`.
pub trait RenderEngine {
    /// WT `RenderEngineBase::Invalidate(&region)` — per-engine invalidation
    /// (renderer.cpp:664-668 `pEngine->Invalidate(&srUpdateRegion)`).
    fn invalidate(&mut self, region: &Viewport);
    /// WT `RenderEngineBase::InvalidateAll()` — full-frame invalidation
    /// (renderer.cpp:710-712 `pEngine->InvalidateAll()`).
    fn invalidate_all(&mut self);
}

/// `RendererState` — port WT `renderer/base/renderer.cpp:110,656-722` +
/// `renderer.hpp:175-185`. Per-engine dispatch + thread lifecycle state.
///
/// State observable qua getters; trigger methods mutate state + dispatch trait
/// calls, không wire GPUI scene (ADR-0003 — `cx.notify()` loop sau).
pub struct RendererState {
    /// WT `_threadKeepRunning` (atomic bool, renderer.hpp) — `true` while
    /// render thread active. `TriggerTeardown` (renderer.cpp:113) stores `false`.
    thread_keep_running: AtomicBool,
    /// WT `_thread` handle presence (re-expression — Rust không có portable
    /// Windows HANDLE; `true` khi thread spawned, `false` sau `trigger_teardown`).
    thread_handle_active: bool,
    /// WT `_engines: std::vector<std::unique_ptr<RenderEngineBase>>` — per-engine
    /// dispatch list (renderer.cpp:664 loop `for (auto& pEngine : _engines)`).
    /// Loop 05: `RenderEngine` trait minimal; GPUI engine impl loop sau.
    engines: Vec<Box<dyn RenderEngine>>,
    /// WT `_pfnBackgroundColorChanged` (renderer.hpp:175, std::function<void()>)
    /// — fire-and-forget callback, gọi trong `TriggerRedrawAll(bgChanged=true)`
    /// (renderer.cpp:715). Optional setter.
    on_background_color_changed: Option<Box<dyn FnMut()>>,
    /// WT `_pfnFrameColorChanged` (renderer.hpp:176) — gọi trong
    /// `TriggerRedrawAll(frameChanged=true)` (renderer.cpp:720).
    on_frame_color_changed: Option<Box<dyn FnMut()>>,
}

impl RendererState {
    pub fn new() -> Self {
        Self {
            thread_keep_running: AtomicBool::new(false),
            thread_handle_active: false,
            engines: Vec::new(),
            on_background_color_changed: None,
            on_frame_color_changed: None,
        }
    }

    /// WT re-expression — mirror render thread init (Renderer ctor spawns
    /// `_thread`). Loop 05 chỉ set lifecycle flags; actual thread spawn loop sau
    /// (GPUI owns render loop, không cần std::thread).
    pub fn start_thread(&mut self) {
        self.thread_keep_running.store(true, Ordering::SeqCst);
        self.thread_handle_active = true;
    }

    /// Port `TriggerRedraw(Viewport region)` (renderer.cpp:656-683). WT:
    /// region.ToExclusive → trim viewport → `for pEngine in _engines:
    /// pEngine->Invalidate(&region)` → `NotifyPaintFrame()`. Loop 05 port
    /// dispatch slice; `NotifyPaintFrame()` wire `cx.notify()` loop sau.
    /// **KHÔNG accumulate bbox union** (GdiEngine pattern, out-of-scope).
    pub fn trigger_redraw(&mut self, region: &Viewport) {
        for engine in &mut self.engines {
            engine.invalidate(region);
        }
    }

    /// Port `TriggerRedraw(point*)` (renderer.cpp:691) — WT delegate
    /// `TriggerRedraw(Viewport::FromDimensions(*pcoord, {1,1}))`. Viewport
    /// inclusive `right==left`, `bottom==top` cho 1×1 cell (width=1, height=1).
    pub fn trigger_redraw_point(&mut self, point: (i32, i32)) {
        let region = Viewport::new(point.0, point.1, point.0, point.1);
        self.trigger_redraw(&region);
    }

    /// Port `TriggerRedrawAll(bgChanged, frameChanged)` (renderer.cpp:704-722).
    /// WT: `for pEngine in _engines: pEngine->InvalidateAll()` →
    /// `NotifyPaintFrame()` → if bg `_pfnBackgroundColorChanged()` → if frame
    /// `_pfnFrameColorChanged()`. Callback fire-and-forget, **không sticky flags**
    /// (loop-04 `_lastFrameType |=` / `_pfnFrameCompiledCallback` was fabricated).
    pub fn trigger_redraw_all(&mut self, background_changed: bool, frame_changed: bool) {
        for engine in &mut self.engines {
            engine.invalidate_all();
        }
        if background_changed {
            if let Some(cb) = &mut self.on_background_color_changed {
                cb();
            }
        }
        if frame_changed {
            if let Some(cb) = &mut self.on_frame_color_changed {
                cb();
            }
        }
    }

    /// Port `TriggerTeardown()` (renderer.cpp:110-122). WT: lock mutex →
    /// `_threadKeepRunning.store(false)` → `_shutdownEvent.SetEvent()` →
    /// `NotifyPaintFrame()` → `_enable.SetEvent()` → `WaitForSingleObject(_thread)`
    /// → `_thread.reset()`. Loop 05 re-expression: set `thread_keep_running=false`
    /// + `thread_handle_active=false` (mirror lifecycle observable; actual
    /// thread join GPUI không cần — render loop GPUI-owned).
    pub fn trigger_teardown(&mut self) {
        self.thread_keep_running.store(false, Ordering::SeqCst);
        self.thread_handle_active = false;
    }

    /// WT `_threadKeepRunning.load()` — render thread keeps running.
    pub fn is_thread_keep_running(&self) -> bool {
        self.thread_keep_running.load(Ordering::SeqCst)
    }
    /// WT `_thread` presence — `true` khi thread spawned, `false` sau teardown.
    pub fn is_thread_active(&self) -> bool {
        self.thread_handle_active
    }
    /// WT `_thread.reset()` sauteardown — teardown complete khi thread stopped
    /// + `thread_keep_running` false.
    pub fn is_teardown_complete(&self) -> bool {
        !self.thread_handle_active && !self.is_thread_keep_running()
    }
    /// WT `_engines` accessor (re-expression cho testability).
    pub fn engines(&self) -> &[Box<dyn RenderEngine>] {
        &self.engines
    }
    /// WT `_engines` mutator — push engine vào dispatch list (mirror
    /// `Renderer::AddRenderEngine`).
    pub fn push_engine(&mut self, engine: Box<dyn RenderEngine>) {
        self.engines.push(engine);
    }
    /// WT `_pfnBackgroundColorChanged` setter (renderer.hpp:175).
    pub fn set_on_background_color_changed(&mut self, cb: Box<dyn FnMut()>) {
        self.on_background_color_changed = Some(cb);
    }
    /// WT `_pfnFrameColorChanged` setter (renderer.hpp:176).
    pub fn set_on_frame_color_changed(&mut self, cb: Box<dyn FnMut()>) {
        self.on_frame_color_changed = Some(cb);
    }
}

impl Default for RendererState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Test-only stub engine — track `invalidate` call regions + `invalidate_all`
    /// count. **Not a production mock** — verify port-faithful dispatch behavior
    /// (per-engine Invalidate/InvalidateAll), không fake để pass production test.
    #[derive(Default)]
    struct CountingEngine {
        invalidate_calls: Vec<Viewport>,
        invalidate_all_calls: usize,
    }

    impl RenderEngine for CountingEngine {
        fn invalidate(&mut self, region: &Viewport) {
            self.invalidate_calls.push(*region);
        }
        fn invalidate_all(&mut self) {
            self.invalidate_all_calls += 1;
        }
    }

    /// Shared wrapper để observe `CountingEngine` state sau khi pushed
    /// vào `Vec<Box<dyn RenderEngine>>` (trait object che xuốngcast).
    struct SharedCountingEngine(Rc<RefCell<CountingEngine>>);
    impl RenderEngine for SharedCountingEngine {
        fn invalidate(&mut self, region: &Viewport) {
            self.0.borrow_mut().invalidate(region);
        }
        fn invalidate_all(&mut self) {
            self.0.borrow_mut().invalidate_all();
        }
    }

    /// VR1 — `trigger_redraw` dispatches `invalidate(&region)` to EACH engine
    /// (mirror WT renderer.cpp:664-668 `for pEngine : _engines`).
    #[test]
    fn test_trigger_redraw_dispatches_invalidate_to_each_engine_rc() {
        let mut r = RendererState::new();
        let s1 = Rc::new(RefCell::new(CountingEngine::default()));
        let s2 = Rc::new(RefCell::new(CountingEngine::default()));
        r.push_engine(Box::new(SharedCountingEngine(Rc::clone(&s1))));
        r.push_engine(Box::new(SharedCountingEngine(Rc::clone(&s2))));
        let region = Viewport::new(5, 7, 20, 15);
        r.trigger_redraw(&region);
        assert_eq!(s1.borrow().invalidate_calls, vec![region]);
        assert_eq!(s2.borrow().invalidate_calls, vec![region]);
        assert_eq!(s1.borrow().invalidate_all_calls, 0);
        assert_eq!(s2.borrow().invalidate_all_calls, 0);
    }

    /// VR2 — `trigger_redraw_point((5,7))` delegates to `trigger_redraw` với
    /// 1×1 inclusive Viewport `{5,7,5,7}` (width=1 height=1). Mirror WT
    /// `TriggerRedraw(point*)` (renderer.cpp:691).
    #[test]
    fn test_trigger_redraw_point_one_by_one_delegates() {
        let mut r = RendererState::new();
        let s = Rc::new(RefCell::new(CountingEngine::default()));
        r.push_engine(Box::new(SharedCountingEngine(Rc::clone(&s))));
        r.trigger_redraw_point((5, 7));
        assert_eq!(
            s.borrow().invalidate_calls,
            vec![Viewport::new(5, 7, 5, 7)],
            "1×1 inclusive (inclusive Viewport convention, width=height=1)"
        );
    }

    /// VR3 — `trigger_redraw_all(bg,frame)` dispatches `invalidate_all()` to each
    /// engine + fires callbacks (fire-and-forget, không sticky). Mirror WT
    /// renderer.cpp:704-722.
    #[test]
    fn test_trigger_redraw_all_dispatches_invalidate_all_and_callbacks() {
        let mut r = RendererState::new();
        let s1 = Rc::new(RefCell::new(CountingEngine::default()));
        let s2 = Rc::new(RefCell::new(CountingEngine::default()));
        r.push_engine(Box::new(SharedCountingEngine(Rc::clone(&s1))));
        r.push_engine(Box::new(SharedCountingEngine(Rc::clone(&s2))));
        // Counter callbacks (Rc<RefCell<usize>> — fire-and-forget increment).
        let bg_counter = Rc::new(RefCell::new(0usize));
        let frame_counter = Rc::new(RefCell::new(0usize));
        let bg_for_cb = Rc::clone(&bg_counter);
        let frame_for_cb = Rc::clone(&frame_counter);
        r.set_on_background_color_changed(Box::new(move || {
            *bg_for_cb.borrow_mut() += 1;
        }));
        r.set_on_frame_color_changed(Box::new(move || {
            *frame_for_cb.borrow_mut() += 1;
        }));

        // trigger_redraw_all(true,true) → mỗi engine invalidate_all + cả callback fire.
        r.trigger_redraw_all(true, true);
        assert_eq!(s1.borrow().invalidate_all_calls, 1);
        assert_eq!(s2.borrow().invalidate_all_calls, 1);
        assert!(s1.borrow().invalidate_calls.is_empty(), "All không per-region");
        assert!(s2.borrow().invalidate_calls.is_empty());
        assert_eq!(*bg_counter.borrow(), 1);
        assert_eq!(*frame_counter.borrow(), 1);

        // trigger_redraw_all(false,false) → invalidate_all lại, callbacks KHÔNG fire.
        r.trigger_redraw_all(false, false);
        assert_eq!(s1.borrow().invalidate_all_calls, 2);
        assert_eq!(*bg_counter.borrow(), 1, "bg callback fire-and-forget, không sticky");
        assert_eq!(*frame_counter.borrow(), 1, "frame callback không sticky");
    }

    /// VR4 — `trigger_teardown` sets thread_keep_running=false + thread handle
    /// inactive. Mirror WT renderer.cpp:113 `_threadKeepRunning.store(false)` +
    /// thread reset. `start_thread()` lifecycles observable.
    #[test]
    fn test_trigger_teardown_sets_thread_keep_running_false() {
        let mut r = RendererState::new();
        assert!(!r.is_thread_keep_running(), "new: thread not started");
        assert!(!r.is_thread_active());
        assert!(r.is_teardown_complete(), "new: teardown trivially complete");
        r.start_thread();
        assert!(r.is_thread_keep_running());
        assert!(r.is_thread_active());
        assert!(!r.is_teardown_complete(), "thread active: teardown not complete");
        r.trigger_teardown();
        assert!(!r.is_thread_keep_running());
        assert!(!r.is_thread_active());
        assert!(r.is_teardown_complete(), "sau teardown: complete");
    }

    /// VR5 — `trigger_redraw` KHÔNG accumulate bbox union (loop-04 fabricated
    /// drift closed). Mỗi `trigger_redraw` dispatch riêng; engine nhận N calls
    /// cho N trigger — không bbox union single accumulated call.
    #[test]
    fn test_trigger_redraw_no_bbox_accumulation() {
        let mut r = RendererState::new();
        let s = Rc::new(RefCell::new(CountingEngine::default()));
        r.push_engine(Box::new(SharedCountingEngine(Rc::clone(&s))));
        // 2 trigger disjoint regions → engine nhận 2 calls riêng, KHÔNG accumulate.
        r.trigger_redraw(&Viewport::new(5, 7, 10, 12));
        r.trigger_redraw(&Viewport::new(20, 3, 25, 8));
        assert_eq!(
            s.borrow().invalidate_calls.len(),
            2,
            "2 dispatch riêng, không bbox union accumulate"
        );
        assert_eq!(s.borrow().invalidate_calls[0], Viewport::new(5, 7, 10, 12));
        assert_eq!(s.borrow().invalidate_calls[1], Viewport::new(20, 3, 25, 8));
    }
}
