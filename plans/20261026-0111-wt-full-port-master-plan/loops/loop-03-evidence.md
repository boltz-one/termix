---
loop: 3
candidate: A_3 (crates/renderer/src/settings.rs + tests/wt_renderer_colorfix.rs + docs/decisions/0015-colorfix-oklab-port.md + parity/ledger/P10-renderer/{files,symbols}.csv)
status: partial
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors:
  - vr1_t2_wire_fg_path_nudges_close_pair
  - vr2_t2_wire_ul_path_nudges
  - vr3_t2_skip_when_flag_off
  - vr4_t2_indexed_requires_default_or_legacy
  - vr5_t2_skip_when_fg_equals_bg
  - vr6_t5_behavioral_5_tests_green
  - vr7_t6_wt_named_integration_3_green
  - vr8_t7_adr_0015_exists
  - vr9_t8_ledger_p10_updated
  - p1_loop02_workspace_regression
  - p2_loop02_colorfix_unit_intact
  - p3_loop02_settings13_intact
  - p4_loop02_colortable256_default
  - p5_loop01_terminal_seam_intact
  - single_writer_scope_respected
unresolved_gaps:
  - L03-G01_t6_clippy_all_targets_fails_missing_non_snake_case_allow
regressions: []
schema_valid: true
---

# Loop 03 Evidence — Wire ColorFix vào RenderSettings + behavioral/integration tests + ADR-0015 + ledger P10

Runtime.check(A_3): build + execute → **ok**. `cargo check -p renderer` 0 error; `cargo test -p renderer --lib` 58/0; `cargo test -p renderer --test wt_renderer_colorfix` 3/0; `cargo test --workspace --lib` 480/0; `cargo test -p core --test wt_terminalcore` 1/0. Candidate assessable. Developer hoàn thành 9/9 task (T2 wire + T5 5 behavioral + T6 3 integration + T7 ADR + T8 ledger) — T2/T5/T6/T7/T8 từ loop-02 gaps đều closed. **1 gap mới**: T6 test file thiếu `#![allow(non_snake_case)]` (loop-01 `wt_terminalcore.rs:14` có) → `cargo clippy -p renderer --all-targets -- -D warnings` fail 4 lỗi `non_snake_case` cho 4 fn `wt_*__*`. Test logic green nhưng lint convention fail (VR7 partial — convention mirror loop-01 chưa trọn). Dev report ghi "clippy -p renderer -- -D warnings clean" **không cover test target** (default chỉ lib, không `--all-targets`) → self-verification blind spot. Status `partial`.

## Verified Behaviors

