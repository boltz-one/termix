//! `td/color_table.rs` — color table (DECAC/Xterm resource) — Phase 5 W1.
//!
//! Port verbatim `windows/src/terminal/adapter/adaptDispatch.cpp:3318-3476`:
//! `SetColorTableEntry`, `RequestColorTableEntry`, `ResetColorTable`,
//! `ResetColorTableEntry`, `SetXtermColorResource`, `RequestXtermColorResource`,
//! `ResetXtermColorResource`, `AssignColor`.
//!
//! WT impl delegates 100% sang `_renderSettings` (RenderSettings) + optional
//! `_renderer->TriggerRedrawAll`. Cùng pattern như OSC: adapter forward sang
//! host bridge.
//!
//! **Phase 5 status**: `RenderSettings` (`_renderSettings`) chưa port (Phase
//! 4.y / P10 render-parity scope). Tạm thời tất cả methods đánh dấu `// STUB`
//! với intent chú thích — host-side impl giao cho Lane C khi
//! `InteractDispatch` trait extend các method color-table tương ứng. KHÔNG
//! `todo!()` để build pass.
//!
//! Anchors:
//! - `adaptDispatch.cpp:3318` — `SetColorTableEntry`.
//! - `adaptDispatch.cpp:3337` — `RequestColorTableEntry`.
//! - `adaptDispatch.cpp:3348` — `ResetColorTable`.
//! - `adaptDispatch.cpp:3363` — `ResetColorTableEntry`.
//! - `adaptDispatch.cpp:3384` — `SetXtermColorResource` (map osc resource→table).
//! - `adaptDispatch.cpp:3404` — `RequestXtermColorResource`.
//! - `adaptDispatch.cpp:3430` — `ResetXtermColorResource`.
//! - `adaptDispatch.cpp:3454` — `AssignColor` (DECAC, ColorItemswitch).
//!
//! `XtermResourceColorTableMappings` table port chờ — lấy từ
//! `adaptDispatch.cpp` static const array (resource 10..14 → DefaultForeground,
//! DefaultBackground, CursorColor, SelectionBackground).

use super::TerminalDispatch;
use crate::dispatch::types::ColorItem;
use crate::dispatch::VtInt;

/// `SetColorTableEntry(tableIndex, dwColor)` (`adaptDispatch.cpp:3318-3335`).
///
/// WT: `_renderSettings.SetColorTableEntry(tableIndex, dwColor)` +
/// `_renderer->TriggerRedrawAll(backgroundChanged, frameChanged)`.
// STUB: RenderSettings + Renderer chưa port (Phase 4.y / P10) — host bridge
// trait InteractDispatch chưa có method color-table. Wire khi Lane C lands.
pub fn set_color_table_entry(_this: &mut TerminalDispatch, _table_index: usize, _color: u32) {
    // STUB: port khi _renderSettings port —_port verbatim 2 lines:
    //   this.render_settings.set_color_table_entry(table_index, color);
    //   let (bg_changed, frame_changed) = compute_alias_deltas(this, table_index);
    //   this.interact.trigger_redraw_all(bg_changed, frame_changed);
}

/// `RequestColorTableEntry(tableIndex)` (`:3337-3346`).
///
/// WT: get color → if `!= INVALID_COLOR` → `_ReturnOscResponse(fmt::format(
/// "4;{};rgb:{:04x}/{:04x}/{:04x}", tableIndex, c.r*0x0101, ...))`.
// STUB: RenderSettings.GetColorTableEntry + response string port chờ.
pub fn request_color_table_entry(_this: &mut TerminalDispatch, _table_index: usize) {
    // STUB: response format "4;{idx};rgb:{r:04x}/{g:04x}/{b:04x}".
}

/// `ResetColorTable()` (`:3348-3357`).
///
/// WT: `_renderSettings.RestoreDefaultIndexed256ColorTable()` +
/// `_renderer->TriggerRedrawAll(true, true)`.
// STUB: RenderSettings.RestoreDefaultIndexed256ColorTable chờ port.
pub fn reset_color_table(_this: &mut TerminalDispatch) {
    // STUB: trigger redraw all (bg + frame).
}

/// `ResetColorTableEntry(tableIndex)` (`:3363-3380`).
///
/// WT: `_renderSettings.RestoreDefaultColorTableEntry(tableIndex)` +
/// alias-delta redraw (như `SetColorTableEntry`).
// STUB: RenderSettings.RestoreDefaultColorTableEntry chờ port.
pub fn reset_color_table_entry(_this: &mut TerminalDispatch, _table_index: usize) {
    // STUB: alias delta redraw (backgroundIndex/frameIndex port chờ).
}

/// `SetXtermColorResource(resource, color)` (`:3384-3398`).
///
/// WT: `resource >= 10` → `mappingIndex = resource - 10` → lookup
/// `XtermResourceColorTableMappings[mappingIndex]` → if aliasIndex>=0
/// `_renderSettings.SetColorAliasIndex(alias, colorTableIndex)` → recurse
/// `SetColorTableEntry(colorTableIndex, color)`.
// STUB: XtermResourceColorTableMappings table port chờ; mapping resource 10..14.
pub fn set_xterm_color_resource(_this: &mut TerminalDispatch, _resource: usize, _color: u32) {
    // STUB: assert resource>=10; map φοξ table- idx; alias pass-through.
}

/// `RequestXtermColorResource(resource)` (`:3404-3426`).
///
/// WT: resolve colorTableIndex (alias if aliasIndex>=0) → get color →
/// `_ReturnOscResponse("{resource};rgb:{r:04x}/{g:04x}/{b:04x}")`.
// STUB: RenderSettings.GetColorAliasIndex/GetColorTableEntry + response port chờ.
pub fn request_xterm_color_resource(_this: &mut TerminalDispatch, _resource: usize) {
    // STUB: response format "{resource};rgb:{r:04x}/{g:04x}/{b:04x}".
}

/// `ResetXtermColorResource(resource)` (`:3430-3445`).
///
/// WT: lookup mapping → if aliasIndex>=0
/// `_renderSettings.RestoreDefaultColorAliasIndex(alias)` → recurse
/// `ResetColorTableEntry(colorTableIndex)`.
// STUB: mapping + RenderSettings.RestoreDefaultColorAliasIndex port chờ.
pub fn reset_xterm_color_resource(_this: &mut TerminalDispatch, _resource: usize) {
    // STUB: alias reset + recursion.
}

/// `AssignColor(item, fgIndex, bgIndex)` (`:3454-3476`) — DECAC.
///
/// WT: match `item`:
///   `NormalText` → set DefaultForeground+DefaultBackground alias.
///   `WindowFrame` → set FrameForeground+FrameBackground alias.
///   `_` → return.
/// + `_renderer->TriggerRedrawAll(item==NormalText, item==WindowFrame)`.
// STUB: RenderSettings.SetColorAliasIndex + trigger redraw port chờ.
pub fn assign_color(
    _this: &mut TerminalDispatch,
    _item: ColorItem,
    _fg_index: VtInt,
    _bg_index: VtInt,
) {
    // STUB: switch ColorItem::NormalText/WindowFrame → alias set pair.
}
