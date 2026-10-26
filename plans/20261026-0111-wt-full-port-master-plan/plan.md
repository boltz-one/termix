---
title: "Windows Terminal → termix full port (100% logic parity)"
description: "Master plan: port all in-scope WT C++/WinRT logic (~183k LOC) to Rust+GPUI with ledger-driven audit loop until 100% parity."
status: pending
priority: P1
effort: 3440h
lane: high-risk
branch: main
tags: [port, windows-terminal, gpui, parity, audit-loop, parallel]
created: 2026-10-26
---

# WT → termix: master plan
SSOT. Baseline `reports/01-inventory-baseline.md`; ledger/tests/audit/ADRs `reports/02-parity-ledger-and-audit-loop.md`. `windows/` = read-only upstream. Scope (ADR-0001, confirmed): conhost `host/`, `server/`, GDI/WDDM, `interactivity/`(unless referenced), audio, tools, WPF/PublicTerminalCore/HwndTerminal and AzureConnection (ADR-0007) OUT; rest IN. Platform: Windows impls sit behind platform-neutral traits (multi-platform option kept). Effort = sum of phases (nominal agent-hours ±40%, not wall-clock; P01 = 100h; P06 60h, P15 170h, P16 200h after Azure removal + platform traits).

## Milestones
| M | Goal | Phases | Exit criteria |
|---|---|---|---|
| M1 | Shell end-to-end | 01-02 | `cargo run -p terminal` shows pwsh prompt, echo, resize, exit; smoke test green |
| M2 | VT complete (vim/tmux) | 03-05, 08-09 | 0 `todo!` in core; adapter/parser/textbuffer/TerminalCore tests ported+passing; vttest goldens |
| M3 | Tabs/panes/selection/clipboard | 06, 10-12 | split/zoom/close/tabs, selection/copy/paste/search; UnitTests_Control + pane/tab LocalTests ported |
| M4 | Settings/actions/keybindings | 07, 13 | JSON layering+reload; all `ShortcutAction`s; `wt` cmdline; UnitTests_SettingsModel + CommandlineTest ported |
| M5 | UI/IME/a11y/editor/host | 14-18 | palette, editor, tray/quake, UIA, IME; ut_app ported |
| Done | 100% | 19 | report 02 §4 holds on 2 consecutive `audit --final` runs |

## Parallel Execution Matrix (unit = phase; waves via Kahn; W1 batches {02-05},{06,07})
| Unit | Owns (plus `parity/ledger/pNN-*/**`, own ADR files, own crate (for shared core/app: own `tests/wt_<suite>*` globs only) incl. its `Cargo.toml` and `tests/**`) | Depends On | Wave | Verify |
|---|---|---|---|---|
| [01 foundations](phase-01-foundations-ledger-xtask.md) | `.cargo/**`, root `Cargo.toml`, `crates/xtask/**`, `docs/decisions/*` stubs, `NOTICE`, `dispatch/seams.rs` (+`HeadlessHost`), `terminal_input/events.rs`, `settings/src/terminal_settings.rs` (create), `parity/{exclusions,phases,e2e}.csv`, `parity/{baseline-ref.txt,summary.json}`, `renderer/src/paint.rs` (IRenderData move only), `dispatch/headless_host.rs` (create; P03 owns after wave 0), `dispatch/{mod.rs,interact_impl.rs}`, `dispatch/{interact.rs,td/modes.rs}` no-op defaults, `core/src/lib.rs` (temporary `allow(non_snake_case)`), `core|app` `Cargo.toml` pre-declared deps, round-0 `reports/audit-001.md`, skeleton crates, file moves, Phase 4.0 buffer edits | — | 0 | `cargo xtask ledger-check` |
| [02 m1-skeleton](phase-02-m1-walking-skeleton.md) | `apps/terminal/**`, `app/src/{app,downstream}.rs`, `app/src/page/mod.rs`, `app/Cargo.toml` (stub-downstream removal only), `control/src/{control,core}.rs`, `control/tests/e2e_shell_smoke.rs` | 01 | 1 | smoke e2e |
| [03 vt-adapter](phase-03-vt-adapter-dispatch.md) | `core/src/dispatch/**` (not seams.rs), removal of the `allow(non_snake_case)` line in `core/src/lib.rs`, `parity/golden/vt-*` | 01 | 1 | `cargo test -p core -- wt_adapter__` |
| [04 parser-input](phase-04-vt-parser-and-input.md) | `core/src/{parser,terminal_input}/**`, `parity/golden/parser-*` | 01 | 1 | `cargo test -p core -- wt_parser__ wt_input__` |
| [05 types-til](phase-05-types-til.md) | `crates/types/**` | 01 | 1 | `cargo test -p types` |
| [06 connection](phase-06-connection.md) | `crates/connection/**` | 01 | 1 | `cargo test -p connection` |
| [07 settings](phase-07-settings-model.md) | `crates/settings/**`, `crates/font/src/css_length.rs` | 01 | 1 | `cargo test -p settings` |
| [08 buffer](phase-08-buffer-parity.md) | `core/src/{buffer/**,attrs,cursor,screen,color_table,sgr_stack}.rs`, `dispatch/buffer_ext.rs` (delete) + its `mod` line in `dispatch/mod.rs` | 03,05 | 2 | `cargo test -p core -- wt_textbuffer__` |
| [09 terminal-core](phase-09-terminal-core.md) | `core/src/terminal/**`, `parity/golden/terminal-*` | 03,04,08 | 3 | `cargo test -p core -- wt_terminalcore__` |
| [10 renderer](phase-10-renderer.md) | `crates/renderer/**`, `crates/font/**`, `parity/golden/render-*` | 05,07,08 | 3 | `cargo test -p renderer -p font` |
| [11 control](phase-11-terminal-control.md) | `crates/control/**` | 02,06,07,09,10 | 4 | `cargo test -p control` |
| [12 app-core](phase-12-app-core-pane-tab-page.md) | `app/src/{pane,tab,page}/**` (absorbs P02 `page/mod.rs`) | 11 | 5 | `cargo test -p app -- wt_app__` |
| [15 a11y-ime](phase-15-a11y-ime.md) | `crates/a11y/**` | 10,11 | 5 | `cargo test -p a11y` |
| [13 actions](phase-13-actions-cmdline-remoting.md) | `app/src/{actions,cmdline,remoting}/**`, `app/tests/e2e_forward.rs` | 07,12 | 6 | `cargo test -p app -- wt_cmdline__ wt_actions__` |
| [14 ui-chrome](phase-14-ui-chrome.md) | `app/src/ui/**`, `app/tests/e2e_palette.rs`, `parity/golden/ui-*` | 12 | 6 | `cargo test -p app -- wt_ui__` |
| [16 host](phase-16-host-window.md) | `crates/host/**`, `apps/terminal/**`, `app/src/app.rs` | 12,13,14,15 | 7 | `cargo test -p host` |
| [17 editor](phase-17-settings-editor.md) | `crates/settings/src/editor/**`, `parity/golden/editor-*` | 07,14 | 7 | `cargo test -p settings editor` |
| [18 extras](phase-18-extras-packaging.md) | `crates/extras/**`, `packaging/**` | 13,16 | 8 | `cargo test -p extras` |
| [19 audit-closure](phase-19-parity-audit-closure.md) | `crates/xtask/**` (summary/fuzz only), `parity/merged/**`, `parity/golden/**` additions, `fuzz/**`, `reports/audit-*.md`, plan Audit Log | all | 9 | `cargo xtask audit --final` |

