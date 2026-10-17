//! The cracked-glass windows of the player's car: the zones the damage has cracked, handed to the rig when they change.

use glam::Vec3;

use super::Drive;

/// The cracked zones this frame and the set last handed to the rig. Both keep their capacity, so checking every
/// frame allocates nothing; the rig only takes a new set when this one differs.
#[derive(Default)]
pub(super) struct CrackedZones {
    now: Vec<usize>,
    sent: Vec<usize>,
}

impl Drive {
    /// Show the cracked-glass texture on the windows the damage has cracked.
    pub fn set_damaged_windows(&mut self, renderer: &mut dyn blackbox_gfx::RenderBackend, to_sun: Vec3) {
        let Some(sim) = self.sim.as_ref() else { return };
        sim.cracked_zones_into(&mut self.cracked.now);
        if self.cracked.now == self.cracked.sent {
            return;
        }
        self.cracked.sent.clone_from(&self.cracked.now);
        self.rig.set_cracked_zones(renderer, self.cracked.sent.clone(), to_sun);
    }
}
