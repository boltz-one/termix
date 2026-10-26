---
loop: 4
candidate: A_4 (crates/renderer/src/renderer.rs NEW + lib.rs expose + tests/wt_renderer_colorfix.rs allow attr fix + parity/ledger/P10-renderer/{files,symbols}.csv)
status: partial
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors:
  - vr1_t1_l03g01_clippy_all_targets_clean_non_snake_case_allow_added
  - p1_loop03_workspace_regression_486
  - p2_loop03_colorfix_unit_intact_8
  - p3_loop03_settings18_intact_18
  - p4_loop03_colortable256_default_untouched
  - p5_loop03_terminal_seam_intact_1
  - p6_loop03_wt_colorfix_integration_intact_3
  - p7_loop03_clippy_lib_renderer_clean
  - runtime_check_build_execute_pass
  - single_writer_scope_respected_renderer_crate_only
unresolved_gaps:
  - L04-G01_renderer_state_port_drift_vs_cited_wt_origin
  - L04-G02_dev_doc_citation_field_list_does_not_match_renderer_hpp_181_185
  - L04-G03_device_doc_citation_field_list_does_not_match_renderer_hpp_181_185
regressions: []
schema_valid: true
---

# Loop 04 Evidence — Fix L03-G01 (clippy allow) + Renderer invalidation/teardown skeleton

Runtime.check(A_4): build + execute → **ok**. `cargo check -p renderer` 0 error; `cargo test -p renderer --lib` 64/0 (58 baseline + 6 new `renderer::tests`); `cargo test -p renderer --lib renderer::tests` 6/0; `cargo test -p renderer --test wt_renderer_colorfix` 3/0; `cargo test --workspace --lib` 486/0; `cargo test -p core --test wt_terminalcore` 1/0; `cargo clippy -p renderer --all-targets -- -D warnings` 0 warning 0 error. Candidate assessable (schema_valid: true). Dev hoàn thành T1 (L03-G01 fix), T2 (RendererState skeleton), T3 (lib.rs expose), T4 (6 unit tests), T5 (ledger P10 update), T6 ADR skip (D_4). **1 gap major L04-G01 + 2 sub-gaps L04-G02/G03**: D_4 + Dev port `RendererState` skeleton viện citation WT `renderer/base/renderer.cpp:656-716 + renderer.hpp:181-185` như chứa `_fInvalidRect/_fInvalidRectUsed/_fAllInvalidated/_fTeardown` — nhưng đia WT QA verify (read-only `windows/src/renderer/base/`) **không có field này**:
- `renderer.hpp:181-185` thực là **selection** (`_lastSelectionPaintSpan/_Size/_RectsByViewport`), không phải invalidation.
- `_fInvalidRect/_fInvalidRectUsed/_fAllInvalidated` thực thuộc **`GdiRenderer`** (`windows/src/renderer/gdi/{gdirenderer.hpp:161,invalidate.cpp:111-135}`) — out-of-scope ADR-0001 (conhost/GDI/WDDM).
- `_fTeardown` field không tồn tại; `TriggerTeardown` (renderer.cpp:110) chỉ set `_threadKeepRunning=false` + event signal + thread join, không có invalidation flag.
- `_lastFrameType |= Frame::Background` và `_pfnFrameCompiledCallback` (Dev doc-comment renderer.rs:55,58 + commit msg) **fabricated** — zero hit trong `windows/src/renderer/`. WT `TriggerRedrawAll` (renderer.cpp:704) chỉ callback `_pfnBackgroundColorChanged/_pfnFrameColorChanged` (function pointers từ settings wiring), không sticky state.

行为 thực: WT `TriggerRedraw(Viewport)` (renderer.cpp:656-683) là per-engine `Invalidate(&region)` + `NotifyPaintFrame()`, **không accumulate `_fInvalidRect` bbox union**, không clear `_fAllInvalidated`. Dev port `trigger_redraw(&Viewport)` accumulate bbox + clear `all_invalidated` — không match cited source. Test VR2-VR4 verify code Dev viết (bbox union behavior), không verify WT verbatim correspondence. Status **partial** (Runtime.check pass + P1-P6 preservation pass + L03-G01 closed; L04-G01/G02/G03 major content gap).

## Verified Behaviors

