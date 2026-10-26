# Validation Round 2

**Lane:** high-risk | **Round type:** delta | **Lenses run:** A,B,D (C carried PASS r1) | **Scope:** fixes from r1 + touched artifacts

## Lens Results
### 4.A Structural — 1 BLOCKER / 5 MINOR
BLOCKER: broken link `../../phase4-terminal-dispatch.md` (P01, P03). MINOR: shared-crate Cargo.toml owner; matrix omissions (P01 exclusions/no-ops, P02 e2e test); P08 garbled text; P07/P09 criteria without command. All r1 structural fixes re-verified PASS.
### 4.B Consistency — 0 formal BLOCKER; 5 MAJOR (treated as BLOCKER) / 7 MINOR
F1 `--wave` vs `tests-check` exit rule; F3 shared-crate Cargo.toml + e2e test ownership; F4 test naming rule inconsistent; F5 seam implementers missing for wave-1 consumers; F6 P08 `mod buffer_ext` line ownership. MINOR: 188 vs stubs baseline, P06 ADR-0006 cross-edit, "(new)" ADR wording, P12 test/file overclaim, P01 vs audit-log ownership + P19 golden, stub criteria vs markers, xtask serde_json. All 11 r1 blocker fixes re-verified PASS (B10 partial → F3).
### 4.C Fidelity — carried PASS (r1)
### 4.D Testability — PASS (0 BLOCKER / 7 MINOR)
r1 blockers (alias, llvm-cov provisioning, multi-filter syntax) verified fixed; reviewer ran `cargo test -p core -- wt_adapter__ buffer:: --list` OK. MINOR: branch coverage tooling (cobertura), baseline ref/git root, `#[ignore]` vs stubs, stubs regex escapes, author column, e2e manifest, per-phase crate list.

## Fixes Applied
1. Link fixed in P01/P03 (`../phase4-terminal-dispatch.md`); grep for `\.\./\.\./phase4` → 0 hits.
2. reports/02: `--wave` tests-check report-only + writes summary.json (F1,F2); single naming rule `wt_<suite>__<method>` (F4); `xtask cov --phase NN` with cobertura branch check, `$BASE`/baseline-ref, git root (D1,D2,D7); `#[ignore = "manual: …"]` exemption + regex code block (D3,D4); `author` column (D5); `parity/e2e.csv` manifest (D6).
3. P01: `HeadlessHost` + moved associated types, phases.csv/e2e.csv/baseline-ref, shared-crate manifest pre-declaration, P02-only stub-downstream removal, round-0 audit ownership, serde_json (F3,F5,M5,M7).
4. plan.md matrix/rules updated (F3,F6, matrix gaps); P02 Cargo.toml scope; P08 dispatch/mod.rs line; P13/P14 e2e test ownership; P12 test/file scope (M4); P03 criteria + HeadlessHost use (F5,M6,M1); P06 ADR wording (M2); ADR "(new)" (M3); P07/P09 criteria; P16 `#[ignore = "manual:…"]`.
Hygiene greps run: `xtask cov -p`, `\.\./\.\./phase4`, `wt_<class>`, placeholders, 17 headers x 19 phases → 0 stale hits.

**BLOCKERs this round:** 6 (1 + 5 major) | **Round counts as clean:** no
**Exit state:** continue → delta round 3 re-running lenses A, B (D, C carried PASS); streak reset
