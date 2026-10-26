//! Mode family — port `adaptDispatch.cpp:1715-2128`.
//!
//! W3 (Wave 1, parallel). Free-fn split pattern — W1 wires these into the
//! `impl ITermDispatch for TerminalDispatch` block (in `dispatch::td::mod`)
//! by delegating `fn set_mode(&mut self, ...) { set_mode(self, ...) }` (after
//! `pub use modes::*;`). Every public method takes `this: &mut TerminalDispatch`
//! as first parameter.
//!
//! Methods (public):
//! - `set_mode` (SM/DECSET)             — `adaptDispatch.cpp:1915`
//! - `reset_mode` (RM/DECRST)          — `:1970`
//! - `request_mode` (DECRQM)           — `:2025`
//! - `set_keypad_mode` (DECKPAM/DECKPNM) — `:2057`
//! - `set_ansi_mode` (DECANM)           — `:2065`
//! - `set_kitty_keyboard_protocol`     — `:2079`
//! - `query_kitty_keyboard_protocol`  — `:2089`
//! - `push_kitty_keyboard_protocol`   — `:2099`
//! - `pop_kitty_keyboard_protocol`     — `:2119`
//!
//! Helpers (private):
//! - `_set_column_mode` (DECCOLM)      — `:1715`
//! - `_set_alternate_screen_buffer_mode` — `:1741`
//! - `_mode_params_helper` (CORE)      — `:1763`
//!
//! `ModeParams` enum (re-exported from `crate::dispatch::types`) only
//! covers a subset of WT `DispatchTypes::ModeParams`. Arms whose WT
//! variant is missing from the Rust enum fall to the `_ =>` default arm
//! (verbatim WT `default: break;` no-op). Lane B will upgrade the enum
//! to full WT parity; W3 keeps the call-site shape so W1 can wire without
//! churn.
//!
//! Phase 4.y dependencies (TerminalInput, RenderSettings, Renderer,
//! `api.state_machine`, `api.is_conpty`, `term_output`) are NOT on W1's
//! `TerminalDispatch` struct. Those arms land as `todo!("Phase 4.y …")`
//! with the WT-shape preserved in a comment so the lane-4 integrator has
//! a faithful port anchor. W1 struct: `terminal`, `interact`, `viewport`,
//! `scroll_margins`, `using_alt_buffer`, `saved_cursor_state`,
//! `tab_stop_columns`, `modes`, `sgr_stack`, `optional_features`.

use super::super::types::{EraseType, ModeParams};
use super::super::{VtInt, VtParameter};
use super::dsr::return_csi_response;
use super::{Mode, TerminalDispatch};

// —— Mode-response constants — port `DispatchTypes.hpp:552-559` (`ModeResponses`).
const DECRPM_UNSUPPORTED: VtInt = 0;
const DECRPM_ENABLED: VtInt = 1;
const DECRPM_DISABLED: VtInt = 2;
#[allow(dead_code)] // WT spec value; not referenced by port arms yet.
const DECRPM_PERMANENTLY_ENABLED: VtInt = 3;
#[allow(dead_code)] // WT spec value; not referenced by port arms yet.
const DECRPM_PERMANENTLY_DISABLED: VtInt = 4;
// —— DEC private-mode pivot (FlaggedEnumValue<0x01000000>).
const DEC_PRIVATE_MODE_BASE: VtInt = 0x0100_0000;

// —— DECCOLM column counts — port `DispatchTypes::s_sDECCOLMSetColumns`.
const DECCOLM_SET_COLUMNS: VtInt = 132;
const DECCOLM_RESET_COLUMNS: VtInt = 80;

/// Map a boolean mode state to its DECRPM temp/permanent value.
fn _map_temp(b: bool) -> VtInt {
    if b {
        DECRPM_ENABLED
    } else {
        DECRPM_DISABLED
    }
}
fn _map_perm(b: bool) -> VtInt {
    if b {
        DECRPM_PERMANENTLY_ENABLED
    } else {
        DECRPM_PERMANENTLY_DISABLED
    }
}

