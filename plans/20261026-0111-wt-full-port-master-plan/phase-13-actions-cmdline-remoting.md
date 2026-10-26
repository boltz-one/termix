# Phase 13 — Actions, key bindings, `wt` command line, multi-window remoting

## Context Links
- `reports/02-*.md` (ADR-0006); WT `cascadia/TerminalApp/{AppActionHandlers.cpp,ShortcutActionDispatch.cpp,AppKeyBindings.cpp,AppCommandlineArgs.cpp,Commandline.cpp,Remoting.cpp+Remoting.idl,ActionPreviewHandlers.cpp}`, `cascadia/WindowsTerminal/WindowEmperor.cpp` (ledger-owned by P16; P13 consumes only the IPC/window-registry behaviour), tests `LocalTests_TerminalApp/CommandlineTest.cpp`

## Overview
Priority P1 · Pending · Wave 6 · ~170h · Milestone M4. Implement every `ShortcutAction` handler (≈70: newTab, splitPane, moveFocus, resizePane, scrollUp/Down, find, copy/paste, toggleFullscreen/focusMode/pinned, openSettings, commandPalette, quakeMode, multipleActions, sendInput, renameTab, setColorScheme preview, …), keybinding dispatch (`AppKeyBindings`), `wt.exe` argument grammar (subcommands `new-tab`, `split-pane`, `focus-tab`, `move-focus`, `swap-pane`, `focus-pane`; `;` chaining; `--window`, `--startingDirectory`, `--title`, `--tabColor`, `--suppressApplicationTitle`, `--colorScheme`, `--elevate`...), single-instance/multi-window dispatch (`--window`, window naming, `WindowEmperor` behaviours: attach to existing window via named pipe/mutex), action preview.

## Key Insights
- `cascadia/Remoting/` has only resw; logic is in `TerminalApp/Remoting.*` + `WindowEmperor` (ADR-0006: IPC → named pipe with same observable semantics).
- Actions need P07 `ActionMap`/args; page effects via `PageActions` (P12).

## Requirements
- Functional: each action's observable effect + args defaults match `AppActionHandlers.cpp`; commandline parser outputs same action list for all CommandlineTest vectors (incl. error text/exit codes, `--help` output).
- Non-functional: second `terminal.exe` invocation forwards within 200 ms.

## Architecture
`app/src/actions/{dispatch,handlers/*.rs,keybindings}.rs`, `cmdline/{parser,commandline,help}.rs` (hand-written grammar replicating CLI11 behaviour — ADR note), `remoting/{emperor_ipc,window_registry}.rs`.

## Related Code Files
- Create: `crates/app/src/{actions,cmdline,remoting}/**`, tests `crates/app/tests/wt_cmdline_*.rs`, `wt_actions_*.rs`.
- Modify: none beyond ownership.
- Delete: none.

## Implementation Steps
1. Ledger actions (one row per `ShortcutAction`) and cmdline vectors; port CommandlineTest red.
2. Implement parser replicating CLI11 semantics (subcommand chaining, option aliases, help text, error codes).
3. Action handlers grouped by domain (tab/pane/window/clipboard/scroll/ui); preview handling.
4. Keybinding dispatch + chord conflicts/layout-aware keys.
5. IPC single-instance + window registry; quake/`_quake` window name semantics (host side in P16).
6. Integration tests per action through headless app.

## Todo List
- [ ] action ledger + tests red
- [ ] cmdline parser
- [ ] handlers
- [ ] keybindings
- [ ] IPC/windows

## Success Criteria
- 100% `ShortcutAction` rows verified; CommandlineTest rows passing.
- Given `wt new-tab -d C:\ ; split-pane -V`, Then parsed action list equals WT expected.
- Given second launch with `-w 0 new-tab`, Then first window receives action (IPC integration test).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "second-instance-forward": `cargo test -p app --test e2e_forward` starts the IPC server in-process, launches a second client with `-w 0 new-tab`, asserts the first window's action queue received `NewTab`.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 13` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p app -- wt_cmdline__ wt_actions__`.

## ADR Rationale
ADR-0006. Context: WinRT activation/COM remoting unavailable. Decision: named-pipe IPC mirroring Monarch/Peasant-like behaviour in WindowEmperor. Why over single-process only: required for `-w`, tear-out, quake semantics.

## Risk Assessment
CLI11 behavioural edge cases; IPC races; action count large.

## Security Considerations
Named pipe ACL restricted to current user; validate/limit forwarded argument sizes; `--elevate` path via shim reviewed (P18); no command injection in forwarded cwd/commandline.

## Parallelization Info
Wave 6 with P14 (disjoint dirs).

## File Ownership
`crates/app/src/{actions,cmdline,remoting}/**`, `crates/app/tests/{wt_cmdline_*,wt_actions_*,e2e_forward}.rs`.

## Conflict Prevention
Consumes `PageActions` (P12) and `ActionMap` (P07) read-only; UI invocation points live in P14 which calls dispatch API only.

## Next Steps
P16 host consumes IPC; P14 lists actions via `ActionMap` (P07) and invokes them through `PageActions` (P12).
