//! Tests for the binned codecs, DeltaF1 (type 21) and DeltaF3 (type 20), spec `docs/specs/eagl-animation-codecs.md`
//! sections 8 to 10. Each leaf is built in memory. The bins are encoded by the spec's inverse rule (`encode`), so the
//! decode is checked against the values it was asked to store. No install data is read.

use super::codec::{UNTOUCHED, assert_close, read, read_error};
use super::elf_builder::Builder;
use crate::codec::{Channel, DELTA_F1, DELTA_F3};
use crate::{Error, Object};

/// The header length: the offset of the first DofInfo record.
const HEADER_LEN: usize = 0x14;

/// One component of a bone: its physical minimum and range, and its quantised delta fields.
#[derive(Clone, Copy)]
struct Comp {
    base: f32,
    range: f32,
    qmin: u16,
    qrange: u16,
}

/// A component whose deltas span `[delta_min, delta_min + 255 * delta_range]`. The quantised fields are the inverse
/// of the spec's delta formulas (section 8), rounded to the nearest integer.
fn comp(base: f32, range: f32, delta_min: f64, delta_range: f64) -> Comp {
    let p = f64::from(range);
    let qmin = ((delta_min + p) * 65535.0 / (2.0 * p)).round().clamp(0.0, 65535.0);
    let qrange = (delta_range * 255.0 * 65535.0 / (2.0 * p)).round().clamp(0.0, 65535.0);
    Comp { base, range, qmin: qmin as u16, qrange: qrange as u16 }
}

/// The decode constants of spec section 8, in f64: the physical step of one key, the delta minimum and the delta
/// step.
fn scales(c: Comp) -> (f64, f64, f64) {
    let p = f64::from(c.range);
    (p / 65535.0, f64::from(c.qmin) * p * 2.0 / 65535.0 - p, f64::from(c.qrange) * p * 2.0 / 65535.0 / 255.0)
}

/// The DofInfo record of one bone: the physical minima, then the ranges, then the quantised minima and ranges, one of
/// each per component.
fn record(comps: &[Comp]) -> Vec<u8> {
    let mut out = Vec::new();
    for c in comps {
        out.extend(c.base.to_le_bytes());
    }
    for c in comps {
        out.extend(c.range.to_le_bytes());
    }
    for c in comps {
        out.extend(c.qmin.to_le_bytes());
    }
    for c in comps {
        out.extend(c.qrange.to_le_bytes());
    }
    out
}

/// A sinusoid around `center`, with `amplitude` and a step of at most `0.3 * amplitude` per frame.
fn wave(center: f32, amplitude: f32, phase: f32, frames: usize) -> Vec<f32> {
    (0..frames).map(|f| center + amplitude * (0.3 * f as f32 + phase).sin()).collect()
}

/// An animated bone to encode: its dof index, its components and the physical target of each component over the
/// frames (`targets[component][frame]`).
struct Bone {
    dof: u16,
    comps: Vec<Comp>,
    targets: Vec<Vec<f32>>,
}

/// The stored bytes of a track, and the values the decoder must return: `decoded[bone][component][frame]`.
struct Encoded {
    records: Vec<u8>,
    bins: Vec<u8>,
    decoded: Vec<Vec<Vec<f64>>>,
}

