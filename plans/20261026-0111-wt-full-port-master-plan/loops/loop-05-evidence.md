---
loop: 5
candidate: A_5 (crates/renderer/src/renderer.rs rework port-faithful + parity/ledger/P10-renderer/{files,symbols}.csv update)
status: passed
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors:
  - vr1_t1_trigger_redraw_per_engine_invalidate_dispatch
  - vr2_t1_trigger_redraw_point_inclusive_1x1_delegate
  - vr3_t1_trigger_redraw_all_invalidate_all_plus_fire_and_forget_callbacks
  - vr4_t1_trigger_teardown_thread_lifecycle
  - vr5_t1_no_bbox_accumulation_drift_closed
  - vr6_t2_module_exposed_full_path_accessible
  - vr7_t3_doc_comment_cite_port_faithful_fabricated_removed
  - vr8_t4_5_tests_green
  - p1_workspace_regression_485
  - p2_colorfix_unit_intact_8
  - p3_settings18_intact_18
  - p4_color_table_256_default
  - p5_terminal_seam_intact_1
  - p6_wt_colorfix_integration_intact_3
  - p7_clippy_lib_renderer_clean
  - runtime_check_build_execute_pass
  - single_writer_scope_respected_renderer_crate_only
  - l04g01_closed_port_faithful_dispatch_no_accumulator
  - l04g02_closed_d5_citation_correct
  - l04g03_closed_doc_comment_corrected
unresolved_gaps:
  - L05-G01_ledger_s014_s016_not_added_minor_completeness
regressions: []
schema_valid: true
---

# Loop 05 Evidence — Close L04-G01/G02/G03: rework `RendererState` port-faithful WT `renderer/base`

Runtime.check(A_5): build + execute → **ok**. `cargo check -p renderer` 0 error; `cargo test -p renderer --lib renderer::tests` 5/0 (5 test — Dev consolidate VR3+VR4 vào 1 test `test_trigger_redraw_all_dispatches_invalidate_all_and_callbacks`, criterion vẫn cover); `cargo test -p renderer --lib` 63/0; `cargo test --workspace --lib` 485/0 (loop-04 486 − 1 do test consolidate); `cargo test -p core --test wt_terminalcore` 1/0; `cargo test -p renderer --test wt_renderer_colorfix` 3/0; `cargo clippy -p renderer --all-targets -- -D warnings` 0 warning 0 error. Candidate assessable (schema_valid: true).

**L04-G01/G02/G03 — ALL CLOSED.** Dev rework `RendererState` mirror WT `renderer/base` thực behavior (chỉ xác minh read-only `windows/src/renderer/base/renderer.cpp:110,656-722`): `trigger_redraw` per-engine `invalidate(region)` (cpp:664-668) **KHÔNG bbox union accumulate**; `trigger_redraw_all` per-engine `invalidate_all()` + fire-and-forget callbacks `_pfnBackgroundColorChanged`/`_pfnFrameColorChanged` (cpp:710-720, no sticky flags); `trigger_teardown` set `thread_keep_running=false` + `thread_handle_active=false` (cpp:113 `_threadKeepRunning.store(false)` + thread reset); field fabricated loop-04 (`invalid_rect`/`invalid_rect_used`/`all_invalidated`/`teardown_requested`/`background_changed`/`frame_changed`) **removed**; field port-faithful added (`thread_keep_running`/`thread_handle_active`/`engines`/`on_*_color_changed`). `RenderEngine` trait minimal (`invalidate`/`invalidate_all`) introduced (port slice WT `RenderEngineBase::Invalidate/InvalidateAll`).

D_5 (Planner loop-05) cita đúng WT origin (`renderer.hpp:175-185` actual `_pfn*ColorChanged`/`_engines`/`_threadKeepRunning`, không phải `_fInvalidRect` như D_4 hallucinate) → **L04-G02 closed by D_5 itself**. Dev doc-comment (`renderer.rs:1-21`) rewrite cite port-faithful, fabricated terms (`_fInvalidRect`/`_lastFrameType`/`_pfnFrameCompiledCallback`/`_fTeardown`) chỉ xuất hiện trong **negation context** ("KHÔNG có", "fabricated", "zero hit") → **L04-G03 closed**.

