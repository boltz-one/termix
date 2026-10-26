//! `TermControl` — port `windows/src/cascadia/TerminalControl/TermControl.cpp/.h`
//! (~4800 LOC). WT XAML `UserControl` -> Rust GPUI `Render`-backed `Entity`.
//!
//! Anchors:
//! - `TermControl::TermControl` ctor `.cpp:180-300`: owns ControlInteractivity.
//! - `_SizeChanged` `.cpp:300-400`: rows*cols calculation -> core.SizeChanged.
//! - `_KeyHandler` `.cpp:400-500`: XAML KeyDown -> ControlInteractivity.HandleKey
//!   (ta dung GPUI `cx.on_key_*`).
//! - `_KeyDown`/`_KeyUp`/`_PointerPressed`/`_PointerMoved`/`_PointerReleased`
//!   (`.cpp:400-720`): XAML pointer events.
//!
//! Phase 2: da drop `assumed::*` imports + wire real `core::TextBuffer`,
//! `connection::ITerminalConnection`, `renderer::RenderEngine` tu
//! Lane A/B. `render` tra `gpui::div()` placeholder; Lane B
//! `RenderEngine::paint_frame_for_engine` chua tra `gpui::IntoElement`
//! (port-blocker phase 3: renderer can not produce GPUI elements yet).

use std::sync::Arc;

use parking_lot::Mutex;

use crate::core::ControlCore;
use crate::interactivity::ControlInteractivity;
use crate::keymap::KeyMapping;
use connection::ITerminalConnection;
use core::TextBuffer;
use settings::Profile;

/// Default cell metrics (phase 3 / port-blocker: renderer finalize).
const DEFAULT_CELL_W: f32 = 8.0;
const DEFAULT_CELL_H: f32 = 16.0;

/// GPUI View thay XAML UserControl. Owns `ControlInteractivity` (von owns
/// `Arc<Mutex<ControlCore>>`). Khi spawn bang `cx.new(|cx| TermControl::new(...))`
/// ta co `Entity<TermControl>`.
pub struct TermControl {
    pub interactivity: ControlInteractivity,
    pub focus_handle: gpui::FocusHandle,
    /// Output pump task (drains `TerminalOutputEvent` channel). Held to
    /// prevent task drop. Port _ConnectionOutputHandler `TermControl::
    /// _Initialize` `.cpp:283` wire connection.TerminalOutput -> handler.
    pub output_task: Option<gpui::Task<()>>,
    /// Set when the connection reached `Closed`/`Failed` (shell exited).
    closed: Arc<std::sync::atomic::AtomicBool>,
    /// Set when the shell exited with an error (pane stays open, see infobar).
    failed: Arc<std::sync::atomic::AtomicBool>,
    /// Task watching the connection state; held so it is not dropped.
    state_task: Option<gpui::Task<()>>,
    /// Scroll position, selection, cursor blink and find box.
    pub(crate) view: crate::view_state::ViewState,
    /// Geometry of the last frame (mouse position → cell).
    pub(crate) metrics: crate::term_view::SharedMetrics,
    /// Cell where the left button went down, while a drag may start a selection.
    pub(crate) drag_anchor: Option<crate::selection::Cell>,
    /// Cursor blink timer; held so it is not dropped.
    blink_task: Option<gpui::Task<()>>,
    /// Button held while mouse reporting is active (for motion reports).
    pub(crate) held_button: Option<crate::mouse_report::Button>,
    /// VT mouse report state (`_mouseInputState`: wheel accumulation, dedupe).
    pub(crate) reporter: crate::mouse_report::Reporter,
    /// Fractional rows accumulated by smooth wheel scrolling.
    pub(crate) scroll_accum: f32,
    /// Focus state seen by the last frame (DECSET 1004 focus reports).
    pub(crate) last_focused: bool,
    /// IME composition (preedit) shown at the cursor until it is committed.
    pub(crate) preedit: Option<String>,
}

