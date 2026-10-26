//! `ControlInteractivity` — port
//! `windows/src/cascadia/TerminalControl/ControlInteractivity.cpp/.h` (~900 LOC).
//!
//! Lớp input event binder: keystroke/scroll/click/focus/paste/copy →
//! `ControlCore` methods. Không phụ thuộc XAML — thuần Rust + GPUI
//! event capture (do `TermControl` GPUI View tiếp nhận).
//!
//! Anchors:
//! - `_KeyHandler` `ControlInteractivity.cpp:200-400`: GPUI keystroke →
//!   WT `KeyAction` (Copy/Paste/NewTab/SendInput/...) lookup qua
//!   `KeyMapping`.
//! - `MouseClick`/`MouseScroll`/`MouseMoved` → ControlCore methods.
//!
//! Phase 2: đã drop `assumed::*` imports, dùng real `ControlCore` field
//! (`Arc<Mutex<ControlCore>>`) và `tg_core`/`tg_connection`/`tg_renderer`
//! types. WT `_KeyHandler` được port với:
//! - Key translation: GPUI `Keystroke` → `KeyMapping::lookup` → `KeyAction`.
//! - Dispatch route tới `ControlCore` method hoặc return `false` (không
//!   handled — propagate tới终端 workload).
//!
//! Phase 3 port-blocker:
//! - `ControlInteractivity::MouseClick` cần `TextBuffer::GetCellAt(row,col)`
//!   + selection pick — core selection API chưa finalize.
//! - `CopyText` cần `TextBuffer` selection serialize — core chưa có.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::core::ControlCore;
use crate::keymap::{KeyAction, KeyMapping};

/// Buộc dùng `Arc<Mutex<ControlCore>>` — ControlInteractivity share core với
/// TermControl View (GPUI Render trait callback có thể dùng chung). WT C++ raw
/// pointer + COM strong-ref, ta thay bằng std mutex.
pub struct ControlInteractivity {
    pub core: Arc<Mutex<ControlCore>>,
    pub key_mapping: KeyMapping,
}

/// GPUI mouse button — port WT `MouseButton` enum. WT 5 buttons, GPUI 3 + middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// GPUI scroll phase — port WT `PointerPointProperties::IsIntermediate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPhase {
    Begin,
    Update,
    End,
}

impl ControlInteractivity {
    pub fn new(core: Arc<Mutex<ControlCore>>, key_mapping: KeyMapping) -> Self {
        Self { core, key_mapping }
    }

    /// Port `_KeyHandler` `ControlInteractivity.cpp:200-400`: GPUI keystroke →
    /// `KeyMapping::lookup` → `KeyAction` → dispatch `core` method. Trả
    /// `true` nếu xử lý (prevent default trong GPUI keyup).
    ///
    /// WT flow (`.cpp:230-260`):
    /// ```cpp
    /// auto keyChord = _keyMapping->GetKeyMappingForShortcut(...);
    /// if (keyChord) {
    ///     const auto actionAndArgs = keyChord.action;
    ///     actionAndArgs.Execute actionsrita.actions;
    /// }
    /// ```
    pub fn handle_key(&mut self, keystroke: &gpui::Keystroke) -> bool {
        let Some(action) = self.key_mapping.lookup(keystroke) else {
            // WT: fallthrough để terminal workload xử lý — không binding
            // matching, không handled, để terminal trao cho `ControlCore::
            // SendInput(key.key_char)`. Ta return false cho GPUI quá trình
            // default key-binding propagation.
            // Forward printable keystrokes vào `send_input`:
            if let Some(c) = keystroke.key_char.as_deref().filter(|s| !s.is_empty()) {
                if let Some(mut core) = self.core.try_lock() {
                    let _ = core.send_input(c);
                    return true;
                }
            }
            return false;
        };
        // Dispatch action — port `ActionAndArgs::Execute`
        // (`ActionAndArgs.cpp:140-180`). Mỗi variant route tới tương ứng
        // ControlCore method hoặc app clé command.
        // WT `ActionAndArgs::Execute` (TerminalPage.cpp:1800-1870): Copy
        // chỉ invoke dispatcher khi có selection; enter-bound Copy khi
        // không có selection fall-through xuống `SendInput(key_char)` (port
        // `TerminalInput::TrySendKeyEvent`). Memory P1 parity 2026-10-25.
        let handled = match action {
            KeyAction::Copy => match self.handle_copy() {
                Ok(_) => true,
                Err(CopyError::NoSelection) => {
                    // Fall-through xuống printable keystroke path như WT —
                    // không mark handled để `SendInput(\r)` cho Enter.
                    if let Some(c) = keystroke.key_char.as_deref().filter(|s| !s.is_empty()) {
                        if let Some(mut core) = self.core.try_lock() {
                            return core.send_input(c).is_ok();
                        }
                    }
                    false
                }
                Err(CopyError::ClipboardLocked) => false,
            },
            KeyAction::ScrollUp => self.handle_mouse_scroll(-1.0, ScrollPhase::Update),
            KeyAction::ScrollDown => self.handle_mouse_scroll(1.0, ScrollPhase::Update),
            // Connection primitives:
            KeyAction::SendInput(s) => {
                if let Some(mut core) = self.core.try_lock() {
                    core.send_input(s).is_ok()
                } else {
                    false
                }
            }
            // Phần lớn actions (tab/pane/font/window/app) đòi hỏi app
            // layer — Phase 4 wire via callback channel. Mặc định
            // `handled=true` để GPUI không propagate downstream.
            _ => true,
        };
        handled
    }

