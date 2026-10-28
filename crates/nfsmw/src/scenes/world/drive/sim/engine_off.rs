//! The gearbox of a car whose engine is off (`bb.engine`, `drive/engine_switch.rs`).

use super::CarSim;

impl CarSim {
    /// Hold the gearbox in neutral while the engine is not running, and let it go when it runs again. The box then leaves
    /// neutral by the car's own rules: an automatic one takes first (or reverse when the brake asks for it), a manual one
    /// waits for the driver.
    pub(in crate::scenes::world) fn set_neutral_hold(&mut self, on: bool) {
        self.vehicle.set_neutral_hold(on);
    }
}
