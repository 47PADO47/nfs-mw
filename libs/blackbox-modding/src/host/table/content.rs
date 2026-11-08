//! `bb.content` on the host side: the backend the game implements, and the trampolines behind `BbContent`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;
use std::sync::{Arc, PoisonError, RwLock};

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of};
use crate::abi::{
    BB_ERR_ARG, BB_ERR_HOST, BB_ERR_STALE, BB_ERR_UNSUPPORTED, BB_OK, BbContent, BbContentBytesSinkFn,
    BbContentReadLazyFn, BbIface, BbStatus, BbStr, IFACE_CONTENT, IFACE_CONTENT_VERSION,
    MAX_PATH_BYTES,
};

/// The most bytes one lazy file may have. A file a mod makes is a file the game reads whole, and none is near this size; a
/// mod that hands over more is refused rather than copied.
pub const MAX_LAZY_FILE_BYTES: usize = 1 << 30;

/// What the game implements for `bb.content`. `owner` is the id of the mod that made the call.
pub trait ContentBackend {
    /// Read the folder of mod `owner` again and add the content it holds that is new: its `files/` layer and its
    /// content manifests. Content already added is not added twice.
    fn rescan(&self, owner: &str) -> BackendResult<()>;

    /// The mod `owner` makes some of its files when they are read: `files` makes them. The game asks it from any thread,
    /// whenever a stub of the mod's layer is read. Once the mod unloads `files` answers every read with an error, so the
    /// game may keep it. A game without lazy files answers `BB_ERR_UNSUPPORTED`, which is also the default.
    fn provide_lazy_files(&self, _owner: &str, _files: Arc<dyn LazyFiles>) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }

    /// The parts of the content API this game has, as the `BB_CONTENT_CAP_*` bits. A game with none answers 0, which is the default.
    fn capabilities(&self) -> u32 {
        0
    }

    /// One line for every replacement set and world patch of mod `owner` and what became of it. A game without them answers
    /// `BB_ERR_UNSUPPORTED`, which is also the default.
    fn report(&self, _owner: &str) -> BackendResult<String> {
        Err(BB_ERR_UNSUPPORTED)
    }
}

/// The maker of a mod's lazy files, as the game sees it: asked for a file by key, from any thread.
pub trait LazyFiles: Send + Sync {
    /// The whole contents of the lazy file `key`, or the status that says why the mod could not make it (`BB_ERR_STALE` when
    /// the mod has unloaded).
    fn read(&self, key: &str) -> BackendResult<Vec<u8>>;
}

/// Build the `bb.content` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn ContentBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbContent {
        head: BbIface {
            size: size_of::<BbContent>() as u32,
            major: IFACE_CONTENT_VERSION.0,
            minor: IFACE_CONTENT_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        rescan: Some(rescan),
        provide_lazy_files: Some(provide_lazy_files),
        capabilities: Some(capabilities),
        report: Some(report),
    });
    // The content a mod adds stays for the run, so nothing is removed on unload but the maker of its lazy files, which
    // stops answering; the release keeps the slot alive.
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_CONTENT, IFACE_CONTENT_VERSION, table, release)
}

/// `rescan`.
unsafe extern "C" fn rescan(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn ContentBackend>(ctx)? };
        slot.backend.rescan(&slot.owner)
    })
}

/// `capabilities`.
unsafe extern "C" fn capabilities(ctx: *mut c_void) -> u32 {
    // SAFETY: `ctx` is this table's slot (see `slot_of`).
    let Ok(slot) = (unsafe { slot_of::<dyn ContentBackend>(ctx) }) else { return 0 };
    slot.backend.capabilities()
}

