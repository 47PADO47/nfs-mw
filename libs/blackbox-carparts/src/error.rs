#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no {0} chunk")]
    Missing(&'static str),
    #[error("parts pack version {found} is not supported (this layout reads version {expected})")]
    UnsupportedVersion { found: u32, expected: u32 },
    #[error("{what}: {detail}")]
    Malformed { what: &'static str, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;
