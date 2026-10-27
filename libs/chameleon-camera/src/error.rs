/// Why the camera data of a bundle could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bundle(#[from] chameleon_bundle::Error),
    #[error("camera data: {0}")]
    Spec(String),
}

pub type Result<T> = std::result::Result<T, Error>;
