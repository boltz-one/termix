# ADR-0002 — VT parser: keep `vte` or port WT `StateMachine` verbatim

- **Status:** proposed
- **Date:** 2026-10-26
- **Owner phase:** P04

## Context

Stub. P04 spikes both paths: keep Rust `vte` crate as parser engine vs. port WT's
`StateMachine` (OutputStateMachineEngine) verbatim into `core::parser`. ADR-0002's
default per master plan: keep `vte` only if all 98 `ut_parser` tests + golden VT byte
streams pass under it; otherwise port `StateMachine` verbatim.

## Decision

See owner phase for finalization.
