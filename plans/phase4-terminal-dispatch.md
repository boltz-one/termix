---
title: "Phase 4 — Port verbatim AdaptDispatch → TerminalDispatch (134 methods)"
description: "Porting WT C++ AdaptDispatch (4912 LOC + 480 LOC SGR + 211 LOC Interact) to 134 ITermDispatch method impls + 8 InteractDispatch impls in Rust."
status: pending
priority: P1
effort: 40h
lane: high-risk
branch: feat/phase4-terminal-dispatch
tags: [port, vt, dispatch, adapter, lane-c]
created: 2026-10-25
---

# Phase 4 — Port verbatim `AdaptDispatch` (WT C++) → `TerminalDispatch` (Rust)

## Overview

WT source (`windows/src/terminal/adapter/`):
- `adaptDispatch.cpp` (4912 LOC, 134 ITermDispatch method impls + ~30 private helpers).
- `adaptDispatchGraphics.cpp` (480 LOC, SGR/graphics).
- `InteractDispatch.cpp` (211 LOC, 8 `IInteractDispatch` impls).
- `adaptDispatch.hpp` (struct decl; member fields: `_api`, `_renderer`, `_renderSettings`, `_terminalInput`, `_termOutput`, `_pages`, `_sixelParser`, `_fontBuffer`, `_macroBuffer`, `_initialCodePage`, `_optionalFeatures`, `_savedCursorState[2]`, `_usingAltBuffer`, `_scrollMargins`, `_modes`, `_sgrStack`, `_tabStopColumns`).

Target (`crates/core/src/dispatch/`): trait `ITermDispatch` (123 methods + stub, `mod.rs`), `InteractDispatch` (8, `interact.rs`). Target API surface: `buffer.rs` `TextBuffer/Row`, `cursor.rs` `Cursor`, `screen.rs` `Viewport`, `attrs.rs` `TextAttribute`, `color_table.rs`, `sgr_stack.rs` `SgrStack`, `parser.rs` `OutputStateMachineEngine::new(Box<dyn ITermDispatch>)` (vte wiring gate).

Goal: count `todo!()` in `crates/core/src/dispatch/` drops 134 → 0; `cargo test --workspace` stays green; e2e `"hello\nworld"` fed via `vte::Parser` renders to `TextBuffer.rows[0]="hello"`, `rows[1]="world"`.

## Phase 4.0 — Prerequisite API gaps (BLOCKER, serial, before any worker)

Fill TextBuffer/Cursor/InteractDispatch-trait gaps (port from `windows/src/buffer/out/textBuffer.cpp` + `cursor.h` + `IInteractDispatch.hpp`). Adapter cannot compile verbatim without these.

**Files created / modified**:
- EDIT `crates/core/src/buffer.rs` — add the methods below.
- EDIT `crates/core/src/doc/cursor.rs` — N/A (cursor already sufficient).
- EDIT `crates/core/src/dispatch/interact.rs` — extend trait with missing 8 methods (default `todo!()`).

**TextBuffer new methods** (anchor: `textBuffer.cpp`):
| Method | Anchor | LOC | Used by |
|---|---|---|---|
| `scroll_rows(first_row, size, delta)` | `:806` | 60 | `_ScrollRectVertically` (W3 scroll), `ScrollUp/ScrollDown`, `InsertLine/DeleteLine` |
| `get_last_non_space_character(view: Option<&Viewport>) -> (i32,i32)` | `:760` | 35 | `_EraseAll` (W4), `HardReset` (W4) |
| `clamp_position_within_line(pos: (i32,i32)) -> (i32,i32)` | `:967` | 5 | `_CursorMovePosition` (W1) — critical |
| `reset_line_rendition_range(start, end)` | `:938` | 8 | `_ScrollRectVertically`, `EraseInDisplay` (W2) |
| `clear_scrollback(new_first_row, rows_to_keep)` | `:999` | 35 | `_EraseScrollback` (W4) |
| `trigger_scroll(delta: (i32,i32))` | `:1121` | 10 | `_EraseAll` (W4) |
| `set_current_line_rendition(rend, attr)` | `:914` | 15 | `SetLineRendition` (W2) |
| `copy_row(src_idx, dst_idx, dst_buffer)` | `:876` | 25 | `CopyRectangularArea` (W2) |

