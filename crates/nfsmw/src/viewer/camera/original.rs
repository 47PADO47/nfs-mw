//! Original PC gameplay cameras. Custom Chase and Dynamic FOV are independent.

mod anchor;
mod custom;
mod environment;
mod follow;
#[cfg(test)]
mod follow_tests;
mod impacts;
#[cfg(test)]
mod native_vectors;
mod noise;
mod obstruction;
mod projection;
mod spline;
mod timeline;
#[cfg(test)]
mod timeline_tests;
mod transients;
mod tweaks;
mod underpass;
mod vehicle_avoidance;

pub use anchor::drift;
pub use custom::{constrain_custom, custom_sample};
pub use environment::covered;
pub use follow::{BodyInput, Sample};
pub use impacts::Kind as ImpactKind;
pub use obstruction::CameraHit;
pub use timeline::{Frame, NoiseControls, PhysicsFrame, Timeline};
pub use underpass::VehicleBounds;
pub use vehicle_avoidance::Avoidable;

use crate::settings::CameraMode;
use crate::viewer::camera::Lens;

/// Native POV order. None denotes an independent custom controller.
pub fn index_of(mode: CameraMode) -> Option<usize> {
    Some(match mode {
        CameraMode::Bumper => 0,
        CameraMode::Hood => 1,
        CameraMode::Close => 2,
        CameraMode::Far => 3,
        CameraMode::SuperFar => 4,
        CameraMode::Drift => 5,
        CameraMode::Pursuit => 6,
        CameraMode::Chase | CameraMode::DynamicFov => return None,
    })
}

pub fn renders_car(mode: CameraMode, look_back: bool) -> bool {
    match index_of(mode) {
        Some(0) => false,
        Some(1) => !look_back,
        _ => true,
    }
}

impl Timeline {
    pub fn lens(&self, body: BodyInput, aspect: f32, interpolate: bool) -> Option<Lens> {
        let sample = self.sample(body, interpolate)?;
        Some(match self.is_custom() {
            true => custom::custom_lens(sample, aspect),
            false => projection::lens_finalized(sample, aspect),
        })
    }
}
