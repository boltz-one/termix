---
loop: 5
status: pending
preservation_constraints:
  - p1_loop04_workspace_regression   # cargo test --workspace --lib không regress (loop04 baseline 486 pass / 0 fail)
  - p2_loop04_colorfix_unit_intact    # renderer --lib colorfix 8 test still green
  - p3_loop04_settings18_intact       # 13 existing + 5 settings test verbatim pass (18 total)
  - p4_loop04_colortable256_default   # RenderSettings::default() 256-table Campbell unchanged
  - p5_loop04_terminal_seam_intact    # cargo test -p core --test wt_terminalcore 1 pass
  - p6_loop04_wt_colorfix_integration_intact  # cargo test -p renderer --test wt_renderer_colorfix 3 pass
  - p7_loop04_clippy_lib_renderer_clean  # cargo clippy -p renderer --all-targets -- -D warnings clean (L03-G01 closed loop-04, giữ nguyên)
replaces_dev_doc_citation:
  - L04-G02  # D_4 line 37 hallucinate field-citation; D_5 cita đúng
candidate: A_5 (rất lại crates/renderer/src/renderer.rs rework + lib.rs re-export + ledger P10 update + ADR-0016 optional + tests rewrite)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 05 — Close L04-G01/G02/G03: rework `RendererState` port-faithful WT `renderer/base`

## Objective

**Rework `RendererState` port sang WT `renderer/base` thực sự: per-engine `Invalidate(*)` + thread teardown lifecycle. Loại bỏ GdiEngine accumulation pattern (`_rcInvalid`/`_fInvalidRectUsed`) + fabricated `_fTeardown`/`_lastFrameType`/`_pfnFrameCompiledCallback`. Planner quyết hướng (a) port-faithful (vs (b) deliberate-deviation+ADR) — master plan yêu cầu 100% logic parity; GPUI scene full-frame repaint không cần dirty-rect accumulator (ADR-0003).**

Một câu: Sau loop-05, `RendererState` mirror WT `renderer/base` `TriggerRedraw/All/Teardown` actual — thread lifecycle state (`_threadKeepRunning` + `_thread`) + per-engine `Invalidate`/`InvalidateAll` dispatch + `_pfn*ColorChanged` callback presence, không có bbox union accumulator, không fabricated sticky flags.

## Context

**E_4 verdict**: loop-04 `partial` — 3 gap mở:
- **L04-G01** (major): `RendererState` port drift — `_fInvalidRect/_fInvalidRectUsed/_fAllInvalidated` thuộc `GdiEngine` (`renderer/gdi/gdirenderer.hpp:159-160`, `invalidate.cpp:110-135`) out-of-scope ADR-0001; WT `renderer/base Renderer` không có accumulator. `_fTeardown`/`_lastFrameType`/`_pfnFrameCompiledCallback` zero hit fabricated. WT `TriggerRedraw(Viewport)` thực là per-engine `Invalidate(*)` + `NotifyPaintFrame()` (cpp:656-683), không bbox union.
- **L04-G02** (Planner gap): D_4 line 37 cite `renderer.hpp:181-185` chứa `_fInvalidRect...` — sai, 181-185 là selection fields.
- **L04-G03** (Dev gap): doc-comment renderer.rs:47-58 + commit msg propagate fabricated cite.

**WT ground (verify read-only, GT confirmed this loop)**:
- `renderer/base/renderer.cpp:656-683` — `TriggerRedraw(Viewport region)`:
  - `region.ToExclusive()` → `srUpdateRegion`
  - loop over rows: if `buffer.IsDoubleWidthLine(row)` expand `srUpdateRegion.Left`/`Right`
  - if `!view.TrimToViewport(&srUpdateRegion)` return (region outside viewport)
  - `for (auto& pEngine : _engines) { pEngine->Invalidate(&srUpdateRegion); }`
  - `NotifyPaintFrame()`
  - **KHÔNG accumulate bbox**, **KHÔNG set `_fAllInvalidated=false`**, **KHÔNG `_fInvalidRectUsed`**.
