//! A part's solid name hash per level of detail (docs/specs/car-assembly.md §2).

use blackbox_hash::bstring_hash_continue;

use super::{Part, PartsDb};

/// Suffixes appended for each level of detail (`_A` is the most detailed).
const LOD_SUFFIXES: [&[u8]; 5] = [b"_A", b"_B", b"_C", b"_D", b"_E"];

impl PartsDb {
    /// The `bStringHash` of the solid that draws `part` at level of detail `lod` (0 = A), or
    /// `None` if the part has no model there. The solid may still be missing from the geometry
    /// files: many stock parts name solids that don't exist, which means "part of the body".
    pub fn model_hash(&self, part: &Part, lod: usize) -> Option<u32> {
        let table = self.model_tables.get(usize::from(part.model_table))?;
        let entry = *table.entries.get(lod)?;
        if entry == u32::MAX {
            return None;
        }
        if !table.templated {
            return Some(entry);
        }
        let base = match part.base_selector {
            0 => u32::MAX,
            1 => self.type_name_hash(part),
            _ => self.attribute(part, "BRAND_NAME").unwrap_or(0),
        };
        let mut hash = base;
        if let Some(middle) = table.middle {
            hash = bstring_hash_continue(hash, self.string(u32::from(middle))?.as_bytes());
        }
        hash = bstring_hash_continue(hash, self.string(entry)?.as_bytes());
        Some(bstring_hash_continue(hash, LOD_SUFFIXES.get(lod)?))
    }
}
