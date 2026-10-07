//! Finding and decoding the index.

use blackbox_chunk::ids;

use crate::layout::SectionLayout;
use crate::{Error, Result, StreamingSection};

/// Read every section from the track metadata file (e.g. `TRACKS/L2RA.BUN`).
pub fn read_sections(meta: &[u8], layout: &SectionLayout) -> Result<Vec<StreamingSection>> {
    let chunk = blackbox_chunk::find(meta, ids::TRACK_STREAMING_SECTIONS).ok_or(Error::NoIndex)?;
    if chunk.payload.len() % layout.len != 0 {
        return Err(Error::BadSize { len: chunk.payload.len(), record: layout.len });
    }
    Ok(chunk.payload.chunks_exact(layout.len).map(|r| decode(r, layout)).collect())
}

fn decode(r: &[u8], l: &SectionLayout) -> StreamingSection {
    let u32_at = |o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    let f32_at = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    let name = &r[l.name..l.name + 8];
    let name_len = name.iter().position(|&b| b == 0).unwrap_or(8);
    StreamingSection {
        name: String::from_utf8_lossy(&name[..name_len]).into_owned(),
        number: i16::from_le_bytes([r[l.number], r[l.number + 1]]),
        file_type: u32_at(l.file_type) as i32,
        file_offset: u32_at(l.file_offset),
        size: u32_at(l.size),
        compressed_size: u32_at(l.compressed_size),
        perm_size: u32_at(l.perm_size),
        priority: u32_at(l.priority) as i32,
        center: [f32_at(l.center), f32_at(l.center + 4)],
        radius: f32_at(l.radius),
        checksum: u32_at(l.checksum),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MOST_WANTED;

    fn record(name: &str, number: i16, offset: u32, size: u32, center: [f32; 2], radius: f32) -> Vec<u8> {
        let mut r = vec![0u8; MOST_WANTED.len];
        r[..name.len()].copy_from_slice(name.as_bytes());
        r[0x08..0x0A].copy_from_slice(&number.to_le_bytes());
        r[0x14..0x18].copy_from_slice(&offset.to_le_bytes());
        r[0x18..0x1C].copy_from_slice(&size.to_le_bytes());
        r[0x28..0x2C].copy_from_slice(&center[0].to_le_bytes());
        r[0x2C..0x30].copy_from_slice(&center[1].to_le_bytes());
        r[0x30..0x34].copy_from_slice(&radius.to_le_bytes());
        r
    }

    #[test]
    fn reads_index() {
        let payload =
            [record("X0", 2400, 0, 0x800, [0.0, 0.0], 0.0), record("A41", 141, 0x800, 100, [10.0, 20.0], 5.0)].concat();
        let mut file = ids::TRACK_STREAMING_SECTIONS.to_le_bytes().to_vec();
        file.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        file.extend_from_slice(&payload);

        let s = read_sections(&file, &MOST_WANTED).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].name.as_str(), s[0].number, s[0].is_spatial()), ("X0", 2400, false));
        assert_eq!((s[1].name.as_str(), s[1].number, s[1].range()), ("A41", 141, 0x800..0x864));
        assert!(s[1].is_spatial());
        assert_eq!(s[1].distance_to(10.0, 20.0), 0.0);
        assert_eq!(s[1].distance_to(10.0, 35.0), 10.0);
    }

    #[test]
    fn missing_index_is_error() {
        assert!(matches!(read_sections(&[], &MOST_WANTED), Err(Error::NoIndex)));
    }
}
