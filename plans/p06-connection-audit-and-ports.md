# Plan: P06 — connection parity audit + winconpty tests + EchoConnection + PtyBackend trait

**Owner**: main-loop agent (delegated to 1 cook-worker sonnet, disjoint crate).
**Wave**: 1 (cùng 02/03/04/05/07 — disjoint `crates/connection/**`).
**Mode**: parity audit + TDD red→green (per master plan P06 context).

## Context

P06 § reports/02 (master plan) báo: trang thai hien tai `connection` 1,746 LOC (port tu Phase 1 skeleton + Phase 4 C1+C2+C3 fix — `conpty.rs` `pipe_out`, `lifecycle.rs` `OutputThreadCtx` by-value, `CancelIoEx(pipe_out)`, `done_rx` cleanup rendezvous). Audit round-4 (2026-10-26) `cargo xtask audit --final` FAIL: 987 rows `not-started` (P06 shard chua seed), clippy regression o renderer (out-of-scope cho P06), 276 stubs.

Target WT scope P06:
- `windows/src/cascadia/TerminalConnection/` (1,315 LOC): `ConptyConnection.{cpp,h,idl}` (892+112+40), `EchoConnection.{cpp,h,idl}` (47+32+? = skip idl), `CTerminalHandoff.{cpp,h}` (159) — ADR-0006 out-of-scope per P06 step 3 (decision note to P13).
- `windows/src/winconpty/` — `winconpty.cpp` (622) replaced by kernel32 ConPTY per architecture decision; `ft_pty/ConPtyTests.cpp` (341 LOC, 7 test methods) — port as red tests.

Pham vi KHONG cham: `AzureConnection.*` (ADR-0007 out-of-scope), `BaseTerminalConnection.h` (template, port ad-hoc), `ConnectionInformation.*` (P07 settings layer), conhost `host/`, `server/` (ADR-0001).

## Current Rust state (verified 2026-10-26)

- `crates/connection/src/lib.rs` — re-exports `ITerminalConnection, ConptyConnection, ConptySettings, PseudoConsoleFlags, StateChangedEvent, TerminalOutputEvent`.
- `crates/connection/src/connection/mod.rs` (226 LOC) — `ITerminalConnection` trait (9 method send), `ConptyConnection` struct (16 field incl `_start_time`, `shared_state: Arc<AtomicU8>`, `done_rx`), `transition_to_state` state-machine guard (forward-progress + terminal reject).
- `crates/connection/src/connection/lifecycle.rs` (661 LOC) — `start`/`write_input`/`resize`/`close`/`initialize`/`reset_size`/`clear_buffer` + `OutputThreadCtx` + `output_thread_loop` + cleanup rendezvous (`done_rx`) + `ReleasePseudoConsole` dynamic-resolve + `utf16_to_utf8` eat-error.
- `crates/connection/src/connection/settings.rs` (111 LOC) — `ConptySettings` (13 field) + `PseudoConsoleFlags` bitflags (3 bit: INHERIT_CURSOR/GLYPH_WIDTH_WIDE/AMBIGUOUS_IS_WIDE).
- `crates/connection/src/connection/state.rs` (52 LOC) — `ConnectionState` enum `#[repr(u8)]`.
- `crates/connection/src/connection/tests.rs` (112 LOC) — 28 unit tests (state transitions, settings default, FFI load).
- `crates/connection/src/conpty.rs` (471 LOC) — FFI shims (`CreatePseudoConsole`/`ResizePseudoConsole`/`ClearPseudoConsole`/`ClosePseudoConsole`/`CreateProcessW`/`ReadFile`/`WriteFile`/`CancelIoEx` + dynamic `ClearPseudoConsole`/`ReleasePseudoConsole` GetProcAddress).
- `crates/connection/src/event.rs` (97 LOC) — `TerminalOutputEvent`/`StateChangedEvent` channel-based.
- `crates/connection/tests/` — DOES NOT EXIST (no `wt_connection_*` / `wt_winconpty_*` integration tests yet).

## Parity gaps (P06 step 1 — diff vs WT)

Based on P06 plan §Requirements + Phase 4.x audit reported gaps:

