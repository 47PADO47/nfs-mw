//! The legacy (2005) AttribSys layout, 32-bit keys and pointers. Verified on every vault of
//! NFS: Most Wanted's `attributes.bin`, `gameplay.bin` and `FE_ATTRIB.bin` (PC v1.3); record
//! names and field names follow the dbalatoni13/nfsmw decomp, the reading order follows
//! VaultLib's `LegacyBase` (`docs/formats/attributes.md`).

use super::{
    ArrayLayout, BlobLayout, ClassLoadLayout, CollectionLoadLayout, DatabaseLoadLayout, DefinitionLayout, EntryLayout,
    ExportLayout, Layout, RefSpecLayout, StringKeyLayout,
};

pub const LEGACY: Layout = Layout {
    name: "legacy",
    used_by: "NFS: Most Wanted (2005)",
    export: ExportLayout { len: 20, id: 0x00, kind: 0x04, reserved: Some(0x08), size: 0x0C, offset: 0x10 },
    class_load: ClassLoadLayout {
        len: 0x1C,
        key: 0x00,
        collection_reserve: 0x04,
        definition_count: 0x08,
        definitions: 0x0C,
        layout_size: 0x10,
        // 0x14: mLayoutKeyShift, always 0.
        layout_count: 0x18,
    },
    definition: DefinitionLayout {
        len: 0x10,
        key: 0x00,
        type_key: 0x04,
        offset: 0x08,
        size: 0x0A,
        max_count: 0x0C,
        flags: 0x0E,
        alignment_log2: 0x0F,
    },
    collection_load: CollectionLoadLayout {
        header_len: 0x20,
        key: 0x00,
        class: 0x04,
        parent: 0x08,
        table_reserve: 0x0C,
        // 0x10: mTableKeyShift, always 0.
        entry_count: 0x14,
        type_count: 0x18,
        layout: 0x1C,
    },
    // 0x0B: entry flags, always 0.
    entry: EntryLayout { len: 0x0C, key: 0x00, data: 0x04, type_index: 0x08, node_flags: 0x0A },
    // 0x00: mNumClasses, 0x04: mDefaultDataSize (neither is needed to read).
    database_load: DatabaseLoadLayout { header_len: 0x10, type_count: 0x08, type_names: 0x0C },
    array: ArrayLayout { header_len: 8, capacity: 0, count: 2, item_size: 4, flags: 6, aligned16: 0x8000 },
    string_key: StringKeyLayout { len: 16, hash64: Some(0x00), hash32: 0x08, string: 0x0C },
    // 0x08: the game's cached collection pointer, stored as 0.
    ref_spec: RefSpecLayout { len: 12, class: 0x00, collection: 0x04 },
    blob: BlobLayout { len: 8, size: 0x00, data: 0x04 },
    inline_max: 4,
    pointer_len: 4,
};
