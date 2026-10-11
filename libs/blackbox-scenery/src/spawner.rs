//! Physics placements and their scenery/attribute keys. `docs/formats/prop-spawners.md`.

use blackbox_chunk::{Chunk, ids};

use crate::{Error, Result, layout::SpawnerLayout};

/// An instance of a physical world object, in source physics coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectSpawner {
    pub orientation: [f32; 4],
    pub position: [f32; 3],
    pub model: u32,
    pub collision: u32,
    pub attributes: u32,
    pub override_index: u32,
    pub unique_id: u32,
    pub exclude_flags: u32,
}

/// Read every object placement pack in a section or file. Runtime pointers are ignored.
pub fn read_object_spawners(data: &[u8], layout: &SpawnerLayout) -> Result<Vec<ObjectSpawner>> {
    let mut out = Vec::new();
    for chunk in blackbox_chunk::find_all(data, ids::SMOKEABLE_SPAWNERS) {
        read_pack(chunk, layout, &mut out)?;
    }
    Ok(out)
}

fn read_pack(chunk: Chunk<'_>, l: &SpawnerLayout, out: &mut Vec<ObjectSpawner>) -> Result<()> {
    let err = |detail: &str| Error::Section { offset: chunk.offset, detail: detail.to_owned() };
    let data = chunk.aligned_payload(l.align);
    let count = data.get(l.count..l.count + 2).ok_or_else(|| err("missing spawner count"))?;
    let count =
        usize::try_from(i16::from_le_bytes(count.try_into().unwrap())).map_err(|_| err("negative spawner count"))?;
    if l.len == 0 || data.len() != l.header_len + count * l.len {
        return Err(err("spawner count does not match payload size"));
    }
    for r in data[l.header_len..].chunks_exact(l.len) {
        let word = |n| -> Result<u32> {
            let bytes = r.get(n..n + 4).ok_or_else(|| err("spawner field outside record"))?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let vector = |offset| -> Result<[f32; 3]> {
            Ok([f32::from_bits(word(offset)?), f32::from_bits(word(offset + 4)?), f32::from_bits(word(offset + 8)?)])
        };
        let xyz = vector(l.orientation)?;
        out.push(ObjectSpawner {
            orientation: [xyz[0], xyz[1], xyz[2], f32::from_bits(word(l.orientation + 12)?)],
            position: vector(l.position)?,
            model: word(l.model)?,
            collision: word(l.collision)?,
            attributes: word(l.attributes)?,
            override_index: word(l.override_index)?,
            unique_id: word(l.unique_id)?,
            exclude_flags: word(l.exclude_flags)?,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MOST_WANTED;

    fn pack(count: i16, records: usize, prefix: usize) -> Vec<u8> {
        let mut data = vec![0x11; 8];
        data.extend_from_slice(&[0; 16]);
        data[20..22].copy_from_slice(&count.to_le_bytes());
        data.resize(data.len() + 64 * records, 0);
        let mut bytes = Vec::new();
        if prefix != 0 {
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(&(prefix as u32).to_le_bytes());
            bytes.resize(8 + prefix, 0);
        }
        // Keep each test payload 8 bytes short of a 16-byte boundary.
        assert_eq!(bytes.len() % 16, 0);
        bytes.extend_from_slice(&ids::SMOKEABLE_SPAWNERS.to_le_bytes());
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&data);
        bytes
    }

    #[test]
    fn fields_and_alignment_survive_a_nonzero_file_offset() {
        let mut bytes = pack(1, 1, 8);
        let record = &mut bytes[48..];
        for (i, v) in [0.0f32, 0.0, 0.0, 1.0, 10.0, 20.0, 30.0, 1.0].iter().enumerate() {
            record[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        for (i, v) in [11u32, 12, 13, 14, 15, 16, u32::MAX, u32::MAX].iter().enumerate() {
            record[32 + i * 4..36 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        let s = read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].position, [10.0, 20.0, 30.0]);
        assert_eq!(s[0].orientation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!((s[0].model, s[0].collision, s[0].attributes), (11, 12, 13));
        assert_eq!((s[0].override_index, s[0].unique_id, s[0].exclude_flags), (14, 15, 16));
    }

    #[test]
    fn rejects_negative_counts_and_incomplete_or_excess_records() {
        for (count, records) in [(-1, 0), (1, 0), (0, 1), (2, 1)] {
            assert!(read_object_spawners(&pack(count, records, 0), &MOST_WANTED.spawners).is_err());
        }
        assert!(read_object_spawners(&pack(0, 0, 0), &MOST_WANTED.spawners).unwrap().is_empty());
    }
}
