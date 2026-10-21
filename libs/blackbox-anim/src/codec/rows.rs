//! The DeltaChan header and the DeltaCompressedData block that the delta codecs (types 10 and 11) share.
//!
//! The byte layout and the decode rule are in `docs/specs/eagl-animation-codecs.md`, sections 4 and 5. The whole
//! track is decoded once, when the leaf is read, into frame-major values. A sample is then a linear blend of two
//! frames.

use super::Channel;
use crate::{Error, Object, Result, bytes};

/// The block header: numDofs (u16), quantisation bits (u8) and one pad byte.
const BLOCK_HEADER_LEN: usize = 4;
/// One DofInfo record: the minimum, the range and the start value, three f32.
const DOF_INFO_LEN: usize = 12;
/// The offset of the inline dof index list in a DeltaChan header.
const DOF_LIST_AT: usize = 0x0A;
const DATA_OFFSET: usize = 4;
const FRAMES_OFFSET: usize = 8;
const WHAT: &str = "delta channel";

/// The per-dof constants of a delta block.
#[derive(Debug, Clone, Copy)]
struct DofInfo {
    min: f32,
    range: f32,
    start: f32,
}

/// A DeltaChan leaf with its track decoded to dense values.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DeltaTrack {
    frames: u32,
    /// The inline dof index list. Each index names the first of `width` consecutive floats.
    dofs: Vec<u16>,
    /// The floats per dof index: 1 for a scalar channel, 4 for a quaternion channel.
    width: usize,
    /// `frames` rows of `dofs.len() * width` values, frame-major.
    values: Vec<f32>,
}

impl DeltaTrack {
    /// Reads the DeltaChan leaf at `offset`. The leaf must have codec `type_id`, and its data block holds
    /// `width` floats per dof index.
    pub(super) fn read(object: &Object, offset: usize, type_id: u16, width: usize) -> Result<Self> {
        let stored = object.u16_at(offset)?;
        if stored != type_id {
            return Err(Error::Invalid(format!("channel at {offset:#x} has type {stored}, expected {type_id}")));
        }
        let block = object
            .ptr_at(offset + DATA_OFFSET)?
            .ok_or_else(|| Error::Invalid(format!("channel at {offset:#x} has no data block")))?;
        let frames = u32::from(object.u16_at(offset + FRAMES_OFFSET)?);
        if frames == 0 {
            return Err(Error::Invalid(format!("channel at {offset:#x} has no frames")));
        }
        let (num_dofs, values) = decode_block(object, block as usize, frames)?;
        if num_dofs % width != 0 {
            return Err(Error::Invalid(format!(
                "channel at {offset:#x} has {num_dofs} dofs, not a multiple of {width}"
            )));
        }
        let dofs =
            (0..num_dofs / width).map(|i| object.u16_at(offset + DOF_LIST_AT + 2 * i)).collect::<Result<Vec<u16>>>()?;
        Ok(Self { frames, dofs, width, values })
    }

    /// The number of floats one frame holds for this channel.
    fn row_len(&self) -> usize {
        self.dofs.len() * self.width
    }
}

impl Channel for DeltaTrack {
    fn frames(&self) -> u32 {
        self.frames
    }

    fn dofs(&self) -> &[u16] {
        &self.dofs
    }

    fn sample(&self, frame: f32, out: &mut [f32]) {
        // A channel has at least one frame (checked on read), so `last` is a valid index. `max` and `min` turn
        // a NaN time into frame 0.
        let last = self.frames - 1;
        let time = frame.max(0.0).min(last as f32);
        let floor = time as usize;
        let fraction = time - floor as f32;
        // At the last frame the fraction is zero, so the blend returns the last frame exactly.
        let next = (floor + 1).min(last as usize);
        let row_len = self.row_len();
        let here = &self.values[floor * row_len..][..row_len];
        let there = &self.values[next * row_len..][..row_len];
        for (j, &dof) in self.dofs.iter().enumerate() {
            for c in 0..self.width {
                let k = j * self.width + c;
                let value = here[k] + (there[k] - here[k]) * fraction;
                if let Some(slot) = out.get_mut(usize::from(dof) + c) {
                    *slot = value;
                }
            }
        }
    }
}

/// Decodes the DeltaCompressedData block at `block` for a channel of `frames` frames. Returns the dof count and
/// the frame-major values. Row `f` is the delta from frame `f` to frame `f + 1`, so the last row is not read.
fn decode_block(object: &Object, block: usize, frames: u32) -> Result<(usize, Vec<f32>)> {
    let num_dofs = usize::from(object.u16_at(block)?);
    let bits = object.u8_at(block + 2)?;
    let stride = row_stride(bits, num_dofs)?;
    let infos = dof_infos(object, block + BLOCK_HEADER_LEN, num_dofs)?;

    let steps = (frames as usize).saturating_sub(1);
    let rows_at = block + BLOCK_HEADER_LEN + num_dofs * DOF_INFO_LEN;
    let rows_len = steps.checked_mul(stride).ok_or_else(|| Error::Invalid(format!("{WHAT} too large")))?;
    let delta_bytes = bytes::slice(&object.data, rows_at, rows_len, WHAT)?;

    let mut values = vec![0.0f32; frames as usize * num_dofs];
    for (i, info) in infos.iter().enumerate() {
        values[i] = info.start;
    }
    for (step, row) in delta_bytes.chunks_exact(stride).enumerate() {
        let base = step * num_dofs;
        for (i, info) in infos.iter().enumerate() {
            let delta = raw(row, bits, i) as f32 * info.range + info.min;
            values[base + num_dofs + i] = values[base + i] + delta;
        }
    }
    Ok((num_dofs, values))
}

/// The bytes in one delta row: one byte per dof at 8 bits, two at 16 bits, and half a byte at 4 bits.
fn row_stride(bits: u8, num_dofs: usize) -> Result<usize> {
    if num_dofs == 0 {
        return Err(Error::Invalid("delta block with no dofs".to_string()));
    }
    match bits {
        8 => Ok(num_dofs),
        16 => Ok(2 * num_dofs),
        4 => Ok(num_dofs.div_ceil(2)),
        other => Err(Error::Unsupported(format!("{other}-bit delta rows"))),
    }
}

/// The unscaled delta of dof `dof` in one row. At 4 bits the even dof is the low nibble of its byte and the odd
/// dof is the high nibble.
fn raw(row: &[u8], bits: u8, dof: usize) -> u32 {
    match bits {
        8 => u32::from(row[dof]),
        16 => u32::from(u16::from_le_bytes([row[2 * dof], row[2 * dof + 1]])),
        _ => u32::from((row[dof / 2] >> (4 * (dof % 2))) & 0x0F),
    }
}

fn dof_infos(object: &Object, at: usize, num_dofs: usize) -> Result<Vec<DofInfo>> {
    (0..num_dofs)
        .map(|i| {
            let record = at + i * DOF_INFO_LEN;
            Ok(DofInfo {
                min: object.f32_at(record)?,
                range: object.f32_at(record + 4)?,
                start: object.f32_at(record + 8)?,
            })
        })
        .collect()
}
