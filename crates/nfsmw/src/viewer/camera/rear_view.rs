//! The rear-view mirror's camera: a fixed view on the car, looking back along it. The mirror shows what is
//! behind the car. The placement and field of view are estimates in the car's frame (x forward, y left, z up),
//! not measured from the original game; see docs/specs/rear-view-mirror.md.

use glam::{Quat, Vec3};

use super::{Camera, Lens};

/// Distance of the near plane, metres. Read from the original's mirror camera object at runtime (0.75).
const NEAR_PLANE: f32 = 0.75;
/// Where the eye sits, in the car's frame: about head height, a little forward of the seat.
const EYE: Vec3 = Vec3::new(0.2, 0.0, 1.1);
/// What it looks at: a point far behind the car, level with the eye.
const LOOK: Vec3 = Vec3::new(-30.0, 0.0, 1.0);
/// Vertical field of view, degrees. Estimated: narrow, so the wide mirror strip is zoomed on the road behind (about 70
/// degrees across at its 3.5:1 shape) rather than a fish-eye of the whole street.
const FOV_Y_DEGREES: f32 = 24.0;

/// The mirror's camera for a car at `position` with the model frame `rotation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RearView {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Camera for RearView {
    fn lens(&self, aspect: f32) -> Lens {
        let eye = self.position + self.rotation * EYE;
        let target = self.position + self.rotation * LOOK;
        Lens::look_at(eye, target, FOV_Y_DEGREES, NEAR_PLANE, aspect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_back_along_the_car() {
        let view = RearView { position: Vec3::new(10.0, -4.0, 0.5), rotation: Quat::IDENTITY };
        let lens = view.lens(2.0);
        assert!(lens.forward.dot(Vec3::NEG_X) > 0.999, "forward {:?}", lens.forward);
        assert!((lens.position - Vec3::new(10.2, -4.0, 1.6)).length() < 1e-4, "eye {:?}", lens.position);
    }

    #[test]
    fn is_zoomed_on_the_road_behind() {
        let lens = RearView { position: Vec3::ZERO, rotation: Quat::IDENTITY }.lens(3.5);
        let across = 2.0 * ((lens.fov_y_degrees.to_radians() * 0.5).tan() * 3.5).atan().to_degrees();
        assert!((60.0..80.0).contains(&across), "{across} degrees across");
    }

    #[test]
    fn follows_the_car_heading() {
        let turned = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let lens = RearView { position: Vec3::ZERO, rotation: turned }.lens(1.0);
        // Car facing +Y, so the mirror looks towards -Y.
        assert!(lens.forward.dot(Vec3::NEG_Y) > 0.999, "forward {:?}", lens.forward);
    }
}
