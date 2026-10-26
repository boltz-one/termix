# Spec: WT -> termix full port

See `plan.md` (phases, waves, ownership) and `reports/02-parity-ledger-and-audit-loop.md` (ledger, audit loop, stop criteria). Context: phase files `phase-*.md`, baseline `reports/01-inventory-baseline.md`.

## Success criteria (verifiable)
1. M1: `cargo run -p terminal` opens a GPUI window with a working pwsh session; `cargo test -p control --test e2e_shell_smoke` passes.
2. M2: 0 stubs in core; ut_adapter/ut_parser/ut_textbuffer/UnitTests_TerminalCore rows `passing`/`waived+ADR`; vttest goldens equal.
3. M3-M5: UnitTests_Control, UnitTests_SettingsModel, LocalTests_TerminalApp, CommandlineTest, ut_app ported and passing; manual checklists `reports/pNN-*-checklist.md` signed.
4. Ledger (`parity/merged`): 100% rows in {verified, replaced, out-of-scope}; each replaced/out-of-scope has an ADR.
5. `cargo xtask audit --final` exit 0 on 2 consecutive runs; fmt, clippy -D warnings, test, release build green; diff coverage line>=90% / branch>=75%.

## Constraints
- `windows/` read-only; MIT notices kept; no edits outside phase File Ownership.
- Out of scope: conhost/host/server/GDI/WPF, AzureConnection, MSIX.

## Verification commands
`cd terminal_gpui && cargo xtask audit --final && cargo xtask e2e`
