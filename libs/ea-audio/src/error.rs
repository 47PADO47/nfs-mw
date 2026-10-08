//! Errors for every reader and decoder in the crate.

/// Everything that can go wrong while reading EA audio data.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The data ends before the structure being read does.
    #[error("data ends early (needed {needed} bytes at offset {offset})")]
    Truncated { offset: u64, needed: usize },
    /// A magic number or block tag is not the expected one.
    #[error("bad magic: expected {expected}")]
    BadMagic { expected: &'static str },
    /// A header field has a value the format does not allow.
    #[error("bad header: {0}")]
    BadHeader(&'static str),
    /// The stream uses a codec this crate does not decode (value of the `codec2`/`codec1` tag).
    #[error("unsupported codec 0x{0:02X}")]
    UnsupportedCodec(u32),
    /// The stream layout is valid but not one this crate reads.
    #[error("unsupported layout: {0}")]
    Unsupported(&'static str),
    /// Encoded data fails a consistency check.
    #[error("corrupt data: {0}")]
    Corrupt(&'static str),
    /// A sound, stream or track index is out of range.
    #[error("no such entry: {0}")]
    NoSuchEntry(usize),
}

/// `Result` with the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