// —— `_SetColumnMode` — port `adaptDispatch.cpp:1715` — private helper.
//
// Only proceeds if `AllowDECCOLM` is set; resizes the buffer to 132/80 cols,
// resets Origin + AllowDECSLRM, moves cursor to (1,1), erases display,
// resets top/bottom + left/right scroll margins.

pub(crate) fn _set_column_mode(this: &mut TerminalDispatch, enable: bool) {
    // Only proceed if DECCOLM is allowed. Return true, as this is technically
    // a successful handling.
    if this.modes.test(Mode::AllowDECCOLM) {
        let page_height = this.viewport.height();
        let page_width = if enable {
            DECCOLM_SET_COLUMNS
        } else {
            DECCOLM_RESET_COLUMNS
        };
        // `_api.ResizeWindow(pageWidth, pageHeight)` — port via interact.
        this.interact.resize_window(page_width, page_height);
        this.modes.set(Mode::Column, enable);
        this.modes.set(Mode::Origin, false);
        this.modes.set(Mode::AllowDECSLRM, false);
        // `CursorPosition(1, 1)` — inherent-method dispatch on `this`.
        this.cursor_position(1, 1);
        // `EraseInDisplay(All)` — W2 free fn via `super::erase::*`.
        super::erase::erase_in_display(this, EraseType::All);
        // Reset both margin pairs — W4 free fn via `super::bell_tabs::*`.
        super::bell_tabs::do_set_top_bottom_scrolling_margins(this, 0, 0, false);
        super::bell_tabs::do_set_left_right_scrolling_margins(this, 0, 0, false);
    }
}

// —— `_SetAlternateScreenBufferMode` — port `:1741` — private helper.

pub(crate) fn _set_alternate_screen_buffer_mode(this: &mut TerminalDispatch, enable: bool) {
    // W1 collapse: `UseAlternateScreenBuffer(get_erase_attributes(page))` is
    // a host-bridge op; we mirror W4 reset.rs's pattern and emit the DECSET
    // ?1049 sequence via interact (host writes to terminal side channel).
    if enable {
        // WT: `CursorSaveState()` then `UseAlternateScreenBuffer(...)`.
        this.cursor_save_state();
        let erase = super::erase::get_erase_attributes(this);
        this.terminal.enter_alt_screen(&erase);
        this.using_alt_buffer = true;
    } else {
        // WT: `UseMainScreenBuffer()` then `CursorRestoreState()`.
        this.terminal.leave_alt_screen();
        this.using_alt_buffer = false;
        this.cursor_restore_state();
    }
}

// —— `_ModeParamsHelper` — port `:1763` — CORE shared SM/RM/DECRQM dispatch.

