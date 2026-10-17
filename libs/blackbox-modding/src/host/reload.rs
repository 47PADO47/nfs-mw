//! Shadow copies for developer runs (ADR 0005 section 3, "Unload and hot reload").
//!
//! Windows locks a loaded DLL, so a rebuild cannot overwrite the library the game has open. A developer run therefore
//! loads a copy: the library is copied to `<cache>/<mod id>/<serial>-<file name>` and the copy is loaded. The cache is
//! purged at the next start.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Copy `source` into the cache for mod `id`, under a name that `serial` makes unique. Returns the copy's path.
pub fn shadow_copy(source: &Path, cache_dir: &Path, id: &str, serial: u32) -> io::Result<PathBuf> {
    let Some(name) = source.file_name() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the library path has no file name"));
    };
    let folder = cache_dir.join(id);
    fs::create_dir_all(&folder)?;
    let target = folder.join(format!("{serial}-{}", name.to_string_lossy()));
    fs::copy(source, &target)?;
    Ok(target)
}

/// Remove the whole cache. A missing cache is not an error. A library still locked by another process fails here, and
/// the caller reports it.
pub fn purge(cache_dir: &Path) -> io::Result<()> {
    if !cache_dir.exists() {
        return Ok(());
    }
    fs::remove_dir_all(cache_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::test_support::Scratch;

    #[test]
    fn a_shadow_copy_holds_the_same_bytes_under_a_unique_name() {
        let scratch = Scratch::new("shadow");
        let source = scratch.write("hello.lib", b"library bytes");
        let cache = scratch.path().join("cache");
        let first = shadow_copy(&source, &cache, "hello.mod", 1).expect("first copy");
        let second = shadow_copy(&source, &cache, "hello.mod", 2).expect("second copy");
        assert_ne!(first, second);
        assert_eq!(fs::read(&first).expect("read first"), b"library bytes");
        assert_eq!(first.parent(), Some(cache.join("hello.mod").as_path()));
    }

    #[test]
    fn purge_removes_the_cache_and_a_missing_cache_is_fine() {
        let scratch = Scratch::new("purge");
        let cache = scratch.path().join("cache");
        assert!(purge(&cache).is_ok());
        let source = scratch.write("lib.bin", b"x");
        shadow_copy(&source, &cache, "a.mod", 1).expect("copy");
        assert!(purge(&cache).is_ok());
        assert!(!cache.exists());
    }
}