1. **No ledger shard** `parity/ledger/P06-connection/` — need `cargo xtask ledger-seed` partial for connection (files.csv + symbols.csv + tests.csv). P06 owns its shard.
2. **No `wt_winconpty_*` red tests** — port 7 ConPtyTests methods (`CreateConPtyNoPipes`, `CreateConPtyBadSize`, `GoodCreate`, `GoodCreateMultiple`, `SurvivesOnBreakOutput`, `DiesOnClose`, `ReleasePseudoConsole`) into `crates/connection/tests/wt_winconpty__*.rs` naming rule `wt_<suite>__<method>` (suite `winconpty`). Most map onto `ConptyConnection::start`/`close` lifecycle (not raw `_CreatePseudoConsole` FT — adapt to Rust `ConptyConnection::new().initialize(s).start()` flow). Red stage: write tests, expect fail (FFI live spawn needs real ConPTY — gated `#[ignore]` if no ConPTY, OR use `cmd /c exit 0` smoke).
3. **Process exit messaging gap** (audit H3) — `lifecycle.rs` exit path thiếu WT `cpp:517-528,549` `"\r\n{ProcessExited}\r\n{CtrlDToClose}\r\n"` `TerminalOutput.raise` — port verbatim.
4. **ProcessFailedToLaunch sub-messages** (audit H2) — `lifecycle.rs:118-120,128-130` failure path thiếu `TerminalOutput.raise(failureText)` before `transition_to_state(Failed)` — port WT `cpp:506-507` failure sub-messages.
5. **ResetSize** (audit H4) — `lifecycle.rs:399-403` chỉ store rows/cols, không gọi `ResizePseudoConsole` khi `isConnected()`. Port WT semantics: queue + apply on next `Resize` (or apply immediately if Connected).
6. **Initialize flags compute** (audit H5) — `lifecycle.rs:59-65` không compute `flags` từ `inherit_cursor`/`text_measurement`/`ambiguous_is_wide`. Add flag computation in `initialize` (per `ConptySettings` fields).
7. **Env block injection** (audit H6) — `lifecycle.rs:277` `CreateProcessW` `None` cho lpEnvironment. Caller app không inject `WT_SESSION`/`WT_PROFILE_ID`/user env overrides. Port `til::env` merge: build env block from `settings.environment` + add `WT_SESSION={session_id}` + `WT_PROFILE_ID={profile_guid}`. Requires `ConptySettings.environment` gate.
8. **EchoConnection** (P06 step 4) — port `EchoConnection.cpp` (47 LOC) trivial: `WriteInput` echoes back to `TerminalOutput`. New file `crates/connection/src/echo.rs`. Trait impl.
9. **CTerminalHandoff decision** (P06 step 3) — ADR-0006 doc stub forwarded to P13 (named-pipe IPC vs COM). Write `docs/decisions/0006-winrt-events-com-replacement.md` final body (status `proposed`, owner P13). NOT implement handoff.
10. **`PtyBackend` trait** (P06 step 5) — extract trait `create/resize/read/write/close` from `ConptyConnection` lifecycle; `ConptyBackend` Windows impl. Connection logic depends only on trait. Doc-comment only if extraction risk regression — minimal: add `pub trait PtyBackend { ... }` + `impl PtyBackend for ConptyBackend` thin wrapper calling existing FFI. Wire `ConptyConnection` to call through `Box<dyn PtyBackend>` field — OR keep current FFI direct calls and add trait as future-ready seam (acceptance: trait exists + 1 test that `ConptyBackend` impl is callable).
11. **`ShowHide`/`ReparentWindow`/`RootProcessHandle`/`Commandline`/`StartingTitle`/`ShowWindow`** (ConptyConnection.h:30-37) — public methods chưa port. P06 §says "full ConptyConnection public behaviour" — port as `pub fn` (ShowHide/ReparentWindow may be no-op on Win32 conhost-removed path; Commandline/StartingTitle are getters on settings).

## Files modify/create

### Create
- `crates/connection/tests/wt_winconpty__create_con_pty_no_pipes.rs` — red test 1
- `crates/connection/tests/wt_winconpty__create_con_pty_bad_size.rs` — red test 2 (may be `#[ignore]` if size-zero rejected at FFI cleanly)
- `crates/connection/tests/wt_winconpty__good_create.rs` — red test 3 (cmd /c exit 0 smoke)
- `crates/connection/tests/wt_winconpty__good_create_multiple.rs` — red test 4
- `crates/connection/tests/wt_winconpty__survives_on_break_output.rs` — red test 5
- `crates/connection/tests/wt_winconpty__dies_on_close.rs` — red test 6 (handle leak check)
- `crates/connection/tests/wt_winconpty__release_pseudo_console.rs` — red test 7
- `crates/connection/src/echo.rs` — EchoConnection port (EchoConnection.cpp 47 LOC)
- `parity/ledger/P06-connection/{files,symbols,tests,decisions}.csv` — ledger shard (seed via xtask or manual)
- `docs/decisions/0006-winrt-events-com-replacement.md` — ADR stub body (status proposed, owner P13)

### Modify
- `crates/connection/src/lib.rs` — add `pub mod echo;` + re-export `EchoConnection`.
- `crates/connection/src/connection/mod.rs` — add `pub trait PtyBackend` + `pub struct ConptyBackend` (if extracted) OR add trait as future-seam only.
- `crates/connection/src/connection/lifecycle.rs` — fix gap 3-7 (process-exit msg, failure sub-msg, ResetSize live-apply, Initialize flags compute, env block injection).
- `crates/connection/src/connection/mod.rs` ConptyConnection struct — add public methods (`show_hide`, `reparent_window`, `root_process_handle`, `commandline`, `starting_title`, `show_window`) per ConptyConnection.h:30-37.
- `crates/connection/src/connection/tests.rs` — add unit tests for env block injection + flags compute + failure messaging (no real spawn needed).

