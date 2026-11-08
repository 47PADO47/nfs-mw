//! `Content`: tell the host that the content in the mod's folder has changed, and make some of it only when the game reads
//! it (catalogue: docs/modding/api-content.md).

use super::ModResult;
use super::ctx::Keep;
use super::ffi::content::{self as host_content, ContentFns};

/// The maker of a mod's **lazy files**: files of the mod's `files/` layer that are small stubs on disk and are made when
/// the game reads them (`Content::provide_lazy_files`). `key` is the text the mod wrote in the stub.
///
/// The game calls [`LazyFiles::read`] on **whatever thread opens the file**, several at once, and never inside a mod callback:
/// the maker may not call the host's functions (`Ctx` is not available to it), and it must be safe to call from many threads.
/// It is called every time the file is read, so a maker that is slow keeps what it made (a few recent results) itself.
pub trait LazyFiles: Send + Sync + 'static {
    /// The whole contents of the lazy file `key`. An error (or a panic, which becomes one) is an unreadable file.
    fn read(&self, key: &str) -> ModResult<Vec<u8>>;
}

/// The `bb.content` interface of the host, as a mod uses it. Obtained from `Ctx::content`.
#[derive(Clone)]
pub struct Content {
    fns: ContentFns,
    keep: Keep,
}

impl Content {
    pub(crate) fn new(fns: ContentFns, keep: Keep) -> Self {
        Content { fns, keep }
    }

    /// Have the host read the mod's folder again and add what is new in it: the `files/` layer and the `cars/`
    /// manifests. A mod that generates its content on a worker thread calls this, from the thread that receives its
    /// events, once the files are in place; the content is then there for everything the game opens afterwards.
    pub fn rescan(&self) -> ModResult<()> {
        host_content::rescan(&self.fns)
    }

    /// Whether the host makes the mod's lazy files (`bb.content` 1.1). A mod that writes stubs checks it first, and writes
    /// the real files when it is `false`.
    pub fn supports_lazy_files(&self) -> bool {
        host_content::supports_lazy_files(&self.fns)
    }

    /// Say who makes the mod's lazy files. A file of the mod's `files/` layer that holds a stub (the text `BBLAZY1`, the
    /// mod's id and a key, one per line; see the catalogue) is read through `files` with that key, whenever the game opens
    /// it. Call it in `Mod::load`, before the game reads any file of the mod: the call is kept until the mod unloads, and a
    /// second call replaces the first. `Unsupported` when the host is older than 1.1.
    pub fn provide_lazy_files(&self, files: impl LazyFiles) -> ModResult<()> {
        host_content::provide_lazy_files(&self.fns, &self.keep, Box::new(files))
    }
}
