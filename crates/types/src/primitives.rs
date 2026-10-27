// Point, Size, Rect - geometric primitives
// Port: til/inc/point.h, til/inc/size.h, til/inc/rect.h

use std::ops::{Add, Sub};

/// 2D point with checked arithmetic
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self {
            x: self.x.checked_add(rhs.x)?,
            y: self.y.checked_add(rhs.y)?,
        })
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        Some(Self {
            x: self.x.checked_sub(rhs.x)?,
            y: self.y.checked_sub(rhs.y)?,
        })
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self {
            x: self.x.saturating_add(rhs.x),
            y: self.y.saturating_add(rhs.y),
        }
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self {
            x: self.x.saturating_sub(rhs.x),
            y: self.y.saturating_sub(rhs.y),
        }
    }
}

impl Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// 2D size with non-negative dimensions
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub const fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    pub fn area(&self) -> Option<i32> {
        self.width.checked_mul(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self {
            width: self.width.checked_add(rhs.width)?,
            height: self.height.checked_add(rhs.height)?,
        })
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self {
            width: self.width.saturating_add(rhs.width),
            height: self.height.saturating_add(rhs.height),
        }
    }
}

/// Axis-aligned rectangle (exclusive coordinates)
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn from_coords(origin: Point, size: Size) -> Self {
        Self {
            left: origin.x,
            top: origin.y,
            right: origin.x + size.width,
            bottom: origin.y + size.height,
        }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    pub fn origin(&self) -> Point {
        Point::new(self.left, self.top)
    }

    pub fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    pub fn contains(&self, pt: Point) -> bool {
        pt.x >= self.left && pt.x < self.right && pt.y >= self.top && pt.y < self.bottom
    }

    pub fn intersect(&self, other: &Rect) -> Rect {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);
        if right <= left || bottom <= top {
            Rect::default() // Empty
        } else {
            Rect::new(left, top, right, bottom)
        }
    }

    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            *other
        } else if other.is_empty() {
            *self
        } else {
            Rect::new(
                self.left.min(other.left),
                self.top.min(other.top),
                self.right.max(other.right),
                self.bottom.max(other.bottom),
            )
        }
    }

    pub fn offset(&self, pt: Point) -> Rect {
        Rect::new(
            self.left + pt.x,
            self.top + pt.y,
            self.right + pt.x,
            self.bottom + pt.y,
        )
    }

    pub fn inflate(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(
            self.left - dx,
            self.top - dy,
            self.right + dx,
            self.bottom + dy,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Point tests
    #[test]
    fn point_default_is_zero() {
        assert_eq!(Point::default(), Point::zero());
    }

    #[test]
    fn point_add() {
        let p1 = Point::new(10, 20);
        let p2 = Point::new(5, 15);
        assert_eq!(p1 + p2, Point::new(15, 35));
    }

    #[test]
    fn point_sub() {
        let p1 = Point::new(10, 20);
        let p2 = Point::new(5, 15);
        assert_eq!(p1 - p2, Point::new(5, 5));
    }

    #[test]
    fn point_checked_add_overflow() {
        let p = Point::new(i32::MAX, i32::MAX);
        assert_eq!(p.checked_add(Point::new(1, 1)), None);
    }

    #[test]
    fn point_saturating_add() {
        let p = Point::new(i32::MAX, i32::MAX);
        assert_eq!(
            p.saturating_add(Point::new(1, 1)),
            Point::new(i32::MAX, i32::MAX)
        );
    }

    // Size tests
    #[test]
    fn size_area() {
        let s = Size::new(10, 20);
        assert_eq!(s.area(), Some(200));
    }

    #[test]
    fn size_area_overflow() {
        let s = Size::new(i32::MAX, 2);
        assert_eq!(s.area(), None);
    }

    #[test]
    fn size_is_empty() {
        assert!(Size::new(0, 10).is_empty());
        assert!(Size::new(10, 0).is_empty());
        assert!(Size::new(-1, 10).is_empty());
        assert!(!Size::new(10, 20).is_empty());
    }

    // Rect tests
    #[test]
    fn rect_from_coords() {
        let r = Rect::from_coords(Point::new(10, 20), Size::new(30, 40));
        assert_eq!(r, Rect::new(10, 20, 40, 60));
    }

    #[test]
    fn rect_width_height() {
        let r = Rect::new(10, 20, 40, 60);
        assert_eq!(r.width(), 30);
        assert_eq!(r.height(), 40);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10, 20, 40, 60);
        assert!(r.contains(Point::new(10, 20))); // Top-left inclusive
        assert!(r.contains(Point::new(39, 59))); // Bottom-right - 1
        assert!(!r.contains(Point::new(40, 60))); // Bottom-right exclusive
        assert!(!r.contains(Point::new(5, 25)));
    }

    #[test]
    fn rect_intersect() {
        let r1 = Rect::new(0, 0, 10, 10);
        let r2 = Rect::new(5, 5, 15, 15);
        assert_eq!(r1.intersect(&r2), Rect::new(5, 5, 10, 10));
    }

    #[test]
    fn rect_intersect_empty() {
        let r1 = Rect::new(0, 0, 10, 10);
        let r2 = Rect::new(20, 20, 30, 30);
        assert!(r1.intersect(&r2).is_empty());
    }

    #[test]
    fn rect_union() {
        let r1 = Rect::new(0, 0, 10, 10);
        let r2 = Rect::new(5, 5, 15, 15);
        assert_eq!(r1.union(&r2), Rect::new(0, 0, 15, 15));
    }

    #[test]
    fn rect_offset() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!(r.offset(Point::new(5, 10)), Rect::new(15, 30, 35, 50));
    }

    #[test]
    fn rect_inflate() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!(r.inflate(2, 3), Rect::new(8, 17, 32, 43));
    }
}
