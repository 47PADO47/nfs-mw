//! The binned layout that DeltaF1 (type 21) and DeltaF3 (type 20) share. The header, the DofInfo records, the bins of
//! 2^power frames and the constant bones are in `docs/specs/eagl-animation-codecs.md` sections 8 to 10.
//!
//! The two codecs differ only in the number of components per bone: one scalar for DeltaF1, three for DeltaF3. The
//! whole track is decoded once, when the leaf is read, into frame-major values (see `rows.rs`).

use super::rows::DeltaTrack;
use crate::{Error, Object, Result, bytes};

/// The offset of the dof index list pointer. The list holds one u16 per animated bone.
const DOF_LIST: usize = 0x04;
/// The offset of the key-time table pointer. Every install channel has it null.
const KEY_TIMES: usize = 0x08;
const FRAMES: usize = 0x0C;
const BONES: usize = 0x0E;
const POWER: usize = 0x10;
const CONSTANTS: usize = 0x11;
/// The header length, which is also the offset of the first DofInfo record.
const HEADER_LEN: usize = 0x14;
const WHAT: &str = "binned channel";
/// The decomp's constants 1/65535 and 1/255, written as the decomp writes them (spec section 8).
const INV_65535: f32 = 1.5259022e-5;
const INV_255: f32 = 0.003921569;
const F32_LEN: usize = 4;
const U16_LEN: usize = 2;

/// The decode constants of one component of one bone, from its DofInfo record (spec section 8).
#[derive(Debug, Clone, Copy)]
struct Component {
    /// The physical minimum.
    base: f32,
    /// The physical value of one step of a stored key: the physical range over 65535.
    scale: f32,
    /// The delta of a raw value of zero.
    delta_min: f32,
    /// The delta of one raw step.
    delta_range: f32,
}

impl Component {
    /// The constants of a component with physical minimum `base` and physical range `range`, and the quantised
    /// minimum and range of its DofInfo record.
    fn new(base: f32, range: f32, quant_min: u16, quant_range: u16) -> Self {
        Self {
            base,
            scale: range * INV_65535,
            delta_min: f32::from(quant_min) * range * (2.0 * INV_65535) - range,
            delta_range: f32::from(quant_range) * range * (2.0 * INV_65535) * INV_255,
        }
    }

    /// The physical value of a stored key.
    fn key(&self, raw: u16) -> f32 {
        self.base + f32::from(raw) * self.scale
    }

    /// The decoded delta of a stored raw delta.
    fn delta(&self, raw: u8) -> f32 {
        f32::from(raw) * self.delta_range + self.delta_min
    }
}

/// Reads the DeltaF1 or DeltaF3 leaf at `offset` in `object`'s `.data`. The leaf has codec `type_id` and
/// `components` floats per bone.
pub(super) fn read(object: &Object, offset: usize, type_id: u16, components: usize) -> Result<DeltaTrack> {
    let stored = object.u16_at(offset)?;
    if stored != type_id {
        return Err(Error::Invalid(format!("channel at {offset:#x} has type {stored}, expected {type_id}")));
    }
    if object.ptr_at(offset + KEY_TIMES)?.is_some() {
        return Err(Error::Unsupported(format!("key-time table in the channel at {offset:#x}")));
    }
    let frames = usize::from(object.u16_at(offset + FRAMES)?);
    if frames == 0 {
        return Err(Error::Invalid(format!("channel at {offset:#x} has no frames")));
    }
    let bones = usize::from(object.u16_at(offset + BONES)?);
    let power = object.u8_at(offset + POWER)?;
    let constants = usize::from(object.u8_at(offset + CONSTANTS)?);
    let bin_frames = match 1usize.checked_shl(u32::from(power)) {
        Some(bin_frames) => bin_frames,
        None => return Err(Error::Invalid(format!("bin power {power} at {offset:#x}"))),
    };

    let mut dofs = read_dof_list(object, offset, bones)?;
    let infos = read_components(object, offset + HEADER_LEN, bones, components)?;

    let row_len = bones * components;
    let bins_at = offset + HEADER_LEN + bones * record_len(components);
    let bins_size = bins_len(row_len, frames, bin_frames)?;
    let bins = bytes::slice(&object.data, bins_at, bins_size, WHAT)?;
    let (constant_dofs, constant_values) = read_constants(object, bins_at + bins_size, constants, components)?;

    let total_row = (bones + constants) * components;
    let mut values = vec![0.0f32; frames * total_row];
    decode_bins(bins, frames, bin_frames, &infos, row_len, total_row, &mut values)?;
    for frame in 0..frames {
        values[frame * total_row + row_len..][..constant_values.len()].copy_from_slice(&constant_values);
    }
    dofs.extend(constant_dofs);
    Ok(DeltaTrack::from_values(frames as u32, dofs, components, values))
}

/// The bytes of one DofInfo record of a bone: a physical minimum and range per component, then the quantised
/// minimum and range per component.
fn record_len(components: usize) -> usize {
    2 * F32_LEN * components + 2 * U16_LEN * components
}

