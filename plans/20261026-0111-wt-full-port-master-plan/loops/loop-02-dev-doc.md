---
loop: 2
status: pending
preservation_constraints:
  - p1_loop01_workspace_regression  # 462 lib test pass / 0 fail (chỉ +loop02 test, không regress)
  - p2_loop01_terminal_seam_intact     # wt_terminalcore__test_notify_scrolling vẫn green
  - p3_settings_rendermode_flags_api   # 13 test hiện có trong settings.rs không regress (Campbell table, flags, FG/BG, alpha, underline)
  - p4_color_table_256_default         # default() vẫn load Campbell 256-table unchanged
candidate: A_2 (crates/renderer/src/{colorfix.rs,settings.rs} + tests/wt_renderer_colorfix.rs)
builder: Developer (vk-hoh-dev)
assessor: QATester (vk-hoh-qa)
---

# Loop 02 — ColorFix port + AdjustIndistinguishable wire vào RenderSettings

## Objective

**Port `ColorFix` (oklab + sRGB LUT + `GetPerceivableColor`) và wire vào `RenderSettings::get_attribute_colors` / `get_attribute_underline_color` khi `ALWAYS_DISTINGUISHABLE` / `INDEXED_DISTINGUISHABLE` bật.**

Một câu: Hai màu gần nhau (ΔEOK < sqrt(0.25)) → fg được nudge sáng tối để distinguishable; phẩm chất observable khi render SGR showcase có 2 row cùng FG gần BG.

## Context (gọi từ S)

Phase 10 step 1 (P10 phase doc): "Ledger renderer/base+atlas symbols/behaviours; define visual feature matrix; port `RenderSettings` keys (`AdjustIndistinguishableColors`, …)". Loop 02 là step 1.2 — port `RenderSettings` **remainder** còn thiếu: `ColorFix::GetPerceivableColor` (gap hiện có trong `crates/renderer/src/settings.rs:128` comment "Không có ColorFix step").

**WT ground (verify read-only)**:
- `windows/src/types/ColorFix.cpp:29-46` — `srgbToRgbLUT[256]` (8-bit sRGB → linear RGB, hex-float constant table, generated OK).
- `windows/src/types/ColorFix.cpp:88-137` — `oklab::linear_srgb_to_oklab` + `oklab_to_linear_srgb` (Björn Ottosson OKlab, public domain).
- `windows/src/types/ColorFix.cpp:51-83` — `saturate` + `cbrtf_est` (fast cube root, "fast inverse sqrt trick" + 1 round Newton, max err 6.7e-4).
- `windows/src/types/ColorFix.cpp:167-210` — `GetPerceivableColor(color, reference, minSquaredDistance)`: tính ΔEOK² = dl²+da²+db²; nếu `≥ minSquaredDistance` → return color; else `deltaL = sqrt(minSq - da² - db²)` (sign theo `color.l < ref.l`), `color.l = ref.l + deltaL` (clamp 0..1, nếu oos flip sign), convert lại sRGB, preserve alpha byte (`& 0xff000000`).
- `windows/src/renderer/base/RenderSettings.cpp:215-225` — `if (_renderMode.any(Indexed|Always) && fg != bg && (AlwaysDistinguishable || (fg.IsDefaultOrLegacy && bg.IsDefaultOrLegacy))) { fg = GetPerceivableColor(fg, bg, 0.5f*0.5f); }` — `Feature_AdjustIndistinguishableText::IsEnabled()` là compile-time feature flag (always-on ở retail build, gating probe già).
- `windows/src/renderer/base/RenderSettings.cpp:280-286` — underline variant: `if (ul != bg && (AlwaysDistinguishable || (Indexed && ul.IsDefaultOrLegacy && attr.GetBackground().IsDefaultOrLegacy))) { ul = GetPerceivableColor(ul, bg, 0.25); }`.

## Tasks

### T1 — Create `crates/renderer/src/colorfix.rs`

Port verbatim `windows/src/types/ColorFix.cpp` (toàn file 225 line) sang Rust:

