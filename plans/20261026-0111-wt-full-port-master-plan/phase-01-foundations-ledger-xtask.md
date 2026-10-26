# Phase 01 — Foundations: ledger, xtask audit, crate skeletons, frozen contracts, Phase 4.0 gaps

## Context Links
- `reports/01-inventory-baseline.md`, `reports/02-parity-ledger-and-audit-loop.md` (ledger/xtask spec, ADR registry — do not restate)
- Existing: `../phase4-terminal-dispatch.md` § Phase 4.0 (buffer API gaps)

## Overview
Priority P1 · Status pending · Wave 0 (serial, blocks everything) · Effort ~100h · Milestone: enabler for M1-M5.
Create audit machinery, freeze structure and cross-phase contracts so later phases run in parallel without touching shared files.

## Key Insights
- Parallel safety depends on pre-created registries and frozen contracts; later phases only add files inside owned paths.
- No ledger = no proof of "100%"; seed it from the filesystem, not memory. Sharded per phase to avoid same-wave CSV conflicts.
- `parser.rs`/`buffer.rs`/`app` single files must become directories before phases 04/08/12 run.

## Requirements
- Functional: `cargo xtask {audit --wave N|--final, stubs, ledger-check [--root], tests-check, include-graph, loc-ratio, ledger-seed, merge, cov, e2e}` exactly per report 02 §3; `.cargo/config.toml` alias; shards seeded; ADR 0001+0008 final, ADR 0002-0007,0009-0012 stubs (status `proposed`, owner phase named).
- Non-functional: xtask deps: std + serde + csv + serde_json only; mechanical commands < 60s excluding `cargo test`.

## Architecture
Workspace additions (empty `lib.rs` + `mod` placeholders): `crates/xtask`, `types`, `a11y`, `host`, `settings::editor`, `extras`, `apps/terminal` (bin `terminal`, stub main). core restructured (moves only): `buffer/mod.rs`, `parser/mod.rs`, `terminal_input/`, `terminal/`, `dispatch/`. app restructured (moves only): `page.rs→page/mod.rs`, `pane.rs→pane/mod.rs`, `tab.rs→tab/mod.rs`, plus empty `actions/ cmdline/ remoting/ ui/`; `app.rs`, `downstream.rs` stay. Cross-crate dependency edges pre-declared in each `Cargo.toml`.
Frozen contracts (signatures only, later phases implement/consume): `core/src/dispatch/seams.rs` = `InputModeSink`, `RenderModeSink`, `ITerminalApi`, `IRenderData` (moved from `renderer/src/paint.rs:29` together with its associated types such as `RenderFrameInfo` (`paint.rs:17`); renderer re-exports) + `HeadlessHost` (in `dispatch/headless_host.rs`; created by P01, owned by P03 afterwards; real minimal implementation of `ITerminalApi`/`InputModeSink`/`RenderModeSink` that stores state in plain fields, port of WT `TestGetSet` semantics; used by the P02 skeleton and by P03/P04 tests until P09 lands) + plain struct `RenderState` (screen-reversed, cursor style/colour, palette overrides; written by `Terminal` (P09) via `RenderModeSink`, read by P10 through `IRenderData`); `terminal_input/events.rs` = `InputEvent`; `settings/src/terminal_settings.rs` = minimal `TerminalSettings` (fields derived from existing profile/appearance/global structs; P07 may add fields additively).