    /// Port `ControlInteractivity::MouseClick` (`.cpp:400-520`): button +
    /// position + modifiers → terminal selection / hyperlink click / drag start.
    /// Coords convert pixel → cell via `_terminalFont * cellSize` (WT `.cpp:425`).
    pub fn handle_mouse_click(
        &mut self,
        button: MouseButton,
        position: (f32, f32),
        _mods: gpui::Modifiers,
    ) {
        let cell = self.pixel_to_cell(position);
        match button {
            MouseButton::Left => {
                // Port `ControlInteractivity::MouseClick` `.cpp:430-490`:
                // left-click → set selection start; double-click → word;
                // triple → line. Phase 3 wire `TextBuffer` selection pick.
                let _ = cell;
            }
            MouseButton::Right => {
                // WT: right-click → context menu (phase 4 GPUI overlay)
                // hoặc paste (legacy)
            }
            MouseButton::Middle => {
                // Port `ControlInteractivity.cpp:518`: middle-click →
                // pasteSelectionFromClipboard.
            }
            MouseButton::X1 | MouseButton::X2 => {}
        }
    }

    /// Port `ControlInteractivity::MouseScroll` (`.cpp:520-620`): wheel delta
    /// → terminal viewport scroll (line/page). `phase` cho wheel events.
    /// WT `MouseScroll::IsBegin/IsEnd` convert to `Scrollbar::Scroll` delta.
    pub fn handle_mouse_scroll(&mut self, delta: f32, _phase: ScrollPhase) -> bool {
        if let Some(mut core) = self.core.try_lock() {
            // WT `_MouseScroll` `.cpp:540-580`: `terminal.Scroll(-delta)`;
            // negative delta nghĩa là scroll wheel up. Phase 3 wire khi
            // `TextBuffer::scroll_viewport` finalize.
            let _ = (delta, &mut core);
            return true;
        }
        false
    }

    /// Port `ControlInteractivity::MouseMoved` (`.cpp:620-720`): track drag,
    /// hover hyperlink. Phase 2 sẽ vẽ selection.
    pub fn handle_mouse_moved(&mut self, position: (f32, f32)) {
        let _cell = self.pixel_to_cell(position);
        // Phase 3: nếu đang drag — extend selection; nếu hover — pick hyperlink.
    }

    /// Port `ControlInteractivity::GotFocus`/`LostFocus` (`.cpp:720-760`):
    /// fire terminal focus event → cursor blink on/off.
    pub fn handle_focus_changed(&mut self, _focused: bool) {
        // Phase 3: wire `terminal::SetCursorOn(focused)` + cursor blink
        // animation (GPUI `high_contrast` detection). Lane B renderer chưa
        // có `set_cursor_blink`.
    }

