//! Native gameplay movers share the stock shaker, driven by their own anchor.
//! See `docs/specs/contact-camera-shake.md` for producer and application order.

use glam::Vec3;

use super::Sample;
use crate::viewer::camera::chase::shake;

#[derive(Clone, Copy)]
pub struct Explosion {
    pub position: Vec3,
    pub expansion_speed: f32,
    pub maximum_radius: f32,
}

impl Explosion {
    fn acceleration(self, eye: Vec3, seconds: f32) -> Option<Vec3> {
        let outward = eye - self.position;
        let distance = outward.length();
        if !seconds.is_finite()
            || seconds <= 0.0
            || !distance.is_finite()
            || distance <= 0.0
            || distance >= self.maximum_radius + 2.0
            || !self.expansion_speed.is_finite()
        {
            return None;
        }
        Some(outward * (self.expansion_speed / (distance * seconds)))
    }
}

#[derive(Default)]
pub(super) struct Shake(shake::Shake);

impl Shake {
    pub fn request(&mut self, acceleration: Vec3) {
        self.0.request(acceleration);
    }

    pub fn advance(&mut self, seconds: f32) {
        self.0.advance(seconds);
    }

    /// Scales the shake's amplitude by `intensity`, 0 to 1 (`[custom_hud] camera_shake`).
    pub fn set_intensity(&mut self, intensity: f32) {
        self.0.set_intensity(intensity);
    }

    /// The shake multiplier in force, which the camera noise also follows.
    pub fn intensity(&self) -> f32 {
        self.0.intensity()
    }

    /// Drops every pulse, keeping the intensity.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn explosions(&mut self, sources: &[Explosion], eye: Vec3, seconds: f32) {
        for source in sources {
            if let Some(acceleration) = source.acceleration(eye, seconds) {
                self.0.select(acceleration);
            }
        }
    }

    pub fn apply(&self, mut sample: Sample) -> Sample {
        let translation = self.0.sample(1.0);
        if translation == Vec3::ZERO {
            return sample;
        }
        let distance = sample.eye.distance(sample.target);
        let view = glam::camera::rh::view::look_at_mat4(sample.eye, sample.target, sample.up);
        let pose = shake::apply(view, translation).inverse();
        sample.eye = pose.w_axis.truncate();
        sample.target = sample.eye - pose.z_axis.truncate() * distance;
        sample.up = pose.y_axis.truncate();
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_explosion_camera_uses_strict_maximum_range_and_real_elapsed() {
        let source = Explosion { position: Vec3::ZERO, expansion_speed: 10.0, maximum_radius: 8.0 };
        assert!(source.acceleration(Vec3::ZERO, 1.0 / 60.0).is_none());
        assert!(source.acceleration(Vec3::Z * 10.0, 1.0 / 60.0).is_none());
        let near = source.acceleration(Vec3::Z, 1.0 / 60.0).unwrap();
        let far = source.acceleration(Vec3::Z * 9.99, 1.0 / 60.0).unwrap();
        assert!((near - Vec3::Z * 600.0).length() < 1e-4);
        assert!((near - far).length() < 1e-4, "the native request has no radial attenuation");
        let faster = source.acceleration(Vec3::Z, 1.0 / 120.0).unwrap();
        assert!((faster - near * 2.0).length() < 1e-4);
        assert!(source.acceleration(Vec3::Z, 0.0).is_none());
    }

    #[test]
    fn native_explosion_requests_preserve_body_first_and_source_order_admission() {
        let first = Explosion { position: -Vec3::Z, expansion_speed: 2.5, maximum_radius: 8.0 };
        let strongest = Explosion { expansion_speed: 10.0, ..first };
        let mut body_first = Shake::default();
        body_first.request(Vec3::Z * 150.0);
        body_first.advance(1.0 / 60.0);
        body_first.explosions(&[strongest], Vec3::ZERO, 1.0 / 60.0);
        assert!((body_first.0.sample(1.0).length() - 0.13).abs() < 1e-5);
        let mut source_first = Shake::default();
        source_first.advance(1.0 / 60.0);
        source_first.explosions(&[first, strongest], Vec3::ZERO, 1.0 / 60.0);
        assert!((source_first.0.sample(1.0).length() - 0.13).abs() < 1e-5);
        let mut reverse = Shake::default();
        reverse.advance(1.0 / 60.0);
        reverse.explosions(&[strongest, first], Vec3::ZERO, 1.0 / 60.0);
        assert!((reverse.0.sample(1.0).length() - 0.4).abs() < 1e-5);
    }
}
