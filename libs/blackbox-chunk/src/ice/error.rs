#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IceError {
    #[error("chunk 0x{0:08X} is not an ICE chunk this reader handles")]
    WrongId(u32),
    #[error("ICE data is truncated at payload offset 0x{0:X}")]
    Truncated(usize),
    #[error("record at payload offset 0x{0:X} has a negative element count {1}")]
    NegativeCount(usize, i16),
    #[error(transparent)]
    Chunk(#[from] crate::Error),
}

pub type IceResult<T> = Result<T, IceError>;
