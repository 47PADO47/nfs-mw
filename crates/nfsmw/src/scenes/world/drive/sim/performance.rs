use super::CarSim;
use nfsmw_data::car::physics::CarPhysics;

impl CarSim {
    pub(in crate::scenes::world::drive) fn set_tunings(&mut self, tunings: blackbox_vehicle::Tunings) {
        self.vehicle.tunings = tunings;
    }

    #[cfg(test)]
    pub(in crate::scenes::world) fn tunings(&self) -> blackbox_vehicle::Tunings {
        self.vehicle.tunings
    }

    pub(in crate::scenes::world::drive) fn speedbreaker_speed(&self) -> f32 {
        self.vehicle.powertrain().speedometer().abs()
    }
    pub(in crate::scenes::world::drive) fn drift_charge(&self) -> (f32, f32, f32) {
        (self.vehicle.drift_strength(), self.vehicle.drift_speed(), self.vehicle.slip_angle())
    }
    pub(in crate::scenes::world::drive) fn set_performance(&mut self, physics: &CarPhysics) {
        self.vehicle.set_spec(physics.spec.clone());
    }
    pub(in crate::scenes::world::drive) fn set_nos_recharge(&mut self, allowed: bool) {
        self.vehicle.can_recharge_nos = allowed;
    }
    pub(in crate::scenes::world::drive) fn set_speedbreaker_effect(&mut self, active: bool) {
        self.vehicle.maneuver = match active {
            true => blackbox_vehicle::vehicle::ManeuverAssist {
                extra_downward_acceleration: 2.0 * 9.8128,
                tire_steering_drag: Some(1.0),
                collision_mass_scale: Some(2.0),
                drag_scale: 0.25,
                front_grip_gain: 0.75,
                steering_degrees: Some(60.0),
            },
            false => Default::default(),
        };
    }
}