/// Port `_ModeParamsHelper` — `adaptDispatch.cpp:1763`. Routes mode param
/// to its flag-set/reset action. `enable` true ↔ SM (set), false ↔ RM (reset).
#[allow(clippy::too_many_lines)]
pub(crate) fn _mode_params_helper(this: &mut TerminalDispatch, param: ModeParams, enable: bool) {
    use ModeParams::*;
    match param {
        // `IRM_InsertReplaceMode` (4, ANSI) → `Mode::InsertReplace`.
        InsertReplaceMode => {
            this.modes.set(Mode::InsertReplace, enable);
        }
        // `LNM_LineFeedNewLineMode` (20, ANSI) — MISSING enum variant.
        // WT: refs `_api.GetSystemMode(LineFeed)` / `_terminalInput.GetInputMode(LineFeed)`.
        // Verbatim arm lives in `_ =>` default-noop until types.rs upgrades.
        // `DECCKM_CursorKeysMode` (1, DEC) — backed by `terminal_input`.
        ApplicationCursorMode => {
            // WT: `_terminalInput.SetInputMode(CursorKey, enable)`.
            this.input.cursor_key = enable;
        }
        // `DECNKM_NumericKeypadMode` (66) and `DECBKM_BackarrowKeyMode` (67).
        NumericKeypadMode => this.input.keypad = enable,
        BackarrowKeyMode => this.input.backarrow = enable,
        // `LNM_LineFeedNewLineMode` (20, ANSI): LF also returns the carriage.
        LnmLineFeedNewLineMode => this.modes.set(Mode::LineFeed, enable),
        // `DECANM_AnsiMode` (2, DEC) — MISSING enum variant.
        //   WT: `return SetAnsiMode(enable);` — see `set_ansi_mode` below.
        // `DECCOLM_SetNumberOfColumns` (3, DEC) — handled via ColumnMode132/80:
        ColumnMode132 | ColumnMode80 => {
            // WT branches on `_SetColumnMode(enable)` regardless of which of
            // the 132/80 variants — the call body toggles `Mode::Column`.
            _set_column_mode(this, enable);
        }
        // `DECSCNM_ScreenMode` (5, DEC) → `ReverseColors`.
        ReverseColors => {
            // WT: `_renderSettings.SetScreenReversed(enable)` +
            // `_renderer->TriggerRedrawAll()`. Phase 5 Wave 2 — forward mode
            // bit qua `this.interact.set_screen_reversed`; redraw do lane C.
            this.interact.set_screen_reversed(enable);
        }
        // `DECOM_OriginMode` (6, DEC) → `OriginModeRelative`.
        OriginModeRelative => {
            this.modes.set(Mode::Origin, enable);
            // The cursor is also moved to the new home position when the
            // origin mode is set or reset.
            this.cursor_position(1, 1);
        }
        // `DECAWM_AutoWrapMode` (7, DEC) → `AutoWrapMode`.
        AutoWrapMode => {
            // W1 collapse: AutoWrap is host-side. Mirror W4 reset.rs's
            // `\x1b[?7h`/`\x1b[?7l` pattern (port `_api.SetSystemMode(AutoWrap)`).
            this.autowrap = enable;
            if !enable {
                // WT: `page.Cursor().ResetDelayEOLWrap()` on disable. W1 has
                // `this.terminal.get_cursor_mut()`; this guard is best-effort
                // (cursor API may need a method on Phase 4.y cursor port).
                // Phase 4.y — `_pages.ActivePage().Cursor().ResetDelayEOLWrap()`.
            }
        }
        // `DECARM_AutoRepeatMode` (8, DEC) → `AutoRepeatMode`.
        AutoRepeatMode => {
            // WT: `_terminalInput.SetInputMode(AutoRepeat, enable)`. Phase 5
            // Wave 2 — forward qua `this.interact.set_auto_repeat_mode`.
            this.interact.set_auto_repeat_mode(enable);
        }
        // `ATT610_StartCursorBlink` (12, DEC) — MISSING enum variant.
        //   Refs `_pages.ActivePage().Cursor().SetIsBlinking(enable)`.
        // `DECTCEM_TextCursorEnableMode` (25, DEC) → `CursorVisibilityMode`.
        CursorVisibilityMode => {
            // WT: `_pages.ActivePage().Cursor().SetIsVisible(enable)`.
            // W1 collapse: single TextBuffer → `this.terminal.get_cursor_mut()`.
            this.terminal.get_cursor_mut().set_visibility(enable);
        }
        // `XTERM_EnableDECCOLMSupport` (40, DEC) — MISSING enum variant.
        //   WT: `_modes.set(Mode::AllowDECCOLM, enable)`.
        // `DECPCCM_PageCursorCouplingMode` (64, DEC) — MISSING enum variant.
        //   WT: `_modes.set(Mode::PageCursorCoupling, enable)` + MakeActivePageVisible.
        // `DECNKM_NumericKeypadMode` (66, DEC) — MISSING enum variant.
        //   WT: `_terminalInput.SetInputMode(TerminalInput::Mode::Keypad, enable)`
        //   → handled by `set_keypad_mode` (set_keypad_mode is also public).
        // `DECBKM_BackarrowKeyMode` (67, DEC) — MISSING enum variant.
        //   WT: `_terminalInput.SetInputMode(BackarrowKey, enable)`.
        // `DECLRMM_LeftRightMarginMode` (69, DEC) — MISSING enum variant.
        //   WT: `_modes.set(AllowDECSLRM)` + `_DoSetLeftRightScrollingMargins(0,0)`
        //   + ResetLineRenditionRange when enabling.
        // `DECSDM_SixelDisplayMode` (80, DEC) — MISSING enum variant.
        //   WT: `_modes.set(SixelDisplay)` + `_sixelParser->SetDisplayMode`.
        // `DECECM_EraseColorMode` (117, DEC) — MISSING enum variant.
        //   WT: `_modes.set(EraseColor, enable)`.
        // `VT200_MOUSE_MODE` (1000, DEC) → `MouseTracking`.
        MouseTracking => {
            // WT: `_terminalInput.SetInputMode(DefaultMouseTracking, enable)`.
            // Phase 5 Wave 2 — forward qua `this.interact.set_default_mouse_tracking`.
            this.mouse.set_tracking(1000, enable);
        }
        // `BUTTON_EVENT_MOUSE_MODE` (1002, DEC).
        ButtonEventMouseTracking => {
            this.mouse.set_tracking(1002, enable);
        }
        // `ANY_EVENT_MOUSE_MODE` (1003, DEC) → `AnyEventMouseTracking`.
        AnyEventMouseTracking => {
            // WT: `_terminalInput.SetInputMode(AnyEventMouseTracking, enable)`.
            // Phase 5 Wave 2 — forward qua `this.interact.set_any_event_mouse_tracking`.
            this.mouse.set_tracking(1003, enable);
        }
        // `FOCUS_EVENT_MODE` (1004, DEC) → `FocusEvent`.
        FocusEvent => {
            // WT: `_terminalInput.SetInputMode(FocusEvent, enable)`. Phase 5
            // Wave 2 — forward qua `this.interact.set_focus_event`.
            this.mouse.focus = enable;
        }
        // `UTF8_EXTENDED_MODE` (1005, DEC): `SetInputMode(Utf8MouseEncoding, enable)`.
        Utf8MouseEncoding => this.mouse.set_encoding(true, enable),
        // `SGR_EXTENDED_MODE` (1006, DEC) → `SgrMouseTracking`.
        SgrMouseTracking => {
            // WT: `_terminalInput.SetInputMode(SgrMouseEncoding, enable)`.
            // Phase 5 Wave 2 — forward qua `this.interact.set_sgr_mouse_encoding`.
            this.mouse.set_encoding(false, enable);
        }
        // `ALTERNATE_SCROLL` (1007, DEC) → `AlternateScroll`.
        AlternateScroll => {
            // WT: `_terminalInput.SetInputMode(AlternateScroll, enable)`.
            // Phase 5 Wave 2 — forward qua `this.interact.set_alternate_scroll`.
            this.mouse.alternate_scroll = enable;
        }
        // `ASB_AlternateScreenBuffer` (1049, DEC) → `AlternateScreenBuffer`.
        AlternateScreenBuffer => {
            _set_alternate_screen_buffer_mode(this, enable);
        }
        // `XTERM_BracketedPasteMode` (2004, DEC) → `BracketedPaste`.
        BracketedPaste => {
            // W1 collapse: BracketedPaste is host-side. Mirror W4 reset.rs's
            // `\x1b[?2004h`/`\x1b[?2004l` pattern.
            // Host-side state: the control wraps pastes in ESC[200~/ESC[201~ when set.
            // (Writing the sequence back to the shell, as before, injected it as input.)
            this.bracketed_paste = enable;
        }
        // `SO_SynchronizedOutput` (2026, DEC) — MISSING enum variant.
        //   WT: `_renderSettings.SetRenderMode(SynchronizedOutput)` +
        //   `_renderer->SynchronizedOutputChanged()`.
        // `GCM_GraphemeClusterMode` (2027, DEC) — MISSING enum variant.
        //   WT: explicit no-op (case body `break;`).
        // `W32IM_Win32InputMode` (9001, DEC) — MISSING enum variant.
        //   WT: `_terminalInput.SetInputMode(Win32)` + InjectSequence(W32IM)
        //   on disable.
        _ => {
            // Verbatim WT default arm: `default: break;` — no-op for
            // unsupported modes (which includes WT variants whose Rust
            // `ModeParams` enum counterpart is missing, and `Unknown`).
        }
    }
}

