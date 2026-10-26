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
    /// `heat`, `[[cop]]` or `cop_class` is given for a car that is not a cop.
    #[error("heat, [[cop]] and cop_class are for cop use: set role to cop or both")]
    HeatWithoutCop,
    /// A `[[cop]]` count is outside 1 to 8.
    #[error("cop count {0} is not from 1 to 8")]
    InvalidCopCount(u32),
    /// A `[[cop]]` chance is outside 1 to 100.
    #[error("cop chance {0} is not from 1 to 100 percent")]
    InvalidCopChance(u32),
    /// A music context is not a name of lower-case letters and `_`.
    #[error("music context {0:?} is not a name (lower-case letters and _)")]
    InvalidMusicContext(String),
    /// A music context is listed twice.
    #[error("music context {0:?} is listed twice")]
    RepeatedMusicContext(String),
    /// `contexts` is an empty list.
    #[error("music contexts lists no context")]
    NoMusicContext,
    /// A racer name is empty, longer than 24 characters or has a control character.
    #[error("racer name {0:?} is empty, longer than 24 characters or has a control character")]
    InvalidRacerName(String),
    /// A racer name is listed twice.
    #[error("racer name {0:?} is listed twice")]
    RepeatedRacerName(String),
    /// The `[pack]` key, or the `[interior]` key, is not the file name.
    #[error("key {key:?} does not match the file name {file:?}")]
    KeyMismatch { file: String, key: String },
    /// The pack file is not a relative path under `files/` ending in `.BUN`.
    #[error("{0:?} is not a pack file: a path under files/, ending in .BUN, with no .. and no drive")]
    InvalidPackFile(String),
    /// The section number is not from 1 to 32767.
    #[error("section {0} is not a number from 1 to 32767")]
    InvalidSection(u16),
    /// The anchor has a value that is not finite, or a radius that is not above 0.
    #[error("anchor is a centre (x, y, z) and a radius above 0, all finite")]
    InvalidAnchor,
    /// The route is not `tile`, the only route in this delivery.
    #[error("route {0:?} is not in this delivery: only \"tile\" is")]
    RouteNotInV1(String),
    /// The pack has no `[[place]]`.
    #[error("the pack places nothing")]
    NoPlace,
    /// A placement is not valid. `at` counts from 1, in file order.
    #[error("place {at}: {reason}")]
    Place { at: usize, reason: &'static str },
    /// A model name is not valid (letters, digits, `_` and `-`, 1 to 63 bytes).
    #[error("{0:?} is not a model name (letters, digits, _ and -, 1 to 63 bytes)")]
    InvalidModel(String),
    /// A text field is empty or has a control character.
    #[error("{0:?} is empty or has a control character")]
    InvalidText(String),
    /// An interior field is not valid.
    #[error("{field}: {reason}")]
    Field { field: &'static str, reason: &'static str },
    /// The zone shape is not `sphere` or `box`.
    #[error("shape {0:?} is not sphere or box")]
    UnknownShape(String),
    /// The interior kind is not `area`, the only kind in this delivery.
    #[error("kind {0:?} is not in this delivery: only \"area\" is")]
    KindNotInV1(String),
    /// A name in `suspend` is not traffic, cops or pursuit.
    #[error("suspend {0:?} is not traffic, cops or pursuit")]
    UnknownSuspend(String),
    /// A name is listed twice in `suspend`.
    #[error("suspend {0:?} is listed twice")]
    RepeatedSuspend(String),
}
