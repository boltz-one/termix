//! `downstream` — concrete Lane B API aliases. Single source of truth cho
//! `app` import — `control`, `connection`, `renderer`,
//! `settings`.
//!
//! Phase 2 note: `connection` đang biên dịch fail do windows-rs 0.61 API
//! drift (`CancelSynchronousIo` không tồn tại, `CreateProcessW` lpcommandline
//! thành `Option<PWSTR>`). Fix nằm ngoài scope (file `connection/src/...`).
//! Code dưới giữ path thực (`pub use connection::...`) — khi Lane B fix,
//! `app` biên dịch sạch. Port-blocker reported below.
//!
//! Quy ước downstream hiện tại:
//! - TermControl → control::TermControl       (Lane B — OK)
//! - ConptyConnection / ConptySettings / ITerminalConnection / PseudoConsoleFlags
//!   → connection::... (Lane B — BLOCKED)
//! - RenderEngine → renderer::RenderEngine     (Lane B — OK)
//! - Profile → settings::Profile         (Lane A — OK)

#[cfg(not(feature = "stub-downstream"))]
pub use connection::{
    ConptyConnection, ConptySettings, ITerminalConnection, PseudoConsoleFlags,
};
#[cfg(not(feature = "stub-downstream"))]
pub use control::TermControl;
#[cfg(not(feature = "stub-downstream"))]
pub use renderer::RenderEngine;
#[cfg(not(feature = "stub-downstream"))]
pub use settings::Profile;

/// Stub path — chỉ enable khi muốn biên dịch `app` standalone không cần
/// Lane B downstream. Theo brief Phase 3: thay `todo!()` bằng impl thực của
/// stub trait/struct — demo path tự chạy được (start Ok, write_input no-op,
/// close Ok; state NotConnected; terminal_output/state_changed return empty
/// event không fire).
#[cfg(feature = "stub-downstream")]
pub mod stub {
    use std::collections::HashMap;
    use uuid::Uuid;

