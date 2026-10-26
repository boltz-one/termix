---
loop: 6
status: pending
preservation_constraints:
  - p1_loop05_workspace_regression         # cargo test --workspace --lib ≥485 pass / 0 fail (loop05 485 base)
  - p2_loop05_colorfix_unit_intact          # renderer --lib colorfix 8 pass
  - p3_loop05_settings18_intact            # renderer --lib settings::tests 18 pass (13+5)
  - p4_loop05_colortable256_default         # RenderSettings::default() 256-table unchanged
  - p5_loop05_terminal_seam_intact          # cargo test -p core --test wt_terminalcore 1 pass
  - p6_loop05_wt_colorfix_integration_intact # cargo test -p renderer --test wt_renderer_colorfix 3 pass
  - p7_loop05_renderer_tests5_intact        # renderer::tests 5 pass (VR1-VR5 loop05 consolidated, không regress)
  - p8_loop05_clippy_lib_renderer_clean     # cargo clippy -p renderer --all-targets -- -D warnings clean
  - p9_loop05_no_bbox_accumulation          # trigger_redraw dispatch per-engine, không bbox union (L04-G01 closed, giữ)
candidate: A_6 (crates/renderer/src/renderer.rs add trigger_scroll + ScrollPoint state + RenderEngine::invalidate_scroll trait method; ledger P10 add symbols s015/s016/s017/s018; tests add 3-new)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 06 — Port `Renderer::TriggerScroll(delta)` + `_ScrollPreviousSelection` + close L05-G01

## Objective

Port `TriggerScroll(til::point)` (1 sentence): WT `renderer/base/renderer.cpp:891-900` per-engine `InvalidateScroll(delta)` + `_ScrollPreviousSelection(delta)` (cpp:1870) state shift — slice port-faithful nhỏ, locally-complete, KHÔNG wire GPUI/GUI/textbuffer (debtor IRenderData defer P09).

## Context (citations verified read-only `windows/src/renderer/`)

- **`TriggerScroll(const til::point* pcoordDelta)`** (`renderer.cpp:891-900`): `for pEngine in _engines: pEngine->InvalidateScroll(pcoordDelta)` → `_ScrollPreviousSelection(*pcoordDelta)` → `NotifyPaintFrame()`. Per-engine dispatch + state shift, fire-and-forget paint notify.
- **`_ScrollPreviousSelection(const til::point delta)`** (`renderer.cpp:1870-1879`): `if delta != {0,0}: for rc in _lastSelectionRectsByViewport: rc += delta; _currentCursorOptions.coordCursor += delta`. Shift 2 state vector/field by delta; no-op khi `delta == {0,0}`.
- **`IRenderEngine::InvalidateScroll(const til::point*)`** (`renderer/inc/IRenderEngine.hpp:73`, abstract virtual): port slice → `RenderEngine::invalidate_scroll(&self, delta: (i32,i32))` trait method.
- **State** (`renderer.hpp`): `_currentCursorOptions` (`:162`, `CursorOptions{}`), `_lastSelectionRectsByViewport` (`:183`, `vector<til::rect>{}`), `_viewport` (`:172`). Loop 06 init 2 field default empty (cursor + selection chưa wired — defer P11/P15). `_ScrollPreviousSelection` chỉ shift khi selection vector non-empty + delta != 0.
- **`TriggerScroll()` no-arg** (`renderer.cpp:874-878`): `_CheckViewportAndScroll()` + `NotifyPaintFrame()` — needs `_viewport` check + `_CheckViewportAndScroll` (cpp:803) phức tạp, phụ thuộc `_pData->GetViewport()` (IRenderData/P09). **OUT-OF-SCOPE loop 06**, defer to viewport-driver loop.
- **NOT wire**: `NotifyPaintFrame()` → `cx.notify()` (GPUI paint loop) defer ADR-0003; `IRenderData`/`IRenderEngine` full port defer; cursor/selection engine trait methods `InvalidateSelection`/`InvalidateHighlight` defer (loop riêng sau	textbuffer wired).
- **L05-G01 fix** (QA loop-05 gap): add `symbols.csv` rows s015 (`Renderer::_threadKeepRunning`, `renderer.hpp:151` — NOT 192 as D_5 mis-cited; verify `sed -n '147,152p' windows/src/renderer/base/renderer.hpp`), s016 (`RenderEngine::Invalidate` trait, port slice `IRenderEngine.hpp:68` → `crates/renderer/src/renderer.rs:44`). s014 (TriggerTeardown) skip — trùng s012 (canonical: keep s012, note skip in symbols.csv `notes` col).

