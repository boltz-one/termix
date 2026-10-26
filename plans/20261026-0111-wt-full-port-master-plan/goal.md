# Goal: Windows Terminal -> termix full port (100% logic parity)

> **Execution flow: /vk-hoh-run** — spec = `terminal_gpui/plans/20261026-0111-wt-full-port-master-plan/spec.md`, loops = 3.
> Inner loop: /vk-hoh-plan → /vk-hoh-dev → /vk-hoh-qa per loop × T.

## Mission
Port all in-scope Windows Terminal C++/WinRT logic (`windows/src`, read-only upstream) to Rust+GPUI in `terminal_gpui/`, done when `cargo xtask audit --final` exits 0 on 2 consecutive runs (ledger 100% verified/replaced/out-of-scope, WT tests ported+passing, 0 stubs).

## Context & Key Files
- Master plan: `terminal_gpui/plans/20261026-0111-wt-full-port-master-plan/plan.md` (19 phases, waves, ownership matrix)
- Phases: `terminal_gpui/plans/20261026-0111-wt-full-port-master-plan/phase-01-*.md` ... `phase-19-*.md`
- Ledger/audit/ADRs: `terminal_gpui/plans/20261026-0111-wt-full-port-master-plan/reports/02-parity-ledger-and-audit-loop.md`
- Baseline numbers: `terminal_gpui/plans/20261026-0111-wt-full-port-master-plan/reports/01-inventory-baseline.md`
- Validation history: `terminal_gpui/plans/20261026-0111-wt-full-port-master-plan/reports/validation-round-*.md`

## Requirements
**Must do:**
- Execute phases in wave order (P01 first); respect each phase's File Ownership; parallel only within a wave (<=4 workers).
- Port WT tests first (red), implement, then green; test names `wt_<suite>__<method>`.
- At each wave barrier run `cargo xtask audit --wave N`; `verified` is set only by independent reviewers.
- Keep WT license headers/NOTICE (MIT); record deviations as ADRs.

**Must not:**
- Edit or commit anything under `windows/`.
- Edit files outside the phase's owned paths; no mocks/fakes to pass tests.
- Declare completion without `audit-NNN.md` evidence.

## Success Criteria
- `cargo xtask audit --final` exit 0 twice in a row (`new_gaps == 0`).
- `cargo xtask stubs` = 0; `cargo xtask tests-check` has no `not-started` in-scope rows.
- fmt, `clippy -D warnings`, `cargo test --workspace`, release build green.
- `cargo xtask cov` diff coverage: line >=90%, branch >=75% per crate.
- `cargo xtask e2e` green (ci rows) and manual checklists signed.
- M1: `cargo run -p terminal` shows a working pwsh session.

## Out of Scope
- conhost `host/`, `server/`, GDI/WDDM, audio, tools, WPF/PublicTerminalCore/HwndTerminal (ADR-0001)
- AzureConnection / Cloud Shell (ADR-0007); MSIX packaging (ADR-0010)

## Verification
```bash
cd terminal_gpui && cargo xtask audit --final && cargo xtask e2e
```
