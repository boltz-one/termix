//! `td/dcs.rs` — Phase 5 W4 DCS / rect-area / Sixel / Macro / Presentation /
//! Sound / Optional-features (20 free-fn methods).
//!
//! Port verbatim `windows/src/terminal/adapter/adaptDispatch.cpp`:
//! - Rectangular attrs family (`:1755-1960`):
//!   - `ChangeAttributesRectangularArea` (DECCARA)  — `:1755`
//!   - `ReverseAttributesRectangularArea` (DECRARA) — `:1790`
//!   - `CopyRectangularArea` (DECCRA)               — `:1810`
//!   - `FillRectangularArea` (DECFRA)               — `:1850`
//!   - `EraseRectangularArea` (DECERA)              — `:1875`
//!   - `SelectiveEraseRectangularArea` (DECSERA)    — `:1900`
//!   - `SelectAttributeChangeExtent` (DECSACE)     — `:1920`
//!   - `RequestChecksumRectangularArea` (DECRQCRA) — `:1940`
//! - Sixel / DRCS / Macro (`:2010-2080`):
//!   - `DefineSixelImage`                — `:2010`
//!   - `DownloadDRCS`                     — `:2030`
//!   - `RequestUserPreferenceCharset`     — `:4027`
//!   - `AssignUserPreferenceCharset`      — `:4040`
//!   - `DefineMacro`                      — `:4074`
//!   - `InvokeMacro`                      — `:4098`
//! - Presentation / terminal-state reports (`:2090-2150` / `:4123-4260`):
//!   - `RequestTerminalStateReport`      — `:4123`
//!   - `RestoreTerminalState`             — `:4142`
//!   - `RequestSetting`                   — `:4256`
//!   - `RequestPresentationStateReport`   — `:4520`
//!   - `RestorePresentationState`         — `:4542`
//! - Sound / optional features (`:2160-2200`):
//!   - `PlaySounds`                       — `:4889`
//!   - `SetOptionalFeatures`              — `:4909`
//!
//! Free-fn shape: `pub fn <name>(this: &mut TerminalDispatch, …<trait args>)`.
//! Main wires `dcs::<name>` vào `impl ITermDispatch for TerminalDispatch`
//! trong `td/mod.rs`. KHÔNG tạo `impl ITermDispatch` ở đây (Rust E0119 — chỉ
//! 1 impl block trong `td/mod.rs`).
//!
//! Phase 5 port ở mức "signature + minimal correct no-op hoặc minimal logic":
//! - `*_rectangular_area` family: WT apply trên TextBuffer rect-applied API
//!   (corner-applied drawing). TextBuffer chưa có method corner-applied →
//!   `// TextBuffer rect-applied API chờ P08 buffer-parity`.
//! - `request_checksum_rectangular_area`, `request_*_state_report`,
//!   `request_setting`, `request_user_preference_charset`: WT gửi response về
//!   host qua `_api.GetStateMachine(...) / _termOutput.*`. Phase 5 thứ cấp →
//!   `// response string port chờ host`.
//! - `define_sixel_image`, `download_drcs`, `define_macro`: WT lưu vào
//!   `_sixelParser` / `_macroBuffer` field — struct chưa có 2 field này (W1
//!   brief bỏ). Gọi `handler(...)` để drain string payload (giữ parser-side
//!   contract), `// macro/sixel buffer field chờ P-P04/P-DCS-B`.
//! - `restore_*_state`: `// state model chờ`.
//! - `play_sounds`: WT parse params + emit sound qua `_api.PlaySounds` →
//!   `interact` bridge. Chưa có `play_sounds` trong `InteractDispatch` →
//!   `// sound port chờ host audio`.
//! - `set_optional_features`: CÓ thể port thật — `this.optional_features =
//!   features` (field đã có `mod.rs:268`).
//!
//! Không dùng `todo!()` — build phải pass.

use super::TerminalDispatch;
use crate::dispatch::types::{ChangeExtent, CharsetSize, PresentationReportFormat, ReportFormat};
use crate::dispatch::{StringHandler, VtInt, VtParameter, VtParameters};

// ——————————————————————————————————————————————————————————————————————————
// Rectangular area family (DECCARA/DECRARA/DECCRA/DECFRA/DECERA/DECSERA/
// DECSACE/DECRQCRA)
// ——————————————————————————————————————————————————————————————————————————

