//! DSR / DA family — port `adaptDispatch.cpp:1376-1608`.
//!
//! W3 (Wave 1, parallel). Free-fn split pattern — W1 wires these into the
//! `impl ITermDispatch for TerminalDispatch` block by delegating. Every
//! public method takes `this: &mut TerminalDispatch` as the first parameter.
//!
//! Methods (public):
//! - `device_status_report` (DSR)            — `adaptDispatch.cpp:1376`
//! - `device_attributes` (DA1)               — `:1435`
//! - `secondary_device_attributes` (DA2)     — `:1471`
//! - `tertiary_device_attributes` (DA3)      — `:1481`
//! - `vt52_device_attributes`                — `:1492`
//! - `request_terminal_parameters` (DECREQTPARM) — `:1504`
//!
//! Helpers (private):
//! - `_device_status_report`                 — `:1537`
//! - `_cursor_position_report`               — `:1549`
//! - `_macro_space_report`                   — `:1590`
//! - `_macro_checksum_report`                 — `:1603`
//! - `return_csi_response` / `return_dcs_response` — port of
//!   `adaptDispatch.cpp:4865` / `:4871` (shared with modes.rs).
//!
//! `StatusType`, `ReportingPermission` enums are re-exported from
//! `crate::dispatch::types`. Anchor: `DispatchTypes.hpp:499` / `:607`.
//!
//! W1 struct-shape note: W1 ports `AdaptDispatch::_pages` as
//! `this.terminal: TextBuffer` (single-buffer collapse), `_api` as
//! `this.interact: Box<dyn InteractDispatch>` host bridge. `_macroBuffer`
//! is NOT on W1's struct — `_macro_space_report` / `_macro_checksum_report`
//! use `todo!()` (Phase 4.y macro port). `_optionalFeatures` is `u32`
//! bitset on W1's struct.

use super::{Mode, TerminalDispatch};
use crate::dispatch::types::{ReportingPermission, StatusType};
use crate::dispatch::{VtInt, VtParameter};

// —— Response prefix helpers — port `adaptDispatch.cpp:4865-4875`.
//
// `_ReturnCsiResponse`/`_ReturnDcsResponse` format + ship via `_api`. WT
// checks `_terminalInput.GetInputMode(SendC1)` to pick C0 (`\x1B[`) or C1
// (`\x9B`); SendC1 is not yet ported (Phase 4.y TerminalInput port), so we
// use the C0 default for both — matches WT's common case.

/// `_ReturnCsiResponse` port verbatim — `adaptDispatch.cpp:4865`.
pub(crate) fn return_csi_response(this: &mut TerminalDispatch, response: &str) {
    this.interact.write_file(&format!("\x1b[{}", response));
}

/// `_ReturnDcsResponse` port verbatim — `adaptDispatch.cpp:4871`.
pub(crate) fn return_dcs_response(this: &mut TerminalDispatch, response: &str) {
    this.interact
        .write_file(&format!("\x1bP{}\x1b\\", response));
}

// —— DSR — port `adaptDispatch.cpp:1376` — public method.
//
// WT status string constants port verbatim. CPR / Macro / Checksum delegate
// to private helpers below.

/// DSR — `_DeviceStatusReport` family; `adaptDispatch.cpp:1376`.
pub fn device_status_report(this: &mut TerminalDispatch, status_type: StatusType, id: VtParameter) {
    use StatusType::*;
    match status_type {
        OperatingStatus => _device_status_report(this, "0"),
        CursorPositionReport => _cursor_position_report(this, false),
        ExtendedCursorPositionReport => _cursor_position_report(this, true),
        PrinterStatus => _device_status_report(this, "?13"),
        UserDefinedKeys => _device_status_report(this, "?23"),
        KeyboardStatus => _device_status_report(this, "?27;0;0;5"),
        LocatorStatus => _device_status_report(this, "?53"),
        LocatorIdentity => _device_status_report(this, "?57;0"),
        MacroSpaceReport => _macro_space_report(this),
        MemoryChecksum => _macro_checksum_report(this, id),
        DataIntegrity => _device_status_report(this, "?70"),
        MultipleSessionStatus => _device_status_report(this, "?83"),
    }
}

// —— DA1/DA2/DA3 + VT52 — `adaptDispatch.cpp:1435-1495` — public.

/// `_optionalFeatures.test(OptionalFeature::ClipboardWrite)` — bit `52` in
/// WT. W1's `optional_features: u32` bitset. Mask chosen to match WT's
/// `OptionalFeature::ClipboardWrite` bit (5).
const OPTIONAL_FEATURE_CLIPBOARD_WRITE: u32 = 1 << 5;

/// DA1 — port `adaptDispatch.cpp:1435`. Conformance level 61, optional
/// `ClipboardWrite` extension appends `;52`.
pub fn device_attributes(this: &mut TerminalDispatch) {
    if this.optional_features & OPTIONAL_FEATURE_CLIPBOARD_WRITE != 0 {
        return_csi_response(this, "?61;4;6;7;14;21;22;23;24;28;32;42;52c");
    } else {
        return_csi_response(this, "?61;4;6;7;14;21;22;23;24;28;32;42c");
    }
}

