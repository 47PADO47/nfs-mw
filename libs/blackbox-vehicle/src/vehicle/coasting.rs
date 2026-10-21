//! Optional caller-selected no-powered-slip contact policy.

use super::Vehicle;
use crate::tires::CoastingTireResponse;

impl Vehicle {
    pub fn set_coasting_tires(&mut self, response: Option<CoastingTireResponse>) {
        self.coasting_tires = response;
    }

    pub(super) fn coasting_controls(&mut self) -> bool {
        let Some(response) = self.coasting_tires else { return false };
        for tire in &mut self.tires {
            tire.brake = self.controls.brake;
            tire.ebrake = 0.0;
            tire.drag_reduction = response.corner_drag;
        }
        true
    }

    pub(super) fn coasting_drive(&mut self, torque: f32, grounded: [bool; 4]) -> bool {
        if self.coasting_tires.is_none() {
            return false;
        }
        let front = self.spec.transmission.torque_split;
        for (i, tire) in self.tires.iter_mut().enumerate() {
            if !grounded[i] {
                continue;
            }
            let share = match i < 2 {
                true => front,
                false => 1.0 - front,
            };
            tire.drive_torque = torque * share * 0.5;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FIXED_STEP, FlatGround, InputState, VehicleSpec};

    #[test]
    fn coasting_policy_reaches_loaded_wheels_and_direct_negative_axle_torque() {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_coasting_tires(Some(CoastingTireResponse {
            angle_range: 12.0 / 360.0,
            lateral_gain: 1.54,
            brake_torque_scale: 2.5,
            brake_force_gain: 2.5,
            corner_drag: 0.5,
        }));
        car.distribute_drive(-1000.0, [0.0; 4], [true; 4]);
        let front = car.spec.transmission.torque_split;
        assert_eq!(
            car.tires.map(|t| t.drive_torque),
            [-500.0 * front, -500.0 * front, -500.0 * (1.0 - front), -500.0 * (1.0 - front)]
        );
        let ground = FlatGround::new(0.0);
        assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
        for _ in 0..120 {
            car.step(FIXED_STEP, &InputState::default(), &ground);
        }
        car.body.linear_velocity = glam::Vec3::new(10.0, 0.0, 30.0);
        let mut supported = 0;
        for _ in 0..30 {
            car.step(FIXED_STEP, &InputState { brake: 0.25, ..Default::default() }, &ground);
            for (tire, p) in car.tires.iter().zip(car.params).filter(|(t, _)| t.load > 0.0) {
                supported += 1;
                assert_eq!(tire.brake, 0.25);
                assert_eq!(tire.slip, 0.0);
                assert!(!tire.brake_locked);
                assert_eq!(tire.av, tire.road_speed / p.radius);
            }
        }
        assert!(supported > 0);
        car.set_coasting_tires(None);
        assert!(!car.coasting_controls(), "live policy restoration must be immediate");
    }
}
