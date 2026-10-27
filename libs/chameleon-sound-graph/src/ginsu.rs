//! The `Ginsu` engine resources (kind 0x80) of a car bundle: a granular engine recording with its tables.
//!
//! The block starts with a volume (f32) and a zero word, then holds the `.gin` layout of the 2005 game with a
//! version-3 header: `"Gnsu30"`, the lowest and highest frequency, the segment, cycle and sample counts, the sample
//! rate, two more words, a table of `seg_count` frequency positions, a table of `cycle_count + 1` cycle positions and
//! the audio. [`GinsuResource::to_gin`] writes the same recording in the version-2 layout that `blackbox-ginsu` and
//! `ea-audio` read.

use crate::bytes::{f32_at, u32_at};

/// The prefix before the `Gnsu` magic: a volume and a zero word.
const PREFIX: usize = 8;
/// The fixed header of the version-3 file, from the magic: the version-2 header plus two words.
const HEADER: usize = 0x28;
/// Bytes of the version-2 fixed header.
const HEADER_V2: usize = 0x20;

/// A Ginsu resource, split into its parts.
#[derive(Debug, Clone, PartialEq)]
pub struct GinsuResource {
    /// The volume at the start of the block (0.5 on the Porsche's).
    pub volume: f32,
    pub min_frequency: f32,
    pub max_frequency: f32,
    pub sample_count: u32,
    pub sample_rate: u32,
    /// `seg_count` positions of the frequency table.
    pub freq_pos: Vec<u32>,
    /// `cycle_count + 1` positions of the cycle table.
    pub cycle_pos: Vec<u32>,
    /// The EA-XAS audio that follows the tables.
    pub payload: Vec<u8>,
}

impl GinsuResource {
    /// Read the main block of a Ginsu resource; `None` when it is not a version-3 Ginsu block.
    pub fn read(block: &[u8]) -> Option<Self> {
        if block.get(PREFIX..PREFIX + 5)? != b"Gnsu3" {
            return None;
        }
        let file = &block[PREFIX..];
        let seg_count = u32_at(file, 0x10)? as usize;
        let cycle_count = u32_at(file, 0x14)? as usize;
        if seg_count == 0 || seg_count > 4096 || cycle_count == 0 || cycle_count > 1 << 20 {
            return None;
        }
        let words = |from: usize, count: usize| -> Option<Vec<u32>> {
            (0..count).map(|n| u32_at(file, from + 4 * n)).collect()
        };
        let freq_pos = words(HEADER, seg_count)?;
        let cycle_pos = words(HEADER + 4 * seg_count, cycle_count + 1)?;
        let payload_at = HEADER + 4 * (seg_count + cycle_count + 1);
        Some(Self {
            volume: f32_at(block, 0)?,
            min_frequency: f32_at(file, 8)?,
            max_frequency: f32_at(file, 0xC)?,
            sample_count: u32_at(file, 0x18)?,
            sample_rate: u32_at(file, 0x1C)?,
            freq_pos,
            cycle_pos,
            payload: file.get(payload_at..)?.to_vec(),
        })
    }

    /// The recording as a version-2 `.gin` file: the frequency table gets the position of the lowest frequency (the
    /// first cycle) in front of the version-3 table's entries, so it has `seg_count + 1` positions.
    pub fn to_gin(&self) -> Vec<u8> {
        let segments = self.freq_pos.len() as u32;
        let mut out =
            Vec::with_capacity(HEADER_V2 + 4 * (self.freq_pos.len() + self.cycle_pos.len() + 1) + self.payload.len());
        out.extend_from_slice(b"Gnsu20\0\0");
        out.extend_from_slice(&self.min_frequency.to_le_bytes());
        out.extend_from_slice(&self.max_frequency.to_le_bytes());
        out.extend_from_slice(&segments.to_le_bytes());
        out.extend_from_slice(&((self.cycle_pos.len() - 1) as u32).to_le_bytes());
        out.extend_from_slice(&self.sample_count.to_le_bytes());
        out.extend_from_slice(&self.sample_rate.to_le_bytes());
        out.extend_from_slice(&self.cycle_pos[0].to_le_bytes());
        for position in self.freq_pos.iter().chain(&self.cycle_pos) {
            out.extend_from_slice(&position.to_le_bytes());
        }
        out.extend_from_slice(&self.payload);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(seg: u32, cycles: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0.5f32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"Gnsu30\0\0");
        out.extend_from_slice(&1600.0f32.to_le_bytes());
        out.extend_from_slice(&7800.0f32.to_le_bytes());
        for word in [seg, cycles, 6400, 48_000, 0, 700] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        for n in 0..seg {
            out.extend_from_slice(&(100 + n * 100).to_le_bytes());
        }
        for n in 0..=cycles {
            out.extend_from_slice(&(1 + n * 64).to_le_bytes());
        }
        out.extend_from_slice(&[0x11; 38]);
        out
    }

    #[test]
    fn the_tables_and_the_audio_are_split() {
        let resource = GinsuResource::read(&block(4, 10)).unwrap();
        assert_eq!((resource.volume, resource.sample_rate, resource.sample_count), (0.5, 48_000, 6400));
        assert_eq!((resource.freq_pos.len(), resource.cycle_pos.len(), resource.payload.len()), (4, 11, 38));
        assert_eq!((resource.freq_pos[0], resource.cycle_pos[10]), (100, 641));
    }

    #[test]
    fn the_version_2_file_has_one_more_frequency_position_and_the_same_audio() {
        let resource = GinsuResource::read(&block(4, 10)).unwrap();
        let gin = resource.to_gin();
        assert_eq!(&gin[..6], b"Gnsu20");
        assert_eq!(u32::from_le_bytes(gin[0x10..0x14].try_into().unwrap()), 4);
        let tables = 0x20 + 4 * (5 + 11);
        assert_eq!(gin.len(), tables + 38);
        assert_eq!(
            u32::from_le_bytes(gin[0x20..0x24].try_into().unwrap()),
            1,
            "the first cycle starts the frequency table"
        );
    }

    #[test]
    fn a_block_that_is_not_ginsu_is_refused() {
        assert!(GinsuResource::read(&[0; 64]).is_none());
        assert!(GinsuResource::read(b"short").is_none());
    }
}
