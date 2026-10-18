//! Session vehicle heat, contact and arrest gates shared by patrol and pursuit.

use super::*;

impl Police {
    pub fn vehicle_heat(&self) -> f32 {
        self.heat.map_or(self.carried_heat.max(1.0), |heat| heat.value())
    }

    pub fn patrol_heat(&self) -> u32 {
        (self.vehicle_heat() as u32).clamp(1, 10)
    }

    pub fn has_contact(&self) -> bool {
        self.seen
    }

    /// The pursuit's tracked sight includes its seven-second grace after raw visibility is lost.
    pub fn is_perp_in_sight(&self) -> bool {
        self.heat.is_some() && self.cooldown.since_seen() < blackbox_pursuit::cooldown::TUNING.grace
    }

    pub fn can_begin(&self) -> bool {
        !self.bust.active()
    }

    pub(super) fn helicopter_sees(&self) -> bool {
        self.heli.as_ref().is_some_and(|heli| {
            !matches!(heli.mode, blackbox_pursuit::helicopter::Mode::Exit(_)) && heli.sight.visible()
        })
    }
}
