---
loop: 6
candidate: A_6
status: passed
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors: [VR1, VR2, VR3, VR4, VR5, P1, P2, P3, P4, P5, P6, P7, P8, P9, L05-G01-closed]
unresolved_gaps: []
regressions: []
schema_valid: true
---

# Loop 06 Evidence — TriggerScroll + _ScrollPreviousSelection + L05-G01 close

## Verified Behaviors

- [x] **VR1 — `trigger_scroll(delta)` dispatches `invalidate_scroll(delta)` per-engine** — evidence: `cargo test -p renderer --lib renderer::tests test_trigger_scroll_dispatches_invalidate_scroll_to_each_engine` pass. White-box: `renderer.rs:179-184` `for engine in &mut self.engines { engine.invalidate_scroll(delta); }` mirror WT `renderer.cpp:893-896` `for pEngine: pEngine->InvalidateScroll(pcoordDelta)`. 2 CountingEngine assert `invalidate_scroll_calls == vec![(3,-2)]` each. Black-box: 2 engine receive same delta (3,-2), `invalidate_calls`/`invalidate_all_calls` empty.

- [x] **VR2 — `trigger_scroll((0,0))` no-op on state** — evidence: `test_scroll_previous_selection_noop_zero_delta` pass. White-box: `renderer.rs:192-195` `if delta == (0,0) { return; }` mirror WT cpp:1872 `if delta != {0,0}`. Pre-populated `last_selection_rects=[InvalidRect{0,0,10,5}]` + cursor (0,0) → unchanged after `trigger_scroll((0,0))`.

- [x] **VR3 — `scroll_previous_selection((3,-2))` shifts selection rects + cursor coord** — evidence: `test_scroll_previous_selection_shifts_state_nonzero_delta` pass. White-box: `renderer.rs:196-205` `for rc: rc.{left,right} += delta.0; rc.{top,bottom} += delta.1` + `current_cursor_options.coord_cursor += delta` mirror WT cpp:1875 `rc += delta` + cpp:1877 `coordCursor += delta`. 2 rects `[0,0,10,5; 20,10,30,15]` → `[3,-2,13,3; 23,8,33,13]` + cursor (0,0)→(3,-2).

- [x] **VR4 — `trigger_scroll` empty state no panic + cursor still shifts** — evidence: `test_trigger_scroll_empty_state_no_panic_and_default_getters` pass. White-box: `RendererState::new()` empty engines + empty selection → `trigger_scroll((5,5))` no panic; empty selection vector iter no-op; cursor (0,0)→(5,5) shift unconditional inside `if delta != 0` block (cpp:1877). mirror WT `_ScrollPreviousSelection` empty-vector no-op behavior.

- [x] **VR5 — Default getters** — evidence: same test asserts `r.last_selection_rects().is_empty()` + `r.current_cursor_coord() == (0,0)` at `RendererState::new()` (before trigger). White-box: `RendererState::new()` sets `last_selection_rects_by_viewport: Vec::new()` + `current_cursor_options: CursorOptions::default()` (coord (0,0)). Mirror WT `CursorOptions{}` default (renderer.hpp:162) + empty vector (renderer.hpp:183).

- [x] **P1 — workspace lib ≥485/0** — evidence: `cargo test --workspace --lib` → 489 passed / 0 failed (12 lib crates: a11y 0, app 31, connection 47, control 37, core 206, extras 0, font 24, host 0, renderer 67, settings 61, types 0, xtask 16). 489 ≥ 485 loop-05 base (+4: renderer 56→67 +11 vs loop05 56; core 206; settings 61).

- [x] **P2 — colorfix 8/0** — evidence: `cargo test -p renderer --lib colorfix` → 8 passed / 0 failed.

- [x] **P3 — settings 18/0** — evidence: `cargo test -p renderer --lib settings::tests` → 18 passed / 0 failed.

- [x] **P4 — RenderSettings::default() 256-table unchanged** — evidence: `colorfix::tests::test_srgb_lut_length_256` (1/8 colorfix) + `settings::tests::get_attribute_colors_default_returns_palette_defaults` (1/18 settings) pass. 256-entry LUT + default palette intact.

- [x] **P5 — terminal seam 1/0** — evidence: `cargo test -p core --test wt_terminalcore` → 1 passed / 0 failed.

