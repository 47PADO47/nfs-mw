//! The error type of the replication layers.

use crate::bits::BitError;
use std::fmt;

/// Why a replication step failed. Malformed input is always reported as one of these, never a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicateError {
    /// A bit stream error: overflow, underflow, a malformed varint and so on.
    Bits(BitError),
    /// A field or kind description is out of range, or a value count does not match its kind.
    BadSchema,
    /// The schema already holds [`super::MAX_KINDS`] kinds.
    TooManyKinds,
    /// Ids or ticks are out of order, or a frame does not carry the cache's tick.
    BadOrder,
    /// A value does not fit its field, or a decoded value is outside the field's range.
    BadValue,
    /// A kind id the schema does not hold.
    UnknownKind,
    /// An entity id the frame does not hold.
    UnknownEntity,
    /// A delta refers to a baseline that is not held.
    MissingBaseline,
    /// An operation code that is not defined.
    BadOp,
}

impl fmt::Display for ReplicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicateError::Bits(err) => write!(f, "bit stream: {err}"),
            ReplicateError::BadSchema => f.write_str("bad schema or value count"),
            ReplicateError::TooManyKinds => f.write_str("too many entity kinds"),
            ReplicateError::BadOrder => f.write_str("ids or ticks out of order"),
            ReplicateError::BadValue => f.write_str("value out of range"),
            ReplicateError::UnknownKind => f.write_str("unknown entity kind"),
            ReplicateError::UnknownEntity => f.write_str("unknown entity id"),
            ReplicateError::MissingBaseline => f.write_str("baseline not held"),
            ReplicateError::BadOp => f.write_str("undefined operation code"),
        }
    }
}

impl std::error::Error for ReplicateError {}

impl From<BitError> for ReplicateError {
    fn from(err: BitError) -> Self {
        ReplicateError::Bits(err)
    }
}
