//! What the cinematic moment of a pursuit breaker asks of the police: the cops chasing the player, and a hold on the
//! busted meter while the cut plays. Spec: `docs/specs/pursuit-breaker-moment.md`.

use glam::Vec3;

use super::TrafficWorld;

impl TrafficWorld {
    /// The ground cops chasing the player (not wrecked), render space.
    pub fn pursuer_positions(&self) -> Vec<Vec3> {
        self.cars.iter().filter(|car| car.is_cop()).map(|car| car.position()).collect()
    }

    /// Holds the busted meter where it is while a cinematic moment plays: it neither fills nor drains.
    pub fn hold_busted(&mut self, hold: bool) {
        self.police.hold_busted(hold);
    }
}