impl TermControl {
    /// Port `TermControl::TermControl(IConnection, settings)` `.cpp:180-300`:
    /// tao `ControlCore`, `ControlInteractivity`, wire events. Khac WT -
    /// tie vao `cx.new(|cx| Self::new(...))` chu khong phai XAML `UserControl`.
    pub fn new(
        connection: Box<dyn ITerminalConnection>,
        renderer: Box<dyn renderer::RenderEngine>,
        terminal: TextBuffer,
        profile: Profile,
        key_mapping: KeyMapping,
        cx: &mut gpui::App,
    ) -> Self {
        let shape = match profile.default_appearance.cursor_shape {
            settings::appearance_enums::CursorShape::Bar => core::CursorShape::VerticalBar,
            settings::appearance_enums::CursorShape::Vintage => core::CursorShape::Legacy,
            settings::appearance_enums::CursorShape::Underscore => core::CursorShape::Underscore,
            settings::appearance_enums::CursorShape::DoubleUnderscore => core::CursorShape::DoubleUnderscore,
            settings::appearance_enums::CursorShape::FilledBox => core::CursorShape::FullBox,
            settings::appearance_enums::CursorShape::EmptyBox => core::CursorShape::EmptyBox,
        };
        let mut core = ControlCore::new(terminal, connection, renderer, profile);
        // Port `ControlCore::Initialize` `ControlCore.cpp:340-400`: subscribe
        // before `Start` so first output not dropped.
        core.initialize();
        // Profile `cursorShape` (programs change it later with DECSCUSR).
        core.engine.dispatch.terminal.get_cursor_mut().set_style(shape);
        let core = Arc::new(Mutex::new(core));
        let interactivity = ControlInteractivity::new(core, key_mapping);
        Self {
            interactivity,
            focus_handle: cx.focus_handle(),
            output_task: None,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            failed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            state_task: None,
            view: Default::default(),
            metrics: Default::default(),
            drag_anchor: None,
            blink_task: None,
            held_button: None,
            reporter: Default::default(),
            scroll_accum: 0.0,
            last_focused: false,
            preedit: None,
        }
    }

    /// Port `TermControl::Initialize` `.cpp:280-320`: start connection, spawn
    /// output pump, hold task. Caller pattern (Wire inside `cx.new` closure):
    /// ```ignore
    /// let mut view = cx.new(|cx| TermControl::new(...));
    /// view.update(cx, |t, cx| t.start(cx));
    /// ```
    ///
    /// Phase 3 (unblock): starts connection, then spawn output-thread pump
    /// via `Self::spawn_output_thread` (holds returned `gpui::Task<()>` in
    /// `self.output_task`). Pump lives until channel closes (connection
    /// dropped) OR `TermControl` entity dropped (WeakEntity upgrade fails).
    ///
    /// **Bypass port-blocker**: `ControlCore::spawn_output_thread` remains
    /// `todo!()` — pump spawn MOVES to `TermControl` layer because `cx.spawn`
    /// requires `Context<T>` (entity handle) để call `cx.notify()`. ControlCore
    /// không có entity handle (không GPUI-aware). Pump chỉ phụ thuộc
    /// `crossbeam_channel::Receiver<String>: Send` (đã thoả) — không cần
    /// `Arc<Mutex<ControlCore>>: Send` (vốn đòi `RenderEngine: Send`).
    pub fn start(&mut self, cx: &mut gpui::Context<Self>) -> anyhow::Result<()> {
        // Start the connection BEFORE pump so ConPTY output thread can run.
        self.interactivity.core.lock().start()?;
        // Spawn output-thread pump; hold returned Task để keep alive.
        self.output_task = self.spawn_output_thread(cx);
        self.state_task = self.spawn_state_watcher(cx);
        self.blink_task = Some(Self::spawn_blink(cx));
        Ok(())
    }

    /// Toggle the cursor blink phase (`TermControl::_CursorTimer`).
    fn spawn_blink(cx: &mut gpui::Context<Self>) -> gpui::Task<()> {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(std::time::Duration::from_millis(530)).await;
            let alive = this.update(cx, |this, cx| {
                let blinking = this.interactivity.core.lock().engine.dispatch.terminal.get_cursor().is_blinking();
                if blinking {
                    this.view.cursor_on = !this.view.cursor_on;
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        })
    }

    /// True when the shell exited with a non-zero code.
    pub fn has_failed(&self) -> bool {
        self.failed.load(std::sync::atomic::Ordering::Acquire)
    }

    /// "[press Ctrl+D to close]": ask the page to close this pane.
    pub(crate) fn request_close(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::Release);
    }

