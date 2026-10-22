//! The co-op guests' contacts with the traffic (`docs/specs/coop-career.md`). The lead's contacts in `collide.rs` also
//! feed the police, the hit counts and the near misses; a guest's only push the cars apart and mark the damage, since
//! the police chase the lead until the shared pursuit (ADR 0010) reads every player.

use super::TrafficWorld;
use super::collide::near;
use super::touches::{CarTouch, Toucher};
use crate::scenes::world::drive::CarSim;

impl TrafficWorld {
    /// Lets each guest's car hit the traffic cars and their trailers. Does nothing without guests.
    pub fn collide_guests(&mut self, guests: &mut [&mut CarSim]) {
        if guests.is_empty() {
            return;
        }
        let mut touched = Vec::new();
        for car in &mut self.cars {
            let entity = car.key();
            let (tractor, mut trailer) = car.sims_mut();
            for (index, guest) in guests.iter_mut().enumerate() {
                if let Some(speed) = touch(guest, tractor) {
                    touched.push(CarTouch { a: Toucher::Guest(index), b: Toucher::Car(entity), speed });
                }
                let Some(trailer) = trailer.as_deref_mut() else { continue };
                if let Some(speed) = touch(guest, trailer) {
                    touched.push(CarTouch { a: Toucher::Guest(index), b: Toucher::Car(entity), speed });
                }
            }
        }
        self.touches.extend(touched);
    }
}

/// Lets a guest's car and a traffic body hit each other when their spheres touch; the speed (m/s) they closed at when
/// they did touch.
fn touch(guest: &mut CarSim, car: &mut CarSim) -> Option<f32> {
    if !near(guest, car) {
        return None;
    }
    let speed = closing(guest, car);
    guest.collide_with_each(car, |_, _, _| {}).then_some(speed)
}

/// How fast two cars close (m/s).
fn closing(a: &CarSim, b: &CarSim) -> f32 {
    (a.body().linear_velocity - b.body().linear_velocity).length()
}
