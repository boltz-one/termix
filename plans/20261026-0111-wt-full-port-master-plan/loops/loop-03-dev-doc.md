---
loop: 3
status: pending
preservation_constraints:
  - p1_loop02_workspace_regression   # workspace --lib test không regress (loop02 baseline 475 pass / 0 fail)
  - p2_loop02_colorfix_unit_intact   # renderer --lib colorfix 8 test vẫn green
  - p3_loop02_settings13_intact      # 13 existing settings test verbatim pass
  - p4_loop02_colortable256_default  # default() vẫn load Campbell 256-table unchanged
  - p5_loop01_terminal_seam_intact   # cargo test -p core --test wt_terminalcore 1 pass
candidate: A_3 (crates/renderer/src/settings.rs + tests/wt_renderer_colorfix.rs + docs/decisions/0015-*.md + parity/ledger/P10-renderer/{files,symbols}.csv)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 03 — Wire ColorFix vào RenderSettings + behavioral/integration tests + ADR-0015 + ledger P10

## Objective

**Wire `get_perceivable_color` vào `RenderSettings::get_attribute_colors` / `get_attribute_underline_color` khi `ALWAYS_DISTINGUISHABLE` / `INDEXED_DISTINGUISHABLE` bật; kèm behavioral + WT-named integration tests, ADR-0015, ledger P10.**

Một câu: Hai màu gần nhau (ΔEOK² < 0.25) → fg được nudge sáng/tối để distinguishable khi flag on; phẩm chất observable qua `cargo test -p renderer` end-to-end.

## Context

Loop 02 port `colorfix.rs` (T1) + expose (T3) + 8 inline test (T4) — tất cả verified pass (E_2 §Verified Behaviors). Loop 02 **omitted** 5/8 task: T2 (wire), T5 (settings behavioral test), T6 (wt-named integration test), T7 (ADR-0015), T8 (ledger P10) — mở 5 gap L02-G01..L02-G05 (`issue-ledger.md`).

Loop 03 = complete the increment: cận Touch **chỉ** `settings.rs` (T2) + thêm test (T5/T6) + viết doc/ledger (T7/T8). **KHÔNG recreate** colorfix.rs — đã có và verified.

**WT ground (verify read-only)**:
- `windows/src/renderer/base/RenderSettings.cpp:215-225` — `if (_renderMode.any(Indexed|Always) && fg != bg && (Always || (fg.IsDefaultOrLegacy && bg.IsDefaultOrLegacy))) { fg = GetPerceivableColor(fg, bg, 0.5f*0.5f); }`.
- `windows/src/renderer/base/RenderSettings.cpp:280-286` — underline variant: `if (ul != bg && (Always || (indexed && ul.IsDefaultOrLegacy && attr.GetBackground().IsDefaultOrLegacy))) { ul = GetPerceivableColor(ul, bg, 0.5f*0.5f); }`.
- `Feature_AdjustIndistinguishableText::IsEnabled()` là compile-time flag (always-on retail); ta mặc định enabled (`ALWAYS_DISTINGUISHABLE`/`INDEXED_DISTINGUISHABLE` trong `render_mode` mới gating).

**Rust hiện trạng** (`crates/renderer/src/settings.rs`):
- `get_attribute_colors` (line 132-160): trả `(fg, bg)` qua `lookup_color` + `dim_fg` + `swap` + invisible; line 156-158 chèn adjust block **sau** invisible check.
- `get_attribute_underline_color` (line 188-206): line 202-204 chèn adjust block **sau** invisible check.
- Doc-comment line 128-131 vẫn ghi "Không có ColorFix step" — xóa/đổi.
- `RenderModeFlags` đã có `ALWAYS_DISTINGUISHABLE`, `INDEXED_DISTINGUISHABLE` (bitflags, line ~50).
- `colorfix::get_perceivable_color` đã `pub` (loop 02 T1/T3) — `crates/renderer/src/colorfix.rs:168` + re-export `lib.rs:46`.

## Tasks