1 minor ledger-completeness gap **L05-G01**: D_5 T5 VR9 yêu cầu add symbols.csv s014(`Renderer::TriggerTeardown`)/s015(`_threadKeepRunning`)/s016(`RenderEngine::Invalidate`) — Dev **chỉ edit s010-s013, không add s014-s016**. Dev xử lý hợp lý phần trùng: s012 đã là `Renderer::TriggerTeardown` (giữ, không duplicate s014). Nhưng s015 (`_threadKeepRunning` field) + s016 (`RenderEngine::Invalidate` trait method) — Dev port có field + method nhưng **không record vào symbols.csv**. Behavioral parity OK (field/method tồn tại + test verify); chỉ ledger row thiếu. Minor (documentation, không block port-faithfulness).

**Status passed** — 18 verified behaviors (VR1-VR8 + P1-P7 + Runtime Check + single-writer + 3 L04 gap closed); 1 minor unresolved gap (L05-G01 ledger completeness, non-blocking); 0 regressions.

## Verified Behaviors

- [x] **VR1 — T1.4 `trigger_redraw` per-engine `Invalidate(region)` dispatch** — evidence: `crates/renderer/src/renderer.rs:99-103` `for engine in &mut self.engines { engine.invalidate(region); }` mirror WT `renderer/base/renderer.cpp:664-668` `for (const auto pEngine : _engines) { LOG_IF_FAILED(pEngine->Invalidate(&srUpdateRegion)); }` (verified read-only `windows/src/renderer/base/renderer.cpp:664-668`). Test `test_trigger_redraw_dispatches_invalidate_to_each_engine_rc`: 2 `SharedCountingEngine` pushed, `trigger_redraw(&Viewport{5,7,20,15})` → mỗi engine `invalidate_calls == vec![Viewport{5,7,20,15}]`, `invalidate_all_calls == 0`. Pass.
- [x] **VR2 — T1.4 `trigger_redraw_point` inclusive 1×1 delegate** — evidence: `crates/renderer/src/renderer.rs:108-111` `let region = Viewport::new(point.0, point.1, point.0, point.1)` delegate `trigger_redraw(&region)`. WT `renderer/base/renderer.cpp:691` `TriggerRedraw(Viewport::FromDimensions(*pcoord, {1,1}))` — `FromDimensions` impl (`windows/src/types/viewport.cpp:37-46`) `inclusive_rect{origin.x, origin.y, origin.x+width-1, origin.y+height-1}` = `{5,7,5,7}` inclusive (right==left, bottom==top). Test `test_trigger_redraw_point_one_by_one_delegates`: engine `invalidate_calls == vec![Viewport::new(5,7,5,7)]`. Port-faithful (inclusive convention). Pass. *(Note: D_5 VR2 ghi `Viewport{5,7,6,8}` exclusive — D_5 sai WT ground; Dev impl đúng inclusive per `FromDimensions`. Minor D_5 doc inconsistency, không Dev gap.)*
- [x] **VR3 — T1.4 `trigger_redraw_all` per-engine `invalidate_all` + fire-and-forget callbacks** — evidence: `crates/renderer/src/renderer.rs:118-132` `for engine in &mut self.engines { engine.invalidate_all(); }` → `if background_changed { if let Some(cb) = &mut self.on_background_color_changed { cb(); } }` → same frame. WT `renderer/base/renderer.cpp:704-720` `for (const auto pEngine : _engines) { pEngine->InvalidateAll(); }` → `NotifyPaintFrame()` → `if (backgroundChanged && _pfnBackgroundColorChanged) _pfnBackgroundColorChanged()` → `if (frameChanged && _pfnFrameColorChanged) _pfnFrameColorChanged()`. Test `test_trigger_redraw_all_dispatches_invalidate_all_and_callbacks` (Dev consolidate VR3+VR4): 2 engine + 2 counter callback, `trigger_redraw_all(true,true)` → mỗi engine `invalidate_all_calls == 1`, `invalidate_calls == []`, both callbacks fired 1×; `trigger_redraw_all(false,false)` → `invalidate_all_calls == 2` (cumulative), callbacks **không fire thêm** (fire-and-forget, no sticky). Pass.
- [x] **VR4 — T1.4 `trigger_teardown` thread lifecycle** — evidence: `crates/renderer/src/renderer.rs:140-143` `self.thread_keep_running.store(false, Ordering::SeqCst); self.thread_handle_active = false;`. WT `renderer/base/renderer.cpp:113` `_threadKeepRunning.store(false, std::memory_order_relaxed)` + cpp:118 `_thread.reset()`. Test `test_trigger_teardown_sets_thread_keep_running_false`: `new()` → `!is_thread_keep_running() && !is_thread_active() && is_teardown_complete()` (trivially complete); `start_thread()` → `is_thread_keep_running() && is_thread_active() && !is_teardown_complete()`; `trigger_teardown()` → `!is_thread_keep_running() && !is_thread_active() && is_teardown_complete()`. Pass.
- [x] **VR5 — T1.4 No bbox accumulation (L04-G01 drift closed)** — evidence: `crates/renderer/src/renderer.rs:99-103` `trigger_redraw` chỉ loop `engine.invalidate(region)`, KHÔNG field accumulator (`invalid_rect`/`invalid_rect_used`/`all_invalidated` removed từ struct `RendererState:55-73`, chỉ `thread_keep_running`/`thread_handle_active`/`engines`/`on_*_color_changed`). Test `test_trigger_redraw_no_bbox_accumulation`: 1 engine, 2 `trigger_redraw` disjoint regions `(5,7,10,12)` then `(20,3,25,8)` → `invalidate_calls.len() == 2`, `invalidate_calls[0] == Viewport::new(5,7,10,12)`, `invalidate_calls[1] == Viewport::new(20,3,25,8)`. Mirror WT `TriggerRedraw` không có `_fInvalidRect` accumulator. L04-G01 closed. Pass.
- [x] **VR6 — T2 module exposed (full-path accessible)** — evidence: `crates/renderer/src/lib.rs:37` `pub mod renderer;` + `lib.rs:46` `pub use renderer::{InvalidRect, RendererState};`. Trait `RenderEngine` (`renderer.rs:41` `pub trait RenderEngine`) accessible via full path `crate::renderer::RenderEngine` (mod public). Dev **not** re-export `renderer::RenderEngine` at lib root — deliberate to avoid name collision with `paint::RenderEngine` (`crates/renderer/src/paint.rs:50` `pub trait RenderEngine`, re-exported `lib.rs:45` `pub use paint::{..., RenderEngine, ...}`). 2 trait cùng tên trong 2 mod khác nhau → re-export cả 2 conflict tại lib root; full-path access preserves port-faithful Rust idiom. D_5 VR6 yêu cầu re-export `RenderEngine` — Dev deviation **with rationale** (name collision), not gap. `renderer.rs #[cfg(test)] mod tests` dùng `use super::*` (trong mod) nên không cần lib re-export. Pass.
- [x] **VR7 — T3 doc-comment cite port-faithful (L04-G03 closed)** — evidence: `crates/renderer/src/renderer.rs:1-21` doc-comment cite `renderer/base/renderer.cpp:110,656-722` + `renderer.hpp:175-185` + `_threadKeepRunning`/`_pfnBackgroundColorChanged`/`_pfnFrameColorChanged`/`_engines` (port-faithful origin, verified read-only `windows/src/renderer/base/`). Fabricated terms `_fInvalidRect`/`_lastFrameType`/`_pfnFrameCompiledCallback`/`_fTeardown` grep 4 hit — **all trong negation context**: `renderer.rs:8` "KHÔNG còn GdiEngine accumulation pattern (`_rcInvalid`/`_fInvalidRectUsed`", `renderer.rs:10` "`renderer/base Renderer` không có `_fInvalidRect` accumulator", `renderer.rs:11` "`_lastFrameType` enum hay `_pfnFrameCompiledCallback` (zero hit fabricated", `renderer.rs:117` "(loop-04 `_lastFrameType |=` / `_pfnFrameCompiledCallback` was fabricated)". Port-faithful cite 14 hit. L04-G03 closed. Pass.
- [x] **VR8 — T4 5 tests green** — evidence: `cargo test -p renderer --lib renderer::tests` → `5 passed; 0 failed`. 5 test (Dev consolidated 6 D_5 VR thành 5: VR3+VR4 gộp `test_trigger_redraw_all_dispatches_invalidate_all_and_callbacks`): `test_trigger_redraw_dispatches_invalidate_to_each_engine_rc` (VR1), `test_trigger_redraw_point_one_by_one_delegates` (VR2), `test_trigger_redraw_all_dispatches_invalidate_all_and_callbacks` (VR3+VR4 consolidated), `test_trigger_teardown_sets_thread_keep_running_false` (VR4 lifecycle), `test_trigger_redraw_no_bbox_accumulation` (VR5). All criterion (per-engine dispatch, inclusive point delegate, invalidate_all + fire-and-forget callbacks, thread lifecycle, no accumulation) verified. Pass.
- [x] **P1 — Loop 04 workspace regression** — evidence: `cargo test --workspace --lib` → **485 passed; 0 failed** (loop-04 baseline 486 − 1 due to test consolidation 6→5; criterion still covered by VR3+VR4 consolidated). Lệch 1 test count, không regression behavior. Pass with note.
- [x] **P2 — Loop 04 colorfix unit intact** — evidence: `cargo test -p renderer --lib colorfix` → **8 passed; 0 failed** (8 test loop-02 verbatim). Dev không touch `colorfix.rs`. Pass.
- [x] **P3 — Loop 04 settings 18 intact** — evidence: `cargo test -p renderer --lib settings::tests` → **18 passed; 0 failed** (13 loop-02 + 5 loop-03). Dev không touch `settings.rs`. Pass.
- [x] **P4 — Loop 04 color table 256 default** — evidence: `cargo test -p renderer --lib settings::tests::render_settings_default_loads_campbell_table` → **1 passed**. `RenderSettings::default()` impl không modified. Pass.
- [x] **P5 — Loop 04 terminal seam intact** — evidence: `cargo test -p core --test wt_terminalcore` → **1 passed; 0 failed** (`wt_terminalcore__test_notify_scrolling`). Dev không touch `crates/core/`. Pass.
- [x] **P6 — Loop 04 wt_colorfix integration intact** — evidence: `cargo test -p renderer --test wt_renderer_colorfix` → **3 passed; 0 failed**. `wt_renderer_colorfix.rs:11` `#![allow(non_snake_case)]` (L03-G01 closed loop-04) preserved. Dev không touch test file loop-05. Pass.
- [x] **P7 — Loop 04 clippy lib renderer clean** — evidence: `cargo clippy -p renderer --all-targets -- -D warnings` → `Finished dev profile` 0 warning 0 error. Rework clippy-clean. Pass.
- [x] **Runtime Check** — evidence: `cargo check -p renderer` 0 error; `cargo build --workspace --tests` Finished `dev` profile; `cargo test -p renderer --lib` 63/0 (58 + 5 renderer::tests consolidated); `cargo test --workspace --lib` 485/0; `cargo test -p core --test wt_terminalcore` 1/0; `cargo test -p renderer --test wt_renderer_colorfix` 3/0; `cargo clippy -p renderer --all-targets -- -D warnings` 0/0. Candidate build+execute pass. `schema_valid: true`.
- [x] **Single-writer scope respected** — evidence: `git diff f4f707d..2f2c4fe --stat` chỉ 4 file: `crates/renderer/src/renderer.rs` (+434/-188), `parity/ledger/P10-renderer/files.csv` (+4/-4), `parity/ledger/P10-renderer/symbols.csv` (+8/-8), `loops/loop-05-dev-doc.md` (+320, D_5 Planner artifact). KHÔNG touch `crates/core/`, `crates/control/`, `crates/font/`, `crates/connection/`, `crates/renderer/src/{colorfix,settings,paint,payload,cell,grid}.rs`, `apps/terminal`, `windows/` (read-only — `git diff -- windows/` empty). Pass.
- [x] **L04-G01 closed — port-faithful dispatch + no accumulator** — evidence: VR1 (`trigger_redraw` per-engine `Invalidate`), VR3 (`trigger_redraw_all` per-engine `InvalidateAll` + fire-and-forget callbacks), VR4 (`trigger_teardown` thread lifecycle store false), VR5 (no bbox accumulation, field `invalid_rect`/`invalid_rect_used`/`all_invalidated` removed). WT origin verified read-only (`renderer.cpp:656-722`, `renderer.cpp:110-122`). Issue-ledger row L04-G01 mark `closed` at loop 5.
- [x] **L04-G02 closed — D_5 citation correct** — evidence: `loop-05-dev-doc.md:34-65` Context section cites WT origin chính xác: `renderer.hpp:175-185` = `_pfnBackgroundColorChanged`/`_pfnFrameColorChanged`/`_pfnRendererEnteredErrorState`/`_isSynchronizingOutput`/`_forceUpdateViewport`/`_lastSelectionPaint*` (selection + callbacks, NOT `_fInvalidRect`); `_fInvalidRect`/`_fInvalidRectUsed`/`_fAllInvalidated` thuộc `renderer/gdi/gdirenderer.hpp:159-160` out-of-scope ADR-0001; `_fTeardown`/`_lastFrameType`/`_pfnFrameCompiledCallback` zero hit fabricated. D_5 cita đúng — L04-G02 closed by D_5 itself (Planner corrected citation).
- [x] **L04-G03 closed — Dev doc-comment corrected** — evidence: VR7. Doc-comment `renderer.rs:1-21` rewritten port-faithful, fabricated terms only in negation. Commit msg `2f2c4fe` "renderer(loop05): port-faithful per-engine dispatch + thread lifecycle" — no fabricated cite. L04-G03 closed.

