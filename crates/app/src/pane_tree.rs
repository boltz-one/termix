//! `PaneTree` — pure pane tree logic (split / close / resize / directional
//! focus), independent of GPUI so it is unit-testable. Port of the tree part
//! of `Pane.cpp` (`_Split`, `_CloseChild`, `ResizePane`, `NavigateFocus`).

use crate::tab::SplitDirection;

/// Leaf payload with a stable identity.
pub trait Leaf {
    type Id: Copy + PartialEq;
    fn id(&self) -> Self::Id;
}

/// Directions for focus movement / resizing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Pane rectangle in the unit square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const UNIT: Rect = Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    fn right(&self) -> f32 {
        self.x + self.w
    }
    fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

pub enum Node<L> {
    Leaf(L),
    /// `Vertical` = side by side (vertical divider), `Horizontal` = stacked.
    Split { direction: SplitDirection, pct: u8, first: Box<Node<L>>, second: Box<Node<L>> },
    /// Transient value while a node is being moved; never observable.
    Empty,
}

pub struct PaneTree<L> {
    pub root: Node<L>,
}

const MIN_PCT: i16 = 10;
const MAX_PCT: i16 = 90;
/// Divider step per resize action (WT `resizePane` moves 5%).
pub const RESIZE_STEP: i16 = 5;
const EPS: f32 = 1e-4;

fn split_rects(direction: SplitDirection, pct: u8, r: Rect) -> (Rect, Rect) {
    let p = pct as f32 / 100.0;
    match direction {
        SplitDirection::Vertical => (Rect { w: r.w * p, ..r }, Rect { x: r.x + r.w * p, w: r.w * (1.0 - p), ..r }),
        _ => (Rect { h: r.h * p, ..r }, Rect { y: r.y + r.h * p, h: r.h * (1.0 - p), ..r }),
    }
}

impl<L: Leaf> Node<L> {
    fn collect<'a>(&'a self, out: &mut Vec<&'a L>) {
        match self {
            Node::Leaf(l) => out.push(l),
            Node::Split { first, second, .. } => {
                first.collect(out);
                second.collect(out);
            }
            Node::Empty => {}
        }
    }

    fn layout(&self, r: Rect, out: &mut Vec<(L::Id, Rect)>) {
        match self {
            Node::Leaf(l) => out.push((l.id(), r)),
            Node::Split { direction, pct, first, second } => {
                let (a, b) = split_rects(*direction, *pct, r);
                first.layout(a, out);
                second.layout(b, out);
            }
            Node::Empty => {}
        }
    }

    /// Rect of the node at `path` (false = first child, true = second).
    fn rect_at(&self, path: &[bool], r: Rect) -> Option<Rect> {
        let Some((&step, rest)) = path.split_first() else { return Some(r) };
        let Node::Split { direction, pct, first, second } = self else { return None };
        let (a, b) = split_rects(*direction, *pct, r);
        if step { second.rect_at(rest, b) } else { first.rect_at(rest, a) }
    }

    fn split_at_mut(&mut self, path: &[bool]) -> Option<(&mut SplitDirection, &mut u8)> {
        match (path.split_first(), self) {
            (None, Node::Split { direction, pct, .. }) => Some((direction, pct)),
            (Some((&step, rest)), Node::Split { first, second, .. }) => {
                if step { second.split_at_mut(rest) } else { first.split_at_mut(rest) }
            }
            _ => None,
        }
    }

    fn set_nth(&mut self, n: &mut usize, leaf: &mut Option<L>) {
        match self {
            Node::Leaf(l) => {
                if *n == 0 {
                    if let Some(new) = leaf.take() {
                        *l = new;
                    }
                }
                *n = n.wrapping_sub(1);
            }
            Node::Split { first, second, .. } => {
                first.set_nth(n, leaf);
                second.set_nth(n, leaf);
            }
            Node::Empty => {}
        }
    }

    fn has(&self, id: L::Id) -> bool {
        match self {
            Node::Leaf(l) => l.id() == id,
            Node::Split { first, second, .. } => first.has(id) || second.has(id),
            Node::Empty => false,
        }
    }

    fn split_leaf(&mut self, id: L::Id, direction: SplitDirection, new: &mut Option<L>) -> bool {
        match self {
            Node::Leaf(l) if l.id() == id => {
                let Some(new_leaf) = new.take() else { return false };
                let old = std::mem::replace(self, Node::Empty);
                *self = Node::Split {
                    direction,
                    pct: 50,
                    first: Box::new(old),
                    second: Box::new(Node::Leaf(new_leaf)),
                };
                true
            }
            Node::Split { first, second, .. } => {
                first.split_leaf(id, direction, new) || second.split_leaf(id, direction, new)
            }
            _ => false,
        }
    }

    fn close_leaf(&mut self, id: L::Id) -> bool {
        let Node::Split { first, second, .. } = self else { return false };
        let is_leaf = |n: &Node<L>| matches!(n, Node::Leaf(l) if l.id() == id);
        let sibling = if is_leaf(first) {
            std::mem::replace(&mut **second, Node::Empty)
        } else if is_leaf(second) {
            std::mem::replace(&mut **first, Node::Empty)
        } else {
            return first.close_leaf(id) || second.close_leaf(id);
        };
        *self = sibling;
        true
    }

    /// Move the nearest matching-axis divider above `id` by `delta` percent
    /// in `dir` (Right/Down grow the first child).
    fn resize(&mut self, id: L::Id, dir: Direction, delta: i16) -> bool {
        let Node::Split { direction, pct, first, second } = self else { return false };
        if (first.has(id) && first.resize(id, dir, delta)) || (second.has(id) && second.resize(id, dir, delta)) {
            return true;
        }
        let axis_matches = match direction {
            SplitDirection::Vertical => matches!(dir, Direction::Left | Direction::Right),
            _ => matches!(dir, Direction::Up | Direction::Down),
        };
        if !axis_matches || !(first.has(id) || second.has(id)) {
            return false;
        }
        let signed = match dir {
            Direction::Right | Direction::Down => delta,
            Direction::Left | Direction::Up => -delta,
        };
        *pct = (*pct as i16 + signed).clamp(MIN_PCT, MAX_PCT) as u8;
        true
    }
}

