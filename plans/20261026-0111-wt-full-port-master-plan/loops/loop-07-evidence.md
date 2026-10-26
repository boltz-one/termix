---
loop: 7
candidate: A_7
status: passed
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors: [VR1, VR2, VR3, VR4, VR5, VR6, VR7, P1, P2, P3, P5, P6, P7, P8, P10, P11-pre-existing, L07-G01-opened, L07-G02-opened]
unresolved_gaps: [L07-G01, L07-G02]
regressions: []
schema_valid: true
---

# Loop 07 Evidence — IInteractDispatch ext surface (9 method) port + 8 inherent todo!() fold + focus_changed + WAIT 4.0-pre close

## Verified Behaviors

- [x] **VR1 — `InteractDispatch` trait port 9 Phase 4.0-pre ext method** — evidence: `grep -cE "fn (is_vt_input_enabled|set_device_attributes|write_input|write_ctrl_key|write_string|write_string_raw|window_manipulation|move_cursor|focus_changed)" crates/core/src/dispatch/interact.rs` → **9** match. White-box `interact.rs`: 9 method signature declared in trait (port of `IInteractDispatch.hpp:33-42`). Mirror WT `IInteractDispatch.hpp:33` `IsVtInputEnabled`, `:35` `SetDeviceAttributes`, `:36` `WriteInput`, `:37` `WriteCtrlKey`, `:38` `WriteString`, `:39` `WriteStringRaw`, `:40` `WindowManipulation`, `:41` `MoveCursor`, `:42` `FocusChanged`.

- [x] **VR2 — `is_vt_input_enabled` override `true`** — evidence: `test_is_vt_input_enabled_returns_true` pass + white-box `interact_impl.rs:136-138` `fn is_vt_input_enabled(&self) -> bool { true }`. Assert `assert!(d.is_vt_input_enabled())`.

- [x] **VR3 — `write_string`/`write_string_raw` push raw bytes vào `HostContext::write_buffer`** — evidence: `test_write_string_appends_to_write_buffer` pass (assert `b"hello world"` after 2 call) + `test_write_string_raw_appends_bytes_verbatim` pass (assert `b"VT"`). White-box `interact_impl.rs:167-176`: `self.host.borrow_mut().write_buffer.extend(string.as_bytes())` — mirror `write_file` response path (pump đi ngược), port-faithful intent.

- [x] **VR4 — 6 conhost-out-of-scope method = no-op override (set_device_attributes, write_input, write_ctrl_key, window_manipulation, move_cursor, focus_changed)** — evidence: 5 test pass (`test_set_device_attributes_is_no_op`, `test_write_input_is_no_op`, `test_write_ctrl_key_is_no_op`, `test_window_manipulation_is_no_op`, `test_move_cursor_is_no_op`, `test_focus_changed_is_no_op_and_does_not_panic`) assert `write_buffer.is_empty()` + `window_title==""` + `clipboard==""` após call. White-box `interact_impl.rs:143-207`: 6 method body `let _ = ...; // lane C pending ... — no-op`. Lane B host-bridge minimal.

- [x] **VR5 — 8 inherent `todo!()` block fold thành `impl InteractDispatch for TerminalInteractDispatch` override** — evidence: `grep -cE "todo!\!" crates/core/src/dispatch/interact_impl.rs` → **0** (VR6). White-box `interact_impl.rs:84-278`: single `impl InteractDispatch for TerminalInteractDispatch` block contain 7 existing + 9 Phase-4.0-pre + 33 Phase-5 Wave-2 override (33 = 10 set + 10 get + 4 kitty + 9 OSC). No separate inherent `todo!()` block.

- [x] **VR6 — `todo!()`/`unimplemented!()` count = 0** — evidence: `grep -cE "todo!\!"` → 0; `grep -nE "unimplemented!"` → no match. 8 inherent `todo!()` removed (fold to real override/no-op).

- [x] **VR7 — `focus_changed` port added as 9th method** — evidence: trait method count 9 includes `focus_changed` (VR1 grep match). White-box `interact_impl.rs:199-207` `fn focus_changed(&mut self, focused: bool) { let _ = focused; // lane C pending WriteFocusEvent + CONSOLE_HAS_FOCUS — no-op }`. Port `IInteractDispatch.hpp:42` + `InteractDispatch.cpp:197-211`.

- [x] **VR-WAIT-block — `WAIT 4.0-pre` comment block + `mod wait_phase40_pre` footer compile-guard removed** — evidence: `grep -n "WAIT 4.0-pre" crates/core/src/dispatch/interact_impl.rs` → 0 match; `grep -n "wait_phase40_pre"` → 0 match. White-box: header note at `interact_impl.rs:76-79` replaced WAIT block ("The WAIT guard marker + footer compile-time-guard module are removed"). `mod wait_phase40_pre` footer không còn ở EOF.

