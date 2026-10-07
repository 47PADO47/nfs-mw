//! Builds small legacy-layout vaults and `VPAK` packs in memory.

use crate::hash::vlt_hash;

pub const CLASS_LOAD: u32 = vlt_hash("Attrib::ClassLoadData");
pub const COLLECTION_LOAD: u32 = vlt_hash("Attrib::CollectionLoadData");

/// The chunk id as stored: the FourCC reversed.
fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = id.iter().rev().copied().collect::<Vec<_>>();
    let padded = (payload.len() + 8).next_multiple_of(16);
    out.extend_from_slice(&(padded as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out.resize(padded, 0);
    out
}

fn le(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub struct VaultBuilder {
    pub name: String,
    vlt: Vec<u8>,
    dat_start: usize,
    pub bin: Vec<u8>,
    exports: Vec<[u32; 4]>,
    vlt_fixups: Vec<(u32, u32)>,
    bin_fixups: Vec<(u32, u32)>,
    pub export_entry_len: usize,
}

impl VaultBuilder {
    /// `strings` go into the `StrE` chunk at the start of the `.bin`.
    pub fn new(name: &str, strings: &[&str]) -> Self {
        let deps = [format!("{name}.vlt"), format!("{name}.bin")];
        let mut dep = le(&[2, vlt_hash(&deps[0]), vlt_hash(&deps[1]), 0, deps[0].len() as u32 + 1]);
        for d in &deps {
            dep.extend_from_slice(d.as_bytes());
            dep.push(0);
        }
        let mut vlt = chunk(b"DepN", &dep);
        vlt.extend(chunk(b"StrN", &[]));
        let dat_start = vlt.len();
        vlt.extend_from_slice(&[0; 8]); // DatN header, patched by `finish`
        let mut strs = Vec::new();
        for s in strings {
            strs.extend_from_slice(s.as_bytes());
            strs.push(0);
        }
        let bin = chunk(b"StrE", &strs);
        Self {
            name: name.into(),
            vlt,
            dat_start,
            bin,
            exports: Vec::new(),
            vlt_fixups: Vec::new(),
            bin_fixups: Vec::new(),
            export_entry_len: 20,
        }
    }

    /// Offset of a string of the `StrE` chunk.
    pub fn string(&self, s: &str) -> u32 {
        let needle = [s.as_bytes(), &[0]].concat();
        let pos = self.bin[8..].windows(needle.len()).position(|w| w == needle).expect("string in StrE");
        (8 + pos) as u32
    }

    /// Appends payload data to the `.bin`, aligned to 16; returns its offset.
    pub fn bin_data(&mut self, bytes: &[u8]) -> u32 {
        self.bin.resize(self.bin.len().next_multiple_of(16), 0);
        let at = self.bin.len() as u32;
        self.bin.extend_from_slice(bytes);
        at
    }

    /// A pointer stored in the `.bin` at `at` that points to `.bin` offset `to`.
    pub fn bin_ptr(&mut self, at: u32, to: u32) {
        self.bin_fixups.push((at, to));
    }

    /// A pointer stored in the `.vlt` at `at` that points to `.bin` offset `to`.
    pub fn vlt_ptr(&mut self, at: u32, to: u32) {
        self.vlt_fixups.push((at, to));
    }

    /// Appends an export record to `DatN`; returns its `.vlt` offset.
    pub fn export(&mut self, id: u32, kind: u32, record: &[u8]) -> u32 {
        self.vlt.resize(self.vlt.len().next_multiple_of(8), 0);
        let at = self.vlt.len() as u32;
        self.vlt.extend_from_slice(record);
        self.exports.push([id, kind, record.len() as u32, at]);
        at
    }

    /// Returns `(name, vlt, bin)`.
    pub fn finish(mut self) -> (String, Vec<u8>, Vec<u8>) {
        self.vlt.resize(self.vlt.len().next_multiple_of(16), 0);
        let dat_size = (self.vlt.len() - self.dat_start) as u32;
        self.vlt[self.dat_start..self.dat_start + 4].copy_from_slice(b"NtaD");
        self.vlt[self.dat_start + 4..self.dat_start + 8].copy_from_slice(&dat_size.to_le_bytes());

        let mut exp = le(&[self.exports.len() as u32]);
        for [id, kind, size, offset] in &self.exports {
            exp.extend(le(&[*id, *kind]));
            if self.export_entry_len == 20 {
                exp.extend(le(&[0]));
            }
            exp.extend(le(&[*size, *offset]));
        }
        self.vlt.extend(chunk(b"ExpN", &exp));

        let record = |fixup: u32, kind: u16, index: u16, dest: u32| {
            [&fixup.to_le_bytes()[..], &kind.to_le_bytes(), &index.to_le_bytes(), &dest.to_le_bytes()].concat()
        };
        let mut ptr = record(0, 2, 1, 0);
        ptr.extend(self.bin_fixups.iter().flat_map(|&(at, to)| record(at, 3, 1, to)));
        ptr.extend(record(0, 2, 0, 0));
        ptr.extend(self.vlt_fixups.iter().flat_map(|&(at, to)| record(at, 3, 1, to)));
        ptr.extend(record(0, 0, 0, 0));
        self.vlt.extend(chunk(b"PtrN", &ptr));
        (self.name, self.vlt, self.bin)
    }
}

/// A `VPAK` pack of finished vaults.
pub fn pack(vaults: &[(String, Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let names: Vec<u8> = vaults.iter().flat_map(|(n, ..)| [n.as_bytes(), &[0]].concat()).collect();
    let strings_at = 16 + 20 * vaults.len();
    let mut out = b"VPAK".to_vec();
    out.extend(le(&[vaults.len() as u32, strings_at as u32, names.len() as u32]));
    let mut data_at = (strings_at + names.len()).next_multiple_of(0x80);
    let mut name_at = 0;
    let mut blobs = Vec::new();
    for (name, vlt, bin) in vaults {
        let bin_at = data_at;
        let vlt_at = (bin_at + bin.len()).next_multiple_of(0x80);
        data_at = (vlt_at + vlt.len()).next_multiple_of(0x80);
        out.extend(le(&[name_at, bin.len() as u32, vlt.len() as u32, bin_at as u32, vlt_at as u32]));
        name_at += name.len() as u32 + 1;
        blobs.push((bin_at, bin));
        blobs.push((vlt_at, vlt));
    }
    out.extend(names);
    for (at, blob) in blobs {
        out.resize(at, 0);
        out.extend_from_slice(blob);
    }
    out
}

/// One `Attrib::Definition`.
pub struct Def {
    pub name: &'static str,
    pub type_name: &'static str,
    pub offset: u16,
    pub size: u16,
    pub max_count: u16,
    /// `FieldFlags` bits.
    pub flags: u8,
    pub align_log2: u8,
}

/// A class with its definitions in the `.bin`.
pub fn add_class(b: &mut VaultBuilder, name: &str, defs: &[Def], layout_size: u32) {
    let mut table = Vec::new();
    for d in defs {
        table.extend(le(&[vlt_hash(d.name), vlt_hash(d.type_name)]));
        for v in [d.offset, d.size, d.max_count] {
            table.extend_from_slice(&v.to_le_bytes());
        }
        table.extend_from_slice(&[d.flags, d.align_log2]);
    }
    let defs_at = b.bin_data(&table);
    let in_layout = defs.iter().filter(|d| d.flags & 2 != 0).count() as u32;
    let record = le(&[vlt_hash(name), 4, defs.len() as u32, 0, layout_size, 0, in_layout]);
    let at = b.export(vlt_hash(name), CLASS_LOAD, &record);
    b.vlt_ptr(at + 0x0C, defs_at);
}

/// What an attribute entry's data word holds.
pub enum Data {
    Inline(u32),
    /// An inline pointer-sized value (a `Text`) pointing to `.bin` offset.
    InlinePtr(u32),
    /// Pointer to the value in the `.bin`.
    Ptr(u32),
}

pub struct Entry {
    pub field: &'static str,
    pub type_index: u16,
    pub node_flags: u8,
    pub data: Data,
}

/// A collection record; `layout` is the `.bin` offset of its layout block.
pub fn add_collection(
    b: &mut VaultBuilder,
    class: &str,
    name: &str,
    parent: Option<&str>,
    layout: Option<u32>,
    types: &[&str],
    entries: &[Entry],
) {
    let parent = parent.map_or(0, vlt_hash);
    let n = entries.len() as u32;
    let mut record = le(&[vlt_hash(name), vlt_hash(class), parent, n, 0, n, types.len() as u32, 0]);
    record.extend(types.iter().flat_map(|t| vlt_hash(t).to_le_bytes()));
    let entries_at = record.len() as u32;
    for e in entries {
        let word = match e.data {
            Data::Inline(v) => v,
            Data::InlinePtr(_) | Data::Ptr(_) => 0,
        };
        record.extend(le(&[vlt_hash(e.field), word]));
        record.extend_from_slice(&e.type_index.to_le_bytes());
        record.extend_from_slice(&[e.node_flags, 0]);
    }
    let id = vlt_hash(&format!("{class}/{name}"));
    let at = b.export(id, COLLECTION_LOAD, &record);
    if let Some(layout) = layout {
        b.vlt_ptr(at + 0x1C, layout);
    }
    for (i, e) in entries.iter().enumerate() {
        if let Data::InlinePtr(to) | Data::Ptr(to) = e.data {
            b.vlt_ptr(at + entries_at + i as u32 * 12 + 4, to);
        }
    }
}
