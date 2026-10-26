# Phase 04 — VT parser (output+input state machines) and TerminalInput

## Context Links
- `reports/02-*.md` (ADR-0002), existing `crates/core/src/parser/mod.rs` (695 LOC, `vte`-based)
- WT: `terminal/parser/{stateMachine.cpp,OutputStateMachineEngine.cpp,InputStateMachineEngine.cpp,ascii.hpp,tracing}`, `terminal/input/{terminalInput.cpp,mouseInput.cpp,terminalInput.hpp,...}`; tests `terminal/parser/ut_parser/*` (98 methods, 8.3k LOC)

## Overview
Priority P1 · Pending · Wave 1 · ~180h · Milestone M2. (a) Output engine parity (CSI/ESC/OSC/DCS routing, param parsing, subparams, split-string handling, C1/ST rules); (b) input state machine (VT input → `INPUT_RECORD`-equivalent events, kitty keyboard, win32-input-mode); (c) `TerminalInput` (key→VT sequence encoding, mouse encodings X10/UTF-8/SGR/URXVT, focus, bracketed paste, application keys, kitty flags).

## Key Insights
- ADR-0002 spike first: run all 98 ut_parser tests against `vte` wrapper; if any irreducible mismatch (e.g. partial-sequence timeout, OSC termination rules, `ESC` flush/`IsPending`), port `stateMachine.cpp` verbatim instead.
- `InputModeSink` seam (frozen by P01 in `dispatch/seams.rs`) is implemented here.

## Requirements
- Functional: identical event/dispatch call sequences to WT for every ut_parser vector; TerminalInput outputs byte-identical sequences for all key/mouse/modifier combos (adapter `inputTest.cpp`, `MouseInputTest.cpp`).
- Non-functional: streaming/chunk-boundary correct; no allocations per byte on hot path.

## Architecture
`parser/{mod,state_machine,output_engine,input_engine}.rs`; `terminal_input/{mod,keys,mouse,kitty}.rs`. Output engine calls `ITermDispatch`; input engine produces `InputEvent`s consumed by control layer (contract: `InputEvent` enum defined in `terminal_input/events.rs`, consumed by P11).

## Related Code Files
- Modify/Create: `crates/core/src/parser/**`, `crates/core/src/terminal_input/**`, tests `crates/core/tests/wt_parser*.rs`, `wt_input*.rs`, `parity/golden/parser-*`.
- Delete: none (rewrite inside `parser/mod.rs` as needed).

## Implementation Steps
1. Port ut_parser tests (red) → run vs current `vte` wrapper → record ADR-0002 decision with failing-case list.
2. Implement chosen engine; add output engine action tables (all `ActionXxx` in `OutputStateMachineEngine.cpp`).
3. Port `InputStateMachineEngine` (+ win32-input-mode, kitty decode, SS3/CSI keys, mouse reports, focus).
4. Port `TerminalInput` + mouse input; implement `InputModeSink`.
5. Chunk-boundary property tests (dispatch built with P01's `HeadlessHost`; gaps in its bodies → ledger entry resolved by P03, its owner, in a serial fix) (split every sequence at every byte offset).
6. Perf check: 100MB random-ish stream < budget set from baseline (document number).

## Todo List
- [ ] ut_parser ported
- [ ] ADR-0002 decision
- [ ] output engine complete
- [ ] input engine + TerminalInput
- [ ] chunk-split + perf checks

## Success Criteria
- All `tests.csv` rows for `terminal/parser/ut_parser` and input/mouse tests `passing`/`waived+ADR`.
- Given any ut_parser vector split at every byte boundary, Then dispatch call log equals unsplit log.
- Given key `Ctrl+Shift+Up` with cursor-key mode on/off, Then bytes equal WT expected strings.
- `cargo xtask ledger-check` shows parser/input symbols `verified`.

## Test Strategy & Quality Gate
- G/W/T: above; E2E scenario "split-stream": feed vttest stream in 1-byte chunks into parser+dispatch, buffer dump == one-shot dump.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 04` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p core -- wt_parser__ wt_input__`, `cargo bench`/timing note in `reports/`.

## ADR Rationale
ADR-0002: Context: `vte` is battle-tested but not WT's machine. Decision: spike-gated — keep `vte` only with 100% vector parity. Why over default-keep: unnoticed divergences surface as app-specific rendering bugs that parity audits cannot enumerate.

## Risk Assessment
Verbatim port cost if spike fails (+~40h); kitty/win32-input subtleties; timing-dependent ESC disambiguation.

## Security Considerations
Bounded parameter/OSC lengths; reject overflow params as WT does; no panic on malformed input (fuzz in P19).

## Parallelization Info
Wave 1; independent of P03 except shared trait `ITermDispatch` and seams (both frozen by P01; P04 implements in its own files).

## File Ownership
`crates/core/src/parser/**`, `crates/core/src/terminal_input/**`, `crates/core/tests/wt_parser*.rs`, `wt_input*.rs`, `parity/golden/parser-*`.

## Conflict Prevention
Seam traits live in `dispatch/seams.rs` (P01-created, frozen); P03 consumes, P04 implements in `terminal_input/**`. Any signature change request → ledger gap, resolved in a serial fix between waves.

## Next Steps
P09 composes parser+dispatch+input into `Terminal`; P11 consumes `InputEvent`.