## Unresolved Gaps

- [ ] **L05-G01 — Ledger symbols.csv s014-s016 not added (minor completeness)** — gap: D_5 T5 VR9 yêu cầu add `symbols.csv` rows s014 (`Renderer::TriggerTeardown`, renderer.cpp:110), s015 (`Renderer::_threadKeepRunning`, renderer.hpp:192), s016 (`RenderEngine::Invalidate`, RenderEngineBase.hpp:NN). Dev **chỉ edit s010-s013, không add s014-s016**. Dev xử lý phần overlap hợp lý: s012 đã是 `Renderer::TriggerTeardown` (giữ, không duplicate s014). Nhưng s015 (`_threadKeepRunning` field) + s016 (`RenderEngine::Invalidate` trait method) — Dev port có field (`renderer.rs:58`) + method (`renderer.rs:44`) nhưng **không record vào symbols.csv**. Behavioral parity OK (field/method tồn tại + VR1/VR4 verify); chỉ ledger row thiếu. Severity: **minor** (documentation completeness, không block port-faithfulness, không build/test failure). Fix (next-loop Developer): add s015/s016 vào `parity/ledger/P10-renderer/symbols.csv` with `wt_anchor` verified via `sed -n` against `windows/src/renderer/base/renderer.hpp` + `windows/src/renderer/inc/RenderEngineBase.hpp`. Note: s014 skip (trùng s012) — hoặc canonical: rename s012→s014 nếu Planner prefer WT class-prefixed naming.

