---
loop: 4
status: pending
preservation_constraints:
  - p1_loop03_workspace_regression   # cargo test --workspace --lib không regress (loop03 baseline 480 pass / 0 fail)
  - p2_loop03_colorfix_unit_intact   # renderer --lib colorfix 8 test still green
  - p3_loop03_settings18_intact      # 13 existing + 5 T5 behavioral settings test verbatim pass (18 total)
  - p4_loop03_colortable256_default  # RenderSettings::default() 256-table Campbell unchanged
  - p5_loop03_terminal_seam_intact   # cargo test -p core --test wt_terminalcore 1 pass
  - p6_loop03_wt_colorfix_integration_intact  # cargo test -p renderer --test wt_renderer_colorfix 3 pass (T6 logic)
  - p7_loop03_clippy_lib_clean       # cargo clippy -p renderer -- -D warnings (default lib target) clean
candidate: A_4 (crates/renderer/tests/wt_renderer_colorfix.rs fix + crates/renderer/src/renderer.rs NEW + lib.rs expose + ledger P10 update + ADR optional)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 04 — Fix L03-G01 (clippy allow) + Renderer invalidation/teardown skeleton

## Objective

**Fix T6 lint gap L03-G01 (1-line `#![allow(non_snake_case)]`) + port `Renderer` invalidation/teardown state skeleton (`crates/renderer/src/renderer.rs` NEW) — observable qua trigger/invalidate unit test, chưa wire paint/scene.**

Một câu: Sau `trigger_redraw(_all|point|region)` / `trigger_teardown`, `Renderer` track invalidation flag observable; tack-on fix loop-03 lint gap.

## Context

Loop 03 (E_3) closed L02-G01..G05 nhưng mở **L03-G01**: `crates/renderer/tests/wt_renderer_colorfix.rs` thiếu `#![allow(non_snake_case)]` (loop-01 `wt_terminalcore.rs:14` có) → `cargo clippy -p renderer --all-targets -- -D warnings` fail 4 error `non_snake_case` cho 4 fn `wt_*__*`. Test logic 3/3 green, build green. Gap low severity nhưng break `clippy -D warnings` success criterion.

P10 step 2 (phase-10 doc): port `Renderer` invalidation/scroll/cursor logic. WT `renderer/base/renderer.cpp` (1954 line) + `renderer.hpp` (185 line). Đây là lớp lớn — loop 04 không port hết. **Bounded slice**: chỉ phần invalidation/teardown state (line 656-716, 110) — `TriggerRedraw(Viewport)` / `TriggerRedraw(point)` / `TriggerRedrawAll(bgChanged, frameChanged)` / `TriggerTeardown` + invalidation tracking. KHÔNG paint driver (`Paint`/`PaintFrame`/`_PaintOutput*/_PaintCursor*` ~800 line), KHÔNG scroll (`InvalidateScroll`/`NotifyScroll` ~150 line, loop sau), KHÔNG selection/highlight (`InvalidateSelection`/`InvalidateHighlight` loop sau), KHÔNG cursor blink tick (`_blinkTimer`/`_blinkIsInUse` loop sau).

**WT ground (verify read-only)**:
- `renderer/base/renderer.cpp:656-716` — 4 trigger methods:
  - `TriggerRedraw(Viewport region)` (656): `_fInvalidRect` union region, `_fInvalidRectUsed=true`, `_fAllInvalidated=false`, `_pEngine->Invalidate(&region)`.
  - `TriggerRedraw(point*)` (691): delegate `TriggerRedraw(Viewport::FromDimensions(*pcoord, {1,1}))`.
  - `TriggerRedrawAll(bgChanged=false, frameChanged=false)` (704): `_fAllInvalidated=true`, `_pEngine->InvalidateAll()`, nếu `bgChanged` set `_lastFrameType|=Frame::Background`, nếu `frameChanged` set `_pfnFrameCompiledCallback`.
  - `TriggerTeardown()` (110): `_fTeardown=true`.
- `renderer.hpp:39-42, 181-185` — state members: `_fInvalidRectUsed`, `_fAllInvalidated`, `_fTeardown`, `_fInvalidRect`.

