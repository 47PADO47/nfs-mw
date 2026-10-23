use super::*;
use crate::scenes::world::impact_effects::{Contact, RawContact};

/// The attribute class of the vehicle an effect contact names, hashed at compile time.
const PVEHICLE_CLASS: u32 = blackbox_attrib::vlt_hash("pvehicle");

impl CarSim {
    pub(in crate::scenes::world) fn take_impact_effects(&mut self) -> Vec<Contact> {
        std::mem::take(&mut self.pending_effects)
    }
    /// As [`Self::take_impact_effects`], appended to `out`: the car keeps its queue's capacity for the next tick.
    pub(in crate::scenes::world) fn drain_impact_effects_into(&mut self, out: &mut Vec<Contact>) {
        out.append(&mut self.pending_effects);
    }
    pub(super) fn queue_effect(&mut self, raw: RawContact) {
        if self.effect_source == 0 || self.pending_effects.len() >= 4096 {
            return;
        }
        self.pending_effects.push(Contact {
            source: blackbox_attrib::RefSpec { class: PVEHICLE_CLASS, collection: self.effect_source },
            owner: self.effect_owner,
            lifetime: std::rc::Rc::downgrade(&self.effect_lifetime),
            raw,
        });
    }
    pub(super) fn queue_ground_effects(&mut self) {
        let body = self.vehicle.body();
        let rot = body.rotation();
        let bottom_point = body.position - rot.y_axis * body.dimension().y - rot.z_axis * body.dimension().z * 0.75;
        let contacts: Vec<_> = self
            .vehicle
            .body_contacts()
            .iter()
            .filter(|c| c.impulse_delta_v > 0.0)
            .map(|c| {
                let bottom = self.vehicle.wheels_on_ground() > 2 && rot.y_axis.dot(c.normal) > 0.2;
                let mut raw = RawContact::ground(c, body.linear_velocity);
                raw.point = if bottom { bottom_point } else { c.point };
                raw.bottom = bottom;
                raw
            })
            .collect();
        for raw in contacts {
            self.queue_effect(raw);
        }
    }
}
