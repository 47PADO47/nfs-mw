#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
    #[error(transparent)]
    Compress(#[from] ea_compress::Error),
    #[error("solid at 0x{offset:X}: {detail}")]
    Solid { offset: usize, detail: String },
    #[error("solid at 0x{offset:X}: SolidInfo version 0x{version:02X} is not supported yet")]
    UnsupportedVersion { offset: usize, version: u8 },
}

pub type Result<T> = std::result::Result<T, Error>;
