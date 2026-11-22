//! The car's dynamics as the mods read them, its nitrous tank as they fill it and its drift assist as they switch it
//! (`bb.vehicles` 1.1 and 1.2, see `drive/mod_feed.rs`).

use super::CarSim;

/// The role tag the tyre model of a mod is told for the player's car (`BB_ROLE_PLAYER`).
const PLAYER_ROLE: u32 = blackbox_modding::abi::BB_ROLE_PLAYER;

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
        self.wheels_down() > 0
    }

    /// Turn the physics' drift assist on or off (`bb.vehicles` 1.2).
    pub(in crate::scenes::world) fn set_drift_assist(&mut self, on: bool) {
        self.vehicle.set_drift_assist(on);
    }

    /// The physical summary of the car (`bb.vehicles` 1.3).
    pub(in crate::scenes::world) fn vehicle_summary(&self) -> blackbox_vehicle::VehicleSummary {
        self.vehicle.summary()
    }

    /// Whether a mod's tyre model is on the car.
    pub(in crate::scenes::world) fn has_tyre_model(&self) -> bool {
        self.vehicle.has_tyre_model()
    }

    /// Put a mod's tyre model on the car. It is for the player's car: the role tag is the host's player role (0).
    pub(in crate::scenes::world) fn set_tyre_model(&mut self, model: Box<dyn blackbox_vehicle::tires::TyreModel>) {
        self.vehicle.set_tyre_model(model, PLAYER_ROLE);
    }

    /// Take a mod's tyre model off the car.
    pub(in crate::scenes::world) fn clear_tyre_model(&mut self) {
        if self.vehicle.has_tyre_model() {
            self.vehicle.clear_tyre_model();
        }
    }

    /// Whether a mod's suspension model is on the car.
    pub(in crate::scenes::world) fn has_suspension_model(&self) -> bool {
        self.vehicle.has_suspension_model()
    }

    /// Put a mod's suspension model on the car (the player's role tag, as for the tyre model).
    pub(in crate::scenes::world) fn set_suspension_model(
        &mut self,
        model: Box<dyn blackbox_vehicle::suspension::SuspensionModel>,
    ) {
        self.vehicle.set_suspension_model(model, PLAYER_ROLE);
    }

    /// Take a mod's suspension model off the car.
    pub(in crate::scenes::world) fn clear_suspension_model(&mut self) {
        if self.vehicle.has_suspension_model() {
            self.vehicle.clear_suspension_model();
        }
    }

    /// Whether a mod's controls model is on the car.
    pub(in crate::scenes::world) fn has_controls_model(&self) -> bool {
        self.vehicle.has_controls_model()
    }

    /// Put a mod's controls model on the car (the player's role tag, as for the tyre model).
    pub(in crate::scenes::world) fn set_controls_model(
        &mut self,
        model: Box<dyn blackbox_vehicle::input::ControlsModel>,
    ) {
        self.vehicle.set_controls_model(model, PLAYER_ROLE);
    }

    /// Take a mod's controls model off the car.
    pub(in crate::scenes::world) fn clear_controls_model(&mut self) {
        if self.vehicle.has_controls_model() {
            self.vehicle.clear_controls_model();
        }
    }

    /// Add `amount` of a full tank to the nitrous; whether the car has a nitrous system.
    pub(in crate::scenes::world) fn add_nitrous(&mut self, amount: f32) -> bool {
        self.vehicle.add_nitrous(amount)
    }
}
