//! `EchoConnection` — port `EchoConnection.cpp` (47 LOC) + `EchoConnection.h` (32).
//!
//! Echo PTY: không có process thực; `write_input` echo ngược ra `TerminalOutput`.
//! Dùng làm test fixture / preview pane không spawn ConPTY.
//!
//! Port verbatim WT:
//! - `State()` luôn return `ConnectionState::Connected` (`EchoConnection.h:23`).
//! - `SessionId()` return `guid{}` (zero UUID) (`EchoConnection.h:21`).
//! - `Start`/`Resize`/`Close`/`Initialize` no-op (no PTY resources).
//! - `WriteInput` echo: `TerminalOutput.raise(buffer)` (`EchoConnection.cpp:14-19`).

use uuid::Uuid;

use crate::connection::settings::ConptySettings;
use crate::connection::state::ConnectionState;
use crate::connection::ITerminalConnection;
use crate::event::{StateChangedEvent, TerminalOutputEvent};

/// `EchoConnection` — port `EchoConnection.h:11-28`.
///
/// Sở hữu duy nhất `terminal_output: TerminalOutputEvent` — không có `state_changed`
/// vì `State()` luôn `Connected` (không bao giờ raise StateChanged). Tuy nhiên trait
/// `ITerminalConnection` yêu cầu getter cho `state_changed` → giữ field unused.
pub struct EchoConnection {
    pub(crate) terminal_output: TerminalOutputEvent,
    #[allow(dead_code)]
    pub(crate) state_changed: StateChangedEvent,
}

impl Default for EchoConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl EchoConnection {
    /// Default ctor — `EchoConnection() noexcept` (`EchoConnection.h:14`).
    pub fn new() -> Self {
        Self {
            terminal_output: TerminalOutputEvent::default(),
            state_changed: StateChangedEvent::default(),
        }
    }
}

impl ITerminalConnection for EchoConnection {
    /// `Start() noexcept` — `EchoConnection.cpp:11` no-op.
    fn start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// `WriteInput(array_view<char16_t>)` — `EchoConnection.cpp:14-19`. Echo back:
    /// UTF-16 → UTF-8 string → `TerminalOutput.raise`.
    ///
    /// WT gốc raise UTF-16 view (`winrt_wstring_to_array_view`), ta dùng channel `String`
    /// (`TerminalOutputEvent = Event<String>`). `String::from_utf16_lossy` cho round-trip
    /// an toàn — echo connection không cần strict UTF-16 (test fixture).
    fn write_input(&mut self, buffer: &[u16]) -> anyhow::Result<()> {
        let s = String::from_utf16_lossy(buffer);
        self.terminal_output.raise(s);
        Ok(())
    }

    /// `Resize(rows, cols) noexcept` — `EchoConnection.cpp:21` no-op.
    fn resize(&mut self, _rows: u32, _cols: u32) -> anyhow::Result<()> {
        Ok(())
    }

    /// `Close() noexcept` — `EchoConnection.cpp:23` no-op.
    fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// `Initialize(ValueSet) const noexcept` — `EchoConnection.h:25` no-op (signature
    /// non-`const` vì trait yêu cầu `&mut self`). Echo không consume settings.
    fn initialize(&mut self, _settings: ConptySettings) -> anyhow::Result<()> {
        Ok(())
    }

    /// `guid SessionId() const noexcept { return {}; }` — `EchoConnection.h:21`.
    /// Trả zero UUID cố định.
    fn session_id(&self) -> Uuid {
        Uuid::nil()
    }

    /// `ConnectionState State() const noexcept { return ConnectionState::Connected; }`
    /// — `EchoConnection.h:23`. Luôn `Connected` (echo không có state machine).
    fn state(&self) -> ConnectionState {
        ConnectionState::Connected
    }

    /// `event TerminalOutput(String)` — `EchoConnection.h:25`.
    fn terminal_output(&self) -> &TerminalOutputEvent {
        &self.terminal_output
    }

    /// `event StateChanged` — `EchoConnection.h:26`. Echo không raise (state constant).
    /// Trait yêu cầu getter — trả reference dummy field.
    fn state_changed(&self) -> &StateChangedEvent {
        &self.state_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_input_echoes_back_ascii() {
        let mut c = EchoConnection::new();
        let rx = c.terminal_output().subscribe();
        let input: Vec<u16> = "hello".encode_utf16().collect();
        c.write_input(&input).unwrap();
        assert_eq!(rx.recv(), Ok("hello".into()));
    }

    #[test]
    fn write_input_echoes_back_emoji() {
        let mut c = EchoConnection::new();
        let rx = c.terminal_output().subscribe();
        let input: Vec<u16> = "hi \u{1F600}".encode_utf16().collect();
        c.write_input(&input).unwrap();
        assert_eq!(rx.recv(), Ok("hi \u{1F600}".into()));
    }

    #[test]
    fn start_is_noop() {
        let mut c = EchoConnection::new();
        assert!(c.start().is_ok());
    }

    #[test]
    fn close_is_noop() {
        let mut c = EchoConnection::new();
        assert!(c.close().is_ok());
    }

    #[test]
    fn initialize_is_noop() {
        let mut c = EchoConnection::new();
        assert!(c.initialize(ConptySettings::default()).is_ok());
    }

    #[test]
    fn state_always_connected() {
        let c = EchoConnection::new();
        assert_eq!(c.state(), ConnectionState::Connected);
    }

    #[test]
    fn session_id_is_nil() {
        let c = EchoConnection::new();
        assert_eq!(c.session_id(), Uuid::nil());
    }
}
