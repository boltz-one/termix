# Phase 12 — TerminalApp core: Pane tree, Tab, TerminalPage, TerminalWindow

## Context Links
- `reports/02-*.md` (ADR-0005); existing `crates/app/src/{page,pane,tab}.rs` (1,175 LOC total in crate)
- WT `cascadia/TerminalApp/` (only the pane/tab/page/window/content/state files named here are ledger-owned by P12; `TaskbarState.cpp` model here, host side in P16): `Pane.cpp`, `Pane.LayoutSizeNode.cpp`, `Tab.cpp`, `TabManagement.cpp`, `TerminalPage.cpp`, `TerminalPaneContent.cpp`, `TerminalTabStatus.cpp`, `TerminalSettingsCache.cpp`, `TerminalWindow.cpp`, `App.cpp`, `AppLogic.cpp`, `ContentManager.cpp`, `TaskbarState.cpp`, `DebugTapConnection.cpp`; tests `LocalTests_TerminalApp` (5,559 / 59: TabTests, SettingsTests…), `ut_app`

## Overview
Priority P1 · Pending · Wave 5 · ~320h · Milestone M3. Pane binary tree (split h/v with sizes, resize, move focus by direction, swap, zoom, close, readonly, broadcast input, `LayoutSizeNode` resize math, pane content types as an enum with content-provider hooks: terminal here; settings/scratchpad/snippets/markdown content implemented in P14/P17), tabs (create/close/reorder/duplicate/color/rename/tab title logic, `TabStatus`, tab search/MRU switching state), window state (restore/layout persistence `GetLayout`/`persistedWindowLayouts`, startup actions, elevation warnings, pane content hand-off/tear-out), page-level actions implementation hooks consumed by P13.

## Key Insights
- Current tab/pane files have placeholder fallbacks (`tab.rs:62`, `pane.rs` header says `todo!` for split/focus).
- Page logic is ~13k LOC in WT; keep method boundaries to preserve tests (`TabTests` call page methods).
- Visual tab strip/command palette are P14; this phase delivers headless model + minimal GPUI tab bar sufficient for M3.

## Requirements
- Functional: pane tree arithmetic (`LayoutSizeNode::_AssignChildSizes`, snap sizes), focus navigation (`FocusPane`, `NavigateFocus`, `PreviousPane`, `MovePane`, `SwapPane`), zoom/unzoom, close semantics (last pane closes tab/window), tab color/title precedence, startup `NewTerminalArgs` resolution, per-profile connection creation (`_CreateConnectionFromSettings`), layout restore.
- Non-functional: all tree ops O(n) pure functions testable headless.

## Architecture
`app/src/pane/{tree,layout_size_node,content,navigation}.rs`; `tab/{mod,title,status,color}.rs`; `page/{mod,management,connections,layout,startup}.rs`; `TerminalWindow` in `page/window.rs`. GPUI views compose `TermControl` (P11).

## Related Code Files
- Modify/Create: `crates/app/src/{pane,tab,page}/**`, tests `crates/app/tests/wt_app_*.rs`.
- Delete: none (P01 already moved the single files into `pane/ tab/ page/` dirs).

## Implementation Steps
1. Ledger the TerminalApp files assigned to P12 (pane/tab/page/window/content/state files only); port only the pane/tab/page subset of LocalTests_TerminalApp (CommandlineTest→P13, palette tests→P14) as headless red tests (need a fake-free headless harness: real `Terminal` with Echo connection).
2. Pane tree + layout math with property tests (sizes sum, min size).
3. Tab model + title/status/color rules.
4. Page: connection creation, tab management, startup, layout persist/restore, zoom, broadcast.
5. Minimal GPUI tab bar + pane dividers (drag resize).
6. Integrate with P11 TermControl; manual checklist (split, zoom, close, reorder).

## Todo List
- [ ] tests red
- [ ] pane tree/layout
- [ ] tab model
- [ ] page/window logic
- [ ] minimal views
- [ ] manual checklist

## Success Criteria
- LocalTests_TerminalApp rows (pane/tab/page related) passing/waived; P12 ledger rows verified.
- Given split right then down then close middle, Then tree shape and sizes equal WT expected.
- `cargo run -p terminal`: Ctrl+Shift+D splits, Alt+arrows move focus, zoom toggles (items in `reports/p12-manual-checklist.md`).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "split-zoom-close-session" using Echo connections in headless GPUI test app.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 12` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p app -- wt_app__`; manual row in `parity/e2e.csv` (non-CI) with signed checklist `reports/p12-manual-checklist.md`: split right/down, move focus, zoom toggle, close middle pane, tab reorder.

## ADR Rationale
ADR-0005. Context: WinUI tree vs GPUI elements. Decision: logic in plain structs (testable), GPUI as thin render layer. Why over porting XAML structure: XAML trees do not map; behaviour parity is test-defined.

## Risk Assessment
Size of page logic; WinUI focus/automation assumptions; hidden coupling to settings/actions (mitigate via P13 interface contract `PageActions` trait defined here, impl P13 consumes).

## Security Considerations
Elevation prompts/warnings preserved; layout persistence file written with user-only perms; command lines never re-parsed via shell.

## Parallelization Info
Wave 5 with P15 (disjoint).

## File Ownership
`crates/app/src/{pane,tab,page}/**` (absorbs P02-owned `page/mod.rs`), `crates/app/tests/wt_app_*.rs`.

## Conflict Prevention
Defines `PageActions` trait in `page/actions_api.rs`; P13/P14 depend on P12 and only consume it; no shared file with P15.

## Next Steps
P13 actions; P14 chrome.
