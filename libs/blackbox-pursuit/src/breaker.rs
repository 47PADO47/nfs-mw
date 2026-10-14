//! Pursuit breaker zones (spec `docs/specs/pursuit-breakers.md` section 7): an `EBreakerStopCops` event stops the cops
//! inside its radius for its duration. The test is horizontal, so the height does not count.

use glam::Vec2;

/// Whether `point` (physics x and z) is inside the zone of `radius` around `centre`. The edge is outside, as the spec
/// says "below the zone radius".
pub fn inside(centre: Vec2, radius: f32, point: Vec2) -> bool {
    point.distance(centre) < radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cop_inside_the_radius_is_stopped() {
        assert!(inside(Vec2::ZERO, 12.0, Vec2::new(10.0, 0.0)));
    }

    #[test]
    fn the_edge_is_outside_the_zone() {
        assert!(!inside(Vec2::ZERO, 12.0, Vec2::new(12.0, 0.0)));
    }

    #[test]
    fn a_zone_is_a_disc_on_the_road_plane() {
        assert!(inside(Vec2::new(5.0, 5.0), 3.0, Vec2::new(7.0, 6.0)));
        assert!(!inside(Vec2::new(5.0, 5.0), 3.0, Vec2::new(9.0, 5.0)));
    }
}