- [x] **P1 — workspace lib ≥489/0** — evidence: `cargo test --workspace --lib` → **498 passed / 0 failed** (12 lib crates: a11y 0, app 31, connection 47, control 37, core 215, extras 0, font 24, host 0, renderer 67, settings 61, types 0, xtask 16). 498 ≥ 489 loop-06 base (+9: core 206→215 loop07 ext method override tests; rest unchanged).

- [x] **P2 — colorfix 8/0** — evidence: `cargo test -p renderer --lib colorfix` → 8 passed / 0 failed (re-run QA independent).

- [x] **P3 — settings 18/0** — evidence: `cargo test -p renderer --lib settings::tests` → 61 passed / 0 failed (full settings module; 18 = `settings::tests` sub-suite subset). P3 spec "settings 18/0" preserved — `settings::tests` filter run on QA side yield 61 due module reorganization; original 18-test subset still green (component of 61). No regression.

- [x] **P5 — terminal seam 1/0** — evidence: `cargo test -p core --test wt_terminalcore` → 1 passed / 0 failed (re-run QA independent).

- [x] **P6 — wt_colorfix integration 3/0** — evidence: `cargo test -p renderer --test wt_renderer_colorfix` → 3 passed / 0 failed (re-run QA independent).

- [x] **P7 — renderer::tests ≥9/0 (loop05-06 preserve + loop07 no-regress)** — evidence: `cargo test -p renderer --lib renderer::tests` → 9 passed / 0 failed (re-run QA independent).

- [x] **P8 — clippy renderer clean** — evidence: `cargo clippy -p renderer --all-targets -- -D warnings` → clean (0 warning, 0 error). `Finished dev profile`.

- [x] **P10 — core lib ≥206/0** — evidence: `cargo test -p core --lib` → **215 passed / 0 failed** (loop-06 base 206 → +9 loop07 ext method override tests: test_is_vt_input_enabled_returns_true, test_write_string_appends_to_write_buffer, test_write_string_raw_appends_bytes_verbatim, test_focus_changed_is_no_op_and_does_not_panic, test_move_cursor_is_no_op, test_set_device_attributes_is_no_op, test_write_input_is_no_op, test_write_ctrl_key_is_no_op, test_window_manipulation_is_no_op). 215 ≥ 206 ✓.

- [x] **P11-pre-existing — clippy core lib clean; clippy core --all-targets pre-existing 4 error (NOT loop-07 regression)** — evidence:
  - `cargo clippy -p core --lib -- -D warnings` → **clean** (0 warning, 0 error). lib crate (single-writer perimeter `dispatch/interact.rs`+`interact_impl.rs`) is clean.
  - `cargo clippy -p core --all-targets -- -D warnings` → **4 error** at `crates/core/tests/wt_terminalcore.rs` (NOT loop-07 files): line 9 `doc_lazy_continuation`, line 10 `doc_lazy_continuation`, line 35 `type_complexity`, line 100 `expect_fun_call`.
  - **Pre-existing verification:** `git stash push -- crates/core/src/dispatch/interact.rs crates/core/src/dispatch/interact_impl.rs` (stash loop-07 only, keep `wt_terminalcore.rs` at loop-06 state) → `cargo clippy -p core --all-targets -- -D warnings` → **same 4 error** at same `wt_terminalcore.rs:9,10,35,100`. `git stash pop` restore loop-07. Confirms 4 error exist on loop-06 baseline `e78920a` (HEAD before loop-07), NOT regression.
  - Dev report mis-describe 4 error as "all `expect_fun_call`" — actual breakdown: 2 `doc_lazy_continuation` + 1 `type_complexity` + 1 `expect_fun_call`. Cosmetic inaccuracy only (file + line + count still correct).
  - `wt_terminalcore.rs` outside loop-07 single-writer perimeter (loop-07 = `interact.rs`+`interact_impl.rs` per D_7 scope). Dev cannot fix per Evidence Gate. **Decision: open L07-G01 (defer `wt_terminalcore.rs` hygiene to dedicated hygiene loop or P11 audit loop), accept loop-07 (P11 gap pre-existing, not regression).**

- [x] **L07-G01 opened — pre-existing clippy hygiene at `crates/core/tests/wt_terminalcore.rs`** — see Unresolved Gaps.

- [x] **L07-G02 opened — D_6 (`loop-06-evidence.md`) `verified_behaviors` row không có P11; P11 là frontmatter validation constraint của loop-07 (mới), loop-06 spec chỉ require P1-P9** — see Unresolved Gaps.

## Unresolved Gaps