**Re-expression trên Rust+GPUI** (ADR-0003 behavioral parity): GPUI owns swapchain/scene → não `_pEngine->Invalidate*` (D3D atlas backend, gone). State tracking (`_fInvalid*`, `_fAllInvalidated`, `_fTeardown`) là observable boundary cho downstream paint scheduler (GPUI `cx.notify()` ở loop sau). Loop 04 chỉ port **state struct + trigger methods (mutate state, không gọi engine)**. Khi need invalidate criterion observable: `is_invalidated()` / `invalid_rect()` / `is_all_invalidated()` / `is_teardown_requested()` public getters.

**Rust hiện trạng** (`crates/renderer/src/`):
- `lib.rs` (loop02 `pub mod colorfix;` line 33, `pub use colorfix::{...}` line 46). Cần thêm `pub mod renderer;` + re-export `Renderer`.
- `settings.rs`, `colorfix.rs`, `paint.rs`, `grid.rs`, `cell.rs`, `payload.rs` đã có (loop02 32 test paint/payload/cell/grid).
- `renderer.rs` (src/) không tồn tại — NEW.

**Reuse**: KHÔNG recreate colorfix/settings. `Renderer` struct mới only. Đọc `crates/renderer/src/lib.rs` confirm module layout trước khi add.

## Tasks

### T1 — Fix L03-G01: `crates/renderer/tests/wt_renderer_colorfix.rs` line 1

Thêm `#![allow(non_snake_case)]` ở line 1 (trước module doc-comment hoặc ngay sau). Mirror `crates/core/tests/wt_terminalcore.rs:14` (`#![allow(non_snake_case)]` sau 13-line module doc). Vị trí: sau module-level `//!` doc nhưng trước `use` items (inner attribute phải ở top of file sau doc).

Verify: `cargo clippy -p renderer --all-targets -- -D warnings` clean (0 error `non_snake_case`).

### T2 — Port `Renderer` invalidation/teardown skeleton: `crates/renderer/src/renderer.rs` (NEW)

T2.1. Define `pub struct Renderer` với state (mirror WT `renderer.hpp:181-185` + `renderer.cpp:656-716`):

```rust
//! `Renderer` — port WT `renderer/base/renderer.cpp` invalidation/teardown state.
//! Loop 04 skeleton: trigger methods + state getters, KHÔNG paint driver, KHÔNG engine
//! backend (ADR-0003 — GPUI owns scene). Loop sau wire `cx.notify()` + scroll/cursor/paint.

use core::{TextAttribute, types::Viewport};  // verify crate `types` path

/// Invalidation region — port WT `_fInvalidRect` (til::rect).
/// `None` = chưa có invalid region (state `_fInvalidRectUsed=false`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InvalidRect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

/// `Renderer` skeleton — port WT `renderer/base/renderer.cpp:110,656-716`.
/// State observable qua getters; trigger methods mutate state, không gọi engine backend
/// (ADR-0003 behavioral parity, GPUI owns scene/paint scheduler ở loop sau).
#[derive(Clone, Debug, Default)]
pub struct Renderer {
    /// `_fInvalidRectUsed` (renderer.hpp:181) — true nếu có invalid rect accumulated.
    invalid_rect_used: bool,
    /// `_fInvalidRect` — accumulated invalid region (None when !used).
    invalid_rect: Option<InvalidRect>,
    /// `_fAllInvalidated` (renderer.hpp:181) — true sau TriggerRedrawAll.
    all_invalidated: bool,
    /// `_fTeardown` (renderer.cpp:110) — true sau TriggerTeardown.
    teardown_requested: bool,
    /// `_lastFrameType |= Frame::Background` — port `TriggerRedrawAll(bgChanged=true)`.
    background_changed: bool,
    /// `_pfnFrameCompiledCallback` trigger — port `TriggerRedrawAll(frameChanged=true)`.
    frame_changed: bool,
}
```

T2.2. Implement trigger methods (port `renderer.cpp:656-716`):