/// DECCARA — `ChangeAttributesRectangularArea` (`adaptDispatch.cpp:1755`).
///
/// WT: parse `attrs` (SGR-like list), for each cell in rect `{left, top, right,
/// bottom}` apply attr changes per `SelectAttributeChangeExtent` mode. Needs
/// TextBuffer rect-applied attr API (corner-applied drawing).
///
/// TextBuffer rect-applied API chờ P08 buffer-parity.
pub fn change_attributes_rectangular_area(
    this: &mut TerminalDispatch,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
    attrs: VtParameters,
) {
    let _ = (this, top, left, bottom, right, attrs);
}

/// DECRARA — `ReverseAttributesRectangularArea` (`adaptDispatch.cpp:1790`).
///
/// WT: same shape as DECCARA but reverses (toggles) the listed attributes in
/// rect. Needs TextBuffer rect-applied attr API.
///
/// TextBuffer rect-applied API chờ P08 buffer-parity.
pub fn reverse_attributes_rectangular_area(
    this: &mut TerminalDispatch,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
    attrs: VtParameters,
) {
    let _ = (this, top, left, bottom, right, attrs);
}

/// DECCRA — `CopyRectangularArea` (`adaptDispatch.cpp:1810`).
///
/// WT: copy rect `{top, left, bottom, right}` from `page` to
/// `{dst_top, dst_left}` on `dst_page`. Needs TextBuffer rect copy API with
/// page index (single-page port phase 1 — page/dst_page ignored).
///
/// TextBuffer rect-copy API chờ P08 buffer-parity.
#[allow(clippy::too_many_arguments)]
pub fn copy_rectangular_area(
    this: &mut TerminalDispatch,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
    page: VtInt,
    dst_top: VtInt,
    dst_left: VtInt,
    dst_page: VtInt,
) {
    let _ = (
        this, top, left, bottom, right, page, dst_top, dst_left, dst_page,
    );
}

/// DECFRA — `FillRectangularArea` (`adaptDispatch.cpp:1850`).
///
/// WT: fill rect with character `ch` (validated through `_termOutput.CharBelongsToCharset`
/// etc.), using current attributes. `ch: VtParameter` is the codepoint
/// (validated at parser level). Needs TextBuffer rect-applied fill (corner-
/// applied). Sibling `erase::fill_rect` is a row-by-row helper but lives on
/// the dispatch layer — reusing it here would require a full-row `Viewport`
/// produced from `ch` + attrs; defer to P08 parity to match WT semantics.
///
/// TextBuffer rect-applied fill API chờ P08 buffer-parity.
pub fn fill_rectangular_area(
    this: &mut TerminalDispatch,
    ch: VtParameter,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
) {
    let _ = (this, ch, top, left, bottom, right);
}

/// DECERA — `EraseRectangularArea` (`adaptDispatch.cpp:1875`).
///
/// WT: erase rect (clear cells to whitespace + current erase attrs, leave
/// outside untouched). Needs TextBuffer rect-applied erase.
///
/// TextBuffer rect-applied erase API chờ P08 buffer-parity.
pub fn erase_rectangular_area(
    this: &mut TerminalDispatch,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
) {
    let _ = (this, top, left, bottom, right);
}

/// DECSERA — `SelectiveEraseRectangularArea` (`adaptDispatch.cpp:1900`).
///
/// WT: like DECERA but skips protected cells. Sibling `erase::selective_erase_rect`
/// is a `pub(crate)` helper taking a `&Viewport` — could be reused here with a
/// rect conversion, but the resulting semantics differ slightly from WT's
/// corner-applied `SelectiveEraseRect` (which goes via buffer API). Defer to
/// P08 parity for fidelity.
///
/// TextBuffer rect-applied selective-erase API chờ P08 buffer-parity.
pub fn selective_erase_rectangular_area(
    this: &mut TerminalDispatch,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
) {
    let _ = (this, top, left, bottom, right);
}

/// DECSACE — `SelectAttributeChangeExtent` (`adaptDispatch.cpp:1920`).
///
/// WT: sets `_modes.set(Mode::RectangularChangeExtent, extent == ChangeExtent::All)`.
/// `ChangeExtent::All` (rectangular, whole rect) vs `ChangeExtent::Stream`
/// (stream, line-by-line from cursor). Stored in `Mode::RectangularChangeExtent`
/// — consumed by DECCARA / DECRARA.
///
/// Port real (no TextBuffer dependency — only mode bit).
pub fn select_attribute_change_extent(this: &mut TerminalDispatch, extent: ChangeExtent) {
    use super::Mode;
    // WT: `_modes.set(Mode::RectangularChangeExtent, extent == ChangeExtent::All)`.
    this.modes
        .set(Mode::RectangularChangeExtent, extent == ChangeExtent::All);
}

