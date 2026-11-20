//! Writing a `ScenerySection` chunk: the inverse of [`crate::read_scenery_sections`], for tools that make a section file
//! (a world streamed from another game's files writes the placements of each piece into the section it hands the game).

use blackbox_chunk::{ChunkWriter, ids};

use crate::layout::SceneryLayout;
use crate::{OverrideHook, SceneryInfo, SceneryInstance, ScenerySection};

/// Bytes of the section header chunk. The reader takes the section number from it; the rest is not read.
const HEADER_LEN: usize = 60;

/// Appends `section` to `out` as a `ScenerySection` container in the record layout of `layout`. `out` must start at a
/// multiple of 16 in the file it ends up in, as the alignment of the instance records is measured from its start.
pub fn write_section(out: &mut ChunkWriter, section: &ScenerySection, layout: &SceneryLayout) {
    let mut header = vec![0u8; HEADER_LEN.max(layout.header_section_number + 4)];
    let at = layout.header_section_number;
    header[at..at + 4].copy_from_slice(&section.section_number.to_le_bytes());
    let infos: Vec<u8> = section.infos.iter().flat_map(|info| info_record(info, layout)).collect();
    let instances: Vec<u8> = section.instances.iter().flat_map(|instance| instance_record(instance, layout)).collect();
    let hooks: Vec<u8> = section.override_hooks.iter().flat_map(|hook| hook_record(hook, layout)).collect();
    out.container(ids::SCENERY_SECTION, |section_chunk| {
        section_chunk.leaf(ids::SCENERY_SECTION_HEADER, &header);
        section_chunk.leaf_aligned(ids::SCENERY_INSTANCES, layout.instance.align, &instances);
        section_chunk.leaf_aligned(ids::SCENERY_INFOS, layout.info.align, &infos);
        if !hooks.is_empty() {
            section_chunk.leaf_aligned(ids::SCENERY_OVERRIDE_HOOKS, layout.hooks.align, &hooks);
        }
    });
}

fn put(record: &mut [u8], at: usize, bytes: &[u8]) {
    record[at..at + bytes.len()].copy_from_slice(bytes);
}

fn info_record(info: &SceneryInfo, layout: &SceneryLayout) -> Vec<u8> {
    let l = &layout.info;
    let mut record = vec![0u8; l.len];
    let name = info.name.as_bytes();
    // The name keeps a terminating NUL, as the game's records do.
    let kept = name.len().min(l.name_len.saturating_sub(1));
    put(&mut record, l.name, &name[..kept]);
    for (slot, key) in info.solid_keys.iter().take(l.lods).enumerate() {
        put(&mut record, l.solid_keys + 4 * slot, &key.to_le_bytes());
    }
    put(&mut record, l.radius, &info.radius.to_le_bytes());
    put(&mut record, l.hierarchy_hash, &info.hierarchy_hash.to_le_bytes());
    record
}

fn instance_record(instance: &SceneryInstance, layout: &SceneryLayout) -> Vec<u8> {
    let l = &layout.instance;
    let mut record = vec![0u8; l.len];
    let floats = |values: [f32; 3]| values.map(f32::to_le_bytes).concat();
    put(&mut record, l.bbox_min, &floats(instance.bbox_min));
    put(&mut record, l.bbox_max, &floats(instance.bbox_max));
    put(&mut record, l.exclude_flags, &instance.exclude_flags.to_le_bytes());
    put(&mut record, l.preculler_index, &instance.preculler_index.to_le_bytes());
    put(&mut record, l.lighting_context, &instance.lighting_context.to_le_bytes());
    put(&mut record, l.position, &floats(instance.position));
    for (row, axis) in instance.rotation.iter().enumerate() {
        for (col, value) in axis.iter().enumerate() {
            let stored = (value * l.rotation_scale).round().clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
            put(&mut record, l.rotation + (row * 3 + col) * 2, &stored.to_le_bytes());
        }
    }
    put(&mut record, l.info_index, &instance.info_index.to_le_bytes());
    record
}

fn hook_record(hook: &OverrideHook, layout: &SceneryLayout) -> Vec<u8> {
    let l = &layout.hooks;
    let mut record = vec![0u8; l.len];
    put(&mut record, l.override_index, &hook.override_index.to_le_bytes());
    put(&mut record, l.instance_index, &hook.instance_index.to_le_bytes());
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{Placement, section as build_section};
    use crate::layout::MOST_WANTED;
    use crate::read_scenery_sections;

    fn placement(name: &str, x: f32, rows: [[f32; 3]; 3]) -> Placement {
        Placement {
            name: name.to_owned(),
            solids: [0xAABB_CCDD, 0, 0x1122_3344, 0],
            local_min: [-1.0, -2.0, 0.0],
            local_max: [1.0, 2.0, 3.0],
            radius: 4.5,
            position: [x, 10.0, 20.0],
            rows,
            exclude_flags: 0x60,
        }
    }

    #[test]
    fn a_written_section_reads_back_the_same() {
        let rows = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let placements = [placement("U1_A", 100.5, rows), placement("A_NAME_LONGER_THAN_THE_FIELD", -5.0, rows)];
        let mut section = build_section(2705, &placements).unwrap();
        section.override_hooks.push(OverrideHook { override_index: 7, instance_index: 1 });
        // Other chunks come first in a section file, so the section starts at an offset that is not a multiple of 0x800.
        let mut out = ChunkWriter::new();
        out.leaf(0x0000_0042, &[1, 2, 3]);
        out.pad_to(0x10);
        write_section(&mut out, &section, &MOST_WANTED);
        let bytes = out.finish().unwrap();
        let read = read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        assert_eq!(read.len(), 1);
        let got = &read[0];
        assert_eq!(got.section_number, 2705);
        assert_eq!(got.override_hooks, section.override_hooks);
        assert_eq!(got.instances, section.instances);
        assert_eq!(got.infos[0], section.infos[0]);
        assert_eq!(got.infos[1].name, "A_NAME_LONGER_THAN_THE_", "the name is cut to leave the NUL");
        assert_eq!(got.infos[1].solid_keys, [0xAABB_CCDD, 0, 0x1122_3344, 0]);
        assert_eq!(got.infos[1].radius, 4.5);
    }

    #[test]
    fn a_section_with_nothing_in_it_reads_back_empty() {
        let section =
            ScenerySection { section_number: 9, infos: Vec::new(), instances: Vec::new(), override_hooks: Vec::new() };
        let mut out = ChunkWriter::new();
        write_section(&mut out, &section, &MOST_WANTED);
        let read = read_scenery_sections(&out.finish().unwrap(), &MOST_WANTED).unwrap();
        assert_eq!(read, vec![section]);
    }
}
