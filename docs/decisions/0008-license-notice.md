# ADR-0008 — License attribution for the WT port

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** main agent (P01), per master plan §6 ADR registry
- **Phase owner:** P01

## Context

WT is MIT licensed. Ported Rust files derived from `windows/src` must carry MIT
attribution and retain the Microsoft copyright notice. The upstream tree
(`terminal/windows/`) is a read-only reference submodule and must never be edited.

## Decision

- WT upstream remains read-only; port workers never edit files under `terminal/windows/`.
- Ported `.rs` files keep a short SPDX-style license header referencing the MIT License
  and the Microsoft copyright line. Template:

  ```rust
  // SPDX-License-Identifier: MIT
  // Ported from Windows Terminal (Microsoft). MIT License; copyright notices retained.
  ```

- A `NOTICE` file lives at repo root (`terminal_gpui/NOTICE`) stating the Microsoft MIT
  origin and that original license/copyright notices are retained in ported source files.

## Consequences

- License compliance is mechanical: header presence is checked at audit barriers
  (`xtask audit --wave N` flags ported files lacking the SPDX header).
- Upstream is preserved as a reference baseline; diffs against upstream serve as the
  parity input without mutating the source of truth.

## Alternatives considered

- Copy the full Microsoft LICENSE text into every ported file — rejected: noisy; the
  SPDX identifier plus repo-root NOTICE is the conventional minimal attribution.
- No headers, NOTICE only — rejected: loses per-file provenance which ADR-0001 relies on
  for the orphan/include-graph check.
