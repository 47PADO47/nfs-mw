//! Per-owner native callbacks, consumed once after a completed physics tick.

use super::*;
use blackbox_vehicle::rigid_body::ObbContact;

pub(super) struct Pair {
    pub owners: [(u32, u32); 2],
    pub surfaces: [u32; 2],
    pub zones: [nfsmw_data::sound::CollisionZone; 2],
    pub contact: ObbContact,
    pub velocity: Vec3,
    pub detached: [bool; 2],
}

impl Pair {
    pub fn emit(self, out: &mut Vec<CarContact>, delta_v: [f32; 2]) {
        for (i, (id, profile)) in self.owners.into_iter().enumerate() {
            if profile == 0 {
                continue;
            }
            let sign = [1.0, -1.0][i];
            out.push(CarContact {
                sound_profile: profile,
                audio_zone: Some(self.zones[i]),
                detached: self.detached[i],
                source: ContactSource::Object,
                identity: u64::from(id),
                surface: self.surfaces[i],
                point: self.contact.point,
                normal: self.contact.normal * sign,
                velocity: self.velocity * sign,
                delta_v: delta_v[i],
                front: false,
                bottom: false,
            });
        }
    }
}

impl PropWorld {
    pub(in crate::scenes::world) fn take_sound_contacts(&mut self) -> Vec<CarContact> {
        std::mem::take(&mut self.sound_contacts)
    }
}
