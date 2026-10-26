# 01 — Inventory & Baseline (measured 2026-10-26)

Method: `find windows/src/<dir> -name '*.cpp|*.h|*.hpp|*.idl|*.xaml'` excluding `ut_*`, `ft_*`, `*UnitTests*`, `test*` (non-test LOC); test LOC counted separately; `TEST_METHOD` = `grep -E '^\s*(BEGIN_)?TEST_METHOD\('` (undercounts data-driven cases; `xtask audit` recounts per-row in P01). Rust LOC = `find crates/*/src -name '*.rs' | xargs cat | wc -l`.

## A. WT source inventory → Rust target

| WT dir (`windows/src/`) | C++ LOC | WT tests (LOC / TEST_METHOD) | Rust target | Rust LOC now | Phase | Scope |
|---|---|---|---|---|---|---|
| terminal/adapter | 11,300 | 6,058 / 74+ | core `dispatch/**` | ~6.4k (188 `todo!`) | P03 | IN |
| terminal/parser | 5,761 | 8,313 / 98 | core `parser/**` | 695 | P04 | IN |
| terminal/input | 2,429 | in adapter tests | core `terminal_input/**` | 0 | P04 | IN |
| buffer/out | 9,895 | 1,375 / 14 | core `buffer/**` | 1,111 | P08 | IN |
| cascadia/TerminalCore | 4,277 | 2,708 / 54 | core `terminal/**` | 0 | P09 | IN |
| types | 9,068 | 2,042 / 17 | types | ~0 (screen.rs 170) | P05 | IN (UIA parts → P15) |
| inc/til + til | 8,646 | 5,466 / 198 | types (only used behaviour; rest `replaced`) | 0 | P05 | IN, mostly `replaced` |
| winconpty | 781 | 357 / 7 | connection `conpty` | 470 | P06 | IN |
| cascadia/TerminalConnection | 2,957 | — | connection | 1,693 | P06 | IN (Azure OUT, ADR-0007) |
| renderer/base | 3,310 | — | renderer | 1,997 (total crate) | P10 | IN |
| renderer/atlas | 10,269 | — | renderer (GPUI text system, ADR-0003) | incl. above | P10 | IN, behaviour-parity |
| renderer/inc | 741 | — | renderer | incl. | P10 | IN |
| renderer/uia | 570 | — | a11y | 0 | P15 | IN |
| renderer/gdi, wddmcon | 2,478 + 586 | — | — | — | — | OUT (ADR-0001) |
| cascadia/TerminalControl | 15,279 | 2,022 / 28 | control | 1,159 | P11 | IN |
| cascadia/TerminalApp | 31,957 | 5,559 / 59 | app | 1,175 | P12/P13/P14 | IN |
| cascadia/TerminalSettingsModel | 21,031 | 10,250 / 158 | settings | 3,134 | P07 | IN |
| cascadia/TerminalSettingsAppAdapterLib | 679 | — | settings | 0 | P07 | IN |
| cascadia/TerminalSettingsEditor | 28,695 | — | settings::editor | 0 | P17 | IN |
| cascadia/WindowsTerminal | 7,551 | 1,256 / 52 (ut_app) | host | 0 | P16 | IN |
| cascadia/Remoting | 0 (`cascadia/Remoting/` holds only `Resources/*.resw`; sources are `TerminalApp/Remoting.{cpp,h,idl}`, multi-window logic in `WindowsTerminal/WindowEmperor.cpp`) | — | app `remoting` + host | 0 | P13/P16 | IN (counted under TerminalApp/WindowsTerminal) |
| cascadia/UIHelpers, UIMarkdown, fzf | 934 + 935 + 459 | — | app `ui/**` | 0 | P14 | IN |
| cascadia/WinRTUtils | 895 | — | app/extras (only if referenced) | 0 | P18 | IN where referenced |
| cascadia/ShellExtension, ElevateShim, wt, CascadiaPackage | 363 + 106 + 52 | — | extras / packaging | 0 | P18 | IN |
| tsf | 1,150 | — | a11y (IME) | 0 | P15 | IN |
| propslib | 1,919 | — | types / settings (registry/ shell helpers) | 0 | P05/P07 | IN where referenced |
| font (WT font helpers, incl. css_length) | — | — | font (css_length 234) + font 1,013 | done | P07/P10 | IN, audit |
| host (conhost) | 26,354 | 28,376 / 453 | — | — | — | OUT (ADR-0001) |
| interactivity | 11,856 | 6,091 / 19 | — | — | — | OUT unless referenced by IN code (include-graph audit, P19) |
| server, audio, internal, tools, WpfTerminalControl, PublicTerminalCore | 6,484 + 204 + 54 + 7,314 + 1,270 | — | — | — | — | OUT (ADR-0001) |

**IN-scope non-test total ≈ 183k C++ LOC** (cascadia 117k + terminal 19.5k + buffer 9.9k + renderer 15k + types/til/tsf/winconpty/propslib 22k). Expected Rust at ~0.6× ≈ 110k LOC. Current Rust 20.4k LOC (part is scaffolding). Honest completion vs full parity ≈ 8–12% (see session estimate).

## B. Rust baseline (cargo, 2026-10-26)

- `cargo check --workspace` PASS (17-18 warnings in core, PascalCase fns). `cargo test --workspace` PASS, ~410 tests, 0 fail.
- `todo!()`/`unimplemented!()` code sites (comments excluded, measured 2026-10-26): core 171 (trait stubs `dispatch/mod.rs` 123, `interact.rs` 15, `interact_impl.rs` 8, `td/modes.rs` 24, rest), app 1 (`page.rs:143`), control 0, connection 0 (doc comment only). Raw grep incl. comments = 188/7/3/1. Canonical number comes from `cargo xtask stubs` (P01).
- `apps/terminal`, `apps/docs`: empty. No `tests/` dirs; no WT tests ported (0 / 759 counted in-scope TEST_METHOD (sum of the table; data-driven cases add more)).
- `terminal_gpui` is untracked in git; `windows/` = upstream read-only.

## C. Unverified (resolve in P01)
- Per-file ledger seeds (P01). Remoting dir is empty here; no Monarch/Peasant sources — multi-window = `WindowEmperor`.
- Whether any IN code references `interactivity/*`/`host/*` symbols (include-graph).
- TEST_METHOD counts for data-driven suites; `ut_adapter` counts via row enumeration.
