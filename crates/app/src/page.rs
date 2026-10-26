//! `TerminalPage` — port của `windows/src/cascadia/TerminalApp/TerminalPage.cpp`
//! (~6300 LOC). Top-level UI host: sở hữu `Tab` list, tab bar, dropdown menu,
//! new-tab, split, close-tab, command palette.
//!
//! Phase 2 wire-up:
//! - `_create_connection_from_settings` (`.cpp:1554-1662`): build `ConptySettings`
//!   từ `Profile`, construct `ConptyConnection`, `connection.initialize(settings)`.
//!   Phase 2 không spawn process thật — `connection.start()` chỉ state-transition
//!   (`Connecting`) trong Lane B (FFI fill phase 4).
//! - Root render: tab bar (placeholder) + active tab delegate. Phase 2 single
//!   pane; full `Pane::render` split + tabbar styling phase 3.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};

use crate::downstream::{
    ConptyConnection, ConptySettings, ITerminalConnection, Profile, PseudoConsoleFlags,
};
use crate::pane_tree::Direction;
use crate::settings_loader::CascadiaSettings;
use crate::tab::{SplitDirection, Tab};
use crate::ui::palette::{
    default_commands, PaletteAction, PaletteCallbacks, PalettePanel, PaletteState,
};
use crate::ui::{PageLayout, TabItem, TabStrip, TabStripCallbacks, TabStripModel, Theme, Titlebar};
// Note: `GlobalAppSettings` không dùng trong produção path — test-only import.
#[cfg(test)]
use settings::GlobalAppSettings;

/// `TerminalPage` — `TerminalPage.h:30`. Root view bind vào `cx.open_window`.
pub struct TerminalPage {
    /// `_tabs` (`TerminalPage.h:88`) — list active tabs.
    tabs: Vec<Entity<Tab>>,
    /// `_settings` (`TerminalPage.h:62`) — loaded CascadiaSettings.
    settings: CascadiaSettings,
    /// `_GetFocusedTab` index — phase 2 single active tab.
    active_tab_index: Option<usize>,
    /// Command palette (Ctrl+Shift+P) — `Some` while open.
    palette: Option<PaletteState<PaletteAction>>,
    /// Root focus target, created on first render.
    focus_handle: Option<FocusHandle>,
    /// Tab being dragged for reordering (index it currently occupies).
    drag_tab: Option<usize>,
    /// Tab whose context menu is open.
    tab_menu: Option<(usize, gpui::Point<gpui::Pixels>)>,
    /// Inline tab rename in progress: `(tab index, text so far)`.
    renaming: Option<(usize, String)>,
    /// Controls whose exit-error infobar was dismissed.
    infobar_dismissed: Vec<gpui::EntityId>,
    /// "Close all tabs?" confirmation showing (window close with several tabs).
    confirm_close: bool,
    /// `on_window_should_close` hook registered.
    close_hook_set: bool,
}

impl TerminalPage {
    /// `TerminalPage::TerminalPage` ctor (`.cpp:~400` —_DELAYED init).
    /// Phase 2: load settings, open initial tab.
    pub fn new(settings: CascadiaSettings, cx: &mut App) -> Self {
        let mut page = Self {
            tabs: Vec::new(),
            settings,
            active_tab_index: None,
            palette: None,
            focus_handle: None,
            drag_tab: None,
            tab_menu: None,
            renaming: None,
            infobar_dismissed: Vec::new(),
            confirm_close: false,
            close_hook_set: false,
        };
        page._open_new_tab_via_dropdown(None, cx);
        page
    }

    /// A page that starts with an existing tab (tab moved from another window).
    pub fn with_tab(settings: CascadiaSettings, tab: Entity<Tab>) -> Self {
        Self {
            tabs: vec![tab],
            settings,
            active_tab_index: Some(0),
            palette: None,
            focus_handle: None,
            drag_tab: None,
            tab_menu: None,
            renaming: None,
            infobar_dismissed: Vec::new(),
            confirm_close: false,
            close_hook_set: false,
        }
    }

    /// Append an existing tab and select it.
    pub fn push_tab(&mut self, tab: Entity<Tab>) {
        self.tabs.push(tab);
        self.active_tab_index = Some(self.tabs.len() - 1);
    }

    /// `_OpenNewTerminalViaDropdown` (`.cpp:1451`) — mở tab mới với profile
    /// `profile` (None = default profile). Phase 2: build connection, spawn
    /// TermControl entity, wrap trong `Tab`, push vào `tabs`.
    fn _open_new_tab_via_dropdown(&mut self, profile_index: Option<usize>, cx: &mut App) {
        let profile = self._select_profile(profile_index);
        let connection = self._create_connection_from_settings(&profile);
        let control = self._make_pane(&profile, connection, cx);
        let tab = cx.new(|_cx| Tab::new(control, _cx));
        tab.update(cx, |t, _| t.set_profile(profile.clone()));
        self.tabs.push(tab);
        // MED-2 fix (memory 2026-10-25): WT `TabManagement.cpp:213` luôn switch
        // sang tab mới được mở (`_tabView.SelectedItem(tabViewItem)`). Trước
        // chỉ set khi `is_none()` → focus ở tab cũ → bug. Giờ luôn switch.
        self.active_tab_index = Some(self.tabs.len() - 1);
    }

    /// Chọn profile theo index (None = default profile).
    fn _select_profile(&self, profile_index: Option<usize>) -> Profile {
        match profile_index {
            Some(i) => self
                .settings
                .profiles
                .get(i)
                .cloned()
                .unwrap_or_else(|| self._default_profile()),
            None => self._default_profile(),
        }
    }

