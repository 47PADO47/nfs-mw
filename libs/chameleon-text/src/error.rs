/// Why a string table could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{what}: needs bytes {start}..{end} but the block has {len}")]
    Truncated { what: &'static str, start: usize, end: usize, len: usize },
    #[error("string {index}: the text does not end with a NUL")]
    Unterminated { index: usize },
    #[error("string {index}: the text is not valid UTF-16")]
    BadText { index: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
