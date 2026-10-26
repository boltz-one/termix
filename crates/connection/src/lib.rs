//! Port `connection` — ConPTY host plumbing native Rust (phase 1 skeleton).
//!
//! Port `windows/src/cascadia/TerminalConnection/{ITerminalConnection,ConptyConnection}.idl|cpp`.
//! Phase 1: trait + struct + lifecycle signatures đầy đủ (`todo!()` body cho FFI).
//! Phase 2: fill ConPTY FFI + output thread loop.
//!
//! Xem `workflows/development-rules.md` Voice/Verification Gate.

pub mod connection;
pub mod conpty;
pub mod event;

pub use connection::{
    ConnectionState, ConptyConnection, ConptySettings, EchoConnection, ITerminalConnection,
    PseudoConsoleFlags,
};
pub use event::{StateChangedEvent, TerminalOutputEvent};
