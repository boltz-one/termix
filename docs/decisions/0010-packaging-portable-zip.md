# ADR-0010 — Packaging: portable zip primary, MSIX deferred

- **Status:** accepted
- **Date:** 2026-10-26
- **Owner phase:** P18

## Context

User has confirmed portable-zip packaging is the primary target; MSIX deferred. The
`packaging/` path (P18) defaults to the portable zip layout that bundles the GPUI
binary, dependencies, and assets into a relocatable directory.

## Decision

Port is the primary, required packaging form (P18). MSIX/signing/winget distribution
is deferred (not blocking parity closure). ADR-0010 deviation row supersedes any WT
packaging source referenced by `include-graph`.

See owner phase (P18) for the packaging layout details.
