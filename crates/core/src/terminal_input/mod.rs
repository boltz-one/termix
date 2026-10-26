//! `terminal_input` — port của `windows/src/terminal/input/` (keyboard /
//! mouse / focus / paste / resize). Engine command `TerminalInput.hpp:18-58`
//! dispatches inbound host events xuống conhost adapter.
//!
//! Phase 1 (P01 wave 0): `InputEvent` enum + re-export. Pha P04 owns the
//! encoder dispatch.

pub mod events;

pub use events::InputEvent;