Row gaps: existing `Row` covers all needs (no add). Cursor gaps: existing `Cursor` covers; only add `CursorState` struct (port `cursor.h:CursorState`) for `_savedCursorState[2]` field — store in `crates/core/src/dispatch/td/mod.rs` next to `TerminalDispatch`.

**InteractDispatch trait extension** (port `IInteractDispatch.hpp:33-42`): add `set_device_attributes`, `write_input`, `write_ctrl_key`, `write_string`, `write_string_raw`, `window_manipulation`, `move_cursor`, `is_vt_input_enabled` (all default `todo!()`). Existing 8 methods kept.

**Success**: `cargo build -p core` clean + new `buffer.rs` tests pass.

## Worker Partition — Cohesive Method Groups

Impl split across files under new `crates/core/src/dispatch/td/`. File globs disjoint → parallel safe post-W1. State struct + ctor defined by W1 (foundation).

### W1 — Foundation + Cursor + Print (Wave 0, BLOCKS Wave 1)
**Owns**: `crates/core/src/dispatch/td/mod.rs`, `td/cursor.rs`, `td/print.rs`. Also: `crates/core/src/lib.rs` (mod re-export only — single-line addition).

**Struct fields** (port `adaptDispatch.hpp` member list; skip PageManager/Sixel/Font/Macro — those are Phase 4.x):
```rust
pub struct TerminalDispatch {
    pub terminal: TextBuffer,                  // host — owns cursor + buffer.
    pub interact: Box<dyn InteractDispatch>,   // host bridge — title/clipboard/bell/resize/write_file.
    pub viewport: Viewport,                    // cached from terminal.get_size(); updated via interact resize.
    pub scroll_margins: (i32,i32,i32,i32),     // top,bottom,left,right — port _scrollMargins.
    pub using_alt_buffer: bool,                // _usingAltBuffer.
    pub saved_cursor_state: [CursorState; 2],  // _savedCursorState.
    pub tab_stop_columns: Vec<u8>,              // _tabStopColumns.
    pub init_default_tab_stops: bool,
    pub modes: ModeSet,                         // _modes; define `ModeSet` (bitflags) port til::enumset<Mode>.
    pub sgr_stack: SgrStack,                     // already defined in sgr_stack.rs.
    pub optional_features: u32,                // _optionalFeatures.
}
```

Build ctor `TerminalDispatch::new(terminal, interact)`: init `viewport=terminal.get_size()`, `scroll_margins=(0,(height).saturating_sub(1),0,(width).saturating_sub(1))`, `tab_stop_columns` prefilled every 8 cols (`_initDefaultTabStops=true`), `modes=ModeSet::PAGE_CURSOR_COUPLING`.

**Methods** (16 ITermDispatch + 2 helpers, ~750 LOC):
| Rust fn | WT anchor | LOC |
|---|---|---|
| `unknown_sequence` | `adaptDispatch.cpp:51` | 3 |
| `print` | `:62` | 10 |
| `print_string` | `:82` | 10 |
| `_write_to_buffer` (private) | `:100` | 110 |
| `cursor_up` | `:212` | 2 (1-liner) |
| `cursor_down` | `:226` | 2 |
| `cursor_forward` | `:235` | 2 |
| `cursor_backward` | `:244` | 2 |
| `cursor_next_line` | `:254` | 2 |
| `cursor_prev_line` | `:264` | 2 |
| `_cursor_move_position` (private) | `:327` | 80 |
| `cursor_horizontal_position_absolute` | `:408` | 3 |
| `vertical_line_position_absolute` | `:417` | 3 |
| `horizontal_position_relative` | `:427` | 3 |
| `vertical_position_relative` | `:437` | 3 |
| `cursor_position` | `:447` | 3 |
| `cursor_save_state` | `:458` | 35 |
| `cursor_restore_state` | `:492` | 55 |

Helpers: `_get_vertical_margins`, `_get_horizontal_margins` (sub-routines of `_cursor_move_position`) inline in W1.

### W2 — Erase + Rect-area (Wave 1, parallel)
**Owns**: `crates/core/src/dispatch/td/erase.rs`, `td/rects.rs`.