    /// Default profile — resolve `default_profile_guid` → matching `Profile`.
    /// Fallback về PowerShell inbox profile nếu not found (defensive).
    fn _default_profile(&self) -> Profile {
        let guid = self.settings.default_profile_guid.hyphenated().to_string();
        self.settings
            .profiles
            .iter()
            .find(|p| {
                let p_guid = p.guid.trim_matches(|c: char| c == '{' || c == '}');
                let theirs: Option<String> = uuid::Uuid::parse_str(p_guid)
                    .ok()
                    .map(|u| u.hyphenated().to_string());
                theirs.as_deref() == Some(guid.as_str())
            })
            .cloned()
            .unwrap_or_else(Profile::default_powershell)
    }

    /// `_CreateConnectionFromSettings` terminal-specific (`.cpp:1554-1662`).
    ///
    /// Build `ConptySettings` từ `Profile`, `ConptyConnection::new()`, gọi
    /// `connection.initialize(settings)`. Phase 2 KHÔNG spawn process thật —
    /// `start()` sẽ được gọi trong `_MakePane`/control بعد.
    ///
    /// WT logic bỏ qua: Azure branch, `_currentWindowSettings().TextMeasurement`,
    /// `AmbiguousWidth`, `_WindowProperties.VirtualEnvVars`, `_evaluatePathForCwd`
    /// (path resolution — Phase 4 command spawning).
    fn _create_connection_from_settings(&self, profile: &Profile) -> Box<dyn ITerminalConnection> {
        let mut settings = build_conpty_settings(profile);
        settings.initial_rows = self.settings.globals.initial_rows as u32;
        settings.initial_cols = self.settings.globals.initial_cols as u32;
        let mut conn = ConptyConnection::new();
        if let Err(e) = conn.initialize(settings) {
            log::error!("ConptyConnection::initialize failed (phase 2 FFI fill pending): {e}");
        }
        Box::new(conn)
    }

    /// `_MakePane` (`.cpp:~3850`/`3911`) — spawn `TermControl` từ connection +
    /// profile. WT build `TermControl` với `Connection` + `Renderer` + settings.
    ///
    /// Phase 3 port (stub-downstream path): `TermControl` là stub unit struct
    /// nên chỉ `cx.new(|_| TermControl)`. Connection drop (phase 4 wire vào
    /// `TermControl::new(connection, renderer, settings)`). Trả entity view.
    ///
    /// Lane B path (`downstream-lane-b`): port thực cần
    /// `core::TextBuffer` + `renderer::RenderEngine` + `KeyMapping` —
    /// wire-up phase 4 sau khi connection/renderer rebuild.
    fn _make_pane(
        &self,
        _profile: &Profile,
        connection: Box<dyn ITerminalConnection>,
        cx: &mut App,
    ) -> Entity<crate::downstream::TermControl> {
        #[cfg(feature = "stub-downstream")]
        {
            let _ = _profile;
            drop(connection);
            cx.new(|_| crate::downstream::TermControl)
        }
        #[cfg(not(feature = "stub-downstream"))]
        {
            use control::keymap::KeyMapping;
            use core::{TextAttribute, TextBuffer};
            let profile = _profile.clone();
            let control = cx.new(|cx| {
                crate::downstream::TermControl::new(
                    connection,
                    Box::new(crate::null_render_engine::NullRenderEngine),
                    TextBuffer::new(80, 25, TextAttribute::default()),
                    profile,
                    KeyMapping::default(),
                    cx,
                )
            });
            // `TermControl::Initialize`: start ConPTY + output pump (`.cpp:283`).
            control.update(cx, |t, cx| {
                if let Err(e) = t.start(cx) {
                    log::error!("TermControl::start failed: {e}");
                }
            });
            control
        }
    }

    /// `_Tabs` accessor.
    pub fn tabs(&self) -> &[Entity<Tab>] {
        &self.tabs
    }

