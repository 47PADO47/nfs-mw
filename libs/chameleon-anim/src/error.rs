/// Why an animation resource could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{what} needs bytes {start}..{end} of {len}")]
    Truncated { what: &'static str, start: usize, end: usize, len: usize },
    #[error("{what}: {detail}")]
    Invalid { what: &'static str, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;
