---
loop: 2
candidate: A_2 (crates/renderer/src/{colorfix.rs,settings.rs,lib.rs} + tests/wt_renderer_colorfix.rs + ADR-0015 + ledger/P10-renderer)
status: partial
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
verified_behaviors:
  - vr1_colorfix_perceivable_color_distinguishable
  - vr2_far_color_passthrough
  - p1_workspace_regression
  - p2_loop01_terminal_seam_intact
  - p3_settings_existing_13_tests
  - p4_color_table_256_default
  - t1_colorfix_port_unit
  - t3_colorfix_module_exposed
  - t4_colorfix_inline_tests
unresolved_gaps:
  - L02-G01_t2_wire_get_perceivable_color_into_settings
  - L02-G02_t5_settings_behavioral_tests
  - L02-G03_t6_wt_named_integration_test
  - L02-G04_t7_adr_0015
  - L02-G05_t8_ledger_p10_update
regressions: []
schema_valid: true
---

# Loop 02 Evidence — ColorFix port + AdjustIndistinguishable wire

Runtime.check(A_2): build + execute → **ok**. `cargo check -p renderer` clean; `cargo test -p renderer --lib` → 53 passed / 0 failed; `cargo test --workspace --lib` → 475 passed / 0 failed; `cargo test -p core --test wt_terminalcore` → 1 passed / 0 failed. Build pass → candidate assessable. Developer 只完成 3/8 task (T1/T3/T4) — wire (T2) + behavioral tests (T5) + integration test (T6) + ADR-0015 (T7) + ledger P10 update (T8) chưa thực hiện → status `partial`.

## Verified Behaviors

- [x] **T1 — ColorFix port unit** — evidence: `crates/renderer/src/colorfix.rs` (322 line) port verbatim `windows/src/types/ColorFix.cpp`. `SRGB_TO_RGB_LUT[256]` (line 30-47, decimal f32 full-precision round-trip, không hex-float vì Rust stable chưa support); `saturate` (49-53); `cbrt_est` (58-67, FP32 bit-hack `u/3 + 709921077` + 1 round Newton); `oklab::linear_srgb_to_oklab` (88-103, coeff verbatim Ottosson); `oklab::oklab_to_linear_srgb` (107-121); `colorref_to_linear` (127-135, ABGR low-byte = R giống COLORREF); `linear_to_colorref` (141-152, `round_ties_even` = `lrintf` half-even); `get_perceivable_color` (168-199, ΔEOK² early-return + deltaL sign theo `color.l < ref.l` + clamp flip + preserve alpha `| (color & 0xff000000)`); `get_lightness` (210-212); `adjust_lightness` (203-207). Module doc (1-22) credit Björn Ottosson, ghi Rust stable hex-float note + `clippy::excessive_precision` allow. Pass — port-of-intent verbatim.
- [x] **T3 — colorfix module exposed** — evidence: `crates/renderer/src/lib.rs:33` `pub mod colorfix;` + line 46 `pub use colorfix::{adjust_lightness, get_lightness, get_perceivable_color};`. Pass.
- [x] **T4 — colorfix inline tests** — evidence: `cargo test -p renderer --lib colorfix` → 8 passed / 0 failed: `test_srgb_lut_length_256`, `test_cbrt_est_within_6e4_of_std_cbrt` (err < 6.7e-4 vs `f32::cbrt`), `test_linear_srgb_oklab_roundtrip` (5 colors, ±7 channel tolerance do `cbrt_est` estimator), `test_get_lightness_black_white_red` (black ~0, white ~1, red L ≈ 0.6279), `test_get_perceivable_color_far_colors_pass_through`, `test_get_perceivable_color_identical_colors_nudges` (alpha 0xFF preserved), `test_get_perceivable_color_preserves_alpha_byte` (alpha 0x80 preserved), `test_get_perceivable_color_darker_than_ref_darkens` (l_out < l_ref). Pass — VR1/VR2 covered white-box.
- [x] **VR1 — ColorFix distinguishable path** — evidence: `colorfix::tests::test_get_perceivable_color_identical_colors_nudges` (`color = ref = 0xFF555555`, min_sq = 0.25) → `assert_ne!(out, reference)` + `assert_eq!(out & 0xff000000, 0xff000000)`. VR1 reframe white-box: identical → nudge → output ≠ input, alpha preserved. Pass.
- [x] **VR2 — Far-color passthrough** — evidence: `colorfix::tests::test_get_perceivable_color_far_colors_pass_through` (`color = 0xFF0000FF` red, `ref = 0xFF00FF00` green, min_sq = 0.25) → `assert_eq!(out, color)`. Pass — early-return path verified.
- [x] **P1 — Workspace regression** — evidence: `cargo test --workspace --lib` → 31+47+37+206+24+53+61+16 = **475 passed; 0 failed; 0 ignored** (settings 31, connection 47, core 37, font 206—recheck, renderer 53, control 24, xtask 61, others 0). Loop01 baseline 462 → 475 (+13 từ phase khác: connection/font growth + renderer +8 colorfix). Loop02 standalone contribution renderer 53-45 = +8 colorfix. 0 regression — mọi crate green. Pass.
- [x] **P2 — Loop01 terminal seam intact** — evidence: `cargo test -p core --test wt_terminalcore` → `wt_terminalcore__test_notify_scrolling ... ok`, 1 passed / 0 failed. Loop02 không touch `crates/core/`. Pass.
- [x] **P3 — Settings existing 13 test** — evidence: `cargo test -p renderer --lib settings::tests` → 13 test pass verbatim (render_settings_default_loads_campbell_table, render_mode_flags_default_empty_and_insert, get_attribute_colors_default_returns_palette_defaults, get_attribute_colors_reverse_video_swaps, get_attribute_colors_intense_brighten_picks_index_8, get_attribute_colors_faint_halves_fg_channels, get_attribute_colors_invisible_sets_fg_equal_bg, get_attribute_colors_with_alpha_opaque_fg, get_attribute_colors_with_alpha_keeps_default_bg_alpha_for_screenreversed_false, get_attribute_colors_with_alpha_forces_bg_opaque_when_reverse_video, get_attribute_underline_color_falls_back_to_fg, get_attribute_underline_color_uses_explicit_when_set, get_attribute_underline_color_invisible_returns_bg). T2 wire KHÔNG áp → flag default empty → adjust path skip → same behavior. Pass.
- [x] **P4 — Color table 256 default** — evidence: `render_settings_default_loads_campbell_table` assert `color_table[0] == 0xFF0C0C0C` (Black), `color_table[16] == 0xFF000000`, `color_table[255] & 0xFF == 238`, `default_fg_index == 15`, `default_bg_index == 0`. `default()` impl không modify. Pass.

