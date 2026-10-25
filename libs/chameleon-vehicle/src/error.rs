/// Why a vehicle could not be assembled.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bundle(#[from] chameleon_bundle::Error),
    #[error(transparent)]
    Gfx(#[from] chameleon_gfx::Error),
    #[error("the bundle has no vehicle graphics spec")]
    NoSpec,
    #[error("graphics spec: {0}")]
    Spec(String),
    #[error("resource {id:016X} is a type 0x{found:X}, expected 0x{expected:X}")]
    WrongKind { id: u64, found: u32, expected: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;
