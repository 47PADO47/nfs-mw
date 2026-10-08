use crate::math::inch_to_m;

/// Tire parameters (the attribute class `tires`). Per-axle arrays are `[front, rear]`.
#[derive(Clone, Debug)]
pub struct TireSpec {
    /// Rim diameter in inches.
    pub rim_size: [f32; 2],
    /// Section width in millimetres.
    pub section_width: [f32; 2],
    /// Sidewall height as a percentage of the section width.
    pub aspect_ratio: [f32; 2],
    /// Multiplier of the lateral force curve; also divides the steering drag.
    pub grip_scale: [f32; 2],
    /// Friction coefficient at the limit of grip.
    pub static_grip: [f32; 2],
    /// Friction coefficient while sliding.
    pub dynamic_grip: [f32; 2],
    /// Multiplier of the steering angle range.
    pub steering: f32,
    /// Stability-assist strength against speed (1 to 4 samples, linearly interpolated over 0..top speed).
    pub yaw_control: Vec<f32>,
    /// Multiplier on the yaw torque of the tire forces (1 = unmodified).
    pub yaw_speed: f32,
}

impl TireSpec {
    /// Outer diameter of the tire of an axle (0 front, 1 rear) in metres.
    pub fn wheel_diameter(&self, axle: usize) -> f32 {
        inch_to_m(self.rim_size[axle]) + 2.0 * self.section_width[axle] * 0.001 * self.aspect_ratio[axle] * 0.01
    }

    /// Rolling radius of an axle in metres, at least 0.1.
    pub fn radius(&self, axle: usize) -> f32 {
        (self.wheel_diameter(axle) * 0.5).max(0.1)
    }

    /// Mean wheel radius of both axles (m).
    pub fn average_radius(&self) -> f32 {
        0.5 * (self.radius(0) + self.radius(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diameter_adds_two_sidewalls_to_the_rim() {
        let t = TireSpec {
            rim_size: [17.0, 17.0],
            section_width: [225.0, 225.0],
            aspect_ratio: [45.0, 45.0],
            grip_scale: [1.0; 2],
            static_grip: [1.0; 2],
            dynamic_grip: [1.0; 2],
            steering: 1.0,
            yaw_control: vec![],
            yaw_speed: 1.0,
        };
        // 17 in = 0.4318 m, two sidewalls of 0.10125 m.
        assert!((t.wheel_diameter(0) - 0.63430).abs() < 1e-4, "{}", t.wheel_diameter(0));
        assert!((t.radius(1) - 0.31715).abs() < 1e-4);
    }
}
