# Phase 15 — Accessibility (UIA) + IME/TSF

## Context Links
- `reports/02-*.md`; WT: `types/{UiaTextRangeBase.cpp,UiaTextRange.cpp,ScreenInfoUiaProviderBase.cpp,TermControlUiaProvider.cpp,...}`, `renderer/uia/*` (570), `cascadia/TerminalControl/{TermControlAutomationPeer.cpp,XamlUiaTextRange.cpp,InteractivityAutomationPeer*}`, `tsf/{Handle.cpp,Implementation.cpp}` (1,150), tests: `types/ut_types` UIA tests, `cascadia/WindowsTerminal_UIATests` (reference only)

## Overview
Priority P2 · Pending · Wave 5 · ~170h · Milestone M5. Expose terminal text/selection/caret to Windows UI Automation (text pattern, text ranges, move/expand units, selection, scroll pattern, notifications for new output, caret/cursor events) and implement TSF/IME composition (preedit rendering, candidate window positioning, reconversion, composition commit to ConPTY).

## Key Insights
- Needs P10 (cursor/cell geometry for UIA bounds) and P11 (control hooks `AutomationProvider`/`ImeSink` defined in P11).
- GPUI may not expose UIA tree; may require raw HWND-level `IRawElementProviderSimple` via Win32 (`windows` crate) — spike first (ADR-0009 raised in step 1).

## Requirements
- Functional: `UiaTextRangeBase` navigation semantics (character/word/line/paragraph/page/document units, endpoints, `GetText`, `GetBoundingRectangles`, `ScrollIntoView`, `FindText`), selection events, live-region style notifications equal WT, TSF: composition string display, active composition cursor, candidate window anchor, `OnEndComposition` input forwarding.
- Non-functional: no deadlocks with render thread; COM objects released correctly.

## Architecture
`a11y/src/{provider_trait.rs (platform-neutral `AccessibilityProvider` + `ImeBackend` traits), uia/{provider,text_range,peer}.rs, tsf/{handle,implementation}.rs}` — UIA/TSF are the Windows impls behind the traits (`windows` crate); hooks attached to control via traits from P11.

## Related Code Files
- Create: `crates/a11y/src/**`, tests `crates/a11y/tests/wt_uia_*.rs` (ported from `ut_types` UIA cases).
- Modify: none outside ownership.
- Delete: none.

## Implementation Steps
1. Spike: how to host UIA provider in GPUI window (HWND `WM_GETOBJECT`), document in ADR-0009.
2. Port `UiaTextRangeBase` + tests (move/expand/compare/endpoints) against core buffer.
3. Provider + selection/caret/notifications; scroll pattern.
4. TSF implementation + `PreeditSink` hook (defined by P10, which owns the renderer-side overlay).
5. Manual verification with Narrator + Accessibility Insights (checklist report); automated UIA client test via `windows` crate.

## Todo List
- [ ] ADR-0009 spike
- [ ] UIA text range + tests
- [ ] provider/events
- [ ] TSF
- [ ] manual a11y checklist

## Success Criteria
- Ported UIA test rows passing; a11y ledger rows verified.
- Given UIA client `ExpandToEnclosingUnit(Word)` at a position, Then range text equals WT expected.
- Given composing "にほん", Then preedit text drawn at cursor and committed bytes sent after commit (integration test + manual).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "uia-read-line": UIA client reads visible line text from running window.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 15` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p a11y`; E2E and Narrator checks need an interactive Windows desktop session (manual rows in `parity/e2e.csv`, non-CI, run by operator at the barrier; signed checklist `reports/p15-a11y-checklist.md`).

## ADR Rationale
ADR-0009 (finalises the P01 stub): UIA hosting strategy. Context: WT uses XAML automation peers. Decision: Win32 raw provider on HWND (pending spike). Why: GPUI exposes no XAML peer tree.

## Risk Assessment
COM lifetime/threading; GPUI hooks; IME on custom rendering; manual-only verification parts.

## Security Considerations
UIA clients can read all terminal text: honour WT behaviours (no extra data); validate ranges from clients to avoid OOB.

## Parallelization Info
Wave 5 with P12 (disjoint crates).

## File Ownership
`crates/a11y/**`.

## Conflict Prevention
Attaches via traits owned by control (P11, earlier wave); wiring call in app handled by P16 host (later wave).

## Next Steps
P16 wires provider to window.
