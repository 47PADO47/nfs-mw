//! `SolidInfo` version 0x16: NFS: Most Wanted (PC). Verified on 15,781 car
//! solids and 20,377 world solids of the PC v1.3 install (`docs/formats/models.md`).

use super::{GroupLayout, InfoLayout, MarkerLayout, SolidLayout};

pub const V16: SolidLayout = SolidLayout {
    version: 0x16,
    used_by: "NFS: Most Wanted (PC)",
    info: InfoLayout {
        align: 0x10,
        version: 0x0C,
        flags: 0x0E,
        name_hash: 0x10,
        bounds_min: 0x20,
        bounds_max: 0x30,
        transform: 0x40,
        num_polys: 0x14,
        density: 0x9C,
        name: 0xA0,
    },
    group: GroupLayout {
        len: 104,
        bounds_min: 0x00,
        bounds_max: 0x0C,
        texture_slots: 0x18,
        effect_id: 0x30,
        flags: 0x38,
        num_vertices: 0x3C,
        num_triangles: 0x40,
        first_index: 0x44,
        num_indices: 0x5C,
    },
    marker: MarkerLayout { len: 0x50, align: 0x10, name_hash: 0x00, int_param: 0x04, float_params: 0x08, matrix: 0x10 },
    groups_align: 0x10,
    indices_align: 0x10,
    vertices_align: 0x80,
};
