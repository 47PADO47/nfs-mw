//! `SlotTypes`: which type names each slot searches, with per-car overrides.

use blackbox_chunk::ids;

use crate::bytes::u32_at;
use crate::layout::CarDataLayout;

/// In a type pair, this stands for the car's own type name.
const OWN_TYPE: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotOverride {
    pub car_type_hash: u32,
    pub slot: u32,
    pub types: [u32; 2],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotTypes {
    /// Per slot: up to two type-name hashes (`0xFFFFFFFF` = the car's type, 0 = none).
    pub defaults: Vec<[u32; 2]>,
    pub overrides: Vec<SlotOverride>,
}

impl SlotTypes {
    /// The type-name hashes to search for `slot` of car type `car_type_hash`, in order.
    pub fn search_types(&self, slot: usize, car_type_hash: u32) -> impl Iterator<Item = u32> + use<> {
        let pair = self
            .overrides
            .iter()
            .find(|o| o.car_type_hash == car_type_hash && o.slot as usize == slot)
            .map(|o| o.types)
            .or_else(|| self.defaults.get(slot).copied())
            .unwrap_or([OWN_TYPE, 0]);
        pair.into_iter().filter(|&t| t != 0).map(move |t| if t == OWN_TYPE { car_type_hash } else { t })
    }
}

/// The first `SlotTypes` chunk of `data`: one default pair per slot of the layout, then overrides.
pub fn read_slot_types(data: &[u8], layout: &CarDataLayout) -> SlotTypes {
    let l = &layout.slot_types;
    let Some(chunk) = blackbox_chunk::find(data, ids::CAR_PART_SLOT_TYPES) else { return SlotTypes::default() };
    let p = chunk.payload;
    let split = (layout.slots.len() * l.default_len).min(p.len());
    let pair = |r: &[u8], o| [u32_at(r, o), u32_at(r, o + 4)];
    SlotTypes {
        defaults: p[..split].chunks_exact(l.default_len).map(|r| pair(r, 0)).collect(),
        overrides: p[split..]
            .chunks_exact(l.override_len)
            .map(|r| SlotOverride { car_type_hash: u32_at(r, 0), slot: u32_at(r, 4), types: pair(r, 8) })
            .collect(),
    }
}