## Verification

1. `cargo check -p connection` clean.
2. `cargo test -p connection` green — 28 existing unit + new unit tests (env/flags/exit-msg). `wt_winconpty__*` integration tests: at least 3 pass live (GoodCreate/DiesOnClose/ReleasePseudoConsole), 4 may be `#[ignore]` if no live ConPTY smoke (mark with comment + ADR-waiver reference if needed).
3. `cargo fmt --check -p connection` + `cargo clippy -p connection --all-targets -- -D warnings` clean.
4. `cargo xtask stubs --path crates/connection` = 0 (or only doc-comment mentions).
5. `cargo xtask ledger-check` — P06 shard passes schema.
6. `cargo xtask audit --wave 1` — regression-only: connection shard not-started count may drop; no NEW red regression in other crates (renderer clippy regression is OUT-of-scope, not introduced by P06).
7. Handle-leak smoke: open+close 1000x in test (gated `#[ignore]` if too slow CI) — handle count delta == 0.

## Pham vi KHONG cham (out-of-scope)

- renderer clippy regression (24 errors) — owner P10 (Wave 3).
- app fmt regression — owner P14 (Wave 6).
- ITerminalConnection trait signature change — frozen by P01/P02; if added method, must be default-impl to not break P02/P11 callers.
- `winconpty.dll` Reparent/ShowHide/Pack/AsUser — replaced by kernel32 ConPTY (architecture decision, not re-port).
- AzureConnection — ADR-0007 out-of-scope.

## Resolved notes

1. **winconpty tests live-spawn gate**: WT `ConPtyTests` uses `_CreatePseudoConsole` FFI helper directly. Rust maps to `ConptyConnection::new().initialize(s).start()`. Tests that need real child process (`cmd /c exit 7`) — `#[ignore]` until `cargo xtask e2e` harness (lane C) wires real spawn. Acceptance: test file EXISTS with correct naming `wt_winconpty__<method>`, compiles, ignored tests marked with `#[ignore = "manual: ..."]` per stubs regex exemption. RED→GREEN for `cmd /c exit 0` smoke if feasible.
2. **PtyBackend trait scope**: extract toàn FFI path là large refactor — risk regression trên C1+C2+C3 fix. Compromise: add `pub trait PtyBackend { fn create/resize/close/... }` + `impl PtyBackend for ConptyBackend` thin wrapper calling existing FFI. Wire `ConptyConnection` to call through `Box<dyn PtyBackend>` field — OR keep current FFI direct calls and add trait as future-ready seam (acceptance: trait exists + 1 test that `ConptyBackend` impl is callable).
3. **ADR-0006 final body**: decision = "COM activation replaced by Rust channels + named-pipe IPC; verified at P13". P06 writes `proposed` stub (status field), P13 finalizes. Do NOT implement IPC now.
4. **Env block format**: `CreateProcessW` lpEnvironment = `KEY=VALUE\0KEY=VALUE\0\0` UTF-16. Build from `settings.environment` + auto-add `WT_SESSION=<uuid>` + `WT_PROFILE_ID=<uuid>` (if profile_guid present). Use `std::env::vars()` for base if `reloadEnvironmentVariables=true` (ConptySettings missing this field — add `reload_env: bool`).
5. **Process exit message resource**: WT uses `ResourceString` `ProcessExited`/`CtrlDToClose` from `Resources.resw`. Rust port hardcode English: `"\r\n[process exited with code {N}]\r\n"` + `"\r\n[press Ctrl+D to close]\r\n"`. ADR-0014 (P14) resorts strings — note in ADR-0006 or new deviation row.

## Open Q (escalate to user if blocked)

1. **`reload_env` field**: ConptySettings needs new `reload_env: bool` (default true) for WT `reloadEnvironmentVariables` key. Breaking change for P11/P14 callers (P11 will rebuild). Accept? → YES (P11 not landed yet, safe to add field).
2. **`isspace("PtyBackend" trait) YAGNI`**: trait used only by ConptyBackend (Windows) for foreseeable future. Mark `#[allow(dead_code)]` or skip trait entirely? → ADD trait per P06 step 5 explicit ("Extract `PtyBackend` trait"), `#[allow(dead_code)]` on unused non-Windows impls.

## Schedule

- 1 cook-worker sonnet (medium model), single brief, disjoint crate.
- Main-loop fix clippy regression o renderer PARALLEL (24 errors mechanical — field_reassign_with_default + hex groups + loop-index + identity-cast).
- Target: P06 closed in 1 worker session + clippy green + audit wave-1 run.
