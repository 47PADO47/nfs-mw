//! The collision stitches of `GLOBAL/InGameB.bun`: a collision sound is a short chain of pieces of
//! `IG_GLOBAL/Stich_Collision_MB.abk`, each with its own pitch and start offset.
//! Layout and what is unconfirmed: `docs/formats/audio.md` ("Sound stitches").

use blackbox_chunk::chunks;

const BUNDLE: u32 = 0x8003_B500;
const DATA: u32 = 0x0003_B502;
const PIECES: u32 = 0x0003_B503;
/// Bytes of one piece record.
const PIECE_LEN: usize = 16;

/// One piece of a stitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StitchPiece {
    /// The sound of the bank, counted from 0 (`BNKl` entry `sample + 1`).
    pub sample: u8,
    pub family: i8,
    /// Volume of the piece, 0 to 1 (`0x7FFF` is 1).
    pub volume: f32,
    /// Authored Q12 playback pitch; this is not a delay or sample count.
    pub pitch: u16,
    /// Independent authored delay passed to the stitch module.
    pub offset: u16,
    pub azimuth: u16,
    pub random_volume: u16,
    pub random_pitch: i16,
    pub priority: u8,
    pub rolloff: u8,
}

/// A stitch: the pieces in playing order and the stitch's own volume.
#[derive(Debug, Clone, PartialEq)]
pub struct Stitch {
    pub volume: f32,
    pub pieces: Vec<StitchPiece>,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2).map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}

fn volume(raw: u16) -> f32 {
    f32::from(raw) / 32767.0
}

fn pieces_of(refs: &[u8]) -> Vec<StitchPiece> {
    let (records, _) = refs.as_chunks::<PIECE_LEN>();
    records
        .iter()
        .map(|r| StitchPiece {
            sample: r[0],
            family: r[1] as i8,
            volume: volume(u16_at(r, 2)),
            pitch: u16_at(r, 4),
            offset: u16_at(r, 6),
            azimuth: u16_at(r, 8),
            random_volume: u16_at(r, 10),
            random_pitch: u16_at(r, 12) as i16,
            priority: r[14],
            rolloff: r[15],
        })
        .collect()
}

/// Family-zero stitches of `InGameB.bun` (the collision sounds), by id. Data that does not
/// parse gives an empty list.
pub fn collision_stitches(ingame_b: &[u8]) -> Vec<Stitch> {
    family_stitches(ingame_b, 0)
}

/// Whoosh stitches, indexed independently from collision and static stitches.
pub fn whoosh_stitches(ingame_b: &[u8]) -> Vec<Stitch> {
    family_stitches(ingame_b, 1)
}

fn family_stitches(ingame_b: &[u8], family: u8) -> Vec<Stitch> {
    let mut result = Vec::new();
    for bundle in chunks(ingame_b).filter_map(Result::ok).filter(|c| c.id == BUNDLE) {
        let children: Vec<_> = bundle.children().filter_map(Result::ok).collect();
        for pair in children.windows(2) {
            let [data, pieces] = pair else { continue };
            if data.id != DATA || pieces.id != PIECES || data.payload.get(8).copied() != Some(family) {
                continue;
            }
            result.push(Stitch { volume: volume(u16_at(data.payload, 4)), pieces: pieces_of(pieces.payload) });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_record_keeps_pitch_separate_from_offset_and_all_authored_controls() {
        let mut record = vec![0u8; 16];
        record[0..2].copy_from_slice(&31u16.to_le_bytes());
        record[2..4].copy_from_slice(&0x3FFFu16.to_le_bytes());
        record[4..6].copy_from_slice(&2733u16.to_le_bytes());
        record[6..8].copy_from_slice(&75u16.to_le_bytes());
        record[8..10].copy_from_slice(&8192u16.to_le_bytes());
        record[10..12].copy_from_slice(&1000u16.to_le_bytes());
        record[12..14].copy_from_slice(&(-400i16).to_le_bytes());
        record[14] = 200;
        record[15] = 2;
        let pieces = pieces_of(&[record.clone(), record].concat());
        assert_eq!(pieces.len(), 2);
        assert_eq!((pieces[0].sample, pieces[0].pitch, pieces[0].offset), (31, 2733, 75));
        assert_eq!((pieces[0].azimuth, pieces[0].random_volume, pieces[0].random_pitch), (8192, 1000, -400));
        assert_eq!((pieces[0].priority, pieces[0].rolloff), (200, 2));
        assert!((pieces[0].volume - 0.5).abs() < 1e-3);
    }

    #[test]
    fn no_bundle_gives_no_stitches() {
        assert!(collision_stitches(&[]).is_empty());
    }

    #[test]
    fn independent_stitch_families_are_read_across_all_bundles() {
        fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
            [id.to_le_bytes().as_slice(), (payload.len() as u32).to_le_bytes().as_slice(), payload].concat()
        }
        fn bundle(family: u8, sample: u8) -> Vec<u8> {
            let mut data = [0; 20];
            data[8] = family;
            let mut piece = [0; 16];
            piece[0] = sample;
            piece[1] = family;
            chunk(BUNDLE, &[chunk(DATA, &data), chunk(PIECES, &piece)].concat())
        }
        let bytes = [bundle(1, 9), bundle(0, 20), bundle(1, 10), bundle(2, 30)].concat();
        assert_eq!(collision_stitches(&bytes)[0].pieces[0].sample, 20);
        let whoosh = whoosh_stitches(&bytes);
        assert_eq!(whoosh.len(), 2);
        assert_eq!(whoosh.iter().map(|s| s.pieces[0].sample).collect::<Vec<_>>(), [9, 10]);
    }
}
