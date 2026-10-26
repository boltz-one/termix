//! `Pane` — port của `TerminalApp/Pane.h/.cpp`. Cây pane (leaf = một
//! `TermControl`, split = hai con + hướng + %) nằm ở `pane_tree` (thuần, có
//! test); file này gắn GPUI: render cây, viền pane active, thanh chia kéo được.

use std::rc::Rc;

use gpui::{
    div, px, relative, rgb, AnyElement, App, Entity, EntityId, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Styled, Window,
};

use crate::pane_tree::{Leaf, Node, PaneTree};
use crate::tab::SplitDirection;

/// `TermControl` từ `downstream` — Lane B `control::TermControl` đã finalize.
pub type TermControl = crate::downstream::TermControl;

impl Leaf for Entity<TermControl> {
    type Id = EntityId;
    fn id(&self) -> EntityId {
        self.entity_id()
    }
}

/// `Pane` — `Pane.h:24`: tree of terminal controls.
pub type Pane = PaneTree<Entity<TermControl>>;

/// Called when the user clicks a pane: makes it the tab's active pane.
pub type ActivateCb = Rc<dyn Fn(EntityId, &mut Window, &mut App)>;
/// Called when a divider is pressed; the path locates the split in the tree.
pub type DragStartCb = Rc<dyn Fn(Vec<bool>, &mut Window, &mut App)>;

/// Colour of the active-pane border when a tab has several panes.
const ACTIVE_BORDER: u32 = 0x0078D4;
/// Width of the invisible drag handle straddling the boundary between panes.
const DIVIDER_HIT: f32 = 4.0;
/// Space between a pane's border and its text.
const PANE_PADDING: f32 = 5.0;
/// Border + padding around the terminal in a pane (per side).
pub(crate) const PANE_INSET: f32 = PANE_PADDING + 1.0;
const PANE_BG: u32 = 0x0C0C0C;

impl Pane {
    /// Test/inspect helper — true khi leaf (chưa split).
    pub fn is_leaf(&self) -> bool {
        matches!(self.root, Node::Leaf(_))
    }

    /// Test/inspect helper — `Some(dir)` khi split, `None` khi leaf.
    pub fn split_direction(&self) -> Option<SplitDirection> {
        match &self.root {
            Node::Split { direction, .. } => Some(*direction),
            _ => None,
        }
    }

    /// Leftmost leaf control (`Pane::_FindFirstLeaf`).
    pub fn first_control(&self) -> &Entity<TermControl> {
        self.first_leaf()
    }

    /// Split the first leaf (kept for callers that don't track an active pane).
    pub fn split(&mut self, direction: SplitDirection, control: Entity<TermControl>) {
        let id = self.first_leaf().entity_id();
        self.split_leaf(id, direction, control);
    }

    /// Build the element tree. `active` gets the accent border when there is
    /// more than one pane; clicking a pane calls `on_activate`, pressing a
    /// divider calls `on_drag`.
    pub fn element(
        &self,
        active: EntityId,
        on_activate: &ActivateCb,
        on_drag: &DragStartCb,
    ) -> AnyElement {
        let multi = !self.is_leaf();
        node_element(
            &self.root,
            &mut Vec::new(),
            active,
            multi,
            on_activate,
            on_drag,
        )
    }

    /// Zoomed view: only the active pane, filling the tab (`TogglePaneZoom`).
    pub fn zoomed_element(&self, active: EntityId, on_activate: &ActivateCb) -> AnyElement {
        let control = self.find(active).unwrap_or_else(|| self.first_leaf());
        leaf_element(control, active, false, on_activate)
    }
}

fn leaf_element(
    control: &Entity<TermControl>,
    active: EntityId,
    multi: bool,
    cb: &ActivateCb,
) -> AnyElement {
    let id = control.entity_id();
    let cb = cb.clone();
    let border = if multi && id == active {
        rgb(ACTIVE_BORDER)
    } else {
        rgb(PANE_BG)
    };
    div()
        .size_full()
        .flex_1()
        .min_w(px(0.))
        .min_h(px(0.))
        .border_1()
        .border_color(border)
        .p(px(PANE_PADDING))
        .bg(rgb(PANE_BG))
        .on_mouse_down(MouseButton::Left, move |_, w, cx| cb(id, w, cx))
        .child(control.clone())
        .into_any_element()
}

fn node_element(
    node: &Node<Entity<TermControl>>,
    path: &mut Vec<bool>,
    active: EntityId,
    multi: bool,
    cb: &ActivateCb,
    drag: &DragStartCb,
) -> AnyElement {
    match node {
        Node::Leaf(control) => leaf_element(control, active, multi, cb),
        Node::Split {
            direction,
            pct,
            first,
            second,
        } => {
            path.push(false);
            let a = node_element(first, path, active, multi, cb, drag);
            path.pop();
            path.push(true);
            let b = node_element(second, path, active, multi, cb, drag);
            path.pop();

            let vertical = *direction == SplitDirection::Vertical;
            let p = relative(*pct as f32 / 100.0);
            let key = path.iter().fold(1usize, |acc, &s| acc * 2 + s as usize);
            let drag_path = path.clone();
            let drag = drag.clone();
            // Panes are flush; the divider is an invisible handle over the seam.
            let handle = div().id(("pane-divider", key)).absolute().on_mouse_down(
                MouseButton::Left,
                move |_, w, cx| {
                    cx.stop_propagation();
                    drag(drag_path.clone(), w, cx);
                },
            );
            let container = div().relative().size_full().flex();
            if vertical {
                container
                    .flex_row()
                    .child(div().w(p).h_full().flex_none().child(a))
                    .child(div().h_full().flex_1().min_w(px(0.)).child(b))
                    .child(
                        handle
                            .top_0()
                            .h_full()
                            .w(px(DIVIDER_HIT))
                            .left(p)
                            .ml(px(-DIVIDER_HIT / 2.))
                            .cursor_col_resize(),
                    )
            } else {
                container
                    .flex_col()
                    .child(div().h(p).w_full().flex_none().child(a))
                    .child(div().w_full().flex_1().min_h(px(0.)).child(b))
                    .child(
                        handle
                            .left_0()
                            .w_full()
                            .h(px(DIVIDER_HIT))
                            .top(p)
                            .mt(px(-DIVIDER_HIT / 2.))
                            .cursor_row_resize(),
                    )
            }
            .into_any_element()
        }
        Node::Empty => div().into_any_element(),
    }
}

/// Pane tree-shape tests — chỉ chạy stub-downstream lane.
#[cfg(all(test, feature = "stub-downstream"))]
mod tests {
    use gpui::TestApp;

    #[test]
    fn pane_new_leaf_unit() {
        let mut app = TestApp::new();
        let ctrl = app.new_entity(|_| crate::downstream::TermControl);
        let pane = super::Pane::new(ctrl);
        assert!(pane.is_leaf());
    }

    #[test]
    fn pane_split_creates_two_children() {
        let mut app = TestApp::new();
        let c1 = app.new_entity(|_| crate::downstream::TermControl);
        let mut pane = super::Pane::new(c1);
        let c2 = app.new_entity(|_| crate::downstream::TermControl);
        pane.split(crate::tab::SplitDirection::Vertical, c2);
        assert!(!pane.is_leaf());
        assert_eq!(
            pane.split_direction(),
            Some(crate::tab::SplitDirection::Vertical)
        );
    }
}
