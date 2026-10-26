//! `Viewport` — port của `Microsoft::Console::Types::Viewport`
//! (`windows/src/types/viewport.hpp`). Renderer/control dùng để tính
//! clip rect cho paint.
//!
//! Phase 1: implement thật `left/top/right/bottom` + `width/height`/`intersect`/`contains` (simple int math).

/// Rect dạng inclusive `left..=right`, `top..=bottom` (khớp WT `Viewport`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Viewport {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Viewport {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn empty() -> Self {
        Self {
            left: 0,
            top: 0,
            right: -1,
            bottom: -1,
        }
    }

    pub fn from_size(width: i32, height: i32) -> Self {
        Self {
            left: 0,
            top: 0,
            right: width - 1,
            bottom: height - 1,
        }
    }

    /// Chiều rộng (cells). Empty viewport → 0.
    pub fn width(&self) -> i32 {
        if self.right < self.left {
            0
        } else {
            self.right - self.left + 1
        }
    }

    /// Chiều cao (rows). Empty viewport → 0.
    pub fn height(&self) -> i32 {
        if self.bottom < self.top {
            0
        } else {
            self.bottom - self.top + 1
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    /// Giao hai viewport — clamp `max(left) ..= min(right)`.
    pub fn intersect(&self, other: &Self) -> Self {
        let l = self.left.max(other.left);
        let r = self.right.min(other.right);
        let t = self.top.max(other.top);
        let b = self.bottom.min(other.bottom);
        if r < l || b < t {
            Self::empty()
        } else {
            Self::new(l, t, r, b)
        }
    }

    /// Hợp hai viewport — bbox bao cả hai.
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self::new(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// Kiểm tra (x, y) nằm trong viewport.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    /// Dịch viewport bởi (dx, dy).
    pub fn offset(&self, dx: i32, dy: i32) -> Self {
        Self::new(
            self.left + dx,
            self.top + dy,
            self.right + dx,
            self.bottom + dy,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_size_dimensions() {
        let v = Viewport::from_size(80, 24);
        assert_eq!(v.width(), 80);
        assert_eq!(v.height(), 24);
        assert_eq!((v.left, v.top, v.right, v.bottom), (0, 0, 79, 23));
    }

    #[test]
    fn empty_viewport_zero_dim() {
        let v = Viewport::empty();
        assert!(v.is_empty());
        assert_eq!(v.width(), 0);
        assert_eq!(v.height(), 0);
    }

    #[test]
    fn intersect_clamps_bounds() {
        let a = Viewport::new(0, 0, 79, 23);
        let b = Viewport::new(10, 5, 100, 50);
        let i = a.intersect(&b);
        assert_eq!((i.left, i.top, i.right, i.bottom), (10, 5, 79, 23));
    }

    #[test]
    fn disjoint_intersect_is_empty() {
        let a = Viewport::new(0, 0, 10, 10);
        let b = Viewport::new(20, 20, 30, 30);
        assert!(a.intersect(&b).is_empty());
    }

    #[test]
    fn contains_point() {
        let v = Viewport::new(5, 5, 15, 15);
        assert!(v.contains(10, 10));
        assert!(v.contains(5, 5));
        assert!(v.contains(15, 15));
        assert!(!v.contains(4, 4));
        assert!(!v.contains(16, 16));
    }

    #[test]
    fn union_bbox() {
        let a = Viewport::new(0, 0, 10, 10);
        let b = Viewport::new(20, 20, 30, 30);
        let u = a.union(&b);
        assert_eq!((u.left, u.top, u.right, u.bottom), (0, 0, 30, 30));
    }

    #[test]
    fn offset_translates_rect() {
        let v = Viewport::new(0, 0, 5, 5);
        let off = v.offset(10, -3);
        assert_eq!((off.left, off.top, off.right, off.bottom), (10, -3, 15, 2));
    }
}
