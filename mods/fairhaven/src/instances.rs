//! The instance lists of a unit: the models it places and where (`docs/formats/chameleon-world.md#instancelist-0x50`), and the
//! props, compound and dynamic objects it places (the lists whose records name an object that has a model).

use std::collections::HashMap;

use anyhow::{Result, ensure};
use chameleon_bundle::Resource;

/// Resource type id of an `InstanceList`, of a `PropInstanceList`, of a `CompoundInstanceList` and of a `DynamicInstanceList`.
pub const INSTANCE_LIST: u32 = 0x50;
pub const PROP_INSTANCE_LIST: u32 = 0x218;
pub const COMPOUND_INSTANCE_LIST: u32 = 0x216;
pub const DYNAMIC_INSTANCE_LIST: u32 = 0x204;
/// Resource type id of a model (the objects of those lists have one among their imports).
pub const MODEL: u32 = 0x51;
/// Bytes of one record.
const RECORD_LEN: usize = 0x60;

/// Where a list keeps the offset of its first record and its record count (words of the header), and where a record keeps its
/// matrix and the slot that holds the id of what it places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub first_at: usize,
    pub count_at: usize,
    pub matrix_at: usize,
    pub slot_at: usize,
}

/// The records of an `InstanceList`: the model's slot first, the matrix at 0x20.
pub const MODELS: Layout = Layout { first_at: 0, count_at: 4, matrix_at: 0x20, slot_at: 0 };
/// The prop, compound and dynamic lists: a header of `kind, first record, count, 0`, and records with the matrix first and the
/// object's slot at 0x40 **[verified]** (the imports of a list are 0x60 bytes apart from 0x50).
pub const OBJECTS: Layout = Layout { first_at: 4, count_at: 8, matrix_at: 0, slot_at: 0x40 };

/// One placed model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    /// The `Model` resource.
    pub model: u64,
    /// A 4 by 4 matrix of row vectors with the translation in the last row (engine space), `world = local * matrix`.
    pub matrix: [f32; 16],
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The instances of an `InstanceList`.
pub fn parse(list: &Resource) -> Result<Vec<Instance>> {
    parse_with(list, MODELS)
}

/// The instances of a list with records laid out as `layout` says. `Instance::model` is the id the record's slot imports: a model,
/// or the object that has one. A record whose slot has no import (a record the loader would leave empty) is left out.
pub fn parse_with(list: &Resource, layout: Layout) -> Result<Vec<Instance>> {
    let main = &list.blocks[0];
    let (Some(first), Some(count)) = (u32_at(main, layout.first_at), u32_at(main, layout.count_at)) else {
        anyhow::bail!("the instance list is too short for its header");
    };
    let (first, count) = (first as usize, count as usize);
    ensure!(
        first + count * RECORD_LEN <= main.len(),
        "the instance list says {count} records and holds {} bytes",
        main.len()
    );
    let models: HashMap<u32, u64> = list.imports.iter().map(|import| (import.slot(), import.id)).collect();
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let at = first + n * RECORD_LEN;
        let Some(&model) = models.get(&((at + layout.slot_at) as u32)) else { continue };
        let mut matrix = [0.0f32; 16];
        for (k, value) in matrix.iter_mut().enumerate() {
            *value = f32::from_bits(u32_at(main, at + layout.matrix_at + 4 * k).unwrap_or(0));
        }
        out.push(Instance { model, matrix });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chameleon_bundle::{Entry, Import};

    /// A list of `(model id, translation)` records with the identity rotation.
    fn list(records: &[(u64, [f32; 3])], skip_import: Option<usize>) -> Resource {
        let first = 0x10usize;
        let mut main = vec![0u8; first + records.len() * RECORD_LEN];
        main[0..4].copy_from_slice(&(first as u32).to_le_bytes());
        main[4..8].copy_from_slice(&(records.len() as u32).to_le_bytes());
        let mut imports = Vec::new();
        for (n, &(model, translation)) in records.iter().enumerate() {
            let at = first + n * RECORD_LEN;
            if skip_import != Some(n) {
                imports.push(Import { id: model, offset: at as u32 });
            }
            let mut matrix = [0.0f32; 16];
            matrix[0] = 1.0;
            matrix[5] = 1.0;
            matrix[10] = 1.0;
            matrix[15] = 1.0;
            matrix[12..15].copy_from_slice(&translation);
            for (k, value) in matrix.iter().enumerate() {
                main[at + MODELS.matrix_at + 4 * k..at + MODELS.matrix_at + 4 * k + 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        let entry = Entry {
            id: 1,
            size_and_alignment: [main.len() as u32, 0, 0, 0],
            disk_size: [0; 4],
            disk_offset: [0; 4],
            import_offset: 0,
            kind: INSTANCE_LIST,
            import_count: imports.len() as u16,
            flags: 0,
            stream_index: 0,
        };
        Resource { entry, blocks: [main, Vec::new(), Vec::new(), Vec::new()], imports }
    }

    #[test]
    fn each_record_gives_its_model_and_matrix() {
        let instances = parse(&list(&[(0xAA, [1.0, 2.0, 3.0]), (0xBB, [-4.0, 5.0, 6.0])], None)).unwrap();
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].model, 0xAA);
        assert_eq!(instances[1].matrix[12..15], [-4.0, 5.0, 6.0]);
        assert_eq!(instances[1].matrix[0], 1.0);
    }

    #[test]
    fn the_records_of_an_object_list_keep_the_matrix_first_and_the_slot_at_0x40() {
        let mut main = vec![0u8; 0x10 + RECORD_LEN];
        main[0..4].copy_from_slice(&2u32.to_le_bytes());
        main[4..8].copy_from_slice(&0x10u32.to_le_bytes());
        main[8..12].copy_from_slice(&1u32.to_le_bytes());
        let matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 7.0, 8.0, 9.0, 0.94];
        for (k, value) in matrix.iter().enumerate() {
            main[0x10 + 4 * k..0x10 + 4 * k + 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut object_list = list(&[], None);
        object_list.blocks[0] = main;
        object_list.imports = vec![Import { id: 0xCC, offset: 0x50 }];
        let instances = parse_with(&object_list, OBJECTS).unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].model, 0xCC);
        assert_eq!(instances[0].matrix[12..16], [7.0, 8.0, 9.0, 0.94]);
    }

    #[test]
    fn a_record_with_no_model_is_left_out_and_a_short_list_is_refused() {
        let instances = parse(&list(&[(0xAA, [0.0; 3]), (0xBB, [0.0; 3])], Some(0))).unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].model, 0xBB);
        let mut broken = list(&[(0xAA, [0.0; 3])], None);
        broken.blocks[0].truncate(0x20);
        assert!(parse(&broken).is_err());
    }
}
