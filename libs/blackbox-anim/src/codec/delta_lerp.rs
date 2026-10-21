//! DeltaLerp (type 10): one scalar per dof index. Spec section 6 of `docs/specs/eagl-animation-codecs.md`.
//!
//! The leaf is a DeltaChan header and a DeltaCompressedData block, both read by `rows.rs`. Each dof index names
//! the one float that the channel writes.

use super::DELTA_LERP;
use super::rows::DeltaTrack;
use crate::{Object, Result};

/// One float per dof index.
const WIDTH: usize = 1;

/// Reads the DeltaLerp leaf at `offset` in `object`'s `.data`.
pub(super) fn read(object: &Object, offset: usize) -> Result<DeltaTrack> {
    DeltaTrack::read(object, offset, DELTA_LERP, WIDTH)
}