### T2 — Wire ColorFix vào `crates/renderer/src/settings.rs`

T2.1. Trong `get_attribute_colors`, **SAU** `if attr.is_invisible() { fg = bg; }` (line 156-158), thêm:

```rust
// Port WT `RenderSettings.cpp:215-225` (`Feature_AdjustIndistinguishableText`).
// `IsDefaultOrLegacy` ≡ default (idx==0 và raw==0, lookup_color nhánh fallback)
// OR legacy (16-color index). Mirror lookup_color branch logic (line 99-122).
let fg_default_or_legacy =
    attr.is_legacy || (attr.fg_index == 0 && attr.foreground == 0);
let bg_default_or_legacy =
    attr.is_legacy || (attr.bg_index == 0 && attr.background == 0);
let mode = self.render_mode;
let always = mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE);
let indexed = mode.contains(RenderModeFlags::INDEXED_DISTINGUISHABLE);
if (always || indexed) && fg != bg && (always || (fg_default_or_legacy && bg_default_or_legacy)) {
    fg = crate::colorfix::get_perceivable_color(fg, bg, 0.5 * 0.5);  // = 0.25
}
```

T2.2. Trong `get_attribute_underline_color`, **SAU** `if attr.is_invisible() { ul = bg; }` (line 202-204), thêm:

```rust
// Port WT `RenderSettings.cpp:280-286`.
let ul_default_or_legacy =
    attr.underline_color == 0 || attr.is_legacy;
let bg_default_or_legacy =
    attr.is_legacy || (attr.bg_index == 0 && attr.background == 0);
let mode = self.render_mode;
let always = mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE);
let indexed = mode.contains(RenderModeFlags::INDEXED_DISTINGUISHABLE);
if ul != bg && (always || (indexed && ul_default_or_legacy && bg_default_or_legacy)) {
    ul = crate::colorfix::get_perceivable_color(ul, bg, 0.5 * 0.5);
}
```

T2.3. Cập nhật doc-comment `get_attribute_colors` (line 125-131): xóa "Không có ColorFix step", đổi thành:
```rust
/// Adjust fg nếu gần bg (ΔEOK² < 0.25) khi `ALWAYS_DISTINGUISHABLE` /
/// `INDEXED_DISTINGUISHABLE` trong `render_mode` bật — port WT
/// `Feature_AdjustIndistinguishableText` (`RenderSettings.cpp:215-225`).
/// ADR-0015. Flag default empty → adjust path skip (backwards-compat).
```
Tương tự cho `get_attribute_underline_color` doc-comment (line 184-187).

T2.4. **KHÔNG touch `crates/core/`** (P10 isolation): `IsDefaultOrLegacy` dùng inline expression.
T2.5. **KHÔNG modify** `RenderSettings::default()` (P4); KHÔNG modify existing dim/swap/invisible logic.

### T5 — Behavioral tests append `crates/renderer/src/settings.rs` `#[cfg(test)] mod tests`

Append 5 test sau (mirror D_2 spec, fix typo từ loop 02 doc):

- `test_get_attribute_colors_no_adjust_when_flags_off` — flag = empty, fg/bg default → fg trả default palette (path skip).
- `test_get_attribute_colors_always_distinguishable_nudges_close_pair` — fg/bg cùng gần (`0xFF555555` & `0xFF575757`), bật `ALWAYS_DISTINGUISHABLE` → `fg_after != fg_before` ∧ `fg_after != bg` ∧ ΔEOK²(fg_after, bg) ≥ 0.25 (recompute qua `colorfix::colorref_to_linear`/`oklab`, hoặc assert cách L đủ lớn).
- `test_get_attribute_colors_indexed_distinguishable_only_default_or_legacy` — bật `INDEXED_DISTINGUISHABLE` (KHÔNG `ALWAYS`), fg/bg default → adjust; đổi sang fg = RGB literal non-legacy (`fg_index=0, foreground=0x55555555`) → KHÔNG adjust.
- `test_get_attribute_colors_skip_when_fg_equals_bg` — fg = bg literal, mọi flag on → trả fg==bg unchanged (guard `fg != bg`).
- `test_get_attribute_underline_color_distinguishable_nudges` — ul gần bg, `ALWAYS_DISTINGUISHABLE` on → `ul_after != bg`.