- [x] **VR1 — T1 L03-G01 fix: `#![allow(non_snake_case)]` added** — evidence: `crates/renderer/tests/wt_renderer_colorfix.rs:11` có `#![allow(non_snake_case)]` (sau 10-line module `//!` doc, trước `use`); `cargo clippy -p renderer --all-targets -- -D warnings` → `Finished dev profile` 0 warning 0 error 0. L03-G01 closed (mirror `crates/core/tests/wt_terminalcore.rs:14` allow convention). Pass.
- [x] **P1 — Loop 03 workspace regression** — evidence: `cargo test --workspace --lib` → `TOTAL passed: 486` (480 loop-03 baseline + 6 new T4 `renderer::tests`). 0 failed. Pass.
- [x] **P2 — Loop 03 colorfix unit intact** — evidence: `cargo test -p renderer --lib colorfix` → 8 passed / 0 failed (8 test loop-02 verbatim). Dev không touch `colorfix.rs`. Pass.
- [x] **P3 — Loop 03 settings18 intact** — evidence: `cargo test -p renderer --lib settings::tests` → 18 passed / 0 failed (13 loop-02 + 5 T5 loop-03). Dev không touch `settings.rs`. Pass.
- [x] **P4 — Loop 03 color table 256 default untouched** — evidence: `RenderSettings::default()` impl không modified (`git diff e99c392 -- crates/renderer/src/settings.rs` = empty); `render_settings_default_loads_campbell_table` pass verbatim trong P3 sweep 18/0. Pass.
- [x] **P5 — Loop 03 terminal seam intact** — evidence: `cargo test -p core --test wt_terminalcore` → 1 passed / 0 failed (`wt_terminalcore__test_notify_scrolling`). Dev không touch `crates/core/`. Pass.
- [x] **P6 — Loop 03 wt_colorfix integration intact** — evidence: `cargo test -p renderer --test wt_renderer_colorfix` → 3 passed / 0 failed. T1 chỉ thêm allow attr, không sửa test logic. Pass.
- [x] **P7 — Loop 03 clippy lib renderer clean** — evidence: `cargo clippy -p renderer --all-targets -- -D warnings` → 0 warning 0 error (VR1 thực hiện). Subsumes loop-03 P7 default-target check. Pass.
- [x] **Runtime Check** — evidence: `cargo check -p renderer` 0 error; `cargo test -p renderer --lib` 64/0; `cargo test --workspace --lib` 486/0; `cargo build --workspace --tests` `Finished dev profile`. Candidate build+execute pass. `schema_valid: true`.
- [x] **Single-writer scope respected** — evidence: `git status --short` post-commit chỉ thay đổi trong renderer crate + parity ledger P10 (`crates/renderer/src/lib.rs`, `crates/renderer/src/renderer.rs`, `crates/renderer/tests/wt_renderer_colorfix.rs`, `parity/ledger/P10-renderer/files.csv`, `parity/ledger/P10-renderer/symbols.csv`). KHÔNG touch `crates/core/`, `crates/control/`, `crates/font/`, `crates/connection/`, `apps/terminal`, `windows/` (read-only). Pass.

## Unresolved Gaps

