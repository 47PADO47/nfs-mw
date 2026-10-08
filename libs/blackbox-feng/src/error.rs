use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// What can go wrong while reading a package or a font.
#[derive(Debug, Error)]
pub enum Error {
    #[error("the data ends inside {0}")]
    Truncated(&'static str),
    #[error("not an FEng package: {0}")]
    NotAPackage(&'static str),
    #[error("package version {0:#x} is older than the 0x20000 this reader supports")]
    OldVersion(u32),
    #[error("{0} is malformed: {1}")]
    Malformed(&'static str, String),
    #[error("not an FEng font: {0}")]
    NotAFont(&'static str),
    #[error("cannot decompress the package: {0}")]
    Decompress(String),
}
