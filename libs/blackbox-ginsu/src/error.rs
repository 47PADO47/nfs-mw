//! Errors of the Gnsu parser and the synthesiser constructor.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The data ends before the header or the tables it announces.
    #[error("Gnsu data is truncated: need {needed} bytes, have {have}")]
    Truncated { needed: u64, have: usize },
    /// The first four bytes are not `Gnsu`.
    #[error("not a Gnsu file (bad magic)")]
    BadMagic,
    /// Only version 2 (`"20"`) is known.
    #[error("unsupported Gnsu version byte 0x{0:02X} (only '2' is known)")]
    UnsupportedVersion(u8),
    /// A table violates what the synthesiser relies on.
    #[error("invalid Gnsu tables: {0}")]
    InvalidTables(&'static str),
    /// The decoded audio does not have `sample_count` samples.
    #[error("the file announces {expected} samples but {actual} were given")]
    SampleCountMismatch { expected: usize, actual: usize },
    /// The synthesiser needs at least one pitch cycle to jump between.
    #[error("the file has no pitch cycles")]
    NoCycles,
    /// Packet sizes are derived from the sample rate; below 2 kHz they would be zero.
    #[error("sample rate {0} Hz is too low for the synthesiser (minimum 2000 Hz)")]
    SampleRateTooLow(u32),
}

pub type Result<T> = std::result::Result<T, Error>;