    /// Drag-reorder (`TabView.TabItemsChanged` → `_ReorderTab`): move tab
    /// `from` to `to`; the selected tab keeps following its content.
    pub fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let n = self.tabs.len();
        if from >= n || to >= n || from == to {
            return;
        }
        let mut model = TabStripModel::new(vec![TabItem::new(""); n], self.active_tab_index);
        model.move_tab(from, to);
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active_tab_index = model.selected();
        cx.notify();
    }

    /// Click tab trên tab row → chọn tab (`TabView.SelectionChanged`).
    pub fn handle_select_tab_action(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() && self.active_tab_index != Some(index) {
            self.active_tab_index = Some(index);
            cx.notify();
        }
    }

    /// Run a palette/keyboard action (`ShortcutAction` subset until P13).
    pub fn run_action(&mut self, action: PaletteAction, cx: &mut Context<Self>) {
        match action {
            PaletteAction::NewTab => self.handle_new_tab_action(cx),
            PaletteAction::CloseTab => {
                if let Some(i) = self.active_tab_index {
                    self.handle_close_tab_action(i, cx);
                }
            }
            PaletteAction::DuplicateTab => {
                if let Some(i) = self.active_tab_index {
                    self.tab_menu_action_duplicate(i, cx);
                }
            }
            PaletteAction::NewTabProfile(i) => {
                if i < self.settings.profiles.len() {
                    self._open_new_tab_via_dropdown(Some(i), cx);
                }
            }
            PaletteAction::SwitchTab(i) => {
                if i < self.tabs.len() {
                    self.handle_select_tab_action(i, cx);
                }
            }
            PaletteAction::NextTab => self.select_relative(1, cx),
            PaletteAction::PreviousTab => self.select_relative(-1, cx),
            PaletteAction::SplitRight => self.split_active_pane(SplitDirection::Vertical, cx),
            PaletteAction::SplitDown => self.split_active_pane(SplitDirection::Horizontal, cx),
            PaletteAction::ClosePane => self.close_active_pane(cx),
            PaletteAction::ZoomPane => self.with_active_tab(cx, |t| t.toggle_zoom()),
            PaletteAction::SwapLeft => self.with_active_tab(cx, |t| t.swap_active(Direction::Left)),
            PaletteAction::SwapRight => {
                self.with_active_tab(cx, |t| t.swap_active(Direction::Right))
            }
            PaletteAction::SwapUp => self.with_active_tab(cx, |t| t.swap_active(Direction::Up)),
            PaletteAction::SwapDown => self.with_active_tab(cx, |t| t.swap_active(Direction::Down)),
            PaletteAction::FocusLeft => self.with_active_tab(cx, |t| t.move_focus(Direction::Left)),
            PaletteAction::FocusRight => {
                self.with_active_tab(cx, |t| t.move_focus(Direction::Right))
            }
            PaletteAction::FocusUp => self.with_active_tab(cx, |t| t.move_focus(Direction::Up)),
            PaletteAction::FocusDown => self.with_active_tab(cx, |t| t.move_focus(Direction::Down)),
            PaletteAction::ResizeLeft => {
                self.with_active_tab(cx, |t| t.resize_active(Direction::Left))
            }
            PaletteAction::ResizeRight => {
                self.with_active_tab(cx, |t| t.resize_active(Direction::Right))
            }
            PaletteAction::ResizeUp => self.with_active_tab(cx, |t| t.resize_active(Direction::Up)),
            PaletteAction::ResizeDown => {
                self.with_active_tab(cx, |t| t.resize_active(Direction::Down))
            }
        }
        cx.notify();
    }

    fn with_active_tab(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Tab)) {
        if let Some(tab) = self
            .active_tab_index
            .and_then(|i| self.tabs.get(i))
            .cloned()
        {
            tab.update(cx, |t, cx| {
                f(t);
                cx.notify();
            });
        }
    }

    /// `TerminalPage::_SplitPane`: new shell (default profile) in a new pane.
    fn split_active_pane(&mut self, direction: SplitDirection, cx: &mut Context<Self>) {
        // `splitMode: duplicate` — the new pane runs the tab's profile.
        let profile = self
            .active_tab_index
            .and_then(|i| self.tabs.get(i))
            .and_then(|t| t.read(cx).profile().cloned())
            .unwrap_or_else(|| self._default_profile());
        let connection = self._create_connection_from_settings(&profile);
        let control = self._make_pane(&profile, connection, cx);
        self.with_active_tab(cx, |t| t.split_pane(direction, control));
    }

    /// Close panes whose shell exited (WT `closeOnExit`), and the tab when it
    /// was the last pane; quit once no tab is left.
    fn close_exited(&mut self, exited: Vec<(Entity<Tab>, gpui::EntityId)>, cx: &mut Context<Self>) {
        // (the window is closed by `close_window_if_empty` once no tab is left)
        for (tab, id) in exited {
            if !tab.update(cx, |t, _| t.close_pane(id)) {
                if let Some(i) = self.tabs.iter().position(|t| *t == tab) {
                    self.handle_close_tab_action(i, cx);
                }
            }
        }
        cx.notify();
    }

    /// Close this window for good (after confirmation); the app quits with its last window.
    fn close_window_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if cx.windows().len() <= 1 {
            cx.quit();
        } else {
            window.remove_window();
        }
    }

    /// An empty window closes; the app quits with its last window.
    fn close_window_if_empty(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            return;
        }
        if cx.windows().len() <= 1 {
            cx.quit();
        } else {
            window.remove_window();
        }
    }

    /// Move tab `index` into a new window placed at `origin` (screen coords).
    pub fn tear_out_tab(
        &mut self,
        index: usize,
        origin: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.len() < 2 || index >= self.tabs.len() {
            return;
        }
        let tab = self.take_tab(index);
        let settings = self.settings.clone();
        let size = window.bounds().size;
        let opts = gpui::WindowOptions {
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Termix".into()),
                appears_transparent: true,
                ..Default::default()
            }),
            window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::new(
                origin, size,
            ))),
            ..Default::default()
        };
        if let Err(e) = cx.open_window(opts, move |_w, cx| {
            cx.new(|_| TerminalPage::with_tab(settings, tab))
        }) {
            log::error!("tear-out window failed: {e}");
        }
        cx.notify();
    }

    /// Remove tab `index` (keeping it alive) and fix the selection.
    fn take_tab(&mut self, index: usize) -> Entity<Tab> {
        let tab = self.tabs.remove(index);
        self.active_tab_index = match self.active_tab_index {
            Some(a) if a == index => {
                (!self.tabs.is_empty()).then(|| index.min(self.tabs.len() - 1))
            }
            Some(a) if a > index => Some(a - 1),
            other => other,
        };
        self.drag_tab = None;
        tab
    }

    /// Mouse released after dragging tab `i`: dropped on another window's tab
    /// row → move it there; dropped outside any tab row → its own window.
    fn finish_drag(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(i) = self.drag_tab.take() else {
            return;
        };
        let y = f32::from(position.y);
        if i >= self.tabs.len() || (0.0..=crate::ui::theme::metrics::TITLEBAR_H).contains(&y) {
            return; // plain click or reorder inside this window
        }
        let screen = window.bounds().origin + position;
        let me = window.window_handle().window_id();
        let tab = self.tabs[i].clone();
        for handle in cx.windows() {
            if handle.window_id() == me {
                continue;
            }
            let Some(other) = handle.downcast::<TerminalPage>() else {
                continue;
            };
            let moving = tab.clone();
            let accepted = other
                .update(cx, move |page, w, cx| {
                    let b = w.bounds();
                    let in_strip = screen.x >= b.origin.x
                        && screen.x <= b.origin.x + b.size.width
                        && screen.y >= b.origin.y
                        && screen.y <= b.origin.y + gpui::px(crate::ui::theme::metrics::TITLEBAR_H);
                    if in_strip {
                        page.push_tab(moving);
                        cx.notify();
                    }
                    in_strip
                })
                .unwrap_or(false);
            if accepted {
                self.take_tab(i);
                cx.notify();
                return;
            }
        }
        let origin = screen - gpui::point(gpui::px(130.), gpui::px(20.));
        self.tear_out_tab(i, origin, window, cx);
    }

    /// Open a new tab with the profile of tab `tab` (`DuplicateTab`).
    fn tab_menu_action_duplicate(&mut self, tab: usize, cx: &mut Context<Self>) {
        let Some(t) = self.tabs.get(tab) else { return };
        let profile = t.read(cx).profile().cloned();
        let index =
            profile.and_then(|p| self.settings.profiles.iter().position(|q| q.guid == p.guid));
        self._open_new_tab_via_dropdown(index, cx);
    }

    /// Entry `item` of tab `tab`'s context menu (see `ui::tab_strip::TAB_MENU`).
    pub fn tab_menu_action(
        &mut self,
        tab: usize,
        item: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_menu = None;
        if tab >= self.tabs.len() {
            return;
        }
        match item {
            0 => self.handle_close_tab_action(tab, cx),
            1 => {
                let keep = self.tabs[tab].clone();
                self.tabs.retain(|t| *t == keep);
                self.active_tab_index = Some(0);
            }
            2 => {
                self.tabs.truncate(tab + 1);
                self.active_tab_index = self.active_tab_index.map(|a| a.min(tab));
            }
            3 => self.tab_menu_action_duplicate(tab, cx),
            4 => {
                let origin = window.bounds().origin + gpui::point(gpui::px(40.), gpui::px(40.));
                self.tear_out_tab(tab, origin, window, cx);
            }
            5 => {
                // Typing replaces the title; an empty name restores the shell's own.
                self.renaming = Some((tab, String::new()));
            }
            _ => {}
        }
        cx.notify();
    }

    /// `TerminalPage::_ClosePane`: close the active pane, or the tab when it
    /// is the last one.
    fn close_active_pane(&mut self, cx: &mut Context<Self>) {
        let Some(i) = self.active_tab_index.filter(|&i| i < self.tabs.len()) else {
            return;
        };
        let tab = self.tabs[i].clone();
        if !tab.update(cx, |t, _| t.close_active_pane()) {
            self.handle_close_tab_action(i, cx);
        }
    }

    fn select_relative(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.tabs.len() as isize;
        if let (Some(i), true) = (self.active_tab_index, n > 0) {
            self.handle_select_tab_action((i as isize + delta).rem_euclid(n) as usize, cx);
        }
    }

    /// Key handling for the page: palette typing when open, shortcuts otherwise.
    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        if self.confirm_close {
            match ks.key.as_str() {
                "enter" => self.close_window_now(window, cx),
                "escape" => self.confirm_close = false,
                _ => {}
            }
            cx.notify();
            return;
        }
        let (ctrl, shift, alt) = (ks.modifiers.control, ks.modifiers.shift, ks.modifiers.alt);
        if self.tab_menu.is_some() && ks.key == "escape" {
            self.tab_menu = None;
            cx.notify();
            return;
        }
        if let Some((tab, text)) = self.renaming.as_mut() {
            match ks.key.as_str() {
                "escape" => self.renaming = None,
                "enter" => {
                    let (tab, text) = (*tab, text.clone());
                    self.renaming = None;
                    if let Some(t) = self.tabs.get(tab) {
                        t.update(cx, |t, _| t.set_title_override(text.trim()));
                    }
                }
                "backspace" => {
                    text.pop();
                }
                _ => return,
            }
            cx.notify();
            return;
        }
        if let Some(p) = self.palette.as_mut() {
            match ks.key.as_str() {
                "escape" => self.palette = None,
                "enter" => {
                    let action = p.selected_action().copied();
                    self.palette = None;
                    if let Some(a) = action {
                        self.run_action(a, cx);
                    }
                }
                "up" => p.move_selection(-1),
                "down" => p.move_selection(1),
                "backspace" => {
                    let mut text = p.search().to_string();
                    text.pop();
                    p.set_search(&text);
                }
                // Printable text reaches the palette through the text-input path
                // (`EntityInputHandler`), not here — handling it twice doubles it.
                _ => return,
            }
            cx.notify();
            return;
        }
        // WT default key bindings (`defaults.json`): tabs, panes.
        let action = match (ks.key.as_str(), ctrl, alt, shift) {
            ("p", true, false, true) => {
                self.palette = Some(PaletteState::new(default_commands()));
                cx.notify();
                return;
            }
            ("t", true, false, true) => PaletteAction::NewTab,
            ("d", true, false, true) => PaletteAction::DuplicateTab,
            ("w", true, false, true) => PaletteAction::ClosePane,
            // ctrl+shift+N: new tab with profile N; ctrl+alt+N: switch to tab N.
            (k, true, false, true) if k.len() == 1 && matches!(k.as_bytes()[0], b'1'..=b'9') => {
                PaletteAction::NewTabProfile((k.as_bytes()[0] - b'1') as usize)
            }
            (k, true, true, false) if k.len() == 1 && matches!(k.as_bytes()[0], b'1'..=b'9') => {
                PaletteAction::SwitchTab((k.as_bytes()[0] - b'1') as usize)
            }
            ("tab", true, false, false) => PaletteAction::NextTab,
            ("tab", true, false, true) => PaletteAction::PreviousTab,
            ("-" | "minus", false, true, true) => PaletteAction::SplitDown,
            ("=" | "plus", false, true, true) => PaletteAction::SplitRight,
            // Windows folds Shift into the key for these: Alt+Shift+- arrives
            // as `_` and Alt+Shift+= as `+`, both without the shift flag.
            ("_", false, true, _) => PaletteAction::SplitDown,
            ("+", false, true, _) => PaletteAction::SplitRight,
            ("left", false, true, false) => PaletteAction::FocusLeft,
            ("right", false, true, false) => PaletteAction::FocusRight,
            ("up", false, true, false) => PaletteAction::FocusUp,
            ("down", false, true, false) => PaletteAction::FocusDown,
            ("left", false, true, true) => PaletteAction::ResizeLeft,
            ("right", false, true, true) => PaletteAction::ResizeRight,
            ("up", false, true, true) => PaletteAction::ResizeUp,
            ("down", false, true, true) => PaletteAction::ResizeDown,
            _ => return,
        };
        self.run_action(action, cx);
    }

    /// Active tab index (None = empty page).
    pub fn active_tab_index(&self) -> Option<usize> {
        self.active_tab_index
    }

    /// `ShortcutAction::NewTab` (`.cpp:522` → `_OpenNewTerminalViaDropdown`).
    /// Phase 3 port: open new tab với default profile. Caller là action dispatch.
    pub fn handle_new_tab_action(&mut self, cx: &mut App) {
        self._open_new_tab_via_dropdown(None, cx);
    }

    /// `ShortcutAction::NewTab` với `NewTabArgs.ContentArgs` cài `ProfileIndex`.
    /// Phase 3 port: open new tab với profile theo index (`args.ProfileIndex`).
    pub fn handle_new_tab_with_profile_action(&mut self, profile_index: usize, cx: &mut App) {
        self._open_new_tab_via_dropdown(Some(profile_index), cx);
    }

    /// `ShortcutAction::CloseTab` (`TerminalPage.cpp:5664`/.
    /// `_HandleCloseTabRequested` `:3737`). Phase 3 port: close tab theo index,
    /// cập nhật `active_tab_index` fallback về cuối tab list.
    pub fn handle_close_tab_action(&mut self, index: usize, _cx: &mut App) {
        if index >= self.tabs.len() {
            log::warn!(
                "handle_close_tab_action: index {index} out of range (len={})",
                self.tabs.len()
            );
            return;
        }
        self.tabs.remove(index);
        // Adjust active index.
        match (self.tabs.len(), self.active_tab_index) {
            (0, _) => self.active_tab_index = None,
            (_, Some(i)) if i == index => {
                // Closed active → fall back to last tab.
                self.active_tab_index = Some(self.tabs.len().saturating_sub(1));
            }
            (_, Some(i)) if i > index => {
                // Closed tab before active → shift down.
                self.active_tab_index = Some(i - 1);
            }
            _ => {}
        }
    }
}

