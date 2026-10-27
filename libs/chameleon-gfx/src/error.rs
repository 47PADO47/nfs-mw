/// Why a graphics resource could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{what}: needs bytes {start}..{end} but the block has {len}")]
    OutOfRange { what: &'static str, start: usize, end: usize, len: usize },
    #[error("{what}: {detail}")]
    Invalid { what: &'static str, detail: String },
    #[error("{what}: no import at offset 0x{offset:X}")]
    MissingImport { what: &'static str, offset: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;
