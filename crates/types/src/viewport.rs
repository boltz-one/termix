// Viewport - 2D screen region with coordinate transformations
// Port: til/inc/viewport.h + types/inc/viewport.hpp
// Migrated from core/src/screen.rs + extended with WT methods

use crate::{Point, Rect, Size};

/// Viewport - rectangular region with coordinate operations
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Viewport {
    rect: Rect,
}

impl Viewport {
    /// Create viewport from coordinates
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            rect: Rect::new(left, top, right, bottom),
        }
    }

    /// Create from origin + size
    pub const fn from_size(origin: Point, size: Size) -> Self {
        Self {
            rect: Rect::from_coords(origin, size),
        }
    }

    /// Empty viewport at origin
    pub const fn empty() -> Self {
        Self {
            rect: Rect::new(0, 0, 0, 0),
        }
    }

    /// Width in columns
    pub fn width(&self) -> i32 {
        self.rect.width()
    }

    /// Height in rows
    pub fn height(&self) -> i32 {
        self.rect.height()
    }

    /// Size as (width, height)
    pub fn size(&self) -> Size {
        self.rect.size()
    }

    /// Dimensions as Size
    pub fn dimensions(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.rect.is_empty()
    }

    /// Left coordinate
    pub fn left(&self) -> i32 {
        self.rect.left
    }

    /// Top coordinate
    pub fn top(&self) -> i32 {
        self.rect.top
    }

    /// Right coordinate (exclusive)
    pub fn right(&self) -> i32 {
        self.rect.right
    }

    /// Bottom coordinate (exclusive)
    pub fn bottom(&self) -> i32 {
        self.rect.bottom
    }

    /// Origin point (top-left)
    pub fn origin(&self) -> Point {
        self.rect.origin()
    }

    /// Get underlying rect
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Translate to origin (0,0)
    pub fn to_origin(&self) -> Self {
        Self {
            rect: Rect::new(0, 0, self.width(), self.height()),
        }
    }

    /// Convert to inclusive rect (bottom-right inclusive)
    pub fn to_inclusive_rect(&self) -> Rect {
        if self.is_empty() {
            Rect::default()
        } else {
            Rect::new(
                self.rect.left,
                self.rect.top,
                self.rect.right - 1,
                self.rect.bottom - 1,
            )
        }
    }

    /// Convert to exclusive rect (already exclusive)
    pub fn to_exclusive_rect(&self) -> Rect {
        self.rect
    }

    /// Check if point is inside viewport
    pub fn contains(&self, pt: Point) -> bool {
        self.rect.contains(pt)
    }

    /// Clamp coordinate to viewport bounds
    pub fn clamp_coord(&self, x: i32, y: i32) -> (i32, i32) {
        let clamped_x = x.clamp(self.rect.left, self.rect.right - 1);
        let clamped_y = y.clamp(self.rect.top, self.rect.bottom - 1);
        (clamped_x, clamped_y)
    }

    /// Walk within bounds, return None if out of bounds
    pub fn walk_in_bounds(&self, x: i32, y: i32, delta_x: i32, delta_y: i32) -> Option<(i32, i32)> {
        let new_x = x + delta_x;
        let new_y = y + delta_y;

        if self.contains(Point::new(new_x, new_y)) {
            Some((new_x, new_y))
        } else {
            None
        }
    }

    /// Walk with circular wrapping
    pub fn walk_in_bounds_circular(&self, x: i32, y: i32, delta_x: i32, delta_y: i32) -> (i32, i32) {
        let width = self.width();
        let height = self.height();

        if width <= 0 || height <= 0 {
            return (x, y);
        }

        let mut new_x = (x - self.rect.left + delta_x) % width;
        let mut new_y = (y - self.rect.top + delta_y) % height;

        // Handle negative modulo
        if new_x < 0 {
            new_x += width;
        }
        if new_y < 0 {
            new_y += height;
        }

        (new_x + self.rect.left, new_y + self.rect.top)
    }

    /// Trim rect to viewport bounds
    pub fn trim_to_viewport(&self, rect: &Rect) -> Rect {
        self.rect.intersect(rect)
    }

    /// Intersect with another viewport
    pub fn intersect(&self, other: &Viewport) -> Viewport {
        Self {
            rect: self.rect.intersect(&other.rect),
        }
    }

    /// Union with another viewport
    pub fn union(&self, other: &Viewport) -> Viewport {
        Self {
            rect: self.rect.union(&other.rect),
        }
    }

    /// Offset viewport by point
    pub fn offset(&self, pt: Point) -> Viewport {
        Self {
            rect: self.rect.offset(pt),
        }
    }

    /// Get walk origin (top-left)
    pub fn get_walk_origin(&self) -> Point {
        self.origin()
    }

    /// Convert coordinate from this viewport to another
    pub fn convert_to_origin(&self, pt: Point, target: &Viewport) -> Point {
        Point::new(
            pt.x - self.rect.left + target.rect.left,
            pt.y - self.rect.top + target.rect.top,
        )
    }

    /// Convert coordinate from origin to this viewport
    pub fn convert_from_origin(&self, pt: Point) -> Point {
        Point::new(pt.x + self.rect.left, pt.y + self.rect.top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_new() {
        let vp = Viewport::new(10, 20, 30, 40);
        assert_eq!(vp.left(), 10);
        assert_eq!(vp.top(), 20);
        assert_eq!(vp.right(), 30);
        assert_eq!(vp.bottom(), 40);
    }

    #[test]
    fn viewport_from_size() {
        let vp = Viewport::from_size(Point::new(5, 10), Size::new(20, 15));
        assert_eq!(vp.left(), 5);
        assert_eq!(vp.top(), 10);
        assert_eq!(vp.width(), 20);
        assert_eq!(vp.height(), 15);
    }

    #[test]
    fn viewport_dimensions() {
        let vp = Viewport::new(0, 0, 80, 25);
        let size = vp.dimensions();
        assert_eq!(size, Size::new(80, 25));
    }

    #[test]
    fn viewport_to_origin() {
        let vp = Viewport::new(10, 20, 30, 40);
        let origin = vp.to_origin();
        assert_eq!(origin.left(), 0);
        assert_eq!(origin.top(), 0);
        assert_eq!(origin.width(), 20);
        assert_eq!(origin.height(), 20);
    }

    #[test]
    fn viewport_to_inclusive_rect() {
        let vp = Viewport::new(10, 20, 30, 40);
        let rect = vp.to_inclusive_rect();
        assert_eq!(rect, Rect::new(10, 20, 29, 39)); // Right/bottom -1
    }

    #[test]
    fn viewport_contains() {
        let vp = Viewport::new(0, 0, 10, 10);
        assert!(vp.contains(Point::new(0, 0)));
        assert!(vp.contains(Point::new(9, 9)));
        assert!(!vp.contains(Point::new(10, 10)));
        assert!(!vp.contains(Point::new(-1, 5)));
    }

    #[test]
    fn viewport_clamp_coord() {
        let vp = Viewport::new(0, 0, 10, 10);
        assert_eq!(vp.clamp_coord(-5, 5), (0, 5));
        assert_eq!(vp.clamp_coord(15, 5), (9, 5));
        assert_eq!(vp.clamp_coord(5, -3), (5, 0));
        assert_eq!(vp.clamp_coord(5, 20), (5, 9));
    }

    #[test]
    fn viewport_walk_in_bounds() {
        let vp = Viewport::new(0, 0, 10, 10);
        assert_eq!(vp.walk_in_bounds(5, 5, 1, 1), Some((6, 6)));
        assert_eq!(vp.walk_in_bounds(9, 9, 1, 1), None); // Out of bounds
        assert_eq!(vp.walk_in_bounds(0, 0, -1, 0), None);
    }

    #[test]
    fn viewport_walk_circular() {
        let vp = Viewport::new(0, 0, 10, 10);
        // Wrap right edge
        assert_eq!(vp.walk_in_bounds_circular(9, 5, 1, 0), (0, 5));
        // Wrap bottom edge
        assert_eq!(vp.walk_in_bounds_circular(5, 9, 0, 1), (5, 0));
        // Wrap left edge (negative)
        assert_eq!(vp.walk_in_bounds_circular(0, 5, -1, 0), (9, 5));
        // Wrap top edge (negative)
        assert_eq!(vp.walk_in_bounds_circular(5, 0, 0, -1), (5, 9));
    }

    #[test]
    fn viewport_trim_to_viewport() {
        let vp = Viewport::new(0, 0, 10, 10);
        let rect = Rect::new(5, 5, 15, 15);
        let trimmed = vp.trim_to_viewport(&rect);
        assert_eq!(trimmed, Rect::new(5, 5, 10, 10));
    }

    #[test]
    fn viewport_intersect() {
        let vp1 = Viewport::new(0, 0, 10, 10);
        let vp2 = Viewport::new(5, 5, 15, 15);
        let result = vp1.intersect(&vp2);
        assert_eq!(result.rect(), Rect::new(5, 5, 10, 10));
    }

    #[test]
    fn viewport_union() {
        let vp1 = Viewport::new(0, 0, 10, 10);
        let vp2 = Viewport::new(5, 5, 15, 15);
        let result = vp1.union(&vp2);
        assert_eq!(result.rect(), Rect::new(0, 0, 15, 15));
    }

    #[test]
    fn viewport_offset() {
        let vp = Viewport::new(10, 20, 30, 40);
        let offset = vp.offset(Point::new(5, 10));
        assert_eq!(offset.rect(), Rect::new(15, 30, 35, 50));
    }

    #[test]
    fn viewport_convert_to_origin() {
        let vp1 = Viewport::new(10, 20, 30, 40);
        let vp2 = Viewport::new(5, 10, 25, 30);
        let pt = Point::new(15, 25);
        let converted = vp1.convert_to_origin(pt, &vp2);
        // Relative to vp1: (15-10, 25-20) = (5, 5)
        // Absolute in vp2: (5+5, 5+10) = (10, 15)
        assert_eq!(converted, Point::new(10, 15));
    }

    #[test]
    fn viewport_convert_from_origin() {
        let vp = Viewport::new(10, 20, 30, 40);
        let pt = Point::new(5, 5);
        let converted = vp.convert_from_origin(pt);
        assert_eq!(converted, Point::new(15, 25));
    }

    #[test]
    fn viewport_empty() {
        let vp = Viewport::empty();
        assert!(vp.is_empty());
        assert_eq!(vp.width(), 0);
        assert_eq!(vp.height(), 0);
    }

    #[test]
    fn viewport_get_walk_origin() {
        let vp = Viewport::new(10, 20, 30, 40);
        assert_eq!(vp.get_walk_origin(), Point::new(10, 20));
    }
}