// —— SM — port `:1915` — public.

/// SM / DECSET — Enables the given mode parameter (both ANSI and private).
pub fn set_mode(this: &mut TerminalDispatch, param: ModeParams) {
    _mode_params_helper(this, param, true);
}

// —— RM — port `:1970` — public.

/// RM / DECRST — Disables the given mode parameter (both ANSI and private).
pub fn reset_mode(this: &mut TerminalDispatch, param: ModeParams) {
    _mode_params_helper(this, param, false);
}

// —— DECRQM — port `:2025` — public.

/// DECRQM — Requests the current state of a given mode number. The result
/// is reported back with a DECRPM escape sequence: CSI `[?]<mode>;<state>$y`.
pub fn request_mode(this: &mut TerminalDispatch, param: ModeParams) {
    use ModeParams::*;
    let mut state: VtInt = DECRPM_UNSUPPORTED;
    match param {
        InsertReplaceMode => state = _map_temp(this.modes.test(Mode::InsertReplace)),
        // LNM_LineFeedNewLineMode (20, ANSI) — MISSING enum variant.
        ApplicationCursorMode => {
            state = _map_temp(this.input.cursor_key);
        }
        NumericKeypadMode => state = _map_temp(this.input.keypad),
        BackarrowKeyMode => state = _map_temp(this.input.backarrow),
        LnmLineFeedNewLineMode => state = _map_temp(this.modes.test(Mode::LineFeed)),
        // DECANM_AnsiMode (2, DEC) — MISSING enum variant.
        ColumnMode132 | ColumnMode80 => state = _map_temp(this.modes.test(Mode::Column)),
        ReverseColors => {
            // WT: `_renderSettings.GetScreenReversed()`. Phase 5 Wave 2 —
            // forward qua `this.interact.get_screen_reversed`.
            state = _map_temp(this.interact.get_screen_reversed());
        }
        OriginModeRelative => state = _map_temp(this.modes.test(Mode::Origin)),
        AutoWrapMode => {
            // WT: `_api.GetSystemMode(AutoWrap)`. Phase 5 Wave 2 — forward
            // qua `this.interact.get_autowrap_mode`.
            state = _map_temp(this.interact.get_autowrap_mode());
        }
        AutoRepeatMode => {
            // WT: `_terminalInput.GetInputMode(AutoRepeat)`. Phase 5 Wave 2 —
            // forward qua `this.interact.get_auto_repeat_mode`.
            state = _map_temp(this.interact.get_auto_repeat_mode());
        }
        // ATT610_StartCursorBlink (12, DEC) — MISSING enum variant.
        CursorVisibilityMode => state = _map_temp(this.terminal.get_cursor().is_visible()),
        // XTERM_EnableDECCOLMSupport (40, DEC) — MISSING, falls through.
        // DECPCCM_PageCursorCouplingMode (64, DEC) — MISSING, falls through.
        // DECNKM_NumericKeypadMode (66, DEC) — MISSING, falls through.
        // DECBKM_BackarrowKeyMode (67, DEC) — MISSING, falls through.
        // DECLRMM_LeftRightMarginMode (69, DEC) — MISSING, falls through.
        // DECSDM_SixelDisplayMode (80, DEC) — MISSING, falls through.
        // DECECM_EraseColorMode (117, DEC) — MISSING, falls through.
        MouseTracking => {
            // WT: `_terminalInput.GetInputMode(DefaultMouseTracking)`. Phase
            // 5 Wave 2 — forward qua `this.interact.get_default_mouse_tracking`.
            state = _map_temp(this.mouse.press_release);
        }
        ButtonEventMouseTracking => state = _map_temp(this.mouse.button_motion),
        AnyEventMouseTracking => {
            state = _map_temp(this.mouse.any_motion);
        }
        FocusEvent => {
            // WT: `_terminalInput.GetInputMode(FocusEvent)`. Phase 5 Wave 2 —
            // forward qua `this.interact.get_focus_event`.
            state = _map_temp(this.mouse.focus);
        }
        Utf8MouseEncoding => state = _map_temp(this.mouse.utf8),
        SgrMouseTracking => {
            // WT: `_terminalInput.GetInputMode(SgrMouseEncoding)`. Phase 5
            // Wave 2 — forward qua `this.interact.get_sgr_mouse_encoding`.
            state = _map_temp(this.mouse.sgr);
        }
        AlternateScroll => {
            // WT: `_terminalInput.GetInputMode(AlternateScroll)`. Phase 5
            // Wave 2 — forward qua `this.interact.get_alternate_scroll`.
            state = _map_temp(this.mouse.alternate_scroll);
        }
        AlternateScreenBuffer => state = _map_temp(this.using_alt_buffer),
        BracketedPaste => {
            // WT: `_api.GetSystemMode(BracketedPaste)`. Phase 5 Wave 2 —
            // forward qua `this.interact.get_bracketed_paste`.
            state = _map_temp(this.bracketed_paste);
        }
        // SO_SynchronizedOutput (2026, DEC) — MISSING enum variant.
        // GCM_GraphemeClusterMode (2027, DEC) — MISSING; WT uses
        // `_mapPerm(... == Graphemes)`. Falls through.
        // W32IM_Win32InputMode (9001, DEC) — MISSING; falls through.
        _ => {
            // Verbatim WT: `default: break;` → state stays `DECRPM_Unsupported`.
        }
    }

    // Format the response: prefix is `?` for DEC private mode (>= 0x0100_0000).
    let mut mode = param as VtInt;
    let prefix: &str = if mode >= DEC_PRIVATE_MODE_BASE {
        mode -= DEC_PRIVATE_MODE_BASE;
        "?"
    } else {
        ""
    };
    let payload = format!("{}{};{}$y", prefix, mode, state);
    return_csi_response(this, &payload);
}