- `renderer/base/renderer.cpp:691` — `TriggerRedraw(point*)`: `TriggerRedraw(Viewport::FromDimensions(*pcoord, {1,1}))`.
- `renderer/base/renderer.cpp:704-722` — `TriggerRedrawAll(bgChanged, frameChanged)`:
  - `for (auto& pEngine : _engines) { pEngine->InvalidateAll(); }`
  - `NotifyPaintFrame()`
  - `if (bgChanged) { if (_pfnBackgroundColorChanged) _pfnBackgroundColorChanged(); }`
  - `if (frameChanged) { if (_pfnFrameColorChanged) _pfnFrameColorChanged(); }`
  - **KHÔNG `_fAllInvalidated=true`**, **KHÔNG sticky state**, **KHÔNG `_lastFrameType |= Frame::Background`**, callback là function pointer fire-and-forget.
- `renderer/base/renderer.cpp:110-122` — `TriggerTeardown()`:
  - `if (lock _threadMutex; _thread)` { `_threadKeepRunning.store(false)`; `_shutdownEvent.SetEvent()`; `NotifyPaintFrame()`; `_enable.SetEvent()`; `WaitForSingleObject(_thread.get(), INFINITE)`; `_thread.reset()`; }
  - `_disablePainting()`
  - **KHÔNG boolean `_fTeardown` field** — lifecycle state là `_threadKeepRunning` (atomic bool) + `_thread` handle presence.
- `renderer/base/renderer.hpp:175-185` — actual state members:
  - `_pfnBackgroundColorChanged` (std::function<void()>), `_pfnFrameColorChanged`, `_pfnRendererEnteredErrorState` (callbacks)
  - `_isSynchronizingOutput` (bool), `_forceUpdateViewport` (bool)
  - `_lastSelectionPaintSpan`, `_lastSelectionPaintSize`, `_lastSelectionRectsByViewport` (selection, NOT invalidation)
- `renderer/base/renderer.hpp` (somewhere) — `_engines: std::vector<std::unique_ptr<RenderEngineBase>>`, `_threadKeepRunning: std::atomic<bool>`, `_thread`, `_shutdownEvent`, `_enable` (winrt event), `_threadMutex`.
- GdiEngine accumulation `_rcInvalid`/`_fInvalidRectUsed` (`renderer/gdi/gdirenderer.hpp:159-160`) — **out-of-scope ADR-0001**, không port.

**Re-expression Rust+GPUI** (ADR-0003 behavioral parity):
- `_engines: Vec<unique_ptr<RenderEngineBase>>` → `engines: Vec<Box<dyn RenderEngine>>` (loop-05 introduce trait `RenderEngine` minimal: `fn invalidate(&mut self, region: &Viewport)`, `fn invalidate_all(&mut self)`. GPUI impl loop sau, hiện `Vec` có thể empty hoặc trait method `default`).
- `_threadKeepRunning: atomic<bool>` → `thread_keep_running: AtomicBool` (re-expression; GPUI `cx.notify()` không cần thread join, nhưng port giữ state để mirror lifecycle observable).
- `_thread` handle presence → `thread_handle_active: bool` (Rust không có Windows HANDLE portable; re-expression cho `is_teardown_complete()` observable — true khi `!thread_handle_active && !thread_keep_running`).
- `_pfnBackgroundColorChanged`/`_pfnFrameColorChanged` → `on_background_color_changed: Option<Box<dyn FnMut()>>` / `on_frame_color_changed` (callback storage; `trigger_redraw_all(bg,frame)` gọi callback if Some).
- `NotifyPaintFrame()` → không port (GPUI `cx.notify()` ở loop sau wire; loop-05 chỉ track `paint_frame_requests: usize` counter cho observability, hoặc bỏ nếu quá speculative).
- `TriggerRedraw(Vp)` per-engine dispatch loop: `for engine in &mut self.engines { engine.invalidate(&region) }`.

## Tasks

### T1 — Rework `crates/renderer/src/renderer.rs` (REWRITE field/method, không add module mới)