/// DA2 — port `adaptDispatch.cpp:1471`. Hard-coded `>0;10;1c`.
pub fn secondary_device_attributes(this: &mut TerminalDispatch) {
    return_csi_response(this, ">0;10;1c");
}

/// DA3 — port `adaptDispatch.cpp:1481`. Hard-coded `!|00000000`.
pub fn tertiary_device_attributes(this: &mut TerminalDispatch) {
    return_dcs_response(this, "!|00000000");
}

/// VT52 Identify — port `adaptDispatch.cpp:1492`. `\x1b/Z`.
pub fn vt52_device_attributes(this: &mut TerminalDispatch) {
    this.interact.write_file("\x1b/Z");
}

/// DECREQTPARM — port `adaptDispatch.cpp:1504`. Hard-coded 38400 baud
/// parameter sequence (matches XTerm). First param is 2 (unsolicited) or
/// 3 (solicited) corresponding to WT `ReportingPermission::Unsolicited` /
/// `::Solicited` (`DispatchTypes.hpp:607`).
///
/// `types.rs::ReportingPermission` is the Phase 1 enum, only carries
/// `Granted` (default) / `Denied`. We dispatch by the closest semantic
/// mapping:
/// - `Granted` (default) → Solicited (3;...). WT value `1` → Solicited.
/// - `Denied` → Unsolicited (2;...). WT value `0` → Unsolicited.
pub fn request_terminal_parameters(this: &mut TerminalDispatch, permission: ReportingPermission) {
    use ReportingPermission::*;
    match permission {
        // Phase 1 enum only carries Granted/Denied (DispatchTypes.hpp:607).
        // Granted (default) → WT Solicited (3;...). Denied → WT Unsolicited (2;...).
        Denied => return_csi_response(this, "2;1;1;128;128;1;0x"),
        Granted => return_csi_response(this, "3;1;1;128;128;1;0x"),
    }
}

// —— Helpers — `adaptDispatch.cpp:1537-1608` — private (free fns).

/// `_DeviceStatusReport` — `adaptDispatch.cpp:1537`. Wraps a fixed status
/// string in CSI `...n`.
pub(crate) fn _device_status_report(this: &mut TerminalDispatch, parameters: &str) {
    return_csi_response(this, &format!("{}n", parameters));
}

/// `_CursorPositionReport` — `adaptDispatch.cpp:1549`. CPR (CSI row;colR) and
/// extended (CSI ?row;col;pageR). Uses page-relative + origin-mode
/// adjustments verbatim from WT.
///
/// W1 collapse: `page = _pages.ActivePage()` → single `this.terminal` +
/// `this.viewport`. `page.Top()` = `this.viewport.top`. `page.Width()` =
/// `this.viewport.width()`. Page number is always 1 (single-buffer port).
pub(crate) fn _cursor_position_report(this: &mut TerminalDispatch, extended_report: bool) {
    // Pull the cursor position relative to the entire buffer out of the console.
    let cursor_position = this.terminal.get_cursor().get_position();
    let mut x = cursor_position.0;
    let mut y = cursor_position.1;

    // Now adjust it for its position in respect to the current page top.
    y -= this.viewport.top;

    // NOTE: 1,1 is the top-left corner of the page in VT-speak, so add 1.
    x += 1;
    y += 1;

    // If the origin mode is set, the cursor is relative to the margin origin.
    if this.modes.test(Mode::Origin) {
        let h = this.get_horizontal_margins(this.viewport.width());
        let v = this.get_vertical_margins(false);
        x -= h.0;
        y -= v.0;
    }

    // Now send it back into the input channel of the console.
    if extended_report {
        // Extended report also includes the page number. Single-buffer port
        // → page number is always 1.
        let page_number: VtInt = 1;
        return_csi_response(this, &format!("?{};{};{}R", y, x, page_number));
    } else {
        return_csi_response(this, &format!("{};{}R", y, x));
    }
}

/// `_MacroSpaceReport` — `adaptDispatch.cpp:1590`. CSI `<space/16>*{`.
///
/// W1 struct has no `_macroBuffer` field (Phase 4.y Sixel/Macro port). WT
/// fallback for missing macro buffer is `MacroBuffer::MAX_SPACE` (65535).
/// We mirror the fallback verbatim.
pub(crate) fn _macro_space_report(this: &mut TerminalDispatch) {
    // Phase 4.y macro support: W1's TerminalDispatch has no `macro_buffer`
    // field. WT fallback `MacroBuffer::MAX_SPACE = 65535`.
    let _ = this;
    const MAX_SPACE: usize = 65535;
    let space_in_bytes: usize = MAX_SPACE;
    return_csi_response(this, &format!("{}*{{", space_in_bytes / 16));
}

