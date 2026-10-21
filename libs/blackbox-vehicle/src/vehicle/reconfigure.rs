//! Replace installed hardware between ticks without moving or repairing a vehicle.

use super::{Vehicle, VehicleSpec};
use crate::rigid_body::RigidBody;

impl Vehicle {
    pub fn set_spec(&mut self, spec: VehicleSpec) {
        let prepared = Self::new(spec.clone());
        let mut body = RigidBody::new(spec.mass, spec.dimension, spec.tensor_scale, spec.body);
        body.place(self.body.position, self.body.orientation);
        body.linear_velocity = self.body.linear_velocity;
        body.angular_velocity = self.body.angular_velocity;
        body.state = self.body.state;
        body.set_cog(self.body.cog());
        body.apply_force(self.body.pending_force());
        body.apply_torque(self.body.pending_torque());
        self.body = body;
        self.powertrain.reconfigure(
            spec.engine.clone(),
            spec.transmission.clone(),
            spec.induction,
            spec.nos,
            spec.tires.average_radius(),
        );
        self.geometry = prepared.geometry;
        self.params = prepared.params;
        self.spec = spec;
        if self.rolling_chassis.is_some() {
            self.set_rolling_chassis(self.rolling_chassis);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FIXED_STEP, FlatGround, InputState};

    #[test]
    fn hardware_replacement_preserves_motion_controls_and_damage() {
        let mut car = Vehicle::new(VehicleSpec::example());
        let ground = FlatGround::new(0.0);
        car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0);
        for _ in 0..120 {
            car.step(FIXED_STEP, &InputState { throttle: 0.5, ..Default::default() }, &ground);
        }
        car.tire_damage.puncture(0);
        car.tunings.handling = 0.3;
        car.can_recharge_nos = false;
        let before = (car.position(), car.orientation(), car.linear_velocity(), car.angular_velocity());
        let mut spec = car.spec().clone();
        spec.engine.torque.iter_mut().for_each(|t| *t *= 1.2);
        spec.tires.static_grip = [1.3, 1.3];
        car.set_spec(spec);
        assert_eq!(before, (car.position(), car.orientation(), car.linear_velocity(), car.angular_velocity()));
        assert_eq!(car.tunings.handling, 0.3);
        assert!(!car.can_recharge_nos);
        assert_eq!(car.spec().tires.static_grip, [1.3, 1.3]);
        assert_eq!(car.params[0].static_grip, 1.3);
        assert!(matches!(car.tire_damage.state(0), crate::tires::TireState::Punctured(_)));
    }
}
