//! Errors of the mixer map parser.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// A table or element reaches past the end of the data.
    #[error("mixer map is truncated: need {needed} bytes at offset {at}, have {have}")]
    Truncated { at: usize, needed: usize, have: usize },
    /// An offset is negative where one is required, or points outside the data.
    #[error("bad offset {0} in the mixer map")]
    BadOffset(i64),
    /// A count that cannot fit in the data that is left.
    #[error("implausible count {0} in the mixer map")]
    BadCount(u32),
}

pub type Result<T> = std::result::Result<T, Error>;
