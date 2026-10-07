//! NFS: Most Wanted (PC). Record sizes and solid keys verified on all 947 scenery
//! sections of the v1.3 install; the rotation encoding (i16 / 8192, row-major,
//! row-vector convention) verified by matching transformed solid bounds against
//! the stored instance boxes (`docs/formats/maps.md`).

use super::{InfoLayout, InstanceLayout, SceneryLayout, VisibilityRules};

pub const MOST_WANTED: SceneryLayout = SceneryLayout {
    used_by: "NFS: Most Wanted (PC)",
    info: InfoLayout {
        len: 72,
        align: 1,
        name: 0x00,
        name_len: 24,
        solid_keys: 0x18,
        lods: 4,
        radius: 0x38,
        hierarchy_hash: 0x40,
    },
    instance: InstanceLayout {
        len: 64,
        align: 0x10,
        bbox_min: 0x00,
        bbox_max: 0x0C,
        exclude_flags: 0x18,
        preculler_index: 0x1C,
        lighting_context: 0x1E,
        position: 0x20,
        rotation: 0x2C,
        rotation_scale: 8192.0,
        info_index: 0x3E,
    },
    header_section_number: 0x0C,
    // docs/specs/scenery-visibility.md: 0x10 = every view, 0x02 = player views.
    visibility: VisibilityRules { inverted_bits: 0x60, player_view: 0x12 },
};
