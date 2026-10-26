# Phase 16 — WindowsTerminal host: window, non-client area, tray, quake, taskbar, emperor

## Context Links
- `reports/02-*.md`; WT `cascadia/WindowsTerminal/{main.cpp,AppHost.cpp,IslandWindow.cpp,NonClientIslandWindow.cpp,WindowEmperor.cpp,VirtualDesktopUtils.cpp,icon.cpp}` (7,551 LOC), `cascadia/TerminalApp/Jumplist.cpp` (and the host side of `TaskbarState.cpp`, ledger-owned by P12), tests `cascadia/ut_app` (1,256 / 52)

## Overview
Priority P1 · Pending · Wave 7 · ~200h · Milestone M5. Win32 host layer: process entry (`main`), `WindowEmperor` (window set, global hotkeys, tray icon + context menu, single-instance), `AppHost`/`IslandWindow` equivalents on GPUI window (title bar integration, custom non-client area w/ snap layouts, borderless/fullscreen/focus mode/pinned/always-on-top, DPI, minimize-to-tray, quake mode (`_quake` window, monitor placement, slide animation, global summon hotkey), virtual-desktop awareness, jump list, taskbar progress/overlay, window position/size persistence, initial position/size/launchMode settings, drag-and-drop of tabs between windows (tear-out), theme/titlebar colours, system backdrop (mica/acrylic where GPUI allows)).

## Key Insights
- WT has no `NotificationIcon.cpp` in this snapshot: tray logic lives in `WindowEmperor.cpp` (verify in ledger).
- Depends on P13 IPC, P14 titlebar/theme, P12 window logic, P15 UIA wiring.

## Requirements
- Functional: behaviours of `AppHost`/`IslandWindow`/`NonClientIslandWindow`/`WindowEmperor` incl. `ut_app` cases; launch modes (default/maximized/fullscreen/focus), `centerOnLaunch`, `alwaysOnTop`, `minimizeToNotificationArea`, `autoHideWindow`, global summon (`globalSummon` action) with `dropdownDuration`, `desktop` move, restore-after-sleep.
- Non-functional: cold start < 1 s target (recorded), no flicker on resize.

## Architecture
`crates/host/src/{host_trait.rs (platform-neutral `HostWindow`, `TrayIcon`, `GlobalHotkey` traits), emperor,app_host,window,nonclient,tray,quake,taskbar,jumplist,vdesktop,persist}.rs` where the Win32 modules are the Windows impls behind the traits; `apps/terminal/src/main.rs` calls `host::run()`.

## Related Code Files
- Create: `crates/host/**`, tests `crates/host/tests/wt_host_*.rs`.
- Modify: `apps/terminal/src/main.rs` (stub from P01, filled by P02; allowed via dependency chain).
- Modify: `crates/app/src/app.rs` (file exists since P02; window setup moves into host).

## Implementation Steps
1. Ledger WindowsTerminal symbols; port ut_app (52) tests as headless/Win32 integration where feasible.
2. Emperor + IPC integration (P13) + tray + hotkeys.
3. Window/non-client implementation with GPUI window hooks (custom titlebar from P14).
4. Quake/taskbar/jumplist/virtual desktops/persistence.
5. Wire UIA/TSF providers (P15) to window.
6. Manual checklist on multi-monitor + DPI scaling.

## Todo List
- [ ] tests ported
- [ ] emperor/tray/hotkeys
- [ ] window + nonclient
- [ ] quake/taskbar/jumplist/vdesktop
- [ ] a11y wiring + manual checklist

## Success Criteria
- ut_app rows passing/waived; host ledger rows verified.
- Given launchMode=maximized + persisted layout, Then window opens maximized on correct monitor with restored tabs.
- Given `globalSummon` hotkey, Then quake window slides in within `dropdownDuration` and focuses (automated Win32 test).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "tray-and-quake": `cargo test -p host --test e2e_quake -- --ignored` (test attribute `#[ignore = "manual: reports/p16-manual-checklist.md"]`; needs an interactive Windows desktop session; manual row in `parity/e2e.csv`, run at the wave barrier by the operator) registers the hotkey, sends it, asserts window visible+focused within `dropdownDuration`.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 16` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p host`, `cargo build --release -p terminal`.

## ADR Rationale
Context: XAML Islands host vs GPUI window. Decision: host owns Win32 behaviours; GPUI renders contents. Why over patching GPUI windowing: keeps vendored `zed_gpui` unmodified where possible (upstream sync).

## Risk Assessment
GPUI window customization limits (non-client area); global hotkeys/UI privilege issues; multi-monitor DPI bugs.

## Security Considerations
Global hotkey registration conflicts; tray menu actions run only built-in commands; window persistence file no secrets; elevated vs non-elevated window separation (UIPI) respected.

## Parallelization Info
Wave 7 with P17.

## File Ownership
`crates/host/**`, `apps/terminal/**` and `crates/app/src/app.rs` (`apps/terminal` stub created by P01, filled by P02; `app.rs` pre-existing, edited by P02; P16 edits both afterwards, no other phase touches them).

## Conflict Prevention
Edits only `app.rs` in app (no other phase touches it after P02; P12/P13/P14 own subdirs); other integration via public APIs; `apps/terminal` and `app.rs` serialised after P02 by dependency chain.

## Next Steps
P18 packaging.
