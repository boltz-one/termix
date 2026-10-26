# Phase 17 — TerminalSettingsEditor (settings UI)

## Context Links
- `reports/02-*.md`; WT `cascadia/TerminalSettingsEditor/*` (28,695 LOC, 43 `.cpp`: MainPage, Profiles*, ProfileViewModel, GlobalAppearance, ColorSchemes, Actions/ActionsViewModel, Launch, Interaction, Rendering, Appearance, NewTabMenu, Extensions, Compatibility, Editor controls, converters, …)

## Overview
Priority P3 · Pending · Wave 7 · ~330h · Milestone M5. GPUI settings UI with the same pages, view-models, validation, settings write-back (`SettingsLoader` write), profile CRUD (add/duplicate/delete/reorder), colour-scheme editor, actions/keybinding editor (key capture, conflict detection), new-tab-menu editor, extensions page, JSON-open buttons, reset-to-default per setting, unsaved-change handling, search/navigation, accessibility labels.

## Key Insights
- View-model layer (`*ViewModel.cpp`) is logic-heavy and testable; port first; XAML pages → GPUI views.
- Writes settings via P07; must preserve unknown JSON like WT.

## Requirements
- Functional: every setting exposed in WT editor present with same grouping, validation ranges, enable/disable dependencies, defaults indicator (inherit/override semantic), and persisted JSON identical to WT for same edits.
- Non-functional: edits applied live via settings reload (P07 watcher) without restart.

## Architecture
`settings/src/editor/{vm/*.rs,pages/*.rs,controls/*.rs,nav.rs}`; VMs depend on `settings` only; views on GPUI + `app::ui` theme/helpers.

## Related Code Files
- Create: `crates/settings/src/editor/**`, tests `crates/settings/tests/wt_editor_*.rs`, snapshots `parity/golden/editor-*`.
- Modify: none outside ownership.
- Delete: none.

## Implementation Steps
1. Ledger 43 cpp + xaml pages; map each setting key to editor row (matrix generated from model keys: coverage check = every user-editable key has editor control or documented exclusion).
2. Port view-models with tests (derive expectations from VM code paths; there are no WT unit tests → spec tests reviewed by audit).
3. Pages: Launch, Interaction, Appearance, Color schemes, Rendering, Compatibility, Actions, Profiles (+ per-profile Appearance/Advanced/Terminal), New tab menu, Extensions.
4. Write-back round-trip tests against P07 serializer.
5. Snapshots + keyboard navigation + manual row in `parity/e2e.csv` with signed checklist `reports/p17-manual-checklist.md` (items: keyboard navigation through every page, search, reset-to-default, unsaved-change prompt).

## Todo List
- [ ] key→control matrix
- [ ] view-models + tests
- [ ] pages
- [ ] write-back parity
- [ ] snapshots/a11y

## Success Criteria
- Key→control matrix: 100% user-editable keys covered or waived+ADR.
- Given changing font size to 14 in profile "pwsh" then save, Then resulting `settings.json` diff equals WT's output for same edit.
- All P17 ledger rows verified.

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "edit-profile-and-reload": edit → file → reload → control reflects.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 17` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p settings editor`; manual row in `parity/e2e.csv` (non-CI) with signed checklist `reports/p17-manual-checklist.md`.

## ADR Rationale
ADR-0012. Decision: MVVM retained (view-models port 1:1). Why over immediate-mode forms: keeps ported validation logic and enables headless tests.

## Risk Assessment
Largest UI surface; GPUI widget availability (lists, combo, colour picker); parity judged by matrix not pixels.

## Security Considerations
Writes user config: atomic write + backup, no path traversal in icon/commandline pickers, file dialogs via OS APIs.

## Parallelization Info
Wave 7 with P16; internally split by page groups across ≤4 workers (disjoint `pages/` files).

## File Ownership
`crates/settings/src/editor/**`, `parity/golden/editor-*`.

## Conflict Prevention
Consumes P07 model/serializer and P14 theme APIs read-only; own crate only.

## Next Steps
P18 packaging includes settings entry; P19 audit.
