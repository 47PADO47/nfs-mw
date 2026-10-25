#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("{0} trailing byte(s) at 0x{1:X} are too short for a chunk header")]
    Trailing(usize, usize),
    #[error("chunk 0x{id:08X} at 0x{offset:X} claims {size} bytes but its parent ends at 0x{parent_end:X}")]
    Overrun { id: u32, offset: usize, size: usize, parent_end: usize },
    #[error("chunk 0x{0:08X} is too big for a 32-bit size field")]
    TooLarge(u32),
}

pub type Result<T> = std::result::Result<T, Error>;