- [x] **VR1 — T2 wire fg path nudges close pair** — evidence: `crates/renderer/src/settings.rs:159-174` adjust block sau invisible check; `cargo test -p renderer --lib settings::tests::get_attribute_colors_always_distinguishable_nudges_close_pair` pass. fg=`0xFF555555`, bg=`0xFF575757`, flag `ALWAYS_DISTINGUISHABLE` → `assert_ne!(fg, 0xFF555555)` ∧ `assert_ne!(fg, bg)` ĩ idempotent re-nudge `assert_eq!(fg2, fg)`. WT `RenderSettings.cpp:215-225` port verbatim (`(always || indexed) && fg != bg && (always || (fg_default_or_legacy && bg_default_or_legacy))`). Pass.
- [x] **VR2 — T2 wire ul path nudges** — evidence: `settings.rs:225-234` ul adjust block; `cargo test -p renderer --lib settings::tests::get_attribute_underline_color_distinguishable_nudges` pass. ul=`0xFF555555` (literal), bg=`0xFF575757`, flag on → `assert_ne!(ul, 0xFF575757)` ĩ idempotent `assert_eq!(ul2, ul)`. WT `RenderSettings.cpp:280-286` port. Pass.
- [x] **VR3 — T2 skip when flag off** — evidence: `settings.rs:162-172` guard `(always || indexed)` với `render_mode` default empty → false → skip; `test_get_attribute_colors_no_adjust_when_flags_off` pass (`assert_eq!(fg, palette[15])` verbatim). Backwards-compat. Pass.
- [x] **VR4 — T2 indexed requires default-or-legacy** — evidence: `settings.rs:165-172` guard `(always || (fg_default_or_legacy && bg_default_or_legacy))`; `test_get_attribute_colors_indexed_distinguishable_only_default_or_legacy` 2 case pass: Case A fg/bg RGB literal non-legacy → guard false → `assert_eq!(fg, 0xFF555555)` (raw skip); Case B default pair → guard true → adjust reach nhưng palette[15] White vs palette[0] Black ΔE lớn → `get_perceivable_color` early-return → `assert_eq!(fg2, palette[15])` unchanged. Mirror WT `IsDefaultOrLegacy` inline expr (ADR-0015 D4). Pass.
- [x] **VR5 — T2 skip when fg == bg** — evidence: `settings.rs:170` guard `fg != bg`; `test_get_attribute_colors_skip_when_fg_equals_bg` pass (`assert_eq!(fg, bg)` ∧ `assert_eq!(fg, 0xFF555555)` unchanged). Pass.
- [x] **VR6 — T5 behavioral 5 tests green** — evidence: `cargo test -p renderer --lib settings::tests` → **18 passed / 0 failed** (13 loop-02 existing + 5 T5 loop-03). 5 new: `get_attribute_colors_no_adjust_when_flags_off`, `get_attribute_colors_always_distinguishable_nudges_close_pair`, `get_attribute_colors_indexed_distinguishable_only_default_or_legacy`, `get_attribute_colors_skip_when_fg_equals_bg`, `get_attribute_underline_color_distinguishable_nudges`. Pass.
- [x] **VR7 — T6 WT-named integration 3 green (logic)** — evidence: `cargo test -p renderer --test wt_renderer_colorfix` → **3 passed / 0 failed**: `wt_colorfix__perceivable_color_distinguishable`, `wt_renderer__always_distinguishable_nudges_close_pair`, `wt_renderer__indexed_distinguishable_requires_default_or_legacy`. Test logic + double-underscore convention `wt_<suite>__<behavior>` pass. **Caveat**: lint convention `#![allow(non_snake_case)]` chưa mirror loop-01 → gap L03-G01 (xem Unresolved Gaps). Pass-partial (logic, không full convention).
- [x] **VR8 — T7 ADR-0015 exists** — evidence: `docs/decisions/0015-colorfix-oklab-port.md` (3871 byte) tồn tại. Frontmatter: `adr: 0015`, `status: accepted`, `date: 2026-10-26`, `phase: P10`, `related: [ADR-0003]`. Body: D1 (colorfix.rs vs separate crate, YAGNI 1 consumer now), D2 (`cbrt_est` bit-hack vs `f32::cbrt` libm, max err 6.7e-4, determinism), D3 (`SRGB_TO_RGB_LUT[256]` decimal f32 vì Rust stable chưa support hex-float literal), D4 (`IsDefaultOrLegacy` inline expr trong settings.rs, không extend frozen `core::TextAttribute` P01). Consequences: `get_attribute_colors` giờ mutate fg khi flag on; `Feature_AdjustIndistinguishableText::IsEnabled()` WT runtime probe bỏ, gating qua `render_mode` flags; `oklab` mod private, pub re-export qua `lib.rs:46`; ΔEOK² check không recompute qua public API → idempotent re-nudge proxy. Pass.
- [x] **VR9 — T8 ledger P10 updated** — evidence: `parity/ledger/P10-renderer/files.csv` row `f00981` (`renderer/base/RenderSettings.cpp`) + `f01002` (`renderer/inc/RenderSettings.hpp`) → `rust_path=crates/renderer/src/settings.rs`, `status=partial`, `test_ids` populated (5 test), `adr=ADR-0015;ADR-0003`, `author=P10-worker`, notes populated; `symbols.csv` (10 line = 1 header + 9 row s001-s009) populated `id,symbol,wt_anchor,kind,rust_path,status,test_ids,adr,notes`. Pass.
- [x] **P1 — Loop 02 workspace regression** — evidence: `cargo test --workspace --lib` → 0+31+47+37+206+0+24+0+58+61+0+16 = **480 passed; 0 failed** (settings 31, connection 47, core 37, font 206, control 24, renderer 58 = 53+5 T5, xtask 61, app 16; others 0 crate chưa có lib test). Loop-02 baseline 475 → 480 (+5 T5 loop-03; +0 regression). Pass.
- [x] **P2 — Loop 02 colorfix unit intact** — evidence: `cargo test -p renderer --lib colorfix` → **8 passed / 0 failed** (T4 loop-02: `test_srgb_lut_length_256`, `test_cbrt_est_within_6e4_of_std_cbrt`, `test_linear_srgb_oklab_roundtrip`, `test_get_lightness_black_white_red`, `test_get_perceivable_color_far_colors_pass_through`, `test_get_perceivable_color_identical_colors_nudges`, `test_get_perceivable_color_preserves_alpha_byte`, `test_get_perceivable_color_darker_than_ref_darkens`). T2 không modify `colorfix.rs`. Pass.
- [x] **P3 — Loop 02 settings 13 existing intact** — evidence: `cargo test -p renderer --lib settings::tests` → 18 pass = 13 (loop-02 P3) + 5 (T5 loop-03). 13 existing verbatim: `render_settings_default_loads_campbell_table`, `render_mode_flags_default_empty_and_insert`, `get_attribute_colors_default_returns_palette_defaults`, `get_attribute_colors_reverse_video_swaps`, `get_attribute_colors_intense_brighten_picks_index_8`, `get_attribute_colors_faint_halves_fg_channels`, `get_attribute_colors_invisible_sets_fg_equal_bg`, `get_attribute_colors_with_alpha_opaque_fg`, `get_attribute_colors_with_alpha_keeps_default_bg_alpha_for_screenreversed_false`, `get_attribute_colors_with_alpha_forces_bg_opaque_when_reverse_video`, `get_attribute_underline_color_falls_back_to_fg`, `get_attribute_underline_color_uses_explicit_when_set`, `get_attribute_underline_color_invisible_returns_bg`. T2 wire guard `flag default empty → skip` → 13 test không set flag → same behavior. Pass.
- [x] **P4 — Loop 02 color table 256 default intact** — evidence: `render_settings_default_loads_campbell_table` pass verbatim (`color_table[0]==0xFF0C0C0C`, `[16]==0xFF000000`, `[255]&0xFF==238`, `default_fg_index==15`, `default_bg_index==0`). `RenderSettings::default()` impl không modify (git status chỉ `M settings.rs` không delete/recreate `default()`). Pass.
- [x] **P5 — Loop 01 terminal seam intact** — evidence: `cargo test -p core --test wt_terminalcore` → `wt_terminalcore__test_notify_scrolling ... ok`, **1 passed / 0 failed**. Loop-03 không touch `crates/core/`. Pass.
- [x] **Single-writer scope respected** — evidence: `git status --short` chỉ thay đổi `M crates/renderer/src/settings.rs`, `?? crates/renderer/tests/`, `?? docs/decisions/0015-colorfix-oklab-port.md`, `M parity/ledger/P10-renderer/files.csv`, `?? parity/ledger/P10-renderer/symbols.csv`, `?? plans/.../loop-03-dev-doc.md`. KHÔNG touch `crates/core/` (P10 isolation honored), `crates/renderer/src/colorfix.rs` (P2), `lib.rs`, `RenderSettings::default()` (P4). Pass.

