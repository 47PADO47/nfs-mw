//! `Saves`: the mod's own blob in the career profile it is loaded with (docs/specs/mod-save-data.md). Obtained from
//! `Ctx::saves`.

use super::ModResult;
use super::ffi::saves::{self as host_saves, SavesFns};

/// The `bb.saves` interface of the host, as a mod uses it. Only this mod's blob is reachable.
pub struct Saves {
    fns: SavesFns,
}

impl Saves {
    pub(crate) fn new(fns: SavesFns) -> Self {
        Saves { fns }
    }

    /// The blob this mod wrote for the loaded profile, or `None` when it has written none. A write made since the last
    /// profile save is read back as written. A stored blob that fails its check is `Error::Corrupt`, and it stays in
    /// place until the mod writes a new one.
    pub fn read(&self) -> ModResult<Option<Vec<u8>>> {
        host_saves::read_blob(&self.fns)
    }

    /// Replace the blob with `body`, at most 256 KiB long (`MAX_BLOB_BYTES` in the ABI). The new blob is written with the next profile
    /// save, before the profile itself, so the two never disagree.
    pub fn write(&self, body: &[u8]) -> ModResult<()> {
        host_saves::write_blob(&self.fns, body)
    }

    /// Delete the blob with the next profile save. A blob that does not exist is not an error.
    pub fn remove(&self) -> ModResult<()> {
        host_saves::remove_blob(&self.fns)
    }
}
