//! Per-owner visual callbacks are queued independently of audio gates.
use super::*;
use crate::scenes::world::impact_effects::{Contact, RawContact};

pub(super) fn push(out: &mut Vec<Contact>, source: u32, owner: u64, lifetime: &std::rc::Rc<()>, raw: RawContact) {
    if source == 0 || out.len() >= 4096 {
        return;
    }
    out.push(Contact {
        source: blackbox_attrib::RefSpec { class: blackbox_attrib::vlt_hash("smackable"), collection: source },
        owner,
        lifetime: std::rc::Rc::downgrade(lifetime),
        raw,
    });
}
impl PropWorld {
    pub(in crate::scenes::world) fn take_impact_effects(&mut self) -> Vec<Contact> {
        std::mem::take(&mut self.effect_contacts)
    }
}
