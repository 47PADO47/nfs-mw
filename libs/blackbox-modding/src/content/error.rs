//! Why a content file is refused. One reason per error, written to the log. The game skips that entry; the mod and the
//! game keep running.

/// Why a content file is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    /// The file is not valid TOML, a field has the wrong type, or it has a key the schema does not know.
    #[error("the file does not parse: {0}")]
    Parse(String),
    /// The file name is not a valid car id.
    #[error("{0:?} is not a valid car id (letters, digits, _ and -, 1 to 32 bytes)")]
    InvalidId(String),
    /// `clone` is not a valid car id.
    #[error("clone {0:?} is not a valid car id (letters, digits, _ and -, 1 to 32 bytes)")]
    InvalidClone(String),
    /// `name` is empty or has a control character.
    #[error("name {0:?} is empty or has a control character")]
    InvalidName(String),
    /// `manufacturer` is empty or has a control character.
    #[error("manufacturer {0:?} is empty or has a control character")]
    InvalidManufacturer(String),
    /// A heat level is 0. Levels start at 1.
    #[error("heat level {0} is not a level: levels start at 1")]
    InvalidHeat(u32),
    /// A heat level is listed twice.
    #[error("heat level {0} is listed twice")]
    RepeatedHeat(u32),
    /// `heat` is an empty list.
    #[error("heat lists no level")]
    EmptyHeat,
    /// `heat` is given for a car that is not a cop.
    #[error("heat is for cop use: set role to cop or both")]
    HeatWithoutCop,
    /// A music context is not a name of lower-case letters and `_`.
    #[error("music context {0:?} is not a name (lower-case letters and _)")]
    InvalidMusicContext(String),
    /// A music context is listed twice.
    #[error("music context {0:?} is listed twice")]
    RepeatedMusicContext(String),
    /// `contexts` is an empty list.
    #[error("music contexts lists no context")]
    NoMusicContext,
}
