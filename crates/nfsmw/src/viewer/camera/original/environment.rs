//! Retained-view tunnel/overpass classification; see original-camera-obstruction spec.

use blackbox_roads::TrackZones;
use glam::{Vec2, Vec3};

/// Query with the previous published camera eye and the current Cubic geometry anchor.
pub fn covered(zones: &TrackZones, previous_eye: Vec3, anchor: Vec3) -> bool {
    let point = Vec2::new(previous_eye.x, previous_eye.y);
    let mut nearest = 9999.0;
    let mut result = false;
    if let Some(zone) = zones.first_of_kind_at(3, point)
        && anchor.z < zone.elevation
    {
        nearest = zone.elevation;
        result = true;
    }
    for kind in [4, 5, 7] {
        if let Some(zone) = zones.first_of_kind_at(kind, point)
            && anchor.z < zone.elevation
            && zone.elevation < nearest
        {
            nearest = zone.elevation;
            result = true;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use blackbox_roads::TrackZone;

    use super::*;

    fn zone(kind: u32, elevation: f32) -> TrackZone {
        TrackZone {
            kind,
            elevation,
            position: Vec2::ZERO,
            direction: Vec2::X,
            visit_info: 0,
            data: [0; 4],
            bbox_min: Vec2::splat(-1.0),
            bbox_max: Vec2::splat(1.0),
            polygon: vec![Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::ONE, Vec2::new(-1.0, 1.0)],
        }
    }

    #[test]
    fn native_cover_kinds_require_anchor_strictly_below_the_first_zone() {
        for kind in [3, 4, 5, 7] {
            let zones = TrackZones { zones: vec![zone(kind, 5.0)] };
            assert!(covered(&zones, Vec3::ZERO, Vec3::Z * 4.999));
            assert!(!covered(&zones, Vec3::ZERO, Vec3::Z * 5.0));
            assert!(!covered(&zones, Vec3::ZERO, Vec3::Z * 6.0));
        }
        assert!(!covered(&TrackZones { zones: vec![zone(8, 5.0)] }, Vec3::ZERO, Vec3::ZERO));
    }

    #[test]
    fn environment_uses_previous_camera_xy_and_current_anchor_height() {
        let zones = TrackZones { zones: vec![zone(3, 5.0)] };
        assert!(covered(&zones, Vec3::Z * 100.0, Vec3::new(50.0, 50.0, 4.0)));
        assert!(!covered(&zones, Vec3::X * 2.0, Vec3::Z * 4.0));
        let zones = TrackZones { zones: vec![zone(3, 1.0), zone(3, 5.0)] };
        assert!(!covered(&zones, Vec3::ZERO, Vec3::Z * 2.0), "later same-kind zones are not selected");
    }

    #[test]
    fn native_nearest_cover_initial_value_is_preserved() {
        assert!(!covered(&TrackZones { zones: vec![zone(4, 9999.0)] }, Vec3::ZERO, Vec3::ZERO));
        assert!(covered(&TrackZones { zones: vec![zone(3, 10000.0)] }, Vec3::ZERO, Vec3::ZERO));
    }
}
