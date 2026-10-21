//! Builds the `.data` of an animation bank field by field, for the bank and compound tests. Every pointer written
//! with [`Data::put_pointer`] is recorded as a relocation, so `Object::read` sees it as the install's objects do.

use super::elf_builder::Builder;

/// The symbol that names the bank header.
pub const BANK_SYMBOL: &str = "__AnimationBank:::bank";

/// The `.data` section under construction: its bytes and the offsets of its relocated words.
#[derive(Default)]
pub struct Data {
    bytes: Vec<u8>,
    relocs: Vec<u32>,
}

impl Data {
    /// The offset the next field will be written at.
    pub fn here(&self) -> u32 {
        self.bytes.len() as u32
    }

    /// Pads with zeros to a multiple of 4 bytes.
    pub fn align(&mut self) {
        while self.bytes.len() % 4 != 0 {
            self.bytes.push(0);
        }
    }

    /// Appends a little-endian u16 and returns its offset.
    pub fn put_u16(&mut self, value: u16) -> u32 {
        self.put_bytes(&value.to_le_bytes())
    }

    /// Appends a little-endian u32 that is not a pointer and returns its offset.
    pub fn put_u32(&mut self, value: u32) -> u32 {
        self.put_bytes(&value.to_le_bytes())
    }

    /// Appends a relocated word that points at `target`, an offset in `.data`, and returns its offset.
    pub fn put_pointer(&mut self, target: u32) -> u32 {
        let at = self.put_u32(target);
        self.relocs.push(at);
        at
    }

    /// Appends raw bytes and returns their offset.
    pub fn put_bytes(&mut self, bytes: &[u8]) -> u32 {
        let at = self.here();
        self.bytes.extend_from_slice(bytes);
        at
    }

    /// The builder for the object: the `.data` above, its relocations, and (when `header` is given) the bank symbol.
    pub fn builder(&self, bank_header: Option<u32>) -> Builder {
        let mut builder = Builder::new(&self.bytes);
        if let Some(header) = bank_header {
            builder = builder.symbol(BANK_SYMBOL, header);
        }
        for &offset in &self.relocs {
            builder = builder.reloc(offset);
        }
        builder
    }
}

/// A child record of `type_id` with the target checksum `checksum`. Its body is not read by the reader, so it is
/// zeros. Returns the record's offset.
pub fn child(data: &mut Data, type_id: u16, checksum: u16) -> u32 {
    data.align();
    let at = data.put_u16(type_id);
    data.put_u16(checksum);
    data.put_bytes(&[0; 8]);
    at
}

/// An attribute block with one entry per `(id, value)` pair. Each entry has size 1 and a value field of the value
/// byte followed by three `'0'` bytes, as the install writes its FPS attributes. Returns the block's offset.
pub fn attribute_block(data: &mut Data, entries: &[(u16, u8)]) -> u32 {
    data.align();
    let at = data.put_u32(entries.len() as u32);
    for &(id, value) in entries {
        data.put_u16(id);
        data.put_u16(1);
        data.put_bytes(&[value, b'0', b'0', b'0']);
    }
    at
}

/// A compound record with the checksum, frame count, attribute block (`None` writes a null pointer) and child
/// records given. Returns the record's offset.
pub fn compound(data: &mut Data, checksum: u16, frames: u16, attributes: Option<u32>, children: &[u32]) -> u32 {
    data.align();
    let at = data.put_u16(15);
    data.put_u16(checksum);
    match attributes {
        Some(block) => data.put_pointer(block),
        None => data.put_u32(0),
    };
    data.put_u16(children.len() as u16);
    data.put_u16(frames);
    for &child in children {
        data.put_pointer(child);
    }
    at
}

/// A bank header of `count` animations whose record and name arrays are at `records` and `names`. Returns the
/// header's offset.
pub fn bank_header(data: &mut Data, count: u32, records: u32, names: u32) -> u32 {
    data.align();
    let at = data.put_u32(0x1C);
    data.put_u32(count);
    data.put_u32(0);
    data.put_u32(0);
    data.put_pointer(records);
    data.put_pointer(names);
    data.put_u32(0);
    at
}

/// A bank over `entries`, a list of `(name, record offset)` in the order to store them. Writes the names, the two
/// pointer arrays and the header. Returns the header's offset.
pub fn bank(data: &mut Data, entries: &[(&str, u32)]) -> u32 {
    let mut names = Vec::new();
    for (name, _) in entries {
        names.push(data.put_bytes(name.as_bytes()));
        data.put_bytes(&[0]);
    }
    data.align();
    let name_array = data.here();
    for &name in &names {
        data.put_pointer(name);
    }
    let record_array = data.here();
    for &(_, record) in entries {
        data.put_pointer(record);
    }
    bank_header(data, entries.len() as u32, record_array, name_array)
}
