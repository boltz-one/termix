//! Event abstraction — thay WinRT `event<T>` tokens bằng `crossbeam-channel`.
//!
//! Port `ITerminalConnection.idl:23-26` (`event TerminalOutput(String); event StateChanged;`)
//! và `ConptyConnection.cpp` raise pattern. `subscribe()` trả `Receiver<T>` (multi-subscriber,
//! `parking_lot::Mutex<Vec<Sender>>` bên trong). `raise()` fan-out cho tất cả subscriber.
//!
//! KHÔNG clone ra `Arc` — `ConptyConnection` sở hữu các event này trực tiếp (`Arc<ConptyConnection>`
//! ở phase 2 cho output thread truy cập `terminal_output` qua reference).

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;

/// Channel buffer size — match WT default (BlockingQueue ~1000 ticks). 256 đủ cho
/// VT output chịu đựng 60fps render mà không drop packet ở phase 2.
const CHANNEL_BOUND: usize = 256;

/// `event <T>` — generic port thay thế WinRT `event_token`. `T: Send + Clone + 'static`.
pub struct Event<T: Send + Clone + 'static> {
    subs: Mutex<Vec<Sender<T>>>,
}

impl<T: Send + Clone + 'static> Default for Event<T> {
    fn default() -> Self {
        Self {
            subs: Mutex::new(Vec::new()),
        }
    }
}

impl<T: Send + Clone + 'static> Event<T> {
    /// Đăng ký subscriber — trả `Receiver<T>`. Multi-subscriber OK.
    pub fn subscribe(&self) -> Receiver<T> {
        let (tx, rx) = bounded(CHANNEL_BOUND);
        self.subs.lock().push(tx);
        rx
    }

    /// Clone 1 `Sender<T>` cho output thread — KHÔNG thêm subscriber mới (đã
    /// subscribe qua `subscribe()` ở `ControlCore::initialize` trước `start()`).
    /// Multi-producer `Sender: Clone` — thread `send` best-effort, queue đầy
    /// return err (caller bỏ qua, parity WT `retain` cũng drop full subscriber).
    ///
    /// Panic nếu chưa có subscriber — `ControlCore::initialize` phải chạy trước
    /// `ConptyConnection::start()` (verify order `control.rs:60` → `:90`).
    pub fn borrow_sender(&self) -> Sender<T> {
        let subs = self.subs.lock();
        subs.first()
            .expect("Event::borrow_sender: chưa có subscriber — ControlCore::initialize phải chạy trước start()")
            .clone()
    }

    /// Raise event cho tất cả subscriber. Subscriber channel đầy/drop sẽ bị remove.
    /// Port `ConptyConnection.cpp:779` `_TerminalOutputHandlers(...)`.
    /// Like `borrow_sender` but `None` when nobody subscribed yet.
    pub fn try_borrow_sender(&self) -> Option<Sender<T>> {
        self.subs.lock().first().cloned()
    }

    pub fn raise(&self, payload: T) {
        let mut subs = self.subs.lock();
        subs.retain(|tx| tx.send(payload.clone()).is_ok());
    }
}

/// `event TerminalOutput(String)` — port `ITerminalConnection.idl:23`.
pub type TerminalOutputEvent = Event<String>;

/// `event StateChanged` — port `ITerminalConnection.idl:25`. Payload = `ConnectionState`.
pub type StateChangedEvent = Event<crate::connection::ConnectionState>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_roundtrip() {
        let ev: TerminalOutputEvent = Event::default();
        let rx = ev.subscribe();
        ev.raise("hi".into());
        assert_eq!(rx.recv(), Ok("hi".into()));
    }

    #[test]
    fn multi_subscriber_fanout() {
        let ev: TerminalOutputEvent = Event::default();
        let a = ev.subscribe();
        let b = ev.subscribe();
        ev.raise("x".into());
        assert_eq!(a.recv(), Ok("x".into()));
        assert_eq!(b.recv(), Ok("x".into()));
    }

    #[test]
    fn dropped_subscriber_pruned() {
        let ev: TerminalOutputEvent = Event::default();
        let _rx = ev.subscribe(); // dropped ngay
        let keep = ev.subscribe();
        ev.raise("y".into());
        // Subscriber đầu đã drop, không crash. Sub次年 raise vẫn nhận được.
        assert_eq!(keep.recv(), Ok("y".into()));
    }
}