```rust
impl Renderer {
    pub fn new() -> Self { Self::default() }

    /// Port `TriggerRedraw(Viewport region)` (renderer.cpp:656). Union region vào
    /// `_fInvalidRect`, set `_fInvalidRectUsed=true`, clear `_fAllInvalidated`.
    pub fn trigger_redraw(&mut self, region: &Viewport) {
        // Union bbox: if !used → set region; else bbox union.
        let r = InvalidRect {
            left: region.left, top: region.top,
            right: region.right, bottom: region.bottom,
        };  // verify Viewport field names
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
        self.all_invalidated = false;
        // WT also calls `_pEngine->Invalidate(&region)` — skipped (no engine; GPUI scene).
    }

    /// Port `TriggerRedraw(point*)` (renderer.cpp:691) — delegate `trigger_redraw`
    /// với Viewport 1×1 tại point.
    pub fn trigger_redraw_point(&mut self, point: (i32, i32)) {
        let region = Viewport::from_dimensions(point, (1, 1));  // verify API
        self.trigger_redraw(&region);
    }

    /// Port `TriggerRedrawAll(bgChanged, frameChanged)` (renderer.cpp:704).
    /// `_fAllInvalidated=true`; clear `_fInvalidRect*` (all này superset). Set bg/frame flags.
    pub fn trigger_redraw_all(&mut self, background_changed: bool, frame_changed: bool) {
        self.all_invalidated = true;
        self.invalid_rect = None;
        self.invalid_rect_used = false;
        if background_changed { self.background_changed = true; }
        if frame_changed { self.frame_changed = true; }
        // WT `_pEngine->InvalidateAll()` — skipped (no engine).
    }

    /// Port `TriggerTeardown()` (renderer.cpp:110). `_fTeardown=true`.
    pub fn trigger_teardown(&mut self) {
        self.teardown_requested = true;
    }
}
```

T2.3. Implement public getters (observable boundary, không có WT tương đương trực tiếp — re-expression cho Rust testability):

```rust
impl Renderer {
    /// Invalidation pending (rect used OR all invalidated).
    pub fn is_invalidated(&self) -> bool {
        self.invalid_rect_used || self.all_invalidated
    }
    pub fn is_all_invalidated(&self) -> bool { self.all_invalidated }
    pub fn invalid_rect(&self) -> Option<&InvalidRect> { self.invalid_rect.as_ref() }
    pub fn is_teardown_requested(&self) -> bool { self.teardown_requested }
    pub fn background_changed(&self) -> bool { self.background_changed }
    pub fn frame_changed(&self) -> bool { self.frame_changed }
}
```

T2.4. **KHÔNG port `_pEngine->Invalidate*` calls** — D3D atlas backend replaced by GPUI (ADR-0003). Note trong module doc. **KHÔNG port `NotifyScroll`/`InvalidateScroll`** (loop sau). **KHÔNG port `Paint`/`PaintFrame`/cursor/selection** (loop sau). **KHÔNG touch `crates/core/`** (P10 isolation — `Renderer` đọc terminal state qua `IRenderData`/`RenderState` traits frozen P01; loop 04 không wire data, chỉ skeleton).

### T3 — Expose module `renderer`: `crates/renderer/src/lib.rs`

Thêm `pub mod renderer;` + `pub use renderer::{Renderer, InvalidRect};` (verify line ~33/46 area, không break `colorfix` export). Nếu lib.rs đã có `renderer/mod` typo → fix. **KHÔNG modify** colorfix/settings/paint/grid/cell/payload exports.

### T4 — Behavioral unit tests `crates/renderer/src/renderer.rs` `#[cfg(test)] mod tests`

Append 6 test (mirror WT `renderer.cpp` trigger behavior, black-box qua public API):

- `test_trigger_redraw_sets_invalid_rect_and_used` — `trigger_redraw(viewport)` → `is_invalidated()==true`, `invalid_rect().is_some()`, `is_all_invalidated()==false`.
- `test_trigger_redraw_point_one_by_one` — `trigger_redraw_point((5,7))` → invalid_rect == Some({left5,top7,right6,bottom8}) (Viewport 1×1).
- `test_trigger_redraw_accumulates_bbox_union` — 2 trigger disjoint rects → `invalid_rect` = bbox union (min left/top, max right/bottom).
- `test_trigger_redraw_all_clears_rect_sets_all_invalidated` — `trigger_redraw_all(false,false)` → `is_all_invalidated()==true`, `invalid_rect().is_none()`, `is_invalidated()==true`. Sau `trigger_redraw(region)` → `all_invalidated` cleared (mirror WT line 708 logic: `_fAllInvalidated=false` set trong `TriggerRedraw`).
- `test_trigger_redraw_all_records_bg_frame_flags` — `trigger_redraw_all(true,true)` → `background_changed()==true`, `frame_changed()==true`. Sau `trigger_redraw_all(false,false)` không clear (sticky flags, mirror WT `_lastFrameType |=`).
- `test_trigger_teardown_sets_flag` — `trigger_teardown()` → `is_teardown_requested()==true`, `is_invalidated()==false` (teardown không invalidate, mirror WT line 110).