**Methods** (22 public + 5 helpers, ~1100 LOC):
| Rust fn | WT anchor | LOC |
|---|---|---|
| `_scroll_rect_vertically` (private) | `adaptDispatch.cpp:549` | 60 |
| `_scroll_rect_horizontally` (private) | `:607` | 50 |
| `_insert_delete_character_helper` (private) | `:654` | 25 |
| `insert_character` | `:677` | 10 |
| `delete_character` | `:687` | 15 |
| `_fill_rect` (private) | `:701` | 12 |
| `erase_characters` | `:713` | 25 |
| `erase_in_display` | `:735` | 60 |
| `erase_in_line` | `:793` | 35 |
| `_selective_erase_rect` (private) | `:827` | 30 |
| `selective_erase_in_display` | `:857` | 35 |
| `selective_erase_in_line` | `:892` | 35 |
| `_change_rect_attributes` (private) | `:926` | 40 |
| `_change_rect_or_stream_attributes` (private) | `:968` | 80 |
| `change_attributes_rectangular_area` | `:1049` | 50 |
| `reverse_attributes_rectangular_area` | `:1099` | 50 |
| `copy_rectangular_area` | `:1150` | 50 |
| `fill_rectangular_area` | `:1199` | 30 |
| `erase_rectangular_area` | `:1227` | 20 |
| `selective_erase_rectangular_area` | `:1243` | 15 |
| `select_attribute_change_extent` | `:1255` | 25 |
| `request_checksum_rectangular_area` | `:1281` | 75 |
| `set_line_rendition` | `:1355` | 25 |

### W3 — Scroll/Page/Modes/DSR (Wave 1, parallel)
**Owns**: `crates/core/src/dispatch/td/scroll.rs`, `td/modes.rs`, `td/dsr.rs`.

**Methods** (25 public + 4 helpers, ~1300 LOC):
| Rust fn | WT anchor | LOC |
|---|---|---|
| `device_status_report` | `:1376` | 60 |
| `device_attributes` | `:1435` | 35 |
| `secondary_device_attributes` | `:1471` | 10 |
| `tertiary_device_attributes` | `:1481` | 12 |
| `vt52_device_attributes` | `:1492` | 12 |
| `request_terminal_parameters` | `:1504` | 35 |
| `_device_status_report` (private) | `:1537` | 10 |
| `_cursor_position_report` (private) | `:1549` | 40 |
| `_macro_space_report`, `_macro_checksum_report` | `:1590,1603` | 25 |
| `_scroll_movement` (private) | `:1616` | 12 |
| `scroll_up` | `:1628` | 10 |
| `scroll_down` | `:1637` | 10 |
| `next_page` / `preceding_page` / `page_position_absolute` / `page_position_relative` / `page_position_back` | `:1647-1697` | 50 |
| `request_displayed_extent` | `:1698` | 20 |
| `_set_column_mode` (private) | `:1715` | 25 |
| `_set_alternate_screen_buffer_mode` (private) | `:1741` | 25 |
| `_mode_params_helper` (private) | `:1763` | 150 |
| `set_mode` / `reset_mode` / `request_mode` | `:1915-2055` | 165 |
| `set_keypad_mode` | `:2057` | 5 |
| `set_ansi_mode` | `:2065` | 15 |
| `set_kitty_keyboard_protocol` / `query_kitty_keyboard_protocol` / `push_kitty_keyboard_protocol` / `pop_kitty_keyboard_protocol` | `:2079-2135` | 60 |

### W4 — Bell/Tabs/Charset/Reset/Colors/OSC/Macro/Play (Wave 1, parallel)
**Owns**: `crates/core/src/dispatch/td/{bell_tabs,charset,reset,colors,window,dcs,reports}.rs`.

