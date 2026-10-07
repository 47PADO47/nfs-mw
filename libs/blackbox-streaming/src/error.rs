#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
    #[error("no TrackStreamingSections chunk")]
    NoIndex,
    #[error("TrackStreamingSections: {len} bytes is not a multiple of the {record}-byte record")]
    BadSize { len: usize, record: usize },
    #[error("no VisibleSectionManager chunk, or one of its tables is missing")]
    NoVisibleSections,
    #[error("{table}: record at 0x{offset:X} runs past the end of the chunk")]
    Truncated { table: &'static str, offset: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
