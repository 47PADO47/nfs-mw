#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{what}: needs {need} bytes at offset 0x{offset:X}, only {len} in the buffer")]
    Truncated { what: &'static str, offset: usize, need: usize, len: usize },
    #[error("{what}: {detail}")]
    Malformed { what: &'static str, detail: String },
    #[error("no {0} in the file")]
    Missing(&'static str),
    #[error(transparent)]
    Carp(#[from] blackbox_collision::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn malformed(what: &'static str, detail: impl Into<String>) -> Error {
    Error::Malformed { what, detail: detail.into() }
}
