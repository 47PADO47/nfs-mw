//! Where an element stands on the screen. A skin is written for a 1080-line screen and anchored to the right and bottom
//! edge (`docs/specs/custom-hud-gauges.md` section 4). The canvas here is the presenter's: centred, y down, a fixed height.

/// The height of the reference screen the skins are written for.
pub const REFERENCE_HEIGHT: f32 = 1080.0;

/// A rectangle by its centre and its size, in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The conversion from reference pixels to canvas units for one window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The canvas's half width and half height: where the right and bottom edge are.
    right: f32,
    bottom: f32,
    /// Canvas units per reference pixel.
    unit: f32,
    offset: (f32, f32),
}

impl Placement {
    /// For a canvas `canvas_width` by `canvas_height` units, the ini's global `scale` and its offsets (reference pixels).
    pub fn new(canvas_width: f32, canvas_height: f32, scale: f32, offset_x: f32, offset_y: f32) -> Self {
        Self {
            right: canvas_width / 2.0,
            bottom: canvas_height / 2.0,
            unit: canvas_height / REFERENCE_HEIGHT * scale,
            offset: (offset_x, offset_y),
        }
    }

    /// Canvas units per reference pixel (the global scale included).
    pub fn unit(&self) -> f32 {
        self.unit
    }

    /// The rectangle of a `w` by `h` (reference pixels) sprite whose right and bottom edge stand `x` and `y` reference
    /// pixels in from the screen's, with the global offset added.
    pub fn rect(&self, w: f32, h: f32, x: f32, y: f32) -> Rect {
        let right_edge = self.right - (x + self.offset.0) * self.unit;
        let bottom_edge = self.bottom - (y + self.offset.1) * self.unit;
        let (width, height) = (w * self.unit, h * self.unit);
        Rect { x: right_edge - width / 2.0, y: bottom_edge - height / 2.0, w: width, h: height }
    }

    /// The centre of a `size` square placed at `x`, `y`: where a needle turns.
    pub fn centre(&self, size: f32, x: f32, y: f32) -> (f32, f32) {
        let rect = self.rect(size, size, x, y);
        (rect.x, rect.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 16:9 canvas of 480 units.
    fn placement() -> Placement {
        Placement::new(480.0 * 16.0 / 9.0, 480.0, 1.0, 0.0, 0.0)
    }

    #[test]
    fn a_square_stands_in_the_bottom_right_corner() {
        let p = placement();
        // 330 px at (30, 60): its right edge 30 reference px from the right edge, the bottom 60 px up.
        let rect = p.rect(330.0, 330.0, 30.0, 60.0);
        let unit = 480.0 / 1080.0;
        assert!((p.unit() - unit).abs() < 1e-6);
        assert!((rect.x + rect.w / 2.0 - (480.0 * 16.0 / 18.0 - 30.0 * unit)).abs() < 1e-3);
        assert!((rect.y + rect.h / 2.0 - (240.0 - 60.0 * unit)).abs() < 1e-3);
        assert!((rect.w - 330.0 * unit).abs() < 1e-3);
    }

    #[test]
    fn a_larger_x_is_further_left_and_a_larger_y_is_higher() {
        let p = placement();
        let near = p.rect(90.0, 90.0, 30.0, 60.0);
        let far = p.rect(90.0, 90.0, 345.0, 275.0);
        assert!(far.x < near.x);
        assert!(far.y < near.y);
    }

    #[test]
    fn scale_and_offset_apply_to_sizes_and_positions() {
        let plain = placement();
        let scaled = Placement::new(480.0 * 16.0 / 9.0, 480.0, 2.0, 0.0, 0.0);
        assert!((scaled.rect(10.0, 10.0, 0.0, 0.0).w - 2.0 * plain.rect(10.0, 10.0, 0.0, 0.0).w).abs() < 1e-4);
        let shifted = Placement::new(480.0 * 16.0 / 9.0, 480.0, 1.0, 100.0, -50.0);
        let a = plain.rect(10.0, 10.0, 0.0, 0.0);
        let b = shifted.rect(10.0, 10.0, 0.0, 0.0);
        assert!((a.x - b.x - 100.0 * plain.unit()).abs() < 1e-3, "a positive offset moves the sprite left");
        assert!((b.y - a.y - 50.0 * plain.unit()).abs() < 1e-3, "a negative offset moves it down");
    }

    #[test]
    fn the_cluster_keeps_to_the_corner_of_a_wider_window() {
        let wide = Placement::new(480.0 * 21.0 / 9.0, 480.0, 1.0, 0.0, 0.0);
        let normal = placement();
        let (a, b) = (wide.rect(100.0, 100.0, 30.0, 60.0), normal.rect(100.0, 100.0, 30.0, 60.0));
        let (edge_a, edge_b) = (a.x + a.w / 2.0, b.x + b.w / 2.0);
        assert!(edge_a > edge_b, "the right edge of a wider canvas is further right");
        assert!((edge_a - 480.0 * 21.0 / 18.0 - (edge_b - 480.0 * 16.0 / 18.0)).abs() < 1e-3);
        assert!((a.y - b.y).abs() < 1e-5);
    }

    #[test]
    fn the_centre_is_the_middle_of_the_square() {
        let p = placement();
        let rect = p.rect(200.0, 200.0, 10.0, 20.0);
        assert_eq!(p.centre(200.0, 10.0, 20.0), (rect.x, rect.y));
    }
}
