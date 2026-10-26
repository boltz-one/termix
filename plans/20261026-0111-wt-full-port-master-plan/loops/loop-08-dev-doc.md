---
loop: 8
status: pending
preservation_constraints:
  - p1_loop07_workspace_499            # cargo test --workspace --lib ≥498 pass / 0 fail (loop07 498 base; target ≥518)
  - p2_loop07_colorfix_8              # renderer --lib colorfix 8 pass
  - p3_loop07_settings_61             # renderer --lib settings::tests 18 (sub-suite, 61 module-wide)
  - p4_loop07_colortable256           # RenderSettings::default() 256-table unchanged
  - p5_loop07_terminal_seam_1          # cargo test -p core --test wt_terminalcore 1 pass
  - p6_loop07_wt_colorfix_3            # cargo test -p renderer --test wt_renderer_colorfix 3 pass
  - p7_loop07_renderer_tests_9        # renderer::tests 9 pass
  - p8_loop07_clippy_renderer         # cargo clippy -p renderer --all-targets clean
  - p9_loop07_core_lib_215           # cargo test -p core --lib ≥215 / 0 fail (loop07 215 base)
  - p10_loop07_clippy_core_lib       # cargo clippy -p core --lib -- -D warnings clean
  - p11_loop07_interact9_method      # 9 InteractDispatch ext method override (interact_impl.rs) intact
  - p12_loop07_phase5wave2_interact_default_intact  # interact.rs 42 default todo!() không regress (lane C defer — out-of-scope P09/P11)
  - l07g01_clippy_wt_terminalcore_pre  # P11 all-targets 4 error wt_terminalcore.rs PRE-EXISTING (L07-G01, defer)
candidate: A_8 (crates/core/src/dispatch/mod.rs fold 126 ITermDispatch trait default todo!() → no-op body lane B host-bridge; docs cite ITermDispatch.hpp + adaptDispatch.cpp; tests +20 covering ITermDispatch method groups non-panic; không touch interact.rs/interact_impl.rs)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 08 — P03 close: fold 126 `ITermDispatch` trait default `todo!()` → no-op (lane B)

## Objective

P03-close mass fold: replace 126 `todo!()` default body trong `ITermDispatch` trait (`crates/core/src/dispatch/mod.rs` 124 trait method) bằng no-op `let _ = (args,);` lane B host-bridge, mirror loop-07 lane-B pattern. One Dev cycle closes the ITermDispatch trait surface of P03 (vt-adapter-dispatch). `interact.rs` (InteractDispatch, 42 default todo, lane C defer) untouched.

## Why now (strategy shift)

Loop-02→07 ported ~30 method at 9/loop cadence (~30 method). Remaining P03 trait surface (`ITermDispatch` 124 trait method) needs 14 more loops at that pace. User decision: abandon incremental cadence; loop-08 = big-batch fold to close `mod.rs` lane-B surface in ONE cycle. Lane C concrete wiring (AdaptDispatch conhost `gci`/`_api`/`InputBuffer`/`VtIo`) still deferred P09/P11 per spec — only the trait-level no-op defaults land now.

## Context (citations verified read-only)

