//! Input handling and rendering glue of `TermControl`: keyboard shortcuts
//! (copy/paste/find/scroll), mouse selection, wheel scrolling, clipboard and
//! the find bar. Mirrors `TermControl::_KeyHandler`, `_PointerPressed…`,
//! `ControlInteractivity` selection logic.

use core::dispatch::td::MouseModes;
use gpui::{
    div, px, rgb, AnyElement, App, ClipboardItem, Context, InteractiveElement, IntoElement,
    KeyDownEvent, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, Render, ScrollEvent, StatefulInteractiveElement, Styled, Window,
};

use crate::control::TermControl;
use crate::mouse_report::{Button, Mods, Msg, Out};
use crate::selection::{Cell, Granularity, Selection};
use crate::term_view::{terminal_canvas, PaintInput, BG};
use crate::view_state::SearchState;

impl TermControl {
    /// Send text typed by the user: output follows again, selection clears.
    pub(crate) fn send_to_shell(&mut self, text: &str) {
        self.view.snap_to_bottom();
        self.view.selection = None;
        self.view.cursor_on = true;
        if let Err(e) = self.interactivity.core.lock().send_input(text) {
            log::warn!("send_input failed: {e}");
        }
    }

    /// Text from the IME / Unicode path; goes to the find box while it is open.
    pub(crate) fn input_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.chars().any(char::is_control) {
            return;
        }
        if self.view.search.is_some() {
            self.search_edit(|s| s.query.push_str(text));
            cx.notify();
        } else {
            self.send_to_shell(text);
        }
    }

    fn page_rows(&self) -> usize {
        self.metrics.get().rows as usize
    }

    fn scroll(&mut self, rows: isize, cx: &mut Context<Self>) {
        let core = self.interactivity.core.clone();
        let core = core.lock();
        self.view.scroll_by(&core.engine.dispatch.terminal, rows);
        cx.notify();
    }

    pub(crate) fn copy_selection(&mut self, cx: &mut App) -> bool {
        let Some(sel) = self.view.selection else {
            return false;
        };
        let text = sel.text(&self.interactivity.core.lock().engine.dispatch.terminal);
        if text.is_empty() {
            return false;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    pub(crate) fn paste(&mut self, cx: &mut App) {
        let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) else {
            return;
        };
        let text = text.replace("\r\n", "\r").replace('\n', "\r");
        // WT `multiLinePasteWarning`: ask before pasting several lines.
        if text.trim_end_matches('\r').contains('\r') {
            self.view.pending_paste = Some(text);
            return;
        }
        self.send_paste(&text);
    }

    pub(crate) fn send_paste(&mut self, text: &str) {
        let bracketed = self
            .interactivity
            .core
            .lock()
            .engine
            .dispatch
            .bracketed_paste;
        let text = if bracketed {
            format!("\x1b[200~{text}\x1b[201~")
        } else {
            text.to_string()
        };
        self.send_to_shell(&text);
    }

    fn confirm_paste(&mut self, accept: bool, cx: &mut Context<Self>) {
        if let Some(text) = self.view.pending_paste.take() {
            if accept {
                self.send_paste(&text);
            }
        }
        cx.notify();
    }

    fn zoom_font(&mut self, delta_pt: f32, cx: &mut Context<Self>) {
        let base = self
            .interactivity
            .core
            .lock()
            .settings
            .default_appearance
            .font
            .size;
        self.view.font_zoom_pt = if delta_pt == 0.0 {
            0.0
        } else {
            (self.view.font_zoom_pt + delta_pt).max(1.0 - base)
        };
        cx.notify();
    }

    fn toggle_case(&mut self, cx: &mut Context<Self>) {
        self.search_edit(|s| s.case_sensitive = !s.case_sensitive);
        cx.notify();
    }

    fn search_edit(&mut self, f: impl FnOnce(&mut SearchState)) {
        let core = self.interactivity.core.clone();
        let core = core.lock();
        let buf = &core.engine.dispatch.terminal;
        if let Some(s) = &mut self.view.search {
            f(s);
            s.current = None;
        }
        self.view.refresh_search(buf);
        let rows = self.page_rows();
        self.view.step_search(buf, true, rows);
    }

    pub(crate) fn handle_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        let (ctrl, shift, alt) = (m.control, m.shift, m.alt);
        let key = ks.key.as_str();
        let mut handled = true;

        // Paste confirmation takes every key until answered.
        if self.view.pending_paste.is_some() {
            match key {
                "enter" => self.confirm_paste(true, cx),
                "escape" => self.confirm_paste(false, cx),
                _ => {}
            }
            cx.stop_propagation();
            return;
        }

        if self.view.search.is_some() {
            if alt && key == "c" {
                self.toggle_case(cx);
                cx.stop_propagation();
                return;
            }
            match (key, ctrl && shift) {
                ("escape", _) | ("f", true) => self.view.search = None,
                ("enter", _) => {
                    let core = self.interactivity.core.clone();
                    let core = core.lock();
                    let rows = self.page_rows();
                    self.view
                        .step_search(&core.engine.dispatch.terminal, !shift, rows);
                }
                ("backspace", _) => self.search_edit(|s| {
                    s.query.pop();
                }),
                // Printable keys reach the find box through the text path.
                _ => handled = !ev.prefer_character_input,
            }
            if handled {
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }

        match (key, ctrl, shift, alt) {
            ("c", true, true, false) | ("insert", true, false, false) => {
                self.copy_selection(cx);
            }
            ("v", true, true, false) | ("insert", false, true, false) => self.paste(cx),
            // WT defaults: ctrl+= / ctrl+- / ctrl+0 (no Shift) change the font size.
            ("=" | "plus", true, false, false) => self.zoom_font(1.0, cx),
            ("-" | "minus", true, false, false) => self.zoom_font(-1.0, cx),
            ("0", true, false, false) => self.zoom_font(0.0, cx),
            ("f", true, true, false) => {
                self.view.search = Some(SearchState::default());
            }
            // After a failed exit the pane says "[press Ctrl+D to close]".
            ("d", true, false, false) if self.has_failed() => self.request_close(),
            // WT: scrollUp/Down use the system "lines per wheel notch" (3); pages = viewport height.
            ("up", true, true, false) => self.scroll(-3, cx),
            ("down", true, true, false) => self.scroll(3, cx),
            ("pageup", true, true, false) => self.scroll(-(self.page_rows() as isize).max(1), cx),
            ("pagedown", true, true, false) => self.scroll((self.page_rows() as isize).max(1), cx),
            ("home", true, true, false) => {
                let core = self.interactivity.core.clone();
                self.view
                    .scroll_to_top(&core.lock().engine.dispatch.terminal);
            }
            ("end", true, true, false) => self.view.snap_to_bottom(),
            // WT default: Enter with a selection copies it instead of sending Enter.
            ("enter", false, false, false) if self.view.selection.is_some() => {
                self.copy_selection(cx);
                self.view.selection = None;
            }
            ("escape", false, false, false) if self.view.selection.is_some() => {
                self.view.selection = None
            }
            // Chords the page binds bubble up untouched.
            _ if crate::shortcuts::is_app_shortcut(ks) => handled = false,
            _ => {
                // `TrySendKeyEvent`: any key that reaches the terminal clears the selection.
                if self.view.selection.take().is_some() {
                    cx.notify();
                }
                let km = {
                    let core = self.interactivity.core.lock();
                    let d = &core.engine.dispatch;
                    crate::key_encode::KeyModes {
                        cursor_key: d.input.cursor_key,
                        backarrow: d.input.backarrow,
                        line_feed: d.modes.test(core::dispatch::td::Mode::LineFeed),
                    }
                };
                let text_path = ev.prefer_character_input && !ctrl && !alt;
                match (text_path, crate::key_encode::encode_keystroke(ks, km)) {
                    (false, Some(text)) => self.send_to_shell(&text),
                    _ => handled = false, // text arrives via the input handler; app shortcuts bubble up
                }
            }
        }
        if handled {
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn cell_at(&self, pos: Point<Pixels>) -> Cell {
        let m = self.metrics.get();
        let fx = f32::from(pos.x - m.origin.x) / f32::from(m.cell_w).max(1.0);
        let fy = f32::from(pos.y - m.origin.y) / f32::from(m.line_h).max(1.0);
        let col = (fx.floor().max(0.0) as usize).min((m.cols as usize).saturating_sub(1));
        let row = (fy.floor().max(0.0) as usize).min((m.rows as usize).saturating_sub(1));
        Cell {
            id: m.top_id + row,
            col,
        }
    }

    fn mouse_modes(&self) -> MouseModes {
        self.interactivity.core.lock().engine.dispatch.mouse
    }

    fn on_alt_screen(&self) -> bool {
        self.interactivity
            .core
            .lock()
            .engine
            .dispatch
            .using_alt_buffer
    }

    /// `_canSendVTMouseInput`: the program tracks the mouse and Shift is not held.
    fn vt_mouse(&self, mods: &Modifiers) -> bool {
        !mods.shift && self.mouse_modes().tracking()
    }

    /// `ControlInteractivity::_sendMouseEventHelper` → `TerminalInput::HandleMouse`.
    fn report(
        &mut self,
        msg: Msg,
        pos: Point<Pixels>,
        mods: &Modifiers,
        delta: i32,
        held: Option<Button>,
    ) -> Out {
        let modes = self.mouse_modes();
        let alt = self.on_alt_screen();
        let cell = self.cell_at(pos);
        let row = cell.id.saturating_sub(self.metrics.get().top_id);
        let m = Mods {
            shift: mods.shift,
            alt: mods.alt,
            ctrl: mods.control,
        };
        let out = self
            .reporter
            .handle(&modes, alt, msg, (cell.col, row), m, delta, held);
        match &out {
            Out::Send(s) if !s.is_empty() => {
                if let Err(e) = self.interactivity.core.lock().send_input(s) {
                    log::warn!("mouse report failed: {e}");
                }
            }
            Out::AltScroll(key) => {
                // `_makeAlternateScrollOutput`: the arrow key, honouring DECCKM.
                if let Ok(ks) = gpui::Keystroke::parse(key) {
                    if let Some(seq) = crate::key_encode::encode_keystroke(&ks, self.key_modes()) {
                        let _ = self.interactivity.core.lock().send_input(&seq);
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn key_modes(&self) -> crate::key_encode::KeyModes {
        let core = self.interactivity.core.lock();
        let d = &core.engine.dispatch;
        crate::key_encode::KeyModes {
            cursor_key: d.input.cursor_key,
            backarrow: d.input.backarrow,
            line_feed: d.modes.test(core::dispatch::td::Mode::LineFeed),
        }
    }

    fn button_of(b: MouseButton) -> Option<Button> {
        match b {
            MouseButton::Left => Some(Button::Left),
            MouseButton::Middle => Some(Button::Middle),
            MouseButton::Right => Some(Button::Right),
            _ => None,
        }
    }

    fn mouse_up(&mut self, ev: &MouseUpEvent, cx: &mut Context<Self>) {
        if let Some(b) = Self::button_of(ev.button) {
            if self.vt_mouse(&ev.modifiers) {
                self.report(Msg::Up(b), ev.position, &ev.modifiers, 0, None);
                self.held_button = None;
                cx.notify();
                return;
            }
        }
        if ev.button == MouseButton::Left {
            self.drag_anchor = None;
        }
        self.held_button = None;
    }

    fn mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus(window, cx);
        if let Some(b) = Self::button_of(ev.button) {
            if self.vt_mouse(&ev.modifiers) {
                self.held_button = Some(b);
                self.report(Msg::Down(b), ev.position, &ev.modifiers, 0, None);
                return;
            }
        }
        let at = self.cell_at(ev.position);
        match ev.button {
            MouseButton::Left => {
                let granularity = match ev.click_count {
                    1 => Granularity::Char,
                    2 => Granularity::Word,
                    _ => Granularity::Line,
                };
                if ev.modifiers.shift && self.view.selection.is_some() {
                    if let Some(s) = &mut self.view.selection {
                        s.extend(at);
                    }
                } else if granularity == Granularity::Char {
                    self.view.selection = None;
                    self.drag_anchor = Some(at);
                } else {
                    self.view.selection = Some(Selection::new(at, granularity));
                    self.drag_anchor = Some(at);
                }
            }
            MouseButton::Right => {
                // WT default: right click copies a selection, else pastes.
                if self.copy_selection(cx) {
                    self.view.selection = None;
                } else {
                    self.paste(cx);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn mouse_move(&mut self, ev: &MouseMoveEvent, cx: &mut Context<Self>) {
        // `PointerMoved`: motion reports only while the control has focus.
        if self.vt_mouse(&ev.modifiers) {
            if self.last_focused {
                let held = ev.pressed_button.and_then(Self::button_of);
                self.report(Msg::Move, ev.position, &ev.modifiers, 0, held);
            }
            return;
        }
        let Some(anchor) = self.drag_anchor else {
            return;
        };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.drag_anchor = None;
            return;
        }
        let at = self.cell_at(ev.position);
        // Dragging above/below the view scrolls it.
        let m = self.metrics.get();
        if ev.position.y < m.origin.y {
            self.scroll(-1, cx);
        } else if ev.position.y > m.origin.y + m.line_h * m.rows as f32 {
            self.scroll(1, cx);
        }
        let gran = self
            .view
            .selection
            .map_or(Granularity::Char, |s| s.granularity());
        if self.view.selection.is_none() && at == anchor {
            return;
        }
        let mut sel = self
            .view
            .selection
            .unwrap_or_else(|| Selection::new(anchor, gran));
        sel.extend(at);
        self.view.selection = Some(sel);
        cx.notify();
    }

    /// `ControlInteractivity::MouseWheel`.
    fn wheel(&mut self, ev: &ScrollEvent, cx: &mut Context<Self>) {
        let line_h = f32::from(self.metrics.get().line_h).max(1.0);
        let (lx, ly) = match ev.delta {
            gpui::ScrollDelta::Lines(p) => (p.x, p.y),
            gpui::ScrollDelta::Pixels(p) => (f32::from(p.x) / line_h, f32::from(p.y) / line_h),
        };
        let shift = ev.modifiers.shift;
        // GPUI reports Shift+wheel as horizontal and a real horizontal wheel
        // negated; undo both to get WT's vertical/horizontal wheel deltas.
        let (vertical, lines) = if shift && ly == 0.0 {
            (true, lx)
        } else {
            (ly != 0.0, if ly != 0.0 { ly } else { -lx })
        };
        // GPUI scales a notch by the system "lines per notch" (3): 40 delta per line.
        let delta = (lines * 40.0).round() as i32;
        if delta == 0 {
            return;
        }
        let modes = self.mouse_modes();
        let alt_scroll = !shift && self.on_alt_screen() && modes.alternate_scroll;
        if !shift && (modes.tracking() || alt_scroll) {
            let msg = if vertical { Msg::Wheel } else { Msg::HWheel };
            self.report(msg, ev.position, &ev.modifiers, delta, None);
            return;
        }
        let ctrl = ev.modifiers.control;
        if ctrl && shift {
            return; // opacity: no acrylic/opacity in this port
        }
        if ctrl {
            self.zoom_font(if delta > 0 { 1.0 } else { -1.0 }, cx);
            return;
        }
        // `_mouseScrollHandler`: accumulate fractional rows.
        self.scroll_accum += -(lines);
        let whole = self.scroll_accum.trunc();
        self.scroll_accum -= whole;
        if whole != 0.0 {
            self.scroll(whole as isize, cx);
        }
        // Scrolling while selecting re-points the selection end.
        if self.drag_anchor.is_some() {
            let at = self.cell_at(ev.position);
            if let Some(mut sel) = self.view.selection {
                sel.extend(at);
                self.view.selection = Some(sel);
            }
        }
    }

    fn paste_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = self.view.pending_paste.as_ref()?;
        let lines: Vec<&str> = text.split('\r').filter(|l| !l.is_empty()).collect();
        let preview: String = lines
            .iter()
            .take(5)
            .map(|l| l.chars().take(80).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        let more = if lines.len() > 5 {
            format!("\n… (+{} more lines)", lines.len() - 5)
        } else {
            String::new()
        };
        let button = |id: &'static str,
                      label: &'static str,
                      primary: bool,
                      accept: bool,
                      cx: &mut Context<Self>| {
            div()
                .id(id)
                .px(px(16.))
                .py(px(6.))
                .rounded(px(4.))
                .bg(rgb(if primary { 0x0078D4 } else { 0x3A3A3A }))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.confirm_paste(accept, cx)))
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .justify_center()
                .items_center()
                .bg(gpui::rgba(0x00000099))
                .child(
                    div()
                        .w(px(520.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .p(px(20.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(rgb(0x3A3A3A))
                        .bg(rgb(0x202020))
                        .text_color(rgb(0xFFFFFF))
                        .text_size(px(13.))
                        .child(div().text_size(px(16.)).child("Warning"))
                        .child(div().child("You are about to paste text that contains multiple lines. If you paste this text into your shell, it may result in the unexpected execution of commands."))
                        .child(div().p(px(8.)).rounded(px(4.)).bg(rgb(0x0C0C0C)).text_color(rgb(0xCCCCCC)).child(format!("{preview}{more}")))
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .justify_end()
                                .child(button("paste-ok", "Paste", true, true, cx))
                                .child(button("paste-cancel", "Cancel", false, false, cx)),
                        ),
                )
                .into_any_element(),
        )
    }

    fn search_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let s = self.view.search.as_ref()?;
        let total = crate::search::group_count(&s.matches);
        let count = match s.current {
            Some(i) => format!("{}/{}", i + 1, total),
            None => format!("0/{}", total),
        };
        Some(
            div()
                .absolute()
                .top(px(6.))
                .right(px(18.))
                .flex()
                .gap(px(10.))
                .px(px(10.))
                .py(px(5.))
                .rounded(px(4.))
                .border_1()
                .border_color(rgb(0x3A3A3A))
                .bg(rgb(0x202020))
                .text_color(rgb(0xFFFFFF))
                .text_size(px(13.))
                .child(div().child(if s.query.is_empty() {
                    "Find…".to_string()
                } else {
                    format!("{}\u{258F}", s.query)
                }))
                .child(
                    div()
                        .id("find-case")
                        .px(px(5.))
                        .rounded(px(3.))
                        .bg(rgb(if s.case_sensitive { 0x0078D4 } else { 0x202020 }))
                        .hover(|st| st.bg(rgb(0x3A3A3A)))
                        .child("Aa")
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_case(cx))),
                )
                .child(div().text_color(rgb(0x9D9D9D)).child(count))
                .into_any_element(),
        )
    }
}

impl Render for TermControl {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let font = self
            .interactivity
            .core
            .lock()
            .settings
            .default_appearance
            .font
            .clone();
        // DECSET 1004 + hollow cursor follow OS activation, not just element focus.
        let focused = self.focus_handle.is_focused(window) && window.is_window_active();
        if focused != self.last_focused {
            self.last_focused = focused;
            if self.mouse_modes().focus {
                let _ = self.interactivity.core.lock().send_input(if focused {
                    "\x1b[I"
                } else {
                    "\x1b[O"
                });
            }
        }
        let canvas = terminal_canvas(PaintInput {
            core: self.interactivity.core.clone(),
            entity: cx.entity(),
            focus: self.focus_handle.clone(),
            focused,
            view: self.view.clone(),
            metrics: self.metrics.clone(),
            font_family: font.face.clone(),
            font_px: ((font.size + self.view.font_zoom_pt).max(1.0)) * 96.0 / 72.0,
            preedit: self.preedit.clone(),
        });
        let search_bar = self.search_bar(cx);
        let paste_dialog = self.paste_dialog(cx);
        div()
            .size_full()
            .relative()
            .bg(rgb(BG))
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| this.handle_key(ev, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, w, cx| this.mouse_down(ev, w, cx)),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, ev: &MouseDownEvent, w, cx| this.mouse_down(ev, w, cx)),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| this.mouse_move(ev, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseUpEvent, _, cx| this.mouse_up(ev, cx)),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, ev: &MouseUpEvent, _, cx| this.mouse_up(ev, cx)),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, ev: &MouseDownEvent, w, cx| this.mouse_down(ev, w, cx)),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, ev: &MouseUpEvent, _, cx| this.mouse_up(ev, cx)),
            )
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, _, cx| this.wheel(ev, cx)))
            .child(canvas)
            .children(search_bar)
            .children(paste_dialog)
    }
}
