//! Tests for the channel codecs. Each leaf is built in memory: its DeltaChan header sits at offset 0 of `.data`
//! and its DeltaCompressedData block at `BLOCK_AT`. No install data is read.

use super::elf_builder::Builder;
use crate::codec::{Channel, DELTA_LERP, DELTA_QUAT, read_leaf};
use crate::{Error, Object};

const BLOCK_AT: usize = 0x40;
/// Written into every slot that a channel must not touch.
const UNTOUCHED: f32 = -999.0;

/// The minimum, range and start value of one dof.
#[derive(Clone, Copy)]
struct Info {
    min: f32,
    range: f32,
    start: f32,
}

/// Packs one row of raw deltas at `bits` (8, 16 or 4). At 4 bits the even dof is the low nibble.
fn encode_row(bits: u8, raws: &[u32]) -> Vec<u8> {
    match bits {
        8 => raws.iter().map(|&raw| raw as u8).collect(),
        16 => raws.iter().flat_map(|&raw| (raw as u16).to_le_bytes()).collect(),
        _ => raws
            .chunks(2)
            .map(|pair| {
                let low = pair[0] as u8 & 0x0F;
                let high = pair.get(1).map_or(0, |&raw| (raw as u8 & 0x0F) << 4);
                low | high
            })
            .collect(),
    }
}

/// A DeltaCompressedData block: the header, one DofInfo per dof, then one row per frame.
fn block(bits: u8, infos: &[Info], rows: &[Vec<u32>]) -> Vec<u8> {
    let mut out = (infos.len() as u16).to_le_bytes().to_vec();
    out.extend([bits, 0]);
    for info in infos {
        out.extend(info.min.to_le_bytes());
        out.extend(info.range.to_le_bytes());
        out.extend(info.start.to_le_bytes());
    }
    for row in rows {
        out.extend(encode_row(bits, row));
    }
    out
}

/// A DeltaChan leaf with `dofs` as its inline list and `block` at `BLOCK_AT`, as a relocated object.
fn leaf_with(type_id: u16, frames: u16, dofs: &[u16], block: &[u8]) -> Object {
    let mut data = vec![0u8; BLOCK_AT];
    data[0..2].copy_from_slice(&type_id.to_le_bytes());
    data[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
    data[4..8].copy_from_slice(&(BLOCK_AT as u32).to_le_bytes());
    data[8..10].copy_from_slice(&frames.to_le_bytes());
    for (i, dof) in dofs.iter().enumerate() {
        data[0x0A + 2 * i..0x0C + 2 * i].copy_from_slice(&dof.to_le_bytes());
    }
    data.extend_from_slice(block);
    Object::read(&Builder::new(&data).reloc(4).bare()).expect("synthetic leaf reads")
}

fn leaf(type_id: u16, frames: u16, dofs: &[u16], bits: u8, infos: &[Info], rows: &[Vec<u32>]) -> Object {
    leaf_with(type_id, frames, dofs, &block(bits, infos, rows))
}

fn read(object: &Object, type_id: u16) -> Box<dyn Channel + Send + Sync> {
    match read_leaf(object, type_id, 0) {
        Ok(channel) => channel,
        Err(error) => panic!("leaf should read: {error}"),
    }
}

fn read_error(object: &Object, type_id: u16) -> Error {
    match read_leaf(object, type_id, 0) {
        Ok(_) => panic!("leaf should not read"),
        Err(error) => error,
    }
}

/// The 32 slots of a sample, with `UNTOUCHED` where the channel wrote nothing.
fn sample(channel: &dyn Channel, frame: f32) -> Vec<f32> {
    let mut out = vec![UNTOUCHED; 32];
    channel.sample(frame, &mut out);
    out
}

fn assert_close(actual: &[f32], expected: &[f32], tolerance: f32) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() <= tolerance, "{actual:?} is not {expected:?}");
    }
}

/// Three scalar dofs at 8 bits over four frames: (1, -2, 0), (1, 0, 1), (0.5, 0, 6), (63.75, 64, 5).
fn lerp_8bit() -> Object {
    let infos = [
        Info { min: -0.5, range: 0.25, start: 1.0 },
        Info { min: 0.0, range: 0.5, start: -2.0 },
        Info { min: -1.0, range: 2.0, start: 0.0 },
    ];
    // The last row is never read.
    let rows = [vec![2, 4, 1], vec![0, 0, 3], vec![255, 128, 0], vec![9, 9, 9]];
    leaf(DELTA_LERP, 4, &[20, 21, 22], 8, &infos, &rows)
}

#[test]
fn delta_lerp_8bit_decodes_frames_and_blends() {
    let object = lerp_8bit();
    let channel = read(&object, DELTA_LERP);
    assert_eq!(channel.frames(), 4);
    assert_eq!(channel.dofs(), [20, 21, 22]);
    assert_eq!(&sample(channel.as_ref(), 0.0)[20..23], [1.0, -2.0, 0.0]);
    assert_eq!(&sample(channel.as_ref(), 1.0)[20..23], [1.0, 0.0, 1.0]);
    assert_eq!(&sample(channel.as_ref(), 0.5)[20..23], [1.0, -1.0, 0.5], "half way between frames 0 and 1");
    assert_eq!(&sample(channel.as_ref(), 2.5)[20..23], [32.125, 32.0, 5.5], "half way between frames 2 and 3");
}

