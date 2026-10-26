# Phase 11 — TerminalControl (ControlCore, ControlInteractivity, TermControl)

## Context Links
- `reports/02-*.md`; existing `crates/control/src/*` (1,159 LOC; P02 wiring)
- WT: `cascadia/TerminalControl/*` (15,279 LOC: ControlCore.cpp, ControlInteractivity.cpp, TermControl.cpp, TermControlAutomationPeer (→P15), ControlSettings, SearchBoxControl.xaml, InteractivityAutomationPeer, KeyChord.cpp, ScrollBarVisualStateManager.cpp, XamlLights.cpp, EventArgs.cpp; `HwndTerminal*.cpp` excluded per ADR-0001; automation peers `*AutomationPeer.cpp`/`XamlUiaTextRange.cpp` → P15), tests `UnitTests_Control` (2,022 / 28)

## Overview
Priority P1 · Pending · Wave 4 · ~230h · Milestone M3. The UI-facing control: settings application (`UpdateSettings`, appearance, hot-reload, opacity), focus handling, keyboard routing (key chords vs terminal, AltGr, dead keys, `Ctrl+C` copy semantics, `sendInput`), mouse (selection drag, click counts, wheel/zoom/horizontal, mouse reporting, shift override), clipboard copy/paste (formats: text/html/rtf, trim, `copyOnSelect`, multi-line warnings, large-paste warnings, bracketed paste), search box + highlights, scrollbar + marks, hyperlink hover/Ctrl+click, drag-drop paths, font size zoom, cursor blink, read-only mode, bell (audible/visual/taskbar), progress, output-event flow (`TerminalOutput`, `WarningBell`, `TitleChanged`, `CursorPositionChanged`, …).

## Key Insights
- P02 files (`control.rs`, `core.rs`) are owned here now (P11 depends on P02).
- TermControl XAML visuals → GPUI view (ADR-0005); behaviour is the parity target.
- TSF/IME and UIA are P15 (hook points defined here: `ImeSink`, `AutomationProvider` traits).

## Requirements
- Functional: ControlCore/Interactivity public methods and events behave per WT incl. all UnitTests_Control scenarios (e.g. `ControlCore` selection/scroll/search, `ControlInteractivity` mouse/zoom, `ControlCore::SearchResults`, `PasteText` conversions).
- Non-functional: UI thread vs connection/render thread separation with documented channels; no blocking in UI callbacks.

## Architecture
`control/src/{core/,interactivity/,view/,input/,clipboard/,search/,settings/}`; `ControlCore` owns `Terminal`(P09)+`Renderer`(P10)+`Connection`(P06); `ControlInteractivity` wraps core; `TermControl` GPUI view composes both and exposes events to app.

## Related Code Files
- Modify/Create: `crates/control/**` (tests `crates/control/tests/wt_control_*.rs`).
- Delete: temporary shims from P02.

## Implementation Steps
1. Ledger ControlCore/Interactivity/TermControl symbols; port UnitTests_Control (28) red.
2. Core: settings/appearance application, connection lifecycle events, output flow, search, marks.
3. Interactivity: mouse state machine, selection modes, wheel/zoom, hyperlink hover.
4. Input: key routing/chords, dead keys, AltGr, `sendInput`, paste pipeline with confirmations.
5. View: GPUI element, scrollbar, search box, context menu hooks, focus/blink, drag-drop.
6. Clipboard formats; bell; taskbar/progress events.
7. Replace P02 shims; manual e2e checklist run, evidence in `reports/`.

## Todo List
- [ ] tests red
- [ ] core
- [ ] interactivity
- [ ] key/input/paste
- [ ] view/scrollbar/search
- [ ] clipboard/bell/progress

## Success Criteria
- UnitTests_Control rows passing; control symbols verified.
- Given triple-click then copy, Then clipboard text equals WT line-selection output.
- Given paste of 6 KiB multiline text with warnings on, Then confirmation event raised before write.
- `cargo run -p terminal`: mouse selection + Ctrl+Shift+C/V + search work (items listed in `reports/p11-manual-checklist.md`).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "select-copy-paste-roundtrip" in real window harness (GPUI test context).
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 11` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p control`; manual row in `parity/e2e.csv` (non-CI, interactive Windows session) with signed checklist `reports/p11-manual-checklist.md` listing: mouse select, triple-click line select, Ctrl+Shift+C/V, search box, scrollbar drag, zoom.

## ADR Rationale
Context: WT control logic spans core/interactivity/XAML. Decision: preserve the 3-layer split in Rust. Why over single view struct: ported tests target Core/Interactivity separately and reuse in P12 headless tests.

## Risk Assessment
Key handling Windows-specific (layouts, dead keys); GPUI input event gaps; clipboard format fidelity.

## Security Considerations
Paste handling: bracketed paste sanitisation (strip ESC[201~), warn on multiline/large; hyperlink open requires scheme allowlist as WT; drag-drop path quoting rules.

## Parallelization Info
Wave 4 alone with its consumers behind; may batch with nothing.

## File Ownership
`crates/control/**` (incl. P02-created files after P02 completes).

## Conflict Prevention
Depends on P02 (shared files) and consumes P06/P07/P09/P10 public APIs only.

## Next Steps
P12 (panes/tabs), P15 (a11y/IME) attach to control hooks.