impl<L: Leaf + Clone> PaneTree<L> {
    /// Exchange the positions of leaves `a` and `b`.
    pub fn swap(&mut self, a: L::Id, b: L::Id) -> bool {
        let ids: Vec<L::Id> = self.leaves().iter().map(|l| l.id()).collect();
        let (Some(i), Some(j)) = (ids.iter().position(|x| *x == a), ids.iter().position(|x| *x == b)) else {
            return false;
        };
        let (la, lb) = (self.leaves()[i].clone(), self.leaves()[j].clone());
        self.root.set_nth(&mut { i }, &mut Some(lb));
        self.root.set_nth(&mut { j }, &mut Some(la));
        true
    }
}

impl<L: Leaf> PaneTree<L> {
    pub fn new(leaf: L) -> Self {
        Self { root: Node::Leaf(leaf) }
    }

    pub fn leaves(&self) -> Vec<&L> {
        let mut v = Vec::new();
        self.root.collect(&mut v);
        v
    }

    pub fn first_leaf(&self) -> &L {
        self.leaves()[0]
    }

    pub fn find(&self, id: L::Id) -> Option<&L> {
        self.leaves().into_iter().find(|l| l.id() == id)
    }

    pub fn layout(&self) -> Vec<(L::Id, Rect)> {
        let mut v = Vec::new();
        self.root.layout(Rect::UNIT, &mut v);
        v
    }

    /// Split leaf `id`, placing `new` second (right / below). False if `id`
    /// is not in the tree.
    pub fn split_leaf(&mut self, id: L::Id, direction: SplitDirection, new: L) -> bool {
        self.root.split_leaf(id, direction, &mut Some(new))
    }

    /// Remove leaf `id`; its sibling takes the space. Refuses to remove the
    /// only remaining leaf (caller closes the tab instead).
    pub fn close_leaf(&mut self, id: L::Id) -> bool {
        self.root.close_leaf(id)
    }

    /// Rect (unit square) of the split node at `path`, for divider dragging.
    pub fn rect_at(&self, path: &[bool]) -> Option<Rect> {
        self.root.rect_at(path, Rect::UNIT)
    }

    /// Direction of the split at `path`.
    pub fn direction_at(&mut self, path: &[bool]) -> Option<SplitDirection> {
        self.root.split_at_mut(path).map(|(d, _)| *d)
    }

    /// Set the divider of the split at `path` (clamped to 10–90%).
    pub fn set_pct_at(&mut self, path: &[bool], pct: f32) -> bool {
        match self.root.split_at_mut(path) {
            Some((_, p)) => {
                *p = pct.round().clamp(MIN_PCT as f32, MAX_PCT as f32) as u8;
                true
            }
            None => false,
        }
    }

    pub fn resize(&mut self, id: L::Id, dir: Direction) -> bool {
        self.root.resize(id, dir, RESIZE_STEP)
    }