KHÔNG dùng `mock`/fake engine — state observable qua getters (re-expression). Tests realWT semantics verbatim.

### T5 — Ledger P10 update `parity/ledger/P10-renderer/{files,symbols}.csv`

`files.csv`:
- Row `f00979`(`renderer/base/renderer.cpp`) → `rust_path=crates/renderer/src/renderer.rs`, `status=partial`, `test_ids=test_trigger_redraw_sets_invalid_rect_and_used;test_trigger_redraw_all_clears_rect_sets_all_invalidated;test_trigger_teardown_sets_flag`, `adr=ADR-0003`, `author=P10-worker`, `notes=Renderer skeleton (loop 04): trigger/invalidate/teardown state only, no paint driver`.
- Row `f00980`(`renderer/base/renderer.hpp`) → `status=partial`, `rust_path=crates/renderer/src/renderer.rs`, `notes=Renderer struct state members (loop 04)`, otros như f00979.

`symbols.csv` append (after s009):
- `s010,Renderer::TriggerRedraw,renderer.cpp:656,fn,crates/renderer/src/renderer.rs,partial,test_trigger_redraw_sets_invalid_rect_and_used,ADR-0003,viewport invalidation bbox union`
- `s011,Renderer::TriggerRedrawAll,renderer.cpp:704,fn,crates/renderer/src/renderer.rs,partial,test_trigger_redraw_all_clears_rect_sets_all_invalidated,ADR-0003,all-invalidated + bg/frame flags`
- `s012,Renderer::TriggerTeardown,renderer.cpp:110,fn,crates/renderer/src/renderer.rs,partial,test_trigger_teardown_sets_flag,ADR-0003,teardown state flag`
- `s013,Renderer::_fInvalidRect,renderer.hpp:181,field,crates/renderer/src/renderer.rs,partial,test_trigger_redraw_accumulates_bbox_union,ADR-0003,accumulated invalid bbox`

### T6 — ADR-0016 optional

Quyết định: **skip ADR-0016** loop này. ADR-0003 đã cover "Atlas D3D backend replaced by GPUI". ADR-0015 cover colorfix. Renderer skeleton không có NEW architectural decision chưa record. Nếu cần: ADR-0016 "Renderer state-only port (no engine backend)" — nhưng trivial (extend ADR-0003 consequence). Ghi release trong notes/symbols notes, không ADR riêng. Out-of-scope.

## Preservation

- **P1 — Loop 03 workspace regression**: `cargo test --workspace --lib` → loop-03 baseline 480 pass / 0 fail. Loop 04 chỉ touch renderer (tests/wt_renderer_colorfix.rs fix + src/renderer.rs NEW + lib.rs expose). Target 480 + 6 (T4) = 486 pass / 0 fail.
- **P2 — Loop 03 colorfix unit intact**: `cargo test -p renderer --lib colorfix` → 8 test still green. T2/T3 không modify `colorfix.rs`.
- **P3 — Loop 03 settings 18 existing intact**: `cargo test -p renderer --lib settings::tests` → 18 pass verbatim. T2/T3 không modify `settings.rs`.
- **P4 — Loop 03 color table 256 default**: `RenderSettings::default()` unchanged. T2/T3 không modify `default()`.
- **P5 — Loop 03 terminal seam intact**: `cargo test -p core --test wt_terminalcore` → 1 pass. Loop 04 không touch `crates/core/`.
- **P6 — Loop 03 wt_colorfix integration intact**: `cargo test -p renderer --test wt_renderer_colorfix` → 3 pass. T1 chỉ thêm allow attr, không sửa logic test.
- **P7 — Loop 03 clippy lib clean**: `cargo clippy -p renderer -- -D warnings` (default lib) still clean; PLUS loop-04 must pass `cargo clippy -p renderer --all-targets -- -D warnings` (T1 fix L03-G01).

## Validation Requirements

**VR1 — T1 L03-G01 fix**:
- **Given** `crates/renderer/tests/wt_renderer_colorfix.rs` có `#![allow(non_snake_case)]` line ~1.
- **When** `cargo clippy -p renderer --all-targets -- -D warnings`.
- **Then** 0 error `non_snake_case`, **0 warning** total, exit 0. (mirror `cargo clippy -p core --all-targets -- -D warnings` clean baseline.)

