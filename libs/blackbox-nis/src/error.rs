use thiserror::Error;

/// Why a NIS scene payload could not be read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// The payload ends before a field that the format requires.
    #[error("truncated {what} at offset {offset:#x}")]
    Truncated {
        /// The structure that was being read, for example `"scene header"`.
        what: &'static str,
        /// The byte offset at which the read ran out of data.
        offset: usize,
    },
    /// The payload is well formed in its framing but its contents contradict the format.
    #[error("invalid NIS scene data: {0}")]
    Invalid(String),
    /// The chunk tree around the scene is malformed.
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
}

/// The result of reading a NIS scene payload.
pub type Result<T> = std::result::Result<T, Error>;
