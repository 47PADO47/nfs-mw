//! A mod's configuration on the host side: where its own file is, and how that file and the game's table for the mod
//! merge into the text `bb.config` hands out (catalogue: docs/modding/api-config.md).
//!
//! A mod's configuration has two sources, either of which may be missing:
//!
//! - its own file in its folder: `config.toml`, or the file the manifest's `config` key names;
//! - the table the game keeps for it in its main config file (the game decides where; the NFS:MW game uses `[mods.<id>]`).
//!
//! The effective configuration is the file deep-merged under the table: the table wins key by key, tables merge
//! recursively, and every other value (an array included) is replaced whole. The functions here are pure apart from the
//! one file read, so the rules are tested without a game.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::abi::{BB_CONFIG_FILE, BB_CONFIG_MAIN};

/// The file a mod's configuration is read from when its manifest names none.
pub const DEFAULT_FILE: &str = "config.toml";

/// Why a mod's configuration cannot be built. Each variant says which source is at fault.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// The file could not be read.
    #[error("cannot read {path}: {reason}")]
    Read { path: String, reason: String },
    /// The file, once links are resolved, is outside the mod folder.
    #[error("{0} is outside the mod folder")]
    Escapes(String),
    /// The path in the manifest is not a relative path inside the folder.
    #[error("config {file:?} is not a path inside the mod folder: {reason}")]
    BadPath { file: String, reason: &'static str },
    /// A source is not valid TOML.
    #[error("{source_name} is not valid TOML: {message}")]
    Parse { source_name: &'static str, message: String },
}

/// The configuration of one mod: the merged TOML text, and which sources existed (`BB_CONFIG_*` bits).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Effective {
    /// The merged configuration as TOML text. Empty when neither source exists.
    pub text: String,
    /// `BB_CONFIG_FILE` when the mod's file exists, `BB_CONFIG_MAIN` when the game has a table for the mod.
    pub sources: u32,
}

/// Check that `file` is a path the host may read inside a mod folder: relative, with `/` as the separator, and with no
/// empty, `.` or `..` part. Whether the file is really inside the folder (a link can lead out) is checked when it is read
/// ([`read_file`]).
pub fn check_relative(file: &str) -> Result<(), &'static str> {
    if file.is_empty() {
        return Err("the path is empty");
    }
    if file.contains('\\') {
        return Err("use / as the separator");
    }
    if file.contains(':') || file.contains('\0') {
        return Err("a drive or a scheme is not allowed");
    }
    if file.starts_with('/') {
        return Err("the path is absolute");
    }
    for part in file.split('/') {
        match part {
            "" => return Err("the path has an empty part"),
            "." | ".." => return Err("`.` and `..` are not allowed"),
            _ => {}
        }
    }
    Ok(())
}

/// The file of the mod in `dir`, or `None` when it does not exist. `file` is the manifest's `config` value, or `None` for
/// [`DEFAULT_FILE`]. A path that is not a relative path inside the folder, or a link that leads out of it, is an error.
pub fn resolve_file(dir: &Path, file: Option<&str>) -> Result<Option<PathBuf>, ConfigError> {
    let relative = file.unwrap_or(DEFAULT_FILE);
    check_relative(relative).map_err(|reason| ConfigError::BadPath { file: relative.to_owned(), reason })?;
    let path = dir.join(relative);
    let resolved = match path.canonicalize() {
        Ok(resolved) => resolved,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(read_error(&path, &error)),
    };
    let root = dir.canonicalize().map_err(|error| read_error(dir, &error))?;
    if !resolved.starts_with(&root) {
        return Err(ConfigError::Escapes(path.display().to_string()));
    }
    Ok(resolved.is_file().then_some(resolved))
}

/// The text of the mod's file in `dir` (see [`resolve_file`]), or `None` when there is none.
pub fn read_file(dir: &Path, file: Option<&str>) -> Result<Option<String>, ConfigError> {
    let Some(path) = resolve_file(dir, file)? else {
        return Ok(None);
    };
    fs::read_to_string(&path).map(Some).map_err(|error| read_error(&path, &error))
}

fn read_error(path: &Path, error: &std::io::Error) -> ConfigError {
    ConfigError::Read { path: path.display().to_string(), reason: error.to_string() }
}

/// Merge `over` into `base`: a key both have as tables merges recursively, any other key of `over` replaces the key of
/// `base` (a scalar, an array, or a table where `base` has a scalar, and the other way round).
pub fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        let merged = match (base.remove(&key), value) {
            (Some(Value::Table(mut inner)), Value::Table(over_inner)) => {
                merge(&mut inner, over_inner);
                Value::Table(inner)
            }
            (_, value) => value,
        };
        base.insert(key, merged);
    }
}

/// The effective configuration of a mod from the text of its file and the text of its table in the main config. Either may
/// be `None` (the source does not exist). The text of `main` is the table's own contents, not the whole config file.
pub fn effective(file: Option<&str>, main: Option<&str>) -> Result<Effective, ConfigError> {
    let mut sources = 0;
    let mut merged = Table::new();
    if let Some(text) = file {
        sources |= BB_CONFIG_FILE;
        merged = parse(text, "the mod's config file")?;
    }
    if let Some(text) = main {
        sources |= BB_CONFIG_MAIN;
        merge(&mut merged, parse(text, "the mod's table in the main config")?);
    }
    if sources == 0 {
        return Ok(Effective::default());
    }
    let text = toml::to_string(&merged)
        .map_err(|error| ConfigError::Parse { source_name: "the merged configuration", message: error.to_string() })?;
    Ok(Effective { text, sources })
}

/// The effective configuration of the mod in `dir`: its file (`file` is the manifest's `config` value) merged under
/// `main`, the text of its table in the game's main config.
pub fn load(dir: &Path, file: Option<&str>, main: Option<&str>) -> Result<Effective, ConfigError> {
    let own = read_file(dir, file)?;
    effective(own.as_deref(), main)
}

fn parse(text: &str, source_name: &'static str) -> Result<Table, ConfigError> {
    text.parse::<Table>().map_err(|error| ConfigError::Parse { source_name, message: error.to_string() })
}
