//! Why a server config was refused.

use std::path::PathBuf;

use nfsmw_net::settings::SettingsError;

/// A config file that could not be read, parsed or accepted. Every variant names the key or the file, so the message is
/// enough to fix the file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("not valid TOML: {0}")]
    Syntax(String),
    #[error("unknown key `{0}` (see docs/specs/multiplayer/server.md for the keys)")]
    UnknownKey(String),
    #[error("key `{key}` must be {expected}")]
    Type { key: String, expected: &'static str },
    #[error("key `{key}` is out of range: {reason}")]
    Range { key: String, reason: &'static str },
    #[error("key `{key}`: {error:?}")]
    Setting { key: String, error: SettingsError },
    #[error("the session settings do not hold together: {0:?}")]
    Settings(SettingsError),
    #[error("key `{key}` is not supported yet: {reason}")]
    Unsupported { key: String, reason: &'static str },
}