**T1.1.** Xóa field fabricated:
- `invalid_rect: Option<InvalidRect>` → XÓA (GdiEngine pattern, out-of-scope).
- `invalid_rect_used: bool` → XÓA.
- `all_invalidated: bool` → XÓA (không có trong WT renderer/base).
- `teardown_requested: bool` → XÓA (WT không có field này).
- `background_changed: bool` / `frame_changed: bool` (sticky) → XÓA (WT không sticky, callback fire-and-forget).

**T1.2.** Thêm field port-faithful:
```rust
pub struct RendererState {
    /// WT `_threadKeepRunning` (renderer.hpp) — `true` while render thread active.
    thread_keep_running: AtomicBool,
    /// WT `_thread` handle presence (re-expression — true khi thread đang chạy).
    thread_handle_active: bool,
    /// WT `_engines: Vec<unique_ptr<RenderEngineBase>>` — per-engine dispatch list.
    /// Loop-05: `RenderEngine` trait minimal; GPUI impl loop sau; có thể empty Vec.
    engines: Vec<Box<dyn RenderEngine>>,
    /// WT `_pfnBackgroundColorChanged` (renderer.hpp:175) — Optional fire-and-forget callback.
    on_background_color_changed: Option<Box<dyn FnMut()>>,
    /// WT `_pfnFrameColorChanged` (renderer.hpp:176).
    on_frame_color_changed: Option<Box<dyn FnMut()>>,
}
```

**T1.3.** Define minimal `RenderEngine` trait (port WT `RenderEngineBase`/`IRenderEngine` API slice cần cho `TriggerRedraw/All`):
```rust
/// Port WT `RenderEngineBase`/`IRenderEngine` API slice cho `TriggerRedraw/All`
/// dispatch (renderer.cpp:656-722). Full trait port loop sau (P10 step 5+).
pub trait RenderEngine {
    /// WT `RenderEngineBase::Invalidate(&region)` — per-engine invalidation.
    fn invalidate(&mut self, region: &Viewport);
    /// WT `RenderEngineBase::InvalidateAll()` — full-frame invalidation.
    fn invalidate_all(&mut self);
}
```
Engine thực (D3D atlas gone ADR-0003; GPUI engine loop sau) — loop-05 không cần concreate impl, chỉ trait. **Test dùng stub engine in `#[cfg(test)]` (test-only, không production mock)**: struct `CountingEngine { invalidate_calls: Vec<Viewport>, invalidate_all_calls: usize }` impl `RenderEngine` track calls. (Note: test-only stub không violates "no mocks" rule vì nó verify port-faithful dispatch behavior, không fake để pass production test.)

**T1.4.** Implement port-faithful methods (giữ method names exist để minimize test churn, nhưng behavior đổi):
```rust
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

    /// Port `TriggerRedraw(Viewport)` (renderer.cpp:656-683).
    /// WT: per-engine `Invalidate(&region)` + `NotifyPaintFrame()`. KHÔNG bbox union.
    pub fn trigger_redraw(&mut self, region: &Viewport) {
        for engine in &mut self.engines {
            engine.invalidate(region);
        }
        // NotifyPaintFrame() — GPUI cx.notify() loop sau; loop-05 chỉ port dispatch.
    }

    /// Port `TriggerRedraw(point*)` (renderer.cpp:691).
    pub fn trigger_redraw_point(&mut self, point: (i32, i32)) {
        let region = Viewport::from_dimensions(point, (1, 1));
        self.trigger_redraw(&region);
    }

    /// Port `TriggerRedrawAll(bg,frame)` (renderer.cpp:704-722).
    /// WT: per-engine `InvalidateAll()` + fire callbacks if Some. KHÔNG sticky flags.
    pub fn trigger_redraw_all(&mut self, background_changed: bool, frame_changed: bool) {
        for engine in &mut self.engines {
            engine.invalidate_all();
        }
        if background_changed {
            if let Some(cb) = &mut self.on_background_color_changed { cb(); }
        }
        if frame_changed {
            if let Some(cb) = &mut self.on_frame_color_changed { cb(); }
        }
    }

    /// Port `TriggerTeardown()` (renderer.cpp:110-122).
    /// WT: `_threadKeepRunning.store(false)` + event signal + thread join. Loop-05 re-expression:
    /// set `thread_keep_running=false` + `thread_handle_active=false` (mirror lifecycle observable).
    pub fn trigger_teardown(&mut self) {
        self.thread_keep_running.store(false, Ordering::SeqCst);
        self.thread_handle_active = false;
        // WT `_shutdownEvent.SetEvent()` + `WaitForSingleObject(_thread)` + `_thread.reset()`
        // — thread join asynchronous, GPUI không cần; re-expression `thread_handle_active=false`.
    }

    /// Start render thread — WT re-expression (constructor-like, không có WT direct name;
    /// mirror `_thread = std::thread(...)` initialization trong Renderer ctor).
    pub fn start_thread(&mut self) {
        self.thread_keep_running.store(true, Ordering::SeqCst);
        self.thread_handle_active = true;
    }
}
```

