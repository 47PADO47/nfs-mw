//! The car's dynamics as the mods read them, its nitrous tank as they fill it and its drift assist as they switch it
//! (`bb.vehicles` 1.1 and 1.2, see `drive/mod_feed.rs`).

use super::CarSim;

impl CarSim {
    /// Body slip angle in radians, positive when the car moves to the left of its heading (the physics counts a slide to the
    /// right as positive). 0 below 1 m/s forward speed.
    pub(in crate::scenes::world) fn slip_to_left(&self) -> f32 {
        -self.vehicle.slip_angle()
    }

    /// Steering, -1 to 1, positive to the left (the physics steers positive to the right).
    pub(in crate::scenes::world) fn steering_to_left(&self) -> f32 {
        -self.steering_input()
    }

    /// At least one wheel touches the ground.
    pub(in crate::scenes::world) fn on_ground(&self) -> bool {
        self.vehicle.wheels_on_ground() > 0
    }

    /// Turn the physics' drift assist on or off (`bb.vehicles` 1.2).
    pub(in crate::scenes::world) fn set_drift_assist(&mut self, on: bool) {
        self.vehicle.set_drift_assist(on);
    }

    /// Add `amount` of a full tank to the nitrous; whether the car has a nitrous system.
    pub(in crate::scenes::world) fn add_nitrous(&mut self, amount: f32) -> bool {
        self.vehicle.add_nitrous(amount)
    }
}