## Unresolved Gaps

- [ ] **L03-G01 — T6 clippy `--all-targets -D warnings` fail: thiếu `#![allow(non_snake_case)]`** — gap: `cargo clippy -p renderer --all-targets -- -D warnings` → **4 error** `non_snake_case` cho 4 fn `wt_*__*` trong `crates/renderer/tests/wt_renderer_colorfix.rs` (`wt_colorfix__perceivable_color_distinguishable` line 19, `wt_renderer__always_distinguishable_nudges_close_pair` line 37, `wt_renderer__indexed_distinguishable_requires_default_or_legacy` line 59; 1 warning duplicate target). Loop-01 `wt_terminalcore.rs:14` có `#![allow(non_snake_case)]` (verify `cargo clippy -p core --all-targets -- -D warnings` pass clean). T6 file mirror loop-01 double-underscore naming convention `wt_<suite>__<behavior>` nhưng **không mirror allow attr** → lint convention break. Dev self-verification ghi "clippy clean" nhưng chỉ `cargo clippy -p renderer -- -D warnings` (default target = lib, không include test target `--all-targets`) → blind spot không surface gap. Test logic 3/3 green (`cargo test -p renderer --test wt_renderer_colorfix` pass) nên VR7 logic pass nhưng convention mirror không trọn. **Fix (next loop Developer)**: thêm `#![allow(non_snake_case)]` ở line 1 của `wt_renderer_colorfix.rs` (mirror `wt_terminalcore.rs:14`). Severity: low (build + test green, chỉ lint convention clean check fail; không break Runtime.check cho `cargo test`, break `cargo clippy --all-targets` của CI/lint gate).

