# ADR-0004 — `til::*` types replacement

- **Status:** proposed
- **Date:** 2026-10-26
- **Owner phase:** P05

## Context

Stub. WT's `types/til` (Terminal Implementation Library) provides a grab-bag of utility
types (`til::point`, `size`, `rect`, `color`, `enum`, `bitmap`, bit-fields). The GPUI port
maps these to idiomatic Rust (`gpui::Point`/`Size`/`Rect`/`Hsla`, etc.) inside `types`
per ADR-0004 default. ADR-0004 records the replacement map for types with behaviour vs.
pure-trivial.

## Decision

See owner phase for finalization.