## Regressions

Không có. Loop-04 P1-P7 + L03-G01 verified behaviors hiệu lực (workspace 486 → 485 due to test consolidation 6→5, criterion covered; colorfix 8/0, settings 18/0, color table 256, terminal seam 1/0, wt_colorfix 3/0, clippy lib clean). L04-G01/G02/G03 (loop-04 port-faithfulness gaps) **closed** this loop (VR1-VR8 + D_5 citation). Behavioral parity restored.

## Runtime Check

- Build: `cargo check -p renderer` → Finished `dev` profile, 0 error.
- Execute renderer::tests: `cargo test -p renderer --lib renderer::tests` → **5 passed; 0 failed** (VR1-VR5, port-faithful dispatch behavior, không bbox union).
- Execute renderer lib: `cargo test -p renderer --lib` → **63 passed; 0 failed** (colorfix 8 + settings 18 + paint/payload/cell/grid 32 + renderer::tests 5).
- Workspace: `cargo test --workspace --lib` → **485 passed; 0 failed** (loop-04 486 − 1 test consolidation).
- Loop-04 seam: `cargo test -p core --test wt_terminalcore` → **1 passed; 0 failed**.
- Loop-04 wt_colorfix: `cargo test -p renderer --test wt_renderer_colorfix` → **3 passed; 0 failed**.
- Lint: `cargo clippy -p renderer --all-targets -- -D warnings` → **0 warning 0 error**.
- `schema_valid: true` — build + execute pass. Gap L05-G01 là ledger completeness, không build/run failure.

