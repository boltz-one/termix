# ADR-0011 — Settings model: plain Rust, no XAML data binding

- **Status:** proposed
- **Date:** 2026-10-26
- **Owner phase:** P07

## Context

Stub. WT's settings model (`CascadiaSettings`, `Profile`, `Appearance`, `FontFace`,
`ColorScheme`, etc.) is C++/WinRT with JSON deserialisation. The Rust port is plain
Rust structs + serde (`settings`); no XAML data binding.

## Decision

See owner phase for finalization.
