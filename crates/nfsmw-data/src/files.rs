//! Reads of install files, and a session-wide memo for data derived from them.
//!
//! [`memo`] keeps a built value (a parsed table, a decoded file) for the rest of the process,
//! so the same file is not read, decompressed and parsed again on every car load. An entry is
//! keyed by the result type and, for each file it was built from, the file's resolved real path,
//! size and modification time. A mod overlay that replaces a file resolves to a different path,
//! and an edited file changes its size or mtime, so either one misses the cache and rebuilds.
//! Where the file system has no modification times only the size tells an edit apart, so a
//! same-size edit can go unseen there.
//!
//! Entries are never evicted: this is for session-global tables, a few tens of MB at most.
//! A failed build is not stored, so the next call tries again.

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::SystemTime;

use anyhow::{Context, Result};
use game_install::GameDir;

/// Read a whole file from the install and remove any compression wrapper.
///
/// A file without a wrapper is returned as it was read, without a copy.
pub fn read_unwrapped(dir: &GameDir, rel: &str) -> Result<Vec<u8>> {
    let raw = dir.read(rel).with_context(|| format!("reading {rel}"))?;
    // `None` when there is no wrapper: the read buffer already holds the contents.
    let decoded = match ea_compress::unwrap(&raw).with_context(|| format!("decompressing {rel}"))? {
        Cow::Owned(data) => Some(data),
        Cow::Borrowed(body) if body.as_ptr() == raw.as_ptr() && body.len() == raw.len() => None,
        // A wrapped payload (RAWW) is a window into the read buffer, past its header.
        Cow::Borrowed(body) => Some(body.to_vec()),
    };
    Ok(decoded.unwrap_or(raw))
}

/// `rel` read and unwrapped once per session: the bytes are shared while the file is unchanged.
pub fn shared_unwrapped(dir: &GameDir, rel: &str) -> Result<Arc<Vec<u8>>> {
    let hit = memo(dir, &[rel], || Ok(Unwrapped(Arc::new(read_unwrapped(dir, rel)?))))?;
    Ok(Arc::clone(&hit.0))
}

/// The memo entry type of [`shared_unwrapped`]. Its own type keeps those entries apart from any
/// other memo of a `Vec<u8>` for the same files.
struct Unwrapped(Arc<Vec<u8>>);

/// `build`'s result, made once per session for the current contents of `files`: later calls with the same type and
/// the same files (same resolved path, size and modification time) return the same `Arc` without calling `build`.
///
/// Calls with the same type and file list share one entry, so give each call site its own private type.
/// `build` runs without the cache lock held, so it may call `memo` itself. If two threads race, the first
/// result stored is returned to both.
pub fn memo<T: Send + Sync + 'static>(
    dir: &GameDir,
    files: &[&str],
    build: impl FnOnce() -> Result<T>,
) -> Result<Arc<T>> {
    let key = Key { type_id: TypeId::of::<T>(), files: files.iter().map(|rel| stamp(dir, rel)).collect() };
    let cached = entries().get(&key).cloned();
    if let Some(entry) = cached {
        return downcast(entry);
    }
    let built: Arc<dyn Any + Send + Sync> = Arc::new(build()?);
    let entry = entries().entry(key).or_insert(built).clone();
    downcast(entry)
}

/// What the memo key records about one file: its resolved path, its size and its modification time.
#[derive(PartialEq, Eq, Hash)]
struct FileStamp {
    path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
}

/// The stamp of `rel`. A file that is not in the install gets the bare path, so its build fails and nothing is stored.
fn stamp(dir: &GameDir, rel: &str) -> FileStamp {
    let Some(path) = dir.resolve(rel) else {
        return FileStamp { path: PathBuf::from(rel), len: 0, modified: None };
    };
    let (len, modified) = match std::fs::metadata(path) {
        Ok(meta) => (meta.len(), meta.modified().ok()),
        Err(_) => (0, None),
    };
    FileStamp { path: path.to_path_buf(), len, modified }
}

/// The memo key: the result type and the stamps of the files it was built from, in order.
#[derive(PartialEq, Eq, Hash)]
struct Key {
    type_id: TypeId,
    files: Vec<FileStamp>,
}

/// Built values by key, each an `Arc<T>` erased to `Any`. The key's type id makes [`downcast`] succeed.
type Entries = HashMap<Key, Arc<dyn Any + Send + Sync>>;

/// The session cache. Nothing is ever removed from it.
static CACHE: OnceLock<Mutex<Entries>> = OnceLock::new();

/// The locked cache. A poisoned lock is used as it is: a panic while it was held cannot leave the map half-written.
fn entries() -> MutexGuard<'static, Entries> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner())
}

/// The typed `Arc` of a cache entry. The key's type id fixes the type, so the error only guards a broken key.
fn downcast<T: Send + Sync + 'static>(entry: Arc<dyn Any + Send + Sync>) -> Result<Arc<T>> {
    entry.downcast::<T>().map_err(|_| anyhow::anyhow!("session memo entry has another type"))
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
