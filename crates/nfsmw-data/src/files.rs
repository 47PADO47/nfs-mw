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
//! Threads asking for the same entry at once build it once: the others wait for that build and share its result.
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
/// `build` runs without the cache lock held, so it may call `memo` for another entry itself. Threads that ask for
/// the same entry while it is being built wait for that build instead of building it again (other entries are not
/// held up). If the build fails, its caller gets the error and a waiting thread builds the entry itself.
pub fn memo<T: Send + Sync + 'static>(
    dir: &GameDir,
    files: &[&str],
    build: impl FnOnce() -> Result<T>,
) -> Result<Arc<T>> {
    let key = Key { type_id: TypeId::of::<T>(), files: files.iter().map(|rel| stamp(dir, rel)).collect() };
    // Consumed by the one build this call runs, if it runs one.
    let mut pending = Some(build);
    loop {
        let slot = slot_for(&key);
        let mut failure = None;
        // The lock is not held here: only threads that want this very entry wait on the slot.
        let built = slot.get_or_init(|| {
            let build = pending.take()?;
            match build() {
                Ok(value) => Some(Arc::new(value) as Arc<dyn Any + Send + Sync>),
                Err(e) => {
                    failure = Some(e);
                    None
                }
            }
        });
        if let Some(entry) = built {
            return downcast(Arc::clone(entry));
        }
        // The build failed, here or on the thread this one waited for. Drop the failed slot so that a later call
        // starts afresh; a waiting thread goes round again and runs its own build.
        forget(&key, &slot);
        if let Some(e) = failure {
            return Err(e);
        }
    }
}

/// What the memo key records about one file: its resolved path, its size and its modification time.
#[derive(Clone, PartialEq, Eq, Hash)]
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
#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    type_id: TypeId,
    files: Vec<FileStamp>,
}

/// The value of one entry once it is built, shared by every thread that asks for it. `None` means the build failed.
type Built = OnceLock<Option<Arc<dyn Any + Send + Sync>>>;

/// Entries by key, each an `Arc<T>` erased to `Any` once built. The key's type id makes [`downcast`] succeed.
type Entries = HashMap<Key, Arc<Built>>;

/// The session cache. Built entries are never removed from it.
static CACHE: OnceLock<Mutex<Entries>> = OnceLock::new();

/// The locked cache. A poisoned lock is used as it is: a panic while it was held cannot leave the map half-written.
fn entries() -> MutexGuard<'static, Entries> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner())
}

/// The slot of `key`, added empty when it is new. Building into it happens outside the lock.
fn slot_for(key: &Key) -> Arc<Built> {
    let mut map = entries();
    if let Some(slot) = map.get(key) {
        return Arc::clone(slot);
    }
    let slot = Arc::new(Built::new());
    map.insert(key.clone(), Arc::clone(&slot));
    slot
}

/// Remove `slot` from the cache if it is still the one stored for `key`: it is a failed build.
fn forget(key: &Key, slot: &Arc<Built>) {
    let mut map = entries();
    if map.get(key).is_some_and(|stored| Arc::ptr_eq(stored, slot)) {
        map.remove(key);
    }
}

/// The typed `Arc` of a cache entry. The key's type id fixes the type, so the error only guards a broken key.
fn downcast<T: Send + Sync + 'static>(entry: Arc<dyn Any + Send + Sync>) -> Result<Arc<T>> {
    entry.downcast::<T>().map_err(|_| anyhow::anyhow!("session memo entry has another type"))
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
