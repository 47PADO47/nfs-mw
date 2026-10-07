//! The parts database (`CarPartPack`): parts, their attributes and model names.

mod model;
mod reader;

pub use reader::read_parts_db;

use blackbox_hash::bstring_hash;

/// One part as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    /// An opaque id (`bStringHash` of an authoring name such as `BMWM3GTR_BASE`).
    pub name_hash: u32,
    /// `CAR_PART_ID`: which kind of part (body, wheel, paint, ...).
    pub part_id: u8,
    pub group: u8,
    /// 0 = stock.
    pub upgrade_level: u8,
    /// Where a templated model name starts: 0 nothing, 1 the type name, 2 the `BRAND_NAME` attribute.
    pub base_selector: i8,
    pub type_index: u8,
    pub(crate) name_offset: u16,
    pub(crate) attribute_list: u16,
    pub(crate) model_table: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelTable {
    pub templated: bool,
    /// String offset, `None` for no middle string.
    pub middle: Option<u16>,
    /// Per LOD: a solid hash (plain) or a string offset (templated); `u32::MAX` = no model.
    pub entries: Vec<u32>,
}

/// What to look for with [`PartsDb::find`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartQuery {
    pub part_id: u8,
    pub type_hash: u32,
    pub name_hash: Option<u32>,
    pub upgrade_level: Option<u8>,
}

impl PartQuery {
    /// Any part of kind `part_id` in type `type_hash` (a car type or a shared group like `PAINT`).
    pub fn new(part_id: u8, type_hash: u32) -> Self {
        Self { part_id, type_hash, name_hash: None, upgrade_level: None }
    }

    pub fn name_hash(mut self, hash: u32) -> Self {
        self.name_hash = Some(hash);
        self
    }

    pub fn upgrade_level(mut self, level: u8) -> Self {
        self.upgrade_level = Some(level);
        self
    }
}

/// The decoded parts database.
#[derive(Debug, Clone, Default)]
pub struct PartsDb {
    pub(crate) string_unit: usize,
    pub(crate) strings: Vec<u8>,
    /// Raw `i16` stream: per list, a count then attribute indices.
    pub(crate) attribute_lists: Vec<u8>,
    /// `(name hash, value)`.
    pub(crate) attributes: Vec<(u32, u32)>,
    pub(crate) model_tables: Vec<ModelTable>,
    pub(crate) type_names: Vec<u32>,
    pub(crate) parts: Vec<Part>,
}

impl PartsDb {
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    /// Type-name hashes: car types and shared groups (`PAINT`, `WHEELS`, ...).
    pub fn type_names(&self) -> &[u32] {
        &self.type_names
    }

    /// The type-name hash a part belongs to.
    pub fn type_name_hash(&self, part: &Part) -> u32 {
        self.type_names.get(usize::from(part.type_index)).copied().unwrap_or(0)
    }

    /// The part's authoring name (`BODY_00`, `METAL_L1_COLOR02`, ...).
    pub fn name(&self, part: &Part) -> &str {
        self.string(u32::from(part.name_offset)).unwrap_or("")
    }

    /// The string at a string-table offset (in the layout's units).
    pub fn string(&self, offset: u32) -> Option<&str> {
        let start = offset as usize * self.string_unit;
        let rest = self.strings.get(start..)?;
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        std::str::from_utf8(&rest[..end]).ok()
    }

    /// The first part matching `query`, in database order (the engine's search order).
    pub fn find(&self, query: &PartQuery) -> Option<&Part> {
        self.parts.iter().find(|p| {
            p.part_id == query.part_id
                && self.type_name_hash(p) == query.type_hash
                && query.name_hash.is_none_or(|h| h == p.name_hash)
                && query.upgrade_level.is_none_or(|l| l == p.upgrade_level)
        })
    }

    /// The first part with this name hash.
    pub fn by_name_hash(&self, name_hash: u32) -> Option<&Part> {
        self.parts.iter().find(|p| p.name_hash == name_hash)
    }

    /// The part's attributes as `(name hash, raw value)`, first match first.
    pub fn attributes<'a>(&'a self, part: &Part) -> impl Iterator<Item = (u32, u32)> + 'a {
        let list = (part.attribute_list != u16::MAX).then(|| usize::from(part.attribute_list) * 2);
        let i16_at = move |o: usize| self.attribute_lists.get(o..o + 2).map(|b| i16::from_le_bytes([b[0], b[1]]));
        let count = list.and_then(i16_at).unwrap_or(0).max(0) as usize;
        let start = list.unwrap_or(0) + 2;
        (0..count).filter_map(move |i| {
            let index = i16_at(start + i * 2)?;
            self.attributes.get(usize::try_from(index).ok()?).copied()
        })
    }

    /// The raw value of the first attribute named `name` (e.g. `RED`, `KITNUMBER`).
    pub fn attribute(&self, part: &Part, name: &str) -> Option<u32> {
        let hash = bstring_hash(name);
        self.attributes(part).find(|&(h, _)| h == hash).map(|(_, v)| v)
    }

    pub fn attribute_f32(&self, part: &Part, name: &str) -> Option<f32> {
        self.attribute(part, name).map(f32::from_bits)
    }

    /// A string attribute (the value is a string-table offset).
    pub fn attribute_str(&self, part: &Part, name: &str) -> Option<&str> {
        self.string(self.attribute(part, name)?)
    }
}
