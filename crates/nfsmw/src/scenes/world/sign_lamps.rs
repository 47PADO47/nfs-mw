//! The flashing yellow lamp on top of the curve warning signs (`XS_*Curve*` scenery). Drawn as a glow that blinks once
//! a second. An approximation: the original's lamp timing is not decoded.

use blackbox_render::EffectVertex;
use blackbox_scene::Aabb;
use glam::Vec3;

use super::vehicle_effects::geometry;

/// Lamps further than this from the camera (metres) are not drawn.
const DRAW_DISTANCE: f32 = 250.0;
/// Glow radius (metres), and the blink period (seconds): on for half of it.
const RADIUS: f32 = 0.5;
const PERIOD: f32 = 1.0;
const COLOUR: [f32; 4] = [1.0, 0.8, 0.1, 1.0];

/// Whether a scenery object is a curve warning sign.
fn is_curve_warning(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("xs_") && lower.contains("curve")
}

/// The point of the lamp of a curve warning sign: above the middle of its bounds. `None` for other objects.
pub fn lamp_point(name: &str, bounds: Aabb) -> Option<Vec3> {
    let centre = bounds.center();
    is_curve_warning(name).then_some(Vec3::new(centre.x, centre.y, bounds.max.z))
}

/// Appends the glows of the lamps in `points` that are lit at `time` (simulated seconds), within sight of `camera`.
pub fn glows(points: impl Iterator<Item = Vec3>, time: f32, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
    if (time.max(0.0) % PERIOD) >= PERIOD / 2.0 {
        return;
    }
    let colour = geometry::color(COLOUR, 1.0);
    for point in points.filter(|p| p.distance(camera) <= DRAW_DISTANCE) {
        geometry::glow(out, point, RADIUS, colour, camera, forward);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_curve_warning_signs_get_a_lamp() {
        let bounds = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 4.0]);
        assert_eq!(lamp_point("XS_WarnCurveR_1b_00", bounds), Some(Vec3::new(0.5, 0.5, 4.0)));
        assert_eq!(lamp_point("XS_WarnGradCurveL_1b_00", bounds).map(|p| p.z), Some(4.0));
        assert_eq!(lamp_point("XS_StopSignB_1b_00", bounds), None);
        assert_eq!(lamp_point("XW_FenceCurveA8M_1a_LL_", bounds), None, "fences are not signs");
    }

    #[test]
    fn the_lamp_blinks_once_a_second_and_only_within_sight() {
        let camera = Vec3::ZERO;
        let near = Vec3::new(10.0, 0.0, 4.0);
        let far = Vec3::new(1000.0, 0.0, 4.0);
        let mut out = Vec::new();
        glows([near, far].into_iter(), 0.25, camera, Vec3::X, &mut out);
        assert_eq!(out.len(), 6, "the near lamp, lit");
        out.clear();
        glows([near].into_iter(), 0.75, camera, Vec3::X, &mut out);
        assert!(out.is_empty(), "the off half");
    }
}
