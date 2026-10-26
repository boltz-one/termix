# Validation Round 1

**Lane:** high-risk | **Round type:** deep (r1)
**Lenses run:** A,B,C,D | **Scope:** whole bundle

## Lens Results
### 4.A Structural — 1 BLOCKER / 5 MINOR
BLOCKER: plan.md 85 lines (>80). MINOR: matrix omitted `parity/golden/*`, test globs, `fuzz/**`; P01 "Exclusive" overstated; page.rs/page/ handoff unstated; weak checklist criteria (P11/P12); P01 "± documented exclusions". Checklist 4.A.1-.7: all other items PASS (17 headers x 19 phases, table↔files, no placeholders, no same-wave overlap, ADR+Security present, links resolve).
### 4.B Consistency — 11 BLOCKER / 12 MINOR
B1 P02 vs P03 `todo!` edits; B2 page.rs vs page/ + downstream.rs owner; B3 P14→P13 same-wave dependency; B4 `IRenderData`/`ITerminalApi` owner + P09↔P10; B5 RenderModeSink 3 claimants; B6 TerminalSettings owner; B7 `parity/**` exclusivity; B8 barrier vs audit exit code; B9 xtask ownership + alias; B10 Cargo.toml/lib.rs exclusivity; B11 ADR ordering/ownership. MINOR M1-M12 (registry 0009/0010, ADR naming, ADR-0005 reuse, packaging question, test-ownership overlaps, double-assigned WT files, stale P02 wording, P08/P09 out-of-ownership edits, P02 API stability, matrix gaps, milestone drift, test-count drift).
### 4.C Fidelity — PASS (0 BLOCKER / 6 MINOR)
Commands run by reviewer: find/ls over windows/src (2093 files), 231 path tokens checked, LOC + TEST_METHOD + todo! recounts, Rust path/line checks. MINOR: `Reflow.cpp`, `types/UiaTextRange.cpp` nonexistent, todo! count basis (171 vs 188), P06 stale todo, Remoting wording, glob abbreviations.
### 4.D Testability — 3 BLOCKER / 5 MINOR
BLOCKER: no `cargo xtask` alias; `cargo-llvm-cov` not provisioned / incomplete measurement command; invalid multi-filter `cargo test` syntax. MINOR: zero-match filters/naming collisions (`wt_rect`), weak inline G/W/T, e2e steps missing (P13/14/16), manual-only evidence, e2e dependencies + stop-criteria measurability.

## Fixes Applied
1. plan.md rewritten (58 lines): single matrix with links, ownership fixes (B2,B3,B7-B10), milestones aligned (M11), scope sentence aligned, Open Question 4 (M4).
2. reports/02: sharded ledger + golden ownership (B7), `audit --wave/--final`, `merge`, `cov`, `e2e`, stubs regex, reviewer≠worker rule, summary.json fields, alias, naming rule `wt_<suite>__<method>` (B8,B9, 4.D), ADR 0009-0012 + stub rule (B11, M1, M3), ADR naming (M2).
3. reports/01: todo! counts basis, 759 test sum, WinRTUtils row, Remoting wording (4.C, M12).
4. phase-01 rewritten: tooling, alias, frozen contracts (`seams.rs` incl. ITerminalApi/IRenderData/RenderState, `events.rs`, `TerminalSettings`), dir moves incl. app, no-op dispatch defaults, exclusions, ADR stubs, file-owner assignment, exact count criterion (B1,B2,B4-B6,B9-B11).
5. P02 (ownership/steps/no core edits, B1,B2,M7,M9), P03/P09/P10 (seam implementers, B4,B5), P07 (ADR-0011, TerminalSettings), P08 (compat re-exports, M8), P12 (ut_app removed, absorbs page/mod.rs, M5/M6), P13/P14 (B3), P15 (PreeditSink), P16 (wording), P17 (ADR-0012), P19 (ownership), fidelity MINORs, test filter syntax, `xtask cov` commands, inline G/W/T, e2e steps, manual-checklist files.
Hygiene greps run (§3): `xtask audit\``, `llvm-cov`, `parity/(files|symbols|tests).csv`, `parity/**`, `extended per phase`, `ADR-0005`, placeholders, headers x 17 x 19 → remaining hits intentional (ADR-0005 owner P12; refs from P11/P14 resolved by P01 stub).

**BLOCKERs this round:** 15 | **Round counts as clean:** no
**Exit state:** continue → delta round 2 re-running lenses B, D (+A always); C carried PASS (r1)
