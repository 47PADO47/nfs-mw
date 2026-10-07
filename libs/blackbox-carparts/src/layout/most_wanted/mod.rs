//! NFS: Most Wanted (PC). Layouts from docs/formats/cardata.md, verified on the
//! v1.3 install's `GLOBAL/GlobalB.lzc` (91 car types, 13,080 parts, 82 presets,
//! 156 light materials).

mod slots;

use super::{
    CarDataLayout, CarTypeLayout, LightMaterialLayout, ModelTableLayout, PartLayout, PartsLayout, PresetLayout,
    SlotTypesLayout,
};

pub const MOST_WANTED: CarDataLayout = CarDataLayout {
    game: "NFS: Most Wanted (PC)",
    car_type: CarTypeLayout {
        len: 0xD0,
        align: 0x10,
        type_name: 0x00,
        base_model_name: 0x10,
        manufacturer: 0x40,
        short_name_len: 16,
        geometry_filename: 0x20,
        filename_len: 32,
        type_name_hash: 0x50,
        index: 0x90,
        usage_type: 0x94,
        default_skin_number: 0xC6,
        skinnable: 0xC7,
        default_base_paint: 0xCC,
    },
    parts: PartsLayout {
        version: 6,
        header_version: 0x08,
        header_num_attributes: 0x20,
        header_num_type_names: 0x28,
        header_num_model_tables: 0x30,
        header_num_parts: 0x38,
        string_unit: 4,
        part: PartLayout {
            len: 14,
            name_hash: 0x0,
            part_id: 0x4,
            group_and_level: 0x5,
            base_selector: 0x6,
            type_index: 0x7,
            name_offset: 0x8,
            attribute_list: 0xA,
            model_table: 0xC,
        },
        attribute_len: 8,
        model_table: ModelTableLayout { len: 24, templated: 0x0, middle_string: 0x2, entries: 0x4, lods: 5 },
    },
    slot_types: SlotTypesLayout { default_len: 8, override_len: 16 },
    preset: PresetLayout {
        len: 0x290,
        car_type_name: 0x08,
        preset_name: 0x28,
        name_len: 32,
        fe_key: 0x48,
        vehicle_key: 0x50,
        parts: 0x60,
    },
    light_material: LightMaterialLayout { name_hash: 0x0C, version: 0x10, name: 0x14, name_len: 28, values: 0x30 },
    slots: &slots::SLOTS,
};
