//! User camera inspection is independent of the normal menu selection keys.

use super::super::super::{camera_move::CameraMove, camera_pose::CameraPose};
use crate::input::{Action, ActionState};
use nfsmw_data::frontend_camera::CameraAngle;

/// The original mover returns to the selected screen after 300 idle ticks at 60 Hz.
const IDLE_SECONDS: f32 = 5.0;
/// The original integrates normalized rotation at sixty degrees per second, and radius at 7.5 metres per second.
const DEGREES_PER_SECOND: f32 = 60.0;
const METRES_PER_SECOND: f32 = 7.5;
/// The shared action layer expresses stick motion as pixels; a full stick supplies 700 pixels per second.
const ORBIT_DEGREES_PER_PIXEL: f32 = DEGREES_PER_SECOND / 700.0;
/// The original's idle horizontal input is 0.5, passed through the same sixty-degree integration as manual input.
const AUTO_HORIZONTAL_INPUT: f32 = 0.5;
const ENTRY_SECONDS: f32 = 0.555;

#[derive(Default)]
pub(super) struct Inspection {
    pose: Option<CameraPose>,
    idle: f32,
    showcase: bool,
    enabled: bool,
    returning: bool,
    automatic: bool,
    shape: Option<CameraAngle>,
    entry: Option<CameraMove>,
}