- `srgbToRgbLUT[256]: [f32; 256]` — copy hex-float constants y nguyên từ `ColorFix.cpp:30-45`. (Rust hex float `0x1.3e4568p-12f` syntax tương tự C++.)
- `fn saturate(f: f32) -> f32` — `f.clamp(0.0, 1.0)` (Rust idiomatic, không cần manual) hoặc mirror WT `f < 0.0 ? 0.0 : (f > 1.0 ? 1.0 : f)`.
- `fn cbrt_est(a: f32) -> f32` — port union FP32 bit-hack + Newton 1 round. Rust dùng `f32::to_bits` / `f32::from_bits` (`std::mem::transmute` không idiomatic).
- `mod oklab { struct Lab{...} struct RGB{...} fn linear_srgb_to_oklab(c: RGB) -> Lab; fn oklab_to_linear_srgb(c: Lab) -> RGB; }` — copy coefficients verbatim.
- `fn colorref_to_linear(c: u32) -> RGB` — `(srgb_lut[(c & 0xff) as usize], srgb_lut[((c>>8)&0xff) as usize], srgb_lut[((c>>16)&0xff) as usize])`. WT encoding: COLORREF = R | (G<<8) | (B<<16), ABGR layout của renderer (a<<24|b<<16|g<<8|r) cần xác nhận — **NOTE quan trọng**: tham khảo setting.rs line 144-151 đã xử lý halving ABGR với `0x7F7F7F7F` mask + `0xFF000000` restore. ColorFix nhận packed color và trả packed color với cùng layout. T1 cần nhận ABGR (vì color_table lưu ABGR). Sửa: `colorref_to_linear` đọc `(c) & 0xff` = R (vì ABGR low byte = R), `((c>>8)&0xff)` = G, `((c>>16)&0xff)` = B — **GIỐNG COLORREF** vì WT cũng COLORREF = R|(G<<8)|(B<<16); chỉ khác byte cao = A trong ABGR vs 0 trong COLORREF. → port không đổi. Verify bằng test T5 (round-trip: ABGR color qua ColorFix → ABGR color).
- `fn linear_to_colorref(c: RGB) -> u32` — port verbatim `RenderSettings.cpp:153-162` (saturate + sRGB companding + `lrintf` rounding → `r | (g<<8) | (b<<16)`). Rust `f32::round_ties_even()` = `lrintf` gần nhất (`std::round` half-away vs `lrint` half-even — WT dùng `lrintf`=`half-even`; Rust `round_ties_even`).
- `pub fn get_perceivable_color(color: u32, reference: u32, min_squared_distance: f32) -> u32` — port `GetPerceivableColor` (line 167-210). ΔEOK², early-return, deltaL sign theo `color.l < ref.l`, clamp flip, preserve alpha `| (color & 0xff000000)`.
- `pub fn get_lightness(color: u32) -> f32` — port line 219-222 (dùng cho future tab-color logic; T1 include vì chỉ 3 line + reusing ColorFix).
- `pub fn adjust_lightness(color: u32, delta: f32) -> u32` — port line 212-217 (Rust `lab.l = (lab.l + delta).clamp(0.0,1.0)`).

Module doc-comment: phạt `windows/src/types/ColorFix.cpp` credit Björn Ottosson OKlab (public domain). **KHÔNG vendor `TIL_FAST_MATH` BEGIN/END** — Rust `f32` math không cần fast-math pragmas; cbrt_est bit-hack port nguyên để mirror fast-path (đo perf phase sau nếu cần `std::f32::cbrt`).

### T2 — Wire ColorFix vào `crates/renderer/src/settings.rs`

`get_attribute_colors` (sau `if attr.is_invisible() { fg = bg; }`, dòng 156-158):

```rust
// Port WT `RenderSettings.cpp:215-225` (Feature_AdjustIndistinguishableText).
let fg_default_or_legacy = /* T2 verify: xem TextAttribute::IsDefaultOrLegacy */;
let bg_default_or_legacy = /* same */;
let mode = self.render_mode;
let adjust = mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE)
    || mode.contains(RenderModeFlags::INDEXED_DISTINGUISHABLE);
let always = mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE);
if adjust && fg != bg && (always || (fg_default_or_legacy && bg_default_or_legacy)) {
    fg = crate::colorfix::get_perceivable_color(fg, bg, 0.5 * 0.5);  // = 0.25
}
```

