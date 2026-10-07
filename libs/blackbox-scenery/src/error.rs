#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
    #[error("scenery section at 0x{offset:X}: {detail}")]
    Section { offset: usize, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;