Helper local: `fn mk_attr(...) -> TextAttribute` dựng attr cho test; `RenderSettings::default()` baseline + set flag qua `settings.render_mode |= RenderModeFlags::ALWAYS_DISTINGUISHABLE` (verify `render_mode` public mut access hoặc có `set_render_mode`/`with_render_mode`; nếu private, add `#[cfg(test)]` setter minimal — **chỉ trong test mod**).

### T6 — WT-named integration test `crates/renderer/tests/wt_renderer_colorfix.rs`

Tạo dir `crates/renderer/tests/` (chưa tồn tại) + file 3 test (mirror D_2 spec T6):

```rust
//! Port-of-intent: WT `ColorFix::GetPerceivableColor` (types/ColorFix.cpp:167)
//! + `RenderSettings::GetAttributeColors` AdjustIndistinguishable block
//! (RenderSettings.cpp:215-225).
use renderer::colorfix::get_perceivable_color;
use renderer::settings::{RenderSettings, RenderModeFlags};
use core::TextAttribute;

#[test] fn wt_colorfix__perceivable_color_distinguishable() { ... }
#[test] fn wt_renderer__always_distinguishable_nudges_close_pair() { ... }
#[test] fn wt_renderer__indexed_distinguishable_requires_default_or_legacy() { ... }
```

Convention `wt_<suite>__<behavior>` (double underscore, mirror loop-01 `wt_terminalcore__`).

### T7 — ADR-0015 `docs/decisions/0015-colorfix-oklab-port.md`

Record (mirror D_2 spec T7):
- **Status** `accepted`, date 2026-10-26, phase P10, supersedes [], related [ADR-0003].
- **Context**: ColorFix port (loop 02) + AdjustIndistinguishable wire (loop 03) — WT `Feature_AdjustIndistinguishableText` re-expression trên Rust+GPUI.
- **D1**: ColorFix port vào `renderer/src/colorfix.rs` (KHÔNG crate `color` riêng — YAGNI, 1 consumer now = RenderSettings; future caller `AdjustLightness`/`GetLightness` re-export via `renderer::colorfix`; tách crate nếu ≥3 consumers).
- **D2**: `cbrt_est` bit-hack port nguyên (max err 6.7e-4) thay `f32::cbrt` (libm) — mirror WT fast-path + determinism cross-platform; cost đo perf phase sau.
- **D3**: `SRGB_TO_RGB_LUT[256]` copy nguyên decimal f32 (hex-float không stable Rust) — static const, mirror WT, zero init cost.
- **D4**: `IsDefaultOrLegacy` expr inline trong `settings.rs` (không extend `core::TextAttribute`) — P10 isolation, không touch frozen struct P01.
- **Consequences**: `get_attribute_colors` giờ mutate fg (previously pure lookup) khi flag on; `get_attribute_underline_color` tương tự. Backwards-compat: flag default empty → no behavior change (T5 `no_adjust_when_flags_off`). `Feature_AdjustIndistinguishableText::IsEnabled()` WT runtime probe — ta bỏ, mặc định enabled (gating qua `render_mode` flags).

### T8 — Ledger P10 update `parity/ledger/P10-renderer/{files,symbols}.csv`

P10 dir đã có `files.csv` (51 row seed, status `not-started`). **Update** 2 row + **tạo** `symbols.csv`:

`files.csv`:
- Row `f00981`(`renderer/base/RenderSettings.cpp`) → `rust_path=crates/renderer/src/settings.rs`, `status=partial`, `test_ids=wt_renderer__always_distinguishable_nudges_close_pair;wt_renderer__indexed_distinguishable_requires_default_or_legacy;test_get_attribute_colors_no_adjust_when_flags_off;test_get_attribute_colors_skip_when_fg_equals_bg;test_get_attribute_underline_color_distinguishable_nudges`, `adr=ADR-0015;ADR-0003`, `author=P10-worker`, `notes=AdjustIndistinguishable wire (loop 02 colorfix + loop 03 wire)`.
- Row `f01002`(`renderer/inc/RenderSettings.hpp`) → `status=partial`, `notes=RenderSettings API Rust port (loop 03 settings.rs)`, otros fields như f00981.

