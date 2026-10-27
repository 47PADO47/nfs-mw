//! `Content`: tell the host that the content in the mod's folder has changed (catalogue: docs/modding/api-content.md).

use super::ModResult;
use super::ffi::content::{self as host_content, ContentFns};

/// The `bb.content` interface of the host, as a mod uses it. Obtained from `Ctx::content`.
#[derive(Clone, Copy)]
pub struct Content {
    fns: ContentFns,
}

impl Content {
    pub(crate) fn new(fns: ContentFns) -> Self {
        Content { fns }
    }

    /// Have the host read the mod's folder again and add what is new in it: the `files/` layer and the `cars/`
    /// manifests. A mod that generates its content on a worker thread calls this, from the thread that receives its
    /// events, once the files are in place; the content is then there for everything the game opens afterwards.
    pub fn rescan(&self) -> ModResult<()> {
        host_content::rescan(&self.fns)
    }
}