#[test]
fn delta_lerp_writes_only_its_dofs() {
    let object = lerp_8bit();
    let channel = read(&object, DELTA_LERP);
    let out = sample(channel.as_ref(), 1.0);
    assert!(out[..20].iter().chain(&out[23..]).all(|&slot| slot == UNTOUCHED));
}

#[test]
fn delta_lerp_clamps_the_frame() {
    let object = lerp_8bit();
    let channel = read(&object, DELTA_LERP);
    let last = [63.75, 64.0, 5.0];
    assert_eq!(&sample(channel.as_ref(), 3.0)[20..23], last);
    assert_eq!(&sample(channel.as_ref(), 10.0)[20..23], last, "beyond the last frame");
    assert_eq!(&sample(channel.as_ref(), -4.0)[20..23], [1.0, -2.0, 0.0], "before frame 0");
    assert_eq!(&sample(channel.as_ref(), f32::NAN)[20..23], [1.0, -2.0, 0.0], "a NaN time reads frame 0");
}

#[test]
fn dofs_beyond_the_buffer_are_skipped() {
    let object = lerp_8bit();
    let channel = read(&object, DELTA_LERP);
    let mut out = vec![UNTOUCHED; 21];
    channel.sample(1.0, &mut out);
    assert_eq!(out[20], 1.0);
    assert_eq!(out.len(), 21);
}

#[test]
fn delta_lerp_16bit_rows_are_two_bytes_per_dof() {
    let infos = [Info { min: 1.0, range: 0.5, start: 0.0 }, Info { min: 0.0, range: 0.25, start: 10.0 }];
    let rows = [vec![2, 65535], vec![0, 4], vec![1000, 1]];
    let object = leaf(DELTA_LERP, 3, &[4, 9], 16, &infos, &rows);
    let channel = read(&object, DELTA_LERP);
    assert_eq!(channel.dofs(), [4, 9]);
    assert_eq!(sample(channel.as_ref(), 0.0)[4], 0.0);
    assert_eq!(sample(channel.as_ref(), 0.0)[9], 10.0);
    assert_eq!(sample(channel.as_ref(), 1.0)[4], 2.0);
    assert_eq!(sample(channel.as_ref(), 1.0)[9], 16393.75);
    assert_eq!(sample(channel.as_ref(), 2.0)[4], 3.0);
    assert_eq!(sample(channel.as_ref(), 2.0)[9], 16394.75);
    assert_eq!(sample(channel.as_ref(), 1.5)[4], 2.5);
    assert_eq!(sample(channel.as_ref(), 1.5)[9], 16394.25);
}

#[test]
fn delta_lerp_4bit_packs_two_dofs_per_byte() {
    let infos = [
        Info { min: 0.0, range: 1.0, start: 0.0 },
        Info { min: 0.0, range: 2.0, start: 0.0 },
        Info { min: -1.0, range: 1.0, start: 5.0 },
    ];
    // Row 0 is [1, 15, 7]: bytes 0xF1 (dof 0 low nibble, dof 1 high nibble) and 0x07 (dof 2 low nibble).
    let object = leaf(DELTA_LERP, 2, &[12, 13, 14], 4, &infos, &[vec![1, 15, 7]]);
    let channel = read(&object, DELTA_LERP);
    assert_eq!(&sample(channel.as_ref(), 0.0)[12..15], [0.0, 0.0, 5.0]);
    assert_eq!(&sample(channel.as_ref(), 1.0)[12..15], [1.0, 30.0, 11.0]);
}

#[test]
fn a_single_frame_channel_holds_its_start_values() {
    let infos = [Info { min: 0.0, range: 1.0, start: 0.25 }];
    let object = leaf(DELTA_LERP, 1, &[3], 8, &infos, &[]);
    let channel = read(&object, DELTA_LERP);
    assert_eq!(channel.frames(), 1);
    assert_eq!(sample(channel.as_ref(), 0.7)[3], 0.25);
}

/// Two quaternions at dof indices 16 and 22, eight dofs at 8 bits. The first starts at (0, 0, 0, 1) and the second
/// at (0, 1, 0, 0).
fn quat_8bit() -> Object {
    let zero = Info { min: 0.0, range: 0.25, start: 0.0 };
    let one = Info { min: 0.0, range: 0.5, start: 1.0 };
    let half = Info { min: 0.0, range: 0.5, start: 0.0 };
    let infos = [zero, zero, zero, one, zero, one, zero, half];
    // One row: the first quaternion steps by (1, 0, 0, 0) and the second by (0, 1, 0, 1).
    leaf(DELTA_QUAT, 2, &[16, 22], 8, &infos, &[vec![4, 0, 0, 0, 0, 2, 0, 2]])
}

