//! `mod.toml`: the manifest every mod folder carries (ADR 0005 section 3).
//!
//! [`parse`] turns the TOML text into a typed [`Manifest`] or a [`ManifestError`]. It checks the values only; whether
//! the host can run the mod is decided later, against the host's offer (see `discovery`).

use std::collections::BTreeMap;

use semver::{Version, VersionReq};
use serde::Deserialize;

use crate::abi::AbiVersion;

/// The file name every mod folder carries.
pub const MANIFEST_FILE: &str = "mod.toml";

/// Longest mod id, in bytes.
const MAX_ID_BYTES: usize = 64;

/// A parsed and checked `mod.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Unique id, lower case: `a-z`, `0-9`, `_`, `-`, `.`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Mod version (semver).
    pub version: Version,
    /// Author names, as written.
    pub authors: Vec<String>,
    /// License, as written. The host does not check it.
    pub license: Option<String>,
    /// The ABI version the mod was built against.
    pub api: AbiVersion,
    /// Games the mod runs in.
    pub games: GameSupport,
    /// The library file per platform. Empty for a content-only mod.
    pub library: Library,
    /// Interfaces the mod cannot run without. An absent one rejects the mod.
    pub required_interfaces: Vec<InterfaceRequirement>,
    /// Interfaces the mod can do without. Probed at run time, so not checked here.
    pub optional_interfaces: Vec<InterfaceRequirement>,
    /// Other mods this one needs, with semver requirements.
    pub dependencies: Vec<Dependency>,
    /// Mods that must load before this one, if they are present. A hint, not a requirement.
    pub load_after: Vec<String>,
    /// Mods that must load after this one, if they are present. A hint, not a requirement.
    pub load_before: Vec<String>,
    /// VFS overlay priority (ADR 0005 section 5). Higher wins; ties follow load order.
    pub priority: i32,
}

/// The games a mod declares it runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameSupport {
    /// `supported = ["*"]`: a mod that only uses `bb.*` interfaces.
    Any,
    /// The listed game ids.
    Only(Vec<String>),
}

impl GameSupport {
    /// Whether the mod declares support for `game`.
    pub fn supports(&self, game: &str) -> bool {
        match self {
            GameSupport::Any => true,
            GameSupport::Only(games) => games.iter().any(|listed| listed == game),
        }
    }
}

/// The library file per platform, as the manifest names it. Each name is a plain file name in the mod folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    /// File name for Windows, ending in `.dll`.
    pub windows: Option<String>,
    /// File name for Linux, ending in `.so`.
    pub linux: Option<String>,
    /// File name for macOS, ending in `.dylib`.
    pub macos: Option<String>,
}

impl Library {
    /// The file name the manifest gives for `os`, a value of `std::env::consts::OS`. `None` means the mod has no
    /// library on that platform.
    pub fn file_for(&self, os: &str) -> Option<&str> {
        match os {
            "windows" => self.windows.as_deref(),
            "linux" => self.linux.as_deref(),
            "macos" => self.macos.as_deref(),
            _ => None,
        }
    }
}

/// An interface version as the manifest writes it: `"major.minor"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceVersion {
    /// Breaking changes. The offer must match it exactly.
    pub major: u16,
    /// Additive changes. The offer must be at least this.
    pub minor: u16,
}

impl std::fmt::Display for InterfaceVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// One interface a mod requires or optionally uses, with the version it was written against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceRequirement {
    /// Interface id, for example `bb.commands`.
    pub id: String,
    /// The version the mod asks for.
    pub version: InterfaceVersion,
}

/// A dependency on another mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Id of the other mod.
    pub id: String,
    /// The versions of it that satisfy the mod.
    pub requirement: VersionReq,
}

