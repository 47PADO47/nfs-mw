/// Why a bundle could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a bnd2 bundle (magic {magic:02X?})")]
    BadMagic { magic: [u8; 4] },
    #[error("bundle version {version} on platform {platform} is not supported (only version 5 on PC)")]
    Unsupported { version: u16, platform: u16 },
    #[error("bundle is truncated: {what} needs bytes {start}..{end} of {len}")]
    Truncated { what: &'static str, start: usize, end: usize, len: usize },
    #[error("resource {index}: block {block} does not inflate: {detail}")]
    Inflate { index: usize, block: usize, detail: String },
    #[error("resource {index}: block {block} is {actual} bytes, the table says {expected}")]
    BlockSize { index: usize, block: usize, expected: usize, actual: usize },
    #[error("resource {index}: import table of {count} entries at 0x{offset:X} is outside its {len}-byte block")]
    ImportTable { index: usize, offset: usize, count: usize, len: usize },
    #[error("no resource {id:016X} in the bundle")]
    NoSuchResource { id: u64 },
}

pub type Result<T> = std::result::Result<T, Error>;
