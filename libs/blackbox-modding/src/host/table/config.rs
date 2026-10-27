//! `bb.config` on the host side: the backend the game implements, and the trampolines behind `BbConfig`.
//!
//! The host reads the mod's own file and merges it with the table the game's backend supplies ([`crate::host::config`]);
//! the game only says what its main config holds for the mod. Both sources are read when the mod calls, so a mod that
//! calls at load sees the files as they are then, and a call after the player edited a file sees the edit.

use core::ffi::c_void;
use core::mem::size_of;
use std::path::PathBuf;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, emit_text, guard, slot_of};
use crate::abi::{
    BB_CONFIG_FILE, BB_CONFIG_MAIN, BB_ERR_ARG, BB_ERR_HOST, BbConfig, BbIface, BbSinkFn, BbStatus, IFACE_CONFIG,
    IFACE_CONFIG_VERSION,
};
use crate::host::config::{self, Effective};

/// What the game implements for `bb.config`: its half of a mod's configuration. `owner` is the id of the mod that made the
/// call. The mod's own file is the host's business.
pub trait ConfigBackend {
    /// The table the game's main config file keeps for `owner` (NFS:MW: `[mods.<id>]`), as TOML text holding the table's
    /// own keys. `None` when the file has no such table. A main config that cannot be read counts as having none.
    fn main_section(&self, owner: &str) -> BackendResult<Option<String>>;
}

/// Where the calling mod's own file is, and the game's backend.
struct Source {
    backend: Rc<dyn ConfigBackend>,
    /// The mod's folder.
    dir: PathBuf,
    /// The manifest's `config` value, or `None` for `config.toml`.
    file: Option<String>,
}

impl Source {
    /// The merged configuration of `owner`. A failure is logged with the mod's id and becomes `BB_ERR_HOST`.
    fn effective(&self, owner: &str) -> Result<Effective, BbStatus> {
        let main = self.backend.main_section(owner)?;
        config::load(&self.dir, self.file.as_deref(), main.as_deref()).map_err(|error| failed(owner, &error))
    }

    /// The `BB_CONFIG_*` bits of the sources `owner` has. Nothing is parsed, so a source that is not valid TOML still counts.
    fn sources(&self, owner: &str) -> Result<u32, BbStatus> {
        let main = self.backend.main_section(owner)?;
        let file = config::resolve_file(&self.dir, self.file.as_deref()).map_err(|error| failed(owner, &error))?;
        let mut bits = 0;
        if file.is_some() {
            bits |= BB_CONFIG_FILE;
        }
        if main.is_some() {
            bits |= BB_CONFIG_MAIN;
        }
        Ok(bits)
    }
}

/// Log why a mod's configuration is not read; the mod sees `BB_ERR_HOST`.
fn failed(owner: &str, error: &config::ConfigError) -> BbStatus {
    log::warn!("mod {owner}: its configuration is not read: {error}");
    BB_ERR_HOST
}

/// Build the `bb.config` table for `owner`, whose folder is `dir` and whose own file is `file` (`None` for `config.toml`),
/// backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn ConfigBackend>, owner: &str, dir: PathBuf, file: Option<String>) -> Attached {
    let slot = Slot::new(Rc::new(Source { backend, dir, file }), owner);
    let table = Box::new(BbConfig {
        head: BbIface {
            size: size_of::<BbConfig>() as u32,
            major: IFACE_CONFIG_VERSION.0,
            minor: IFACE_CONFIG_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        read: Some(read),
        sources: Some(sources),
    });
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_CONFIG, IFACE_CONFIG_VERSION, table, release)
}

/// `read`: the merged text goes to the sink.
unsafe extern "C" fn read(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<Source>(ctx)? };
        let merged = slot.backend.effective(&slot.owner)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &merged.text) }
    })
}

/// `sources`: the bits of the sources that exist go to `out`.
unsafe extern "C" fn sources(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<Source>(ctx)? };
        if out.is_null() {
            return Err(BB_ERR_ARG);
        }
        let bits = slot.backend.sources(&slot.owner)?;
        // SAFETY: `out` is non-null (checked above) and valid for one write.
        unsafe { out.write(bits) };
        Ok(())
    })
}
