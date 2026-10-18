//! Screen geometry in FEng units: a 640 x 480 screen whose origin is its centre.

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
}

#[cfg(test)]
mod tests {
    use super::Rect;

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