impl Render for TerminalPage {
    /// Root render — port `TerminalPage.xaml`: titlebar chứa tab row, content
    /// là tab đang chọn (layout ở `ui::PageLayout`).
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        let items = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let tab = t.read(cx);
                let title = match &self.renaming {
                    Some((r, text)) if *r == i => format!("{text}\u{258F}"),
                    _ => tab.live_title(cx),
                };
                let mut item = TabItem::new(title);
                item.status.zoomed = tab.is_zoomed();
                item
            })
            .collect();
        if self.tabs.is_empty() {
            cx.defer_in(window, |this, window, cx| {
                this.close_window_if_empty(window, cx)
            });
        }
        // Panes whose shell has exited are closed after this frame.
        #[cfg(not(feature = "stub-downstream"))]
        {
            let mut exited = Vec::new();
            for tab in &self.tabs {
                for control in tab.read(cx).controls() {
                    if control.read(cx).is_closed() {
                        exited.push((tab.clone(), control.entity_id()));
                    }
                }
            }
            if !exited.is_empty() {
                cx.defer_in(window, move |this, _, cx| this.close_exited(exited, cx));
            }
        }
        let model = TabStripModel::new(items, self.active_tab_index);
        let page = cx.entity();
        let callbacks = TabStripCallbacks {
            on_select: Rc::new({
                let page = page.clone();
                move |i, _, cx| page.update(cx, |p, cx| p.handle_select_tab_action(i, cx))
            }),
            on_close: Rc::new({
                let page = page.clone();
                move |i, _, cx| {
                    page.update(cx, |p, cx| {
                        p.handle_close_tab_action(i, cx);
                        cx.notify();
                    })
                }
            }),
            on_new_tab: Rc::new({
                let page = page.clone();
                move |_, cx| {
                    page.update(cx, |p, cx| {
                        p.handle_new_tab_action(cx);
                        cx.notify();
                    })
                }
            }),
            on_drag_start: Rc::new({
                let page = page.clone();
                move |i, _, cx| {
                    page.update(cx, |p, cx| {
                        p.drag_tab = Some(i);
                        p.handle_select_tab_action(i, cx);
                    })
                }
            }),
            on_drag_over: Rc::new({
                let page = page.clone();
                move |i, _, cx| {
                    page.update(cx, |p, cx| {
                        if let Some(from) = p.drag_tab {
                            if from != i {
                                p.move_tab(from, i, cx);
                                p.drag_tab = Some(i);
                            }
                        }
                    })
                }
            }),
            on_context: Rc::new({
                let page = page.clone();
                move |i, pos, _, cx| {
                    page.update(cx, |p, cx| {
                        p.tab_menu = Some((i, pos));
                        p.handle_select_tab_action(i, cx);
                        cx.notify();
                    })
                }
            }),
            on_tab_menu_pick: Rc::new({
                let page = page.clone();
                move |tab, item, window, cx| {
                    page.update(cx, |p, cx| p.tab_menu_action(tab, item, window, cx))
                }
            }),
        };
        let tab_pick = callbacks.on_tab_menu_pick.clone();
        let titlebar = Titlebar::new(
            TabStrip::new(model, theme, callbacks),
            theme,
            window.is_maximized(),
        );
        let content = match self.active_tab_index {
            Some(i) if i < self.tabs.len() => gpui::div().size_full().child(self.tabs[i].clone()),
            _ => gpui::div().size_full(),
        };
        // Closing the window with several tabs asks first (`confirmCloseAllTabs`).
        if !self.close_hook_set {
            self.close_hook_set = true;
            let weak = cx.entity().downgrade();
            window.on_window_should_close(cx, move |_, cx| {
                weak.update(cx, |p, cx| {
                    if p.tabs.len() > 1 {
                        p.confirm_close = true;
                        cx.notify();
                        false
                    } else {
                        true
                    }
                })
                .unwrap_or(true)
            });
        }
        let mut layout = PageLayout::new(theme, titlebar, content);
        if self.confirm_close {
            let n = self.tabs.len();
            let button = |id: &'static str,
                          label: &'static str,
                          primary: bool,
                          close: bool,
                          cx: &mut Context<Self>| {
                gpui::div()
                    .id(id)
                    .px(gpui::px(16.))
                    .py(gpui::px(6.))
                    .rounded(gpui::px(4.))
                    .bg(gpui::rgb(if primary { 0x0078D4 } else { 0x3A3A3A }))
                    .child(label)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if close {
                            this.close_window_now(window, cx);
                        } else {
                            this.confirm_close = false;
                        }
                        cx.notify();
                    }))
            };
            layout = layout.overlay(
                gpui::div()
                    .size_full()
                    .flex()
                    .justify_center()
                    .items_center()
                    .bg(gpui::rgba(0x00000099))
                    .child(
                        gpui::div()
                            .w(gpui::px(440.))
                            .flex()
                            .flex_col()
                            .gap(gpui::px(12.))
                            .p(gpui::px(20.))
                            .rounded(gpui::px(8.))
                            .border_1()
                            .border_color(theme.stroke)
                            .bg(theme.page_bg)
                            .text_color(theme.fg)
                            .text_size(gpui::px(13.))
                            .child(gpui::div().text_size(gpui::px(16.)).child("Close all tabs?"))
                            .child(gpui::div().child(format!("This window has {n} tabs. Closing it ends every shell running in them.")))
                            .child(
                                gpui::div()
                                    .flex()
                                    .gap(gpui::px(8.))
                                    .justify_end()
                                    .child(button("confirm-close", "Close all", true, true, cx))
                                    .child(button("confirm-cancel", "Cancel", false, false, cx)),
                            ),
                    ),
            );
        }
        // `CloseOnExitInfoBar`: the active pane's shell exited with an error.
        #[cfg(not(feature = "stub-downstream"))]
        if let Some(i) = self.active_tab_index.filter(|&i| i < self.tabs.len()) {
            let tab = self.tabs[i].clone();
            let control = tab.read(cx).active_control().clone();
            let id = control.entity_id();
            if control.read(cx).has_failed() && !self.infobar_dismissed.contains(&id) {
                let button = |label: &'static str, close: bool, cx: &mut Context<Self>| {
                    let tab = tab.clone();
                    gpui::div()
                        .id(label)
                        .px(gpui::px(12.))
                        .py(gpui::px(3.))
                        .rounded(gpui::px(4.))
                        .bg(gpui::rgb(0x3A3A3A))
                        .hover(|s| s.bg(gpui::rgb(0x4A4A4A)))
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if close {
                                this.close_exited(vec![(tab.clone(), id)], cx);
                            } else {
                                this.infobar_dismissed.push(id);
                            }
                            cx.notify();
                        }))
                };
                layout = layout.infobar(
                    gpui::div()
                        .flex()
                        .items_center()
                        .gap(gpui::px(12.))
                        .px(gpui::px(12.))
                        .py(gpui::px(6.))
                        .border_b_1()
                        .border_color(theme.stroke)
                        .bg(gpui::rgb(0x2D2D2D))
                        .text_color(theme.fg)
                        .text_size(gpui::px(13.))
                        .child(
                            gpui::div()
                                .flex_1()
                                .child("A process in this pane exited with an error."),
                        )
                        .child(button("Close pane", true, cx))
                        .child(button("Dismiss", false, cx)),
                );
            }
        }
        if let Some(p) = &self.palette {
            let page = cx.entity();
            layout = layout.overlay(PalettePanel::new(
                p,
                theme,
                PaletteCallbacks {
                    on_pick: Rc::new(move |i, _, cx| {
                        page.update(cx, |pg, cx| {
                            if let Some(p) = pg.palette.as_mut() {
                                p.move_selection(i as isize - p.selected() as isize);
                                let action = p.selected_action().copied();
                                pg.palette = None;
                                if let Some(a) = action {
                                    pg.run_action(a, cx);
                                }
                            }
                        })
                    }),
                },
            ));
        }
        let focus = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        // Focus: palette open → page (it handles typing); otherwise the active
        // terminal so keystrokes reach the shell.
        let want = match (
            self.palette.is_some() || self.renaming.is_some() || self.confirm_close,
            self.active_tab_index,
        ) {
            (false, Some(i)) if i < self.tabs.len() => self.tabs[i]
                .read(cx)
                .active_control()
                .read(cx)
                .focus_handle
                .clone(),
            _ => focus.clone(),
        };
        if !want.is_focused(window) {
            window.focus(&want, cx);
        }
        let entity = cx.entity();
        let palette_open = self.palette.is_some() || self.renaming.is_some();
        let input_focus = focus.clone();
        gpui::div()
            .size_full()
            .track_focus(&focus)
            .on_key_down(
                cx.listener(|this, ev: &KeyDownEvent, window, cx| this.on_key_down(ev, window, cx)),
            )
            // A drag ends wherever the button is released; any click closes the menu.
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, ev: &gpui::MouseUpEvent, window, cx| {
                    this.finish_drag(ev.position, window, cx)
                }),
            )
            .when_some(self.tab_menu, |d, (i, pos)| {
                d.child(crate::ui::tab_strip::tab_context_menu(
                    theme,
                    i,
                    pos,
                    tab_pick.clone(),
                ))
            })
            // Click anywhere else closes an open menu (menus draw above this).
            .when(self.tab_menu.is_some(), |d| {
                d.child(
                    gpui::deferred(
                        gpui::div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.tab_menu = None;
                                    cx.notify();
                                }),
                            )
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(|this, _, _, cx| {
                                    this.tab_menu = None;
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(5),
                )
            })
            // While the palette is open the page owns text input (IME, Unicode).
            .when(palette_open, |d| {
                d.child(
                    gpui::canvas(
                        |b, _, _| b,
                        move |b, _, window, cx| {
                            window.handle_input(
                                &input_focus,
                                gpui::ElementInputHandler::new(b, entity.clone()),
                                cx,
                            )
                        },
                    )
                    .absolute()
                    .size_full(),
                )
            })
            .child(layout)
    }
}

