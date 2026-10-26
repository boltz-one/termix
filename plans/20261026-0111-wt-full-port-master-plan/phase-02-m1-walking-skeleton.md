# Phase 02 — M1 Walking Skeleton (shell end-to-end)

## Context Links
- `reports/02-parity-ledger-and-audit-loop.md`; existing `crates/app/src/{app,page,pane,tab,downstream}.rs`, `crates/control/src/{control,core}.rs`, `crates/connection/src/conpty.rs`
- WT: `cascadia/TerminalControl/ControlCore.cpp`, `cascadia/TerminalApp/TerminalPage.cpp` (`_MakePane`), `cascadia/WindowsTerminal/main.cpp`

## Overview
Priority P1 · Pending · Wave 1 · ~80h · Milestone M1. Thin vertical slice: ConPTY → parser → TextBuffer → renderer → GPUI window, keyboard in. Quality is "works", parity deferred to later phases (they replace internals behind the same seams).

## Key Insights
- Blocker today: `TerminalPage::_make_pane` is `todo!()`; `apps/terminal` empty; pump spawn deferred to `TermControl` layer (`cx.spawn`).
- Print/cursor/erase/scroll dispatch already exist, enough for a prompt.

## Requirements
- Functional: window with one tab/pane running default shell (`pwsh`/`cmd`); text output, typed input, Enter/Backspace/arrows, resize → ConPTY resize, process exit closes/ marks pane.
- Non-functional: no panics on unimplemented VT (unimplemented dispatch methods log + no-op in the skeleton only; removed by P03).

## Architecture
`apps/terminal` (bin) → `app::app::run()` → `TerminalPage` → `Pane` → `TermControl` (entity) owns `ConptyConnection`, output pump (`cx.spawn` + crossbeam), `core` parser+dispatch+buffer, `renderer` paint. Seams to keep stable for later phases: `ITerminalConnection`, `TermControl::new(connection, settings)`, `RenderData` payload.

## Related Code Files
- Modify: `crates/app/src/{app.rs,downstream.rs,page/mod.rs}`, `crates/control/src/{control.rs,core.rs}`, `apps/terminal/src/main.rs` (stub created by P01).
- Create: `crates/control/tests/e2e_shell_smoke.rs` (uses the existing `connection` ConPTY API frozen at wave start).
- Delete: `stub-downstream` feature path + stub `TermControl` in `downstream.rs` once real path works.

## Implementation Steps
1. Wire `_make_pane` → `TermControl::new(connection, settings)`; remove `todo!`.
2. Output pump: connection bytes → `vte` parser → dispatch → buffer; notify GPUI to repaint.
3. Key input → `control::keymap` → `write_input` to ConPTY (basic set).
4. Resize: viewport cells from font metrics → `ConptyConnection::resize`.
5. Exit handling; `apps/terminal` main; smoke test spawning `cmd /c echo hello` and asserting buffer row text.
6. Build the dispatch with P01's `HeadlessHost` and rely on the non-panicking no-op dispatch defaults provided by P01 (P03 replaces them); do not edit core.

## Todo List
- [ ] `_make_pane` real
- [ ] pump + repaint
- [ ] input + resize
- [ ] e2e smoke test
- [ ] `cargo run -p terminal` manual check recorded in `reports/`

## Success Criteria
- Given `cargo run -p terminal`, When pwsh starts and `echo hello` is typed, Then "hello" appears on screen and in `TextBuffer` row text.
- Given window resize, Then ConPTY size matches cell grid (assert via `conpty` test hook).
- `cargo test -p control --test e2e_shell_smoke` passes.

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "cmd-echo": spawn `cmd /c echo hello`, pump to EOF, assert buffer row 0 == "hello".
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 02` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p control --test e2e_shell_smoke`, `cargo run -p terminal` screenshot saved under `reports/` (evidence only, not an `e2e.csv` row; the automated row is `e2e_shell_smoke`).

## ADR Rationale
Context: need early integration signal. Decision: walking skeleton before parity. Why over bottom-up-only: de-risks GPUI/ConPTY integration (largest unknown) in week 1 and gives every later phase a runnable harness.

## Risk Assessment
GPUI entity/async model mismatch with blocking pump (use `cx.spawn` + channel). ConPTY handle lifetime leaks on exit.

## Security Considerations
Spawns user shell: no elevation, command line from settings only; no shell string interpolation.

## Parallelization Info
Wave 1 with P03-P07; P02 owns its files exclusively; P11 later extends them (dependency edge).

## File Ownership
`apps/terminal/**`, `crates/app/Cargo.toml` (only the `stub-downstream` feature removal), `crates/app/src/{app,downstream}.rs`, `crates/app/src/page/mod.rs` (P12 absorbs it later), `crates/control/src/{control,core}.rs`, `crates/control/tests/e2e_shell_smoke.rs`.

## Conflict Prevention
Does not touch core internals (consumes only APIs frozen by P01: parser/dispatch/buffer public API, `ITerminalConnection`, `TerminalSettings`), nor connection/renderer crates; a breaking change from P03/P04/P06 in the same wave is resolved by a serial fix before the barrier; needed API change → ledger gap for owning phase, adapter shim in control.

## Next Steps
P11 deepens control; P03/P04 replace stubs the skeleton tolerates.
