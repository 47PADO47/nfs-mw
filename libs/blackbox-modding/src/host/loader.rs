//! Opening one mod library and calling into it (ADR 0005 section 3).
//!
//! The order is fixed: read the library's bytes (for the SHA-256), open it, read `bb_mod_info` and refuse a major the
//! host does not speak, only then build the host table and call `bb_mod_create`. A `Module` owns the library, the host
//! table and the callbacks; dropping it runs `destroy` (once) and then closes the library.

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to `abi`, `host::table`, `host::loader` and `sdk::ffi`.

use core::ffi::c_void;
use core::mem::{MaybeUninit, offset_of, size_of};
use core::ptr::addr_of;
use std::fs;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};
use sha2::{Digest, Sha256};

use crate::abi::{
    ABI_VERSION, AbiVersion, BB_OK, BbDestroyFn, BbEvent, BbEventResult, BbHost, BbModApi, BbModCreateFn, BbModInfo,
    BbModInfoFn, BbOnEventFn, BbStatus, MOD_CREATE_SYMBOL, MOD_INFO_SYMBOL, check_handshake, has_field,
};

use super::offer::HostOffer;
use super::table::HostTable;

/// Why a library could not be loaded. Each variant ends the load of that mod only.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The library file could not be read for hashing.
    #[error("cannot read the library: {0}")]
    Io(String),
    /// The OS refused to open the library (missing dependency, wrong architecture, not a library).
    #[error("cannot open the library: {0}")]
    Open(String),
    /// The library lacks one of the two entry symbols.
    #[error("the library has no {0} symbol")]
    MissingSymbol(&'static str),
    /// `bb_mod_info` returned a null pointer.
    #[error("bb_mod_info returned a null pointer")]
    NullInfo,
    /// A table is too short to hold a field the reader needs.
    #[error("the {0} table is too short")]
    ShortTable(&'static str),
    /// The mod speaks another ABI major, or a newer minor than the host.
    #[error("the mod is built for ABI {module}, this host speaks {host}")]
    Handshake { module: String, host: String },
    /// `bb_mod_create` returned an error status.
    #[error("bb_mod_create returned status {0}")]
    Create(BbStatus),
}

/// A mod library loaded into the process, with the callbacks its mod table gave.
///
/// Field order matters: the fields are dropped after [`Drop::drop`] has run `destroy`, in declaration order, so the
/// host table outlives the mod's state and the library is closed last.
pub struct Module {
    id: String,
    path: PathBuf,
    sha256: String,
    state: *mut c_void,
    on_event: Option<BbOnEventFn>,
    destroy: Option<BbDestroyFn>,
    destroyed: bool,
    _host: HostTable,
    _library: Library,
}

impl Module {
    /// Load the library at `path` for mod `id`. The library is opened only after its bytes are hashed, and its ABI is
    /// checked before the host table is built.
    pub fn load(path: &Path, id: &str, offer: &HostOffer) -> Result<Module, LoadError> {
        let bytes = fs::read(path).map_err(|error| LoadError::Io(error.to_string()))?;
        let sha256 = sha256_hex(&bytes);
        // SAFETY: opening a library runs its initialisers. The library is a mod the user put in the mods folder, and
        // ADR 0005 section 6 treats such mods as trusted code with the user's privileges.
        let library = unsafe { Library::new(path) }.map_err(|error| LoadError::Open(error.to_string()))?;
        let declared = read_info(&library)?;
        check_mod_abi(declared)?;
        let host = HostTable::new(id, offer);
        let callbacks = call_create(&library, host.as_ptr())?;
        Ok(Module {
            id: id.to_owned(),
            path: path.to_owned(),
            sha256,
            state: callbacks.state,
            on_event: callbacks.on_event,
            destroy: callbacks.destroy,
            destroyed: false,
            _host: host,
            _library: library,
        })
    }

    /// The mod id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The library file that was loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Lower-case hex SHA-256 of the library bytes that were loaded.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Deliver one event. `None` when the mod has no event callback or has already been destroyed; otherwise the
    /// status the mod returned. A negative status is a fault.
    pub fn deliver(&mut self, event: &BbEvent) -> Option<BbEventResult> {
        if self.destroyed {
            return None;
        }
        let on_event = self.on_event?;
        // SAFETY: `state` is the pointer `bb_mod_create` returned, and `destroy` has not run (checked above). `event` is
        // valid for the call. The host does not call into the mod from another thread (ADR 0005 section 4).
        Some(unsafe { on_event(self.state, event) })
    }

    /// Run the mod's `destroy`, once. Called by `Drop`; the runtime calls it earlier to unload in order.
    pub fn destroy_now(&mut self) {
        if self.destroyed {
            return;
        }
        self.destroyed = true;
        let Some(destroy) = self.destroy else {
            log::warn!("mod {} has no destroy callback; its state is leaked", self.id);
            return;
        };
        // SAFETY: `state` came from `bb_mod_create` and this is the only call that passes it to `destroy` (the
        // `destroyed` flag guards a second call).
        unsafe { destroy(self.state) };
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        self.destroy_now();
    }
}

/// The callbacks a mod table holds.
struct Callbacks {
    state: *mut c_void,
    on_event: Option<BbOnEventFn>,
    destroy: Option<BbDestroyFn>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Call `bb_mod_info` and read the ABI version it declares.
fn read_info(library: &Library) -> Result<AbiVersion, LoadError> {
    // SAFETY: the symbol is the mod's `bb_mod_info`, which the SDK exports with the signature `BbModInfoFn`.
    let symbol: Symbol<BbModInfoFn> =
        unsafe { library.get(MOD_INFO_SYMBOL.as_bytes()) }.map_err(|_| LoadError::MissingSymbol(MOD_INFO_SYMBOL))?;
    let info_fn: BbModInfoFn = *symbol;
    let info: *const BbModInfo = info_fn();
    if info.is_null() {
        return Err(LoadError::NullInfo);
    }
    // SAFETY: a non-null pointer from `bb_mod_info` points to a static `BbModInfo` that the mod keeps for its lifetime.
    unsafe { read_version(info) }
}

/// Read the version fields of a `BbModInfo`, each only when its `size` covers it.
///
/// # Safety
/// `info` must be non-null and point to a readable `BbModInfo` of the size its own `size` field declares.
unsafe fn read_version(info: *const BbModInfo) -> Result<AbiVersion, LoadError> {
    // SAFETY: the size word is the first field of every version of the table.
    let size = unsafe { addr_of!((*info).size).read() };
    if !has_field(size, offset_of!(BbModInfo, abi_minor) + size_of::<u16>()) {
        return Err(LoadError::ShortTable("bb_mod_info"));
    }
    // SAFETY: the table covers both version fields (checked above).
    let (major, minor) = unsafe { (addr_of!((*info).abi_major).read(), addr_of!((*info).abi_minor).read()) };
    Ok(AbiVersion { major, minor })
}

/// The handshake of ADR 0005 section 7, checked before the host table is built.
fn check_mod_abi(declared: AbiVersion) -> Result<(), LoadError> {
    if check_handshake(ABI_VERSION, declared).is_ok() {
        return Ok(());
    }
    Err(LoadError::Handshake { module: version_text(declared), host: version_text(ABI_VERSION) })
}

fn version_text(version: AbiVersion) -> String {
    format!("{}.{}", version.major, version.minor)
}

/// Call `bb_mod_create` with the host table and read back the mod table.
fn call_create(library: &Library, host: *const BbHost) -> Result<Callbacks, LoadError> {
    // SAFETY: the symbol is the mod's `bb_mod_create`, which the SDK exports with the signature `BbModCreateFn`.
    let symbol: Symbol<BbModCreateFn> = unsafe { library.get(MOD_CREATE_SYMBOL.as_bytes()) }
        .map_err(|_| LoadError::MissingSymbol(MOD_CREATE_SYMBOL))?;
    let create: BbModCreateFn = *symbol;
    let mut out = MaybeUninit::<BbModApi>::uninit();
    // SAFETY: `host` is the table the caller keeps alive while the mod is loaded. `out` is valid for one write of a
    // `BbModApi`. The handshake ran first, so a mod built for a newer minor (which could write a larger table) was
    // refused before this call.
    let status = unsafe { create(host, out.as_mut_ptr()) };
    if status != BB_OK {
        return Err(LoadError::Create(status));
    }
    // SAFETY: the mod reported success, so it wrote its table; only the fields its `size` covers are read.
    unsafe { read_callbacks(out.as_ptr()) }
}

/// Read the callbacks of a mod table the mod has written.
///
/// # Safety
/// `table` must point to a `BbModApi` that the mod wrote in `bb_mod_create`.
unsafe fn read_callbacks(table: *const BbModApi) -> Result<Callbacks, LoadError> {
    // SAFETY: the size word is the first field of every version of the table, and the mod wrote it.
    let size = unsafe { addr_of!((*table).size).read() };
    if !has_field(size, offset_of!(BbModApi, state) + size_of::<*mut c_void>()) {
        return Err(LoadError::ShortTable("mod"));
    }
    // SAFETY: the table covers `state` (checked above).
    let state = unsafe { addr_of!((*table).state).read() };
    // SAFETY: `table` and `size` are as checked above.
    let on_event = unsafe { read_on_event(table, size) };
    // SAFETY: `table` and `size` are as checked above.
    let destroy = unsafe { read_destroy(table, size) };
    Ok(Callbacks { state, on_event, destroy })
}

/// # Safety
/// `table` must point to a mod table whose first `size` bytes the mod wrote.
unsafe fn read_on_event(table: *const BbModApi, size: u32) -> Option<BbOnEventFn> {
    if !has_field(size, offset_of!(BbModApi, on_event) + size_of::<Option<BbOnEventFn>>()) {
        return None;
    }
    // SAFETY: the table covers `on_event` (checked above).
    unsafe { addr_of!((*table).on_event).read() }
}

/// # Safety
/// `table` must point to a mod table whose first `size` bytes the mod wrote.
unsafe fn read_destroy(table: *const BbModApi, size: u32) -> Option<BbDestroyFn> {
    if !has_field(size, offset_of!(BbModApi, destroy) + size_of::<Option<BbDestroyFn>>()) {
        return None;
    }
    // SAFETY: the table covers `destroy` (checked above).
    unsafe { addr_of!((*table).destroy).read() }
}
