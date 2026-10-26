# Issue Ledger — WT full port master plan

Track behavior gaps/regressions qua loop. Mở (`opened`) lần đầu tiên thấy tại loop `t`; đóng
(`closed`) khi verified hành vi đúng; mở lại (`reopened`) khi đã đóng mà fail lại (giữ row
đóng cũ để regression có history). Cross-link tới evidence line.

| id | behavior | loop_opened | loop_closed | status | evidence_ref |
|----|----------|-------------|-------------|--------|--------------|
| L02-G01 | T2 wire `get_perceivable_color` vào `RenderSettings::get_attribute_colors` / `get_attribute_underline_color` khi `ALWAYS_DISTINGUISHABLE` / `INDEXED_DISTINGUISHABLE` bật (loop-02-dev-doc Tasks T2, VR3/VR4/VR6) | 2 | — | opened | loop-02-evidence.md §Unresolved Gaps G01 |
| L02-G02 | T5 settings.rs bổ sung 5 behavioral test (no_adjust_when_flags_off, always_distinguishable_nudges_close_pair, indexed_distinguishable_only_default_or_legacy, skip_when_fg_equals_bg, ul_distinguishable_nudges) (loop-02-dev-doc Task T5, VR1-VR6 white-box) | 2 | — | opened | loop-02-evidence.md §Unresolved Gaps G02 |
| L02-G03 | T6 WT-named integration test `crates/renderer/tests/wt_renderer_colorfix.rs` 3 test (wt_colorfix__perceivable_color_distinguishable, wt_renderer__always_distinguishable_nudges_close_pair, wt_renderer__indexed_distinguishable_requires_default_or_legacy) (loop-02-dev-doc Task T6, VR7) | 2 | — | opened | loop-02-evidence.md §Unresolved Gaps G03 |
| L02-G04 | T7 ADR-0015 `docs/decisions/0015-colorfix-oklab-port.md` ghi D1-D4 + Consequences (loop-02-dev-doc Task T7) | 2 | — | opened | loop-02-evidence.md §Unresolved Gaps G04 |
| L02-G05 | T8 ledger P10 update `files.csv` row f00981 (`renderer/base/RenderSettings.cpp`) + f01002 (`renderer/inc/RenderSettings.hpp`) status partial + tạo `symbols.csv` (loop-02-dev-doc Task T8) | 2 | — | opened | loop-02-evidence.md §Unresolved Gaps G05 |