## Tasks

- **T1 — `RendererState::trigger_scroll(delta: (i32,i32))`** (`crates/renderer/src/renderer.rs`): port `renderer.cpp:891-900`. `for engine in &mut self.engines { engine.invalidate_scroll(delta); }` → `self.scroll_previous_selection(delta)` → (skip `NotifyPaintFrame` per ADR-0003 defer). Doc-comment cite `renderer.cpp:891-900,1870-1879` + `IRenderEngine.hpp:73` + state `renderer.hpp:162,183`.
- **T2 — `RendererState::scroll_previous_selection(delta: (i32,i32))`** private helper: port `_ScrollPreviousSelection` (cpp:1870-1879). `if delta != (0,0): for rc in &mut self.last_selection_rects_by_viewport { rc.left += delta.0; rc.right += delta.0; rc.top += delta.1; rc.bottom += delta.1; } self.current_cursor_options.coord = (self.current_cursor_options.coord.0 + delta.0, self.current_cursor_options.coord.1 + delta.1);`. No-op khi delta `(0,0)` (mirror WT `if delta != til::point{0,0}` early-return). Doc-comment cite cpp:1870.
- **T3 — `RenderEngine` trait add `invalidate_scroll(&mut self, delta: (i32,i32))`** (`renderer.rs:41-48`): port slice `IRenderEngine.hpp:73`. Default impl optional (mirror loop-05 không default — engines must impl). Update `CountingEngine`/`SharedCountingEngine` test stub (`renderer.rs:193-217`) add `invalidate_scroll_calls: Vec<(i32,i32)>` field + impl.
- **T4 — 2 new state members** `RendererState`: `current_cursor_options: CursorOptions` (new struct, field `coord: (i32,i32)`, default `(0,0)`) + `last_selection_rects_by_viewport: Vec<InvalidRect>` (default empty). Mirror `renderer.hpp:162,183`. Public getters `current_cursor_coord()` + `last_selection_rects()` cho testability. **Reuse `InvalidRect`** (`renderer.rs:30`, loop-05) as `til::rect` port — no new rect type.
- **T5 — Ledger P10 update** (`parity/ledger/P10-renderer/symbols.csv`): add s015 (`Renderer::_threadKeepRunning`, field, `renderer.hpp:151`, `crates/renderer/src/renderer.rs:58`, status `partial`, test `test_trigger_teardown_sets_thread_keep_running_false`, ADR-0003, note "atomic bool, thread lifecycle"); add s016 (`RenderEngine::Invalidate` trait method, `IRenderEngine.hpp:68`, `crates/renderer/src/renderer.rs:44`, partial, test `test_trigger_redraw_dispatches_invalidate_to_each_engine_rc`, ADR-0003, note "trait dispatch port slice"); add s017 (`Renderer::TriggerScroll(point*)`, fn, `renderer.cpp:891`, `crates/renderer/src/renderer.rs:<T1 line>`, partial, test `test_trigger_scroll_dispatches_invalidate_scroll_to_each_engine` + `test_scroll_previous_selection_noop_zero_delta`, ADR-0003, note "per-engine InvalidateScroll + ScrollPreviousSelection"); add s018 (`Renderer::_ScrollPreviousSelection`, fn, `renderer.cpp:1870`, `crates/renderer/src/renderer.rs:<T2 line>`, partial, test `test_scroll_previous_selection_shifts_state_nonzero_delta`, ADR-0003, note "shift selection rects + cursor coord"). **Verify `wt_anchor` via `sed -n`** read-only against `windows/src/renderer/base/{renderer.cpp,renderer.hpp}` + `windows/src/renderer/inc/IRenderEngine.hpp` BEFORE write row (L04-G02 lesson). Update `files.csv` f00979/f00980 `notes` append "L05-G01 closed" + "loop06 scroll port".
- **T6 — ADR**: no new ADR (scroll port-faithful, no new decision). ADR-0003 behavioral-parity references unchanged.

## Preservation

Re-run loop-05 verified behaviors (E_5): VR1-VR5 (renderer::tests 5/0), P1-P7 (workspace 485/0, colorfix 8/0, settings 18/0, color table 256, terminal seam 1/0, wt_colorfix 3/0, clippy lib clean). Loop-06 không touch colorfix/settings/paint/payload/cell/grid; không touch crates/core|control|font|connection. Single-writer `crates/renderer/src/renderer.rs` + `parity/ledger/P10-renderer/{files,symbols}.csv` only.