**T1.5.** Implement getters (re-expression cho Rust testability, port observable boundary):
```rust
impl RendererState {
    /// WT `_threadKeepRunning.load()` — render thread keeps running.
    pub fn is_thread_keep_running(&self) -> bool {
        self.thread_keep_running.load(Ordering::SeqCst)
    }
    /// WT `_thread` presence — thread đang chạy (chưa teardown).
    pub fn is_thread_active(&self) -> bool { self.thread_handle_active }
    /// WT `_thread` reset sau teardown — teardown complete.
    pub fn is_teardown_complete(&self) -> bool {
        !self.thread_handle_active && !self.is_thread_keep_running()
    }
    /// WT `_engines` accessor (testabilidade).
    pub fn engines(&self) -> &[Box<dyn RenderEngine>] { &self.engines }
    pub fn engines_mut(&mut self) -> &mut [Box<dyn RenderEngine>] { &mut self.engines }
    pub fn push_engine(&mut self, engine: Box<dyn RenderEngine>) { self.engines.push(engine) }
    /// WT `_pfnBackgroundColorChanged` set callback.
    pub fn set_on_background_color_changed(&mut self, cb: Box<dyn FnMut()>) {
        self.on_background_color_changed = Some(cb);
    }
    pub fn set_on_frame_color_changed(&mut self, cb: Box<dyn FnMut()>) {
        self.on_frame_color_changed = Some(cb);
    }
}
```

**T1.6.** KHÔNG giữ `is_invalidated()`/`is_all_invalidated()`/`invalid_rect()`/`is_teardown_requested()`/`background_changed()`/`frame_changed()` (không có WT tương đương, đã remove field).

**T1.7.** `InvalidRect` struct: KEEP (re-export cho loop sau `RenderEngine` full port + scroll) — nhưng move note "port WT `_fInvalidRect`" → "port WT `til::rect` (TIL) re-expression; GdiEngine uses this for accumulation but `renderer/base` does not". Update doc-comment.

### T2 — Update `lib.rs` re-export

`pub use renderer::{Renderer, InvalidRect};` → đổi sang `pub use renderer::{RendererState, InvalidRect, RenderEngine};` (rename `Renderer` → `RendererState` từ loop-04; hoặc giữ tên `Renderer`? — Dev decide consistent với paint::Renderer conflict note loop-04. **Recommend keep `RendererState`** để tránh paint::Renderer conflict + судеб WT class name `Renderer` dù re-expression). Verify export không break `colorfix`/`settings`/`paint`/`grid` exports.

### T3 — Rewrite doc-comment `renderer.rs` (close L04-G03)

