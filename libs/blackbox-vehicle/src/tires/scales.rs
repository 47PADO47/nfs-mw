//! Speed-dependent per-step scales shared by all tires (spec section 3.7 and 3.8).

use crate::math::{mph_to_ms, ramp, table};

const TRACTION_TABLE: [f32; 10] = [0.909, 1.045, 1.09, 1.09, 1.09, 1.09, 1.09, 1.045, 1.0, 1.0];
const GRIP_TABLE: [f32; 10] = [0.833, 0.958, 1.008, 1.0167, 1.033, 1.033, 1.033, 1.0167, 1.0, 1.0];

/// The three scales computed once per step from the body speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepScales {
    /// Slip speed (m/s) beyond which a wheel counts as lost.
    pub max_slip: f32,
    /// Multiplier of the lateral force curve.
    pub grip_scale: f32,
    /// Multiplier of both friction limits.
    pub traction_scale: f32,
}

impl StepScales {
    pub fn new(speed: f32, reverse: bool) -> Self {
        let r = ramp(speed, 0.0, mph_to_ms(85.0));
        let max_slip = 0.5 + ramp(speed, 10.0, 71.0);
        let traction_scale = if reverse { 2.0 } else { table(&TRACTION_TABLE, 0.0, 1.0, r) * 1.1 };
        Self { max_slip, grip_scale: table(&GRIP_TABLE, 0.0, 1.0, r) * 1.2, traction_scale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grip_grows_with_speed_and_flattens() {
        let slow = StepScales::new(0.0, false);
        let mid = StepScales::new(15.0, false);
        let fast = StepScales::new(60.0, false);
        assert!((slow.grip_scale - 0.833 * 1.2).abs() < 1e-5);
        assert!(mid.grip_scale > slow.grip_scale);
        assert!((fast.grip_scale - 1.2).abs() < 1e-5);
        assert!(slow.max_slip < fast.max_slip);
    }

    #[test]
    fn reverse_has_double_traction() {
        assert_eq!(StepScales::new(3.0, true).traction_scale, 2.0);
    }
}