/// DECRQCRA — `RequestChecksumRectangularArea` (`adaptDispatch.cpp:1940`).
///
/// WT: compute checksum over rect, send `DCS id;page; top,left,bottom,right,
/// checksum ST` response string to host. Needs host response bridge.
///
/// response string port chờ host.
#[allow(clippy::too_many_arguments)]
pub fn request_checksum_rectangular_area(
    this: &mut TerminalDispatch,
    id: VtInt,
    page: VtInt,
    top: VtInt,
    left: VtInt,
    bottom: VtInt,
    right: VtInt,
) {
    let _ = (this, id, page, top, left, bottom, right);
}

// ——————————————————————————————————————————————————————————————————————————
// Sixel / DRCS / Macro
// ——————————————————————————————————————————————————————————————————————————

/// DEC Sixel — `DefineSixelImage` (`adaptDispatch.cpp:2010`).
///
/// WT: hand off `handler` to `_sixelParser` (which feeds the byte stream into
/// the image engine). The `StringHandler` is a one-shot callback that the
/// engine invokes per decoded chunk. Struct field `_sixelParser` chưa có
/// (W1 brief drop) — drain `handler` with empty payload to satisfy parser-side
/// contract (no-op image) and keep the host-state machine moving.
///
/// macro/sixel buffer field chờ P-P04/P-DCS-B.
pub fn define_sixel_image(
    this: &mut TerminalDispatch,
    macro_parameter: VtInt,
    background_color: VtInt,
    private_color_registers: bool,
    handler: StringHandler,
) {
    let _ = (
        this,
        macro_parameter,
        background_color,
        private_color_registers,
    );
    // Drain the string handler with an empty payload to satisfy the parser-
    // side contract (engine invokes `handler("")` to consume the chunk).
    // Without this, the state machine's DCS parser hangs waiting for the
    // image engine to accept the buffer.
    handler("");
}

/// DECDLD — `DownloadDRCS` (`adaptDispatch.cpp:2030`).
///
/// WT: hand off `handler` to the DRCS font engine, which decodes the byte
/// stream into a charset cell matrix. Field not in struct (W1 brief drop) —
/// drain `handler` empty, same as `define_sixel_image`.
///
/// macro/sixel buffer field chờ P-P04/P-DCS-B.
pub fn download_drcs(
    this: &mut TerminalDispatch,
    font_number: VtInt,
    start_char: VtInt,
    cleanup: bool,
    cell_matrix: VtInt,
    font_sets: VtInt,
    handler: StringHandler,
) {
    let _ = (
        this,
        font_number,
        start_char,
        cleanup,
        cell_matrix,
        font_sets,
    );
    handler("");
}

/// DECRQUPSS — `RequestUserPreferenceCharset` (`adaptDispatch.cpp:4027`).
///
/// WT: respond with `DCS 0;1;0;u ST` (default) or assigned charset. Response
/// string sent to host via `_termOutput.RequestUserPreferenceCharset()`.
///
/// response string port chờ host.
pub fn request_user_preference_charset(this: &mut TerminalDispatch) {
    let _ = this;
}

/// DECAUPSS — `AssignUserPreferenceCharset` (`adaptDispatch.cpp:4040`).
///
/// WT: set `_termOutput.AssignUserPreferenceCharset(charset_id, size == 96)`,
/// where `charset_id` extracted from prior DCS string (not exposed in trait
/// signature — only `charset_size` reaches us). Without the charset id we
/// cannot apply the assignment. Defer to host port.
///
/// charset id state chờ host.
pub fn assign_user_preference_charset(this: &mut TerminalDispatch, charset_size: CharsetSize) {
    let _ = (this, charset_size);
}