Tạo `symbols.csv` (header `id,symbol,wt_anchor,kind,rust_path,status,test_ids,adr,notes`):
- `s001,srgbToRgbLUT,ColorFix.cpp:29,const,crates/renderer/src/colorfix.rs,partial,test_srgb_lut_length_256,ADR-0015,256-entry sRGB→linear LUT`
- `s002,cbrtf_est,ColorFix.cpp:58,fn,crates/renderer/src/colorfix.rs,partial,test_cbrt_est_within_6e4_of_std_cbrt,ADR-0015,fast cube root bit-hack + 1 round Newton`
- `s003,oklab::linear_srgb_to_oklab,ColorFix.cpp:104,fn,crates/renderer/src/colorfix.rs,partial,test_linear_srgb_oklab_roundtrip,ADR-0015,OKlab transform`
- `s004,oklab::oklab_to_linear_srgb,ColorFix.cpp:121,fn,crates/renderer/src/colorfix.rs,partial,test_linear_srgb_oklab_roundtrip,ADR-0015,OKlab inverse`
- `s005,GetPerceivableColor,ColorFix.cpp:167,fn,crates/renderer/src/colorfix.rs,partial,wt_colorfix__perceivable_color_distinguishable,ADR-0015,ΔEOK nudge`
- `s006,GetLightness,ColorFix.cpp:219,fn,crates/renderer/src/colorfix.rs,partial,test_get_lightness_black_white_red,ADR-0015,OKlab L`
- `s007,AdjustLightness,ColorFix.cpp:212,fn,crates/renderer/src/colorfix.rs,partial,,ADR-0015,OKlab L+offset`
- `s008,GetAttributeColors,RenderSettings.cpp:183,fn,crates/renderer/src/settings.rs,partial,wt_renderer__always_distinguishable_nudges_close_pair,ADR-0015;ADR-0003,AdjustIndistinguishable wire (loop 03)`
- `s009,GetAttributeUnderlineColor,RenderSettings.cpp:259,fn,crates/renderer/src/settings.rs,partial,test_get_attribute_underline_color_distinguishable_nudges,ADR-0015,ul adjust`

## Preservation

- **P1 — Loop 02 workspace regression**: `cargo test --workspace --lib` → loop 02 baseline 475 pass / 0 fail. Loop 03 chỉ touch renderer (settings.rs + tests) — không touch core/connection/font/control/app. Target 475 + (5 T5 + 3 T6) = 483 pass / 0 fail.
- **P2 — Loop 02 colorfix unit intact**: `cargo test -p renderer --lib colorfix` → 8 test (T4 loop 02) vẫn green. T2 không modify colorfix.rs.
- **P3 — Loop 02 settings 13 existing test intact**: `cargo test -p renderer --lib settings::tests` → 13 test (P3 loop 02) verbatim pass. T2 wire có guard `flag default empty → skip`; flag default trong `RenderSettings::default()` empty → 13 existing test (không set flag) → same behavior.
- **P4 — Loop 02 color table 256 default**: `RenderSettings::default().color_table` 256 entry Campbell unchanged; KHÔNG modify `default()` impl.
- **P5 — Loop 01 terminal seam intact**: `cargo test -p core --test wt_terminalcore` → 1 pass. Loop 03 không touch `crates/core/`.

## Validation Requirements

**VR1 — T2 wire fg path (`get_attribute_colors`)**:
- **Given** `RenderSettings` flag `ALWAYS_DISTINGUISHABLE` set, `TextAttribute` fg/bg = near pair (`0xFF555555` / `0xFF575757`).
- **When** gọi `get_attribute_colors(attr)`.
- **Then** returned fg ≠ input fg (nudged) ∧ ≠ bg; ΔEOK²(fg_returned, bg) ≥ 0.25 (recompute trong test qua `colorfix` oklab helpers, or assert sign deltaL + cách L đủ lớn).

