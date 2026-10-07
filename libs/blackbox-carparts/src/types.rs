//! `CarTypeInfos`: one record per car type.

use blackbox_chunk::ids;

use crate::bytes::{cstr, u32_at};
use crate::layout::CarDataLayout;

#[derive(Debug, Clone, PartialEq)]
pub struct CarTypeInfo {
    /// e.g. `BMWM3GTR`.
    pub type_name: String,
    /// Prefix of the car's solid names; lower-cased, it also names the car's gameplay records.
    pub base_model_name: String,
    pub geometry_filename: String,
    pub manufacturer: String,
    /// `bStringHash(type_name)`: the car's type in the parts database.
    pub type_name_hash: u32,
    pub index: i32,
    /// 0 racer, 1 cop, 2 traffic, 3 wheels, 4 universal (NFS: MW).
    pub usage_type: i32,
    pub default_skin_number: u8,
    /// The body is painted through a composite skin texture.
    pub skinnable: bool,
    /// Part name hash of the stock paint.
    pub default_base_paint: u32,
}

/// Every car type in the first `CarTypeInfos` chunk of `data`.
pub fn read_car_types(data: &[u8], layout: &CarDataLayout) -> Vec<CarTypeInfo> {
    let l = &layout.car_type;
    let Some(chunk) = blackbox_chunk::find(data, ids::CAR_TYPE_INFOS) else { return Vec::new() };
    let name = |r: &[u8], o| cstr(r, o, l.short_name_len);
    chunk
        .aligned_payload(l.align)
        .chunks_exact(l.len)
        .map(|r| CarTypeInfo {
            type_name: name(r, l.type_name),
            base_model_name: name(r, l.base_model_name),
            geometry_filename: cstr(r, l.geometry_filename, l.filename_len),
            manufacturer: name(r, l.manufacturer),
            type_name_hash: u32_at(r, l.type_name_hash),
            index: u32_at(r, l.index) as i32,
            usage_type: u32_at(r, l.usage_type) as i32,
            default_skin_number: r[l.default_skin_number],
            skinnable: r[l.skinnable] != 0,
            default_base_paint: u32_at(r, l.default_base_paint),
        })
        .collect()
}
