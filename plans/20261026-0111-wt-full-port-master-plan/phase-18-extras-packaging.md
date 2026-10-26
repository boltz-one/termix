# Phase 18 — Extras: shell extension, `wt` alias shim, elevate shim, packaging

## Context Links
- `reports/02-*.md`; WT `cascadia/ShellExtension/{dllmain.cpp,OpenTerminalHere.cpp}` (363), `cascadia/ElevateShim/elevate-shim.cpp` (106), `cascadia/wt/shim.cpp` (52), `cascadia/CascadiaPackage/*` (manifest/assets), `cascadia/WinRTUtils/*` (895, remaining pieces), `res/` icons (SVG/ICO policy: use existing assets, no new raster generation)

## Overview
Priority P3 · Pending · Wave 8 · ~50h · Milestone M5. Explorer context-menu "Open in Terminal" COM extension (Rust `windows` crate in-proc DLL), `wt.exe` app-execution-alias shim forwarding to the main executable, elevation shim (`--elevate` handoff), app manifest/installer layout (MSIX or portable zip decision), default-terminal registration (`CTerminalHandoff` registration from P06), file associations, `WinRTUtils` leftovers (if referenced).

## Key Insights
- Small LOC but cross-cutting registration (registry/COM/manifest); verification manual + install tests in a clean VM/sandbox.
- Packaging decided by user: portable zip first (ADR-0010); MSIX deferred (needs signing) and tracked as a follow-up, not part of this plan.

## Requirements
- Functional: right-click folder background → opens terminal in that directory with default profile; `wt` alias launches/forwards (P13 IPC); elevated launch via shim equals WT behaviours; uninstall removes registrations.
- Non-functional: no admin required except explicitly elevated features.

## Architecture
`crates/extras/{shell_ext,wt_shim,elevate_shim}` (cdylib/bin targets), `packaging/{manifest,scripts}`.

## Related Code Files
- Create: `crates/extras/**`, `packaging/**`, tests `crates/extras/tests/*`.
- Modify: none.
- Delete: none.

## Implementation Steps
1. Ledger files; finalise ADR-0010 (portable zip first, user decision); MSIX deferred.
2. Implement shims/extension with `windows` crate; install/uninstall scripts.
3. Integration tests: spawn shim with args → verify forwarded command list (P13 parser) ; COM registration test in sandbox.
4. Release build + package; checklist on clean machine.

## Todo List
- [ ] ADR-0010 finalised (portable zip)
- [ ] shims
- [ ] shell extension
- [ ] install/uninstall tests
- [ ] release package

## Success Criteria
- Given `wt.exe -d C:\x`, Then main app receives matching action list (integration test).
- Given install then uninstall in sandbox, Then registry/COM keys absent after uninstall (script assertion).
- Ledger rows for these dirs verified.

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "explorer-open-here" (manual + scripted invocation of the COM handler).
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 18` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p extras`; install/uninstall run in a Windows Sandbox/VM (non-CI, operator-run), manual row in `parity/e2e.csv`; signed checklist `reports/p18-install-checklist.md` (with logs attached).

## ADR Rationale
ADR-0010 (finalises the P01 stub): packaging. Context: MSIX requires certificates; portable is simpler. Decision (user): portable zip first, MSIX deferred. Why: no signing dependency; manifest/alias work stays reusable for MSIX later.

## Risk Assessment
Registry/COM pollution; elevation security; signing.

## Security Considerations
Elevation shim must not accept arbitrary command lines from untrusted sources beyond WT's contract; COM server registered per-user only; paths quoted; DLL search path hardened (no CWD loading).

## Parallelization Info
Wave 8 alone.

## File Ownership
`crates/extras/**`, `packaging/**`.

## Conflict Prevention
Only consumes public CLI/IPC contract from P13 and binary from P16.

## Next Steps
P19 final audit.
