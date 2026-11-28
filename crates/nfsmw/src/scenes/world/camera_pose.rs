//! The exact pose of the drawn view (the status line's `cam …` line) and the `cam` command that sets the free camera to one, so a
//! picture's camera can be set up again for a graphics comparison.

use crate::viewer::camera::pose::{self, Pose};

use super::{View, WorldScene};

impl WorldScene {
    /// The pose the world was last drawn from: the lens of whichever camera is active (the chase camera's eye, not the car), at
    /// the aspect of the last frame. None while the garage's backdrop is drawn instead of the world.
    pub(super) fn view_pose(&self) -> Option<Pose> {
        if self.garage_backdrop.is_some() {
            return None;
        }
        Some(Pose::from_lens(&self.gameplay_lens(self.drawn_aspect.max(0.1))))
    }

    /// Put the free camera exactly at `pose`. In local single player it is the F5 free camera, which keeps a field of view
    /// (70 degrees unless given) and drops the roll; a session's free camera has a fixed field of view.
    fn place_camera(&mut self, pose: Pose) -> String {
        let (yaw, pitch) = pose.yaw_pitch_radians();
        self.camera.position = pose.eye;
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;
        self.view = View::Fly;
        self.cut.arm();
        // A `goto` still waiting for its ground must not move the camera to its height above it.
        self.grounded = true;
        if !self.extra_options_allowed() {
            let note = match pose.fov {
                Some(_) => " (the field of view stays 70 degrees outside local single player)",
                None => "",
            };
            return format!("free camera at {}{note}", Pose { fov: None, ..pose }.line());
        }
        self.extra.freecam = true;
        self.extra.selected_bookmark = None;
        self.extra.free_roll = 0.0;
        let fov = pose.fov.unwrap_or(crate::viewer::camera::FlyCamera::FOV_Y_DEGREES);
        self.extra.free_fov = Some(fov);
        format!("free camera at {} (F or `freecam` returns to the car)", Pose { fov: Some(fov), ..pose }.line())
    }
}

/// `cam <x> <y> <z> <heading> <pitch> [fov]`: the free camera at that absolute pose; a line from the status line pastes as is.
pub(super) fn command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return scene.view_pose().map(|pose| pose.line()).ok_or_else(|| format!("usage: {}", pose::USAGE));
    }
    Pose::parse(args).map(|pose| scene.place_camera(pose))
}