impl Inspection {
    pub(super) fn set_showcase(&mut self, showcase: bool) {
        self.showcase = showcase;
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(super) fn set_shape(&mut self, shape: Option<CameraAngle>) {
        self.shape = shape;
    }

    pub(super) fn pose(&self) -> Option<CameraPose> {
        self.pose
    }

    pub(super) fn take_pose(&mut self) -> Option<CameraPose> {
        self.idle = 0.0;
        self.returning = false;
        self.automatic = false;
        self.entry = None;
        self.pose.take()
    }

    fn begin_entry(&mut self, pose: CameraPose) {
        if self.pose.is_some() {
            return;
        }
        let Some(mut goal) = self.shape else { return };
        (goal.pitch, goal.yaw, goal.distance) = (pose.pitch, pose.yaw, pose.distance);
        (goal.roll, goal.fov, goal.target, goal.duration) = (0.0, 45.0, [0.0, 0.0, 0.75], ENTRY_SECONDS);
        self.entry = Some(CameraMove::from_pose(pose, &goal));
    }

    fn advance_entry(&mut self, dt: f32) {
        let Some(entry) = &mut self.entry else { return };
        entry.advance(dt);
        if let Some(pose) = &mut self.pose {
            let sample = entry.pose();
            (pose.roll, pose.fov, pose.target) = (sample.roll, sample.fov, sample.target);
        }
        if entry.finished() {
            self.entry = None;
        }
    }

    /// A returned pose hands control back to the installed screen's camera mover.
    pub(super) fn update(
        &mut self,
        input: &ActionState,
        base: Option<CameraPose>,
        dt: f32,
        moving: bool,
    ) -> Option<CameraPose> {
        if !self.enabled {
            return self.take_pose();
        }
        if self.returning && moving {
            return None;
        }
        if self.returning {
            self.returning = false;
            self.automatic = true;
        }
        let dt = dt.max(0.0);
        let mut yaw = -input.value(Action::OrbitX) * ORBIT_DEGREES_PER_PIXEL;
        let mut pitch = input.value(Action::OrbitY) * ORBIT_DEGREES_PER_PIXEL;
        let axis = |plus, minus| input.value(plus) - input.value(minus);
        if self.showcase {
            yaw -= axis(Action::MenuRight, Action::MenuLeft) * DEGREES_PER_SECOND * dt;
            pitch += axis(Action::MenuUp, Action::MenuDown) * DEGREES_PER_SECOND * dt;
        }
        let zoom = axis(Action::MenuZoomIn, Action::MenuZoomOut) * METRES_PER_SECOND * dt;
        if yaw.abs() + pitch.abs() + zoom.abs() > 0.0 {
            // Original speed setters leave a camera's authored move in control until it reaches its goal.
            if moving {
                return None;
            }
            let mut pose = self.pose.or(base)?;
            self.begin_entry(pose);
            pose.yaw = (pose.yaw + yaw).rem_euclid(360.0);
            pose.pitch = (pose.pitch + pitch).clamp(94.5, 110.0);
            pose.distance = (pose.distance - zoom).clamp(4.65, 6.65);
            self.pose = Some(pose);
            self.advance_entry(dt);
            self.idle = 0.0;
            self.automatic = false;
            return None;
        }
        if self.automatic {
            let mut pose = self.pose.or(base)?;
            self.begin_entry(pose);
            pose.yaw = (pose.yaw + AUTO_HORIZONTAL_INPUT * DEGREES_PER_SECOND * dt).rem_euclid(360.0);
            pose.pitch = pose.pitch.clamp(94.5, 110.0);
            pose.distance = pose.distance.clamp(4.65, 6.65);
            self.pose = Some(pose);
            self.advance_entry(dt);
            return None;
        }
        self.advance_entry(dt);
        self.idle += dt;
        if self.idle < IDLE_SECONDS {
            return None;
        }
        let pose = self.take_pose().or(base);
        self.returning = true;
        pose
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::camera_pose::SAFE_HOUSE;

    #[test]
    fn normal_menu_arrows_never_rotate_or_zoom_the_car() {
        let mut input = ActionState::default();
        input.update_menu_buttons(&[Action::MenuRight, Action::MenuUp]);
        let mut camera = Inspection::default();
        camera.set_enabled(true);
        assert!(camera.update(&input, Some(SAFE_HOUSE), 1.0, false).is_none());
        assert!(camera.pose().is_none());
    }

    #[test]
    fn inspection_returns_after_five_idle_seconds_and_uses_a_new_screen_pose_afterwards() {
        let mut input = ActionState::default();
        input.update_menu_buttons(&[Action::MenuZoomIn]);
        let mut camera = Inspection::default();
        camera.set_enabled(true);
        let base = CameraPose { distance: 5.5, ..SAFE_HOUSE };
        camera.update(&input, Some(base), 0.1, false);
        let inspected = camera.pose().unwrap();
        assert!((inspected.distance - 4.75).abs() < 0.0001);
        input.update_menu_buttons(&[]);
        assert!(camera.update(&input, Some(base), 4.9, false).is_none());
        assert_eq!(camera.update(&input, Some(base), 0.11, false), Some(inspected));
        assert!(camera.pose().is_none());
        assert!(camera.update(&input, Some(base), 0.1, true).is_none());
        assert!(camera.pose().is_none(), "authored return move retains control");
        let next = CameraPose { yaw: 200.0, distance: 5.5, ..SAFE_HOUSE };
        input.update_menu_buttons(&[Action::MenuZoomOut]);
        camera.update(&input, Some(next), 0.1, false);
        assert_eq!(camera.pose().unwrap().yaw, next.yaw);
    }

    #[test]
    fn showcase_arrows_use_native_rates_and_idle_auto_rotation_follows_the_return_move() {
        let mut camera = Inspection::default();
        camera.set_showcase(true);
        camera.set_enabled(true);
        let mut input = ActionState::default();
        input.update_menu_buttons(&[Action::MenuRight, Action::MenuUp, Action::MenuZoomIn]);
        let base = CameraPose { distance: 5.5, ..SAFE_HOUSE };
        camera.update(&input, Some(base), 0.1, false);
        let pose = camera.pose().unwrap();
        assert_ne!(pose.yaw, SAFE_HOUSE.yaw);
        assert_ne!(pose.pitch, SAFE_HOUSE.pitch);
        assert!((pose.distance - 4.75).abs() < 0.0001);
        assert!((pose.yaw - (base.yaw - 6.0)).abs() < 0.0001);
        input.update_menu_buttons(&[]);
        assert_eq!(camera.update(&input, Some(base), 5.0, false), Some(pose));
        camera.update(&input, Some(base), 1.0, false);
        assert!((camera.pose().unwrap().yaw - (base.yaw + 30.0)).abs() < 0.0001);
    }

    #[test]
    fn screen_policy_disables_inspection_even_when_zoom_or_showcase_navigation_is_pressed() {
        let mut input = ActionState::default();
        input.update_menu_buttons(&[Action::MenuZoomIn, Action::MenuRight]);
        let mut camera = Inspection::default();
        camera.set_showcase(true);
        camera.update(&input, Some(SAFE_HOUSE), 1.0, false);
        assert!(camera.pose().is_none());
    }

    fn inspection_with_shape() -> Inspection {
        let mut camera = Inspection::default();
        camera.set_enabled(true);
        camera.set_shape(Some(CameraAngle {
            pitch: SAFE_HOUSE.pitch,
            yaw: SAFE_HOUSE.yaw,
            distance: SAFE_HOUSE.distance,
            roll: SAFE_HOUSE.roll,
            fov: SAFE_HOUSE.fov,
            target: SAFE_HOUSE.target.to_array(),
            decay: 8.0,
            duration: 0.6,
            order: 2,
        }));
        camera
    }

    #[test]
    fn inspection_entry_eases_the_lens_and_target_and_finishes_after_the_native_duration() {
        let mut camera = inspection_with_shape();
        let base = CameraPose { distance: 5.5, ..SAFE_HOUSE };
        let mut input = ActionState::default();
        input.update_menu_buttons(&[Action::MenuZoomIn]);
        camera.update(&input, Some(base), 0.05, false);
        let started = camera.pose().unwrap();
        assert_ne!(started.fov, base.fov, "the transition advances immediately");
        assert_ne!(started.fov, 45.0, "entering inspection must not snap onto its goal");
        assert_ne!(started.target, base.target);
        assert_eq!(started.yaw, base.yaw);
        // Repeated input must advance the existing transition rather than restart it every frame.
        for _ in 0..11 {
            camera.update(&input, Some(base), 0.05, false);
        }
        let settled = camera.pose().unwrap();
        assert_eq!((settled.roll, settled.fov, settled.target), (0.0, 45.0, glam::Vec3::new(0.0, 0.0, 0.75)));
        assert_eq!(settled.yaw, base.yaw);
        assert_eq!(settled.distance, 4.65, "native zoom clamp remains independent of the lens transition");
    }

    #[test]
    fn releasing_inspection_input_keeps_its_entry_transition_running_and_a_screen_change_takes_the_current_pose() {
        let mut camera = inspection_with_shape();
        let base = CameraPose { distance: 5.5, ..SAFE_HOUSE };
        let mut input = ActionState::default();
        input.update_menu_buttons(&[Action::MenuZoomOut]);
        camera.update(&input, Some(base), 0.05, false);
        input.update_menu_buttons(&[]);
        camera.update(&input, Some(base), 0.1, false);
        let intermediate = camera.pose().unwrap();
        assert_ne!(intermediate.fov, 45.0);
        assert_eq!(camera.take_pose(), Some(intermediate), "a new screen starts from the visible inspection pose");
        assert!(camera.entry.is_none());
        camera.update(&input, Some(base), 0.6, false);
        assert!(camera.pose().is_none(), "discarded entry motion cannot overwrite the new screen");
        input.update_menu_buttons(&[Action::MenuZoomOut]);
        camera.update(&input, Some(base), 0.05, false);
        input.update_menu_buttons(&[]);
        camera.update(&input, Some(base), 0.6, false);
        assert_eq!(camera.pose().unwrap().fov, 45.0, "entry easing continues after releasing the input");
    }
}