    /// True once the shell has exited (connection `Closed`/`Failed`).
    pub fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Watch `StateChangedEvent`: on a terminal state set `closed` and wake
    /// the UI so the page can close this pane (`closeOnExit`).
    fn spawn_state_watcher(&mut self, cx: &mut gpui::Context<Self>) -> Option<gpui::Task<()>> {
        use futures::channel::mpsc;
        use futures::StreamExt;

        let state_rx = self.interactivity.core.lock().take_state_rx()?;
        let (tx, mut rx) = mpsc::unbounded::<()>();
        let closed = self.closed.clone();
        let failed = self.failed.clone();
        let exited = self.interactivity.core.lock().exited.clone();
        std::thread::Builder::new()
            .name(String::from("tg-state-watch"))
            .spawn(move || {
                while let Ok(state) = state_rx.recv() {
                    if state.is_terminal() {
                        exited.store(true, std::sync::atomic::Ordering::Release);
                        // Only a clean exit closes the pane; a failed one keeps
                        // its "[process exited with code N]" message on screen.
                        if state == connection::ConnectionState::Closed {
                            closed.store(true, std::sync::atomic::Ordering::Release);
                        } else {
                            failed.store(true, std::sync::atomic::Ordering::Release);
                        }
                        let _ = tx.unbounded_send(());
                        break;
                    }
                }
            })
            .ok()?;
        Some(cx.spawn(async move |this, cx| {
            if rx.next().await.is_some() {
                let _ = this.update(cx, |_, cx| cx.notify());
            }
        }))
    }

    /// Port `TermControl::_Initialize` `.cpp:283` + `_ConnectionOutputHandler`
    /// (`ControlCore.cpp:200-330`): spawn pump drain `TerminalOutputEvent`
    /// channel (`crossbeam Receiver<String>`), invoke
    /// `ControlCore::connection_output_handler(chunk)`, sau `cx.notify()` để
    /// trigger GPUI re-render.
    ///
    /// **Architecture**: 2-stage bridge (port-blocker bypass):
    /// 1. **OS thread** bridge: blocking `crossbeam_rx.recv()` → push vào
    ///    `futures::channel::mpsc::UnboundedSender<String>` (async stream).
    ///    Thread ends khi crossbeam channel close (connection dropped) — drop
    ///    sender tự close async stream → main-thread task `next().await`
    ///    returns None → exit.
    /// 2. **GPUI main-thread task** (`cx.spawn`): `rx_async.next().await` —
    ///    return chunk; `WeakEntity::update` lock core (try_lock — không block
    ///    main thread), dispatch tới `connection_output_handler`, `cx.notify()`.
    ///
    /// **Port-blocker bypass**: task KHÔNG giữ `Arc<Mutex<ControlCore>>` (vốn
    /// đòi `RenderEngine: Send`). Thay vào đó, dùng `WeakEntity<TermControl>`
    /// (held trong `cx.spawn` closure naturaly) → `this.update(cx, ...)`
    /// lock+process on main thread. Bypass gate `Send` bound trên `RenderEngine`.
    ///
    /// **Send-checks**: `crossbeam_channel::Receiver<String>: Send` ✓,
    /// `mpsc::UnboundedSender<String>: Send` ✓ — thread closure Send. Main
    /// task: `mpsc::UnboundedReceiver<String>: Send` → `cx.spawn` closure Send.
    ///
    /// Returns `None` nếu `output_rx` đã taken (caller double-call) HOẶC OS
    /// thread spawn fails (rare) — caller should not depend on task in that
    /// case (connection still active, output silent dropped).
    pub fn spawn_output_thread(&mut self, cx: &mut gpui::Context<Self>) -> Option<gpui::Task<()>> {
        use futures::channel::mpsc;
        use futures::StreamExt;

        // Drain `TerminalOutputEvent` subscription của ControlCore (chỉ một
        // consumer — `take_output_rx` để tránh double-pump).
        let crossbeam_rx = self.interactivity.core.lock().take_output_rx()?;

        // Bridge: OS thread reads crossbeam blocking → async channel.
        let (tx_async, rx_async) = mpsc::unbounded::<String>();
        let pump_thread_result = std::thread::Builder::new()
            .name(String::from("tg-output-pump"))
            .spawn(move || {
                // Drain original crossbeam → futures channel. Khi crossbeam
                // close (connection drop), `recv()` Err → exit; sender drop
                // tự close async stream → main task `next()` None → exit.
                while let Ok(chunk) = crossbeam_rx.recv() {
                    if tx_async.unbounded_send(chunk).is_err() {
                        break; // Consumer gone — bail.
                    }
                }
            });
        // Spawn failure (rare — OS resource exhaustion) → bail với None.
        if pump_thread_result.is_err() {
            return None;
        }

        // Main-thread GPUI task: awaits chunks từ async stream —
        // `cx.spawn` gives `WeakEntity<TermControl>` (`this`) + `AsyncApp`
        // (`cx`) able to call `WeakEntity::update` on main thread.
        let task = cx.spawn(async move |this, cx| {
            let mut rx_async = rx_async;
            while let Some(chunk) = rx_async.next().await {
                // `WeakEntity::update` returns `Result<R>` — bail nếu entity
                // dropped (window closed). Lock core short, no await inside
                // lock — giữ mutex không held across await points.
                let dispatch_result = this.update(cx, |this, cx| {
                    if let Some(mut core) = this.interactivity.core.try_lock() {
                        // Port _ConnectionOutputHandler
                        // `ControlCore.cpp:200-330`. Phase 3: still no-op
                        // (Lane C TerminalDispatch impl pending) — nhưng pump
                        // đã unblock, wiring sẵn sàng.
                        core.connection_output_handler(&chunk);
                        // New output: keep find results current and restart the blink.
                        this.view.refresh_search(&core.engine.dispatch.terminal);
                        this.view.cursor_on = true;
                    }
                    // Trigger GPUI re-render — port `NotifyPaintFrame`.
                    cx.notify();
                });
                if dispatch_result.is_err() {
                    // Entity dropped (window closed) — exit pump.
                    break;
                }
            }
        });
        Some(task)
    }

