//! DeltaQuat (type 11): one quaternion per dof index. Spec section 7 of `docs/specs/eagl-animation-codecs.md`.
//!
//! The leaf layout is the DeltaLerp one (`delta_lerp.rs`), with four floats per dof index. The four components are
//! written as decoded, in data order 0 to 3, with no renormalisation: the caller normalises the quaternion.

use super::DELTA_QUAT;
use super::rows::DeltaTrack;
use crate::{Object, Result};

/// Four floats per dof index, in stored order (the last is W, which the spec marks as inferred).
const WIDTH: usize = 4;

/// Reads the DeltaQuat leaf at `offset` in `object`'s `.data`.
pub(super) fn read(object: &Object, offset: usize) -> Result<DeltaTrack> {
    DeltaTrack::read(object, offset, DELTA_QUAT, WIDTH)
}
