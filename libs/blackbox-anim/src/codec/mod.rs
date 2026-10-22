//! Channel codecs: one leaf object per animated channel, decoded once when it is read and sampled by time.
//!
//! The byte layouts and decode rules are in `docs/specs/eagl-animation-codecs.md`. A channel writes its values into
//! a flat `f32` buffer that holds 12 floats per bone (see [`Channel::sample`]).

mod bins;
mod delta_f1;
mod delta_f3;
mod delta_lerp;
mod delta_quat;
mod rows;

use crate::{Error, Object, Result};

/// The codec type of a DeltaLerp leaf: one scalar per dof index.
pub const DELTA_LERP: u16 = 10;
/// The codec type of a DeltaQuat leaf: four consecutive floats per dof index.
pub const DELTA_QUAT: u16 = 11;
/// The codec type of a DeltaF3 leaf: three floats per bone, in bins of 2^power frames.
pub const DELTA_F3: u16 = 20;
/// The codec type of a DeltaF1 leaf: one float per bone, in bins of 2^power frames.
pub const DELTA_F1: u16 = 21;

/// A decoded animation channel.
pub trait Channel {
    /// The number of frames in the channel.
    fn frames(&self) -> u32;

    /// The dof indices the channel writes. Each index is the first of the channel's floats for one value. A
    /// quaternion channel writes four floats per index.
    fn dofs(&self) -> &[u16];

    /// Writes the value at `frame` into `out`. `frame` is in frames and may be fractional. It is clamped to
    /// `[0, frames - 1]`, and a value between two frames is a linear blend of them. Only the slots of
    /// [`Channel::dofs`] are written. A slot past the end of `out` is skipped.
    fn sample(&self, frame: f32, out: &mut [f32]);
}

/// Reads the channel leaf of codec `type_id` at `offset` in `object`'s `.data`.
///
/// Returns [`Error::Unsupported`] for a codec this reader does not implement yet.
pub fn read_leaf(object: &Object, type_id: u16, offset: u32) -> Result<Box<dyn Channel + Send + Sync>> {
    let offset = offset as usize;
    match type_id {
        DELTA_LERP => {
            let channel: Box<dyn Channel + Send + Sync> = Box::new(delta_lerp::read(object, offset)?);
            Ok(channel)
        }
        DELTA_QUAT => {
            let channel: Box<dyn Channel + Send + Sync> = Box::new(delta_quat::read(object, offset)?);
            Ok(channel)
        }
        DELTA_F3 => {
            let channel: Box<dyn Channel + Send + Sync> = Box::new(delta_f3::read(object, offset)?);
            Ok(channel)
        }
        DELTA_F1 => {
            let channel: Box<dyn Channel + Send + Sync> = Box::new(delta_f1::read(object, offset)?);
            Ok(channel)
        }
        other => Err(Error::Unsupported(format!("channel codec type {other}"))),
    }
}