    /// Tinh rows x cols tu cell metrics + viewport size - port
    /// `TermControl::_SizeChanged` `.cpp:300-400` (cols = pixel / cell).
    /// Phase 2: dung default cell metrics; phase 3 lay tu renderer.
    pub fn bordered_size(&self, viewport: gpui::Size<gpui::DevicePixels>) -> (u16, u16) {
        let w = viewport.width.0 as f32 / DEFAULT_CELL_W;
        let h = viewport.height.0 as f32 / DEFAULT_CELL_H;
        let cols = (w.floor() as u16).max(1);
        let rows = (h.floor() as u16).max(1);
        (rows, cols)
    }

    /// Port `TermControl::Focus` `.cpp:780-810`: GPUI `focus_handle.focus(cx)`.
    pub fn focus(&self, window: &mut gpui::Window, cx: &mut gpui::App) {
        self.focus_handle.focus(window, cx);
    }

    /// Port `TermControl::LostFocus` `.cpp:810-840`: dismiss cursor blink +
    /// tooltip. WT `_LostFocus` handler.
    pub fn dismiss_focus(&mut self) {
        self.interactivity.handle_focus_changed(false);
    }

    /// Port `TermControl::_SizeChanged` `.cpp:300-400` callback - ban khi
    /// GPUI window resize. Lay `rows*cols` -> `interactivity.core.size_changed`.
    pub fn on_resize(&mut self, new_rows: u16, new_cols: u16) -> anyhow::Result<()> {
        if let Some(mut core) = self.interactivity.core.try_lock() {
            core.size_changed(new_rows, new_cols)
        } else {
            Ok(())
        }
    }
}

/// Text input path: committed text (typing, IME, `VK_PACKET`) goes to the shell.
/// Composition (preedit) is not rendered yet — only the commit is forwarded.
impl gpui::EntityInputHandler for TermControl {
    fn text_for_range(
        &mut self,
        _range: std::ops::Range<usize>,
        _adjusted: &mut Option<std::ops::Range<usize>>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<gpui::UTF16Selection> {
        let end = self.preedit.as_ref().map_or(0, |p| p.encode_utf16().count());
        Some(gpui::UTF16Selection { range: end..end, reversed: false })
    }

    fn marked_text_range(
        &self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.preedit.as_ref().map(|p| 0..p.encode_utf16().count())
    }

    fn unmark_text(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if self.preedit.take().is_some() {
            cx.notify();
        }
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        text: &str,
        window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) {
        // The WM_CHAR of an Alt-chord is a key binding, not typed text.
        let m = window.modifiers();
        if m.alt && !m.control {
            return;
        }
        self.preedit = None;
        self.input_text(text, _cx);
    }

    /// IME composition: remember the in-progress text so it is drawn at the cursor.
    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<std::ops::Range<usize>>,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.preedit = (!new_text.is_empty()).then(|| new_text.to_string());
        cx.notify();
    }

    /// Alt-chords (pane shortcuts like Alt+Shift+`+`) are key bindings, not
    /// text. AltGr reports Ctrl+Alt and stays text input.
    fn accepts_text_input(&self, window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> bool {
        let m = window.modifiers();
        !(m.alt && !m.control)
    }

    /// Where the IME should put its candidate window: the cursor cell.
    fn bounds_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        element_bounds: gpui::Bounds<gpui::Pixels>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        let m = self.metrics.get();
        if f32::from(m.cell_w) <= 0.0 {
            return Some(element_bounds);
        }
        let (x, y) = self.interactivity.core.lock().engine.dispatch.terminal.get_cursor().get_position();
        Some(gpui::Bounds::new(
            gpui::point(m.origin.x + m.cell_w * x.max(0) as f32, m.origin.y + m.line_h * y.max(0) as f32),
            gpui::size(m.cell_w, m.line_h),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<gpui::Pixels>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<usize> {
        None
    }
}