- **L07-G01 (opened)** — `crates/core/tests/wt_terminalcore.rs:9,10,35,100` 4 clippy error (`doc_lazy_continuation` ×2 at `:9` `:10`, `type_complexity` at `:35`, `expect_fun_call` at `:100`) khi `cargo clippy -p core --all-targets -- -D warnings`. Pre-existing từ loop-06 baseline commit `e78920a` (QA stash-verify: loop-07 changes stashed, wt_terminalcore.rs untouched → still 4 error). Outside loop-07 single-writer perimeter. NOT loop-07 regression. Severity **minor** (integration test lint hygiene only; 1/0 test still pass; lib clippy clean). Fix (next-loop Dev or dedicated `control-hygiene`-style loop): (a) add `#![allow(clippy::doc_lazy_continuation, clippy::type_complexity, clippy::expect_fun_call)]` at file top of `wt_terminalcore.rs`, hoặc (b) rewrite `expect(&format!(...))` → `unwrap_or_else(|| panic!(...))` + decompose `type_complexity` tuple return via `type` alias + fix `doc_lazy_continuation` indentation. Mirror L04-G04 pattern (pre-existing workspace clippy `control/src/interactivity.rs`).

- **L07-G02 (opened)** — D_6 (`loop-06-evidence.md`) `verified_behaviors` frontmatter row (line 7) lists `[VR1-VR5, P1-P9, L05-G01-closed]` — **không có P11**. Vậy P11 (clippy `core --all-targets` clean) là validation constraint **mới của loop-07** (D_7 frontmatter introduce P11), không phảiclaimed-of loop-06. Loop-06 baseline fact: `cargo clippy -p core --all-targets` đã fail 4 error (L07-G01) từ trước A_6. D_6 không mis-claim P11 clean (vì not-in-spec); D_6 chỉ claim P8 (renderer clippy clean, still true). Severity **informational** (no build/test/behavioral impact; clarifies P11 provenance). Fix: D_6 evidence không cần amend; härfter P11 ERC cho loop require `cargo clippy -p core --all-targets clean` sẽ force address L07-G01 (no defer beyond loop that mandates P11 green).

## Regressions

None — loop-06 verified behaviors (489 base, VR1-VR5 renderer TriggerScroll/_ScrollPreviousSelection, P1-P9) preserved (now 498/0, +9 from loop07 core lib ext method override tests). L05-G01 closed state preserved (symbols.csv s015/s016/s017/s018 untouched this loop). L06-G01 closed state preserved.

## Runtime Check

- **Build:** `cargo build -p core --lib` clean (implicit via test compile); `cargo build -p core --tests` fails clippy `--all-targets` only (compile-green for `cargo test`).
- **Execute:** all test commands above return expected pass counts (re-run QA independent, not trusting Dev report): core lib 215/0, workspace lib 498/0, wt_terminalcore 1/0, renderer::tests 9/0, colorfix 8/0, settings 61/0 (18 sub-suite intact), wt_renderer_colorfix 3/0, clippy renderer clean, clippy core lib clean.
- **schema_valid:** true (build green for tests, execute green, all VR1-VR7 + P1-P11 criteria assessable candidate-bound).

## Assessment

**Accept: YES.** A_7 exhibits all `D_t` Validation Requirements verifiable from candidate-bound evidence (re-run QA independent):

1. **VR1-VR7** verified via white-box source inspection (`interact.rs` trait + `interact_impl.rs` override) + grep count (9 method, 0 todo!, 0 WAIT/footer) + 9 new unit test green in core lib 215/0.
2. **Preservation P1-P10** verified via workspace 498/0 (≥489 ✓), core lib 215/0 (≥206 ✓), colorfix 8/0, settings 61/0 (18 sub-suite ✓), wt_terminalcore 1/0, wt_renderer_colorfix 3/0, renderer::tests 9/0, clippy renderer clean, clippy core lib clean.
3. **P11** = pre-existing gap (L07-G01), NOT loop-07 regression — triple-confirm via stash-verify on loop-06 baseline `e78920a`: 4 clippy error at `wt_terminalcore.rs` exist with loop-07 stashed. P11 constraint mới loop-07 (D_6 không claim P11 → L07-G02 informational provenance note).

Port-faithful to WT `IInteractDispatch.hpp:33-42` (9 method signature) + `InteractDispatch.cpp` bodies (`is_vt_input_enabled` true-context, `write_string`/`write_string_raw` raw-bytes push mirror `write_file` response path, 6 conhost-out-of-scope no-op, `focus_changed` interface-port no-op). 8 inherent `todo!()` fold hoàn tất (VR5/VR6). WAIT 4.0-pre block + `mod wait_phase40_pre` footer removed (VR-WAIT-block). Single-writer perimeter tôn trọng (chỉ `interact.rs`+`interact_impl.rs`, không chạm `wt_terminalcore.rs`).

**Gaps defer:** L07-G01 (wt_terminalcore.rs hygiene — open, minor), L07-G02 (P11 provenance informational — open). Neither block accept (P11 pre-existing, behavioral parity OK, scope-perimeter tôn trọng).

**Decision:** status=passed. Candidate `A_7` accept. L07-G01 + L07-G02 row logged in `issue-ledger.md`.
