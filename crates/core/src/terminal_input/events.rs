//! `InputEvent` — port của WT `TerminalInput::InputEventType` / `InputEvent`
//! (`windows/src/terminal/input/` + `terminalInput.hpp:18-58`).
//!
//! Variants cover keyboards (`Key*`, `Char`), mouse (`Mouse*`), `Scroll*`,
//! `Focus`, `Paste`, `Resize` — truyền từ host adapter vào conhost VT
//! encoder phase P04.
//!
//! Rust idioms: snake_case variants, payload-packed; preserve WT
//! semantics (key scan codes / unicode chars / mouse coordinate + button -
//! không collapse subtypes).

/// `InputEvent` — top-level tagged union. Port verbatim WT
/// `TerminalInput::InputEventType` enum evolved with payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputEvent {
    // —— Keyboard — port `terminalInput.hpp:30-46`.
    /// `KeyEvent`: scanned key. WT `_wch`, `_scan`, `_state` (modifier
    /// flags), `_keyDown`. Phase 1 dùng `char` cho unicode codepoint + flag
    /// set to mirror `_state`; P04 sẽ expand sang
    /// `KeyState` bitflags.
    Key {
        ch: char,
        scan_code: u16,
        state: u32,
        key_down: bool,
        repeat_count: u16,
    },
    /// `CharEvent`: typed UTF-16 char. Port `InputEventType::CharEvent`.
    Char {
        ch: char,
        scan_code: u16,
        state: u32,
    },
    /// Raw merged event from interactivity once the active PTY doesn't
    /// distinguish (legacy `INPUT_RECORD` `KEY_EVENT`). Phase 1 keep shape
    /// thin; P04 collapses per WT codepath.
    KeyLegacy {
        ch: char,
        scan_code: u16,
        state: u32,
        key_down: bool,
        repeat_count: u16,
    },

    // —— Mouse — port `:55-78`.
    /// Mouse move — `InputEventType::MouseMoveEvent` (mods + coords).
    MouseMove {
        position: (i32, i32),
        buttons: u32,
        mods: u32,
    },
    /// Mouse button down/up — WT `InputEventType::MouseButtonEvent`.
    MouseButton {
        position: (i32, i32),
        button: MouseButton,
        state: u32,
        mods: u32,
        pressed: bool,
    },
    /// Mouse wheel / horizontal pan — `InputEventType::MouseWheelEvent`.
    MouseWheel {
        position: (i32, i32),
        delta: i32,
        horizontal: bool,
        mods: u32,
    },

    // —— Scroll — port `:80-86`.
    /// Scroll up by N lines (keyboard-driven).
    ScrollUp { lines: u32 },
    /// Scroll down by N lines (keyboard-driven).
    ScrollDown { lines: u32 },
    /// Scroll to top of buffer history (keyboard CTRL+HOME in WT).
    ScrollTop,
    /// Scroll to bottom of buffer history (keyboard CTRL+END in WT).
    ScrollBottom,

    // —— Focus / paste / resize — port `:88-105`.
    /// Focus gained (`InputEventType::FocusEvent`) — "foreground tab gained
    /// focus".
    FocusGained,
    /// Focus lost (`InputEventType::FocusEvent`).
    FocusLost,
    /// Paste buffer content into host <stdin> — `InputEventType::PasteEvent`.
    /// WT splits clipboard-event into coerce-text and paste-text phase; P04
    /// wires encoding/branch logic.
    Paste { text: String },
    /// Window/terminal resize — `InputEventType::ResizeEvent`.
    Resize { rows: u16, cols: u16 },
}

/// `MouseButton` — port của WT mouse button enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    /// X1 mouse "extended" button (Win32 VK_XBUTTON1).
    X1,
    /// X2 mouse "extended" button (Win32 VK_XBUTTON2).
    X2,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 1 sanity: enum surface exhaustive so P04 dispatch match arms
    /// catch unknown variants fast.
    #[test]
    fn input_event_construction_smoke() {
        let _ = InputEvent::Key {
            ch: 'a',
            scan_code: 0x1E,
            state: 0,
            key_down: true,
            repeat_count: 1,
        };
        let _ = InputEvent::Char {
            ch: 'b',
            scan_code: 0,
            state: 0,
        };
        let _ = InputEvent::MouseMove {
            position: (10, 5),
            buttons: 0,
            mods: 0,
        };
        let _ = InputEvent::MouseButton {
            position: (0, 0),
            button: MouseButton::Left,
            state: 0,
            mods: 0,
            pressed: true,
        };
        let _ = InputEvent::MouseWheel {
            position: (0, 0),
            delta: 120,
            horizontal: false,
            mods: 0,
        };
        let _ = InputEvent::ScrollUp { lines: 1 };
        let _ = InputEvent::ScrollDown { lines: 1 };
        let _ = InputEvent::ScrollTop;
        let _ = InputEvent::ScrollBottom;
        let _ = InputEvent::FocusGained;
        let _ = InputEvent::FocusLost;
        let _ = InputEvent::Paste {
            text: String::from("x"),
        };
        let _ = InputEvent::Resize {
            rows: 30,
            cols: 120,
        };
    }
}
