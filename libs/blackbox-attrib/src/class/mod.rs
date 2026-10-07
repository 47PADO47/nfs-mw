//! Classes: the schemas collections are instances of.

mod load;

pub(crate) use load::read_class;

use crate::hash::vlt_hash;

/// `Attrib::Definition` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldFlags(pub u8);

impl FieldFlags {
    /// The field holds an `Attrib::Array` of up to `max_count` values.
    pub const ARRAY: u8 = 0x01;
    /// The field lives in the collection's fixed layout block at `offset`; otherwise it is an
    /// optional attribute entry that may be inherited from the parent collection.
    pub const IN_LAYOUT: u8 = 0x02;
    pub const BOUND: u8 = 0x04;
    pub const NOT_SEARCHABLE: u8 = 0x08;

    pub fn is_array(self) -> bool {
        self.0 & Self::ARRAY != 0
    }

    pub fn in_layout(self) -> bool {
        self.0 & Self::IN_LAYOUT != 0
    }

    /// Not [`Self::NOT_SEARCHABLE`]: the field is in the class's lookup table.
    pub fn searchable(self) -> bool {
        self.0 & Self::NOT_SEARCHABLE == 0
    }
}

/// One field of a class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Hash of the field name (`vlt_hash("MASS")`).
    pub key: u32,
    /// Hash of the type name (`vlt_hash("EA::Reflection::Float")`); see [`crate::TypeKind`].
    pub type_key: u32,
    /// Offset in the layout block (in-layout fields only, else 0).
    pub offset: u16,
    /// Size of one value in bytes.
    pub size: u16,
    /// Capacity of an array field (1 otherwise).
    pub max_count: u16,
    pub flags: FieldFlags,
    /// Alignment of a value in bytes.
    pub alignment: u32,
}

impl Field {
    pub fn is_array(&self) -> bool {
        self.flags.is_array()
    }

    pub fn in_layout(&self) -> bool {
        self.flags.in_layout()
    }
}

/// A class definition (`Attrib::ClassLoadData`).
#[derive(Debug, Clone)]
pub struct Class {
    /// Hash of the class name (`vlt_hash("pvehicle")`).
    pub key: u32,
    /// Fields in file order.
    pub fields: Vec<Field>,
    /// Size of the layout block every collection of the class carries.
    pub layout_size: u32,
    /// `mLayoutCount`: number of searchable in-layout fields (those without
    /// [`FieldFlags::NOT_SEARCHABLE`]); not the number of in-layout fields.
    pub layout_count: u32,
    /// `mCollectionReserve`: how many collections the game reserves room for.
    pub collection_reserve: u32,
    /// Index of the defining vault in [`crate::Database::vaults`].
    pub vault: usize,
}

impl Class {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.field_by_key(vlt_hash(name))
    }

    pub fn field_by_key(&self, key: u32) -> Option<&Field> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// The in-layout fields, by offset.
    pub fn layout_fields(&self) -> impl Iterator<Item = &Field> {
        let mut fields: Vec<_> = self.fields.iter().filter(|f| f.in_layout()).collect();
        fields.sort_by_key(|f| f.offset);
        fields.into_iter()
    }
}