/// Encodes the bones into bins of `2^power` frames by the spec's inverse rule. A bin starts with the key that rounds
/// the first target. Each later frame adds the delta row that brings the running value closest to its target, so the
/// value of a frame is the key plus the deltas before it in its bin (the cumulative rule).
fn encode(bones: &[Bone], frames: usize, power: u8) -> Encoded {
    let bin_frames = 1usize << power;
    let flat: Vec<(usize, usize)> =
        bones.iter().enumerate().flat_map(|(i, bone)| (0..bone.comps.len()).map(move |c| (i, c))).collect();
    let consts: Vec<(f64, f64, f64)> = flat.iter().map(|&(i, c)| scales(bones[i].comps[c])).collect();
    let mut encoded = Encoded {
        records: bones.iter().flat_map(|bone| record(&bone.comps)).collect(),
        bins: Vec::new(),
        decoded: bones.iter().map(|bone| vec![vec![0.0; frames]; bone.comps.len()]).collect(),
    };
    for first in (0..frames).step_by(bin_frames) {
        let in_bin = (frames - first).min(bin_frames);
        let mut bin = Vec::new();
        let mut acc = Vec::new();
        for (k, &(i, c)) in flat.iter().enumerate() {
            let (scale, _, _) = consts[k];
            let base = f64::from(bones[i].comps[c].base);
            let target = f64::from(bones[i].targets[c][first]);
            let raw = ((target - base) / scale).round().clamp(0.0, 65535.0) as u16;
            bin.extend(raw.to_le_bytes());
            acc.push(base + f64::from(raw) * scale);
        }
        for pos in 0..in_bin {
            let frame = first + pos;
            for (k, &(i, c)) in flat.iter().enumerate() {
                encoded.decoded[i][c][frame] = acc[k];
            }
            if pos + 1 == in_bin {
                break;
            }
            for (k, &(i, c)) in flat.iter().enumerate() {
                let (_, delta_min, delta_range) = consts[k];
                let target = f64::from(bones[i].targets[c][frame + 1]);
                let raw = ((target - acc[k] - delta_min) / delta_range).round().clamp(0.0, 255.0) as u8;
                acc[k] += delta_min + f64::from(raw) * delta_range;
                bin.push(raw);
            }
        }
        if bin.len() % 2 == 1 {
            bin.push(0);
        }
        encoded.bins.extend(bin);
    }
    encoded
}

/// A DeltaF1 or DeltaF3 leaf at offset 0 of `.data`.
struct Leaf {
    type_id: u16,
    frames: u16,
    power: u8,
    /// The dof index of each animated bone. Empty means no animated bone, and the list pointer is null.
    dofs: Vec<u16>,
    records: Vec<u8>,
    bins: Vec<u8>,
    constant_dofs: Vec<u16>,
    /// The constant values, bone-major, one f32 per component.
    constant_values: Vec<f32>,
    /// Whether the key-time table pointer is set. No install channel has one.
    key_times: bool,
}

