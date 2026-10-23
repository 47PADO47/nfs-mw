//! Player window and brake-surface damage, sharing lazily prepared appearances.

use glam::Vec3;

use super::{Drive, SurfaceDamage};

/// Reuse zone-vector capacity; the rig reuses each requested appearance until its state changes.
#[derive(Default)]
pub(super) struct CrackedZones {
    now: Vec<usize>,
    sent: Vec<usize>,
}

impl CrackedZones {
    pub(super) fn mask(&self) -> u16 {
        self.sent.iter().filter(|&&zone| zone < 10).fold(0, |mask, &zone| mask | (1 << zone))
    }
}

impl Drive {
    /// Show the cracked-glass texture on the windows the damage has cracked.
    pub fn set_damaged_windows(&mut self, renderer: &mut dyn blackbox_gfx::RenderBackend, to_sun: Vec3) {
        let Some(sim) = self.sim.as_ref() else { return };
        sim.cracked_zones_into(&mut self.cracked.now);
        self.cracked.sent.clone_from(&self.cracked.now);
        let state = SurfaceDamage::new(self.cracked.mask(), sim.lamp_damage());
        self.rig.prepare_surface_slice(renderer, &[state], to_sun);
    }
}
