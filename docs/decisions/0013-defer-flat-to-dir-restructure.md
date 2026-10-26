# ADR-0013 — Defer flat→dir module restructure (keep flat layout in `core`)

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** main agent (P01), per master plan §6 ADR registry (deviation, separate from 0001–0012)
- **Phase owner:** P01

## Context

Spec report 02 (`02-parity-ledger-and-audit-loop.md`) and the master plan do not
mandate a particular crate-internal module layout. A natural split would push
`core`'s larger modules into directories (`buffer/mod.rs`, `parser/mod.rs`,
`sgr_stack/mod.rs`, `terminal_input/mod.rs`) — closer to WT's `out/` and
`parser/` folder shape and easier to grow in P04/P08.

However, by the time the backfill reached this decision, phases P02–P04
(buffer, parser, dispatch) had already landed imports against the **flat**
layout (`core::buffer::...`, `core::parser::...` from `renderer`,
`control`, `app`, `extras`, and the `xtask` orphan/ledger tests).
A restructure now is a mechanical rename that ripples across every consumer
crate with no offsetting parity gain; it is pure churn at the wrong time.

## Decision

- Defer the flat→dir restructure of `core` (`buffer`, `parser`, `sgr_stack`,
  `terminal_input`) until **P08 (buffer-parity)** or later, when the buffer
  module is the active phase and the rename cost is paid once alongside real
  buffer work.
- Keep `core` at its current flat layout for P03 (vt-adapter / dispatch)
  and P04 (vt-parser): no `mod.rs` reshuffles, no `pub use` re-export churn.
- `dispatch/` and `terminal_input/` subdirs (introduced during the contract
  backfill) stay as directories because they already own multiple files
  (`seams.rs`, `headless_host.rs`, `.../events.rs`) and no consumer imports
  a path that a rename would break.

## Consequences

- Cross-crate imports in P02–P04 stay stable; no rename patch lands out-of-phase.
- The deferral is recorded (this ADR) so `xtask audit` does not flag the
  shape mismatch with WT's `out/` tree as an unexplained gap.
- A future phase (P08+) re-opens this decision if buffer work makes the split
  pay for itself; until then the cost outweighs the benefit.

## Alternatives considered

- **Restructure now (P01 backfill).** Rejected: breaks imports in `renderer`,
  `control`, `app`, `extras`, `xtask` tests for zero parity value;
  P03 owns dispatch files, not layout, so it is the wrong phase anyway.
- **Restructure per phase as it owns the module.** Rejected for the buffer
  case: P04 (parser) lands before P08, and a mid-stream rename of `parser.rs`
  while P04 is moving parser files would collide with the parser port. Doing
  all of it once at P08 is cheaper than doing parser-at-P04 and buffer-at-P08.
- **Pin flat forever, no ADR.** Rejected: an unwritten deviation is an audit
  gap; `include-graph` / orphan sanity could later surface the WT folder
  mismatch with no explanation, so the deviation is recorded here.