## Unresolved Gaps

- [ ] **L02-G01 — T2 wire `get_perceivable_color` vào settings.rs** — gap: `grep "get_perceivable_color\|colorfix" crates/renderer/src/settings.rs` → No matches. `get_attribute_colors` (line 132-160) trả `(fg, bg)` thuần lookup_color + dim/swap/invisible, không có AdjustIndistinguishable block. `get_attribute_underline_color` (line 188-206) tương tự. Doc-comment line 128-131 vẫn ghi "Không có `ColorFix::GetPerceivableColor` step". VR3 (Always distinguishable nudges close pair), VR4 (Indexed requires DefaultOrLegacy), VR6 (Underline nudge) **not assessable** — behavior chưa implement, không có observable output để verify. WT ground `RenderSettings.cpp:215-225` + `:280-286` chưa port.
- [ ] **L02-G02 — T5 settings behavioral tests** — gap: `crates/renderer/src/settings.rs` `#[cfg(test)] mod tests` chỉ có 13 test cũ (P3 list), không append 5 T5 test: `test_get_attribute_colors_no_adjust_when_flags_off`, `test_get_attribute_colors_always_distinguishable_nudges_close_pair`, `test_get_attribute_colors_indexed_distinguishable_only_default_or_legacy`, `test_get_attribute_colors_skip_when_fg_equals_bg`, `test_get_attribute_underline_color_distinguishable_nudges`. VR1-VR6 white-box dưới settings boundary not assessable (T2 chưa wire thì T5 không thể verify được hành vi mới — T2/T5 cùng fail).
- [ ] **L02-G03 — T6 WT-named integration test** — gap: `cargo test -p renderer --test wt_renderer_colorfix` → error `no test target named 'wt_renderer_colorfix' in 'renderer' package` (dir `crates/renderer/tests/` không tồn tại). VR7 partial-fail (workspace green nhưng thiếu test file riêng). WT-named double-underscore convention gap (mirror loop-01 `wt_terminalcore__`) chưa test trong renderer.
- [ ] **L02-G04 — T7 ADR-0015** — gap: `docs/decisions/0015-colorfix-oklab-port.md` không tồn tại (list 14 ADR hiện có 0001-0014). D1-D4 + Consequences chưa record (crate placement `renderer/src/colorfix.rs` vs separate crate, `cbrt_est` vs `f32::cbrt`, LUT static vs runtime, `IsDefaultOrLegacy` inline expr). ADR-gap không block Runtime.check nhưng break P10 doc-of-record trail.
- [ ] **L02-G05 — T8 ledger P10 update** — gap: `parity/ledger/P10-renderer/files.csv` (seed, 52 row) chưa update row `f00981` (renderer/base/RenderSettings.cpp) + `f01002` (renderer/inc/RenderSettings.hpp) → status vẫn `not-started` (rust_path trống). `symbols.csv` không tồn tại (P10 dir chỉ có files.csv). Test_ids, ADR column trống. Parity ledger không reflect loop02 port progress.

