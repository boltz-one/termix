# Phase 10 — Renderer (base + Atlas behaviour on GPUI) + fonts

## Context Links
- `reports/02-*.md` (ADR-0003); existing `crates/renderer/src/*` (1,997 LOC), `crates/font/src/*` (1,013 LOC)
- WT: `renderer/base/{renderer.cpp,RenderSettings.cpp,fontinfo.cpp,FontInfoBase.cpp,FontInfoDesired.cpp,RenderEngineBase.cpp,CSSLengthPercentage.cpp,FontResource.cpp}` (3,310), `renderer/atlas/*` (10,269: AtlasEngine.cpp, BackendD3D.cpp, BackendD2D.cpp, BuiltinGlyphs.cpp, shader_*.hlsl, Backend.cpp, DWriteTextAnalysis.cpp, dwrite_helpers.cpp), `renderer/inc/*`

## Overview
Priority P1 · Pending · Wave 3 · ~260h · Milestone M3/M5. Port renderer orchestration (`Renderer`: invalidation, scroll optimisation, paint scheduling, cursor blink, selection/search highlight, hyperlink underline, overlays) and the *behaviour* of AtlasEngine on GPUI: font fallback/axes/features/ligatures, bold/italic/faint/blink/underline styles (curly, dotted, dashed, double), strikethrough, overline, grid lines, box-drawing/block/powerline built-in glyphs, DECDHL/DECDWL line rendition, wide/emoji/colour fonts, cursor shapes (bar, underscore, empty box, filled box, double underscore, `DECSCUSR`), background image, opacity/acrylic hooks, retro effect + custom pixel shader (re-expressed or waived), dpi scaling, cell-size computation (`FontInfo`).

## Key Insights
- ADR-0003: GPUI paint replaces D3D11 atlas backend; parity = per-cell output (glyph, colours, attrs, position, clipping) + settings semantics, verified by golden `CellPayload` dumps and image snapshots with tolerance.
- Existing `paint.rs/grid.rs/cell.rs/payload.rs` is a base; unknown how many attrs handled → ledger step 1.
- Reads terminal state only through `IRenderData`/`RenderState` (frozen by P01; implemented by P09); P10 does not implement `RenderModeSink`. Adds a preedit-overlay hook (`PreeditSink`) consumed by P15.

## Requirements
- Functional: every `RenderSettings` key (`ColorTable`, `ScreenReversed`, `BlinkingAllowed`, `IntenseIsBold/Bright`, `AdjustIndistinguishableColors`, `DECAWM`, …) and `FontInfo`/`FontConfig` keys honoured; wide glyph/ligature behaviour per WT (`textMeasurementMode`, `builtinGlyphs`, `colorGlyphs`).
- Non-functional: 60fps at 200×60 with scrolling output; <16ms frame budget measured; no per-frame heap churn.

## Architecture
`renderer`: `Renderer` (invalidate/paint driver) → `RenderEngine` trait (kept from WT) → `GpuiEngine` (GPUI scene builder using `font` shaping + builtin glyph rasterizer); `font`: font resolution, metrics, features/axes parsing (`FontInfo` parity).

## Related Code Files
- Modify/Create: `crates/renderer/**`, `crates/font/**`, tests `crates/renderer/tests/wt_renderer_*.rs`, snapshots `parity/golden/render-*`.
- Delete: none.

## Implementation Steps
1. Ledger renderer/base+atlas symbols/behaviours; define "visual feature matrix" (rows = attr/feature, cols = WT behaviour, test).
2. Port `RenderSettings`, `Renderer` invalidation/scroll/cursor logic with tests derived from WT behaviours (no WT unit tests exist → spec-by-behaviour tests written from C++ reading, reviewed by audit).
3. Font stack: families/fallback/features/axes/`FontInfoDesired→FontInfo` cell metrics.
4. Style rendering: underline variants, strike, overline, blink, faint, reverse, selection/search colours, min-contrast adjust.
5. Builtin glyphs (box/block/braille/powerline/sextant) drawn procedurally.
6. Line rendition + wide/emoji; cursor shapes/colours; background image/opacity.
7. Effects: retro scanlines/pixel shader via GPUI shader hooks if available else ADR-waive with explicit list.
8. Perf harness + snapshot tests.

## Todo List
- [ ] feature matrix
- [ ] RenderSettings + Renderer
- [ ] fonts
- [ ] styles/glyphs/cursor/rendition
- [ ] effects decision
- [ ] perf + snapshots

## Success Criteria
- Every feature-matrix row has a passing test or ADR waiver; `renderer/**` ledger rows verified/replaced.
- Given golden buffer with all SGR styles, Then `CellPayload` dump equals golden and image snapshot within tolerance.
- Given 200×60 scrolling stream benchmark, Then frame time p95 < 16ms (number recorded in `reports/`).

## Test Strategy & Quality Gate
- G/W/T: Success Criteria bullets 2-3 are the acceptance tests; E2E scenario "sgr-showcase": render fixture, diff against `parity/golden/render-*` snapshots (golden image files, per-pixel tolerance 2/255 over ≤0.1% pixels, compared by a small helper in `renderer/tests/`).
- Coverage target: diff coverage on this phase's new/changed code line ≥90% AND branch ≥75% (SSOT `workflows/quality-gate.md` §5); measure with `cargo xtask cov --phase 10` (nightly llvm-cov + diff-cover, report 02 §3); GUI/Win32 glue is covered via integration/snapshot tests, any irreducible uncovered lines listed in the phase report.
- Evidence: `cargo test -p renderer -p font`, `cargo bench -p renderer`.

## ADR Rationale
ADR-0003. Context: Atlas relies on D3D11 + DirectWrite internals not exposed by GPUI. Decision: behavioural parity on GPUI. Why over embedding D3D backend: GPUI owns the swapchain/layout; double-rendering breaks integration and portability of the UI layer.

## Risk Assessment
GPUI text API limits (ligatures/axes/colour fonts); shader effects; perf; vendored GPUI churn.

## Security Considerations
Font files untrusted: use platform loaders, bound glyph cache growth; background image decode limits/format allowlist.

## Parallelization Info
Wave 3 with P09 (disjoint crates).

## File Ownership
`crates/renderer/**`, `crates/font/**`, `parity/golden/render-*`.

## Conflict Prevention
Reads core via `IRenderData`/`RenderState` only (traits frozen in P01, so P10 does not wait for P09); settings via settings structs (read-only); no edits to those crates.

## Next Steps
P11 hosts renderer; P15 adds UIA hooks.