`IsDefaultOrLegacy` WT (`TextColor.h`): default (không set) OR legacy (16-color index). Trong `TextAttribute` rust:
- default ≡ `fg_index == 0 && foreground == 0 && !is_legacy` (lookup_color nhánh fallback).
- legacy ≡ `is_legacy == true`.
- T2 cần expose helper `TextAttribute::fg_is_default_or_legacy(&self) -> bool` / `bg_is_default_or_legacy` ở `crates/core/src/attrs.rs` (1-line inline method, **out-of-scope?** — NO: core struct đã frozen P01? Verify P09 PDR/consequences. Nếu frozen, dùng inline expression trong settings.rs thay không modify core.

**Decision**: T2 dùng inline expression trong `settings.rs` (không touch core, giữ P10 isolation — reads via crate boundary):

```rust
let fg_default_or_legacy = attr.is_legacy || (attr.fg_index == 0 && attr.foreground == 0);
let bg_default_or_legacy = attr.is_legacy || (attr.bg_index == 0 && attr.background == 0);
```

Mirror `lookup_color` branch logic (line 99-122). Verify bằng T5 test (default fg/bg default-or-legacy = true, RGB literal = false).

`get_attribute_underline_color` (sau `if attr.is_invisible() { ul = bg; }`, dòng 202-204):

```rust
let ul_default_or_legacy = attr.underline_color == 0 || attr.is_legacy;
let bg_default_or_legacy = attr.is_legacy || (attr.bg_index == 0 && attr.background == 0);
let mode = self.render_mode;
if ul != bg && (mode.contains(RenderModeFlags::ALWAYS_DISTINGUISHABLE)
    || (mode.contains(RenderModeFlags::INDEXED_DISTINGUISHABLE) && ul_default_or_legacy && bg_default_or_legacy)) {
    ul = crate::colorfix::get_perceivable_color(ul, bg, 0.5 * 0.5);
}
```

Update module doc-comment (line 1-15) + `get_attribute_colors` doc-comment (line 125-131): xóa "Không có ColorFix step", đổi thành "Port `ColorFix::GetPerceivableColor` khi `Indexed`/`Always` mode bật (ADR-0003 Phase 10 step 1.2)".

### T3 — Module expose trong `crates/renderer/src/lib.rs`

Add `pub mod colorfix;` (mirror `pub mod settings;`). Verify re-export: `renderer::colorfix::get_perceivable_color`.

### T4 — Test `crates/renderer/src/colorfix.rs` inline (white-box unit tests)

Inline `#[cfg(test)] mod tests` dưới colorfix.rs:

- `test_srgb_lut_length_256` — `assert_eq!(srgbToRgbLUT.len(), 256)` + assert endpoints `LUT[0] == 0.0`, `LUT[255] == 1.0` (round-tolerance 1e-6).
- `test_cbrt_est_within_6e4_of_std_cbrt` — for `a in [0.0, 0.1, 0.25, 0.5, 0.9, 1.0]`: `(cbrt_est(a) - a.cbrt()).abs() < 6.7e-4` (mirror WT comment "max error 6.7e-4").
- `test_linear_srgb_oklab_roundtrip` — pick 5 colors (black, white, red `0xFF0000`, green `0x00FF00`, blue `0x0000FF`), convert to OKlab, back, compare RGB channels ±1 (per `lrintf` rounding).
- `test_get_lightness_black_white_red` — black `0xFF000000` → ~0.0; white `0xFFFFFFFF` → ~1.0; red `0xFF0000FF` (ABGR) → oklab l ≈ 0.5 (verify trong tolerance 0.05).
- `test_get_perceivable_color_far_colors_pass_through` — `color = 0xFF0000FF` (red), `ref = 0xFF00FF00` (green), `min_sq = 0.25` → return `color` unchanged (ΔEOK² > 0.25).
- `test_get_perceivable_color_identical_colors_nudges` — `color = ref = 0xFF555555` → return ≠ ref (l nudged), ABGR alpha preserved (high byte = 0xFF).
- `test_get_perceivable_color_preserves_alpha_byte` — `color = 0x80123456` (alpha 0x80), `ref = 0xFF000000`, `bg` default → return high byte == 0x80 (mask `& 0xff000000`).
- `test_get_perceivable_color_darker_than_ref_darkens` — brightness relação: color darker than ref → darkens further (sign -deltaL), check returned color `l < ref.l` (Oklab check).

### T5 — Test `crates/renderer/src/settings.rs` inline (behavioral, port-named)

Append to existing `#[cfg(test)] mod tests`:

- `test_get_attribute_colors_no_adjust_when_flags_off` — flag = 0, fg khíttítre = bg (default+default, faint/intense=false) → fg trả default fg palette (không nudge, fg==bg vẫn fg==bg — WT `fg != bg` guard early-skip).
- `test_get_attribute_colors_always_distinguishable_nudges_close_colors` — set fg/bg cùng gần (~`0xFF555555` & `0xFF575757`), bật `ALWAYS_DISTINGUISHABLE` → `fg_after != fg_before` và `fg_after != bg`. Verify deltaL sign & ΔEOK² ≥ 0.25 sau adjust.
- `test_get_attribute_colors_indexed_distinguishable_only_default_or_legacy` — bật `INDEXED_DISTINGUISHABLE`, set fg/bg default (idx 0, no RGB) → adjust. Đổi sang fg = RGB literal (`set_color(0x555555)` non-legacy) → KHÔNG adjust (IsDefaultOrLegacy = false).
- `test_get_attribute_colors_skip_when_fg_equals_bg` — fg = bg literally, bật luôn༨adjust flags → trả fg==bg unchanged (WT line 220 `fg != bg` guard).
- `test_get_attribute_underline_color_distinguishable_nudges` — set ul gần bg + bật `ALWAYS_DISTINGUISHABLE` → `ul_after != bg` và ΔEOK² ≥ 0.25.

### T6 — WT-named integration test `crates/renderer/tests/wt_renderer_colorfix.rs`

Mirror WT (no `ColorFixTest.cpp` exists → spec-by-behaviour):

```rust
//! Port-of-intent: WT `ColorFix::GetPerceivableColor` (types/ColorFix.cpp:167)
//! + `RenderSettings::GetAttributeColors` AdjustIndistinguishableText block
//! (RenderSettings.cpp:215-225). Behavior: 2 màu gần nhau → fg được nudge
//! để distinguishable khi `Indexed`/`Always` mode bật.

use renderer::colorfix::get_perceivable_color;
use renderer::settings::{RenderSettings, RenderModeFlags};
use core::TextAttribute;

#[test] fn wt_colorfix__perceivable_color_distinguishable() { ... }
#[test] fn wt_renderer__always_distinguishable_nudges_close_pair() { ... }
#[test] fn wt_renderer__indexed_distinguishable_requires_default_or_legacy() { ... }
```

3 test. Verbatim-naming `wt_<suite>__<behavior>` (double underscore, mirror loop-01 convention).

### T7 — ADR-0015 write `docs/decisions/0015-colorfix-oklab-port.md`

ADR này record:
- **Decision D1**: `ColorFix` port vào `renderer/src/colorfix.rs` (KHÔNG tạo crate `color` mới — YAGNI; only 1 consumer now = RenderSettings. Future caller (Tab.cpp port `AdjustLightness`, AtlasEngine `GetLightness`) re-export via `renderer::colorfix`, hoặ category fit nếu fountain of use cases → tách crate riêng phase sau).
- **D2**: `cbrt_est` bit-hack port nguyên (max err 6.7e-4) thay `f32::cbrt` (libm) để mirror WT fast-path + determinism cross-platform. Cost `< libm` (đã đo perf phase sau nếu cần).
- **D3**: `srgbToRgbLUT[256]` copy nguyên hex-float constants (plymorphism vs generating LUT at runtime — choose static const để mirror WT + zero init cost).
- **D4**: `IsDefaultOrLegacy` expr inline trong `settings.rs` (không extend `core::TextAttribute`) — P10 isolation, không touch P01 frozen struct.
- **Consequences**: `RenderSettings::get_attribute_colors` giờ có thể mutate fg (previously pure lookup) khi flag on. `get_attribute_underline_color` tương tự. Backwards-compat: flag default = empty → no behavior change (T5 `no_adjust_when_flags_off`). Status `accepted`, date 2026-10-26, phase P10, supersedes [], related [ADR-0003].

### T8 — Ledger P10 update `parity/ledger/P10-renderer/`

Tạo NEW ledger dir `parity/ledger/P10-renderer/{files,symbols}.csv` (P10 chưa có dir — phase doc chưa step 1).

**`files.csv`** (port-of-intent tracked):
```
file_id,wt_path,rust_path,status,test_ids,adr,notes
f01001,types/ColorFix.cpp,crates/renderer/src/colorfix.rs,partial,wt_colorfix__perceivable_color_distinguishable;test_srgb_lut_length_256,ADR-0015,oklab+sRGB LUT+GetPerceivableColor (loop 02)
f01002,renderer/base/RenderSettings.cpp,crates/renderer/src/settings.rs,partial,wt_renderer__always_distinguishable_nudges_close_pair;wt_renderer__indexed_distinguishable_requires_default_or_legacy,ADR-0015;ADR-0003,AdjustIndistinguishable wire (loop 02)
```

**`symbols.csv`** (port-of-intent, per-symbol):
```
file_id,symbol,wt_anchor,kind,rust_path,status,test_ids,adr,notes
f01001,ColorFix::srgbToRgbLUT,ColorFix.cpp:29,const,crates/renderer/src/colorfix.rs,partial,test_srgb_lut_length_256,ADR-0015,256-entry sRGB→linear LUT
f01001,ColorFix::cbrtf_est,ColorFix.cpp:58,fn,crates/renderer/src/colorfix.rs,partial,test_cbrt_est_within_6e4_of_std_cbrt,ADR-0015,fast cube root bit-hack + 1 round Newton
f01001,oklab::linear_srgb_to_oklab,ColorFix.cpp:104,fn,crates/renderer/src/colorfix.rs,partial,test_linear_srgb_oklab_roundtrip,ADR-0015,OKlab transform
f01001,oklab::oklab_to_linear_srgb,ColorFix.cpp:121,fn,crates/renderer/src/colorfix.rs,partial,test_linear_srgb_oklab_roundtrip,ADR-0015,OKlab inverse
f01001,ColorFix::GetPerceivableColor,ColorFix.cpp:167,fn,crates/renderer/src/colorfix.rs,partial,wt_colorfix__perceivable_color_distinguishable,ADR-0015,ΔEOK nudge
f01001,ColorFix::GetLightness,ColorFix.cpp:219,fn,crates/renderer/src/colorfix.rs,partial,test_get_lightness_black_white_red,ADR-0015,OKlab L
f01001,ColorFix::AdjustLightness,ColorFix.cpp:212,fn,crates/renderer/src/colorfix.rs,partial,,ADR-0015,OKlab L+offset
f01002,RenderSettings::GetAttributeColors,RenderSettings.cpp:183,fn,crates/renderer/src/settings.rs,partial,wt_renderer__always_distinguishable_nudges_close_pair,ADR-0015;ADR-0003,AdjustIndistinguishable wire (loop 02)
f01002,RenderSettings::GetAttributeUnderlineColor,RenderSettings.cpp:259,fn,crates/renderer/src/settings.rs,partial,,ADR-0015,ul adjust (loop 02)
```

## Preservation

- **P1 — Workspace lib regression**: `cargo test --workspace --lib` → 462 (loop-01 baseline) + N loop-02 tests pass / 0 fail. Loop-02 chỉ add `renderer` test (file mới + inline) — không touch core/terminal/scroll.
- **P2 — Loop 01 terminal seam intact**: `cargo test -p core --test wt_terminalcore` → 1 test green. Loop-02 không touch `crates/core/`.
- **P3 — Settings existing 13 test**: `cargo test -p renderer --lib` → 13 test hiện (render_settings_default_loads_campbell_table, render_mode_flags, get_attribute_colors_* × 6, get_attribute_underline_color × 3) + 8 mới (T5) = 21 pass / 0 fail. ✱ Verify không regress: existing test default flag = empty → adjust path skip (T5 test_no adjust mirror).
- **P4 — Color table default 256**: `RenderSettings::default().color_table` còn 256 entry Campbell (P3 subsumes — but explicit vì P4 guard lookup behavior). Không sửa `default()` impl.

## Validation Requirements

**VR1 — ColorFix `GetPerceivableColor` distinguishable path**:
- **Given** `color = 0xFF555555` (mid gray), `reference = 0xFF575757` (gần giống nhau, ΔEOK² < 0.25), `min_squared_distance = 0.25`.
- **When** gọi `get_perceivable_color(color, reference, 0.25)`.
- **Then** return value ≠ color AND ≠ reference; ΔEOK²(returned, reference) ≥ 0.25 (recompute trong test); alpha byte preserved (high byte == 0xFF).

**VR2 — ΔEOK² far-color passthrough**:
- **Given** `color = 0xFF0000FF` (red), `reference = 0xFF00FF00` (green), ΔEOK² ≫ 0.25.
- **When** gọi `get_perceivable_color(color, reference, 0.25)`.
- **Then** return == color (early-return path, no mutation).

**VR3 — Always distinguishable nudges close pair trong `get_attribute_colors`**:
- **Given** `TextAttribute` với fg/bg same near-gray (`set_color(0x555555)` cho cả hai), `RenderSettings` flag `ALWAYS_DISTINGUISHABLE` set.
- **When** gọi `RenderSettings::get_attribute_colors(attr)`.
- **Then** `fg_before` (qua path không adjust với flag off) ≠ `fg_after` (flag on); `fg_after != bg`; `-1.0 < ΔEOK²(fg_after, bg)kachō ≥ 0.25` (thực tế `fg_after` Oklab-L cách `bg` Oklab-L đủ lớn để pass).

**VR4 — Indexed distinguishable requires `IsDefaultOrLegacy`**:
- **Given** `RenderSettings` flag `INDEXED_DISTINGUISHABLE` set (KHÔNG `ALWAYS`).
- **When** set attr fg/bg = default (idx 0, foreground==0) và call `get_attribute_colors`:
- **Then** adjust áp dụng (fg được nudge). Khi đổi sang fg = RGB literal (`set_color(0x555555)`, non-legacy idx>0): KHÔNG adjust (return unchanged). Mirror WT `IsDefaultOrLegacy` guard.

**VR5 — Skip when `fg == bg`**:
- **Given** attr fg = bg literal (same value), bất kỳ `ALWAYS`/`INDEXED` flag on.
- **When** gọi `get_attribute_colors`.
- **Then** return (fg, bg) với fg == bg unchanged. WT line 220 guard `fg != bg`.

**VR6 — Underline color nudge path**:
- **Given** attr với underline_color = near-bg color, `ALWAYS_DISTINGUISHABLE` flag set.
- **When** gọi `get_attribute_underline_color`.
- **Then** returned ul ≠ bg (nudged).

**VR7 — Build + workspace green**:
- **Given** all changes (colorfix.rs, settings.rs, lib.rs, tests, ADR, ledger).
- **When** chạy `cargo check -p renderer` + `cargo test -p renderer --lib` + `cargo test -p renderer --test wt_renderer_colorfix` + `cargo test --workspace --lib` + `cargo test -p core --test wt_terminalcore`.
- **Then** build clean 0 error; `renderer --lib` ≥ 21 pass / 0 fail; `wt_renderer_colorfix` ≥ 3 pass / 0 fail; workspace ≥ 462 pass / 0 fail; `wt_terminalcore` 1 pass (P2 intact).

**VR8 — Existing 13 settings test không regress**:
- **Given** source changes (settings.rs wire adjust + doc).
- **When** chạy `cargo test -p renderer --lib`.
- **Then** 13 existing test (named above P3) pass verbatim (không sửa test code, chỉ thêm test). Flag default = empty → adjust path skipped → same behavior.

## Out-of-scope

- **`RenderSettings` các key còn thiếu**: `get_color_alias_index`, `set_color_table_entry`, `set_render_mode` xịn (chỉ 2 flag bằng bitflags + `toggle_blink` đã có) — chờ P10 loop sau (step 3+).
- **`Renderer` invalidation/cursor/paint driver** — P10 step 2, loop sau.
- **Font stack (`FontInfo`/`FontConfig`)** — P10 step 3.
- **Style rendering (underline variants, strike, overline, blink paint), builtin glyphs, cursor shapes** — P10 step 4-6.
- **Atlas/CpuiEngine/GPUI paint** — P10 step 5+ (ADR-0003).
- **`color` crate tách rời** — YAGNI (D1 đã decide, 1 consumer now). Tách sau nếu ≥ 3 consumers.
- **`ColorFix::GetLightness`/`AdjustLightness`  consumer** (Tab.cpp port `AdjustLightness`, AtlasEngine `GetLightness` cho selection fg) — port-only, không wire consumer. Consumer thuộc phase TerminalApp (P12+) / AtlasEngine (P10 step 5).
- **Perf benchmarking** (`cargo bench -p renderer`) — P10 step 8.
- **Golden snapshot SGR showcase** — chờ paint driver (P10 step 2); VR3-VR6 là behavioral white-box, không image diff.
- **Feature_Attònde flag runtime probe** — WT `Feature_AdjustIndistinguishableText::IsEnabled()` là VE runtime flag probe; ta không implement runtime feature flag, mặc định enabled — comment rõ trong colorfix.rs.
- **`cbrt` vs `cbrt_est` perf measurement** — chờ step 8; loop 02 giữ `cbrt_est` để mirror WT determinism.
