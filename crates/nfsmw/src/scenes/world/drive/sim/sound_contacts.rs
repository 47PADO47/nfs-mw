//! Vehicle-owned callbacks survive AI callers discarding their local impact return.
use super::{CarSim, Impact};
use crate::audio::{CarContact, ContactVehicle};

impl CarSim {
    pub(super) fn queue_non_player_sounds(&mut self, impact: &Impact) {
        if !self.collision_role.player {
            self.pending_sound_contacts.extend_from_slice(&impact.sounds);
        }
    }

    /// Called after vehicle pairs resolve; source identity is independent of the other collider.
    pub(in crate::scenes::world) fn take_owned_sound_contacts(&mut self) -> Vec<CarContact> {
        let mut contacts = self.take_sound_contacts();
        if self.effect_source == 0 {
            contacts.clear();
            return contacts;
        }
        for contact in &mut contacts {
            contact.vehicle = Some(ContactVehicle { profile: self.effect_source, identity: self.sound_identity });
        }
        contacts
    }
}

#[cfg(test)]
mod installed;
#[cfg(test)]
mod tests;