- [x] **P6 — wt_colorfix integration 3/0** — evidence: `cargo test -p renderer --test wt_renderer_colorfix` → 3 passed / 0 failed.

- [x] **P7 — renderer::tests ≥5/0 (loop05 VR1-VR5)** — evidence: `cargo test -p renderer --lib renderer::tests` → 9 passed / 0 failed (5 loop05: trigger_redraw_dispatches, trigger_redraw_point_one_by_one, trigger_redraw_all_dispatches, trigger_teardown, trigger_redraw_no_bbox + 4 loop06: trigger_scroll_dispatches_invalidate_scroll, scroll_previous_selection_noop_zero_delta, scroll_previous_selection_shifts_state_nonzero_delta, trigger_scroll_empty_state_no_panic_and_default_getters). 9 ≥ 8 expected (5+3).

- [x] **P8 — clippy lib renderer clean** — evidence: `cargo clippy -p renderer --all-targets -- -D warnings` → clean (0 warning, 0 error). `Finished dev profile`.

- [x] **P9 — no bbox accumulation (L04-G01 closed giữ)** — evidence: `test_trigger_redraw_no_bbox_accumulation` pass. 2 disjoint `trigger_redraw` → engine nhận 2 separate calls (len=2), không bbox union.

- [x] **L05-G01 closed — symbols.csv s015/s016 + s017/s018 added, wt_anchor verified** — evidence: `parity/ledger/P10-renderer/symbols.csv` rows s015 (`Renderer::_threadKeepRunning`, wt_anchor `renderer.hpp:151`, verified via `grep -n "_threadKeepRunning" windows/src/renderer/base/renderer.hpp` → `151:        std::atomic<bool> _threadKeepRunning{ false };`), s016 (`RenderEngine::Invalidate`, wt_anchor `IRenderEngine.hpp:68`, verified via `grep -n "Invalidate\b"` → `68:        [[nodiscard]] virtual HRESULT Invalidate(const til::rect* psrRegion) noexcept = 0;`), s017 (`Renderer::TriggerScroll`, wt_anchor `renderer.cpp:891`, verified via `sed -n '891p'` → `void Renderer::TriggerScroll(const til::point* const pcoordDelta)`), s018 (`Renderer::_ScrollPreviousSelection`, wt_anchor `renderer.cpp:1870`, verified via `sed -n '1870p'` → `void Renderer::_ScrollPreviousSelection(const til::point delta)`). s014 skip (trùng s012 TriggerTeardown). `files.csv` f00979/f00980 notes append "Loop 06" + L05-G01 closed.

## Unresolved Gaps

None — all VR1-VR5 + P1-P9 + L05-G01 closure verified candidate-bound.

## Regressions

None — loop-05 verified behaviors (485 base) preserved (now 489/0, +4 from loop06 renderer lib tests).

## Runtime Check

- **Build:** `cargo build -p renderer` clean (implicit via test compile).
- **Execute:** all test commands above return expected pass counts.
- **schema_valid:** true (build green, execute green, all criteria assessable).

## Assessment

**Accept: YES.** A_6 exhibits all `D_t` Validation Requirements (VR1-VR5) verifiable from candidate-bound test execution + white-box source inspection, preserves loop-05 behaviors (P1-P9, 489/0 ≥ 485 base, no regression), and closes L05-G01 (symbols.csv s015/s016 + s017/s018 with independently-verified wt_anchors). Port-faithful to WT `renderer.cpp:891-900` (TriggerScroll) + `renderer.cpp:1870-1879` (_ScrollPreviousSelection) + `IRenderEngine.hpp:73` (InvalidateScroll trait). NotifyPaintFrame wire deferred per ADR-0003; TriggerScroll() no-arg + viewport-driver deferred (out-of-scope). 

**QA-independent correction (L06-G01):** Dev's initial L05-G01 fix mis-recorded 2 wt_anchors (s015 `renderer.hpp:152` should be `:151`; s016 `IRenderEngine.hpp:69` should be `:68`) + 1 rust_path (s016 `:52` should be `:44`). QA `grep -n` cross-check caught mismatch; fixed in-place this loop. L06-G01 row logged, closed same loop. Root cause: Dev read `sed -n 'A,Bp'` block-display rather than exact `grep -n "sym"` — repeat of B_4 cross-check lesson. Ledger doc only; no build/test/behavioral impact. Issue-ledger updated.
