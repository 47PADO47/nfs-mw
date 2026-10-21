//! Named lamp events mutate independent damage, not cosmetic panel severity.
use super::VehicleEvents;

impl VehicleEvents {
    /// Destruction overrides the independent mask until full repair.
    pub fn broken_lights(&self) -> u32 {
        match self.destroyed {
            true => u32::MAX,
            false => self.light_damage,
        }
    }

    pub fn repair_lights(&mut self) {
        self.destroyed = false;
        self.light_damage = 0;
    }

    pub(super) fn set_light_damage(&mut self, name: u32, broken: bool) -> bool {
        let Some(mask) = nfsmw_data::car::lights::damage_mask(name) else { return false };
        self.light_damage = match broken {
            true => self.light_damage | mask,
            false => self.light_damage & !mask,
        };
        true
    }
}