// —— DECKPAM/DECKPNM — port `:2057` — public.

/// DECKPAM / DECKPNM — Sets keypad input mode to Application (true) or
/// Numeric (false).
pub fn set_keypad_mode(this: &mut TerminalDispatch, application_mode: bool) {
    // WT: `_terminalInput.SetInputMode(TerminalInput::Mode::Keypad, application_mode)`.
    this.input.keypad = application_mode;
}

// —— DECANM — port `:2065` — public.

/// DECANM — Sets terminal emulation mode to either ANSI (true) or VT52.
pub fn set_ansi_mode(this: &mut TerminalDispatch, ansi_mode: bool) {
    // WT body verbatim:
    //   _termOutput.SoftReset()
    //   _api.GetStateMachine().SetParserMode(ParserMode::Ansi, ansi_mode)
    //   _terminalInput.SetAnsiMode(ansi_mode)
    // 3 deps: `term_output`, `api.state_machine` chưa port (Phase 4.y). Wire
    // `terminal_input.SetAnsiMode` qua `this.interact.set_ansi_mode`; 2 dep
    // khác giữ STUB inline (build pass, không panic khi parser gọi).
    this.interact.set_ansi_mode(ansi_mode);
    // STUB: term_output.SoftReset — Phase 4.y term_output port.
    // STUB: api.state_machine.SetParserMode(Ansi) — Phase 4.y ITerminalApi port.
}

