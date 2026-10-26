# ADR-0006 — WinRT/COM remoting → Rust IPC

- **Status:** proposed
- **Date:** 2026-10-26
- **Owner phase:** P13

## Context

Stub. WT's `Remoting` and `Cmdline` paths use WinRT/COM for inter-process marshalling.
The Rust port re-expresses these as a Rust IPC boundary (channels/Unix-streams/
named-pipe variant). ADR-0006 records the chosen transport and marshalling shape.

## Decision

See owner phase for finalization.