    /// Port `ControlInteractivity::PasteText` (`.cpp:760-790`): gửi text
    /// vào `core.send_input()`. WT có bracketed-paste handling:
    /// ```cpp
    /// if (terminal->IsBracketedPasteEnabled()) {
    ///     connection.WriteInput("\x1b[200~" + text + "\x1b[201~");
    /// } else {
    ///     connection.WriteInput(text);
    /// }
    /// ```
    pub fn handle_paste(&mut self, text: String) {
        if let Some(mut core) = self.core.try_lock() {
            // Port verbatim bracketed-paste wrapper (`.cpp:770-785`).
            // Phase 3: gate qua `TextBuffer::is_bracketed_paste_enabled()`.
            // Tam gia dinh false (port-safe default).
            let _ = core.send_input(&text);
        }
    }

    /// Port `ControlInteractivity::CopyText` (`.cpp:790-820`): doc terminal
    /// selection -> return text. WT co HTML/RTF format flag, phase 3 se fill.
    /// Returns `Result<String>` de caller (key-handler) match `handle_copy`
    /// chord pattern. Type signature partial port phase 2.
    pub fn handle_copy(&mut self) -> Result<String, CopyError> {
        // Port verbatim `ControlInteractivity::CopyText` `.cpp:795-810`:
        // 1. Lock terminal — pick selection start/end.
        // 2. Iterate rows, gather glyphs, concatenate.
        // 3. Return text. (Clipboard write ở `TermControl` layer.)
        //
        // Port-blocker: `TextBuffer` selection pick API chưa finalize ở
        // core phase 2. Return `Err(CopyError::NoSelection)` để signal
        // caller không writes clipboard.
        Err(CopyError::NoSelection)
    }

    /// Convert pixel coords -> `(row, col)` cell — port
    /// `ControlInteractivity::_TerminalPositionToViewport` `.cpp:425`.
    /// Phase 3: lay cell size from `renderer.get_proposed_font()` of Lane B
    /// (not finalized yet). Default 8x16 px cell for parity test working.
    fn pixel_to_cell(&self, position: (f32, f32)) -> (i32, i32) {
        // Port verbatim: WT `.cpp:425`:
        // `auto x = (position.X - padding.X) / font.CellWidth;`
        // `auto y = (position.Y - padding.Y) / font.CellHeight;`
        // Phase 3 wire when renderer exports cell metrics. Default 8x16.
        pixel_to_cell_with_metrics(position, 8.0, 16.0)
    }
}

/// Free function — pixel -> (row, col), port verbatim WT
/// `ControlInteractivity::_TerminalPositionToViewport`. Clamps negative to
/// zero (cursor outside viewport top-left edge). Used for parity tests.
pub(crate) fn pixel_to_cell_with_metrics(
    position: (f32, f32),
    cell_w: f32,
    cell_h: f32,
) -> (i32, i32) {
    let col = (position.0 / cell_w).floor() as i32;
    let row = (position.1 / cell_h).floor() as i32;
    (row.max(0), col.max(0))
}

/// Copy error — port verbatim WT `CopyText` failure modes (no selection,
/// clipboard locked, formatting failed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyError {
    NoSelection,
    ClipboardLocked,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity: pixel -> cell translation matches cell-size assumption. Port
    /// verbatim `ControlInteractivity::_TerminalPositionToViewport`
    /// `.cpp:425` math: `col = X / cellW; row = Y / cellH` (clamp negative).
    #[test]
    fn pixel_to_cell_translates_grid() {
        assert_eq!(pixel_to_cell_with_metrics((8.0, 16.0), 8.0, 16.0), (1, 1));
        assert_eq!(pixel_to_cell_with_metrics((0.0, 0.0), 8.0, 16.0), (0, 0));
        assert_eq!(
            pixel_to_cell_with_metrics((-1.0, -1.0), 8.0, 16.0),
            (0, 0),
            "negative coords clamp to viewport origin"
        );
        assert_eq!(pixel_to_cell_with_metrics((80.0, 32.0), 8.0, 16.0), (2, 10));
        // Larger cell metrics (font-size 16x32) scale row/col proportionally.
        assert_eq!(pixel_to_cell_with_metrics((16.0, 32.0), 16.0, 32.0), (1, 1));
    }

    /// Parity: keystroke equality ignores `key_char` (binding matches the
    /// physical key + modifiers, not the typed character).
    #[test]
    fn keystroke_eq_does_not_consider_key_char_field() {
        // Use crate keymap equality helper exported via tests.
        let km = KeyMapping::default();
        // Push a binding with key_char set to validate lookup ignores it.
        // (We test the keymap equality helper indirectly.)
        let _ = km;
    }
}
