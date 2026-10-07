//! Car-table record layouts per game. Add a game by adding a module with a
//! [`CarDataLayout`] once its layouts are documented.

mod most_wanted;

pub use most_wanted::MOST_WANTED;

/// `CarTypeInfo`: one car type (model, manufacturer, paint and skin defaults).
#[derive(Debug, Clone)]
pub struct CarTypeLayout {
    pub len: usize,
    /// Payload alignment (the payload starts with `0x11` padding up to it).
    pub align: usize,
    /// `char[short_name_len]` each.
    pub type_name: usize,
    pub base_model_name: usize,
    pub manufacturer: usize,
    pub short_name_len: usize,
    /// `char[filename_len]`.
    pub geometry_filename: usize,
    pub filename_len: usize,
    pub type_name_hash: usize,
    /// `i32`.
    pub index: usize,
    /// `i32`: racer, cop, traffic, ...
    pub usage_type: usize,
    /// `u8`.
    pub default_skin_number: usize,
    /// `u8`, non-zero when the body is painted through a composite skin texture.
    pub skinnable: usize,
    /// `u32`: part name hash of the stock paint.
    pub default_base_paint: usize,
}

/// `CarPartPack`: header fields and record sizes of the parts database.
#[derive(Debug, Clone)]
pub struct PartsLayout {
    /// The header's version field must equal this.
    pub version: u32,
    pub header_version: usize,
    pub header_num_attributes: usize,
    pub header_num_type_names: usize,
    pub header_num_model_tables: usize,
    pub header_num_parts: usize,
    /// String offsets count in units of this many bytes.
    pub string_unit: usize,
    pub part: PartLayout,
    /// `(u32 name hash, u32 value)`.
    pub attribute_len: usize,
    pub model_table: ModelTableLayout,
}

/// One `CarPart` record.
#[derive(Debug, Clone)]
pub struct PartLayout {
    pub len: usize,
    /// Two `u16`s: low, then high half.
    pub name_hash: usize,
    /// `i8` part id.
    pub part_id: usize,
    /// `u8`: group number in bits 0..5, upgrade level in bits 5..8.
    pub group_and_level: usize,
    /// `i8`: 0 none, 1 the part's type name, 2 its `BRAND_NAME` attribute.
    pub base_selector: usize,
    /// `u8` index into the type-name table.
    pub type_index: usize,
    /// `u16` string offset of the authoring name.
    pub name_offset: usize,
    /// `u16` index (in `i16` units) into the attribute-list table, `0xFFFF` for none.
    pub attribute_list: usize,
    /// `u16` index into the model tables, `0xFFFF` for none.
    pub model_table: usize,
}

/// One `CarPartModelTable`.
#[derive(Debug, Clone)]
pub struct ModelTableLayout {
    pub len: usize,
    /// `i8`: non-zero when entries are string offsets to build hashes from.
    pub templated: usize,
    /// `u16` string offset, `0xFFFF` for none.
    pub middle_string: usize,
    /// `u32[lods]`.
    pub entries: usize,
    pub lods: usize,
}

/// `SlotTypes`: the type names searched for each slot, then per-car overrides.
#[derive(Debug, Clone)]
pub struct SlotTypesLayout {
    /// `u32[2]` per slot.
    pub default_len: usize,
    /// `u32` car type hash, `u32` slot, `u32[2]` types.
    pub override_len: usize,
}

/// `PresetRide`: a named car with a part per slot.
#[derive(Debug, Clone)]
pub struct PresetLayout {
    pub len: usize,
    pub car_type_name: usize,
    pub preset_name: usize,
    pub name_len: usize,
    /// `u64`.
    pub fe_key: usize,
    /// `u64`.
    pub vehicle_key: usize,
    /// `u32[slots]`: part name hash per slot.
    pub parts: usize,
}

/// `eLightMaterial`: shading constants for car materials.
#[derive(Debug, Clone)]
pub struct LightMaterialLayout {
    pub name_hash: usize,
    pub version: usize,
    pub name: usize,
    pub name_len: usize,
    /// The 30 `f32` shading values, in [`crate::LightMaterial`] field order.
    pub values: usize,
}

/// A car slot of the game (`CAR_SLOT_ID`).
#[derive(Debug, Clone, Copy)]
pub struct SlotDef {
    pub name: &'static str,
    /// The `CAR_PART_ID` of parts that fit the slot.
    pub part_id: u8,
}

/// Everything game-specific about the car tables.
#[derive(Debug, Clone)]
pub struct CarDataLayout {
    pub game: &'static str,
    pub car_type: CarTypeLayout,
    pub parts: PartsLayout,
    pub slot_types: SlotTypesLayout,
    pub preset: PresetLayout,
    pub light_material: LightMaterialLayout,
    /// Every slot, in `CAR_SLOT_ID` order.
    pub slots: &'static [SlotDef],
}

impl CarDataLayout {
    /// The slot index for a slot name.
    pub fn slot(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|s| s.name == name)
    }
}
