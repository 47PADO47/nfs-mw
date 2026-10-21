//! Taking a cop that fell out of play off the road. Spec: `docs/specs/ai-pursuit-cops.md` sections 2, 3.3 and 5.
//!
//! The original's patrol pass visits every active cop car: one that may be recycled and stands where a car can come and
//! go unseen is removed, unless its chase is about to meet it; a wreck goes once it has been off screen for 3 s. The
//! traffic's own unseen-time rules do not apply to cops.

use blackbox_pursuit::respawn::{self, COP_WINDOW};
use glam::Vec3;

use super::super::{AiCar, Focus};
use super::visibility::Viewing;
use crate::viewer::camera::Lens;

/// The view and the chase target one pass measures every cop against.
pub(super) struct Base {
    /// The camera eye and its forward direction, render space: the cop spawn window is measured from here.
    eye: Vec3,
    forward: Vec3,
    /// The point 75 m along the player's travel, physics space, when a chase runs: the cops it is about to meet stay.
    ahead: Option<Vec3>,
}

impl Base {
    pub fn new(camera: &Lens, focus: Focus, chasing: bool) -> Self {
        let ahead = focus.body.filter(|_| chasing).map(|body| {
            let forward = Vec3::new(body.forward.x, 0.0, body.forward.y);
            respawn::ahead_point(body.position, body.velocity, forward)
        });
        Self { eye: camera.position, forward: camera.forward, ahead }
    }

    /// The direction from the camera to `at` against the camera's facing, the cone of `RespawnAvailable`.
    fn dot(&self, at: Vec3) -> Option<f32> {
        let to = (at - self.eye).try_normalize()?;
        Some(to.dot(self.forward))
    }
}

/// Whether a cop car stays on the road this tick. A car that is not a cop is not asked.
pub(super) fn keeps(car: &mut AiCar, view: &Viewing, base: &Base, airborne_limit: f32) -> bool {
    if !car.is_finite() {
        return false;
    }
    let at = car.position();
    let distance = (at.x - base.eye.x).hypot(at.y - base.eye.y);
    let available = respawn::available(COP_WINDOW, distance, base.dot(at));
    let can_respawn = car.may_respawn(available);
    let in_chase = !car.is_patrol() && !car.is_roadblock();
    let about_to_meet =
        in_chase && base.ahead.is_some_and(|ahead| respawn::near_target(car.physics_position(), ahead, false));
    let remove = respawn::should_remove(&respawn::Cop {
        available,
        can_respawn,
        destroyed: car.is_wrecked(),
        offscreen: car.offscreen_time(),
        about_to_meet,
    });
    // The rewrite's own rule: a cop with no wheel on the ground for long and out of sight fell off the map.
    let fell = car.airborne_time() >= airborne_limit && car.offscreen_time() > view.grace;
    !(remove || fell)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(forward: Vec3, ahead: Option<Vec3>) -> Base {
        Base { eye: Vec3::ZERO, forward, ahead }
    }

    #[test]
    fn the_cone_is_measured_from_the_camera_eye() {
        let base = base(Vec3::Y, None);
        assert!((base.dot(Vec3::new(0.0, 100.0, 0.0)).unwrap() - 1.0).abs() < 1e-5, "ahead");
        assert!(base.dot(Vec3::new(100.0, 0.0, 0.0)).unwrap().abs() < 1e-5, "beside");
        assert!((base.dot(Vec3::new(0.0, -100.0, 0.0)).unwrap() + 1.0).abs() < 1e-5, "behind");
        assert!(base.dot(Vec3::ZERO).is_none(), "at the eye there is no direction");
    }

    #[test]
    fn a_chase_measures_from_the_point_ahead_of_its_target_only_while_it_runs() {
        let lens = Lens::look_at(Vec3::ZERO, Vec3::Y, 90.0, 0.1, 1.0);
        let body = blackbox_roads::Body {
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 0.0, 30.0),
            forward: glam::Vec2::Y,
            half_width: 1.0,
            half_length: 2.5,
        };
        let focus = Focus { position: [0.0, 0.0], heading: 0.0, speed: 30.0, body: Some(body) };
        assert!(Base::new(&lens, focus, false).ahead.is_none());
        let ahead = Base::new(&lens, focus, true).ahead.unwrap();
        assert!((ahead - Vec3::new(0.0, 0.0, 75.0)).length() < 1e-4);
    }
}
