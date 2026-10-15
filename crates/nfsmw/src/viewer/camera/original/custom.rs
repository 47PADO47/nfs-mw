//! Custom-controller optics with shared outside-view clearance geometry.
use super::{Avoidable, CameraHit, Sample, obstruction::Obstruction, vehicle_avoidance::Avoidance};
use crate::viewer::camera::Lens;
use glam::Vec3;

const UNITS: f32 = 65536.0 / std::f32::consts::TAU;

/// Record the final unoccluded custom shot, including existing shake, before Speedbreaker optics.
pub fn custom_sample(lens: Lens) -> Sample {
    Sample {
        eye: lens.position,
        target: lens.position + lens.forward,
        up: lens.view.inverse().y_axis.truncate().normalize_or(Vec3::Z),
        fov_bangle: lens.fov_y_degrees.to_radians() * UNITS,
    }
}

pub fn custom_lens(sample: Sample, aspect: f32) -> Lens {
    let mut lens = Lens::look_at(sample.eye, sample.target, (sample.fov_bangle / UNITS).to_degrees(), 0.3, aspect);
    lens.view = glam::camera::rh::view::look_at_mat4(sample.eye, sample.target, sample.up);
    lens
}

pub(super) fn horizontal(mut sample: Sample, aspect: f32) -> Sample {
    sample.fov_bangle = 2.0 * ((sample.fov_bangle / UNITS * 0.5).tan() * aspect).atan() * UNITS;
    sample
}

/// Final hard clearance is stateless; repeated draws cannot change recovery speed.
pub fn constrain_custom(
    car: Vec3,
    lens: Lens,
    aspect: f32,
    covered: bool,
    bodies: &[Avoidable],
    cast: impl FnMut(Vec3, Vec3) -> Option<CameraHit>,
    ground: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
) -> Lens {
    let sample = horizontal(custom_sample(lens), aspect);
    let sample = Obstruction::constrain(car, sample, covered, cast, ground);
    let mut sample = Avoidance::constrain(car, sample, bodies);
    sample.fov_bangle = lens.fov_y_degrees.to_radians() * UNITS;
    custom_lens(sample, aspect)
}

/// A milder custom effect, using Far's retained eye/FOV response durations.
/// Native view targets are unchanged; the free orbit keeps its own framing.
#[derive(Default)]
pub(super) struct Breaker {
    lag: super::spline::Scalar,
    fov: super::spline::Scalar,
    initialized: bool,
}

impl Breaker {
    pub fn apply(&mut self, mut sample: Sample, body: super::BodyInput) -> Sample {
        let zoom = 1.0 - 0.5 * (1.0 - body.zoom.clamp(0.6, 1.0));
        if !self.initialized {
            self.lag.target(1.0);
            self.lag.snap();
            self.fov.target(1.0);
            self.fov.snap();
            self.initialized = true;
        }
        let response = super::tweaks::Tweaks::at(3, body.speed);
        self.lag.advance(super::timeline::STEP as f32, response.eye_duration);
        self.fov.advance(super::timeline::STEP as f32, response.fov_duration);
        self.lag.target(zoom.recip());
        self.fov.target(zoom);
        let offset = (sample.eye - body.position) * Vec3::new(1.0, 1.0, 0.0);
        let displacement = offset * (self.lag.value.clamp(1.0, 1.25) - 1.0);
        sample.eye += displacement;
        sample.target += displacement;
        sample.fov_bangle *= self.fov.value.clamp(0.8, 1.0);
        sample
    }
}

#[cfg(test)]
mod tests;