Replace fabricated cite (`_fInvalidRect renderer.hpp:181`, `_lastFrameType |= Frame::Background`, `_pfnFrameCompiledCallback`) bằng port-faithful cite:
```rust
//! `RendererState` — port WT `renderer/base/renderer.cpp:110,656-722` `TriggerRedraw/All/Teardown`
//! + `renderer.hpp:175-185` state members (`_threadKeepRunning`/`_pfn*ColorChanged`/`_engines`).
//! Loop 05 rework: port-faithful dispatch (per-engine Invalidate/InvalidateAll + thread lifecycle),
//! KHÔNG GdiEngine accumulation pattern (`_rcInvalid`/`_fInvalidRectUsed` — `renderer/gdi/`, out-of-scope
//! ADR-0001). `_lastFrameType`/`_pfnFrameCompiledCallback` không tồn tại trong WT renderer/base.
//! GPUI owns scene (ADR-0003); `NotifyPaintFrame()` wire sang `cx.notify()` loop sau.
```

### T4 — Rewrite 6 unit tests `crates/renderer/src/renderer.rs #[cfg(test)]`

Replace 6 test loop-04 (verify fabricated bbox union behavior) bằng 6 test verify port-faithful WT behavior (black-box qua public API + test-only `CountingEngine`):

- `test_trigger_redraw_dispatches_invalidate_to_each_engine` — 2 `CountingEngine` trong `engines`, `trigger_redraw(&vp)` → mỗi engine `invalidate_calls` == `vec![vp.clone()]`. Verify per-engine dispatch (mirror WT cpp:664-668).
- `test_trigger_redraw_point_one_by_one_delegates` — `trigger_redraw_point((5,7))` → engine `invalidate_calls` == `[Viewport{left5,top7,right6,bottom8}]`. Verify `TriggerRedraw(point*)` delegate (cpp:691).
- `test_trigger_redraw_all_dispatches_invalidate_all` — 2 engine, `trigger_redraw_all(false,false) → mỗi engine `invalidate_all_calls == 1`, không gọi `invalidate`. Verify `TriggerRedrawAll` per-engine (cpp:710-712).
- `test_trigger_redraw_all_fires_background_frame_callbacks` — set 2 callback (counter capture), `trigger_redraw_all(true,true)` → both callback fired once. `trigger_redraw_all(false,false)` → không fired (fire-and-forget, không sticky). Verify WT `_pfnBackgroundColorChanged()`/`_pfnFrameColorChanged()` (cpp:715,720).
- `test_trigger_teardown_sets_thread_keep_running_false` — `start_thread()` → `is_thread_keep_running()==true`, `is_thread_active()==true`. `trigger_teardown()` → `is_thread_keep_running()==false`, `is_thread_active()==false`, `is_teardown_complete()==true`. Verify WT cpp:113 `_threadKeepRunning.store(false)` + thread reset.
- `test_trigger_redraw_no_bbox_accumulation` — engine `invalidate_calls` track; 2 `trigger_redraw` disjoint regions → engine `invalidate_calls.len() == 2` (each dispatch riêng, KHÔNG accumulate bbox union). Verify negative — loại bỏ loop-04 fabricated behavior. (Mirror WT `TriggerRedraw` không có `_fInvalidRect` accumulator.)

**Note**: `CountingEngine` test-only struct inside `#[cfg(test)] mod tests`, không production. `Viewport` cần `Clone` + `PartialEq` (verify in `crates/core/src/types` — if `Viewport` không derive, test dùng manual compare field-by-field).

### T5 — Update ledger P10 `parity/ledger/P10-renderer/{files,symbols}.csv`

`files.csv` row f00979/f00980: update `notes` từ "Renderer skeleton (loop 04)..." → "RendererState port-faithful WT renderer/base (loop 05): per-engine Invalidate/InvalidateAll dispatch + thread lifecycle. Rework loop-04 GdiEngine accumulation drift (L04-G01 closed).". `test_ids` update sang 6 new test names.

`symbols.csv` row s010-s013: update `test_ids` sang 6 new test names + `notes` reflect port-faithful. Add s014-s016:
- `s014,Renderer::TriggerTeardown,renderer.cpp:110,fn,crates/renderer/src/renderer.rs,partial,test_trigger_teardown_sets_thread_keep_running_false,ADR-0003,thread lifecycle store false + handle reset re-expression`
- `s015,Renderer::_threadKeepRunning,renderer.hpp:192,field,crates/renderer/src/renderer.rs,partial,test_trigger_teardown_sets_thread_keep_running_false,ADR-0003,atomic bool thread keep-running flag` (verify hpp line actual)
- `s016,RenderEngine::Invalidate,RenderEngineBase.hpp:NN,fn,crates/renderer/src/renderer.rs (trait),partial,test_trigger_redraw_dispatches_invalidate_to_each_engine,ADR-0003,per-engine invalidation dispatch (renderer/base loop over _engines)`