## Regressions

Không có. Loop-02 P1-P4 + T1/T3/T4 verified behaviors hiệu lực (workspace 480/0 vs baseline 475, colorfix 8/0, settings 18/0 = 13 existing + 5 mới, color table 256, terminal seam 1/0). T2 wire có guard `flag default empty → skip` → không regress 13 existing settings test. `cargo check -p renderer` clean, `clippy -p renderer -- -D warnings` (default) clean. L03-G01 là gap mới (T6 lint convention), không regression behavior loop-02 (loop-02 T6 chưa tồn tại → không có row close để reopen).

## Runtime Check

- Build: `cargo check -p renderer` → Finished `dev` profile, 0 error.
- Execute renderer lib: `cargo test -p renderer --lib` → **58 passed; 0 failed** (colorfix 8 + settings 18 + paint/payload/cell/grid 32).
- Execute T6: `cargo test -p renderer --test wt_renderer_colorfix` → **3 passed; 0 failed**.
- Workspace: `cargo test --workspace --lib` → **480 passed; 0 failed; 0 ignored** (loop-02 baseline 475 + 5 T5).
- Loop-01 seam: `cargo test -p core --test wt_terminalcore` → **1 passed; 0 failed**.
- Lint (default target): `cargo clippy -p renderer -- -D warnings` → clean (Dev report).
- Lint (`--all-targets`): `cargo clippy -p renderer --all-targets -- -D warnings` → **FAIL 4 error** `non_snake_case` (L03-G01).
- `schema_valid: true` — build + execute pass, candidate assessable. Gap là lint convention omission (T6 allow attr), không build/run failure.

## Assessment

**Accept: NO (partial) — 1 unresolved gap L03-G01.**

Developer hoàn thành 9/9 task vs D_3: T2 (wire ColorFix vào `get_attribute_colors` + `get_attribute_underline_color` khi flag on, port WT `RenderSettings.cpp:215-225` + `:280-286` verbatim, guard `flag default empty → skip` backwards-compat), T5 (5 behavioral test green, 13 existing + 5 = 18), T6 (3 WT-named integration green), T7 (ADR-0015 D1-D4 + Consequences accepted), T8 (ledger P10 f00981+f01002 partial + symbols.csv 9 row). VR1-VR6 + VR8 + VR9 + P1-P5 + single-writer scope đều verified. Loop-02 5 gap (L02-G01..L02-G05) **đều closed** (issue-ledger update).

**1 gap mới L03-G01**: T6 `wt_renderer_colorfix.rs` thiếu `#![allow(non_snake_case)]` (loop-01 `wt_terminalcore.rs:14` có) → `cargo clippy -p renderer --all-targets -- -D warnings` fail 4 error `non_snake_case`. Test logic 3/3 green, build green, nhưng lint convention mirror chưa trọn. Dev self-verification blind spot: report ghi "clippy clean" nhưng chạy `cargo clippy -p renderer -- -D warnings` (default target = lib, không `--all-targets`) → không surface test-target lint. Gap low severity (không block `cargo test`/`cargo check`, chỉ block `cargo clippy --all-targets` của CI lint gate), nhưng VR7 (convention mirror loop-01) chỉ partial-pass.

**Fix cho next loop**: 1-line edit `crates/renderer/tests/wt_renderer_colorfix.rs` line 1 thêm `#![allow(non_snake_case)]` (mirror loop-01). Out-of-scope QA (read-only artifact). Loop-04 Developer tack-on hoặc Runtime gate commit.

Master plan NOT done — loop 03/19 partial, 0 → 9/54 P10 symbols ported (colorfix + settings wire), 0 P10 step 2 (paint driver) onwards. Runtime owns subsequent loops.

**Notes (pre-existing, out-of-scope QA loop 03)**:
- `apps/terminal/{Cargo.toml,src/main.rs}` git status sạch loop này (loop-02 QA đã restore từ commit `492a06a`). Workspace build ok. Out-of-scope.
- `cargo xtask audit --final` chưa re-run (loop-01 note: `merge symbols` duplicate-id preexisting schema bug, phase-agnostic, out-of-scope).