## Assessment

**Accept: YES (passed) — 3 loop-04 gaps L04-G01/G02/G03 closed; 1 new minor gap L05-G01 (ledger completeness); 0 regressions.**

Dev hoàn thành T1-T5 vs D_5 port-faithful:
- **T1** Rework `RendererState`: xóa 6 field fabricated (`invalid_rect`/`invalid_rect_used`/`all_invalidated`/`teardown_requested`/`background_changed`/`frame_changed`), thêm 5 field port-faithful (`thread_keep_running`/`thread_handle_active`/`engines`/`on_background_color_changed`/`on_frame_color_changed`), introduce `RenderEngine` trait minimal (`invalidate`/`invalidate_all`), implement `trigger_redraw`/`trigger_redraw_point`/`trigger_redraw_all`/`trigger_teardown`/`start_thread` port-faithful. ✓
- **T2** `lib.rs` re-export: `pub use renderer::{InvalidRect, RendererState}` — `RenderEngine` deliberately not re-export at lib root (avoid name collision with `paint::RenderEngine`); full-path `crate::renderer::RenderEngine` accessible. Deviation with rationale. ✓
- **T3** Doc-comment rewrite: cite `renderer.cpp:110,656-722` + `renderer.hpp:175-175-185` + actual fields; fabricated terms only in negation. ✓
- **T4** 5 unit tests (consolidated from D_5 6): all criterion (per-engine dispatch, inclusive point, invalidate_all + fire-and-forget callbacks, thread lifecycle, no accumulation) covered. ✓
- **T5** Ledger P10 update: `files.csv` f00979/f00980 `notes` mention "port-faithful" + "L04-G01 closed"; `symbols.csv` s010-s013 `test_ids` + `notes` updated. **KHÔNG add s014-s016** per D_5 VR9 — gap L05-G01. ◐
- **T6** ADR-0016 skip: D_5 recommend skip (rework là correction, không new decision). ✓