#[test]
fn delta_quat_writes_four_components_per_index() {
    let object = quat_8bit();
    let channel = read(&object, DELTA_QUAT);
    assert_eq!(channel.frames(), 2);
    assert_eq!(channel.dofs(), [16, 22]);
    assert_eq!(&sample(channel.as_ref(), 0.0)[16..20], [0.0, 0.0, 0.0, 1.0]);
    let frame1 = sample(channel.as_ref(), 1.0);
    assert_eq!(&frame1[16..20], [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(&frame1[22..26], [0.0, 2.0, 0.0, 1.0]);
}

#[test]
fn delta_quat_skips_components_past_the_buffer() {
    let object = quat_8bit();
    let channel = read(&object, DELTA_QUAT);
    let mut out = vec![UNTOUCHED; 24];
    channel.sample(1.0, &mut out);
    assert_eq!(&out[22..24], [0.0, 2.0]);
    assert_eq!(&out[20..22], [UNTOUCHED, UNTOUCHED]);
}

#[test]
fn delta_quat_is_not_renormalised() {
    let object = quat_8bit();
    let channel = read(&object, DELTA_QUAT);
    let mut out = vec![0.0; 32];
    channel.sample(0.5, &mut out);
    assert_close(&out[16..20], &[0.5, 0.0, 0.0, 1.0], 1e-6);
}

/// The DofInfo records of the airliner `_q` leaf, from `docs/specs/eagl-animation-codecs.md` section 14, example A.
const EXAMPLE_INFO: [[u8; 12]; 4] = [
    [0x80, 0x92, 0xB2, 0xBB, 0xBC, 0x6B, 0xBB, 0x38, 0xD9, 0xA0, 0x2C, 0xBE],
    [0xE8, 0x47, 0x28, 0xBC, 0x96, 0x2F, 0x9A, 0x38, 0x0D, 0xD8, 0x88, 0xB5],
    [0x98, 0xC8, 0x77, 0xBD, 0x7B, 0x89, 0x02, 0x3A, 0x69, 0x07, 0xC8, 0x36],
    [0xBB, 0x2B, 0xB9, 0xBD, 0x88, 0xCC, 0xB9, 0x39, 0x05, 0x56, 0x7C, 0x3F],
];

// The expected values are the spec's literals, kept verbatim; the extra digits are the spec's own.
#[test]
#[allow(clippy::excessive_precision)]
fn spec_example_a_decodes_the_airliner_quaternion() {
    // Header 04 00 08 00 (4 dofs, 8 bits), the four DofInfo records, then the first delta row (raw 61, 136, 141,
    // 255). Only frames 0 and 1 are in the example, so the channel has two frames.
    let mut block_bytes = vec![0x04, 0x00, 0x08, 0x00];
    for info in EXAMPLE_INFO {
        block_bytes.extend(info);
    }
    block_bytes.extend([0x3D, 0x88, 0x8D, 0xFF]);
    let object = leaf_with(DELTA_QUAT, 2, &[16], &block_bytes);
    let channel = read(&object, DELTA_QUAT);
    let frame0 = sample(channel.as_ref(), 0.0);
    assert_close(&frame0[16..20], &[-0.168582335, -1.01956687e-06, 5.96132713e-06, 0.985687554], 1e-7);
    let frame1 = sample(channel.as_ref(), 1.0);
    assert_close(&frame1[16..20], &[-0.168580406, -0.000273137704, 0.00972418195, 0.985639874], 1e-7);
}

#[test]
fn unsupported_codecs_are_named() {
    let object = quat_8bit();
    assert_eq!(read_error(&object, 20), Error::Unsupported("channel codec type 20".to_string()));
    assert!(matches!(read_error(&object, 17), Error::Unsupported(_)));
}

#[test]
fn a_leaf_with_the_wrong_type_is_invalid() {
    let object = quat_8bit();
    assert!(matches!(read_error(&object, DELTA_LERP), Error::Invalid(_)));
}

#[test]
fn a_leaf_without_frames_is_invalid() {
    let infos = [Info { min: 0.0, range: 1.0, start: 0.0 }];
    let object = leaf(DELTA_LERP, 0, &[3], 8, &infos, &[]);
    assert!(matches!(read_error(&object, DELTA_LERP), Error::Invalid(_)));
}

#[test]
fn a_quaternion_needs_four_floats_per_index() {
    let infos = [Info { min: 0.0, range: 1.0, start: 0.0 }; 3];
    let object = leaf(DELTA_QUAT, 2, &[0], 8, &infos, &[vec![0, 0, 0]]);
    assert!(matches!(read_error(&object, DELTA_QUAT), Error::Invalid(_)));
}

#[test]
fn an_unknown_row_width_is_unsupported() {
    let infos = [Info { min: 0.0, range: 1.0, start: 0.0 }];
    let object = leaf(DELTA_LERP, 2, &[0], 12, &infos, &[vec![0]]);
    assert!(matches!(read_error(&object, DELTA_LERP), Error::Unsupported(_)));
}

#[test]
fn a_block_without_dofs_is_invalid() {
    let object = leaf_with(DELTA_LERP, 2, &[0], &[0, 0, 8, 0]);
    assert!(matches!(read_error(&object, DELTA_LERP), Error::Invalid(_)));
}
