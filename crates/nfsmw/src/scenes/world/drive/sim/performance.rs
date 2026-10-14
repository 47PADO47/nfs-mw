use super::CarSim;
use blackbox_vehicle::ManeuverAssist;
use nfsmw_data::car::physics::CarPhysics;

use crate::settings::speedbreaker::PhysicsModifiers;

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
    /// The Speedbreaker's vehicle effects while it is active; the neutral assist otherwise.
    pub(in crate::scenes::world::drive) fn set_speedbreaker_effect(
        &mut self,
        active: bool,
        physics: &PhysicsModifiers,
    ) {
        self.vehicle.maneuver = match active {
            true => breaker_assist(physics),
            false => Default::default(),
        };
    }
}

/// The PC gravity the downforce is scaled from (`rigidbodyspecs.GRAVITY`, see docs/specs/speedbreaker.md).
const GRAVITY: f32 = 9.8128;

/// The vehicle effects of the Speedbreaker. The extra downforce is `(gravity_scale - 1)` g, so the vanilla
/// scale 3 gives two extra g, as the PC reference does.
fn breaker_assist(physics: &PhysicsModifiers) -> ManeuverAssist {
    ManeuverAssist {
        extra_downward_acceleration: (physics.gravity_scale - 1.0) * GRAVITY,
        tire_steering_drag: Some(physics.slip_drag),
        collision_mass_scale: Some(physics.car_mass_scale),
        drag_scale: physics.aero_drag_scale,
        front_grip_gain: physics.front_grip_gain,
        steering_degrees: Some(physics.max_steering_degrees),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_tuning_gives_the_verified_vehicle_effects() {
        let assist = breaker_assist(&PhysicsModifiers::default());
        assert_eq!(assist.extra_downward_acceleration, 2.0 * GRAVITY);
        assert_eq!(assist.tire_steering_drag, Some(1.0));
        assert_eq!(assist.collision_mass_scale, Some(2.0));
        assert_eq!((assist.drag_scale, assist.front_grip_gain), (0.25, 0.75));
        assert_eq!(assist.steering_degrees, Some(60.0));
    }

    #[test]
    fn configured_scales_pass_through_to_the_assist() {
        let physics = PhysicsModifiers { gravity_scale: 1.0, car_mass_scale: 4.0, ..PhysicsModifiers::default() };
        let assist = breaker_assist(&physics);
        assert_eq!(assist.extra_downward_acceleration, 0.0, "a gravity scale of 1 adds no downforce");
        assert_eq!(assist.collision_mass_scale, Some(4.0));
    }
}