**L04-G01/G02/G03 — closed** (port-faithfulness restored; D_5 citation correct; Dev doc-comment corrected). WT origin verified read-only `windows/src/renderer/base/renderer.cpp:110,656-722` — `TriggerRedraw` per-engine `Invalidate(*)` no bbox union, `TriggerRedrawAll` per-engine `InvalidateAll()` + fire-and-forget callbacks, `TriggerTeardown` `_threadKeepRunning.store(false)` + thread reset. Re-expression Rust+GPUI faithful (ADR-0003 behavioral parity: GPUI owns scene, `NotifyPaintFrame()` wire `cx.notify()` deferred to paint-driver loop).

**Master plan NOT done** — loop 05 passed, P10 renderer crate: 9 (loop02 colorfix) + 4 (loop03 settings) + 2 (loop03 RenderSettings API) + 4 (loop05 RendererState port-faithful) = 19 symbol entries `partial` (chưa `verified` — required independent reviewer). L05-G01 minor ledger completeness. Còn P10 step còn lại (paint driver, scroll, cursor, selection, font stack) + P11-P19 (control, app, host, uia, adapter, telemetry, launch). Runtime owns subsequent loops.

**Pre-existing (NOT loop-05 regression, recorded earlier)**: `cargo clippy --workspace --all-targets -- -D warnings` fail tại `crates/control/src/interactivity.rs:168` (ảo non-binding `let _ = lock`) từ commit `4028642` workspace scaffold. Out-of-scope renderer crate (control là P11). User decision: loop riêng `control-hygiene` hoặc address khi P11 touch `control` (issue-ledger L04-G04 stays open).

**Notes (pre-existing, out-of-scope QA loop 05)**:
- `apps/terminal` git status sạch. Workspace build ok.
- `cargo xtask audit --final` chưa re-run (loop-01 note: `merge symbols` duplicate-id preexisting schema bug, phase-agnostic, out-of-scope; L05-G01 minor không block).
- Master plan còn P10 step còn lại + P11..P19; global lint gate `control` cần addressed ở P11 hoặc loop `control-hygiene` riêng.
