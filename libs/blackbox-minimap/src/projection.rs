//! World metres to map units. Spec: `docs/specs/hud-minimap.md` section 3.

/// Where a map picture sits in the world. The picture is square.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    /// World x of the picture's left edge and world y of its bottom edge, metres.
    pub origin: [f32; 2],
    /// World width of the whole picture, metres.
    pub width: f32,
}

impl Calibration {
    /// A world position `(x, y)` on the picture in units of its width: `(0, 0)` is the top left corner, `(1, 1)`
    /// the bottom right, so +x is to the right and +y up the picture.
    pub fn to_map(&self, world: [f32; 2]) -> [f32; 2] {
        [(world[0] - self.origin[0]) / self.width, (self.origin[1] - world[1]) / self.width + 1.0]
    }
}

/// The direction of a world vector `(x, y)` as a compass bearing in degrees, `0..360`: 0 is up the picture (+y),
/// 90 to the right (+x). The game's `bATan(y, x)` reads its arguments as `(x, y)`, hence the swapped roles.
pub fn bearing_degrees(direction: [f32; 2]) -> f32 {
    let degrees = direction[0].atan2(direction[1]).to_degrees();
    if degrees < 0.0 { degrees + 360.0 } else { degrees }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITY: Calibration = Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };

    #[test]
    fn the_corners_of_the_picture() {
        let bottom_left = CITY.to_map([-1224.8894, -1591.1449]);
        assert!((bottom_left[0]).abs() < 1e-6 && (bottom_left[1] - 1.0).abs() < 1e-6, "{bottom_left:?}");
        let top_right = CITY.to_map([-1224.8894 + 6659.332, -1591.1449 + 6659.332]);
        assert!((top_right[0] - 1.0).abs() < 1e-5 && top_right[1].abs() < 1e-5, "{top_right:?}");
    }

    #[test]
    fn east_is_right_and_north_is_up() {
        let a = CITY.to_map([0.0, 0.0]);
        let east = CITY.to_map([100.0, 0.0]);
        let north = CITY.to_map([0.0, 100.0]);
        assert!(east[0] > a[0] && (east[1] - a[1]).abs() < 1e-6);
        assert!(north[1] < a[1] && (north[0] - a[0]).abs() < 1e-6, "up the picture is a smaller v");
        assert!(((east[0] - a[0]) - 100.0 / 6659.332).abs() < 1e-7);
    }

    #[test]
    fn bearings_run_clockwise_from_up() {
        let b = |x: f32, y: f32| bearing_degrees([x, y]);
        assert!((b(0.0, 1.0) - 0.0).abs() < 1e-4, "up the picture");
        assert!((b(1.0, 0.0) - 90.0).abs() < 1e-4, "right");
        assert!((b(0.0, -1.0) - 180.0).abs() < 1e-4, "down");
        assert!((b(-1.0, 0.0) - 270.0).abs() < 1e-4, "left");
        assert!((b(1.0, 1.0) - 45.0).abs() < 1e-4);
        assert!(b(-0.001, 1.0) > 359.0, "just left of up wraps to the top of the range");
    }
}
