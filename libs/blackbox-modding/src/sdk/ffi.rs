//! The only SDK code that handles raw pointers or calls host function pointers (ADR 0005 sections 1 and 3).
//!
//! The trust boundary is the host table. Its fields are read only after the size check, and its function pointers
//! are called only while the mod is loaded. Each interface table is read the same way (see [`read_or`]). Everything a
//! mod sees is safe Rust built on top of this module.

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to `abi`, `host::loader` and `sdk::ffi`.

pub(crate) mod audio;
pub(crate) mod camera;
pub(crate) mod commands;
pub(crate) mod content;
mod event;
pub(crate) mod hud;
pub(crate) mod i18n;
pub(crate) mod input;
pub(crate) mod map;
pub(crate) mod menus;
#[cfg(all(test, feature = "host"))]
mod menus_tests;
pub(crate) mod time;
pub(crate) mod ui;
#[cfg(all(test, feature = "host"))]
mod ui_tests;
pub(crate) mod vehicles;
pub(crate) mod world;

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::{self, NonNull, addr_of};
use std::cell::RefCell;
use std::rc::Rc;

use super::ctx::Keep;
use super::{Ctx, Error, Interface, Mod, ModResult, guard};
use crate::abi::{
    ABI_MAJOR, ABI_MINOR, ABI_VERSION, AbiVersion, BB_ERR_ABI, BB_ERR_ARG, BB_EVENT_CONTINUE, BB_LOG_ERROR, BB_OK,
    BbEvent, BbEventResult, BbHost, BbIface, BbLogFn, BbModApi, BbQueryInterfaceFn, BbStatus, BbStr, check_handshake,
    has_field,
};

/// The host table as the SDK reads it. The header is read at once. Every other field is read only when the host's
/// `size` covers it, because a host built for an older minor has a shorter table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HostTable {
    ptr: NonNull<BbHost>,
    size: u32,
    version: AbiVersion,
}

impl HostTable {
    /// Read the header of the host table at `ptr`.
    ///
    /// # Safety
    /// `ptr` must point to a host table that stays valid while the mod is loaded, and its first eight bytes (the
    /// header) must be readable.
    pub(crate) unsafe fn new(ptr: NonNull<BbHost>) -> Self {
        let raw = ptr.as_ptr();
        // SAFETY: the header is present in every version of the table, and the caller keeps it valid.
        let (size, major, minor) = unsafe {
            (addr_of!((*raw).size).read(), addr_of!((*raw).abi_major).read(), addr_of!((*raw).abi_minor).read())
        };
        HostTable { ptr, size, version: AbiVersion { major, minor } }
    }

    /// The ABI version the host reports in its header.
    pub(crate) fn version(&self) -> AbiVersion {
        self.version
    }

    /// Whether the host's table is large enough to hold a field that ends at byte `field_end`.
    fn covers(&self, field_end: usize) -> bool {
        has_field(self.size, field_end)
    }

    fn context(&self) -> *mut c_void {
        if !self.covers(offset_of!(BbHost, ctx) + size_of::<*mut c_void>()) {
            return ptr::null_mut();
        }
        // SAFETY: the table covers this field (checked above), and the host stores a pointer here.
        unsafe { addr_of!((*self.ptr.as_ptr()).ctx).read() }
    }

    fn logger(&self) -> Option<BbLogFn> {
        if !self.covers(offset_of!(BbHost, log) + size_of::<Option<BbLogFn>>()) {
            return None;
        }
        // SAFETY: the table covers this field (checked above); the host stores a function pointer or null.
        unsafe { addr_of!((*self.ptr.as_ptr()).log).read() }
    }

    fn query_fn(&self) -> Option<BbQueryInterfaceFn> {
        if !self.covers(offset_of!(BbHost, query_interface) + size_of::<Option<BbQueryInterfaceFn>>()) {
            return None;
        }
        // SAFETY: the table covers this field (checked above); the host stores a function pointer or null.
        unsafe { addr_of!((*self.ptr.as_ptr()).query_interface).read() }
    }

    fn game_id_field(&self) -> Option<BbStr> {
        if !self.covers(offset_of!(BbHost, game_id) + size_of::<BbStr>()) {
            return None;
        }
        // SAFETY: the table covers this field (checked above); the string is valid while the mod is loaded.
        Some(unsafe { addr_of!((*self.ptr.as_ptr()).game_id).read() })
    }