    /// `ITerminalConnection` — port mỏng của
    /// `Microsoft::Terminal::TerminalConnection::ITerminalConnection`.
    /// Stub impl: default methods no-op/sane-default — không `todo!()` panic.
    pub trait ITerminalConnection: Send {
        /// `Start` — mở session. Stub: Ok (caller quản lý state transition).
        fn start(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        /// `WriteInput` — gửi user input. Stub: no-op Ok (không có conpty thật).
        fn write_input(&mut self, _b: &[u16]) -> anyhow::Result<()> {
            Ok(())
        }
        /// `Resize` — đổi rows/cols. Stub: no-op Ok.
        fn resize(&mut self, _r: u32, _c: u32) -> anyhow::Result<()> {
            Ok(())
        }
        /// `Close` — đóng session. Stub: Ok (idempotent).
        fn close(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        /// `Initialize` — apply settings trước `start`. Stub: Ok.
        fn initialize(&mut self, _s: ConptySettings) -> anyhow::Result<()> {
            Ok(())
        }
        /// `SessionId` — GUID phiên. Stub default: `Uuid::nil()`.
        fn session_id(&self) -> Uuid {
            Uuid::nil()
        }
        /// `State` — current `.Stub default: `NotConnected`.
        fn state(&self) -> ConnectionState {
            ConnectionState::NotConnected
        }
        /// `TerminalOutput` — event stream cho render. Stub default: empty.
        fn terminal_output(&self) -> &TerminalOutputEvent {
            static EMPTY: TerminalOutputEvent = TerminalOutputEvent;
            &EMPTY
        }
        /// `StateChanged` — event stream cho state machine. Stub default: empty.
        fn state_changed(&self) -> &StateChangedEvent {
            static EMPTY: StateChangedEvent = StateChangedEvent;
            &EMPTY
        }
    }

    /// `ConptyConnection` — port mỏng của
    /// `Microsoft::Terminal::TerminalConnection::ConptyConnection`. Stub giữ
    /// settings + session_id để `session_id`/`state` trả giá trị thực sau khi
    /// `initialize`/`start`.
    pub struct ConptyConnection {
        settings: Option<ConptySettings>,
        state: ConnectionState,
    }

    impl ConptyConnection {
        /// `ConptyConnection::CreateSettings` ctor (`.cpp:~140`). Stub: empty.
        pub fn new() -> Self {
            Self {
                settings: None,
                state: ConnectionState::NotConnected,
            }
        }
    }

    impl Default for ConptyConnection {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ITerminalConnection for ConptyConnection {
        /// Lưu settings để `session_id` (lấy từ settings) trả giá trị thực.
        fn initialize(&mut self, s: ConptySettings) -> anyhow::Result<()> {
            self.settings = Some(s);
            Ok(())
        }
        /// `Start` — state transition `NotConnected → Connecting`. Stub không
        /// spawn process. Trả Ok để demo path tiếp tục.
        fn start(&mut self) -> anyhow::Result<()> {
            self.state = ConnectionState::Connecting;
            Ok(())
        }
        fn write_input(&mut self, _b: &[u16]) -> anyhow::Result<()> {
            Ok(())
        }
        fn resize(&mut self, _r: u32, _c: u32) -> anyhow::Result<()> {
            Ok(())
        }
        /// `Close` — state transition `→ Closed`. Idempotent.
        fn close(&mut self) -> anyhow::Result<()> {
            self.state = ConnectionState::Closed;
            Ok(())
        }
        /// `SessionId` — từ `settings.session_id` (v4 fresh) nếu đã initialize,
        /// `Uuid::nil()` nếu chưa. Demo path cần giá trị stable cho trace.
        fn session_id(&self) -> Uuid {
            self.settings
                .as_ref()
                .map(|s| s.session_id)
                .unwrap_or_default()
        }
        fn state(&self) -> ConnectionState {
            self.state
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct ConptySettings {
        pub commandline: String,
        pub starting_directory: String,
        pub starting_title: Option<String>,
        pub initial_rows: u32,
        pub initial_cols: u32,
        pub environment: HashMap<String, String>,
        pub flags: PseudoConsoleFlags,
        pub session_id: Uuid,
        pub profile_guid: Option<Uuid>,
        pub inherit_cursor: bool,
        pub text_measurement: u8,
        pub ambiguous_is_wide: bool,
    }
    pub use settings::Profile;

    bitflags::bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct PseudoConsoleFlags: u32 {
            const INHERIT_CURSOR    = 0x1;
            const GLYPH_WIDTH_WIDE  = 0x4;
            const AMBIGUOUS_IS_WIDE = 0x8;
        }
    }

    /// `TerminalOutputEvent` — port mỏng của
    /// `ConptyConnection::TerminalOutput` event. Stub: empty struct (không fire).
    #[derive(Default)]
    pub struct TerminalOutputEvent;
    /// `StateChangedEvent` — port mỏng của
    /// `ConptyConnection::StateChanged` event. Stub: empty struct (không fire).
    #[derive(Default)]
    pub struct StateChangedEvent;

    /// `ConnectionState` — `ITerminalConnection.h:ConnectionState`. Stub enum.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ConnectionState {
        #[default]
        NotConnected,
        Connecting,
        Connected,
        Closing,
        Closed,
        Failed,
    }

    /// `TermControl` — stub port của `control::TermControl`. Phase 2: render
    /// empty div (Lane B real thay bằng control đang finalize).
    pub struct TermControl;
    impl gpui::Render for TermControl {
        fn render(
            &mut self,
            _w: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    /// `RenderEngine` — stub port của `renderer::RenderEngine`. Phase 2 chỉ
    /// giữ trait rỗng (Lane B real đang finalize grid render).
    pub trait RenderEngine {}
}

#[cfg(feature = "stub-downstream")]
pub use stub::{
    ConptyConnection, ConptySettings, ITerminalConnection, Profile, PseudoConsoleFlags,
    RenderEngine, TermControl,
};

#[cfg(all(test, feature = "stub-downstream"))]
mod tests {
    use super::stub::{ConnectionState, ConptyConnection, ITerminalConnection};

    /// Stub `ConptyConnection::default().start()` returns Ok (brief verify cond).
    #[test]
    fn stub_conpty_connection_default_start_ok() {
        let mut c = ConptyConnection::default();
        assert!(c.start().is_ok());
        assert_eq!(c.state(), ConnectionState::Connecting);
        assert!(c.close().is_ok());
        assert_eq!(c.state(), ConnectionState::Closed);
    }

    /// Stub `write_input` no-op (không panic, không fire event).
    #[test]
    fn stub_write_input_noop() {
        let mut c = ConptyConnection::default();
        let buf: &[u16] = &[b'a' as u16];
        assert!(c.write_input(buf).is_ok());
        // Resize cũng no-op.
        assert!(c.resize(30, 120).is_ok());
    }

    /// Stub `session_id` trả settings.session_id sau initialize; `Uuid::nil()`
    /// trước.
    #[test]
    fn stub_session_id_after_initialize() {
        use super::stub::ConptySettings;
        use uuid::Uuid;
        let mut c = ConptyConnection::default();
        assert_eq!(c.session_id(), Uuid::nil());
        let mut s = ConptySettings::default();
        let id = Uuid::new_v4();
        s.session_id = id;
        assert!(c.initialize(s).is_ok());
        assert_eq!(c.session_id(), id);
    }
}
