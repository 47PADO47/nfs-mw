//! One object the host steps: all four cameras of a car and the choice between them.

use crate::chase::ChaseCamera;
use crate::motion::{CameraOptions, CameraPose, CameraView, CarMotion};
use crate::mounted::{MountedRig, RearViewRig};
use crate::params::CarCameras;

/// The camera model of one car. Step it **once per rendered frame**, after the car has moved, with that frame's
/// `dt` (it cuts `dt` to [`crate::MAX_DT`]). Every view is stepped each frame so that switching views never
/// starts from a stale history; only the chosen view's pose is returned.
#[derive(Debug, Clone)]
pub struct CameraState {
    chase: ChaseCamera,
    bonnet: MountedRig,
    bumper: MountedRig,
    rear: RearViewRig,
    view: CameraView,
}

impl CameraState {
    /// The model for the car with `cameras`, showing the chase view.
    pub fn new(cameras: &CarCameras, options: &CameraOptions) -> Self {
        let full_speed = cameras.external.speed_movement.shake.full_speed_mph;
        Self {
            chase: ChaseCamera::new(cameras, options),
            bonnet: MountedRig::new(&cameras.bonnet, options.bonnet_anchor, full_speed, options),
            bumper: MountedRig::new(&cameras.bumper, options.bumper_anchor, full_speed, options),
            rear: RearViewRig::new(&cameras.rear_view, options),
            view: CameraView::Chase,
        }
    }

    pub fn view(&self) -> CameraView {
        self.view
    }

    pub fn set_view(&mut self, view: CameraView) {
        self.view = view;
    }

    /// The next view in [`CameraView::ALL`] order (the host's "change camera" button); returns it.
    pub fn next_view(&mut self) -> CameraView {
        let all = CameraView::ALL;
        let at = all.iter().position(|&view| view == self.view).unwrap_or(0);
        self.view = all[(at + 1) % all.len()];
        self.view
    }

    /// Put every camera at its rest pose with no spring history, and return the chosen view's pose.
    pub fn reset(&mut self, motion: &CarMotion) -> CameraPose {
        let chase = self.chase.reset(motion);
        let bonnet = self.bonnet.reset(motion);
        let bumper = self.bumper.reset(motion);
        self.choose(chase, bonnet, bumper, motion)
    }

    /// Advance all cameras by one frame and return the chosen view's pose.
    pub fn step(&mut self, motion: &CarMotion) -> CameraPose {
        let chase = self.chase.step(motion);
        let bonnet = self.bonnet.step(motion);
        let bumper = self.bumper.step(motion);
        self.choose(chase, bonnet, bumper, motion)
    }

    /// The chase camera, for hooks such as [`ChaseCamera::add_impact`].
    pub fn chase_mut(&mut self) -> &mut ChaseCamera {
        &mut self.chase
    }

    /// The wind buffeting of the bonnet view at `speed_mps`, 0..1 of the data's strength.
    pub fn wind_shake(&self, speed_mps: f32) -> f32 {
        self.bonnet.wind_shake(speed_mps)
    }

    fn choose(&self, chase: CameraPose, bonnet: CameraPose, bumper: CameraPose, motion: &CarMotion) -> CameraPose {
        match self.view {
            CameraView::Chase => chase,
            CameraView::Bonnet => bonnet,
            CameraView::Bumper => bumper,
            CameraView::RearView => self.rear.pose(motion),
        }
    }
}
