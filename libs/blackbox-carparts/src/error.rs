#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no {0} chunk")]
    Missing(&'static str),
    #[error("parts pack version {found} is not supported (this layout reads version {expected})")]
    UnsupportedVersion { found: u32, expected: u32 },
    #[error("{what}: {detail}")]
    Malformed { what: &'static str, detail: String },
    /// A packed table has no room for one more entry: its index field is too narrow.
    #[error("the {0} table is full")]
    Full(&'static str),
    /// A value does not fit the bits the part record keeps for it.
    #[error("{what} {value} is out of range (at most {max})")]
    OutOfRange { what: &'static str, value: u32, max: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;
