---
adr: 0015
title: ColorFix OKlab port + AdjustIndistinguishable wire
status: accepted
date: 2026-10-26
phase: P10
supersedes: []
related: [ADR-0003]
---

# ADR-0015 — ColorFix OKlab port + AdjustIndistinguishable wire

## Context

Phase P10 (renderer) port `windows/src/types/ColorFix.cpp` (OKlab color-appearance
math: `linear_srgb ↔ oklab`, `GetPerceivableColor`, `GetLightness`,
`AdjustLightness`) + wire `Feature_AdjustIndistinguishableText` vào
`RenderSettings::GetAttributeColors` (`base/RenderSettings.cpp:215-225`) và
`GetAttributeUnderlineColor` (`:280-286`). Loop 02 port `colorfix.rs` (T1);
loop 03 wire (T2) + behavioral/integration tests (T5/T6).

WT `Feature_AdjustIndistinguishableText::IsEnabled()` là compile-time flag
(always-on retail). Re-expression trên Rust+GPUI: mặc định enabled, gating
qua `RenderModeFlags::ALWAYS_DISTINGUISHABLE`/`INDEXED_DISTINGUISHABLE` trong
`render_mode` (bitflags). Phẩm chất observable qua `cargo test -p renderer`
end-to-end — near-by-ΔEOK² pair (`0xFF555555`/`0xFF575757`) nudged → output
≠ input, idempotent re-nudge.

## Decisions

### D1 — ColorFix port vào `renderer/src/colorfix.rs` (không crate `color` riêng)

ColorFix có 1 consumer ngay = `RenderSettings`. YAGNI — tách crate `color` riêng
trả về setup/deplicensing overhead không có caller thứ hai. Future caller
(`AdjustLightness`/`GetLightness` cho Tab.cpp port, AtlasEngine selection)
re-export via `renderer::colorfix` (`lib.rs:46`); tách crate nếu ≥3 consumers.

### D2 — `cbrt_est` bit-hack port nguyên thay `f32::cbrt` (libm)

`ColorFix.cpp:58-67` dùng FP32 bit-hack `u/3 + 709921077` + 1 round Newton —
fast-path + determinism cross-platform (libm `cbrt` platform-specific rounding).
Max err `6.7e-4` vs `f32::cbrt` (test `test_cbrt_est_within_6e4_of_std_cbrt`).
Cost đo perf phase sau (P10 step 8).

### D3 — `SRGB_TO_RGB_LUT[256]` copy nguyên decimal f32

WT C++ dùng hex-float literal (`0x1.3e4568p-12f`) cho 256-entry sRGB→linear LUT.
Rust stable **chưa** support hex float literal (nightly `hex_float_literal`
only) → convert sang decimal f32 with full round-trip precision (bit-identical
giá trị với C++ origin). Static const, zero init cost. `clippy::excessive_precision`
allow crate-wide (`lib.rs:29`) — truncate theo suggest break port-verbatim intent.

### D4 — `IsDefaultOrLegacy` inline expr trong `settings.rs`

T2 wire dùng inline expression `attr.is_legacy || (attr.fg_index == 0 && attr.foreground == 0)`
cho `IsDefaultOrLegacy` (mirror WT `TextColor::IsDefaultOrLegacy`). KHÔNG extend
`core::TextAttribute` — P10 isolation, không touch frozen struct (P01). Mirror
`lookup_color` branch logic (line 99-122). `is_legacy` global field của
`TextAttribute` (set true bởi `set_indexed_foreground`/`set_indexed_background`)
route cả fg/bg vào legacy path trong `lookup_color` — WT semantic, giữ.

## Consequences

- `get_attribute_colors` giờ mutate fg (previously pure lookup) khi flag on;
  `get_attribute_underline_color` tương tự. Backwards-compat: flag default
  empty → no behavior change (`test_get_attribute_colors_no_adjust_when_flags_off`).
- 13 existing settings test verbatim pass (P3): flag default empty trong
  `RenderSettings::default()` → adjust path skip → same behavior.
- `Feature_AdjustIndistinguishableText::IsEnabled()` WT runtime probe — ta bỏ,
  mặc định enabled, gating qua `render_mode` flags. Nếu sau cần toggle runtime
  (accessibility setting), add `bool` field + AND với bitflags guard.
- `oklab` mod private trong `colorfix.rs` — `get_lightness`/`get_perceivable_color`
  pub re-export qua `lib.rs:46`. ΔEOK² check (3 oklab axes) không thể recompute
  trong test qua public API → dùng idempotent re-nudge proxy (nudged fg vs
  bg đã distinguishable → re-call trả unchanged).
