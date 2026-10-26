//! `ConnectionState` enum — port `BaseTerminalConnection.h`. 6 variants với
//! thứ tự chuyển tiếp mang ngữ nghĩa:
//!
//! `NotConnected` → `Connecting` → `Connected` → `Closing` → `Closed`
//!                                                     ↓
//!                                            `Failed` (từ bất kỳ state không-Closed)
//!
//! `_transition_to_state` (xem `mod.rs`) enforce order này — guard chống "flapping"
//! giữa 3 termination entry points (`user Close`, `spawn failure`, `pipe-broken`).
//! Tham khảo `ConptyConnection.cpp:537-551`.

use std::fmt;

/// 6 variants — port `BaseTerminalConnection.h::ConnectionState`. Copy vì so sánh
/// trong `_transition_to_state` và raise qua `StateChangedEvent` payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConnectionState {
    /// Số initial state — chưa `start()`.
    NotConnected = 0,
    /// `start()` đã gọi, đang `ConptyCreatePseudoConsole` + `CreateProcessW`.
    Connecting = 1,
    /// PTY + child process đã chạy, output thread đang `ReadFile` loop.
    Connected = 2,
    /// `close()` đã gọi — huỷ output thread + `ClosePseudoConsole`.
    Closing = 3,
    /// Output thread terminate, child exit, `ClosePseudoConsole` xong.
    Closed = 4,
    /// Spawn/read failure — không thể phục hồi, `_LastConPtyClientDisconnected`.
    Failed = 5,
}

impl ConnectionState {
    /// Trả `true` nếu state là terminal (không thể transition ra ngoài).
    /// `_transition_to_state` dùng để reject invalid transition.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Failed)
    }
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => f.write_str("NotConnected"),
            Self::Connecting => f.write_str("Connecting"),
            Self::Connected => f.write_str("Connected"),
            Self::Closing => f.write_str("Closing"),
            Self::Closed => f.write_str("Closed"),
            Self::Failed => f.write_str("Failed"),
        }
    }
}