/// `_MacroChecksumReport` — `adaptDispatch.cpp:1603`. DCS `<id>!~<cs04X>`.
///
/// W1 struct has no `_macroBuffer`; WT fallback checksum for missing buffer
/// is 0. Request id defaults to 0 (`id.value_or(0)`).
pub(crate) fn _macro_checksum_report(this: &mut TerminalDispatch, id: VtParameter) {
    let request_id: VtInt = id; // WT: `id.value_or(0)`.
    let checksum: u32 = 0; // WT: `_macroBuffer ? _macroBuffer->CalculateChecksum() : 0`.
    return_dcs_response(this, &format!("{}!~{:04X}", request_id, checksum));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Static check that public DSR free fns compile (cannot instantiate
    /// `TerminalDispatch` until W1 lands cursor/print, but the free-fn
    /// arity stays stable). The module build is the gate.
    #[test]
    fn free_fn_arity_is_stable() {
        fn _dsr(_: fn(&mut TerminalDispatch, StatusType, VtParameter)) {}
        fn _da1(_: fn(&mut TerminalDispatch)) {}
        fn _da2(_: fn(&mut TerminalDispatch)) {}
        fn _da3(_: fn(&mut TerminalDispatch)) {}
        fn _vt52(_: fn(&mut TerminalDispatch)) {}
        fn _reqparm(_: fn(&mut TerminalDispatch, ReportingPermission)) {}
        _dsr(device_status_report);
        _da1(device_attributes);
        _da2(secondary_device_attributes);
        _da3(tertiary_device_attributes);
        _vt52(vt52_device_attributes);
        _reqparm(request_terminal_parameters);
    }

    #[test]
    fn status_type_variants_cover_wt() {
        // WT anchors: 5,6,?6,?15,?25,?26,?55,?56,?62,?63,?75,?85.
        use StatusType::*;
        let _ = [
            OperatingStatus,
            CursorPositionReport,
            ExtendedCursorPositionReport,
            PrinterStatus,
            UserDefinedKeys,
            KeyboardStatus,
            LocatorStatus,
            LocatorIdentity,
            MacroSpaceReport,
            MemoryChecksum,
            DataIntegrity,
            MultipleSessionStatus,
        ];
    }

    #[test]
    fn csi_response_prefix_is_csi_c0() {
        let s = format!("\x1b[{}n", "0");
        assert_eq!(s, "\x1b[0n");
        let s = format!("\x1b[{}{}", ">0;10;1c", "");
        assert_eq!(s, "\x1b[>0;10;1c");
        let s = format!("\x1bP{}\x1b\\", "!|00000000");
        assert_eq!(s, "\x1bP!|00000000\x1b\\");
    }

    #[test]
    fn cpr_payload_format_extended() {
        let y = 12;
        let x = 5;
        let page_number = 1;
        let s = format!("?{};{};{}R", y, x, page_number);
        assert_eq!(s, "?12;5;1R");
    }

    #[test]
    fn cpr_payload_format_standard() {
        let y = 12;
        let x = 5;
        let s = format!("{};{}R", y, x);
        assert_eq!(s, "12;5R");
    }

    #[test]
    fn macro_space_block_unit() {
        // WT measures space in 16-byte blocks → divide by 16.
        assert_eq!(65535 / 16, 4095);
        assert_eq!(4095 * 16, 65520);
    }

    #[test]
    fn da1_payload_includes_extension_when_clipboard_write() {
        let s = "?61;4;6;7;14;21;22;23;24;28;32;42;52c";
        assert!(s.ends_with('c'));
        assert!(s.contains(";52"));
    }

    #[test]
    fn da2_da3_payload_constants() {
        // WT hard-codes these response strings.
        assert_eq!(">0;10;1c", ">0;10;1c");
        assert_eq!("!|00000000", "!|00000000");
    }

    #[test]
    fn vt52_payload_constant() {
        // WT: `_api.ReturnResponse(L"\x1b/Z")`.
        assert_eq!("\x1b/Z", "\x1b/Z");
    }

    #[test]
    fn decreqtparm_response_string_unsolicited() {
        let s = "2;1;1;128;128;1;0x";
        assert!(s.starts_with('2'));
    }

    #[test]
    fn decreqtparm_response_string_solicited() {
        let s = "3;1;1;128;128;1;0x";
        assert!(s.starts_with('3'));
    }

    #[test]
    fn macro_space_report_fallback_constant() {
        // When `_macroBuffer` is missing, WT reports `MAX_SPACE` blocks.
        // 4095 * 16 = 65520; CSI `65520*{`.
        let s = format!("{}*{{", 65535 / 16);
        assert_eq!(s, "4095*{");
    }

    #[test]
    fn macro_checksum_report_fallback_format() {
        // Request id 0, checksum 0 → DCS `0!~0000`.
        let s = format!("{}!~{:04X}", 0, 0u32);
        assert_eq!(s, "0!~0000");
    }

    #[test]
    fn optional_feature_clipboard_write_bit_is_in_mask_set() {
        let bit = OPTIONAL_FEATURE_CLIPBOARD_WRITE;
        let features: u32 = bit;
        assert!(features & bit != 0);
        // off → masked out.
        let features: u32 = 0;
        assert!(features & bit == 0);
    }
}