/// Why a manifest is refused. Each variant gives one reason, shown in the log and the `mods` console command.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// The file is not valid TOML, or lacks a required field.
    #[error("the manifest does not parse: {0}")]
    Parse(String),
    /// The file could not be read.
    #[error("the manifest cannot be read: {0}")]
    Read(String),
    /// The id is not `a-z 0-9 _ - .`, 1 to 64 bytes, without a leading or trailing dot or a `..`.
    #[error("{0:?} is not a valid mod id")]
    InvalidId(String),
    /// The display name is empty.
    #[error("the name is empty")]
    EmptyName,
    /// The version is not semver.
    #[error("version {0:?} is not semver")]
    InvalidVersion(String),
    /// The ABI version is not `MAJOR.MINOR`.
    #[error("api version {0:?} is not MAJOR.MINOR")]
    InvalidApiVersion(String),
    /// An interface version is not `MAJOR.MINOR`.
    #[error("interface {id} has version {value:?}, which is not MAJOR.MINOR")]
    InvalidInterfaceVersion { id: String, value: String },
    /// A dependency's version requirement is not valid semver.
    #[error("dependency {id} has the version requirement {value:?}, which is not valid")]
    InvalidDependency { id: String, value: String },
    /// `games.supported` is empty.
    #[error("games.supported lists no game")]
    NoGames,
    /// A game id is empty or has characters other than `a-z 0-9 _ -`, or `*` is listed with other games.
    #[error("game id {0:?} is not valid")]
    InvalidGame(String),
    /// A library file name is not a plain file name with the platform's extension.
    #[error("library file {file:?} for {key} must be a plain file name ending in .{ext}")]
    InvalidLibrary { key: &'static str, file: String, ext: &'static str },
    /// The mod names itself as a dependency or in a load-order hint.
    #[error("mod {0} names itself")]
    SelfReference(String),
}

/// Parse and check the text of a `mod.toml`.
pub fn parse(text: &str) -> Result<Manifest, ManifestError> {
    let raw: RawManifest = toml::from_str(text).map_err(|error| ManifestError::Parse(error.to_string()))?;
    validate(raw)
}

/// The id rule of ADR 0005 section 3: `a-z 0-9 _ - .`, 1 to 64 bytes, starting and ending with `a-z` or `0-9`, and
/// no `..`. The id is also a folder name in the reload cache, so it must not step out of that folder.
pub fn is_valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_ID_BYTES {
        return false;
    }
    let is_edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !is_edge(bytes[0]) || !is_edge(bytes[bytes.len() - 1]) {
        return false;
    }
    if id.contains("..") {
        return false;
    }
    bytes.iter().all(|&byte| is_edge(byte) || matches!(byte, b'_' | b'-' | b'.'))
}

/// The raw shape of the file, as serde reads it. Everything is checked in [`validate`].
#[derive(Deserialize)]
struct RawManifest {
    #[serde(rename = "mod")]
    module: RawModule,
    api: RawApi,
    games: RawGames,
    #[serde(default)]
    library: RawLibrary,
    #[serde(default)]
    requires: RawInterfaces,
    #[serde(default)]
    optional: RawInterfaces,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default)]
    load: RawLoad,
    #[serde(default)]
    content: RawContent,
}

#[derive(Deserialize)]
struct RawModule {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    authors: Vec<String>,
    license: Option<String>,
}

#[derive(Deserialize)]
struct RawApi {
    blackbox_modding: String,
}

#[derive(Deserialize)]
struct RawGames {
    supported: Vec<String>,
}

#[derive(Default, Deserialize)]
struct RawLibrary {
    windows: Option<String>,
    linux: Option<String>,
    macos: Option<String>,
}

#[derive(Default, Deserialize)]
struct RawInterfaces {
    #[serde(default)]
    interfaces: BTreeMap<String, String>,
}

#[derive(Default, Deserialize)]
struct RawLoad {
    #[serde(default)]
    after: Vec<String>,
    #[serde(default)]
    before: Vec<String>,
}

#[derive(Default, Deserialize)]
struct RawContent {
    #[serde(default)]
    priority: i32,
}