/// The dof index of each animated bone, or none when the channel has no animated bone.
fn read_dof_list(object: &Object, offset: usize, bones: usize) -> Result<Vec<u16>> {
    if bones == 0 {
        return Ok(Vec::new());
    }
    let list = object
        .ptr_at(offset + DOF_LIST)?
        .ok_or_else(|| Error::Invalid(format!("channel at {offset:#x} has {bones} bones but no dof list")))?;
    (0..bones).map(|bone| object.u16_at(list as usize + U16_LEN * bone)).collect()
}

/// The decode constants of every component of every bone, bone-major.
fn read_components(object: &Object, at: usize, bones: usize, components: usize) -> Result<Vec<Component>> {
    let mut infos = Vec::with_capacity(bones * components);
    for bone in 0..bones {
        let record = at + bone * record_len(components);
        let range_at = record + F32_LEN * components;
        let quant_min_at = range_at + F32_LEN * components;
        let quant_range_at = quant_min_at + U16_LEN * components;
        for c in 0..components {
            infos.push(Component::new(
                object.f32_at(record + F32_LEN * c)?,
                object.f32_at(range_at + F32_LEN * c)?,
                object.u16_at(quant_min_at + U16_LEN * c)?,
                object.u16_at(quant_range_at + U16_LEN * c)?,
            ));
        }
    }
    Ok(infos)
}

/// The bytes of one bin of `in_bin` frames. Its physical keys take `2 * row_len` bytes and each of its `in_bin - 1`
/// delta rows takes `row_len`, so the bin is `row_len * (in_bin + 1)` bytes, rounded up to an even size.
fn bin_len(row_len: usize, in_bin: usize) -> Result<usize> {
    let len = row_len.checked_mul(in_bin + 1).ok_or_else(|| Error::Invalid(format!("{WHAT} too large")))?;
    Ok(len + (len & 1))
}

/// The bytes of all the bins of a track of `frames` frames cut into bins of `bin_frames` frames. The last bin may be
/// partial.
fn bins_len(row_len: usize, frames: usize, bin_frames: usize) -> Result<usize> {
    (0..frames).step_by(bin_frames).try_fold(0usize, |total, first| {
        let size = bin_len(row_len, (frames - first).min(bin_frames))?;
        total.checked_add(size).ok_or_else(|| Error::Invalid(format!("{WHAT} too large")))
    })
}

/// Decodes the bins, in order, into `values`. Bin `k` holds the frames from `k * bin_frames` on.
fn decode_bins(
    bins: &[u8],
    frames: usize,
    bin_frames: usize,
    infos: &[Component],
    row_len: usize,
    total_row: usize,
    values: &mut [f32],
) -> Result<()> {
    let mut at = 0;
    for first in (0..frames).step_by(bin_frames) {
        let in_bin = (frames - first).min(bin_frames);
        let size = bin_len(row_len, in_bin)?;
        let bin = bins.get(at..at + size).ok_or(Error::Truncated { what: WHAT, offset: at })?;
        decode_bin(bin, first, in_bin, infos, row_len, total_row, values);
        at += size;
    }
    Ok(())
}

/// Decodes one bin of `in_bin` frames, starting at frame `first`. The bin holds the physical key of every bone
/// component, then `in_bin - 1` delta rows. A frame's value is the key plus the decoded deltas of every row before it
/// in the bin (the cumulative rule, spec section 8 step 3).
fn decode_bin(
    bin: &[u8],
    first: usize,
    in_bin: usize,
    infos: &[Component],
    row_len: usize,
    total_row: usize,
    values: &mut [f32],
) {
    let key_len = 2 * row_len;
    let (keys, _) = bin[..key_len].as_chunks::<U16_LEN>();
    let mut acc: Vec<f32> = keys.iter().zip(infos).map(|(key, info)| info.key(u16::from_le_bytes(*key))).collect();
    let rows = &bin[key_len..];
    for pos in 0..in_bin {
        let frame = first + pos;
        values[frame * total_row..][..row_len].copy_from_slice(&acc);
        if pos + 1 == in_bin {
            return;
        }
        let row = &rows[pos * row_len..][..row_len];
        for ((value, info), &raw) in acc.iter_mut().zip(infos).zip(row) {
            *value += info.delta(raw);
        }
    }
}

/// The constant bones: a u16 dof index per bone, then `components` f32 per bone, the f32 run aligned to 4 bytes.
/// Returns the dof indices and the values, bone-major.
fn read_constants(object: &Object, at: usize, count: usize, components: usize) -> Result<(Vec<u16>, Vec<f32>)> {
    if count == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let dof_bytes = bytes::slice(&object.data, at, count * U16_LEN, WHAT)?;
    let (dof_words, _) = dof_bytes.as_chunks::<U16_LEN>();
    let dofs = dof_words.iter().map(|word| u16::from_le_bytes(*word)).collect();
    let values_at = align4(at + count * U16_LEN);
    let value_bytes = bytes::slice(&object.data, values_at, count * components * F32_LEN, WHAT)?;
    let (value_words, _) = value_bytes.as_chunks::<F32_LEN>();
    let values = value_words.iter().map(|word| f32::from_le_bytes(*word)).collect();
    Ok((dofs, values))
}

/// The offset rounded up to a multiple of 4.
fn align4(at: usize) -> usize {
    (at + 3) & !3
}
