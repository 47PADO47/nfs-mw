//! What the mods read of the player's car beyond the HUD's numbers, and the one thing they change in it (`bb.vehicles` 1.1,
//! docs/modding/api-world.md): the strongest hit of the latest physics step, and the nitrous tank.

use super::Drive;
use crate::audio::ContactSource;

/// Closing speed of a hit, m/s, that counts as a full-strength hit for a mod (the same as a camera's, see `extra_view.rs`).
const FULL_HIT_SPEED: f32 = 20.0;

impl Drive {
    /// The strongest hit of the latest physics step against the world or an object, 0 to 1. The ground does not count, so a
    /// landing is not a hit. The same contacts are reported until the physics steps again.
    pub(in crate::scenes::world) fn mod_impact(&self) -> f32 {
        if self.request.is_some() || self.sim.is_none() {
            return 0.0;
        }
        let strongest = self
            .sound
            .camera_contacts()
            .iter()
            .filter(|contact| !matches!(contact.source, ContactSource::Ground))
            .map(|contact| contact.closing_speed())
            .filter(|closing| closing.is_finite())
            .fold(0.0_f32, f32::max);
        (strongest / FULL_HIT_SPEED).clamp(0.0, 1.0)
    }

    /// Add `amount` of a full tank to the nitrous (a rule a mod runs pays it). Returns whether the car has a tank to fill.
    pub(in crate::scenes::world) fn add_nitrous(&mut self, amount: f32) -> bool {
        let Some(sim) = self.sim.as_mut() else {
            return false;
        };
        let changed = sim.add_nitrous(amount);
        self.telemetry.nos = sim.telemetry().nos;
        changed
    }
}