fn validate(raw: RawManifest) -> Result<Manifest, ManifestError> {
    let id = checked_id(&raw.module.id)?;
    let name = raw.module.name.trim().to_owned();
    if name.is_empty() {
        return Err(ManifestError::EmptyName);
    }
    let version =
        Version::parse(&raw.module.version).map_err(|_| ManifestError::InvalidVersion(raw.module.version.clone()))?;
    let (api_major, api_minor) = parse_pair(&raw.api.blackbox_modding)
        .ok_or_else(|| ManifestError::InvalidApiVersion(raw.api.blackbox_modding.clone()))?;
    let games = game_support(raw.games.supported)?;
    let library = library(raw.library)?;
    let dependencies = dependencies(&id, raw.dependencies)?;
    let load_after = references(&id, raw.load.after)?;
    let load_before = references(&id, raw.load.before)?;
    Ok(Manifest {
        id,
        name,
        version,
        authors: raw.module.authors,
        license: raw.module.license,
        api: AbiVersion { major: api_major, minor: api_minor },
        games,
        library,
        required_interfaces: interfaces(raw.requires.interfaces)?,
        optional_interfaces: interfaces(raw.optional.interfaces)?,
        dependencies,
        load_after,
        load_before,
        priority: raw.content.priority,
    })
}

fn checked_id(text: &str) -> Result<String, ManifestError> {
    if !is_valid_id(text) {
        return Err(ManifestError::InvalidId(text.to_owned()));
    }
    Ok(text.to_owned())
}

/// Parse `"MAJOR.MINOR"`: exactly two numbers.
fn parse_pair(text: &str) -> Option<(u16, u16)> {
    let (major, minor) = text.split_once('.')?;
    let major = major.parse::<u16>().ok()?;
    let minor = minor.parse::<u16>().ok()?;
    Some((major, minor))
}

fn interfaces(map: BTreeMap<String, String>) -> Result<Vec<InterfaceRequirement>, ManifestError> {
    let mut out = Vec::with_capacity(map.len());
    for (id, value) in map {
        let checked = checked_id(&id)?;
        let Some((major, minor)) = parse_pair(&value) else {
            return Err(ManifestError::InvalidInterfaceVersion { id: checked, value });
        };
        out.push(InterfaceRequirement { id: checked, version: InterfaceVersion { major, minor } });
    }
    Ok(out)
}

fn dependencies(own_id: &str, map: BTreeMap<String, String>) -> Result<Vec<Dependency>, ManifestError> {
    let mut out = Vec::with_capacity(map.len());
    for (id, value) in map {
        let checked = checked_id(&id)?;
        if checked == own_id {
            return Err(ManifestError::SelfReference(checked));
        }
        let Ok(requirement) = VersionReq::parse(&value) else {
            return Err(ManifestError::InvalidDependency { id: checked, value });
        };
        out.push(Dependency { id: checked, requirement });
    }
    Ok(out)
}

fn references(own_id: &str, ids: Vec<String>) -> Result<Vec<String>, ManifestError> {
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let checked = checked_id(&id)?;
        if checked == own_id {
            return Err(ManifestError::SelfReference(checked));
        }
        out.push(checked);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn game_support(games: Vec<String>) -> Result<GameSupport, ManifestError> {
    if games.is_empty() {
        return Err(ManifestError::NoGames);
    }
    if games.iter().any(|game| game == "*") {
        if games.len() != 1 {
            return Err(ManifestError::InvalidGame("*".to_owned()));
        }
        return Ok(GameSupport::Any);
    }
    for game in &games {
        let valid = !game.is_empty()
            && game
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-'));
        if !valid {
            return Err(ManifestError::InvalidGame(game.clone()));
        }
    }
    Ok(GameSupport::Only(games))
}

fn library(raw: RawLibrary) -> Result<Library, ManifestError> {
    Ok(Library {
        windows: plain_library_file("windows", raw.windows, "dll")?,
        linux: plain_library_file("linux", raw.linux, "so")?,
        macos: plain_library_file("macos", raw.macos, "dylib")?,
    })
}

/// A library name must be a file name in the mod folder: no separators, no drive prefix, and the extension of its
/// platform. The host then opens exactly `<mod folder>/<name>`.
fn plain_library_file(
    key: &'static str,
    file: Option<String>,
    ext: &'static str,
) -> Result<Option<String>, ManifestError> {
    let Some(file) = file else {
        return Ok(None);
    };
    let plain = file.len() > ext.len() + 1 && !file.contains(['/', '\\', ':']) && file.ends_with(&format!(".{ext}"));
    if !plain {
        return Err(ManifestError::InvalidLibrary { key, file, ext });
    }
    Ok(Some(file))
}

#[cfg(test)]
mod tests;
