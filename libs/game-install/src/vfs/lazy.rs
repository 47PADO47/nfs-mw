//! Lazy files: files of a layer whose bytes are made when they are read.
//!
//! A mod that can produce a file on demand (a car converted from another game's files the first time the game opens it)
//! writes a small **stub** in its layer in place of the file. The stub is a real file, so the layer indexes it like any
//! other: it exists, lists, resolves and has a provider. [`super::GameDir::read`] recognises a stub and asks the
//! [`LazySource`] the stub names for the whole contents. Nothing else changes: a caller that reads the file as bytes gets
//! the real contents, and none of them knows the file was lazy.
//!
//! A stub is text of at most [`MAX_STUB_BYTES`] bytes: the line `BBLAZY1`, the owner (the id the source was added under)
//! and the key the source understands, each on its own line. [`stub`] writes one.
//!
//! What a stub does not do: [`super::GameDir::open_file`] and [`super::GameDir::read_range`] open the stub's own bytes,
//! and [`super::GameDir::size`] is the stub's size, so a lazy file is for callers that read the whole file with `read`.
//! A file that starts like a stub but is longer than the limit is a plain file.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::sync::{Arc, PoisonError, RwLock};

/// The first line of a stub.
const MAGIC: &str = "BBLAZY1";
/// The longest stub, in bytes. A bigger file is never taken for one.
pub const MAX_STUB_BYTES: usize = 1024;

/// Whatever makes lazy files for one owner. Called on whatever thread reads the file, possibly from several at once.
pub trait LazySource: Send + Sync {
    /// The whole contents of the file the stub named `key` stands for.
    fn read(&self, key: &str) -> io::Result<Vec<u8>>;
}

/// The stub that stands for the file `key` of the source added under `owner`. `owner` and `key` are one line each: a line
/// break in either makes a stub that does not parse, which is read as the plain file it is.
pub fn stub(owner: &str, key: &str) -> Vec<u8> {
    format!("{MAGIC}\n{owner}\n{key}\n").into_bytes()
}

/// The owner and the key of a stub, when `bytes` is one.
pub(super) fn parse(bytes: &[u8]) -> Option<(&str, &str)> {
    if bytes.len() > MAX_STUB_BYTES {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    if lines.next()? != MAGIC {
        return None;
    }
    let owner = lines.next()?;
    let key = lines.next()?;
    if owner.is_empty() || key.is_empty() || lines.next().is_some() {
        return None;
    }
    Some((owner, key))
}

/// The sources of one install, by owner. Shared by every clone of the directory, like the late layers.
#[derive(Default)]
pub(super) struct LazySources {
    sources: RwLock<HashMap<String, Arc<dyn LazySource>>>,
}

impl fmt::Debug for LazySources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sources = self.sources.read().unwrap_or_else(PoisonError::into_inner);
        f.debug_struct("LazySources").field("owners", &sources.keys().collect::<Vec<_>>()).finish()
    }
}

impl LazySources {
    /// Add `source` for `owner`, replacing the one it had.
    pub(super) fn add(&self, owner: &str, source: Arc<dyn LazySource>) {
        self.sources.write().unwrap_or_else(PoisonError::into_inner).insert(owner.to_owned(), source);
    }

    /// Ask the source of `owner` for the file `key`.
    pub(super) fn read(&self, owner: &str, key: &str) -> io::Result<Vec<u8>> {
        // The lock is not held while the source works: it may take long, and may read other files.
        let source = self.sources.read().unwrap_or_else(PoisonError::into_inner).get(owner).cloned();
        let Some(source) = source else {
            let reason = format!("the lazy file {key} of `{owner}` has no source: the mod that makes it is not loaded");
            return Err(io::Error::new(io::ErrorKind::NotFound, reason));
        };
        source.read(key)
    }
}
