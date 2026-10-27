//! Reading Genesys objects: the self-describing gameplay data of a Chameleon bundle.
//!
//! The same layout `chameleon-vehicle` reads (`docs/formats/chameleon-vehicle-stats.md`, "Genesys types and objects"):
//! an object is one block of memory with every nested structure inline, and its import table names a type import for
//! each structure (slot offset with the top bit set = the structure's offset in the block, id = the `GenesysType`
//! resource). Field names are hashed, so fields are addressed by offset. This copy keeps the crate free of a
//! dependency on the car crate; it only needs the structure offsets by type name.

use std::collections::HashMap;

use chameleon_bundle::{Bundle, Resource, ResourceKind};

use crate::error::{Error, Result};

/// The top bit of an import slot offset: the slot names a structure type, not a pointer.
const TYPE_FLAG: u32 = 0x8000_0000;
/// Where a type resource keeps the offset of its name, relative to the 32-bit value at 0x20 of its main block.
const NAME_SKIP: usize = 8;

/// The names of the Genesys types of one bundle, by resource id.
#[derive(Debug, Default)]
pub(crate) struct TypeNames {
    names: HashMap<u64, String>,
}

impl TypeNames {
    pub fn read(bundle: &Bundle) -> Result<Self> {
        let kind = ResourceKind::GenesysType.id();
        let mut names = HashMap::new();
        for (index, entry) in bundle.entries().iter().enumerate() {
            if entry.kind != kind {
                continue;
            }
            let resource = bundle.resource(index)?;
            if let Some(name) = type_name(&resource.blocks[0]) {
                names.insert(entry.id, name);
            }
        }
        Ok(Self { names })
    }

    pub fn name(&self, id: u64) -> Option<&str> {
        self.names.get(&id).map(String::as_str)
    }
}

fn type_name(main: &[u8]) -> Option<String> {
    let header = u32::from_le_bytes(main.get(0x20..0x24)?.try_into().ok()?) as usize;
    let start = header.checked_add(NAME_SKIP)?;
    let tail = main.get(start..)?;
    let end = tail.iter().position(|&byte| byte == 0)?;
    let name = std::str::from_utf8(&tail[..end]).ok()?;
    (name.len() >= 5 && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')))
        .then(|| name.to_owned())
}

/// A Genesys object with the structures it holds, by type name.
#[derive(Debug)]
pub(crate) struct Object {
    pub main: Vec<u8>,
    /// `(type name, offset in main)` of every inline structure, in memory order.
    pub structs: Vec<(String, usize)>,
}

impl Object {
    /// An object from its block and its structures (the tests build objects this way).
    #[cfg(test)]
    pub fn new(main: Vec<u8>, structs: Vec<(String, usize)>) -> Self {
        Self { main, structs }
    }

    /// The offset of the first structure of type `name`.
    pub fn at(&self, name: &str) -> Result<usize> {
        self.structs
            .iter()
            .find(|(found, _)| found == name)
            .map(|(_, offset)| *offset)
            .ok_or_else(|| Error::Spec(format!("no {name} in the object")))
    }

    pub fn f32(&self, at: usize) -> Result<f32> {
        Ok(f32::from_bits(self.u32(at)?))
    }

    pub fn u32(&self, at: usize) -> Result<u32> {
        let bytes = self.main.get(at..at + 4).ok_or_else(|| Error::Spec(format!("object ends before 0x{at:X}")))?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn floats<const N: usize>(&self, at: usize) -> Result<[f32; N]> {
        let mut out = [0.0; N];
        for (n, slot) in out.iter_mut().enumerate() {
            *slot = self.f32(at + 4 * n)?;
        }
        Ok(out)
    }
}

/// Every object of the bundle that holds a structure of type `marker`, with the key of its resource id.
pub(crate) fn find_objects(bundle: &Bundle, types: &TypeNames, marker: &str) -> Result<Vec<(u64, Object)>> {
    let kind = ResourceKind::GenesysObject.id();
    let mut found = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.import_count == 0 {
            continue;
        }
        let resource = bundle.resource(index)?;
        if let Some(object) = object_of(resource, types, marker) {
            found.push((entry.id, object));
        }
    }
    Ok(found)
}

fn object_of(resource: Resource, types: &TypeNames, marker: &str) -> Option<Object> {
    let mut structs: Vec<(String, usize)> = resource
        .imports
        .iter()
        .filter(|import| import.offset & TYPE_FLAG != 0)
        .filter_map(|import| Some((types.name(import.id)?.to_owned(), (import.offset & !TYPE_FLAG) as usize)))
        .collect();
    if !structs.iter().any(|(name, _)| name == marker) {
        return None;
    }
    structs.sort_by_key(|(_, offset)| *offset);
    let [main, ..] = resource.blocks;
    Some(Object { main, structs })
}