    fn mod_dir_field(&self) -> Option<BbStr> {
        if !self.covers(offset_of!(BbHost, mod_dir) + size_of::<BbStr>()) {
            return None;
        }
        // SAFETY: the table covers this field (checked above); the string is valid while the mod is loaded.
        Some(unsafe { addr_of!((*self.ptr.as_ptr()).mod_dir).read() })
    }

    fn game_dir_field(&self) -> Option<BbStr> {
        if !self.covers(offset_of!(BbHost, game_dir) + size_of::<BbStr>()) {
            return None;
        }
        // SAFETY: the table covers this field (checked above); the string is valid while the mod is loaded.
        Some(unsafe { addr_of!((*self.ptr.as_ptr()).game_dir).read() })
    }

    fn game_version_field(&self) -> Option<BbStr> {
        if !self.covers(offset_of!(BbHost, game_version) + size_of::<BbStr>()) {
            return None;
        }
        // SAFETY: the table covers this field (checked above); the string is valid while the mod is loaded.
        Some(unsafe { addr_of!((*self.ptr.as_ptr()).game_version).read() })
    }
}

/// Read one field of an interface table, or `absent` when the table's size does not cover it.
///
/// # Safety
/// `iface` must be a table the host offered, which stays valid while the mod is loaded. The field at `offset` must be
/// of type `T` in that table's layout.
pub(crate) unsafe fn read_or<T: Copy>(iface: &Interface, offset: usize, absent: T) -> T {
    if !iface.has_field(offset + size_of::<T>()) {
        return absent;
    }
    // SAFETY: the table covers the field (checked above) and stays valid while the mod is loaded (see `# Safety`).
    unsafe { iface.as_ptr().cast::<u8>().add(offset).cast::<T>().read_unaligned() }
}

