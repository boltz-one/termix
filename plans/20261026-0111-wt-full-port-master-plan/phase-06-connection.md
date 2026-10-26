# Phase 06 — TerminalConnection + winconpty

## Context Links
- `reports/02-*.md` (ADR-0007); existing `crates/connection/src/{conpty.rs,connection/*,event.rs}` (1,693 LOC, 28+ tests)
- WT: `cascadia/TerminalConnection/{ConptyConnection.cpp,AzureConnection.cpp (OUT of scope, ADR-0007),CTerminalHandoff.cpp,ITerminalConnection.idl,...}` (2,957 LOC), `winconpty/*` (781; tests 357/7)

## Overview
Priority P2 · Pending · Wave 1 · ~60h · Milestone M3 (needed by shell reliability). Audit + complete: ConPTY lifecycle (create/resize/pause/resume/close), `ConptyConnection` features (startup info, env block, working dir, reload env vars, `ShowHide`, passthrough mode, `ConptyPassthrough`/win32 input), terminal handoff (`CTerminalHandoff`, default-terminal), Echo connection.

## Key Insights
- Current crate has lifecycle + conpty; no live `todo!` (doc comment only); no test with a real child process beyond unit tests (verify in step 1).
- AzureConnection (Cloud Shell) is OUT of scope per user decision (ADR-0007 = out-of-scope; its WT files are rows in `exclusions.csv`). Platform: ConPTY sits behind a `PtyBackend` trait (Windows impl `ConptyBackend`) so other platforms can add backends later.

## Requirements
- Functional: parity with `ConptyConnection` public behaviour incl. `Initialize(settings)` keys, `StartInboundListener`, `Resize`, `WriteInput`, `Close`, `ConnectionState` transitions and `TerminalOutput`/`StateChanged` events, process exit codes/messages ("[process exited with code N]"), environment/`WT_SESSION`/`WT_PROFILE_ID`; `EchoConnection`; `CTerminalHandoff` COM replacement (named server or documented waiver ADR).
- Non-functional: no handle leaks (tested), thread-safe.

## Architecture
`ITerminalConnection` trait (channel-based events) → `ConptyConnection`, `EchoConnection`; `ConptyConnection` runs on `PtyBackend` (Windows impl `ConptyBackend`).

## Related Code Files
- Modify/Create: `crates/connection/**` (`pty_backend.rs`, `handoff.rs`, tests `wt_winconpty_*.rs`, `wt_connection_*.rs`).
- Delete: none.

## Implementation Steps
1. Ledger rows + diff current vs WT; port `winconpty` tests (7) and ConptyConnection behaviours as red tests.
2. Fill gaps (settings keys, env block merge, startup directory, process-exit messaging, passthrough flag, reload env).
3. Handoff (`CTerminalHandoff`) design: a decision note forwarded to P13 (owner of ADR-0006: COM activation replaced by named pipe, or waived).
4. Echo connection; leak tests (open/close 1000×, handle count stable).
5. Extract `PtyBackend` trait (create/resize/read/write/close) with `ConptyBackend` impl; connection logic depends only on the trait; tests run on the real ConPTY impl.

## Todo List
- [ ] gap diff + tests red
- [ ] conpty parity
- [ ] handoff decision
- [ ] echo + leak tests
- [ ] `PtyBackend` trait + `ConptyBackend`

## Success Criteria
- Given spawn `cmd /c exit 7`, Then state → Closed and output contains WT exit message with code 7.
- Given 1000 open/close cycles, Then handle count delta == 0.
- `tests.csv` rows `winconpty/ut_*` all passing/waived; connection symbols verified; 0 `todo!` in crate.

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "conpty-roundtrip": write `echo x\r`, read output containing `x`.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 06` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p connection`, `cargo xtask stubs`.

## ADR Rationale
ADR-0007: AzureConnection (Cloud Shell) out of scope. Context: external provider needing Microsoft auth + network, unverifiable offline. Decision (user, 2026-10-26): exclude; record rows in `exclusions.csv`. Why over feature-gated port: no credentials/network dependency and no hard-gate onboarding risk.

## Risk Assessment
Handle leaks/races at close; handoff COM semantics; trait abstraction hiding ConPTY-specific behaviour.

## Security Considerations
Env block handling (no secret logging); command-line quoting/escaping parity.

## Parallelization Info
Wave 1; own crate, disjoint.

## File Ownership
`crates/connection/**`.

## Conflict Prevention
Public trait `ITerminalConnection` signature frozen by P01/P02 usage; breaking change needs serial fix between waves; P02 only consumes it.

## Next Steps
P11 consumes; P13 `wt` commandline passes connection settings.