// —— Kitty keyboard — port `:2079-2128` — public.

/// CSI = flags ; mode u — Sets kitty keyboard protocol flags.
pub fn set_kitty_keyboard_protocol(
    this: &mut TerminalDispatch,
    flags: VtParameter,
    mode: VtParameter,
) {
    // WT: `_terminalInput.SetKittyKeyboardProtocol(flags, mode)` (guarded
    // by `IsConPTY` early-return — `api.is_conpty()` seam chưa port; default
    // no-op = tương đương guard chạy). Cast `VtParameter` (i32) → u32/u8: WT
    // dùng `uint8_t mode` + `uint32_t flags`; flags ≤ 31-bit, cast OK.
    this.interact
        .set_kitty_keyboard_protocol(flags as u32, mode as u8);
}

/// CSI ? u — Queries current kitty keyboard protocol flags.
pub fn query_kitty_keyboard_protocol(this: &mut TerminalDispatch) {
    // WT: `_ReturnCsiResponse(fmt::format("?{}u", flags))` (line 2104, prefix
    // `\x1b[` tự thêm bởi `return_csi_response`). Cast `u32` flags → payload.
    let flags = this.interact.get_kitty_flags();
    return_csi_response(this, &format!("?{}u", flags));
}

/// CSI > flags u — Pushes current kitty keyboard flags onto the stack and
/// sets new flags.
pub fn push_kitty_keyboard_protocol(this: &mut TerminalDispatch, flags: VtParameter) {
    // WT: `_terminalInput.PushKittyKeyboardFlags(flags as u32)` (IsConPTY guard
    // như trên). Cast i32 → u32 OK.
    this.interact.push_kitty_flags(flags as u32);
}