/// DECDMAC — `DefineMacro` (`adaptDispatch.cpp:4074`).
///
/// WT: store `handler` body into `_macroBuffer` slot `macro_id`. Field not in
/// struct. Drain `handler` empty to satisfy parser contract.
///
/// macro/sixel buffer field chờ P-P04/P-DCS-B.
pub fn define_macro(
    this: &mut TerminalDispatch,
    macro_id: VtInt,
    private_macro: bool,
    handler: StringHandler,
) {
    let _ = (this, macro_id, private_macro);
    handler("");
}

/// DECMD — `InvokeMacro` (`adaptDispatch.cpp:4098`).
///
/// WT: if `_macroBuffer`, register a callback on state-machine `OnCsiComplete`
/// that replays the stored macro. Without `_macroBuffer` field there is
/// nothing to invoke.
///
/// macro/sixel buffer field chờ P-P04/P-DCS-B.
pub fn invoke_macro(this: &mut TerminalDispatch, macro_id: VtInt) {
    let _ = (this, macro_id);
}

// ——————————————————————————————————————————————————————————————————————————
// Presentation / terminal-state reports
// ——————————————————————————————————————————————————————————————————————————

/// DECRQTSR — `RequestTerminalStateReport` (`adaptDispatch.cpp:4123`).
///
/// WT: switch on `format` — only `ColorTableReport` branch does work
/// (`_ReportColorTable(formatOption)`), default breaks. Response strings sent
/// to host via state machine. Phase 5 stub.
///
/// response string port chờ host.
pub fn request_terminal_state_report(
    this: &mut TerminalDispatch,
    format: ReportFormat,
    format_option: VtParameter,
) {
    let _ = (this, format, format_option);
}

/// DECRSTS — `RestoreTerminalState` (`adaptDispatch.cpp:4142`).
///
/// WT: receive state payload via `handler`, deserialize + apply to terminal
/// state model. Field/state model chưa có.
///
/// state model chờ.
pub fn restore_terminal_state(this: &mut TerminalDispatch, format: ReportFormat) {
    let _ = (this, format);
}

/// DECRQSS — `RequestSetting` (`adaptDispatch.cpp:4256`).
///
/// WT: receive the setting selector via `handler` (a DCS string), look it up,
/// emit response string. Without state model Deferred.
///
/// response string port chờ host.
pub fn request_setting(this: &mut TerminalDispatch) {
    let _ = this;
}

/// DECRQPSR — `RequestPresentationStateReport` (`adaptDispatch.cpp:4520`).
///
/// WT: switch on `format` (CursorInformation / TabulationInformation) → emit
/// response. Response strings require host bridge.
///
/// response string port chờ host.
pub fn request_presentation_state_report(
    this: &mut TerminalDispatch,
    format: PresentationReportFormat,
) {
    let _ = (this, format);
}

/// DECRSPS — `RestorePresentationState` (`adaptDispatch.cpp:4542`).
///
/// WT: receive payload via `handler`, restore cursor / tab stops. State model
/// chưa có.
///
/// state model chờ.
pub fn restore_presentation_state(this: &mut TerminalDispatch, format: PresentationReportFormat) {
    let _ = (this, format);
}

// ——————————————————————————————————————————————————————————————————————————
// Sound / optional features
// ——————————————————————————————————————————————————————————————————————————

/// DECPS — `PlaySounds` (`adaptDispatch.cpp:4889`).
///
/// WT: parse `parameters` (each param-group of 4 → frequency, duration,
/// volume, note), then call `_api.PlaySounds(...)` to emit. `InteractDispatch`
/// không có `play_sounds` method — host audio bridge chưa có.
///
/// sound port chờ host audio.
pub fn play_sounds(this: &mut TerminalDispatch, parameters: VtParameters) {
    let _ = (this, parameters);
}

/// DECSACE-optional-features — `SetOptionalFeatures` (`adaptDispatch.cpp:4909`).
///
/// WT body (verbatim):
/// ```cpp
/// void AdaptDispatch::SetOptionalFeatures(const til::enumset<OptionalFeature> features) noexcept
/// {
///     _optionalFeatures = features;
/// }
/// ```
///
/// Port real — field `this.optional_features: u32` đã có (`td/mod.rs:268`).
/// WT `til::enumset<OptionalFeature>` is a bitmask; we store the raw `u32`
/// verbatim (no enumset crate in `core`).
pub fn set_optional_features(this: &mut TerminalDispatch, features: u32) {
    // Port verbatim `adaptDispatch.cpp:4909-4911`:
    //   _optionalFeatures = features;
    this.optional_features = features;
}