- [ ] **L04-G01 — `RendererState` port drift vs cited WT origin (`renderer/base/renderer.cpp:656-716 + renderer.hpp:181-185`)** — gap: QA white-box verify WT origin read-only:
  - **`renderer/base/renderer.hpp:181-185`** thực chứa `til::point_span _lastSelectionPaintSpan{};` (181), `size_t _lastSelectionPaintSize{};` (182), `std::vector<til::rect> _lastSelectionRectsByViewport{};` (183) — **selection** fields, **KHÔNG** `_fInvalidRect/_fInvalidRectUsed/_fAllInvalidated/_fTeardown` như D_4 line 37 + Dev doc-comment renderer.rs:47-58 + commit msg viện. D_4 + Dev citation field list **không match** cited hpp line range.
  - **`_fInvalidRect`/`_fInvalidRectUsed`/`_fAllInvalidated`** thực thuộc **`GdiRenderer`** (`windows/src/renderer/gdi/gdirenderer.hpp:161`, `invalidate.cpp:111-135`, `paint.cpp:234`, `state.cpp:29`) — **out-of-scope ADR-0001** (conhost/GDI/WDDM waived). Dev port field này sang `renderer/base` `RendererState` mà không flag cross-cite + không ADR mới.
  - **`_fTeardown` field** không tồn tại trong `renderer/base`. WT `TriggerTeardown` (renderer.cpp:110-122) chỉ: `_threadMutex.lock_exclusive()` → `_threadKeepRunning.store(false)` → `_shutdownEvent.SetEvent()` → `NotifyPaintFrame()` → `_enable.SetEvent()` → `WaitForSingleObject(_thread)` → `_thread.reset()`. Không set boolean teardown flag. Dev `trigger_teardown` chỉ set `teardown_requested=true` (re-expression khác behavior).
  - **`_lastFrameType |= Frame::Background` + `_pfnFrameCompiledCallback`** (Dev doc-comment renderer.rs:55-58 + commit msg "sticky flag mirror WT `_lastFrameType |=`") — **zero hit** trong `windows/src/renderer/` (grep confirmation). WT `TriggerRedrawAll` (renderer.cpp:704-722) callback `_pfnBackgroundColorChanged()` (function pointer từ settings wiring, line 715) và `_pfnFrameColorChanged()` (line 720), không sticky state, không `_lastFrameType enum`, không `_pfnFrameCompiledCallback`.
  - **WT `TriggerRedraw(Viewport)` (renderer.cpp:656-683)** thực: `region.ToExclusive()` → loop check `buffer.IsDoubleWidthLine(row)` → if `view.TrimToViewport(&srUpdateRegion)` → for `pEngine : _engines: pEngine->Invalidate(&srUpdateRegion)` → `NotifyPaintFrame()`. **KHÔNG accumulate `_fInvalidRect` bbox union, KHÔNG clear `_fAllInvalidated`** như Dev `trigger_redraw` (renderer.rs:71-80). Dev port behavior bbox union thuộc GdiEngine accumulation pattern.
  - **Behavioral consequence**: VR2-VR4 (TriggerRedraw bbox union/accumulation) verify code Dev viết, **không** verify WT `TriggerRedraw` verbatim — bài test có thể pass mà không match cited WT origin (test circular với impl). Test pass = self-consistent re-expression, không port-faithful criterion.
  - **Severity**: **major** (content-faithfulness port-parity fail), không block Runtime.check (build/test green). Master plan success criterion "`cargo xtask audit --final` exit 0" requires ledger `verified` rows — f00979/f00980/s010-s013 `status=partial` đã record, không `verified`, tương xứng gap này.
  - **Fix (next-loop Developer)**: 1 trong 2 hướng — (a) port lại faithful `renderer/base` `TriggerRedraw/All/Teardown` thực sự WT (per-engine `Invalidate(*)` + `NotifyPaintFrame()` callback pattern), giữ `_engines`/`_pfnBackgroundColorChanged`/`_pfnFrameColorChanged` re-expression, **KHÔNG accumulate bbox** (sang trait `RenderEngine::Invalidate(Vector<til::rect>)`); hoặc (b) deliberate deviation: explicit ADR-0016 "GdiRenderer invalidation accumulation pattern re-expressed into `RendererState` cho GPUI repaint scheduler" + sửa D citation từ `renderer/base` sang `renderer/gdi` origin + ghi rổ GdiRenderer field source. Cần Planner quyết hướng (port-faithful vs deliberate-deviation+ADR).
- [ ] **L04-G02 — D_4 citation field list mismatch `renderer.hpp:181-185`** — gap: `loop-04-dev-doc.md` line 37 "state members: `_fInvalidRectUsed`, `_fAllInvalidated`, `_fTeardown`, `_fInvalidRect`" viện `renderer/base/renderer.hpp:39-42, 181-185` — nhưng 181-185 là selection fields, 39-42 là method decls. **D_4 hallucinate field-citation match**. Planner gap (D_t schema: Out-of-scope/Context bị sai事实). Fix: Planner loop sau phải verify citation bằng `sed -n` against `windows/src/...` trước khi write D_t (cross-check rule).
- [ ] **L04-G03 — Dev doc-comment + commit msg cite fabricated WT origin** — gap: `crates/renderer/src/renderer.rs:47-58` doc-comment ghi "_fInvalidRect (renderer.hpp:181)" / "_lastFrameType |= Frame::Background" / "_pfnFrameCompiledCallback trigger" + commit msg "mirror WT `_fInvalidRect/_fInvalidRectUsed/_fAllInvalidated/_fTeardown`" + "mirror WT `_lastFrameType |=`" — fabricated citation (zero hit trong WT renderer/base). Dev blind from D_4 wrong citation, propagate into code docs + commit. Fix: together với L04-G01 fix, update doc-comment cite đúng WT origin (GdiRenderer field hoặc `renderer/base` actual fields `_pfnBackgroundColorChanged/_pfnFrameColorChanged` + `_threadKeepRunning`).

## Regressions

Không có. Loop-03 P1-P7 + L02-G01..G05 verified behaviors hiệu lực (workspace 480 → 486, colorfix 8/0, settings 18/0 = 13 + 5, color table 256, terminal seam 1/0, wt_colorfix 3/0, clippy lib clean). T1 only thêm allow attr (no logic change). T2/T3/T4 chỉ **add** new module + tests, không modify existing behaviors. P4 `RenderSettings::default()` untouched (git diff confirm). L03-G01 (loop-03 lint gap) **closed** this loop (VR1).