**Methods** (50 public + 8 helpers, ~1800 LOC):
| Group | Rust fn list | WT anchors | LOC |
|---|---|---|---|
| Lines/margins | `_insert_delete_line_helper`, `insert_line`, `delete_line`, `_insert_delete_column_helper`, `insert_column`, `delete_column`, `_do_set_top_bottom_scrolling_margins`, `set_top_bottom_scrolling_margins`, `_do_set_left_right_scrolling_margins`, `set_left_right_scrolling_margins` | `:2136-2384` | 250 |
| Bell/CR/LF | `enquire_answerback`, `warning_bell`, `carriage_return`, `_do_line_feed`, `line_feed`, `reverse_line_feed`, `back_index`, `forward_index` | `:2386-2610` | 200 |
| Tabs | `horizontal_tab_set`, `forward_tab`, `backwards_tab`, `tab_clear`, `_clear_single_tab_stop`, `_clear_all_tab_stops`, `tab_set`, `_init_tab_stops_for_width` | `:2649-2834` | 200 |
| Charset | `designate_coding_system`, `designate94_charset`, `designate96_charset`, `locking_shift`, `locking_shift_right`, `single_shift`, `accept_c1_controls`, `send_c1_controls`, `announce_code_structure` | `:2836-2987` | 150 |
| Reset | `soft_reset`, `hard_reset`, `screen_alignment_pattern`, `_erase_scrollback`, `_erase_all` | `:2990-3254` | 270 |
| Cursor style | `set_cursor_style` | `:3256` | 50 |
| Colors | `set_clipboard`, `set_color_table_entry`, `request_color_table_entry`, `reset_color_table`, `reset_color_table_entry`, `set_xterm_color_resource`, `request_xterm_color_resource`, `reset_xterm_color_resource`, `assign_color` | `:3308-3480` | 175 |
| Window | `window_manipulation`, `add_hyperlink`, `end_hyperlink`, `set_window_title`, `set_current_working_directory` | `:2613-2650,3488-3560` | 90 |
| OSC actions | `do_con_emu_action`, `do_iterm2_action`, `do_final_term_action`, `do_vs_code_action`, `do_wt_action`, `do_urxvt_action` | `:3564-4024` | 280 (each ~50; many stub through interact) |
| DCS/macro | `request_user_preference_charset`, `invoke_macro`, `request_terminal_state_report`, `restore_terminal_state`, `request_setting`, `request_presentation_state_report`, `restore_presentation_state`, report helpers `_report_color_table`, `_report_sgr_setting`, `_report_decstbm_setting`, `_report_decsrlrm_setting`, `_report_decscusr_setting`, `_report_decsca_setting`, `_report_decsace_setting`, `_report_decac_setting`, `_report_cursor_information`, `_report_tab_stops` | `:4027-4794` | 280 |
| Misc | `play_sounds`, `set_optional_features`, `define_sixel_image`, `download_drcs`, `assign_user_preference_charset`, `define_macro`, `request_user_preference_charset` | scattered | 150 |

## Parallel Execution Matrix

| Unit | Owns (globs) | Depends On | Wave | Verify |
|---|---|---|---|---|
| phase-4.0-pre (API gaps) | `core/src/buffer.rs`, `core/src/dispatch/interact.rs` | — | 0 | `cargo test -p core` |
| W1-foundation | `core/src/dispatch/td/mod.rs`, `td/cursor.rs`, `td/print.rs`, `core/src/lib.rs` | phase-4.0-pre | 0 | `cargo test -p core` |
| W0-interact-impl | `core/src/dispatch/interact_impl.rs` | — (independent of W1) | 0 | `cargo build -p core` |
| W2-erase-rect | `core/src/dispatch/td/erase.rs`, `td/rects.rs` | W1 | 1 | `cargo test -p core` |
| W3-modes-dsr | `core/src/dispatch/td/scroll.rs`, `td/modes.rs`, `td/dsr.rs` | W1 | 1 | `cargo test -p core` |
| W4-bell-colors-osc | `core/src/dispatch/td/{bell_tabs,charset,reset,colors,window,dcs,reports}.rs` | W1 | 1 | `cargo test -p core` |
| phase-4.x-wire (vte→control pump) | `control/src/control.rs::TermControl::spawn_output_thread`, `core/src/parser.rs` (csi_dispatch full match) | W1-W4 + W0 | 2 | `cargo test --workspace` + e2e `"hello\nworld"` test |