**VR2 — T2 wire ul path (`get_attribute_underline_color`)**:
- **Given** flag `ALWAYS_DISTINGUISHABLE` set, attr underline_color gần bg.
- **When** gọi `get_attribute_underline_color(attr)`.
- **Then** returned ul ≠ bg (nudged).

**VR3 — T2 skip when flag off**:
- **Given** `render_mode` flag default empty.
- **When** gọi `get_attribute_colors(attr)` với near pair.
- **Then** fg == input unchanged (adjust path skip; mirror existing behavior before T2).

**VR4 — T2 indexed requires default-or-legacy**:
- **Given** flag `INDEXED_DISTINGUISHABLE` only (no `ALWAYS`), fg/bg = default (idx 0, raw 0).
- **When** gọi `get_attribute_colors` → adjust áp dụng. Đổi fg = RGB literal non-legacy → KHÔNG adjust.

**VR5 — T2 skip when fg == bg**:
- **Given** fg = bg literal, flag `ALWAYS_DISTINGUISHABLE` on.
- **When** gọi `get_attribute_colors`.
- **Then** fg == bg unchanged (guard `fg != bg`).

**VR6 — T5 behavioral 5 test green**: `cargo test -p renderer --lib settings::tests` → 13 existing + 5 new = 18 pass / 0 fail.

**VR7 — T6 WT-named integration green**: `cargo test -p renderer --test wt_renderer_colorfix` → 3 pass / 0 fail. Convention `wt_<suite>__<behavior>` đúng.

**VR8 — T7 ADR-0015 exists**: `docs/decisions/0015-colorfix-oklab-port.md` tồn tại, status `accepted`, date 2026-10-26, có D1-D4 + Consequences.

**VR9 — T8 ledger P10 updated**: `parity/ledger/P10-renderer/files.csv` row f00981 + f01002 status `partial`, rust_path populated, test_ids + adr populated; `symbols.csv` tồn tại 9 row.

**VR10 — Build + workspace green**:
- `cargo check -p renderer` → 0 error.
- `cargo test -p renderer --lib` → ≥ 53 + 5 (T5) = 58 pass / 0 fail (colorfix 8 + settings 18 + paint/payload/cell/grid 32).
- `cargo test -p renderer --test wt_renderer_colorfix` → 3 pass / 0 fail.
- `cargo test --workspace --lib` → ≥ 483 pass / 0 fail (P1).
- `cargo test -p core --test wt_terminalcore` → 1 pass (P5).

## Out-of-scope

- **`RenderSettings` keys còn thiếu** (`get_color_alias_index`, `set_color_table_entry`, `set_render_mode` xịn ngoài bitflags) — P10 loop sau (step 3+).
- **`Renderer` invalidation/cursor/paint driver** — P10 step 2.
- **Font stack** — P10 step 3.
- **Style paint (underline variants, strike, overline, blink paint), builtin glyphs, cursor shapes** — P10 step 4-6.
- **Atlas/GpuiEngine** — P10 step 5+ (ADR-0003).
- **`ColorFix::GetLightness`/`AdjustLightness` consumer** (Tab.cpp port, AtlasEngine selection) — port-only, không wire; phase P12+ / P10 step 5.
- **Perf benchmarking** — P10 step 8.
- **Golden snapshot SGR showcase** — chờ paint driver P10 step 2; VR1-VR5 behavioral black-box, không image diff.
- **`Feature_AdjustIndistinguishableText` runtime probe** — ta bỏ, mặc định enabled, gating qua `render_mode` flags.
- **Tách crate `color`** — YAGNI (D1 loop 02 ADR-0015); tách sau nếu ≥3 consumers.
- **`cbrt` vs `cbrt_est` perf** — chờ step 8.
