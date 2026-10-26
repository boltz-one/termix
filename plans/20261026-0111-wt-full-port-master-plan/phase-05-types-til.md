# Phase 05 — `types` + `til` + propslib helpers → types

## Context Links
- `reports/01-inventory-baseline.md`; ADR-0004 in `reports/02-*.md`
- WT: `types/*` (utils.cpp, colorTable, CodepointWidthDetector, Viewport.cpp, convert.cpp, GlyphWidth, inputparser, …, 9.0k LOC), `inc/til/*` (8.6k), `til/ut_til` (5.4k, 198 methods), `propslib/*` (1.9k)

## Overview
Priority P2 · Pending · Wave 1 · ~90h · Milestone M2 enabler. Port behaviour actually used by in-scope code into `crates/types`; everything idiomatic-replaceable by Rust std becomes `replaced` with ADR.

## Key Insights
- `til::point/size/rect/inclusive_rect`, `rle`, `color`, `enumset`, `small_vector`, `pmr`, `string`/`u8u16`, `unicode`, `bitmap`, `generational`… ledger every header.
- `UiaTextRange*` in `types/` is out of this phase → P15.

## Requirements
- Functional: ported types keep arithmetic semantics (clamp/overflow checks, `Viewport` clamp/walk/convert APIs, `CodepointWidthDetector` tables, color ops/`ColorFix`, `utils::` GUID/colour parsing/console helpers).
- Non-functional: `#![forbid(unsafe_code)]` where possible; no deps on core.

## Architecture
`types/src/{point,size,rect,viewport,rle,color,width,unicode,convert,utils,enumset}.rs`; consumed by core (buffer P08), renderer (P10), settings (P07).

## Related Code Files
- Create: `crates/types/src/**`, `crates/types/tests/wt_til_*.rs`, `wt_types_*.rs`.
- Modify: none outside ownership (consumers migrate in their phases).
- Delete: none (duplicate `screen.rs` Viewport in core migrates in P08).

## Implementation Steps
1. Ledger every `til`/`types` header: `port` | `replaced(ADR)` | `out-of-scope(ADR)`.
2. Port `til` tests (198) for ported items; waive/replace for std-equivalents with explicit Rust equivalence test.
3. Implement `rle`, point/size/rect (+checked ops), `Viewport`, colours, width detector (tables from WT), utf conversion helpers.
4. `propslib` pieces referenced by IN code (registry/console props) behind `cfg(windows)`.
5. Publish crate API doc table in `reports/` for consumers.

## Todo List
- [ ] ledger decisions for each header
- [ ] rle/point/size/rect/viewport
- [ ] color/width/convert/utils
- [ ] til+types tests ported

## Success Criteria
- Every `til`/`types` ledger row ∈ {verified, replaced+ADR, out-of-scope+ADR}.
- Given WT `ut_til` vectors for `rle`/`rect`/`size`, Then Rust results equal.
- Given width table samples (CJK, emoji, combining), Then widths equal `CodepointWidthDetector` expected.

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are each Given/When/Then and are the acceptance tests; E2E scenario "viewport-walk": port `ViewportTests` walk/clamp across edge rects.
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 05` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p types`, `cargo xtask ledger-check`.

## ADR Rationale
ADR-0004. Context: `til` exists because C++ std lacked features. Decision: replace where Rust std equals semantics, port where behaviour is custom. Why over blanket port: avoids dead code (YAGNI) while ledger keeps accountability.

## Risk Assessment
Hidden dependence on `til` quirks (overflow saturation) — covered by ported tests. Width tables version drift.

## Security Considerations
Checked arithmetic (overflow) preserved; no unsafe in conversions.

## Parallelization Info
Wave 1; pure new crate, zero overlap.

## File Ownership
`crates/types/**`.

## Conflict Prevention
New crate only; consumers adopt in their own phases (no edits here to core/renderer).

## Next Steps
P08, P10 consume.