## Runtime Check

- Build: `cargo check -p renderer` → Finished `dev` profile, 0 error.
- Execute renderer lib: `cargo test -p renderer --lib` → **64 passed; 0 failed** (colorfix 8 + settings 18 + paint/payload/cell/grid 32 + renderer::tests 6 new).
- Execute T4 renderer::tests: `cargo test -p renderer --lib renderer::tests` → **6 passed; 0 failed** (VR2-VR6 logic, caveat L04-G01: verify code行为 không verify WT verbatim).
- Execute T6 wt_colorfix: `cargo test -p renderer --test wt_renderer_colorfix` → **3 passed; 0 failed**.
- Workspace: `cargo test --workspace --lib` → **486 passed; 0 failed** (loop-03 baseline 480 + 6 T4).
- Loop-03 seam: `cargo test -p core --test wt_terminalcore` → **1 passed; 0 failed**.
- Lint: `cargo clippy -p renderer --all-targets -- -D warnings` → **0 warning 0 error** (VR1 — L03-G01 closed).
- `schema_valid: true` — build + execute pass. Gap L04-G01/G02/G03 là content-faithfulness port-parity, không build/run failure.

## Assessment

**Accept: NO (partial) — 3 unresolved gaps L04-G01/G02/G03 (port-faithfulness).**

Dev hoàn thành 5/6 task vs D_4 về mặt **mechanical**: T1 (L03-G01 `#![allow(non_snake_case)]` line 11, clippy clean), T2 (`RendererState` struct 6 field + 4 trigger method + 6 getter + helper union_rect), T3 (`lib.rs` `pub mod renderer` + `pub use renderer::{InvalidRect, RendererState}`), T4 (6 unit test VR2-VR6 green), T5 (ledger P10 f00979/f00980 partial + s010-s013 partial); T6 ADR skip (D_4 explicit out-of-scope). L03-G01 (loop-03 lint gap) **closed** (VR1). P1-P7 + Runtime.check + single-writer scope đều verified (10 verified_behaviors).

**3 gap mới L04-G01/G02/G03** — major content-faithfulness port-parity:
1. **L04-G01**: `RendererState` port field/method behavior viện WT `renderer/base` origin nhưng thực không match — `_fInvalidRect/_fInvalidRectUsed/_fAllInvalidated` thuộc `renderer/gdi` (out-of-scope ADR-0001); `_fTeardown`/`_lastFrameType`/`_pfnFrameCompiledCallback` fabricated; `TriggerRedraw` WT không accumulate bbox union. Deviation cần decide: (a) port lại faithful WT actual behavior (per-engine `Invalidate(*)` + callback, no accumulation), hoặc (b) deliberate deviation + ADR-0016 + sửa citation sang `renderer/gdi`.
2. **L04-G02**: D_4 hallucinate field-citation match (renderer.hpp:181-185 = selection, không invalidation). Planner gap.
3. **L04-G03**: Dev doc-comment + commit msg propagate fabricated citation.

**Master plan NOT done** — loop 04 partial, 9/9 + 4/9 + 4/9 (loop02 + loop03 + loop04) P10 symbols ported = 17 symbol entry, but **4 symbol loop-04 (s010-s013) port-faithfulness gap chưa verified** — `partial` không `verified`. Runtime owns subsequent loops.

**Pre-existing (NOT loop-04 regression, recorded earlier)**: `cargo clippy --workspace --all-targets -- -D warnings` fail tại `crates/control/src/interactivity.rs:168` (ảo non-binding `let _ = lock`) từ commit `4028642` workspace scaffold, trước A_4. Triple-confirmed: checkout `4028642` đã fail 3 error. Out-of-scope	renderer crate (control là P11). User decision: loop riêng `control-hygiene` (recorded vào issue-ledger L04-G04 khi Runtime/Planner open).

**Notes (pre-existing, out-of-scope QA loop 04)**:
- `apps/terminal` git status sạch. Workspace build ok.
- `cargo xtask audit --final` chưa re-run (loop-01 note: `merge symbols` duplicate-id preexisting schema bug, phase-agnostic, out-of-scope).
- Master plan còn 15+ phase (P11..P19); global lint gate `cargo clippy --workspace -- --all-targets -D warnings` cần addressed ở phase đầu có touch `control` (P11) hoặc loop `control-hygiene` riêng.
