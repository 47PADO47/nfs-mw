//! `ScenerySection` chunks: header, infos and instances.

use blackbox_chunk::{Chunk, ids};

use crate::layout::SceneryLayout;
use crate::{Error, Result, SceneryInfo, SceneryInstance};

#[derive(Debug, Clone, PartialEq)]
pub struct ScenerySection {
    /// The streaming section it belongs to (`TrackStreamingSection::number`).
    pub section_number: u32,
    pub infos: Vec<SceneryInfo>,
    pub instances: Vec<SceneryInstance>,
}

impl ScenerySection {
    /// The info of an instance, if its index is valid.
    pub fn info_of(&self, instance: &SceneryInstance) -> Option<&SceneryInfo> {
        usize::try_from(instance.info_index).ok().and_then(|i| self.infos.get(i))
    }
}

/// Every `ScenerySection` in `data` (one streamed section, or a whole file).
pub fn read_scenery_sections(data: &[u8], layout: &SceneryLayout) -> Result<Vec<ScenerySection>> {
    blackbox_chunk::find_all(data, ids::SCENERY_SECTION).into_iter().map(|c| read_section(c, layout)).collect()
}

fn read_section(section: Chunk<'_>, layout: &SceneryLayout) -> Result<ScenerySection> {
    let err = |detail: String| Error::Section { offset: section.offset, detail };
    let header = section.child(ids::SCENERY_SECTION_HEADER).ok_or_else(|| err("no header".into()))?.payload;
    let n = layout.header_section_number;
    let section_number = header
        .get(n..n + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| err("header too short".into()))?;

    let records = |id: u32, len: usize, align: usize| -> Result<Vec<&[u8]>> {
        let Some(chunk) = section.child(id) else { return Ok(Vec::new()) };
        let payload = chunk.aligned_payload(align);
        if payload.len() % len != 0 {
            return Err(err(format!("chunk 0x{id:08X}: {} bytes is not a multiple of {len}", payload.len())));
        }
        Ok(payload.chunks_exact(len).collect())
    };
    let infos = records(ids::SCENERY_INFOS, layout.info.len, layout.info.align)?
        .into_iter()
        .map(|r| SceneryInfo::decode(r, &layout.info))
        .collect();
    let instances = records(ids::SCENERY_INSTANCES, layout.instance.len, layout.instance.align)?
        .into_iter()
        .map(|r| SceneryInstance::decode(r, &layout.instance))
        .collect();
    Ok(ScenerySection { section_number, infos, instances })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MOST_WANTED;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn reads_section() {
        let mut header = vec![0u8; 60];
        header[0x0C..0x10].copy_from_slice(&141u32.to_le_bytes());
        let mut info = vec![0u8; 72];
        info[..5].copy_from_slice(b"LAMP1");
        info[0x18..0x1C].copy_from_slice(&0xAABB_CCDDu32.to_le_bytes());
        let mut inst = vec![0u8; 64];
        inst[0x20..0x24].copy_from_slice(&5.0f32.to_le_bytes()); // position.x
        for (i, v) in [8192i16, 0, 0, 0, 0, 8192, 0, -8192, 0].iter().enumerate() {
            inst[0x2C + i * 2..0x2E + i * 2].copy_from_slice(&v.to_le_bytes());
        }
        // Section payload starts at 8; header ends at 76; instances chunk payload at 84 -> pad 12 to 96.
        let mut padded = vec![0x11; 12];
        padded.extend_from_slice(&inst);
        let body = [
            chunk(ids::SCENERY_SECTION_HEADER, &header),
            chunk(ids::SCENERY_INSTANCES, &padded),
            chunk(ids::SCENERY_INFOS, &info),
        ]
        .concat();
        let data = chunk(ids::SCENERY_SECTION, &body);

        let sections = read_scenery_sections(&data, &MOST_WANTED).unwrap();
        let s = &sections[0];
        assert_eq!(s.section_number, 141);
        assert_eq!(s.infos[0].name, "LAMP1");
        assert_eq!(s.infos[0].best_solid(), Some(0xAABB_CCDD));
        let i = &s.instances[0];
        assert_eq!(i.position, [5.0, 0.0, 0.0]);
        assert_eq!(i.rotation, [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]]);
        assert_eq!(s.info_of(i).unwrap().name, "LAMP1");
        assert_eq!(i.matrix_columns()[3], [5.0, 0.0, 0.0, 1.0]);
        let rules = &MOST_WANTED.visibility;
        assert!(i.visible_in(rules.player_view, rules));
        let barrier = SceneryInstance { exclude_flags: 0x0280_8070, ..i.clone() };
        assert!(!barrier.visible_in(rules.player_view, rules));
        let mirror_only = SceneryInstance { exclude_flags: 0x20, ..i.clone() };
        assert!(mirror_only.visible_in(0x20, rules) && !i.visible_in(0x20, rules));
    }
}
