//! Why a buffer cannot be read as ACL transform tracks.

use thiserror::Error;

/// A failure to parse or decode a compressed track list.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AclError {
    /// The buffer ends before a field or a block the headers promise.
    #[error("the buffer is cut short at byte {0}")]
    Truncated(usize),
    /// The first words are not a compressed track list.
    #[error("not an ACL compressed track list (tag {0:#010x})")]
    BadTag(u32),
    /// A format revision this reader does not know (it reads revisions 9 and 10, ACL 2.1).
    #[error("unsupported ACL buffer version {0}")]
    BadVersion(u16),
    /// Only transform tracks of the uniformly sampled algorithm are read.
    #[error("unsupported track list: algorithm {algorithm}, track type {track_type}")]
    BadKind { algorithm: u8, track_type: u8 },
    /// A feature the buffer uses that is not implemented (streamed databases, unknown packing formats).
    #[error("unsupported feature: {0}")]
    Unsupported(&'static str),
    /// A field has a value no encoder writes.
    #[error("invalid data: {0}")]
    Invalid(&'static str),
    /// The clip needs the skeleton's reference pose for its default sub-tracks and none (or too few) was given.
    #[error("the clip needs {0} default transforms")]
    MissingDefaults(usize),
    /// A sample index or a track index past the end.
    #[error("index {index} out of range ({count})")]
    OutOfRange { index: usize, count: usize },
}
