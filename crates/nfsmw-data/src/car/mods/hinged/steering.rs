//! A hinged part the steering turns: the steering wheel of a mod car (`[damage_parts.steering]`,
//! `docs/modding/car-damage-parts.md`).
//!
//! The part turns about its pivot and axis by the steering input times the lock angle. The input is the car's control value
//! (-1 full left to 1 full right, the one the driver's pose is picked by), so the wheel moves continuously with it and agrees
//! with the driver's lean. The axis points forward from the driver's eye, so a positive angle (right-handed about it) is
//! clockwise as the driver sees it: a turn to the right.

/// How the steering turns a part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Steering {
    /// The degrees the part turns at full steering, each way.
    pub lock_degrees: f32,
}

impl Steering {
    /// The degrees the part has turned at `steering` (-1 to 1, positive to the right). A value outside the range counts as
    /// full lock, and one that is not a number as straight ahead.
    pub fn angle(&self, steering: f32) -> f32 {
        if !steering.is_finite() {
            return 0.0;
        }
        steering.clamp(-1.0, 1.0) * self.lock_degrees
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3};

    use super::*;
    use crate::car::mods::hinged::Hinge;

    fn wheel() -> Hinge {
        Hinge {
            name: "SteeringWheel".into(),
            model: "CAR_KIT00_STEERING_WHEEL".into(),
            zones: vec![0],
            pivot: [0.3, 0.4, 0.8],
            axis: [1.0, 0.0, 0.0],
            angle: 0.0,
            stages: vec![0.0],
            stage_angles: Vec::new(),
            detach_at: None,
            detach: false,
            mass_kg: 2.0,
            active: None,
            ancestors: Vec::new(),
            tracks: Vec::new(),
            drives: Vec::new(),
            wing: None,
            steering: Some(Steering { lock_degrees: 450.0 }),
        }
    }

    #[test]
    fn the_angle_is_the_steering_times_the_lock_and_stops_at_full_lock() {
        let lock = Steering { lock_degrees: 450.0 };
        assert_eq!(lock.angle(0.0), 0.0);
        assert_eq!(lock.angle(1.0), 450.0);
        assert_eq!(lock.angle(-1.0), -450.0);
        assert_eq!(lock.angle(0.5), 225.0, "linear, no steps");
        assert_eq!(lock.angle(3.0), 450.0);
        assert_eq!(lock.angle(f32::NAN), 0.0);
    }

    #[test]
    fn a_right_turn_is_clockwise_for_the_driver_and_the_hub_stays() {
        let hinge = wheel();
        assert_eq!(hinge.steered(0.0), Mat4::IDENTITY);
        let hub = Vec3::from_array(hinge.pivot);
        let right = hinge.steered(0.25);
        assert!((right.transform_point3(hub) - hub).length() < 1e-5, "the pivot stays");
        // The driver looks forward (+x) with up +z and left +y. A rim point at the top (+z) of the wheel goes to the driver's
        // right (-y) when the wheel is turned to the right.
        let top = hub + Vec3::Z * 0.17;
        let moved = right.transform_point3(top) - hub;
        assert!(moved.y < -0.1, "the top of the rim moves to the right: {moved}");
        let left = hinge.steered(-0.25).transform_point3(top) - hub;
        assert!(left.y > 0.1, "and to the left for a left turn: {left}");
        assert!((moved.length() - 0.17).abs() < 1e-5, "the rim stays on its circle");
    }

    #[test]
    fn a_part_the_steering_does_not_turn_stays_put() {
        let plain = Hinge { steering: None, ..wheel() };
        assert_eq!(plain.steered(1.0), Mat4::IDENTITY);
    }
}
