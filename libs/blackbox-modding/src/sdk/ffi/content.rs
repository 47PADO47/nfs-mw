//! `bb.content` on the mod side: read the table, and call its functions.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{
    BB_OK, BbContent, BbContentBytesSinkFn, BbContentCapabilitiesFn, BbContentProvideLazyFn, BbContentReadLazyFn,
    BbContentReportFn, BbContentRescanFn, BbStatus, BbStr,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{Error, Interface, LazyFiles, ModResult, guard};

/// The function pointers of one `bb.content` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct ContentFns {
    ctx: *mut c_void,
    rescan: Option<BbContentRescanFn>,
    provide_lazy_files: Option<BbContentProvideLazyFn>,
    capabilities: Option<BbContentCapabilitiesFn>,
    report: Option<BbContentReportFn>,
}

/// Read the `bb.content` table the host offered.
pub(crate) fn read(iface: Interface) -> ContentFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        ContentFns {
            ctx: read_or(&iface, offset_of!(BbContent, ctx), null_mut()),
            rescan: read_or(&iface, offset_of!(BbContent, rescan), None),
            provide_lazy_files: read_or(&iface, offset_of!(BbContent, provide_lazy_files), None),
            capabilities: read_or(&iface, offset_of!(BbContent, capabilities), None),
            report: read_or(&iface, offset_of!(BbContent, report), None),
        }
    }
}

/// Ask the host to read the mod's folder again.
pub(crate) fn rescan(fns: &ContentFns) -> ModResult<()> {
    let rescan = fns.rescan.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; the call takes no other argument.
    status_result(unsafe { rescan(fns.ctx) })
}

/// Whether the host takes lazy files (`bb.content` 1.1).
pub(crate) fn supports_lazy_files(fns: &ContentFns) -> bool {
    fns.provide_lazy_files.is_some()
}

/// The `BB_CONTENT_CAP_*` bits of the host; 0 for a host older than 1.2.
pub(crate) fn capabilities(fns: &ContentFns) -> u32 {
    let Some(capabilities) = fns.capabilities else { return 0 };
    // SAFETY: the table is valid while the mod is loaded; the call takes no other argument.
    unsafe { capabilities(fns.ctx) }
}

/// The text the host reports for the mod's sets and patches.
pub(crate) fn report(fns: &ContentFns) -> ModResult<String> {
    let report = fns.report.ok_or(Error::Unsupported)?;
    let mut text: Vec<u8> = Vec::new();
    let sink: BbContentBytesSinkFn = collect_text;
    // SAFETY: the table is valid while the mod is loaded; `text` outlives the call and `collect_text` only appends to it.
    let status = unsafe { report(fns.ctx, Some(sink), (&mut text as *mut Vec<u8>).cast()) };
    status_result(status)?;
    String::from_utf8(text).map_err(|_| Error::Host)
}

/// The sink of `report`: appends the bytes the host hands over to the `Vec<u8>` that `sink` names.
unsafe extern "C" fn collect_text(sink: *mut c_void, bytes: *const u8, len: u64) {
    // SAFETY: `sink` is the `Vec<u8>` that `report` passed, alive for the call.
    let Some(out) = (unsafe { sink.cast::<Vec<u8>>().as_mut() }) else { return };
    let Ok(len) = usize::try_from(len) else { return };
    if len == 0 || bytes.is_null() {
        return;
    }
    // SAFETY: the host guarantees `len` readable bytes at `bytes` for this call.
    out.extend_from_slice(unsafe { core::slice::from_raw_parts(bytes, len) });
}

/// What the host's calls reach: the mod's maker of lazy files, boxed once so its address is the `user` pointer.
type Maker = Box<dyn LazyFiles>;

/// Give the host the maker of the mod's lazy files. The maker is kept in `keep` until the mod unloads, and the host stops
/// calling it then.
pub(crate) fn provide_lazy_files(fns: &ContentFns, keep: &Keep, files: Maker) -> ModResult<()> {
    let provide = fns.provide_lazy_files.ok_or(Error::Unsupported)?;
    let held: Box<Maker> = Box::new(files);
    let user: *mut c_void = (&*held as *const Maker).cast_mut().cast();
    let read: BbContentReadLazyFn = read_trampoline;
    // SAFETY: the host calls `read_trampoline` with `user` only while the mod is loaded. `held` moves into `keep` below,
    // which lives until the mod's `destroy`, and the host stops calling when the mod unloads; the box's address does not
    // change when the box moves.
    let status = unsafe { provide(fns.ctx, Some(read), user) };
    status_result(status)?;
    keep.borrow_mut().push(held);
    Ok(())
}

/// The function the host calls to make a lazy file: runs the mod's maker for `key` and hands the bytes to `sink`. Runs on
/// whatever thread reads the file, so it touches nothing but the maker, which is `Send + Sync`.
unsafe extern "C" fn read_trampoline(
    user: *mut c_void,
    key: BbStr,
    sink: Option<BbContentBytesSinkFn>,
    sink_ctx: *mut c_void,
) -> BbStatus {
    let result = guard::run(|| {
        // SAFETY: `user` is the `Maker` that `provide_lazy_files` kept alive (see there).
        let maker = unsafe { user.cast::<Maker>().as_ref() }.ok_or(Error::Arg)?;
        // SAFETY: the host passes a string valid for this call.
        let key = unsafe { key.as_str() }.ok_or(Error::Arg)?;
        let sink = sink.ok_or(Error::Arg)?;
        let bytes = maker.read(key)?;
        // SAFETY: the host's sink takes bytes valid for the call, which `bytes` is, and copies them.
        unsafe { sink(sink_ctx, bytes.as_ptr(), bytes.len() as u64) };
        Ok(())
    });
    result.err().map_or(BB_OK, Error::code)
}
