//! NFS: Most Wanted (PC). Record sizes and solid keys verified on all 947 scenery
//! sections of the v1.3 install; the rotation encoding (i16 / 8192, row-major,
//! row-vector convention) verified by matching transformed solid bounds against
//! the stored instance boxes (`docs/formats/maps.md`).

use crate::sway::SwayRules;

use super::{HookLayout, InfoLayout, InstanceLayout, LodRules, SceneryLayout, SpawnerLayout, VisibilityRules};

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
    hooks: HookLayout { len: 4, align: 1, override_index: 0, instance_index: 2 },
    spawners: SpawnerLayout {
        align: 16,
        header_len: 16,
        count: 12,
        len: 64,
        orientation: 0,
        position: 16,
        model: 32,
        collision: 36,
        attributes: 40,
        override_index: 44,
        unique_id: 48,
        exclude_flags: 52,
    },
    // docs/specs/scenery-visibility.md: 0x10 = every view, 0x02 = player views.
    visibility: VisibilityRules { inverted_bits: 0x60, player_view: 0x12 },
    // docs/specs/scenery-lod.md.
    lod: LodRules {
        radius_pad: 6.0,
        min_size: 2,
        boost_flag: 0x0200_0000,
        boost: 10,
        draw_threshold: 17,
        poly_threshold: 39,
        density_floor: 6.0,
        density_threshold: 8.7,
        coarse_distance: 25.0,
        detailed_slot: 0,
        coarse_slot: 2,
    },
    // docs/specs/scenery-sway.md.
    sway: SwayRules { solid_flag: 0x80, unmatrixed_flag: 0x200, rate: 45.0, amplitude: 1.5, seed_scale: 60.0 },
};
