//! Record layouts per AttribSys generation.
//!
//! AttribSys changed in 2006: NFS: Most Wanted (2005) uses the **legacy** layout; Carbon and
//! later use a "modern" one (16-byte export entries, static fields, 8-byte `StringKey`s). Each
//! generation gets one file here with a [`Layout`] constant, and [`detect`] picks it from a
//! vault's export table. Only the legacy layout is implemented; modern vaults fail with
//! [`crate::Error::UnsupportedLayout`] until a `modern.rs` is added.
//!
//! Offsets are byte offsets inside the record. Spec: `docs/formats/attributes.md`.

mod legacy;

pub use legacy::LEGACY;

/// One `ExpN` entry.
#[derive(Debug, Clone)]
pub struct ExportLayout {
    pub len: usize,
    pub id: usize,
    pub kind: usize,
    /// A `u32` that is always 0 (used to tell layouts apart).
    pub reserved: Option<usize>,
    pub size: usize,
    pub offset: usize,
}

/// `Attrib::ClassLoadData`, the exported record of a class.
#[derive(Debug, Clone)]
pub struct ClassLoadLayout {
    pub len: usize,
    pub key: usize,
    pub collection_reserve: usize,
    pub definition_count: usize,
    /// Pointer to `definition_count` [`DefinitionLayout`] records.
    pub definitions: usize,
    /// Size of the per-collection layout block (the in-layout fields).
    pub layout_size: usize,
    /// Number of searchable in-layout fields (without the not-searchable flag).
    pub layout_count: usize,
}

/// `Attrib::Definition`, one field of a class.
#[derive(Debug, Clone)]
pub struct DefinitionLayout {
    pub len: usize,
    pub key: usize,
    pub type_key: usize,
    /// `u16`: offset in the layout block (in-layout fields only).
    pub offset: usize,
    /// `u16`: size of one value.
    pub size: usize,
    /// `u16`: capacity of an array field.
    pub max_count: usize,
    /// `u8`: [`crate::FieldFlags`].
    pub flags: usize,
    /// `u8`: log2 of the alignment.
    pub alignment_log2: usize,
}

/// `Attrib::CollectionLoadData`: a header, `u32 types[type_count]`, then `entry_count` entries.
#[derive(Debug, Clone)]
pub struct CollectionLoadLayout {
    pub header_len: usize,
    pub key: usize,
    pub class: usize,
    pub parent: usize,
    pub table_reserve: usize,
    pub entry_count: usize,
    pub type_count: usize,
    /// Pointer to the layout block (null when the class has no in-layout fields).
    pub layout: usize,
}

/// One attribute entry of a collection (a field that is not in the layout block).
#[derive(Debug, Clone)]
pub struct EntryLayout {
    pub len: usize,
    pub key: usize,
    /// The value itself if it fits ([`Layout::inline_max`] bytes, not an array), else a pointer.
    pub data: usize,
    /// `u16`: index into the collection's type list.
    pub type_index: usize,
    /// `u8`: node flags (0x02 array, 0x20 stored by value).
    pub node_flags: usize,
}

/// `Attrib::DatabaseLoadData`: a header, then `u32 size[type_count]`.
#[derive(Debug, Clone)]
pub struct DatabaseLoadLayout {
    pub header_len: usize,
    pub type_count: usize,
    /// Pointer to `type_count` NUL-terminated type names.
    pub type_names: usize,
}

/// `Attrib::Array`: `{u16 capacity, u16 count, u16 item size, u16 flags}`, then the items.
#[derive(Debug, Clone)]
pub struct ArrayLayout {
    pub header_len: usize,
    pub capacity: usize,
    pub count: usize,
    pub item_size: usize,
    pub flags: usize,
    /// Flag bit: the items start `header_len` bytes later (16-byte alignment).
    pub aligned16: u16,
}

/// `Attrib::StringKey`: hashes of a string plus a pointer to it.
#[derive(Debug, Clone)]
pub struct StringKeyLayout {
    pub len: usize,
    /// `u64` hash (not the lookup2 hash; kept as stored).
    pub hash64: Option<usize>,
    /// `u32`: `vlt_hash` of the string.
    pub hash32: usize,
    pub string: usize,
}

/// `Attrib::RefSpec`: a reference to a collection of a class.
#[derive(Debug, Clone)]
pub struct RefSpecLayout {
    pub len: usize,
    pub class: usize,
    pub collection: usize,
}

/// `Attrib::Blob`: `{u32 size, pointer}` to a (usually compressed) byte block.
#[derive(Debug, Clone)]
pub struct BlobLayout {
    pub len: usize,
    pub size: usize,
    pub data: usize,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub name: &'static str,
    /// Which games are known to use it, for messages and docs.
    pub used_by: &'static str,
    pub export: ExportLayout,
    pub class_load: ClassLoadLayout,
    pub definition: DefinitionLayout,
    pub collection_load: CollectionLoadLayout,
    pub entry: EntryLayout,
    pub database_load: DatabaseLoadLayout,
    pub array: ArrayLayout,
    pub string_key: StringKeyLayout,
    pub ref_spec: RefSpecLayout,
    pub blob: BlobLayout,
    /// Largest non-array value stored inside an entry instead of behind a pointer.
    pub inline_max: usize,
    /// Size of a pointer in the data.
    pub pointer_len: usize,
}

/// All supported layouts.
pub const LAYOUTS: &[&Layout] = &[&LEGACY];

/// The layout whose export entries fit `export_table` (an `ExpN` payload): the entries fill it
/// up to the 16-byte padding, and their reserved fields are 0.
pub fn detect(export_table: &[u8]) -> Option<&'static Layout> {
    let count = crate::bytes::u32_at(export_table, 0)? as usize;
    LAYOUTS.iter().copied().find(|layout| {
        let e = &layout.export;
        let Some(needed) = count.checked_mul(e.len).and_then(|n| n.checked_add(4)) else { return false };
        let fits = export_table.len() >= needed && export_table.len() - needed < 16;
        fits && e
            .reserved
            .is_none_or(|r| (0..count).all(|i| crate::bytes::u32_at(export_table, 4 + i * e.len + r) == Some(0)))
    })
}