## Related Code Files
- Create: `dispatch/headless_host.rs`, `parity/summary.json` (round 0), `reports/audit-001.md`, `.cargo/config.toml`, `parity/{phases,e2e}.csv` + `parity/baseline-ref.txt`, `crates/xtask/**`, `parity/ledger/pNN-*/{files,symbols,tests,decisions}.csv` + `parity/exclusions.csv`, `docs/decisions/0001..0012-*.md`, `NOTICE`, skeleton crates, `seams.rs`, `events.rs`, `terminal_settings.rs`.
- Modify: root `Cargo.toml`, `crates/core/src/{lib.rs,buffer.rs→buffer/mod.rs,parser.rs→parser/mod.rs}`, `crates/app/src/{lib.rs,page,pane,tab moves}`, `crates/renderer/src/paint.rs` (IRenderData import only), `core/src/dispatch/{mod.rs,interact.rs,interact_impl.rs,td/modes.rs}` (replace `todo!` defaults with logged non-panicking no-ops marked `stub` so P02's skeleton never panics; P03 replaces them with real bodies), `crates/core/src/buffer/*` (Phase 4.0: `scroll_rows`, `get_last_non_space_character`, `clamp_position_within_line`, `reset_line_rendition_range`, `clear_scrollback`, `trigger_scroll`, `set_current_line_rendition`, `copy_row`), `dispatch/interact.rs` trait ext (8 methods).
- Delete: none.

## Implementation Steps
0. Operator step (outside automation; committing is the user's decision): create the baseline git commit and write its ref to `parity/baseline-ref.txt` before restructuring so `xtask cov` has a `$BASE`.
1. Install tooling: `cargo install cargo-llvm-cov`, `rustup toolchain install nightly`, `rustup component add llvm-tools-preview --toolchain nightly`, confirm `diff-cover` on PATH; add `.cargo/config.toml` alias `xtask = "run -p xtask --"`.
2. Write ADR-0001 (scope; mark assumption for user confirmation) + ADR-0008 (MIT/NOTICE) final; create stubs for the other ADRs.
3. Record the pre-conversion `cargo xtask stubs`-equivalent baseline (171 core + 1 app, via a one-off comment-stripped grep until xtask exists) in the draft audit report, then create skeleton crates/dirs; perform moves; fix imports; `cargo check --workspace` green after each move.
4. Implement `ledger-seed`: walk in-scope `windows/src` dirs → per-phase shards `files.csv` (status `not-started`; existing ports pre-marked `partial` via hand-verified mapping); seed `tests.csv` from `TEST_METHOD`/`BEGIN_TEST_METHOD` with data-row expansion lists for ut_adapter/ut_parser and suite names matching the `wt_<suite>__<method>` rule; out-of-scope files go to `exclusions.csv` with ADR-0001 (and `TerminalConnection/AzureConnection*` with ADR-0007). Owning phase per file follows the mapping list in step 5 (apply it while seeding).
5. Seed `symbols.csv` for high-risk modules (regex over `adaptDispatch.hpp`, `ITermDispatch.hpp`, `textBuffer.hpp`, `Row.hpp`, `terminalInput.hpp`, `Terminal.hpp`, `ControlCore.h`, `ControlInteractivity.h`, `ActionAndArgs.h`, `CascadiaSettings.h`); hand-review counts vs headers. Assign each WT file exactly one owning phase (e.g. `WindowEmperor.cpp`→P16 with P13 consuming the IPC part; `TaskbarState.cpp`→P12; Markdown/Snippets/Scratchpad content→P14; `ut_adapter` input tests→P04, rest→P03; `CommandlineTest`→P13; pane/tab LocalTests→P12; palette tests→P14).
6. Implement all xtask commands incl. `parity/phases.csv` (phase → crates/paths, from the plan matrix) and `parity/e2e.csv` (every e2e scenario named in the phases: command + `ci`/`manual`), `merge`, `cov`, `e2e`, `audit --wave/--final`; write `audit-001.md` baseline.
7. Pre-declare in `crates/core/Cargo.toml` and `crates/app/Cargo.toml` every dependency/feature later phases need (e.g. `windows` features for named pipes/UIA/TSF, tray, shell), so shared-crate manifests are not edited again except append-only at barriers; P02 alone removes the `stub-downstream` feature from `app/Cargo.toml` (serial, before wave 5). Freeze contracts (seams, events, TerminalSettings) with doc comments; TDD Phase 4.0 buffer methods (anchors per phase4 plan). Also convert every code `todo!`/`unimplemented!` default in `dispatch/{mod,interact,interact_impl,td/modes}.rs` into a logged `// STUB` no-op, and add a temporary crate-level `#![allow(non_snake_case)]` to `core/src/lib.rs` (the only line P03 may later remove, when it renames the PascalCase fns) so `clippy -D warnings` is clean at the wave-0 barrier.
8. Run `cargo xtask audit --wave 0` and record round-0 numbers (the audit report number NNN auto-increments; this first run writes `reports/audit-001.md`, replacing any draft; `parity/summary.json` round 0 and the first plan.md `## Audit Log` entry).

## Todo List
- [ ] tooling + alias
- [ ] ADR 0001/0008 final + stubs, NOTICE
- [ ] skeleton crates + moves, workspace green
- [ ] frozen contracts + `// STUB` no-op conversion + temporary `allow(non_snake_case)` (clippy clean)
- [ ] ledger shards seeded + exclusions
- [ ] xtask full command set + `audit-001.md`
- [ ] Phase 4.0 buffer/interact gaps with tests

## Success Criteria
- `cargo check --workspace && cargo test --workspace` pass after restructuring (test count ≥ 410, none lost).
- `cargo xtask ledger-check` exits 0 and the sum of the shard `files.csv` row counts + `exclusions.csv` row count equals the file count of `find windows/src -type f \( -name '*.cpp' -o -name '*.h' -o -name '*.hpp' -o -name '*.idl' -o -name '*.xaml' \)` exactly (no tolerance).
- Given a fresh checkout, When `cargo xtask audit --wave 0` runs, Then it exits 0 (no regression) and writes `audit-001.md` with per-crate stub counts; `cargo xtask audit --final` exits non-zero (work remaining).
- `cargo xtask stubs` baseline equals the comment-stripped count (171 core + 1 app) before the no-op conversion, and afterwards counts each new `// STUB` no-op marker, recorded in `audit-001.md`.
- Phase 4.0 new buffer tests pass.

## Test Strategy & Quality Gate
- Acceptance (G/W/T): Given `parity/ledger/**` shards, When a WT file row is deleted in a temp copy and `cargo xtask ledger-check --root <tmp>` runs, Then it fails with an orphan-file error; restoring the row makes it pass.
- E2E scenario "ledger-orphan-roundtrip": exactly the step above, executed by `crates/xtask/tests/ledger_orphan.rs` on a temp copy (never mutates real files).
- Pyramid: ~70% unit / 20% integration / 10% e2e (stated per quality-gate §4).
- Coverage target: diff coverage line ≥90% AND branch ≥75% on this phase's new/changed code (SSOT `workflows/quality-gate.md` §5), measured by `cargo xtask cov --phase 01`.
- Evidence: `cargo test -p xtask`, `cargo test -p core -- buffer::`, `cargo xtask ledger-check`.

## ADR Rationale
Context: need provable completion. Decision: sharded CSV ledger + Rust xtask with barrier/final audit modes. Why over alternatives: CSV diffs cleanly and shards avoid parallel-write conflicts; xtask is cross-platform and joinable with `cargo test --list`; a markdown table cannot be machine-checked.

## Risk Assessment
Restructure breaks imports (moves only, compile after each). Regex symbol seeding misses overloads (hand review step 5). Nightly toolchain drift for branch coverage (fallback in report 02 §3 + ADR waiver).

## Security Considerations
Not network-facing. xtask never writes into `windows/`. Keep license headers/NOTICE (MIT) intact. Tool installs from crates.io/PyPI only with pinned versions recorded in `reports/`.

## Parallelization Info
Runs alone (wave 0). Everything else waits on its barrier audit.

## File Ownership
Exclusive (wave 0): `.cargo/**`, `parity/{phases,e2e}.csv`, `parity/{baseline-ref.txt,summary.json}`, `reports/audit-001.md`, `core|app` `Cargo.toml` pre-declared deps, `renderer/src/paint.rs` (IRenderData move only), `dispatch/headless_host.rs` (creation), `core/src/lib.rs` (modules + temporary `allow(non_snake_case)`; P03 may remove only that line), `dispatch/{interact_impl.rs}` no-ops, root `Cargo.toml` (later phases append-only under `[workspace.dependencies]`), `crates/xtask/**` (P19 extends only summary/fuzz glue), `parity/ledger/pNN-*/` seed shards (creation; each phase owns its shard afterwards), `parity/exclusions.csv`, `docs/decisions` stubs, `NOTICE`, `dispatch/seams.rs`, `terminal_input/events.rs`, `terminal_settings.rs` (creation), skeleton crates, all file moves, Phase 4.0 edits to `buffer/**` + `dispatch/interact.rs`, no-op defaults in `dispatch/{mod.rs,td/modes.rs}`.

## Conflict Prevention
Serial wave; afterward each phase owns its crate (incl. its `Cargo.toml`), `parity/ledger/pNN-*/**` shard and its ADR files; `lib.rs` module lists are complete from P01 (later phases add files, not module lines, except inside their own dirs).

## Next Steps
Unblocks P02-P07 (wave 1).
