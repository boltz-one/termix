//! `Tab` — port của `TerminalApp/Tab.h/.cpp`. Sở hữu root `Pane`, title, và
//! thao tác split/close. Phase 2: ctor + render delegate. `split`/`close`
//! placeholder (cần `TerminalPage::_MakePane`).

use std::cell::Cell;
use std::rc::Rc;

use crate::downstream::Profile;
use crate::pane::{ActivateCb, DragStartCb, Pane, TermControl};
use crate::pane_tree::Direction;
use gpui::{
    canvas, div, App, Bounds, Context, Entity, EntityId, FocusHandle, InteractiveElement,
    IntoElement, MouseButton, MouseMoveEvent, ParentElement, Pixels, Point, Styled, Window,
};

/// `SplitDirection` — `Tab.h:28` / `TerminalPage.cpp:SplitPane`. Auto = WT tự
/// chọn dọc/ngang theo tỉ lệ khung nhìn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDirection {
    /// Dọc — vách ngăn đứng, hai pane kế trái/phải (`SplitState::Vertical`).
    Vertical,
    /// Ngang — vách nằm, hai pane trên/dưới (`SplitState::Horizontal`).
    Horizontal,
    /// Tự chọn — `Pane::_CalculateAutoSplit` (`Pane.cpp`).
    Auto,
}

