//! `PresetRides`: named cars (story cars, showcase cars) with a part per slot.

use blackbox_chunk::ids;

use crate::bytes::{cstr, u32_at, u64_at};
use crate::layout::CarDataLayout;

#[derive(Debug, Clone, PartialEq)]
pub struct PresetRide {
    pub car_type_name: String,
    pub preset_name: String,
    pub fe_key: u64,
    /// Low 32 bits: the car's gameplay record (AttribSys collection key).
    pub vehicle_key: u64,
    /// Per slot: a part name hash, [`PresetRide::EMPTY`] or [`PresetRide::STOCK`].
    pub parts: Vec<u32>,
}

impl PresetRide {
    /// The slot is left empty.
    pub const EMPTY: u32 = 0;
    /// The slot keeps the car's stock part.
    pub const STOCK: u32 = 1;
}

/// Every preset in every `PresetRides` chunk of `data`.
pub fn read_preset_rides(data: &[u8], layout: &CarDataLayout) -> Vec<PresetRide> {
    let l = &layout.preset;
    let slots = layout.slots.len();
    let mut out = Vec::new();
    for chunk in blackbox_chunk::find_all(data, ids::PRESET_RIDES) {
        for r in chunk.payload.chunks_exact(l.len) {
            out.push(PresetRide {
                car_type_name: cstr(r, l.car_type_name, l.name_len),
                preset_name: cstr(r, l.preset_name, l.name_len),
                fe_key: u64_at(r, l.fe_key),
                vehicle_key: u64_at(r, l.vehicle_key),
                parts: (0..slots).map(|i| u32_at(r, l.parts + i * 4)).collect(),
            });
        }
    }
    out
}
