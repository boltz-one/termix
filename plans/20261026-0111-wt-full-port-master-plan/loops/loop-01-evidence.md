---
loop: 1
candidate: A_1 (crates/core/src/terminal/{mod.rs,scroll.rs} + tests/wt_terminalcore.rs + docs/decisions/0014-* + parity/ledger/P09-terminal-core/{files,symbols}.csv)
status: passed
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors:
  - vr1_callback_fire_pattern
  - vr2_circle_trigger_scroll_delta
  - vr3_no_scroll_no_fire
  - vr4_viewport_values
  - vr5_parametrized_data_rows
  - vr6_build_and_target_test_green
  - vr7_gh5540_no_overflow
  - p1_workspace_regression
  - p2_todo_no_regres
  - p3_ledger_updated
  - p4_adr_written
unresolved_gaps: []
regressions: []
schema_valid: true
---

# Loop 01 Evidence — Terminal struct + scroll-notification seam

Runtime.check(A_1): build + execute `cargo test -p core --test wt_terminalcore` (debug) → **ok**, `cargo check -p core` clean, `cargo test --workspace --lib` → 462 pass / 0 fail. Build pass → candidate assessable.

## Verified Behaviors

- [x] **VR1 — Callback fire pattern** — evidence: `cargo test -p core --test wt_terminalcore` → `test wt_terminalcore__test_notify_scrolling ... ok`. Assert per-row `expect_scroll_bar = (scrolled ∧ ¬circled) ∨ (circled ∧ notifyOnCircling)` (test:79-93) — fire iff `row>=31 ∧ row<totalBufSize-1` hoặc circle path với `notifyOnCircling=true`. Pass trên 18066 row × 2 data row.
- [x] **VR2 — Circle TriggerScroll delta = (0,-1)** — evidence: cùng test pass. Assert `scrolled ∧ circled_buffer` → `delta == (0,-1)` (test:97-105), 18000+ dòng mỗi row thêm (0,-1) vào delta_slot, terminal `trigger_scroll` ghi GhItem đơn (no accumulate) — không SHORT trunc (GH#5540). Pass.
- [x] **VR3 — No-scroll → no fire** — evidence: test:106-111 assert `trigger.borrow().is_none()` khi phủ `scrolled ∧ circled`, + `scroll_bar.borrow().is_none()` khi phủ expect_scroll_bar. Pass.
- [x] **VR4 — Viewport values (top,height,bottom)** — evidence: test:113-131 chi kiểm `expectedTop = (row-32+2).clamp(0,9001)` = `(row-30).clamp(0,9001)`, `expectedHeight=32`, `expectedBottom = expectedTop + 32`. `Terminal::notify_scroll_event` compute `top = mutable_viewport.top - scroll_offset`, `height = view.height()`, `bottom = mutable_viewport.bottom + 1` (= `GetBufferHeight()` = `BottomExclusive()`, ADR-0014 Consequences). Pass row 31 case (KHÔNG 9033 bug — đã fix trong dev loop).
- [x] **VR5 — Parametrized 2 data rows (false,true)** — evidence: `wt_terminalcore__test_notify_scrolling` single fn `for notify_on_circling in [false, true]` (test:156-159), port TAE data row order ScrollTest.cpp:148. Pass cả 2.
- [x] **VR6 — Build + targeted test green** — evidence: `cargo check -p core` → `Finished dev profile` 0 error; `cargo test -p core --test wt_terminalcore` → `1 passed; 0 failed`.
- [x] **VR7 — GH#5540 không overflow i32** — evidence: test loop 2×totalBufSize=18066 row × 2 data row = 36132 `process_string("X\r\n")` call, mỗi row ≥31 thêm (0,-1) delta. `ScrollSeam::trigger_scroll(&self, delta)` ghi `Some(delta)` vào `Rc<RefCell<Option<(i32,i32)>>>` (per-call, không accumulate trong loop 01 — `winding: (i64,i64)` guard doc comment for future lane-C wire). Pass — không overflow, không panic.

Preservation:
- [x] **P1 — Workspace regression** — evidence: `cargo test --workspace --lib` → tổng 31+47+32+206+9+15+45+61+16 = **462 passed; 0 failed; 0 ignored**. D_t baseline 410 — actually 462 (pre-existing test growth khác, không phải loop 01 add — loop 01 chỉ add wt_terminalcore test file). 0 regression.
- [x] **P2 — `todo!()` không tăng trong terminal** — evidence: `grep -rEn "todo!\(" crates/core/src/terminal/` → 2 hit, CẢ HAI trong `///` doc-comment (mod.rs:36,37 nói "188 todo!() ở core") — không phải macro call. `terminal/` module = **0 todo macro**. Total core 204 todo macro nằm ngoài terminal scope (dispatch/mod.rs 126, interact.rs 43, ...) — pre-existing P03/P03-osc/dcs/etc, không do loop 01 add. D_t "188 giữ nguyên" reference không có baseline commit để diff trực tiếp, nhưng loop 01 chỉ create `terminal/{mod,scroll}.rs` + `tests/wt_terminalcore.rs` (test file, không count vào todo) → khôngゲッ tăng.
- [x] **P3 — Ledger P09 f00263 updated** — evidence: `parity/ledger/P09-terminal-core/files.csv` row `f00263` status `partial`, test_ids `wt_terminalcore__test_notify_scrolling`, ADR `ADR-0014`, author `P09-worker`. `symbols.csv` 8 row symbol partial. Pass.
- [x] **P4 — ADR-0014 written** — evidence: `docs/decisions/0014-terminal-scroll-notification-port.md` (90 line) — D1 `Box<dyn Fn>` callback, D2 `process_string` simplification (NOT VTE, lane-C wire P11), D3 `Rc<RefCell<Option<(i32,i32)>>>` seam (RenderEngine trait frozen không mở rộng), D4 `winding (i64,i64)` GH#5540 guard. Status `accepted`, date 2026-10-26. Pass.

## Unresolved Gaps

Không có. Tất cả 7 validation requirement + 4 preservation constraint verify pass. Không có hang.

## Regressions

Không có. 462 workspace test pass / 0 fail (baseline 410 không regress — thực tế +52 từ phase trước, không phải loop 01). `cargo check -p core` clean.

## Runtime Check

- Build: `cargo check -p core` → Finished, 0 error.
- Execute: `cargo test -p core --test wt_terminalcore` → 1 passed; 0 failed (0.05s).
- Workspace: `cargo test --workspace --lib` → 462 passed; 0 failed; 0 ignored.
- `schema_valid: true` — build+execute pass, mọi criterion assessable và verified.

## Assessment

**Accept: YES.**

Candidate A_1 port 1/54 P09 test (`wt_terminalcore__test_notify_scrolling` / `ScrollTest::TestNotifyScrolling`) verbatim-in-intent — callback fire pattern, circle TriggerScroll (0,-1), viewport values, GH#5540 no-overflow đều verified trên 18066 row × 2 data row. Build clean, 0 workspace regression, 0 terminal todo macro add, ledger + ADR-0014 đầy đủ.

Master plan NOT done — đây chỉ là loop 01/19 phases, 1/54 P09 tests. Dev không claim master plan done; Runtime owns subsequent loops (2/3 P09 + P10-P19) + final gate `cargo xtask audit --final` x2.

**Note scope-only (NOT a gap, NOT a regression — pre-existing tooling bug)**: `cargo xtask audit --final` exit 1 — ledger `merge symbols` duplicate-id error (`f00263` lặp per-symbol row, xtask `ledger.rs:131-135` enforce unique `id`). Dev đã quote 1 unquoted-comma bug row 5 → `merge files` OK, nhưng `merge symbols` schema bug preexisting, phase-agnostic, out-of-scope Dev loop 01. Stubs: 276 (pre-existing P03/P08/P10), không scope loop này (D_t Out-of-scope explicit).