Verify `wt_anchor` line numbers actual bằng `sed -n` before write.

### T6 — ADR-0016 (optional, Dev decide)

Skip ADR-0016 — loop-05 không NEW architectural decision, chỉ **fix port-faithfulness drift** loop-04. ADR-0003 đã cover "GPUI owns scene, no D3D engine backend". Rework ghi trong notes + commit msg. Nếu Dev thấy cần ADR explicit "renderer/base port-faithful rework (L04-G01 close)": skip — rework không phải decision mới, là correction. **Recommend: skip ADR-0016 loop này**.

## Preservation

- **P1 — Loop 04 workspace regression**: `cargo test --workspace --lib` → 486 pass / 0 fail. Loop-05 chỉ touch renderer crate (renderer.rs rework + lib.rs + tests within renderer.rs). T4 rewrite 6 test trong `renderer::tests` (cùng count), 0 net change test count → target 486 pass / 0 fail.
- **P2 — Loop 04 colorfix unit intact**: `cargo test -p renderer --lib colorfix` → 8 pass. Dev không touch `colorfix.rs`.
- **P3 — Loop 04 settings 18 intact**: `cargo test -p renderer --lib settings::tests` → 18 pass. Dev không touch `settings.rs`.
- **P4 — Loop 04 color table 256 default**: `RenderSettings::default()` untouched. Dev không touch settings.rs.
- **P5 — Loop 04 terminal seam intact**: `cargo test -p core --test wt_terminalcore` → 1 pass. Dev không touch `crates/core/`.
- **P6 — Loop 04 wt_colorfix integration intact**: `cargo test -p renderer --test wt_renderer_colorfix` → 3 pass. Dev không touch test file (T1 chỉ touch `wt_renderer_colorfix.rs`? NO — KHÔNG touch test file loop-05, all changes in `renderer.rs`). Verify `wt_renderer_colorfix.rs:11` `#![allow(non_snake_case)]` (L03-G01 closed loop-04) preserved.
- **P7 — Loop 04 clippy lib renderer clean**: `cargo clippy -p renderer --all-targets -- -D warnings` clean. Loop-05 rework phải clippy-clean (no new warning).

## Validation Requirements

**VR1 — T1.4 TriggerRedraw per-engine dispatch**:
- **Given** `RendererState::new()` + 2 `CountingEngine` pushed via `push_engine`.
- **When** `trigger_redraw(&Viewport{left:5,top:7,right:20,bottom:15})`.
- **Then** mỗi engine `invalidate_calls == vec![Viewport{5,7,20,15}]`, `invalidate_all_calls == 0`.

**VR2 — T1.4 TriggerRedrawPoint delegate**:
- **Given** RendererState + 1 CountingEngine.
- **When** `trigger_redraw_point((5,7))`.
- **Then** engine `invalidate_calls == [Viewport{left:5,top:7,right:6,bottom8}]` (1×1 exclusive).

**VR3 — T1.4 TriggerRedrawAll per-engine + callbacks**:
- **Given** RendererState + 2 CountingEngine + 2 counter callback set.
- **When** `trigger_redraw_all(true,true)`.
- **Then** mỗi engine `invalidate_all_calls == 1`, `invalidate_calls == []` (no per-region invalidate). Both callbacks fired once. Sau `trigger_redraw_all(false,false)` → callback counters unchanged (no additional fire).

**VR4 — T1.4 TriggerTeardown thread lifecycle**:
- **Given** `RendererState::new()`. `start_thread()` → `is_thread_keep_running()==true`, `is_thread_active()==true`, `is_teardown_complete()==false`.
- **When** `trigger_teardown()`.
- **Then** `is_thread_keep_running()==false`, `is_thread_active()==false`, `is_teardown_complete()==true`.