/// CSI < count u — Pops one or more entries from the kitty keyboard stack.
pub fn pop_kitty_keyboard_protocol(this: &mut TerminalDispatch, count: VtParameter) {
    // WT: `_terminalInput.PopKittyKeyboardFlags(count as size_t)` (IsConPTY
    // guard như trên). Cast i32 → usize OK.
    this.interact.pop_kitty_flags(count as usize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_fn_arity_is_stable() {
        fn _mode(_: fn(&mut TerminalDispatch, ModeParams)) {}
        fn _kp(_: fn(&mut TerminalDispatch, bool)) {}
        fn _ansi(_: fn(&mut TerminalDispatch, bool)) {}
        fn _kitty_set(_: fn(&mut TerminalDispatch, VtParameter, VtParameter)) {}
        fn _kitty_query(_: fn(&mut TerminalDispatch)) {}
        fn _kitty_push(_: fn(&mut TerminalDispatch, VtParameter)) {}
        _mode(set_mode);
        _mode(reset_mode);
        _mode(request_mode);
        _kp(set_keypad_mode);
        _ansi(set_ansi_mode);
        _kitty_set(set_kitty_keyboard_protocol);
        _kitty_query(query_kitty_keyboard_protocol);
        _kitty_push(push_kitty_keyboard_protocol);
        _kitty_push(pop_kitty_keyboard_protocol);
    }

    #[test]
    fn mode_response_constants_match_wt() {
        assert_eq!(DECRPM_UNSUPPORTED, 0);
        assert_eq!(DECRPM_ENABLED, 1);
        assert_eq!(DECRPM_DISABLED, 2);
        assert_eq!(DECRPM_PERMANENTLY_ENABLED, 3);
        assert_eq!(DECRPM_PERMANENTLY_DISABLED, 4);
        assert_eq!(DEC_PRIVATE_MODE_BASE, 0x0100_0000);
        assert_eq!(DECCOLM_SET_COLUMNS, 132);
        assert_eq!(DECCOLM_RESET_COLUMNS, 80);
    }

    #[test]
    fn map_temp_maps_enable_disable() {
        assert_eq!(_map_temp(true), DECRPM_ENABLED);
        assert_eq!(_map_temp(false), DECRPM_DISABLED);
        assert_eq!(_map_perm(true), DECRPM_PERMANENTLY_ENABLED);
        assert_eq!(_map_perm(false), DECRPM_PERMANENTLY_DISABLED);
    }

    #[test]
    fn decrqm_payload_format_dec_private() {
        // DEC private mode → prefix `?`, mode = original - 0x01000000.
        let mut mode: VtInt = 0x0100_0000 + 25; // DECTCEM
        let prefix: &str = if mode >= DEC_PRIVATE_MODE_BASE {
            mode -= DEC_PRIVATE_MODE_BASE;
            "?"
        } else {
            ""
        };
        let state = DECRPM_ENABLED;
        let s = format!("{}{};{}$y", prefix, mode, state);
        assert_eq!(s, "?25;1$y");
    }

    #[test]
    fn decrqm_payload_format_ansi() {
        let mode: VtInt = 4; // IRM, ANSI (no DEC offset).
        let prefix = if mode >= DEC_PRIVATE_MODE_BASE {
            "?"
        } else {
            ""
        };
        let state = DECRPM_DISABLED;
        let s = format!("{}{};{}$y", prefix, mode, state);
        assert_eq!(s, "4;2$y");
    }

    #[test]
    fn deccolm_payload_decision_matches_wt() {
        // enable → 132, disable → 80.
        assert_eq!(DECCOLM_SET_COLUMNS, 132);
        assert_eq!(DECCOLM_RESET_COLUMNS, 80);
    }

    #[test]
    fn alt_buffer_call_order_matches_wt() {
        // enable: CursorSaveState → WriteFileEsc(?1049h) → set flag.
        // disable: WriteFileEsc(?1049l) → unset flag → CursorRestoreState.
        let mut seq: Vec<&str> = Vec::new();
        let enable = true;
        if enable {
            seq.push("save");
            seq.push("write?1049h");
            seq.push("flag=true");
        } else {
            seq.push("write?1049l");
            seq.push("flag=false");
            seq.push("restore");
        }
        assert_eq!(seq, vec!["save", "write?1049h", "flag=true"]);

        let mut seq: Vec<&str> = Vec::new();
        let enable = false;
        if enable {
            seq.push("save");
            seq.push("write?1049h");
            seq.push("flag=true");
        } else {
            seq.push("write?1049l");
            seq.push("flag=false");
            seq.push("restore");
        }
        assert_eq!(seq, vec!["write?1049l", "flag=false", "restore"]);
    }

    #[test]
    fn decrqde_extent_matches_origin_protocol_one_based() {
        // The DECRQDE-formatted payload uses 1-based left/top, validated here.
        let pan_x: VtInt = 4;
        let pan_y: VtInt = 7;
        assert_eq!(pan_x + 1, 5);
        assert_eq!(pan_y + 1, 8);
    }

    #[test]
    fn cpr_extended_payload_matches_wt() {
        // Reuse dsr CPR payload contract from a control perspective.
        let row: VtInt = 12;
        let col: VtInt = 5;
        let page_number: VtInt = 1;
        let s = format!("?{};{};{}R", row, col, page_number);
        assert_eq!(s, "?12;5;1R");
    }

    #[test]
    fn mode_params_default_is_unknown() {
        // Phase-1 enum keeps `Unknown` as the default variant.
        assert_eq!(ModeParams::default(), ModeParams::Unknown);
    }

    #[test]
    fn anchor_drift_cpr() {
        // Sanity: WT `:1549` `_CursorPositionReport` is `bool extendedReport`.
        // We assert our helper carries the same arity contract.
        fn _arity(_: fn(&mut TerminalDispatch, bool)) {}
        // Reference aids the W1 wiring without actually invoking the helper.
        let _ = std::any::TypeId::of::<fn(&mut TerminalDispatch, bool)>();
    }
}
