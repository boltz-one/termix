# Phase 03 — VT adapter dispatch (AdaptDispatch full port)

## Context Links
- `../phase4-terminal-dispatch.md` (worker partition W1-W4 detail — reuse, do not restate), `reports/02-*.md`
- WT: `terminal/adapter/{adaptDispatch.cpp (4912),adaptDispatchGraphics.cpp,InteractDispatch.cpp,adaptDispatch.hpp,ITermDispatch.hpp,DispatchTypes.hpp,SixelParser.cpp,FontBuffer.cpp,MacroBuffer.cpp,PageManager.cpp,terminalOutput.cpp,charsets.hpp,sgrStack,...}`; tests `terminal/adapter/ut_adapter/*` (input/mouse tests belong to P04)

## Overview
Priority P1 · Pending · Wave 1 · ~320h · Milestone M2. Port every `ITermDispatch`/`IInteractDispatch` method and helper to `crates/core/src/dispatch/**`, including the items phase4 deferred ("Phase 4.x": PageManager, Sixel, FontBuffer/DECDLD, MacroBuffer/DECDMAC, kitty keyboard, DECRQSS/DECRQM, DCS handlers, rectangular ops, color queries).

## Key Insights
- Baseline (before P01 no-op conversion): 123 trait stubs in `dispatch/mod.rs`, 15 `interact.rs`, 8 `interact_impl.rs`, 24 `td/modes.rs` (170 located sites; `cargo xtask stubs` baseline is 171 — P01 enumerates the remaining site). Trait has 123 vs WT 134 methods → ledger gap audit in step 1.
- Mode arms need `TerminalInput`/render state handles → seam traits frozen in P01 (`dispatch/seams.rs`); implementers: `InputModeSink` P04, `RenderModeSink` + `ITerminalApi` P09 (see Architecture).

## Requirements
- Functional: behaviour identical to `adaptDispatch.cpp` for all CSI/ESC/OSC/DCS paths incl. alt buffer, margins, tab stops, charset (G0-G3/GR), SGR stack, DECSTBM/DECSLRM, scrolling, erase, DECCRA/DECFRA/DECERA/DECSERA/DECCARA/DECRARA, DECRQCRA checksum, device attributes, status reports, sixel, soft fonts, macros, window manipulation, title stack, clipboard (OSC 52), hyperlinks (OSC 8), marks (OSC 133), progress (OSC 9;4).
- Non-functional: no `todo!`; files <200 LOC (split `td/*`); `ITermDispatch` method count == WT.

## Architecture
`TerminalDispatch` (state: modes, margins, saved cursor, tab stops, pages, sgr stack) implements `ITermDispatch`; host callbacks through `InteractDispatch`/`ITerminalApi`. Mode side-effects go through two seam traits (defined and frozen by P01 in `dispatch/seams.rs`): `InputModeSink` (cursor-key, keypad, mouse modes, kitty flags, auto-repeat, focus, alt-scroll) and `RenderModeSink` (screen reverse, cursor style/color, palette). Implemented by P04 (`InputModeSink`, in `terminal_input/**`) and P09 (`RenderModeSink`, `ITerminalApi`, in `terminal/**`); P03 only consumes them via `Box<dyn ...>`.

## Related Code Files
- Modify/Create: `crates/core/src/dispatch/**` (`headless_host.rs` (owned from wave 1), `td/*`, new `pages.rs`, `sixel.rs`, `font_buffer.rs`, `macro_buffer.rs`, `buffer_ext.rs`, `kitty.rs`; `seams.rs` is created+frozen by P01, P03 may not change signatures), tests `crates/core/tests/wt_adapter*.rs`, `parity/golden/vt-*`.
- Delete: trait default `todo!` bodies.

## Implementation Steps
1. Ledger: enumerate WT 134 methods vs trait; add missing as `symbols.csv` rows.
2. Port ut_adapter tests first (excluding input/mouse tests → P04; 74+ methods, data rows) as red tests (`wt_adapter__*`), constructing `TerminalDispatch` with P01's `HeadlessHost` (WT `TestGetSet` semantics) until P09 provides the real `Terminal`.
3. Execute phase4 waves W1→W4 (foundation+cursor+print; modes+charset+tabs; erase+scroll+rect; reset+dsr+graphics), then 4.x items (pages, sixel, font, macro, kitty, OSC set).
4. InteractDispatch 8+7 methods (`interact_impl.rs`).
5. Golden streams: vttest menus 1-11 feeds → buffer dump compared to WT-recorded dumps (`parity/golden/vt-*`).
6. Remove all `todo!`/`// STUB`; rename PascalCase fns to snake_case and remove the temporary `#![allow(non_snake_case)]` line from `core/src/lib.rs` (clippy stays clean).

## Todo List
- [ ] method-count ledger diff
- [ ] ut_adapter port (red)
- [ ] W1-W4 + 4.x groups green
- [ ] interact dispatch
- [ ] goldens, 0 `todo!`

## Success Criteria
- `cargo xtask stubs --path crates/core/src/dispatch` reports 0 (no `todo!`, `unimplemented!`, `// STUB`).
- Every `tests.csv` row for `terminal/adapter/ut_adapter` except the input/mouse tests (`inputTest.cpp`, `MouseInputTest.cpp` → P04) is `passing` or `waived`+ADR.
- Given `printf` of vttest screen-1 stream, When fed through parser→dispatch, Then buffer dump equals golden.
- `symbols.csv` adapter rows all `verified` by audit review.

## Test Strategy & Quality Gate
- G/W/T: Given a fresh `TerminalDispatch` 80x25, When CSI `1;3H` is dispatched, Then cursor is at (row0,col2) per WT adapter test values.
- E2E scenario "vttest-cursor-movements" golden compare.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 03` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p core -- wt_adapter__`, `cargo xtask stubs`, `cargo xtask tests-check`.

## ADR Rationale
Context: largest correctness surface. Decision: verbatim structural port (same method boundaries/helpers) with Rust ownership instead of `ITerminalApi` COM. Why over redesign: parity is the acceptance criterion and WT tests encode helper-level expectations; redesign would invalidate them.

## Risk Assessment
Volume (4.9k LOC) → file-size/ownership splits; sixel/DECDLD heavy; subtle margin/wrap semantics → rely on ported tests + goldens.

## Security Considerations
Untrusted byte stream: bound DCS/OSC payload sizes (as WT), sixel image dimension limits, clipboard write (OSC 52) policy via settings, no unbounded alloc; fuzz target for dispatch (`cargo fuzz` optional, run in P19).

## Parallelization Info
Wave 1 batch A with P02/P04/P05. Internal sub-parallelism: W2-W4 file groups disjoint under `dispatch/td/`.

## File Ownership
`crates/core/src/dispatch/**`, `crates/core/tests/wt_adapter*.rs`, `parity/golden/vt-*`.

## Conflict Prevention
Needs buffer API beyond P01's surface → implement as `impl TextBuffer` extension in P03-owned `dispatch/buffer_ext.rs` (never edit `buffer/**`; P08, which depends on P03, folds it into `buffer/**`); seams are traits so P04/P09/P10 implement without editing `dispatch/**`.

## Next Steps
P09 consumes dispatch; P19 reviews 100% of adapter rows.
