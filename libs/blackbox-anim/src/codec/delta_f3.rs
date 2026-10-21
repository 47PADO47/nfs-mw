//! DeltaF3 (type 20): three components per bone, in bins of 2^power frames. Spec section 9 of
//! `docs/specs/eagl-animation-codecs.md`.
//!
//! The leaf is read by `bins.rs`, which decodes it to dense frames. Each dof index names the first of the three floats
//! that its bone writes (scale, rotation or translation, by channel).

use super::DELTA_F3;
use super::bins;
use super::rows::DeltaTrack;
use crate::{Object, Result};

/// Three components per bone.
const COMPONENTS: usize = 3;

/// Reads the DeltaF3 leaf at `offset` in `object`'s `.data`.
pub(super) fn read(object: &Object, offset: usize) -> Result<DeltaTrack> {
    bins::read(object, offset, DELTA_F3, COMPONENTS)
}
