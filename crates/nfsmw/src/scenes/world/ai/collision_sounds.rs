//! Both traffic bodies publish completed callbacks, including independently rolling trailers.
use super::TrafficWorld;
use crate::audio::CarContact;

impl TrafficWorld {
    pub fn take_collision_sounds(&mut self) -> Vec<CarContact> {
        let mut contacts = Vec::new();
        for car in &mut self.cars {
            let (tractor, trailer) = car.sims_mut();
            tractor.drain_owned_sound_contacts_into(&mut contacts);
            if let Some(trailer) = trailer {
                trailer.drain_owned_sound_contacts_into(&mut contacts);
            }
        }
        contacts
    }
}
