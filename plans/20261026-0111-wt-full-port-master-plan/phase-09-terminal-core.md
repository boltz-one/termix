# Phase 09 — TerminalCore (`Terminal` class: selection, scrolling, mouse, title, colours)

## Context Links
- `reports/02-*.md`; WT: `cascadia/TerminalCore/*` (4,277 LOC: Terminal.cpp, TerminalApi.cpp, TerminalSelection.cpp, terminalrenderdata.cpp, Terminal.hpp, ControlKeyStates.hpp, ITerminalInput.hpp), tests `UnitTests_TerminalCore` (2,708 LOC / 54)
- Depends on core parser (P04), dispatch (P03), buffer (P08)

## Overview
Priority P1 · Pending · Wave 3 · ~130h · Milestone M2/M3. The `Terminal` host object: owns buffer(s) (main/alt), viewport + scroll offset, selection model (all modes: char/word/line/block, shift-click extend, keyboard selection), `ITerminalApi` implementation for dispatch, title/tab-color/progress state, `IRenderData` for the renderer, mouse/key input routing to `TerminalInput`, bracketed paste, search highlight state, bell, cursor visibility, scroll marks, snap-on-input, `UserScrollViewport`, resize with reflow.

## Key Insights
- Today there is no `terminal/` module; control `core.rs` holds ad-hoc glue (P02/P11 own) — migrate logic here, leave thin adapter.
- Implements `RenderModeSink` (writes `RenderState`) and `ITerminalApi`; both traits frozen in P01 `dispatch/seams.rs`; P10 only reads state through `IRenderData`.

## Requirements
- Functional: behaviour parity with `Terminal`/`TerminalSelection` for all UnitTests_TerminalCore cases (scroll, selection, hyperlink, mark, resize, alt buffer, title stack, `Terminal::SendKeyEvent`, `SendMouseEvent`, `SendCharEvent`, `FocusChanged`, `GetHyperlinkAtBufferPosition`, `GetTextBuffer*`, URL detection, `ExpandSelectionToWord`).
- Non-functional: thread-safe lock discipline equal to WT (`LockForReading/Writing` → `RwLock` with documented order).

## Architecture
`terminal/{mod,api,selection,scroll,input_bridge,render_data,hyperlinks,marks,search_state}.rs` implementing `ITerminalApi`, `RenderModeSink` and `IRenderData` (all trait definitions frozen by P01 in `dispatch/seams.rs`; `IRenderData` consumed by P10). Single `Terminal` struct exposed to control.

## Related Code Files
- Create: `crates/core/src/terminal/**`, tests `crates/core/tests/wt_terminalcore*.rs`.
- Modify: none outside ownership.
- Delete: none.

## Implementation Steps
1. Ledger TerminalCore symbols; port UnitTests_TerminalCore (54) red. Ad-hoc glue in `control/core.rs` is NOT edited here: P09 exposes the `Terminal` API and P11 migrates callers.
2. Terminal struct + `ITerminalApi` impl (all methods dispatch calls need).
3. Viewport/scroll/resize + alt buffer.
4. Selection (all anchor/pivot semantics from `TerminalSelection.cpp`).
5. Input bridge (`terminalInput` from P04), title/progress/bell/marks/hyperlinks.
6. `IRenderData` impl + parity test over scroll+selection+cursor cases.

## Todo List
- [ ] tests red
- [ ] Terminal + API
- [ ] scroll/resize
- [ ] selection
- [ ] input bridge + state
- [ ] render data

## Success Criteria
- UnitTests_TerminalCore rows all passing/waived; terminal symbols verified.
- Given word-select on "foo bar", Then selection anchors equal WT expected coordinates.
- Given scroll-up 3 lines then new output, Then viewport snaps per `snapOnInput` semantic (`cargo test -p core -- wt_terminalcore__scroll`).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "vim-session-replay": replay captured vim byte stream (golden) through Terminal, compare final visible rows + cursor.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 09` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p core -- wt_terminalcore__`, goldens `parity/golden/terminal-*`.

## ADR Rationale
Context: C++ `Terminal` mixes concerns via COM-ish interfaces. Decision: single struct implementing Rust traits equivalent to `ITerminalApi`/`IRenderData`, behaviour-identical. Why over splitting into actor-model: lock ordering parity with WT tests; refactor after 100%.

## Risk Assessment
Selection semantics subtle; lock contention with renderer thread; cross-phase API drift (mitigate with traits frozen in P01).

## Security Considerations
Hyperlink/URL handling: no auto-open here (policy in P11); bound selection/text extraction sizes; title length caps.

## Parallelization Info
Wave 3 with P10 (disjoint).

## File Ownership
`crates/core/src/terminal/**`, `crates/core/tests/wt_terminalcore*.rs`, `parity/golden/terminal-*`.

## Conflict Prevention
Consumes buffer/dispatch/parser public APIs only; no edits to their dirs (gaps → serial fix between waves via ledger).

## Next Steps
P11 builds control atop `Terminal`.