Rules: later phases add files only inside owned paths; `Cargo.toml`: a phase owns the manifest of crates it exclusively owns; shared crates (core, app) are pre-declared by P01 and edited append-only by the phase that needs the change, serially at its own wave barrier (barrier runner = main agent); root `Cargo.toml` append-only under `[workspace.dependencies]`. Barrier per wave: `cargo xtask audit --wave N` (regression-only: fmt, clippy, tests, no status regressions); strict `--final` only in P19. Ledger writes: each phase writes only its shard; `verified` set solely by audit reviewers at barriers. Risks per phase files (GPUI vs Atlas, vte drift, XAML re-expression, IME/UIA, Azure, GPUI churn).

## Audit Log
Round 0 baseline (P01 will write `reports/audit-001.md`); no audits run yet. Append-only by barrier runner / P19.

## Automatic Validation
**Rounds run:** 5 (deep r1 + 4 delta) | **Outcome:** ESCALATED (cap 5; user chose manual fix pass, residual 1 BLOCKER + 6 MINOR from r5 fixed manually, not re-reviewed)

| Round | Type | Structural | Consistency | Fidelity | Testability | BLOCKERs | Clean |
|---|---|---|---|---|---|---|---|
| 1 | deep | 1/5m | 11/12m | PASS | 3/5m | 15 | no |
| 2 | delta | 1/5m | 5 major/7m | carried | PASS | 6 | no |
| 3 | delta | PASS | 1/13m | carried | carried | 1 | no |
| 4 | delta | PASS | 2/8m | carried | carried | 2 | no |
| 5 | delta | PASS | 1/6m | carried | carried | 1 | no |

Full artifacts: `reports/validation-round-*.md`.

## Validation Summary
**Validated:** 2026-10-26 | **Questions asked:** 4 (replaces the former Open Questions)
- Scope ADR-0001: confirmed as written (conhost etc. OUT; `interactivity/` only if referenced).
- Azure Cloud Shell: **OUT of scope** → ADR-0007 becomes `out-of-scope`.
- Platform: **keep future multi-platform option** (not Windows-only).
- Packaging: **portable zip first**, MSIX later (ADR-0010).
### Action Items (all applied; not re-reviewed by a reviewer round)
- [x] P06: Azure removed; `PtyBackend` trait extracted; ADR-0007 out-of-scope (done 2026-10-26).
- [x] P06/P15/P16: platform traits added (`PtyBackend`, `AccessibilityProvider`/`ImeBackend`, `HostWindow`/`TrayIcon`/`GlobalHotkey`); effort +40h → 3440h.
- [x] P18: portable zip primary, MSIX deferred; ADR-0010 final.
- [x] Reports 01/02 + plan.md scope sentence: Azure OUT.

## Validation Summary (round 2)
**Validated:** 2026-10-26 | **Questions asked:** 4
- Coverage: pin nightly date for `cargo xtask cov`; per-crate branch-cov shortfalls need an ADR waiver accepted by audit.
- MSIX: **out of scope** (goal.md already says so) → P18/ADR-0010 must say out-of-scope, not "deferred".
- GPUI: pin exact rev in root `Cargo.toml`; bump only at wave barriers.
- Reviewers: fresh subagent per barrier + xtask mechanical checks (reject self-verified rows).
### Action Items (phase files NOT modified)
- [ ] P01: add nightly date pin + waiver ADR template + GPUI rev pin; xtask check rejecting `verified` rows authored by the owning phase.
- [ ] P18 + ADR-0010 + plan.md Validation Summary (round 1): "MSIX later/deferred" → out-of-scope.
- [ ] Barrier runner doc (report 02): define reviewer subagent brief (ledger+code only, no author report).
