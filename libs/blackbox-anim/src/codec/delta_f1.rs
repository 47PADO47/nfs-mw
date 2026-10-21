//! DeltaF1 (type 21): one scalar per bone, in bins of 2^power frames. Spec section 8 of `docs/specs/eagl-animation-codecs.md`.
//!
//! The leaf is read by `bins.rs`, which decodes it to dense frames. Each dof index names the one float that its bone
//! writes.

use super::DELTA_F1;
use super::bins;
use super::rows::DeltaTrack;
use crate::{Object, Result};

/// One scalar per bone.
const COMPONENTS: usize = 1;

/// Reads the DeltaF1 leaf at `offset` in `object`'s `.data`.
pub(super) fn read(object: &Object, offset: usize) -> Result<DeltaTrack> {
    bins::read(object, offset, DELTA_F1, COMPONENTS)
}