impl Leaf {
    /// The leaf as an object. The header, the records, the bins, the constant bones (aligned to 4 bytes), then the
    /// dof list and the key-time table, each pointer relocated.
    fn object(&self) -> Object {
        let mut data = vec![0u8; HEADER_LEN];
        data[0..2].copy_from_slice(&self.type_id.to_le_bytes());
        data[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        data[0x0C..0x0E].copy_from_slice(&self.frames.to_le_bytes());
        data[0x0E..0x10].copy_from_slice(&(self.dofs.len() as u16).to_le_bytes());
        data[0x10] = self.power;
        data[0x11] = self.constant_dofs.len() as u8;
        data.extend(&self.records);
        data.extend(&self.bins);
        data.extend(self.constant_dofs.iter().flat_map(|dof| dof.to_le_bytes()));
        pad4(&mut data);
        for value in &self.constant_values {
            data.extend(value.to_le_bytes());
        }
        let mut relocs = Vec::new();
        if !self.dofs.is_empty() {
            pad4(&mut data);
            let list = data.len() as u32;
            data.extend(self.dofs.iter().flat_map(|dof| dof.to_le_bytes()));
            data[4..8].copy_from_slice(&list.to_le_bytes());
            relocs.push(4);
        }
        if self.key_times {
            pad4(&mut data);
            let table = data.len() as u32;
            data.extend([0u8; 4]);
            data[8..12].copy_from_slice(&table.to_le_bytes());
            relocs.push(8);
        }
        let mut builder = Builder::new(&data);
        for reloc in relocs {
            builder = builder.reloc(reloc);
        }
        Object::read(&builder.bare()).expect("synthetic binned leaf reads")
    }
}

fn pad4(data: &mut Vec<u8>) {
    while data.len() % 4 != 0 {
        data.push(0);
    }
}

/// A leaf of `bones` (each with its dof) and no constant bones, encoded from the targets.
fn encoded_leaf(type_id: u16, frames: usize, power: u8, bones: &[Bone]) -> (Leaf, Encoded) {
    let encoded = encode(bones, frames, power);
    let leaf = Leaf {
        type_id,
        frames: frames as u16,
        power,
        dofs: bones.iter().map(|bone| bone.dof).collect(),
        records: encoded.records.clone(),
        bins: encoded.bins.clone(),
        constant_dofs: Vec::new(),
        constant_values: Vec::new(),
        key_times: false,
    };
    (leaf, encoded)
}

/// The slots of a sample of a channel that writes up to dof 63.
fn sample_wide(channel: &dyn Channel, frame: f32) -> Vec<f32> {
    let mut out = vec![UNTOUCHED; 64];
    channel.sample(frame, &mut out);
    out
}

/// Checks every frame of every animated bone: the sample equals what the encoder decoded, and that is within half a
/// quantisation step of the target.
fn assert_track(channel: &dyn Channel, bones: &[Bone], encoded: &Encoded, frames: usize) {
    for frame in 0..frames {
        let out = sample_wide(channel, frame as f32);
        for (i, bone) in bones.iter().enumerate() {
            for (c, comp) in bone.comps.iter().enumerate() {
                let slot = usize::from(bone.dof) + c;
                let decoded = encoded.decoded[i][c][frame];
                let got = f64::from(out[slot]);
                assert!((got - decoded).abs() < 1e-4, "bone {i} comp {c} frame {frame}: {got} is not {decoded}");
                let (scale, _, delta_range) = scales(*comp);
                let tolerance = 0.5 * scale.max(delta_range) + 1e-6;
                let target = f64::from(bone.targets[c][frame]);
                assert!((got - target).abs() <= tolerance, "bone {i} comp {c} frame {frame}: {got} vs target {target}");
            }
        }
    }
}

#[test]
fn delta_f1_decodes_several_bins_and_a_partial_last_bin() {
    // Bins of 4 frames: frames 0 to 3, 4 to 7, and 8 to 9, a partial bin of two frames.
    let frames = 10;
    let bones = [
        Bone { dof: 5, comps: vec![comp(-1.0, 2.0, -0.05, 0.0004)], targets: vec![wave(0.0, 0.1, 0.0, frames)] },
        Bone { dof: 9, comps: vec![comp(0.5, 4.0, -0.1, 0.0008)], targets: vec![wave(2.5, 0.1, 1.0, frames)] },
    ];
    let (leaf, encoded) = encoded_leaf(DELTA_F1, frames, 2, &bones);
    let object = leaf.object();
    let channel = read(&object, DELTA_F1);
    assert_eq!(channel.frames(), 10);
    assert_eq!(channel.dofs(), [5, 9]);
    assert_track(channel.as_ref(), &bones, &encoded, frames);
}

#[test]
fn delta_f1_writes_one_float_per_bone() {
    let frames = 4;
    let bones =
        [Bone { dof: 3, comps: vec![comp(0.0, 2.0, -0.05, 0.0004)], targets: vec![wave(1.0, 0.1, 0.5, frames)] }];
    let (leaf, _) = encoded_leaf(DELTA_F1, frames, 1, &bones);
    let object = leaf.object();
    let channel = read(&object, DELTA_F1);
    let out = sample_wide(channel.as_ref(), 2.0);
    assert!(out[3].is_finite());
    assert!(out[..3].iter().chain(&out[4..]).all(|&slot| slot == UNTOUCHED));
}

#[test]
fn delta_f3_decodes_three_components_per_bone() {
    // Bins of 8 frames: frames 0 to 7, 8 to 15, and 16 to 18, a partial bin of three frames.
    let frames = 19;
    let bones = [
        Bone {
            dof: 20,
            comps: vec![comp(-1.0, 2.0, -0.05, 0.0004), comp(0.0, 4.0, -0.1, 0.0008), comp(5.0, 2.0, -0.05, 0.0004)],
            targets: vec![wave(0.0, 0.1, 0.0, frames), wave(2.0, 0.1, 0.7, frames), wave(5.5, 0.1, 1.4, frames)],
        },
        Bone {
            dof: 40,
            comps: vec![comp(-2.0, 4.0, -0.1, 0.0008), comp(-1.0, 2.0, -0.05, 0.0004), comp(1.0, 4.0, -0.1, 0.0008)],
            targets: vec![wave(0.0, 0.1, 2.0, frames), wave(0.0, 0.1, 2.5, frames), wave(3.0, 0.1, 3.0, frames)],
        },
    ];
    let (leaf, encoded) = encoded_leaf(DELTA_F3, frames, 3, &bones);
    let object = leaf.object();
    let channel = read(&object, DELTA_F3);
    assert_eq!(channel.frames(), 19);
    assert_eq!(channel.dofs(), [20, 40]);
    assert_track(channel.as_ref(), &bones, &encoded, frames);
    let out = sample_wide(channel.as_ref(), 3.0);
    assert!(out[23] == UNTOUCHED && out[24..40].iter().all(|&slot| slot == UNTOUCHED));
}

#[test]
fn constant_bones_hold_their_values_in_every_frame() {
    // The airliner `_s` shape: no animated bone, one constant bone at dof 12 whose three floats are (1, 1, 1).
    let object = Leaf {
        type_id: DELTA_F3,
        frames: 5,
        power: 2,
        dofs: Vec::new(),
        records: Vec::new(),
        bins: Vec::new(),
        constant_dofs: vec![12],
        constant_values: vec![1.0, 1.0, 1.0],
        key_times: false,
    }
    .object();
    let channel = read(&object, DELTA_F3);
    assert_eq!(channel.frames(), 5);
    assert_eq!(channel.dofs(), [12]);
    for frame in 0..5 {
        assert_eq!(&sample_wide(channel.as_ref(), frame as f32)[12..15], [1.0, 1.0, 1.0]);
    }
}

#[test]
fn an_odd_constant_list_is_padded_before_its_values() {
    // One constant dof is two bytes, so its f32 must start two bytes after the bins to be aligned.
    let frames = 6;
    let bones =
        [Bone { dof: 3, comps: vec![comp(0.0, 2.0, -0.05, 0.0004)], targets: vec![wave(1.0, 0.1, 0.0, frames)] }];
    let (mut leaf, encoded) = encoded_leaf(DELTA_F1, frames, 1, &bones);
    leaf.constant_dofs = vec![7];
    leaf.constant_values = vec![0.25];
    let object = leaf.object();
    let channel = read(&object, DELTA_F1);
    assert_eq!(channel.dofs(), [3, 7]);
    assert_track(channel.as_ref(), &bones, &encoded, frames);
    for frame in 0..frames {
        assert_eq!(sample_wide(channel.as_ref(), frame as f32)[7], 0.25);
    }
}

#[test]
fn frames_are_clamped_and_fractional_frames_blend() {
    let frames = 10;
    let bones =
        [Bone { dof: 5, comps: vec![comp(-1.0, 2.0, -0.05, 0.0004)], targets: vec![wave(0.0, 0.1, 0.0, frames)] }];
    let (leaf, _) = encoded_leaf(DELTA_F1, frames, 2, &bones);
    let object = leaf.object();
    let channel = read(&object, DELTA_F1);
    let at = |frame: f32| sample_wide(channel.as_ref(), frame)[5];
    assert_eq!(at(-3.0), at(0.0), "before frame 0");
    assert_eq!(at(100.0), at(9.0), "beyond the last frame");
    assert!((at(1.5) - (at(1.0) + at(2.0)) / 2.0).abs() < 1e-6, "half way between frames 1 and 2");
    assert!((at(3.5) - (at(3.0) + at(4.0)) / 2.0).abs() < 1e-6, "across the bin boundary at frame 4");
}

#[test]
fn a_buffer_shorter_than_the_dof_skips_the_slot() {
    let frames = 4;
    let bones =
        [Bone { dof: 5, comps: vec![comp(-1.0, 2.0, -0.05, 0.0004)], targets: vec![wave(0.0, 0.1, 0.0, frames)] }];
    let (leaf, _) = encoded_leaf(DELTA_F1, frames, 2, &bones);
    let object = leaf.object();
    let channel = read(&object, DELTA_F1);
    let mut out = vec![UNTOUCHED; 5];
    channel.sample(1.0, &mut out);
    assert!(out.iter().all(|&slot| slot == UNTOUCHED), "dof 5 is past a 5-float buffer");
}

/// The DofInfo and the first bin of the airliner `_t` channel, from `docs/specs/eagl-animation-codecs.md` section 14,
/// example B. Bin 0's key is raw (258, 36170, 0) and its first delta row is raw (124, 96, 151).
#[test]
#[allow(clippy::excessive_precision)]
fn spec_example_b_decodes_the_airliner_translation() {
    let comps = [
        Comp { base: -18.6462765, range: 4643.01416, qmin: 29945, qrange: 5520 },
        Comp { base: -2003.93237, range: 3645.07251, qmin: 30526, qrange: 4887 },
        Comp { base: 12.8326864, range: 822.481384, qmin: 29389, qrange: 6034 },
    ];
    let mut block = record(&comps);
    // The key bytes 02 01 4A 8D 00 00, the row bytes 7C 60 97, and one byte of padding to an even bin.
    let bin = [0x02, 0x01, 0x4A, 0x8D, 0x00, 0x00, 124, 96, 151, 0];
    block.extend(bin);
    let object = Leaf {
        type_id: DELTA_F3,
        frames: 2,
        power: 3,
        dofs: vec![20],
        records: block[..36].to_vec(),
        bins: block[36..].to_vec(),
        constant_dofs: Vec::new(),
        constant_values: Vec::new(),
        key_times: false,
    }
    .object();
    let channel = read(&object, DELTA_F3);
    assert_close(&sample_wide(channel.as_ref(), 0.0)[20..23], &[-0.367_529_95, 7.851_752_7, 12.832_686], 1e-3);
    assert_close(&sample_wide(channel.as_ref(), 1.0)[20..23], &[-19.959_046, -36.832_24, 17.716_608], 1e-3);
}

#[test]
fn a_leaf_with_the_wrong_type_is_invalid() {
    let frames = 2;
    let bones = [Bone { dof: 1, comps: vec![comp(0.0, 2.0, 0.0, 0.0004)], targets: vec![wave(1.0, 0.1, 0.0, frames)] }];
    let (leaf, _) = encoded_leaf(DELTA_F1, frames, 1, &bones);
    let object = leaf.object();
    assert!(matches!(read_error(&object, DELTA_F3), Error::Invalid(_)));
}

#[test]
fn a_leaf_without_frames_is_invalid() {
    let object = Leaf {
        type_id: DELTA_F1,
        frames: 0,
        power: 1,
        dofs: Vec::new(),
        records: Vec::new(),
        bins: Vec::new(),
        constant_dofs: vec![0],
        constant_values: vec![0.0],
        key_times: false,
    }
    .object();
    assert!(matches!(read_error(&object, DELTA_F1), Error::Invalid(_)));
}

#[test]
fn a_key_time_table_is_unsupported() {
    let frames = 2;
    let bones = [Bone { dof: 1, comps: vec![comp(0.0, 2.0, 0.0, 0.0004)], targets: vec![wave(1.0, 0.1, 0.0, frames)] }];
    let (mut leaf, _) = encoded_leaf(DELTA_F1, frames, 1, &bones);
    leaf.key_times = true;
    let object = leaf.object();
    assert!(matches!(read_error(&object, DELTA_F1), Error::Unsupported(_)));
}

#[test]
fn bins_that_end_early_are_truncated() {
    let frames = 10;
    let bones = [Bone { dof: 1, comps: vec![comp(0.0, 2.0, 0.0, 0.0004)], targets: vec![wave(1.0, 0.1, 0.0, frames)] }];
    let (mut leaf, _) = encoded_leaf(DELTA_F1, frames, 2, &bones);
    leaf.bins.truncate(4);
    let object = leaf.object();
    assert!(matches!(read_error(&object, DELTA_F1), Error::Truncated { .. }));
}

#[test]
fn a_bin_power_too_large_for_the_machine_is_invalid() {
    let frames = 2;
    let bones = [Bone { dof: 1, comps: vec![comp(0.0, 2.0, 0.0, 0.0004)], targets: vec![wave(1.0, 0.1, 0.0, frames)] }];
    let (mut leaf, _) = encoded_leaf(DELTA_F1, frames, 1, &bones);
    leaf.power = 64;
    let object = leaf.object();
    assert!(matches!(read_error(&object, DELTA_F1), Error::Invalid(_)));
}
