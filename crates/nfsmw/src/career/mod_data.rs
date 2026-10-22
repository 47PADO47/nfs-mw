//! Mod data: one blob per mod and career profile, in the profile's `mods` folder (`docs/specs/mod-save-data.md`).
//!
//! A blob is `<key>.bin`: the MD5 of everything after it (`save_store::digest`), the magic `MODD`, the format version
//! (1), the body length (u64, little endian), then the body. The body belongs to the mod. A write goes to a temporary
//! file that then replaces the blob, so a failed write leaves the old blob whole.

use std::fs;
use std::io;
use std::path::PathBuf;

use save_store::digest;

/// The magic at the start of the body.
const MAGIC: &[u8; 4] = b"MODD";
/// The format version this build writes and reads.
const VERSION: u32 = 1;
/// The longest body a blob may hold, in bytes (`docs/specs/mod-save-data.md`, section 3).
pub const MAX_BODY: usize = 256 * 1024;
/// The longest key, in characters.
const MAX_KEY: usize = 64;
/// The extension of a blob.
const EXTENSION: &str = "bin";

/// Why a mod data call failed.
#[derive(Debug)]
pub enum ModDataError {
    /// The key is not a mod id: lower case letters, digits, `_`, `-` and `.`, 1 to 64 characters, not starting with `.`.
    BadKey(String),
    /// The body is longer than [`MAX_BODY`].
    TooLarge(usize),
    /// The blob fails the digest, the magic, the version or the length check. It is left in place.
    Corrupt,
    /// The file could not be read, written or deleted.
    Io(io::Error, PathBuf),
}

impl std::fmt::Display for ModDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModDataError::BadKey(key) => write!(f, "{key:?} is not a mod id"),
            ModDataError::TooLarge(len) => write!(f, "a mod data body of {len} bytes is over {MAX_BODY}"),
            ModDataError::Corrupt => write!(f, "the mod data blob is corrupt"),
            ModDataError::Io(source, path) => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ModDataError {}

/// The mod data folder of one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModData {
    dir: PathBuf,
}

impl ModData {
    /// The blobs in `dir`. Nothing is created until a write.
    pub fn new(dir: impl Into<PathBuf>) -> ModData {
        ModData { dir: dir.into() }
    }

    /// The body of the blob of `key`; `None` when the mod has not written one.
    pub fn read(&self, key: &str) -> Result<Option<Vec<u8>>, ModDataError> {
        let path = self.path(key)?;
        let file = match fs::read(&path) {
            Ok(file) => file,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(ModDataError::Io(source, path)),
        };
        Ok(Some(decode(&file)?.to_vec()))
    }

    /// Replaces the blob of `key` with `body`.
    pub fn write(&self, key: &str, body: &[u8]) -> Result<(), ModDataError> {
        let path = self.path(key)?;
        if body.len() > MAX_BODY {
            return Err(ModDataError::TooLarge(body.len()));
        }
        let file = digest::with_prefix(&encode(body));
        fs::create_dir_all(&self.dir).map_err(|source| ModDataError::Io(source, self.dir.clone()))?;
        let temp = path.with_extension(format!("{EXTENSION}.tmp"));
        fs::write(&temp, &file).map_err(|source| ModDataError::Io(source, temp.clone()))?;
        fs::rename(&temp, &path).map_err(|source| ModDataError::Io(source, path))
    }

    /// Deletes the blob of `key`. A blob that does not exist is not an error.
    pub fn remove(&self, key: &str) -> Result<(), ModDataError> {
        let path = self.path(key)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ModDataError::Io(source, path)),
        }
    }

    /// The file of `key`, after the key is checked. The key is never used as a path before that.
    fn path(&self, key: &str) -> Result<PathBuf, ModDataError> {
        check_key(key)?;
        Ok(self.dir.join(format!("{key}.{EXTENSION}")))
    }
}

/// Refuses a key that is not a mod id (`docs/specs/mod-save-data.md`, section 3).
fn check_key(key: &str) -> Result<(), ModDataError> {
    let length_ok = (1..=MAX_KEY).contains(&key.len());
    let characters_ok =
        key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'));
    if length_ok && characters_ok && !key.starts_with('.') {
        return Ok(());
    }
    Err(ModDataError::BadKey(key.to_owned()))
}

/// The body with its magic, version and length in front of it (the file without the digest).
fn encode(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + body.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Checks a blob and returns its body. Every failure is [`ModDataError::Corrupt`].
fn decode(file: &[u8]) -> Result<&[u8], ModDataError> {
    let body = digest::check_prefixed(file).map_err(|_| ModDataError::Corrupt)?;
    let (magic, rest) = body.split_first_chunk::<4>().ok_or(ModDataError::Corrupt)?;
    let (version, rest) = rest.split_first_chunk::<4>().ok_or(ModDataError::Corrupt)?;
    let (length, rest) = rest.split_first_chunk::<8>().ok_or(ModDataError::Corrupt)?;
    let version = u32::from_le_bytes(*version);
    let length = u64::from_le_bytes(*length);
    if magic != MAGIC || version != VERSION || length != rest.len() as u64 {
        return Err(ModDataError::Corrupt);
    }
    Ok(rest)
}

#[cfg(test)]
#[path = "mod_data_tests.rs"]
mod tests;
