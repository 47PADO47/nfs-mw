//! The pose a NIS scene gives the player's car while the scene plays. The physics stays frozen under it (the world does
//! not step while a scene plays), and the car is drawn at the scene's pose instead. The scene's side is in
//! `nis/car_world.rs`.

use glam::Vec3;

use super::{CarPose, Drive};

/// The fastest motion a scene's car moves at, m/s: more is a cut. Over 400 km/h.
const MAX_SCENE_SPEED: f32 = 120.0;

impl Drive {
    /// Sets the pose a NIS scene gives the car, or clears it with `None`: the car then draws its physics pose again.
    pub fn set_nis_pose(&mut self, pose: Option<CarPose>) {
        self.nis_pose = pose;
    }

    /// Sets the pose a NIS scene gives the car this frame, `dt` scene seconds after the last, and lets the cutscene trail
    /// follow the car's motion between the two. A jump faster than any car drives is a cut of the scene's camera or
    /// car, not motion, and starts no trail.
    pub fn move_nis_pose(&mut self, pose: CarPose, dt: f32) {
        let Some(before) = self.nis_pose.replace(pose) else { return };
        let Some(velocity) = scene_velocity(before.position, pose.position, dt) else { return };
        self.vehicle_effects.cutscene_step(pose, velocity, dt);
    }

    /// The pose the car is drawn at now: the NIS scene's pose while a scene drives it, a multiplayer client's smoothed
    /// pose (`predict.rs`), otherwise the physics pose.
    pub fn render_pose(&self) -> CarPose {
        self.nis_pose.or(self.net_pose).unwrap_or_else(|| self.pose())
    }
}

/// How fast a scene moved a car from `before` to `after` in `dt` seconds, or `None` when it did not move it by motion:
/// no time passed, or the jump is a cut.
fn scene_velocity(before: Vec3, after: Vec3, dt: f32) -> Option<Vec3> {
    if !dt.is_finite() || dt <= 0.0 {
        return None;
    }
    let velocity = (after - before) / dt;
    (velocity.is_finite() && velocity.length() <= MAX_SCENE_SPEED).then_some(velocity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_car_has_the_velocity_of_its_motion_unless_it_was_cut_to_or_did_not_move_in_time() {
        let step = 1.0 / 60.0;
        let moved = scene_velocity(Vec3::ZERO, Vec3::X, step).unwrap();
        assert!(moved.abs_diff_eq(Vec3::X * 60.0, 1e-3));
        assert_eq!(scene_velocity(Vec3::ZERO, Vec3::X * 500.0, step), None, "a cut");
        assert_eq!(scene_velocity(Vec3::ZERO, Vec3::X, 0.0), None, "no time");
        assert_eq!(scene_velocity(Vec3::ZERO, Vec3::X, f32::NAN), None);
        assert_eq!(scene_velocity(Vec3::ZERO, Vec3::NAN, step), None);
    }
}