## Regressions

Không có. 475 workspace test pass / 0 fail (loop01 baseline 462 → +13 phase khác, không regress). Loop01 terminal seam 1/0 intact. Renderer 13 existing settings test verbatim pass. `cargo check -p renderer` clean. T2 chưa áp nên không break existing 13 test (flag default empty adjust path skip).

## Runtime Check

- Build: `cargo check -p renderer` → Finished `dev` profile, 0 error.
- Execute: `cargo test -p renderer --lib` → 53 passed; 0 failed (colorfix 8 + settings 13 + paint/payload/cell/grid 32).
- Workspace: `cargo test --workspace --lib` → 475 passed; 0 failed; 0 ignored.
- Loop01 seam: `cargo test -p core --test wt_terminalcore` → 1 passed; 0 failed.
- T6: `cargo test -p renderer --test wt_renderer_colorfix` → error no test target.
- `schema_valid: true` — build+execute pass, candidate assessable. Gaps là task omission (T2/T5/T6/T7/T8 chưa thực hiện), không phải build/run failure.

## Assessment

**Accept: NO (partial).**

Developer hoàn thành 3/8 task: T1 (ColorFix port — oklab + sRGB LUT + cbrt_est + GetPerceivableColor + GetLightness + AdjustLightness, port verbatim), T3 (colorfix module expose trong lib.rs), T4 (8 inline white-box test green cover VR1/VR2). 5/8 task omitted: T2 (wire ColorFix vào RenderSettings adjust path — core objective của loop, ghi rõ trong headline Objective), T5 (5 behavioral settings test), T6 (3 WT-named integration test), T7 (ADR-0015), T8 (ledger P10 update). VR3/VR4/VR6 (behavior end-to-end qua `RenderSettings::get_attribute_colors` adjust path) **not assessable** — behavior chưa implement, không có observable output. VR7 partial (build+renderer+workspace green nhưng thiếu T6 test file). VR8 pass (13 existing verbatim, T5 không add nên không có regression vector).

Loop objective (`wire ColorFix vào RenderSettings → distinguishable colors khi flag on`) **chưa đạt** — T2 là single most load-bearing task (headline Objective), không wire = increment chưa deliver observable behavior. Candidate không thể accept. Reassign cho Developer loop 03 (re-plan hoặc continue) hoàn thành T2/T5/T6/T7/T8 — preservation list giờ thêm loop02 P1-P4 + T1/T3/T4 verified behaviors.

Master plan NOT done — loop 02/19 + partial, 4/54 P09 + 0 P10 tests ported. Runtime owns subsequent loops.

**Notes (pre-existing, out-of-scope QA loop 02)**:
- `apps/terminal/{Cargo.toml,src/main.rs}` bị rm trong working tree (git status `D`) — pre-existing break, không do loop 02. QA đã restore từ commit `492a06a` để workspace build được (`git restore --source=HEAD -- terminal_gpui/apps/terminal/` từ parent repo `terminal/`). Báo cho Runtime/Developer fix trong loop sau hoặc commit lại scaffold.
- `cargo xtask audit --final` chưa re-run (loop01 note: `merge symbols` duplicate-id preexisting schema bug, phase-agnostic, out-of-scope).