/// Text typed into the command palette arrives here (IME commits, Unicode
/// input that has no key-down event).
impl gpui::EntityInputHandler for TerminalPage {
    fn text_for_range(
        &mut self,
        _range: std::ops::Range<usize>,
        _adjusted: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<gpui::UTF16Selection> {
        Some(gpui::UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if text.chars().any(char::is_control) {
            return;
        }
        if let Some((_, buf)) = self.renaming.as_mut() {
            buf.push_str(text);
            cx.notify();
        } else if let Some(p) = self.palette.as_mut() {
            let next = format!("{}{text}", p.search());
            p.set_search(&next);
            cx.notify();
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        _new_text: &str,
        _new_selected_range: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        element_bounds: gpui::Bounds<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }

    fn accepts_text_input(&self, _window: &mut Window, _cx: &mut Context<Self>) -> bool {
        self.palette.is_some() || self.renaming.is_some()
    }
}

/// `ExpandEnvironmentStringsW` equivalent: replace `%NAME%` with the process
/// environment value (case-insensitive on Windows); unknown names are kept.
fn expand_env(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => match std::env::var(&after[..end]) {
                Ok(v) => {
                    out.push_str(&v);
                    rest = &after[end + 1..];
                }
                Err(_) => {
                    out.push('%');
                    rest = after;
                }
            },
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}
/// Build `ConptySettings` từ `Profile` — port
/// `ConptyConnection::CreateSettings` (`ConptyConnection.cpp:206-243`) +
/// `_CreateConnectionFromSettings` aggregation (`.cpp:1618-1627`).
///
/// WT flow:
/// - `commandline` ← `profile.commandline` / `settings.Commandline()`
/// - `startingDirectory` ← `_evaluatePathForCwd(settings.StartingDirectory())`
///   (path resolution skip phase 2 — verbatim)
/// - `startingTitle` ← `settings.StartingTitle()`
/// - `initialRows`/`initialCols` ← `globals.initial_rows`/`initial_cols`
/// - `environment` ← profile.environment_variables (override)
/// - `flags` ← `PseudoConsoleFlags::empty()` (no inherit cursor)
/// - `session_id` ← `Uuid::new_v4()`
/// - `profile_guid` ← profile.guid parsed
/// - `inherit_cursor` ← false (caller — `inheritCursor` param phase 4)
/// - `text_measurement` ← 0 (foreground-default)
/// - `ambiguous_is_wide` ← false (foreground-default)
fn build_conpty_settings(profile: &Profile) -> ConptySettings {
    let session_id = uuid::Uuid::new_v4();
    let profile_guid =
        uuid::Uuid::parse_str(profile.guid.trim_matches(|c: char| c == '{' || c == '}')).ok();
    let environment: HashMap<String, String> =
        profile.environment_variables.clone().unwrap_or_default();
    ConptySettings {
        commandline: expand_env(&profile.commandline),
        starting_directory: profile
            .starting_directory
            .as_deref()
            .map(expand_env)
            .unwrap_or_default(),
        starting_title: profile.tab_title.clone(),
        initial_rows: 25, // overridden from `globals` by the caller
        initial_cols: 80, // overridden from `globals` by the caller
        environment,
        flags: PseudoConsoleFlags::empty(),
        session_id,
        profile_guid,
        inherit_cursor: false,
        text_measurement: 0,
        ambiguous_is_wide: false,
        reload_env: true, // WT default (`ConptyConnection.cpp:298`)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `build_conpty_settings` — parity `ConptyConnection::CreateSettings`.
    /// PowerShell inbox profile → expected field mapping.
    #[test]
    fn build_conpty_settings_from_powershell_profile() {
        let p = Profile::default_powershell();
        let s = build_conpty_settings(&p);
        assert!(!s.commandline.is_empty());
        assert!(s.profile_guid.is_some());
        // session_id fresh (v4).
        assert_eq!(s.session_id.get_version(), Some(uuid::Version::Random));
        assert!(s.flags.is_empty());
        assert!(!s.inherit_cursor);
        assert_eq!(s.text_measurement, 0);
        assert!(!s.ambiguous_is_wide);
    }

    #[test]
    fn expand_env_replaces_known_and_keeps_unknown() {
        std::env::set_var("TG_TEST_VAR", "abc");
        assert_eq!(expand_env("%TG_TEST_VAR%/x"), "abc/x");
        assert_eq!(expand_env("%NO_SUCH_TG_VAR%/y"), "%NO_SUCH_TG_VAR%/y");
        assert_eq!(expand_env("50% done"), "50% done");
    }

    /// Default profile fallback when index out-of-range.
    #[test]
    fn default_profile_fallback_powershell() {
        let settings = CascadiaSettings {
            profiles: vec![Profile::default_powershell()],
            default_profile_guid: uuid::Uuid::parse_str("61c54bbd-c2c6-5271-96e7-009a87ff44bf")
                .unwrap(),
            globals: GlobalAppSettings::default(),
        };
        let page = TerminalPage {
            tabs: vec![],
            settings,
            active_tab_index: None,
            palette: None,
            focus_handle: None,
            drag_tab: None,
            tab_menu: None,
            renaming: None,
            infobar_dismissed: Vec::new(),
            confirm_close: false,
            close_hook_set: false,
        };
        let p = page._default_profile();
        assert_eq!(p.guid, "{61c54bbd-c2c6-5271-96e7-009a87ff44bf}");
    }

    /// `handle_close_tab_action` + `handle_new_tab_action` — parity
    /// `TerminalPage.cpp:_OpenNewTerminalViaDropdown` (`:1451`) +
    /// `_HandleCloseTabRequested` (`:3737`). Phase 3 port (stub-downstream):
    /// build qua TestApp (constructor đã open 1 tab default profile).
    ///
    /// Chỉ chạy stub-downstream lane: default lane `_make_pane` rơi vào
    /// `todo!()` (phase 4 wire connection→TermControl::new) — không fit cho
    /// unit test. Default lane verify qua `cargo check --workspace`.
    #[cfg(feature = "stub-downstream")]
    #[test]
    fn handle_new_and_close_tab_actions_indexes() {
        use gpui::TestApp;
        let mut app = TestApp::new();
        let settings = CascadiaSettings {
            profiles: vec![Profile::default_powershell()],
            default_profile_guid: uuid::Uuid::parse_str("61c54bbd-c2c6-5271-96e7-009a87ff44bf")
                .unwrap(),
            globals: GlobalAppSettings::default(),
        };
        let mut page = app.update(|cx| TerminalPage::new(settings, cx));
        assert_eq!(page.tabs.len(), 1);
        assert_eq!(page.active_tab_index, Some(0));
        // Open 2 more → 3 tabs. MED-2 regression: mỗi `handle_new_tab_action`
        // phải switch `active_tab_index` sang tab mới (WT `TabManagement.cpp:213`).
        // Trước fix: chỉ set khi `is_none()` → active stuck ở 0 → bug.
        app.update(|cx| page.handle_new_tab_action(cx));
        assert_eq!(page.tabs.len(), 2);
        assert_eq!(
            page.active_tab_index,
            Some(1),
            "MED-2: open new tab must switch active"
        );
        app.update(|cx| page.handle_new_tab_action(cx));
        assert_eq!(page.tabs.len(), 3);
        assert_eq!(
            page.active_tab_index,
            Some(2),
            "MED-2: open new tab must switch active"
        );
        // Close active (0) → active fall back last (index 1 of remaining).
        app.update(|cx| page.handle_close_tab_action(0, cx));
        assert_eq!(page.tabs.len(), 2);
        assert_eq!(page.active_tab_index, Some(1));
        // Out-of-range ignored.
        app.update(|cx| page.handle_close_tab_action(99, cx));
        assert_eq!(page.tabs.len(), 2);
    }
}