/// `Tab` — `Tab.h:24`. Root `Pane` + tab title + the active pane.
#[allow(dead_code)] // `focus_handle` reserved for tab-level focus.
pub struct Tab {
    /// Root pane tree của tab. Single-pane khi vừa mở (`Tab.cpp:30`).
    root_pane: Pane,
    /// `Tab::_Title` (`Tab.h:96`). Bắt nguồn từ control title (`TermControl::Title`).
    title: String,
    /// Focus handle cho tab; deleg focus xuống leaf pane.
    focus_handle: FocusHandle,
    /// `_activePane` — the pane whose control receives keyboard input.
    active: EntityId,
    /// Profile this tab was opened with; new panes duplicate it.
    profile: Option<Profile>,
    /// Title set with `Rename tab`; wins over the shell's title.
    title_override: Option<String>,
    /// `_zoomedPane` — the active pane fills the tab while true.
    zoomed: bool,
    /// Split being dragged (path from the root), set on divider mouse-down.
    dragging: Option<Vec<bool>>,
    /// Pixel bounds of the pane area, refreshed every frame for drag maths.
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl Tab {
    /// `Tab.cpp:30` ctor — single-pane, title rỗng (sẽ set khi control raise Title).
    pub fn new(term_control: Entity<TermControl>, cx: &mut App) -> Self {
        let active = term_control.entity_id();
        Self {
            root_pane: Pane::new(term_control),
            title: String::new(),
            focus_handle: cx.focus_handle(),
            active,
            profile: None,
            title_override: None,
            zoomed: false,
            dragging: None,
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }

    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = Some(profile);
    }

    pub fn profile(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoomed
    }

    /// `TogglePaneZoom`: no-op for a single pane.
    pub fn toggle_zoom(&mut self) {
        self.zoomed = !self.zoomed && self.pane_count() > 1;
    }

    /// Every control of the tab, in pane order.
    pub fn controls(&self) -> Vec<Entity<TermControl>> {
        self.root_pane.leaves().into_iter().cloned().collect()
    }

    /// Remove pane `id` (e.g. its shell exited). False if it is the last one.
    pub fn close_pane(&mut self, id: EntityId) -> bool {
        let previous = self.active;
        self.active = id;
        if self.close_active_pane() {
            // Keep the user's pane focused unless it is the one that went away.
            if previous != id && self.root_pane.find(previous).is_some() {
                self.active = previous;
            }
            true
        } else {
            self.active = previous;
            false
        }
    }

    /// `SwapPane`: exchange the active pane with its neighbour in `dir`.
    pub fn swap_active(&mut self, dir: Direction) {
        if let Some(other) = self.root_pane.neighbour(self.active, dir) {
            self.root_pane.swap(self.active, other);
        }
    }

    fn drag_to(&mut self, position: Point<Pixels>) {
        let Some(path) = self.dragging.clone() else {
            return;
        };
        let b = self.bounds.get();
        let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let (Some(rect), Some(dir)) = (
            self.root_pane.rect_at(&path),
            self.root_pane.direction_at(&path),
        ) else {
            return;
        };
        let fx = f32::from(position.x - b.origin.x) / w;
        let fy = f32::from(position.y - b.origin.y) / h;
        let pct = match dir {
            SplitDirection::Vertical => (fx - rect.x) / rect.w,
            _ => (fy - rect.y) / rect.h,
        } * 100.0;
        self.root_pane.set_pct_at(&path, pct);
    }

    /// Control of the active pane (falls back to the first leaf).
    pub fn active_control(&self) -> &Entity<TermControl> {
        self.root_pane
            .find(self.active)
            .unwrap_or_else(|| self.root_pane.first_control())
    }

    /// User-chosen title (`Rename tab`); empty clears it.
    pub fn set_title_override(&mut self, title: &str) {
        self.title_override = (!title.is_empty()).then(|| title.to_string());
    }

    pub fn title_override(&self) -> Option<&str> {
        self.title_override.as_deref()
    }

    /// Live title: the user's override, else the shell's OSC title, else the stored one.
    pub fn live_title(&self, cx: &App) -> String {
        if let Some(t) = &self.title_override {
            return t.clone();
        }
        #[cfg(not(feature = "stub-downstream"))]
        {
            let t = self
                .active_control()
                .read(cx)
                .interactivity
                .core
                .lock()
                .title();
            if !t.is_empty() {
                return t;
            }
        }
        let _ = cx;
        self.title.clone()
    }

    /// `Tab::_Title` accessor (`Tab.h:96`).
    pub fn title(&self) -> &str {
        &self.title
    }

    /// `Tab::SplitPane`: split the active pane; the new pane becomes active.
    pub fn split_pane(&mut self, direction: SplitDirection, control: Entity<TermControl>) {
        let new_id = control.entity_id();
        if self.root_pane.split_leaf(self.active, direction, control) {
            self.active = new_id;
            self.zoomed = false;
        }
    }

    /// `Tab::ClosePane`: remove the active pane. Returns false when it is the
    /// only pane, so the caller closes the tab instead.
    pub fn close_active_pane(&mut self) -> bool {
        let layout = self.root_pane.layout();
        let Some(i) = layout.iter().position(|(id, _)| *id == self.active) else {
            return false;
        };
        if !self.root_pane.close_leaf(self.active) {
            return false;
        }
        // Focus the pane that took over (previous in order, else first).
        let leaves = self.root_pane.leaves();
        self.active = leaves[i.saturating_sub(1).min(leaves.len() - 1)].entity_id();
        if leaves.len() < 2 {
            self.zoomed = false;
        }
        true
    }

    /// `Tab::NavigateFocus`: move the active pane in `dir` if there is one.
    pub fn move_focus(&mut self, dir: Direction) {
        if let Some(id) = self.root_pane.neighbour(self.active, dir) {
            self.active = id;
        }
    }

    /// `Tab::ResizePane`.
    pub fn resize_active(&mut self, dir: Direction) {
        self.root_pane.resize(self.active, dir);
    }

    pub fn set_active(&mut self, id: EntityId) {
        if self.root_pane.find(id).is_some() {
            self.active = id;
        }
    }

    pub fn pane_count(&self) -> usize {
        self.root_pane.leaves().len()
    }
}

impl gpui::Render for Tab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = cx.entity();
        let on_activate: ActivateCb = Rc::new(move |id, _, cx| {
            tab.update(cx, |t, cx| {
                if t.active != id {
                    t.set_active(id);
                    cx.notify();
                }
            })
        });
        let tab = cx.entity();
        let on_drag: DragStartCb = Rc::new(move |path, _, cx| {
            tab.update(cx, |t, _| t.dragging = Some(path));
        });
        let body = if self.zoomed && self.pane_count() > 1 {
            self.root_pane.zoomed_element(self.active, &on_activate)
        } else {
            self.root_pane.element(self.active, &on_activate, &on_drag)
        };
        let bounds = self.bounds.clone();
        div()
            .size_full()
            .relative()
            // Records the pane-area bounds so divider drags can map pixels to %.
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .child(body)
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if this.dragging.is_some() {
                    if ev.pressed_button == Some(MouseButton::Left) {
                        this.drag_to(ev.position);
                        cx.notify();
                    } else {
                        this.dragging = None;
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = None),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SplitDirection` parity — 3 variants như WT `SplitState`.
    #[test]
    fn split_direction_variants() {
        assert_ne!(SplitDirection::Vertical, SplitDirection::Horizontal);
        assert_ne!(SplitDirection::Vertical, SplitDirection::Auto);
        assert_ne!(SplitDirection::Horizontal, SplitDirection::Auto);
        assert_eq!(SplitDirection::Vertical, SplitDirection::Vertical);
    }

    /// `SplitDirection` copy + size stable.
    #[test]
    fn split_direction_is_copy() {
        let a = SplitDirection::Auto;
        let b = a;
        assert_eq!(a, b);
    }
}
