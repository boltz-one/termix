# ADR-0007 — Azure Connection out of scope

- **Status:** accepted
- **Date:** 2026-10-26
- **Owner phase:** P06

## Context

User has confirmed Azure Connection is out of scope for this port. The
`TerminalConnection/AzureConnection*` source files (C++ WinRT implementation +
adjacent) are excluded from the parity ledger under ADR-0001's exclusion clause and
the dedicated ADR-0007 row.

## Decision

`TerminalConnection/AzureConnection*` is OUT of scope. Files are ledgered in
`exclusions.csv` with `adr=ADR-0007` (counted as out-of-scope) and never promoted IN.
P06's `crates/connection` port omits the Azure variant entirely; the
`ConptyConnection` and other in-scope connection variants are the ported surface.

See owner phase (P06) for ledger-seed seeding of the Azure rows.
