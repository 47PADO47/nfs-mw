//! What a mod's extra camera view reads of the player's car (`bb.camera` 1.1): the body as the presented car shows it, and the
//! controls and the strongest hit since the last time it was asked.

use glam::{Quat, Vec3};

use super::Drive;
use crate::viewer::camera::original::BodyInput;

/// Closing speed of a hit, m/s, that counts as a full-strength impact for a camera.
const FULL_IMPACT_SPEED: f32 = 20.0;

/// The car for an extra view.
pub struct ExtraViewBody {
    /// Velocity and ground contact of the presented body, render space. Its position and axes are the car model's (see
    /// [`model_frame`]), not those of the physics body's centre of gravity.
    pub body: BodyInput,
    /// Steering, -1 to 1, positive to the left.
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    /// Physics step the body was taken at; an impact is reported once per step.
    pub step: u32,
    /// The strongest hit of this step, 0 to 1.
    pub impact: f32,
}

/// The model's origin and its forward and up axes, for a car pose of `position` and `rotation` (the model's frame: forward,
/// left, up). A camera's offsets are measured from the model's origin, on the ground between the axles, so a view is
/// stepped with that point and not with the body's centre of gravity, which sits half a metre above it and ahead of it.
fn model_frame(position: Vec3, rotation: Quat) -> (Vec3, Vec3, Vec3) {
    (position, rotation * Vec3::X, rotation * Vec3::Z)
}

impl Drive {
    /// The car for an extra camera view; `None` before the car has a body. `last_step` is the step of the previous call, so
    /// a hit is reported once.
    pub fn extra_view_body(&self, last_step: Option<u32>) -> Option<ExtraViewBody> {
        let mut body = self.original_body(true)?;
        let sim = self.sim.as_ref()?;
        let shown = self.pose();
        (body.position, body.forward, body.up) = model_frame(shown.position, shown.rotation);
        let step = self.original_step();
        let impact = match last_step == Some(step) {
            true => 0.0,
            false => self.original_impacts().iter().map(|(_, closing)| *closing).fold(0.0_f32, f32::max),
        };
        Some(ExtraViewBody {
            body,
            // The physics steers positive to the right; the ABI is positive to the left.
            steer: -sim.steering_input(),
            throttle: sim.gas_pedal(),
            brake: sim.brake_pedal(),
            step,
            impact: (impact / FULL_IMPACT_SPEED).clamp(0.0, 1.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_frame_is_the_origin_and_the_models_forward_and_up() {
        let at = Vec3::new(10.0, -4.0, 2.0);
        let (position, forward, up) = model_frame(at, Quat::IDENTITY);
        assert_eq!((position, forward, up), (at, Vec3::X, Vec3::Z));
        // Turned a quarter to the left, the nose points along +y and the roof stays up.
        let (_, forward, up) = model_frame(at, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        assert!((forward - Vec3::Y).length() < 1e-5 && (up - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn a_full_impact_is_twenty_metres_per_second_of_closing_speed() {
        assert_eq!((FULL_IMPACT_SPEED / FULL_IMPACT_SPEED).clamp(0.0, 1.0), 1.0);
        assert_eq!((40.0_f32 / FULL_IMPACT_SPEED).clamp(0.0, 1.0), 1.0);
        assert_eq!((10.0_f32 / FULL_IMPACT_SPEED).clamp(0.0, 1.0), 0.5);
    }
}
