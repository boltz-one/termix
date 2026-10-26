# Phase 19 — Parity audit & closure (the loop)

## Context Links
- `reports/02-parity-ledger-and-audit-loop.md` (procedure §5, stop criteria §4 — SSOT, do not restate); `parity/*.csv`; `crates/xtask`

## Overview
Priority P1 · Pending · Wave 9 (but the loop runs at EVERY wave barrier from P01) · ~120h · Milestone Done. Final independent verification and closure: drive ledger to 100%, resolve residual gaps, add fuzz/e2e/perf/regression suites, include-graph promotion check, final two clean audit rounds.

## Key Insights
- Audit reviewers are fresh `code-reviewer` agents with only ledger rows + WT/Rust sources (never worker summaries) to avoid self-preference bias.
- Gap churn guard: >20 new gaps in one module for 5 rounds → escalate to user.

## Requirements
- Functional: all stop criteria (report 02 §4) met on two consecutive rounds; `plan.md` Audit Log complete; ADR registry consistent with `decisions.csv`.
- Non-functional: audit run < 30 min excluding human review.

## Architecture
`cargo xtask audit --final` orchestrates mechanical checks; review rounds produce `reports/audit-NNN.md` + `parity/summary.json`; `include-graph` promotes OUT files referenced by IN code into the ledger.

## Related Code Files
- Modify: `crates/xtask/**` (summary/fuzz glue), `parity/merged/**`, `plans/*/reports/audit-*.md`, `plans/*/plan.md` (Audit Log only), fuzz targets under `fuzz/**` (create).
- Create: `fuzz/**` (parser/dispatch/settings JSON/cmdline), `parity/golden/**` additions.
- Delete: none.

## Implementation Steps
1. Run audit; triage every non-terminal row (assign back to owning phase if work remains).
2. Include-graph pass: promote referenced OUT files (e.g. `interactivity/*`) → IN rows + ownership addendum.
3. Review rounds: fresh reviewers per module (100% high-risk, ≥25% elsewhere) → verified/partial.
4. Add fuzzers (parser, dispatch, settings JSON, `wt` cmdline) with time-boxed run (≥30 min each) and fix crashes.
5. Performance regression suite vs baselines recorded in P10/P04.
6. Final two consecutive clean audits → mark plan `completed`.

## Todo List
- [ ] audit triage
- [ ] include-graph promotions
- [ ] review rounds to 100% verified
- [ ] fuzz + perf suites
- [ ] two clean rounds

## Success Criteria
- `cargo xtask audit --final` exits 0 twice consecutively with `parity/summary.json` showing 0 non-terminal rows, 0 stubs, 0 un-ported in-scope tests.
- Given fuzz corpus run ≥30 min per target, Then no crashes/hangs.
- plan.md frontmatter `status: completed` set only after the above evidence is linked.

## Test Strategy & Quality Gate
- G/W/T: Given the final tree, When `cargo xtask audit --final` runs, Then exit 0 and report lists all criteria PASS.
- E2E scenario: full-suite `cargo test --workspace` + shell smoke + vttest goldens + GUI snapshots.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 19` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo xtask audit --final`, `cargo xtask e2e`, `cargo xtask cov --phase <each NN>`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.

## ADR Rationale
Context: "100%" claims historically unverifiable. Decision: closure only via mechanical audit + independent review twice. Why over self-attestation: avoids fabricated completion.

## Risk Assessment
Late discovery of whole missing modules (mitigated by file-level orphan check); reviewer cost; flaky GUI snapshots.

## Security Considerations
Fuzzing for memory/DoS in parsers and settings loader; dependency audit (`cargo audit`/`cargo deny`) before release; licence check (MIT notices, third-party `fzf`, etc.).

## Parallelization Info
Wave 9 (after all). Review rounds fan out per module (≤4 concurrent reviewers).

## File Ownership
`crates/xtask/**` (summary/fuzz glue only, after P01), `parity/merged/**` (generated), `parity/golden/**` additions, `fuzz/**`, plan `## Audit Log` and `reports/audit-*.md` (append-only; the barrier runner appends at earlier barriers).

## Conflict Prevention
Runs after all phases; fixes discovered are done by reopening owning phase (serial, one phase at a time per module) — P19 itself does not edit product crates.

## Next Steps
Release/maintenance plan (out of scope).
