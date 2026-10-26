# ADR-0001 — Port scope: in-scope vs out-of-scope in `windows/src`

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** main agent (P01), per master plan §6 ADR registry
- **Phase owner:** P01

## Context

The master plan ports Windows Terminal (WT) `windows/src` to Rust + GPUI. Not all of WT is
relevant to a GPUI terminal emulator: conhost (the legacy console host), server/host IPC,
gdi/wddmcon rendering, audio, tools, WPF control, certain Hwnd/Public shells, and the Azure
Connection are out of scope for this port. A first-pass categorisation (per `01-inventory-baseline.md`
and `02-parity-ledger-and-audit-loop.md`) needs to be locked so the `exclusions.csv` ledger
(P01, ADR-0001) and the xtask orphan check have a single source of truth for what is and
isn't counted toward the "100% port" goal.

## Decision

### OUT of scope (exclusions ledgered under ADR-0001)

- conhost `host/`, `server/`.
- `renderer/gdi|wddmcon` (gdi and wddmcon rendering paths).
- `audio/`.
- `tools/`.
- WPF control.
- `TerminalControl/HwndTerminal*` (WPF-facing host shim).
- `PublicTerminalCore` (public COM surface excluded).
- `TerminalConnection/AzureConnection*` — see ADR-0007.
- `interactivity/` only if NOT referenced by in-scope code (assumption confirmed during P01).
- Non-logic files `.vcxproj/.props/.sln/.def/.rc/.inc/.cs`, images — not ledgered per file
  (row-count per file would be noise; counted as aggregate in `exclusions.csv`).
- `.resw`/`defaults.json` consumed by P14/P07 respectively, not ledgered as source.

### IN scope

- Rest of `windows/src`: parser, adapter, buffer, terminal, terminal_input, types/til,
  renderer (atlas/portable logic only — see ADR-0003), settings,
  TerminalConnection excluding Azure, TerminalControl/UI elements that map to GPUI
  (XAML → GPUI — see ADR-0005).

## Consequences

- `xtask ledger-seed` writes one `exclusions.csv` row per out-of-scope source file with `adr=ADR-0001`
  (and `adr=ADR-0007` for Azure files); the orphan check (`include-graph`) flags any referenced
  OUT file that lacks an IN promotion row or a matching ADR row.
- Total phase ledger (in-scope + out-of-scope) closes against this scope. Adding new OUT
  files or pulling an OUT file IN requires an ADR-approved deviation row in `decisions.csv`.
- `.resw`/`defaults.json` are consumed late (P14/P07) and follow those phases' model rules,
  not the per-file files.csv schema.

## Alternatives considered

-/port every `windows/src` file verbatim including conhost — rejected: doubles the surface,
  none of it serves the GPUI terminal emulator.
- exclude conhost + Azure by silent omission — rejected: orphan check would fire on every
  OUT file with an inbound include; needs an explicit ADR for each exclusion class.