**Overlap**: W1-W4 + W0 are disjoint by file glob. `crates/core/src/lib.rs` touched only by W1 (single-line `pub mod dispatch_td;` re-export). `crates/core/src/dispatch/mod.rs` NOT modified (trait stays as default; impls go in sub-module `td::*`). `crates/core/src/parser.rs`'s `csi_dispatch` Phase-1 minimal match kept; W4 may extend via host-driven Phase-4.x.

**Max concurrency**: 3 units in Wave 1 (cap). Wave 0 has 2 units (W1 deps + W0 standalone) — both can run.

## Vte Wiring — Phase 4.x

`OutputStateMachineEngine` (`parser.rs`) already takes `Box<dyn ITermDispatch>`. Wire-up:

1. `TermControl::spawn_output_thread` (lane C, `control/src/control.rs`) constructs `TerminalDispatch::new(terminal_clone, interact)` and wraps in `Box`. Creates `OutputStateMachineEngine::new(Box::new(dispatch))`. Build `vte::Parser::new()`.
2. Drain `output_rx` channel (per `ControlCore::connection_output_handler`); for each chunk, advance bytes through parser.
3. After each chunk, fire `cx.notify()` to GPUI for repaint (`Pump` already wired in `control`; just remove `todo!()` markers).
4. `parser.rs::csi_dispatch` extended: turn Phase-1 minimal-CSI match into full `OutputStateMachineEngine.hpp:93-175` dispatch (`CUU/CUD/CUF/CUB/CNL/CPL/CUP/HVP/CHA/VPA/HPR/VPR/SU/SD/ICH/DCH/ED/EL/ECH/IL/DL/DECIC/DECDC/SGR/SC/ORC/DECSC/DECRC/SM/RM/DECRQM/DSR/DA1/DA2/DA3/TBC/HTS/CTT/CHT/CBT/DECSWL/DECDWL/DECDHL/DECSCA/XTPUSHSGR/XTPOPSGR/DECSTBM/DECSLRM/...`). Phase-4.x owns this match extension.

`InteractDispatch` impl (W0): `TerminalInteractDispatch { host: Rc<RefCell<HostContext>> }` where HostContext wraps gpui Window/clipboard/etc. Phase 4.x wires `InteractDispatch` ctor in `TermControl::new`.

## Test Strategy

**Per-worker Verify (qualify for merge to Wave 1 gate)**:
- Each worker writes ≥1 unit test per public method (e.g., `cursor_up` moves cursor N rows; `erase_in_display All` clears all rows; `set_graphics_rendition [1]` sets `intense=true`).
- `cargo build -p core` clean. `cargo test -p core` passes.
- Grep: `grep -c "todo!(" crates/core/src/dispatch/td/<this-worker-files>.rs` → 0.

**Wave-1 barrier**: aggregate `cargo test --workspace`; grep `todo!()` count in `crates/core/src/dispatch/` is now ≤8 (`interact.rs` W0 fallback) before W0 finishes; becomes 0 in `interact.rs` once W0 lands.

**End-to-end parity test** (Phase 4.x, adds to `crates/core/src/dispatch/td/mod.rs` `#[cfg(test)] mod tests`):
- Test: build `TextBuffer::new(80, 24, default)`, wrap in `TerminalDispatch`, wire `vte::Parser::new()`, feed `&b"hello\nworld"`, advance byte-by-byte. Assert `terminal.rows[0].glyphs` == `"hello"`, `terminal.rows[1].glyphs` == `"world"`, cursor at `(0, 1)` (LF → CR+LF behaviour). 
- Test: count `todo!()` in `crates/core/src/dispatch/` (all sub-files) outputs 0 (`#[test] fn no_dispatch_todo_left() { assert_eq!(grep_count, 0) }`).

**State regression**: `crates/control/src/core.rs` still has 9 `todo!()` (acceptable; brief says "control/core.rs + app/page.rs còn"). `app/src/page.rs::142` (1 todo) stays.

## Risk Assessment