## Validation Requirements

- **VR1 — `trigger_scroll(delta)` dispatches `invalidate_scroll(delta)` per-engine** — Given `RendererState` with 2 `CountingEngine` + `trigger_scroll((3, -2))`, Then each engine `invalidate_scroll_calls == vec![(3,-2)]` (2 engine × 1 call, mirror `renderer.cpp:894-896` `for pEngine: pEngine->InvalidateScroll(pcoordDelta)`). Black-box: 2 engine receive same delta.
- **VR2 — `trigger_scroll((0,0))` no-op on state** — Given `RendererState` with `last_selection_rects_by_viewport` non-empty (`vec![InvalidRect{0,0,10,5}]`) + `current_cursor_options.coord = (4, 7)` + `trigger_scroll((0,0))`, Then `last_selection_rects_by_viewport[0] == InvalidRect{0,0,10,5}` (unchanged) + `current_cursor_options.coord == (4,7)` (unchanged). Mirror `_ScrollPreviousSelection` early-return `delta != {0,0}` (cpp:1872).
- **VR3 — `scroll_previous_selection((3,-2))` shifts selection rects + cursor coord** — Given state with `last_selection_rects_by_viewport = vec![InvalidRect{0,0,10,5}, InvalidRect{20,10,30,15}]` + `current_cursor_options.coord = (4,7)`, Then after `trigger_scroll((3,-2))`: rects == `[InvalidRect{3,-2,13,3}, InvalidRect{23,8,33,13}]` (left/right += 3, top/bottom += -2) + `coord == (7,5)`. Mirror `rc += delta` (cpp:1875) + `coordCursor += delta` (cpp:1877).
- **VR4 — `trigger_scroll` empty state no panic** — Given `RendererState::new()` (empty engines + empty selection) + `trigger_scroll((5,5))`, Then no panic, `last_selection_rects_by_viewport` empty, `current_cursor_options.coord == (5,5)` (cursor still shifts per cpp:1877). Mirror WT: `_ScrollPreviousSelection` iter empty vector no-op, but cursor shift unconditional inside `if delta != 0`.
- **VR5 — Default getters** — Given `RendererState::new()`, Then `current_cursor_coord() == (0,0)` + `last_selection_rects() == &[]`. Mirror `CursorOptions{}` default (`renderer.hpp:162`) + empty vector default (`renderer.hpp:183`).
- **P1-P9 preservation** — `cargo test --workspace --lib` ≥485/0; `cargo test -p renderer --lib renderer::tests` 8/0 (5 loop-05 + 3 loop-06); `cargo test -p renderer --lib colorfix` 8/0; `cargo test -p renderer --lib settings::tests` 18/0; `cargo test -p core --test wt_terminalcore` 1/0; `cargo test -p renderer --test wt_renderer_colorfix` 3/0; `cargo clippy -p renderer --all-targets -- -D warnings` 0/0; `trigger_redraw` no bbox accumulate (VR5 loop-05) giữ.
- **L05-G01 closed** — `symbols.csv` rows s015/s016 exist with `wt_anchor` matching `sed -n` verify.

## Out-of-scope

- `TriggerScroll()` no-arg (`renderer.cpp:874-878`) — needs `_CheckViewportAndScroll` + `_viewport` check, defer viewport-driver loop.
- `TriggerSelection`/`TriggerSearchHighlight`/`TriggerTitleChange`/`TriggerFontChange`/`TriggerNewTextNotification` — defer (cần IRenderData wire, selection state, font stack, P11/P15).
- `NotifyPaintFrame()` → `cx.notify()` GPUI paint wire (ADR-0003 defer).
- `InvalidateSelection`/`InvalidateHighlight` engine trait methods — defer (textbuffer/selection wire P09/P11).
- `CursorOptions` full struct (only `coord` field used by `_ScrollPreviousSelection`; full struct defer cursor-render loop).
- `_fInvalidRect`/GdiEngine accumulation (out-of-scope ADR-0001 conhost/GDI/WDDM).
- `crates/control` clippy L04-G04 (pre-existing, P11 scope).
- P10 step còn lại (font stack, builtin glyphs, style rendering, cursor shapes, line rendition, effects, perf) — defer subsequent loops.