/// Turn a host status into the SDK's result: a non-negative status is success.
pub(crate) fn status_result(status: BbStatus) -> ModResult<()> {
    match Error::from_status(status) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Write one log line through the host. A host without a log function drops the line.
pub(crate) fn log(host: HostTable, level: u32, msg: &str) {
    let Some(logger) = host.logger() else {
        return;
    };
    // SAFETY: the host's log function takes the host's own context pointer and a string valid for the call. `msg`
    // outlives the call.
    unsafe { logger(host.context(), level, BbStr::new(msg)) }
}

/// The game id, copied out of the host table.
pub(crate) fn game_id(host: HostTable) -> ModResult<String> {
    owned_text(host.game_id_field())
}

/// The game version, copied out of the host table.
pub(crate) fn game_version(host: HostTable) -> ModResult<String> {
    owned_text(host.game_version_field())
}

/// The folder of the mod itself, copied out of the host table. `Unsupported` for a host that predates the field.
pub(crate) fn mod_dir(host: HostTable) -> ModResult<String> {
    owned_text(host.mod_dir_field())
}

/// The folder of the game's install, copied out of the host table. Empty when the host has none.
pub(crate) fn game_dir(host: HostTable) -> ModResult<String> {
    owned_text(host.game_dir_field())
}

fn owned_text(text: Option<BbStr>) -> ModResult<String> {
    let Some(text) = text else {
        return Err(Error::Unsupported);
    };
    // SAFETY: the host keeps its strings valid while the mod is loaded; the copy is taken inside this call.
    let borrowed = unsafe { text.as_str() };
    match borrowed {
        Some(text) => Ok(text.to_owned()),
        None => Err(Error::Host),
    }
}

/// Ask the host for an interface. The header of the returned table is checked against the request, so a host that
/// answers with a table of another version is treated as absent.
pub(crate) fn query_interface(host: HostTable, id: &str, major: u16, min_minor: u16) -> Option<Interface> {
    let query = host.query_fn()?;
    // SAFETY: the host's query function takes its own context pointer and an id string valid for the call. It returns
    // null or a table that starts with a `BbIface` header (ADR 0005 section 1).
    let table = unsafe { query(host.context(), BbStr::new(id), major, min_minor) };
    let table = NonNull::new(table.cast_mut())?;
    // SAFETY: a non-null table starts with its header, which is read by value.
    let header: BbIface = unsafe { ptr::read_unaligned(table.as_ptr()) };
    if header.major != major || header.minor < min_minor {
        return None;
    }
    Some(Interface::new(table, header))
}

/// What `BbModApi::state` points to: the mod, the host table its callbacks need, and the closures it keeps alive for
/// the host.
struct Slot<M> {
    host: HostTable,
    module: M,
    keep: Keep,
}

/// The body of `bb_mod_create` for the mod type `M`.
///
/// It checks the arguments and the handshake, then loads the mod inside the panic guard. It writes the mod table to
/// `out` only when the mod loaded.
///
/// # Safety
/// `host` must be null or point to a host table that stays valid while the mod is loaded. `out` must be null or
/// valid for one write of a `BbModApi`.
pub unsafe fn create<M: Mod>(host: *const BbHost, out: *mut BbModApi) -> BbStatus {
    let Some(host) = NonNull::new(host.cast_mut()) else {
        return BB_ERR_ARG;
    };
    if out.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: the caller guarantees the host table is valid (see `# Safety`).
    let host = unsafe { HostTable::new(host) };
    if check_handshake(host.version(), ABI_VERSION).is_err() {
        return BB_ERR_ABI;
    }
    let keep: Keep = Rc::new(RefCell::new(Vec::new()));
    let loaded = guard::run(|| M::load(&mut Ctx::new(host, keep.clone())));
    let module = match finish(host, "load", loaded) {
        Ok(module) => module,
        Err(error) => {
            // The closures of a refused mod are dropped now; the host has not called them, and it removes its
            // registrations when it drops this mod's table.
            keep.borrow_mut().clear();
            return error.code();
        }
    };
    let slot = Box::into_raw(Box::new(Slot { host, module, keep }));
    let table = BbModApi {
        size: size_of::<BbModApi>() as u32,
        abi_major: ABI_MAJOR,
        abi_minor: ABI_MINOR,
        state: slot.cast::<c_void>(),
        on_event: Some(on_event::<M>),
        destroy: Some(destroy::<M>),
    };
    // SAFETY: `out` is non-null and valid for one write (see `# Safety`).
    unsafe { out.write(table) };
    BB_OK
}

/// The `on_event` trampoline for the mod type `M`.
///
/// # Safety
/// `state` must be the pointer `create::<M>` produced and not yet passed to `destroy`. `event` must be null or valid
/// for reads of the bytes its `size` field declares.
unsafe extern "C" fn on_event<M: Mod>(state: *mut c_void, event: *const BbEvent) -> BbEventResult {
    if state.is_null() || event.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: `state` is live until `destroy` (see `# Safety`), and the host does not call back into the mod while
    // this one runs, so the reference is unique.
    let slot = unsafe { &mut *state.cast::<Slot<M>>() };
    // SAFETY: `event` is valid for this call (see `# Safety`).
    let Some(event) = (unsafe { event::decode(event) }) else {
        return BB_ERR_ARG;
    };
    let host = slot.host;
    let keep = slot.keep.clone();
    let result = guard::run(|| {
        let mut ctx = Ctx::new(host, keep);
        slot.module.event(&mut ctx, &event);
        Ok(())
    });
    match finish(host, "event", result) {
        Ok(()) => BB_EVENT_CONTINUE,
        Err(error) => error.code(),
    }
}

/// The `destroy` trampoline for the mod type `M`: unloads the mod, drops what it kept for the host, and frees its state.
///
/// # Safety
/// `state` must be the pointer `create::<M>` produced, passed here exactly once.
unsafe extern "C" fn destroy<M: Mod>(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: `state` came from `Box::into_raw` in `create::<M>` and is passed here once, so this reclaims it exactly
    // once.
    let mut slot = unsafe { Box::from_raw(state.cast::<Slot<M>>()) };
    let host = slot.host;
    let keep = slot.keep.clone();
    let result = guard::run(|| {
        let mut ctx = Ctx::new(host, keep);
        slot.module.unload(&mut ctx);
        Ok(())
    });
    // `destroy` returns nothing to the host; a panic has already been logged by `finish`.
    let _ = finish(host, "unload", result);
    slot.keep.borrow_mut().clear();
}

/// Log a caught panic through the host, then pass the result on.
fn finish<T>(host: HostTable, callback: &str, result: ModResult<T>) -> ModResult<T> {
    if let Err(Error::Panic) = result {
        let message = format!("mod panicked in {callback}; the call returned an error");
        log(host, BB_LOG_ERROR, &message);
    }
    result
}

#[cfg(test)]
mod tests;
