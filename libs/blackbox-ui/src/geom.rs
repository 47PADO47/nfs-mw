//! Screen geometry in FEng units: a 640 x 480 screen whose origin is its centre.

use glam::{Mat4, Vec3};

/// An axis-aligned rectangle. `x` and `y` are the centre; `w` and `h` are the full size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// The whole 640 x 480 screen, centred on the origin.
    pub const SCREEN: Rect = Rect { x: 0.0, y: 0.0, w: 640.0, h: 480.0 };

    /// A zero-size rectangle at the origin: the placeholder a component holds before its first layout.
    pub const ZERO: Rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn min_x(&self) -> f32 {
        self.x - self.w / 2.0
    }

    pub fn max_x(&self) -> f32 {
        self.x + self.w / 2.0
    }

    pub fn min_y(&self) -> f32 {
        self.y - self.h / 2.0
    }

    pub fn max_y(&self) -> f32 {
        self.y + self.h / 2.0
    }

    /// The rectangle shrunk by `dx` on each side horizontally and `dy` on each side vertically.
    /// A margin larger than half the size gives a zero-size rectangle, never a negative one.
    pub fn inset(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x, y: self.y, w: (self.w - 2.0 * dx).max(0.0), h: (self.h - 2.0 * dy).max(0.0) }
    }

    /// Whether the point lies inside the rectangle (edges included).
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.min_x() && px <= self.max_x() && py >= self.min_y() && py <= self.max_y()
    }

    /// The world transform of a unit quad drawn over this rectangle at depth `z`. The unit quad spans -0.5 to
    /// 0.5, so this scales it to the rectangle's size and moves it to the rectangle's centre. Image nodes use it.
    pub fn world(&self, z: f32) -> Mat4 {
        Mat4::from_translation(Vec3::new(self.x, self.y, z)) * Mat4::from_scale(Vec3::new(self.w, self.h, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;
    use glam::Vec3;

    #[test]
    fn world_scales_the_unit_quad_to_the_rect_and_centres_it() {
        let w = Rect::new(10.0, -20.0, 100.0, 40.0).world(3.0);
        assert_eq!(w.w_axis.truncate(), Vec3::new(10.0, -20.0, 3.0));
        assert_eq!((w.x_axis.x, w.y_axis.y), (100.0, 40.0));
    }

    #[test]
    fn screen_is_640_by_480_centred_on_origin() {
        let s = Rect::SCREEN;
        assert_eq!((s.min_x(), s.max_x()), (-320.0, 320.0));
        assert_eq!((s.min_y(), s.max_y()), (-240.0, 240.0));
    }

    #[test]
    fn inset_shrinks_each_side_and_clamps_at_zero() {
        let r = Rect::new(0.0, 0.0, 100.0, 40.0).inset(10.0, 5.0);
        assert_eq!(r, Rect::new(0.0, 0.0, 80.0, 30.0));
        let gone = Rect::new(0.0, 0.0, 10.0, 10.0).inset(20.0, 20.0);
        assert_eq!((gone.w, gone.h), (0.0, 0.0));
    }

    #[test]
    fn contains_includes_edges_and_excludes_outside() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(0.0, 0.0));
        assert!(r.contains(5.0, -5.0));
        assert!(!r.contains(5.1, 0.0));
    }
}
