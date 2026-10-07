#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
    #[error(transparent)]
    Compress(#[from] ea_compress::Error),
    #[error("texture pack at 0x{offset:X}: {detail}")]
    Pack { offset: usize, detail: String },
    #[error("texture pack at 0x{offset:X}: TPK version {version} is not supported yet")]
    UnsupportedVersion { offset: usize, version: u32 },
    #[error("texture 0x{hash:08X}: {detail}")]
    Texture { hash: u32, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;