1. **PageManager collapse** — WT AdaptDispatch uses `_pages.ActivePage().Cursor()/Buffer()/Width()/Top()/Bottom()` as multi-page abstraction. We collapse to single `TextBuffer` + cursor + viewport. Each method that calls `_pages` rewrite as direct `self.terminal.cursor` / `self.terminal.get_size()`. **Mitigation**: W1 docs the rewrite rule; all workers apply it.
2. **Ring buffer vs `Vec::rotate` divergence** — current `increment_circular_buffer` uses `Vec<Row>::rotate_left(1)`. WT `_firstRow++` is virtual. `_ScrollRectVertically` fast path relies on `ScrollRows` rotating just `[firstRow, firstRow+size)` not whole `Vec`. **Mitigation**: Phase 4.0 adds `scroll_rows` implementing real rotation on `Vec<Row>::rotate_within(first_row..first_row+size, delta)` (Rust std). Document but no rewrite of `increment_circular_buffer` (different semantics — full ring bump).
3. **`til::rect`/`til::point`/`til::inclusive_rect` Rust type mapping** — use `(i32, i32)` / `Viewport` natively; no `til` crate. W1 defines type aliases as needed (in `td/mod.rs`).
4. **`Offset::Forward(n)`/`Backward(n)`/`Absolute(n)`/`Unchanged()`** — `_CursorMovePosition` uses `Offset` enum. W1 defines `enum Offset { Forward(i32), Backward(i32), Absolute(i32), Unchanged }` + helpers.
5. **`_api.ReturnResponse(string)`** — every DSR/DA/report uses it. Map to `InteractDispatch::write_file(&str)` (already in trait). For `RequestColorTableEntry` etc. → also via `write_file`.
6. **OSC action methods (DoConEmu/DoITerm2/DoVsCode/DoWT/DoITerm2/DoFinalTerm/DoUrxvt)** — WT impls parse-and-call interact. Many sub-actions to port. Phase 4 leans toward "structured minimal": implement the easy sub-actions (OSC 133 FinalTerm prompt/command/output marks; OSC 9 ConEmu set clipboard) and stub the rest via `InteractDispatch::write_file` (host bridges). Keep verbatim signatures — bodies eased with `match` over payload prefix & delegate.
7. **Sixel/DRCS/Macro string handlers** — return `StringHandler` (closure). Phase 4: bodies parse via stub closure that captures string; real SixelImage rasterization deferred to Phase 4.y (paint-tier). Function returns `Some(Box::new(|_s: &str| ()))` placeholder, with minimum behavior of writing to buffer captured.
8. **`_renderSettings`** — color table reads/writes require host-side `RenderSettings` (port `RenderSettings.hpp`). Build `RenderSettings` Rust port minimal in `crates/renderer` or `settings` (out of phase-4 scope if it lands later). Interim: `TerminalDispatch` uses `interact.set_color_*` host trait methods; `RenderSettings` mock replaced by delegating to Interact host.
9. **`_terminalInput` keyboard protocol modes (Kitty)** — affects `TerminalInput` (lane C input). Phase 4 keeps adapters writing modes to `_modes` bitset only; lane C input engine (Phase 5) reads them. Phase 4 does not port `TerminalInput.cpp`.

## Open Questions

1. **`ModeSet` bitflags**: define new in `td/mod.rs` OR extend `dispatch/types.rs`? Recommendation: `td/mod.rs` (worker-local; referenced only via `td::*`).
2. **CursorState struct placement**: alongside `Cursor` (`cursor.rs`) or `td/mod.rs` (worker-local)? Inline in `td/mod.rs` (avoids cross-crate coupling).
3. **Phase 4.x wiring** — should `parser.rs::csi_dispatch` full match extension be part of this plan or separate Phase 4.x? Recommendation: split Phase 4.x out so Phase 4 itself focuses on dispatch-side method bodies; the parser match extension is `core::parser` (lane B) coordinated change. Leave in plan as Phase 4.x separated section; confirm with user pre-execution.
4. **Test verification of vte→buffer with TerminalDispatch**: should pump be tested in `core` (no GPUI) or in `control` test (real pump)? Recommendation: `core` (no GPUI dep) — dispatch impl is crate-local, test feeds `vte::Parser` directly.

## Next Steps (post-Phase-4)

- Phase 4.x: vte `csi_dispatch`/`esc_dispatch` full match extension + `TermControl::spawn_output_thread` pump connect.
- Phase 5: `TerminalInput` input handling (lane C keyboard).
- Phase 6: Sixel/DRCS rasterization pipeline.
