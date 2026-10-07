#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
    #[error("no TrackStreamingSections chunk")]
    NoIndex,
    #[error("TrackStreamingSections: {len} bytes is not a multiple of the {record}-byte record")]
    BadSize { len: usize, record: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