    /// Nearest pane in `dir` from `from` (WT `NavigateFocus` directional).
    pub fn neighbour(&self, from: L::Id, dir: Direction) -> Option<L::Id> {
        let layout = self.layout();
        let (_, r) = *layout.iter().find(|(id, _)| *id == from)?;
        let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| (a1.min(b1) - a0.max(b0)).max(0.0);
        layout
            .iter()
            .filter(|(id, _)| *id != from)
            .filter_map(|(id, o)| {
                let (dist, ov) = match dir {
                    Direction::Left => (r.x - o.right(), overlap(r.y, r.bottom(), o.y, o.bottom())),
                    Direction::Right => (o.x - r.right(), overlap(r.y, r.bottom(), o.y, o.bottom())),
                    Direction::Up => (r.y - o.bottom(), overlap(r.x, r.right(), o.x, o.right())),
                    Direction::Down => (o.y - r.bottom(), overlap(r.x, r.right(), o.x, o.right())),
                };
                (dist > -EPS && ov > EPS).then_some((*id, dist, ov))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(b.2.partial_cmp(&a.2).unwrap()))
            .map(|(id, ..)| id)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    impl Leaf for u32 {
        type Id = u32;
        fn id(&self) -> u32 {
            *self
        }
    }

    fn ids(t: &PaneTree<u32>) -> Vec<u32> {
        t.leaves().into_iter().copied().collect()
    }

    /// 1 | (2 over 3): split 1 right → 2, split 2 down → 3.
    fn three() -> PaneTree<u32> {
        let mut t = PaneTree::new(1);
        assert!(t.split_leaf(1, SplitDirection::Vertical, 2));
        assert!(t.split_leaf(2, SplitDirection::Horizontal, 3));
        t
    }

    #[test]
    fn wt_app__pane_split_orders_leaves_and_unknown_id_fails() {
        let mut t = three();
        assert_eq!(ids(&t), [1, 2, 3]);
        assert!(!t.split_leaf(99, SplitDirection::Vertical, 4));
        assert_eq!(ids(&t), [1, 2, 3]);
    }

    #[test]
    fn wt_app__pane_close_gives_space_to_sibling() {
        let mut t = three();
        assert!(t.close_leaf(3));
        assert_eq!(t.layout().iter().map(|(i, r)| (*i, r.w)).collect::<Vec<_>>(), vec![(1, 0.5), (2, 0.5)]);
        assert!(t.close_leaf(1));
        assert_eq!(ids(&t), [2]);
        assert!(!t.close_leaf(2), "last pane cannot be closed by the tree");
    }

    #[test]
    fn wt_app__pane_directional_focus() {
        let t = three();
        assert_eq!(t.neighbour(1, Direction::Right), Some(2));
        assert_eq!(t.neighbour(3, Direction::Left), Some(1));
        assert_eq!(t.neighbour(2, Direction::Down), Some(3));
        assert_eq!(t.neighbour(3, Direction::Up), Some(2));
        assert_eq!(t.neighbour(1, Direction::Left), None);
        assert_eq!(t.neighbour(1, Direction::Down), None);
    }

    #[test]
    fn wt_app__pane_swap_and_divider_drag() {
        impl Leaf for (u32, u8) {
            type Id = u32;
            fn id(&self) -> u32 {
                self.0
            }
        }
        let mut t = PaneTree::new((1u32, 0u8));
        t.split_leaf(1, SplitDirection::Vertical, (2, 0));
        t.split_leaf(2, SplitDirection::Horizontal, (3, 0));
        assert!(t.swap(1, 3));
        let order: Vec<u32> = t.leaves().iter().map(|l| l.0).collect();
        assert_eq!(order, [3, 2, 1]);
        assert!(!t.swap(1, 99));
        // Drag the root divider (path []) to 30%, and the nested one ([true]) to 70%.
        assert!(t.set_pct_at(&[], 30.0));
        assert_eq!(t.rect_at(&[true]).unwrap().x, 0.3);
        assert!(t.set_pct_at(&[true], 70.0));
        assert!(t.set_pct_at(&[], 3.0)); // clamped
        assert_eq!(t.layout()[0].1.w, 0.1);
        assert_eq!(t.direction_at(&[true]), Some(SplitDirection::Horizontal));
        assert!(!t.set_pct_at(&[false], 50.0), "a leaf is not a split");
    }

    #[test]
    fn wt_app__pane_resize_moves_matching_divider_with_clamp() {
        let mut t = three();
        assert!(t.resize(1, Direction::Right));
        assert_eq!(t.layout()[0].1.w, 0.55);
        assert!(t.resize(3, Direction::Up)); // horizontal split 2/3 → 2 shrinks
        assert!(t.layout()[1].1.h < 0.5);
        assert!(!t.resize(1, Direction::Down), "no horizontal divider above pane 1");
        for _ in 0..20 {
            t.resize(1, Direction::Left);
        }
        assert_eq!(t.layout()[0].1.w, 0.1);
    }
}