**VR5 — T1.4 No bbox accumulation (drift closed)**:
- **Given** RendererState + 1 CountingEngine.
- **When** 2 `trigger_redraw` disjoint regions `(5,7,10,12)` then `(20,3,25,8)`.
- **Then** engine `invalidate_calls.len() == 2` (2 dispatch riêng, không accumulate). `invalid_rect` accessor không tồn tại (compile-fail if used) — verify field removed.

**VR6 — T2 module exposed**:
- `crates/renderer/src/lib.rs` có `pub use renderer::{RendererState, InvalidRect, RenderEngine}` (grep verify). `Renderer` (old name) không còn re-export (rename to `RendererState`).

**VR7 — T3 doc-comment cite port-faithful**:
- `crates/renderer/src/renderer.rs` doc-comment không chứa `_fInvalidRect`/`_lastFrameType`/`_pfnFrameCompiledCallback` (grep zero hit). Chứa `renderer/base/renderer.cpp:110,656-722` + `renderer.hpp:175-185` + `RenderEngineBase` + `_threadKeepRunning`/`_pfnBackgroundColorChanged` (grep confirm).

**VR8 — T4 6 tests green**: `cargo test -p renderer --lib renderer::tests` → 6 pass / 0 fail.

**VR9 — T5 ledger P10 updated**: `files.csv` f00979/f00980 `notes` mention "port-faithful" + "L04-G01 closed"; `symbols.csv` s010-s013 `test_ids` new + s014-s016 added.

**VR10 — Build + workspace + clippy green**:
- `cargo check -p renderer` → 0 error.
- `cargo test -p renderer --lib` → 64 pass / 0 fail (58 + 6 T4 rework; same count).
- `cargo test --workspace --lib` → 486 pass / 0 fail (P1).
- `cargo test -p core --test wt_terminalcore` → 1 pass (P5).
- `cargo test -p renderer --test wt_renderer_colorfix` → 3 pass (P6).
- `cargo clippy -p renderer --all-targets -- -D warnings` → 0 warning 0 error (P7).

## Out-of-scope

- **GPUI `cx.notify()` wire** (replace `NotifyPaintFrame()`) — loop sau khi scene/paint port.
- **`RenderEngine` concreate impl (GPUI engine)** — loop sau (P10 step 5+ ADR-0003).
- **Full `RenderEngineBase`/`IRenderEngine` trait port** (Paint/PaintFrame/StartPaint/EndPaint ~50 method) — loop sau, loop-05 chỉ slice 2 method cần cho `TriggerRedraw/All` dispatch.
- **Render thread actual spawn** (winrt `_thread = std::thread(...)`) — GPUI owns render loop, re-expression `start_thread()` flag only.
- **`NotifyPaintFrame()` counter/observability** — drop loop-05 (speculative); wire ở paint driver loop sau.
- **Scroll/cursor/selection/paint driver** — same as loop-04 out-of-scope.
- **`crates/core/`, `crates/font/`, `crates/connection/`, `crates/control/`** — isolation.
- **`apps/terminal`, `windows/`, `zed_gpui/`** — vendored.
- **L04-G02 (D_4 citation)** — closed by THIS D_5 (Planner cita đúng this loop; no Developer action).
- **L04-G03 (Dev doc-comment)** — closed by T3 (Developer rewrites doc-comment).
- **L04-G04 (pre-existing control hygiene)** — out-of-scope P10 renderer; P11 or `control-hygiene` loop.
- **ADR-0016** — skip (recommend).

## Issue-ledger closure plan

- **L04-G01** → closed when VR1+VR3+VR4+VR5 green (port-faithful dispatch + no accumulator + thread lifecycle).
- **L04-G02** → closed by D_5 itself (this doc cita đúng WT origin — `renderer.hpp:175-185` actual `_pfn*ColorChanged`, not `_fInvalidRect`).
- **L04-G03** → closed when VR7 green (doc-comment cite port-faithful).
- **L04-G04** → stays open (pre-existing, out-of-scope renderer loop).