- **`ITermDispatch.hpp`** (`windows/src/terminal/adapter/ITermDispatch.hpp:35-200`, 124 virtual method): trait interface for VT dispatch events. Each method declared with default virtual body (C++) — Rust port mirrors `todo!()` placeholder from phase-1 skeleton. Loop-08 replaces default body, not sig.
- **`adaptDispatch.cpp`** (4912 LOC) + **`adaptDispatch.hpp`** (359 LOC, 159 method): concrete `AdaptDispatch` impl — wires `ITerminalApi` (host) + `gci` (ServiceLocator/ConsoleInformation) + `InputBuffer` + `VtIo`. **Out-of-scope conhost** lane C (P09/P11). Loop-08 only touches trait default bodies at `crates/core/src/dispatch/mod.rs:53-622` (lines 53 = `pub trait ITermDispatch {`, 622 = trait close `}`).
- **Blanket impl `Box<T>`** (`mod.rs:632-1077`, 124 method forward) — already forward-only (**no `todo!()`**, search confirmed within blanket body none). Loop-08 does NOT touch blanket (forward `(**self).method(args)` unchanged — already correct delegation).
- **`interact.rs`** (separate trait `InteractDispatch`, 42 `todo!()` defaults at `:52-85` (7 essential) + `:177-334` (33 Wave-2/OSC)) — **NOT touched by loop-08** (lane C P09/P11 defer per D_7 loop-07 closure +Out-of-scope). Single-writer perimeter: `mod.rs` only.
- **Counts verified** (`grep -cE "todo!\(" crates/core/src/dispatch/mod.rs` = **126**; trait fn count `grep -cE "^(    fn )" ...` = **248** (124 trait default + 124 blanket forward); trait body range lines 53-622 has `awk 'NR>=53 && NR<=622' ... | grep -cE "todo!\("` = **123** (1 method, `print` or similar, has implicit body — Dev to verify exact line by line when implementing, count remaining 126-123 = 3 are in test/dev-doc modules — Dev verify all 3 location).
- **Behavior groups** (qualitative — exact line mapping determined per-task below; Dev verifies each against `mod.rs` actual line):
  1. **Cursor movement** (12): `cursor_up`/`cursor_down`/`cursor_forward`/`cursor_backward`/`cursor_next_line`/`cursor_prev_line`/`cursor_horizontal_position_absolute`/`vertical_line_position_absolute`/`horizontal_position_relative`/`vertical_position_relative`/`cursor_position`/`cursor_save_state`/`cursor_restore_state` (`mod.rs:67-117`). WT `ITermDispatch.hpp:103-169` + `adaptDispatch.cpp` bodies delegate `gci.GetActiveBuffer().MoveCursor*()`.
  2. **Char/Line/Page edit** (~16): `insert_character`/`delete_character`/`scroll_up`/`scroll_down`/`next_page`/`preceding_page`/`page_position_absolute`/`page_position_relative`/`page_position_back`/`request_displayed_extent`/`insert_line`/`delete_line`/`insert_column`/`delete_column` (`mod.rs:119-173`). WT `ITermDispatch.hpp:170-194`.
  3. **Modes** (~6): `set_keypad_mode`/`set_ansi_mode`/`set_top_bottom_scrolling_margins`/`set_left_right_scrolling_margins`/`enquire_answerback` (`mod.rs:176-209`). WT `:195-215`.
  4. **Kitty keyboard** (4): `set_kitty_keyboard_protocol`/`query_kitty_keyboard_protocol`/`push_kitty_keyboard_protocol`/`pop_kitty_keyboard_protocol` (`mod.rs:184-197`). WT `:215-228`.
  5. **Bell/CR/LF/Index** (~8): `warning_bell`/`carriage_return`/`line_feed`/`reverse_line_feed`/`back_index`/`forward_index` (`mod.rs:212-230`). WT `:230-245`.
  6. **Window/Title/Clipboard/Hyperlink/OSC actions** (~13): `set_window_title`/`set_current_working_directory`/`set_clipboard`/`window_manipulation`/`add_hyperlink`/`end_hyperlink`/`do_con_emu_action`/`do_iterm2_action`/`do_final_term_action`/`do_vs_code_action`/`do_wt_action`/`do_urxvt_action` (`mod.rs:233-278`). WT `:245-280` (overlap with `InteractDispatch` — `set_window_title`/`set_clipboard`/`window_manipulation` also in `interact.rs:63/69/...`; this trait dev by `AdaptDispatch` adapter — lane C).
  7. **Tabs** (5): `horizontal_tab_set`/`forward_tab`/`backwards_tab`/`tab_clear`/`tab_set` (`mod.rs:282-300`). WT `:280-295`.
  8. **Color/SGR** (~18): `set_color_table_entry`/`request_color_table_entry`/`reset_color_table`/`reset_color_table_entry`/`set_xterm_color_resource`/`request_xterm_color_resource`/`reset_xterm_color_resource`/`assign_color`/`set_graphics_rendition`/`set_line_rendition`/`set_character_protection_attribute`/`push_graphics_rendition`/`pop_graphics_rendition` (`mod.rs:303-454`). WT `:295-340`.
  9. **Erase/Rect ops** (~12): `erase_in_display`/`erase_in_line`/`erase_characters`/`selective_erase_in_display`/`selective_erase_in_line`/`change_attributes_rectangular_area`/`reverse_attributes_rectangular_area`/`copy_rectangular_area`/`fill_rectangular_area`/`erase_rectangular_area`/`selective_erase_rectangular_area`/`select_attribute_change_extent`/`request_checksum_rectangular_area` (`mod.rs:336-434`). WT `:340-385`.
  10. **Mode/Status report** (~9): `set_mode`/`reset_mode`/`request_mode`/`device_status_report`/`device_attributes`/`secondary_device_attributes`/`tertiary_device_attributes`/`vt52_device_attributes`/`request_terminal_parameters` (`mod.rs:457-489`). WT `:385-410`.
  11. **Charset/Coding** (~9): `designate_coding_system`/`designate94_charset`/`designate96_charset`/`locking_shift`/`locking_shift_right`/`single_shift`/`accept_c1_controls`/`send_c1_controls`/`announce_code_structure` (`mod.rs:491-525`). WT `:410-430`.
  12. **Reset/Cursor/Alignment** (4): `soft_reset`/`hard_reset`/`screen_alignment_pattern`/`set_cursor_style` (`mod.rs:529-547`). WT `:430-445`.
  13. **Sixel/DCS/Macro/Presentation** (~13): `define_sixel_image`/`download_drcs`/`request_user_preference_charset`/`assign_user_preference_charset`/`define_macro`/`invoke_macro`/`request_terminal_state_report`/`restore_terminal_state`/`request_setting`/`request_presentation_state_report`/`restore_presentation_state`/`play_sounds`/`set_optional_features` (`mod.rs:547-621`). WT `:445-200`.
- **Port-faithful strategy** (lane B): every WT `ITermDispatch` default body in `adaptDispatch.cpp` depends on `_api` (ITerminalApi) or `gci` (ServiceLocator/ConsoleInformation/InputBuffer) — out-of-scope per spec conhost/host/server. Loop-08 port = **interface surface preserve + minimal no-op default**:
  - Each method: change `todo!("ITermDispatch::<Name>")` → `let _ = (arg1, arg2, ...);` (mirror loop-07 lane B no-op pattern, e.g. `interact_impl.rs:143-207` `write_input`/`write_ctrl_key`/`window_manipulation`/`move_cursor`/`focus_changed` no-op).
  - **No `HostContext` field growth** (YAGNI — lane C owns gpui/textbuffer state). `ITermDispatch` trait has no `&self.host` accessor in trait body (the impl-level struct owns state in `AdaptDispatch` adl, lane C); trait default = pure no-op, NOT host-bridge override (unlike loop-07 `InteractDispatch` where `TerminalInteractDispatch` overrode at impl-time).
  - **Header doc-comment** (`mod.rs:1-9`) note: "Phase 1 skeleton: 123 trait methods với default `todo!()` body" → update to "lane B mass no-op fold (loop-08): 124 trait method default `todo!()` → no-op; lane C `AdaptDispatch` concrete impl defer P09/P11".
- **Test pattern** (mirror loop-07 T8 scaled): unit test per behavior-group calling each method non-panic + verifying trait object state untouched. 20 test covers 13 groups (groups 4/12/13 sub-merged).

## Tasks

- **T0 — Verify exact 126 `todo!()` location + group mapping** (`mod.rs`): Dev run `grep -nE "todo!\(" crates/core/src/dispatch/mod.rs` → confirm 126 line, group by section comment header (`// Unknown / Print — port :NN,NN` etc.). If line in groups above differ from actual (drift since loop-07), Dev updates mapping in code comment (not in D_8 — task is to verify, not re-edit this doc). 3 todo OUT of trait default range 53-622 are NOT trait method (likely in `tests`/`traits` sub-block) — Dev confirms and excludes from T1-T13 scope (those are unit-test scaffolding, NOT port surface).

- **T1-T13 — Fold method-group `todo!()` → no-op** (each task = 1 behavior group from list above): 
  - T1: cursor-movement 12 method (`mod.rs:67-117`)
  - T2: char/line/page edit 14 method (`mod.rs:119-173`)
  - T3: modes 5 method (`mod.rs:176-209`)
  - T4: kitty keyboard 4 method (`mod.rs:184-197` — may overlap T3; Dev verifies)
  - T5: bell/CR/LF/index 6 method (`mod.rs:212-230`)
  - T6: window/title/clipboard/hyperlink/OSC 13 method (`mod.rs:233-278`)
  - T7: tabs 5 method (`mod.rs:282-300`)
  - T8: color/SGR 13 method (`mod.rs:303-454`)
  - T9: erase/rect 13 method (`mod.rs:336-434`)
  - T10: mode/status report 9 method (`mod.rs:457-489`)
  - T11: charset/coding 9 method (`mod.rs:491-525`)
  - T12: reset/cursor/alignment 4 method (`mod.rs:529-547`)
  - T13: sixel/DCS/macro/presentation/sound 13 method (`mod.rs:547-621`)
  
  Each T{n}: change `todo!("ITermDispatch::<Name>")` → `let _ = (arg1, arg2, ...);` (fold unused args; mirror loop-07 `interact_impl.rs:143-207` no-op pattern with `let _ = (args, ...);`). Keep doc-comment `/// port :NN + cite` (group comment header preserved). Cite `adaptDispatch.cpp:<body-line>` in inline `//` per method (where Dev can locate the cpp body via `grep -n "<MethodName>" windows/src/terminal/adapter/adaptDispatch.cpp` — Dev verify each, NOT hallucinate line). If method body in cpp depends only on `gci`/`_api` (conhost out-of-scope), note `// lane C defer P09/P11 (conhost)` per method.

- **T14 — Header doc-comment update** (`mod.rs:1-9`): replace "Phase 1 skeleton: 123 trait methods với default `todo!()` body. Lane B (parser) build trên top của trait này; Lane C (adapter) implement." → "Phase 1 complete skeleton: 124 trait method default `todo!()` → no-op (lane B mass fold loop-08). Lane C `AdaptDispatch` concrete impl defer P09/P11 (conhost out-of-scope: ITerminalApi/ServiceLocator/InputBuffer/VtIo)." Keep `windows/src/terminal/adapter/ITermDispatch.hpp:39-199` cite. Cite total 124 method explicitly.

- **T15 — Tests +20** (`mod.rs` `#[cfg(test)] mod tests`, append after existing test module): cover 13 behavior groups non-panic + no-op semantics. Mirror pattern:
  - `test_trait_method_group_cursor_movement_no_panic` — call `cursor_up(1)` + `cursor_down(2)` + `cursor_position(3,4)` + `cursor_save_state()` + `cursor_restore_state()` on a `NoopDispatch` test stub (Dev defines `struct NoopDispatch;` impl `ITermDispatch` with default inheritance, i.e. trait default inheritance test path).
  - `test_trait_method_group_char_line_edit_no_panic` — `insert_character(1)`/`delete_line(2)`/`scroll_up(3)`/`next_page(1)` etc.
  - (one per group T1-T13; groups sub-merged if overlap).
  - 13 group tests + ~7 cross-cut (default-inheritance: `NoopDispatch::default().warning_bell()` no panic; trait object via `Box<dyn ITermDispatch>`: `Box::new(NoopDispatch)` dispatch `cursor_up(1)` no panic; blanket impl `Box<T>` verify forward path not panic).
  - DO NOT use mocks/fakes — `NoopDispatch` is a real `struct` that inherits trait default by NOT overriding methods (Rust trait = default inheritance when not overridden). This gives `NoopDispatch::cursor_up(1)` calling the no-op default body — proves the fold works (was `todo!()` panic, now no-op).
  - Test pattern: 1 test reuses `RecordingDispatch` (existing test stub in `td/` or `tests/`) — Dev verifies existing test infra reuse per `workflows/orchestration-protocol.md` Reuse principle BEFORE creating `NoopDispatch` (check `grep -n "struct .*Dispatch" crates/core/src/dispatch/` + check `td/mod.rs` for stub).
  - L06-G01 + L04-G02 lesson: each cpp `:NN` cite in test doc-comment MUST be verified via `grep -n "<Method>" windows/src/terminal/adapter/adaptDispatch.cpp` BEFORE write test (one symbol → one `grep -n` verify — repeat cross-check rule).

- **T16 — ADR**: optional ADR-0017 "lane B mass no-op default trait body fold" — **defer** (YAGNI: hành vi suy ra từ spec "conhost out-of-scope" + loop-07 ADR pattern không cần re-document). Dev does NOT create ADR-0017 unless QA flags ambiguity at review.

- **T17 — Ledger P03**: P03 ledger seed (`parity/ledger/P03-vt-adapter/symbols.csv`) — **out-of-scope** mirror loop-07 T9 decision. P03 audit is separate audit loop; loop-08 chỉ fold trait default surface, không add ledger rows (P03 ledger seed khi `AdaptDispatch` concrete impl land lane C). Dev note in evidence.

## Preservation

Re-run loop-07 verified behaviors (E_7): VR1-VR7 (InteractDispatch 9 ext method override intact), P1-P11 (workspace 498/0, core lib 215/0 + 20 new loop-08 test → target ≥235/0, colorfix 8/0, settings 61/0, terminal seam 1/0, wt_colorfix 3/0, renderer::tests 9/0, clippy renderer clean, clippy core lib clean), L07-G01 (pre-existing `wt_terminalcore.rs` 4 clippy error — pre-existing, NOT loop-08 regression — defer as before), L07-G02 (informational, no action). Loop-08 single-writer perimeter = `crates/core/src/dispatch/mod.rs` ONLY. KHÔNG touch `interact.rs` (42 trait default todo!() — lane C defer P09/P11), KHÔNG touch `interact_impl.rs` (loop-07 landed — intact), KHÔNG touch `td/`, `seams.rs`, `types.rs`, `headless_host.rs`, `parser.rs`, `terminal_input/*`, `terminal/*`, renderer/control/font/settings/app/connection. L07-G01 carry-over: `cargo clippy -p core --all-targets` still 4 error at `wt_terminalcore.rs:9,10,35,100` (pre-existing, out-of-scope loop-08 perimeter).

## Validation Requirements

- **VR1 — `mod.rs` `todo!()` count → 0** — `grep -cE "todo!\(" crates/core/src/dispatch/mod.rs` returns **≤3** (target **0** if 3 outliers in test scaffolding also fold; if 3 are legitimate test `todo!()` for deferred-test-pending then keep, but trait default range 53-622 has 0 todo). Mirror loop-07 VR6 (那里的0 todo) — spec demand: trait default body no `todo!()`, all bodies no-op lane B.
- **VR2 — `ITermDispatch` trait method count = 124** — `awk 'NR>=53 && NR<=622' crates/core/src/dispatch/mod.rs | grep -cE "^(    fn |    pub fn |    fn |^\s+fn )"` returns **≥124** (one per WT `ITermDispatch.hpp:35-200` virtual method). Mirror loop-07 VR1 trait count sanity.
- **VR3 — Blanket impl `Box<T>` intact** — `awk 'NR>=632' crates/core/src/dispatch/mod.rs | grep -cE "todo!\("` returns **0** (forwarding impl unchanged — should be 0 already, verify post-fold no regression).
- **VR4 — `cargo test -p core --lib` ≥235/0** — 215 loop-07 base + 20 loop-08 new unit test → target ≥235 pass / 0 fail. All 124 trait method default body no-op (no panic on call).
- **VR5 — `cargo test --workspace --lib` ≥518/0** — 498 loop-07 base + 20 loop-08 new (in core) → ≥518 pass / 0 fail. No regression in 11 other lib crates.
- **VR6 — `cargo clippy -p core --lib -- -D warnings` clean** — 0 warning, 0 error (loop-08 no-op body `let _ = (args, ...);` may trigger `unused_variables`; trait already `#[allow(unused_variables)]` at line 53 — verify still works; if not, add `#[allow(unused_variables)]` per method).
- **VR7 — `cargo clippy -p core --all-targets -- -D warnings` clean EXCEPT L07-G01 pre-existing** — 4 error at `wt_terminalcore.rs:9,10,35,100` pre-existing (NOT loop-08 regression, QA stash-verify against loop-07 baseline commit `e78920a`). Dev cannot fix per single-writer perimeter (out-of-scope `wt_terminalcore.rs`). Status: open, defer.
- **VR8 — Phase 5 Wave 2 / `interact.rs` 42 todo!() intact** — `grep -cE "todo!\(" crates/core/src/dispatch/interact.rs` returns **42** (unchanged — out-of-scope lane C defer P09/P11). NOT fold. Mirror loop-07 P12 (33 Wave-2 + 7 essential intact).
- **VR9 — `interact_impl.rs` loop-07 landing intact** — `grep -cE "todo!\(" crates/core/src/dispatch/interact_impl.rs` returns **0** (loop-07 9 ext method override bodies preserved); `grep -n "WAIT 4.0-pre" crates/core/src/dispatch/interact_impl.rs` returns **0 match** (loop-07 WAIT-block removed). NOT regress.
- **VR10 — header doc-comment updated** — `grep -n "Phase 1 skeleton: 123 trait methods" crates/core/src/dispatch/mod.rs` returns **0 match** (old text removed — T14 update); `grep -n "lane B mass no-op fold (loop-08)" crates/core/src/dispatch/mod.rs` returns **≥1 match** (new text present).
- **VR-Preserve loop07 P1-P8** — colorfix 8/0, settings 61/0, terminal seam 1/0, wt_colorfix 3/0, renderer::tests 9/0, clippy renderer clean — re-run QA independent, NOT regression per single-writer perimeter (loop-08 only touches `crates/core/src/dispatch/mod.rs`).
- **VR-L07G01 carry-over** — `cargo clippy -p core --all-targets -- -D warnings` still 4 error at `wt_terminalcore.rs:9,10,35,100` (pre-existing, NOT loop-08 regression) — QA re-stash-verify: `git stash push -- crates/core/src/dispatch/mod.rs`, run clippy, still 4 error; `git stash pop`. Confirms loop-08 không gây P11 regression.

## Out-of-scope

- **conhost wiring** (`ITerminalApi` real, `gci`/ServiceLocator/ConsoleInformation, `InputBuffer`, `VtIo`, `CharToKeyEvents`, `WriteFocusEvent`, `CONSOLE_HAS_FOCUS`, `MoveCursor*` actual) — out-of-scope per spec (conhost/host/server). Lane C (P09/P11) wires concrete `AdaptDispatch` impl.
- **`AdaptDispatch` concrete impl** (`dispatch/mod.rs` line 632+ blanket `Box<T>` is forward-only — actual `AdaptDispatch` struct + impl `ITermDispatch for AdaptDispatch` with real host-bridge bodies) — defer P09/P11 lane C. Loop-08 only lands trait default body (lane B surface).
- **`interact.rs` 42 trait default `todo!()`** (`InteractDispatch` trait, separate from `ITermDispatch`) — lane C defer P09/P11. Loop-08 does NOT touch. Mirror D_7 loop-07 (`InteractDispatch` 9 ext method override landed; 42 default kept as lane C defer).
- **`interact_impl.rs`** — loop-07 landed, intact. Loop-08 NOT touch.
- **P03 ledger seed** (`parity/ledger/P03-vt-adapter/symbols.csv`) — out-of-scope (P03 audit is separate audit loop per loop-07 T9 decision). Loop-08 không seed.
- **`wt_terminalcore.rs` 4 clippy error** (L07-G01) — pre-existing, out-of-scope loop-08 single-writer perimeter.
- **`parser.rs`/`terminal_input/*`** (P04 vt-parser) — out-of-scope. Loop-08 only P03 trait default surface.
- **`renderer`/`control`/`font`/`settings`/`app`/`connection`** crates — not touched.
- **`windows/`** — read-only upstream, no edits.
- **ADR-0017** — defer (YAGNI; loop-07 lane B pattern + spec "conhost out-of-scope" sufficient).
- **Test infra `RecordingDispatch`/`NoopDispatch`** — Dev reuse existing stub if exists (check `td/mod.rs`); only create new if no reusable stub (Reuse principle).