/// `report`: the text goes to the mod's sink in one piece.
unsafe extern "C" fn report(ctx: *mut c_void, sink: Option<BbContentBytesSinkFn>, sink_ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn ContentBackend>(ctx)? };
        let sink = sink.ok_or(BB_ERR_ARG)?;
        let text = slot.backend.report(&slot.owner)?;
        // SAFETY: the mod's sink takes bytes valid for the call, which `text` is, and copies them.
        unsafe { sink(sink_ctx, text.as_ptr(), text.len() as u64) };
        Ok(())
    })
}

/// `provide_lazy_files`.
unsafe extern "C" fn provide_lazy_files(
    ctx: *mut c_void,
    read: Option<BbContentReadLazyFn>,
    user: *mut c_void,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn ContentBackend>(ctx)? };
        let read = read.ok_or(BB_ERR_ARG)?;
        let files = Arc::new(ModLazyFiles { maker: RwLock::new(Some(Maker { read, user })) });
        slot.backend.provide_lazy_files(&slot.owner, files.clone())?;
        slot.on_release(Box::new(move || files.disable()));
        Ok(())
    })
}

/// The mod's function and its pointer, as `provide_lazy_files` got them.
#[derive(Clone, Copy)]
struct Maker {
    read: BbContentReadLazyFn,
    user: *mut c_void,
}

// SAFETY: a `Maker` is a function pointer and the mod's own pointer, which the mod promises (`BbContentReadLazyFn`) it can
// be called with from any thread. The host never reads through `user`.
unsafe impl Send for Maker {}
// SAFETY: as for `Send`: the mod's function is callable from several threads at once.
unsafe impl Sync for Maker {}

/// The lazy files of one mod: the mod's function while the mod is loaded, nothing after.
struct ModLazyFiles {
    maker: RwLock<Option<Maker>>,
}

impl ModLazyFiles {
    /// Stop calling the mod. A read that is in progress finishes first: it holds the lock while the mod works.
    fn disable(&self) {
        *self.maker.write().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl LazyFiles for ModLazyFiles {
    fn read(&self, key: &str) -> BackendResult<Vec<u8>> {
        if key.len() > MAX_PATH_BYTES {
            return Err(BB_ERR_ARG);
        }
        // Held until the end of the read: `disable` waits for it, so the mod's function is never called after it unloaded.
        let held = self.maker.read().unwrap_or_else(PoisonError::into_inner);
        let Some(maker) = *held else { return Err(BB_ERR_STALE) };
        let mut made: Option<Vec<u8>> = None;
        let sink: BbContentBytesSinkFn = collect;
        // SAFETY: `maker` is the function and pointer the mod gave, valid while the mod is loaded, which the read lock held
        // here guarantees (`disable` waits for it). `key` and `made` outlive the call, and `collect` only writes `made`.
        let status = unsafe {
            (maker.read)(maker.user, BbStr::new(key), Some(sink), (&mut made as *mut Option<Vec<u8>>).cast())
        };
        if status != BB_OK {
            return Err(status);
        }
        made.ok_or(BB_ERR_HOST)
    }
}

/// The sink of a lazy read: copies the bytes the mod hands over into the `Option<Vec<u8>>` that `sink` names. Bytes that are
/// too many, or a null pointer for a non-empty file, leave it empty, which the read reports as a host error.
unsafe extern "C" fn collect(sink: *mut c_void, bytes: *const u8, len: u64) {
    // SAFETY: `sink` is the `Option<Vec<u8>>` that `ModLazyFiles::read` passed, alive for the read.
    let Some(out) = (unsafe { sink.cast::<Option<Vec<u8>>>().as_mut() }) else { return };
    let Ok(len) = usize::try_from(len) else { return };
    if len > MAX_LAZY_FILE_BYTES || (bytes.is_null() && len != 0) {
        return;
    }
    if len == 0 {
        *out = Some(Vec::new());
        return;
    }
    // SAFETY: the mod guarantees `len` readable bytes at `bytes` for this call (`BbContentBytesSinkFn`).
    *out = Some(unsafe { core::slice::from_raw_parts(bytes, len) }.to_vec());
}