**VR2 — T2 trigger_redraw(Viewport)**:
- **Given** `Renderer::new()` (clean state, `is_invalidated()==false`).
- **When** `trigger_redraw(&Viewport{left:5,top:7,right:20,bottom:15})`.
- **Then** `is_invalidated()==true`, `invalid_rect()==Some({5,7,20,15})`, `is_all_invalidated()==false`, `is_teardown_requested()==false`.

**VR3 — T2 trigger_redraw_point**:
- **Given** `Renderer::new()`.
- **When** `trigger_redraw_point((5,7))`.
- **Then** `invalid_rect()==Some({left5, top7, right6, bottom8})` (Viewport 1×1 — right=left+1, bottom=top+1).

**VR4 — T2 bbox accumulation**:
- **Given** `Renderer::new()`. Trigger red `(left5,top7,right10,bottom12)` then `(left20,top3,right25,bottom8)`.
- **When** observe `invalid_rect()`.
- **Then** == `Some({left5, top3, right25, bottom12})` (min left/top, max right/bottom).

**VR5 — T2 trigger_redraw_all + clear interaction**:
- **Given** `Renderer::new()`.
- **When** `trigger_redraw_all(true,true)` then `trigger_redraw(&vp)`.
- **Then** sau redraw_all: `is_all_invalidated()==true`, `background_changed()==true`, `frame_changed()==true`, `invalid_rect().is_none()`. Sau trigger_redraw: `is_all_invalidated()==false` (cleared), `is_invalidated()==true` (rect used lại).

**VR6 — T2 trigger_teardown**:
- **Given** `Renderer::new()`.
- **When** `trigger_teardown()`.
- **Then** `is_teardown_requested()==true`, `is_invalidated()==false` (teardown không invalidate), `invalid_rect().is_none()`.

**VR7 — T3 module exposed**:
- `crates/renderer/src/lib.rs` có `pub mod renderer;` + `pub use renderer::{Renderer, InvalidRect};` (grep verify).

**VR8 — T4 behavioral 6 tests green**: `cargo test -p renderer --lib renderer::tests` → 6 pass / 0 fail.

**VR9 — T5 ledger P10 updated**: `files.csv` row f00979 + f00980 status `partial`, rust_path populated, test_ids + adr populated; `symbols.csv` append 4 row s010-s013.

**VR10 — Build + workspace + clippy green**:
- `cargo check -p renderer` → 0 error.
- `cargo test -p renderer --lib` → 64 pass / 0 fail (58 + 6 T4).
- `cargo test --workspace --lib` → 486 pass / 0 fail (P1).
- `cargo test -p core --test wt_terminalcore` → 1 pass (P5).
- `cargo test -p renderer --test wt_renderer_colorfix` → 3 pass (P6).
- `cargo clippy -p renderer --all-targets -- -D warnings` → 0 error 0 warning (VR1).

## Out-of-scope

- **Renderer paint driver** (`Paint`/`PaintFrame`/`_PaintOutput`/`_PaintCursor` ~800 line) — loop sau.
- **Scroll invalidation** (`NotifyScroll`/`InvalidateScroll`, `renderer.cpp:793-820,879-895` ~150 line) — loop sau.
- **Selection/highlight invalidation** (`InvalidateSelection`, `InvalidateHighlight`, `renderer.cpp:740-790`) — loop sau.
- **Cursor blink** (`_blinkTimer`/`_blinkIsInUse`/`_PaintCursor`, `renderer.cpp:1650-1680`) — loop sau.
- **Title invalidation** (`InvalidateTitle`, `renderer.cpp:903-920`) — loop sau.
- **Engine backend wiring** (`_pEngine` interface, D3D atlas gone — ADR-0003) — loop sau, GPUI `cx.notify()` integration.
- **`IRenderData`/`RenderState` wire vào Renderer** (read terminal state) — loop sau, khi paint driver port.
- **`RenderEngine` trait + `GpuiEngine`** — P10 step 5+ (ADR-0003).
- **Font stack** (`FontInfo`/`FontConfig` cell metrics) — P10 step 3.
- **ADR-0016** — skip (ADR-0003 extension trivial, ghi notes).
- **Perf benchmarking + golden snapshot** — P10 step 8.
- **`crates/core/`, `crates/font/`** — isolation.
- **`apps/terminal`, `windows/`, `zed_gpui/`** — vendored.
