# Phase 14 — UI chrome: tab strip, command palette, suggestions, dialogs, markdown

## Context Links
- `reports/02-*.md` (ADR-0005); WT `cascadia/TerminalApp/{CommandPalette.cpp,CommandPaletteItems.cpp,FilteredCommand.cpp,HighlightedTextControl.cpp,SuggestionsControl.cpp,PaletteItemTemplateSelector.cpp,TabHeaderControl.cpp,TabRowControl.cpp,TitlebarControl.cpp,MinMaxCloseControl.cpp,ColorPickupFlyout.cpp,AboutDialog.cpp,Toast.cpp,DesktopNotification.cpp,LanguageProfileNotifier.cpp,MarkdownPaneContent.cpp,SnippetsPaneContent.cpp,ScratchpadContent.cpp}`, `cascadia/UIHelpers/*`, `cascadia/UIMarkdown/*`, `cascadia/fzf/*`, `*.xaml` styles/themes

## Overview
Priority P2 · Pending · Wave 6 · ~230h · Milestone M5. Full visual layer parity: tab strip (drag reorder, tear-out hooks, overflow, close buttons, new-tab dropdown with profiles/actions, tab context menu, colour/icon, bell/progress/readonly indicators), titlebar (custom, min/max/close), command palette (action/commandline/tab-search/tab-switcher modes, fuzzy match via `fzf` port, highlighting, nested commands, preview), suggestions control, toasts, about dialog, color picker flyout, notifications, language-profile notifier, markdown rendering, snippets/scratchpad panes, themes (light/dark/system, tab-row colours, acrylic/mica hooks), `UIHelpers` (converters, icon path resolution, text menu flyout).

## Key Insights
- XAML styling/resources (~tens of xaml files) → GPUI theme tokens; visual identity measured by layout spec + screenshot tolerance, behaviour by tests.
- `fzf` is MIT third-party C (459 LOC) → port algorithm with tests from its behaviours.

## Requirements
- Functional: every palette mode/filter/ordering result equals WT for same inputs (fuzzy score, tie-breaks, highlight spans); tab strip interactions mapped to actions; keyboard navigation + focus order parity.
- Non-functional: UI interactions < 50 ms; accessibility labels set (hooks for P15).

## Architecture
`app/src/ui/{tab_strip,titlebar,palette/{mod,filter,fzf,items},suggestions,dialogs,toasts,markdown,theme,helpers}.rs` — pure state machines (testable) + GPUI views.

## Related Code Files
- Create: `crates/app/src/ui/**`, tests `crates/app/tests/wt_ui_*.rs`, snapshots `parity/golden/ui-*`.
- Modify: none outside ownership.
- Delete: none.

## Implementation Steps
1. Ledger UI files and behaviours; derive behaviour specs from C++ + `FilteredCommand` tests.
2. `fzf` + filtering/highlight state machine; palette modes.
3. Tab strip + titlebar; drag reorder state machine.
4. Suggestions, dialogs, toasts, notifications.
5. Markdown builder → GPUI elements; snippets/scratchpad.
6. Theme system + settings `theme` application.
7. Snapshot + interaction tests.

## Todo List
- [ ] fzf + palette
- [ ] tab strip/titlebar
- [ ] suggestions/dialogs/toasts
- [ ] markdown/snippets
- [ ] themes
- [ ] snapshots

## Success Criteria
- Given query "np" on WT default commands, Then ordered result list + highlight spans equal WT's (fixture from running upstream algorithm on same inputs).
- Given tab drag from index 0 to 2, Then reorder action + indices equal WT.
- All P14 ledger rows verified/replaced+ADR; snapshots within tolerance.

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "palette-run-action": `cargo test -p app --test e2e_palette` (headless GPUI test context) opens the palette, types "split", presses Enter, asserts `PageActions::split_pane` was invoked once.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 14` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p app -- wt_ui__`.

## ADR Rationale
References ADR-0005 (owned by P12). Decision: behaviour+layout parity not XAML identity. Why over XAML-compat layer: no mature XAML runtime on GPUI; cost exceeds benefit.

## Risk Assessment
Visual drift subjective (mitigate with explicit layout spec table); drag/drop complexity; GPUI widget gaps.

## Security Considerations
Markdown/HTML content from settings or snippets sanitised (no remote loads); URL opening allowlist; notifications don't leak command line.

## Parallelization Info
Wave 6 with P13.

## File Ownership
`crates/app/src/ui/**`, `crates/app/tests/{wt_ui_*,e2e_palette}.rs`, `parity/golden/ui-*`.

## Conflict Prevention
Invokes actions only via `PageActions` (P12) and reads `ActionMap` (P07); never depends on P13 (the `PageActions` trait is frozen by P12 in `page/actions_api.rs`; P13 and P14 share a wave but have no edge).

## Next Steps
P16 host embeds titlebar; P17 settings UI reuses theme/helpers.
