//! The co-op guests' contacts with the traffic (`docs/specs/coop-career.md`). The lead's contacts in `collide.rs` also
//! feed the police, the hit counts and the near misses; a guest's only push the cars apart and mark the damage, since
//! the police chase the lead until the shared pursuit (ADR 0010) reads every player.

use super::TrafficWorld;
use super::collide::near;
use crate::scenes::world::drive::CarSim;

impl TrafficWorld {
    /// Lets each guest's car hit the traffic cars and their trailers. Does nothing without guests.
    pub fn collide_guests(&mut self, guests: &mut [&mut CarSim]) {
        if guests.is_empty() {
            return;
        }
        for car in &mut self.cars {
            let (tractor, mut trailer) = car.sims_mut();
            for guest in guests.iter_mut() {
                if near(guest, tractor) {
                    guest.collide_with_each(tractor, |_, _, _| {});
                }
                let Some(trailer) = trailer.as_deref_mut() else { continue };
                if near(guest, trailer) {
                    guest.collide_with_each(trailer, |_, _, _| {});
                }
            }
        }
    }
}
