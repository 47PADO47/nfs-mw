//! What every interface table shares: the backend, the owning mod's id, the ids the mod registered, the trampoline
//! guard and the attachment that keeps a table alive for as long as its mod is loaded.

use core::any::Any;
use core::ffi::c_void;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use crate::abi::{BB_ERR_ARG, BB_ERR_PANIC, BB_OK, BbIface, BbSinkFn, BbStatus, BbStr, MAX_NAME_BYTES};

/// One interface table's state. The table's `ctx` is `Rc::as_ptr` of a `Slot`, so every trampoline reads the backend and
/// the owner from it. The `Rc` is also held by the table's release callback, so the slot outlives the table.
pub(crate) struct Slot<T: ?Sized> {
    /// The game's implementation of the interface.
    pub(crate) backend: Rc<T>,
    /// Id of the mod this table was built for.
    pub(crate) owner: String,
    /// Handles the mod registered and has not removed. They are removed when the mod unloads.
    registered: RefCell<Vec<u64>>,
}

impl<T: ?Sized> Slot<T> {
    /// A new slot for `owner`.
    pub(crate) fn new(backend: Rc<T>, owner: &str) -> Rc<Self> {
        Rc::new(Slot { backend, owner: owner.to_owned(), registered: RefCell::new(Vec::new()) })
    }

    /// The `ctx` value a table carries for this slot.
    pub(crate) fn ctx(slot: &Rc<Self>) -> *mut c_void {
        Rc::as_ptr(slot).cast_mut().cast::<c_void>()
    }

    /// Remember a handle the mod registered, so it can be removed on unload.
    pub(crate) fn remember(&self, handle: u64) {
        self.registered.borrow_mut().push(handle);
    }

    /// Forget a handle the mod removed itself.
    pub(crate) fn forget(&self, handle: u64) {
        self.registered.borrow_mut().retain(|known| *known != handle);
    }

    /// Remove every handle still registered, through `remove`.
    pub(crate) fn release_with(&self, remove: impl Fn(&T, &str, u64)) {
        let handles = std::mem::take(&mut *self.registered.borrow_mut());
        for handle in handles {
            remove(&*self.backend, &self.owner, handle);
        }
    }
}

/// The slot a table's `ctx` names.
///
/// # Safety
/// `ctx` must be null, or the value [`Slot::ctx`] gave for a slot of the same `T` that is still alive. The table that
/// carries it is alive while its `Attached` is, and the host calls the table only while the mod is loaded.
pub(crate) unsafe fn slot_of<'a, T: ?Sized>(ctx: *mut c_void) -> Result<&'a Slot<T>, BbStatus> {
    if ctx.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller guarantees `ctx` is a live slot of this type (see `# Safety`).
    Ok(unsafe { &*ctx.cast::<Slot<T>>() })
}

/// Read a mod-supplied name or text. It must be UTF-8 and at most `max` bytes.
///
/// # Safety
/// `text` must be valid for the call that carries it (ADR 0005 section 4).
pub(crate) unsafe fn text_within<'a>(text: BbStr, max: usize) -> Result<&'a str, BbStatus> {
    // SAFETY: the caller guarantees the string is valid for this call.
    let text = unsafe { text.as_str() }.ok_or(BB_ERR_ARG)?;
    if text.len() > max {
        return Err(BB_ERR_ARG);
    }
    Ok(text)
}

/// A name the host takes: at most [`MAX_NAME_BYTES`].
///
/// # Safety
/// As for [`text_within`].
pub(crate) unsafe fn name_of<'a>(name: BbStr) -> Result<&'a str, BbStatus> {
    // SAFETY: forwarded to the caller's guarantee.
    unsafe { text_within(name, MAX_NAME_BYTES) }
}

/// Write `value` to a size-prefixed struct the caller owns. Its first field is the `size` word the caller filled in, and
/// the host writes only the bytes that word allows, so a caller built for an older layout gets the fields it knows. The
/// size word itself is left as the caller wrote it. A size below the word itself is refused.
///
/// # Safety
/// `out` must be null or valid for writes of the bytes its own size word declares (ADR 0005 section 4).
pub(crate) unsafe fn write_prefixed<T: Copy>(out: *mut T, value: &T) -> Result<(), BbStatus> {
    if out.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the size word is the first field of the struct, and the caller guarantees it is readable.
    let allowed = unsafe { out.cast::<u32>().read_unaligned() } as usize;
    let word = core::mem::size_of::<u32>();
    if allowed < word {
        return Err(BB_ERR_ARG);
    }
    let len = allowed.min(core::mem::size_of::<T>());
    // SAFETY: `value` is a live `T`, and `out` is valid for `len` bytes (checked against the size word above).
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(value).cast::<u8>().add(word),
            out.cast::<u8>().add(word),
            len - word,
        );
    }
    Ok(())
}

/// Write a text the host produced to a sink the caller gave. The sink takes it for the duration of the call.
///
/// # Safety
/// `sink` and `user` are the caller's, valid for this call (ADR 0005 section 4).
pub(crate) unsafe fn emit_text(sink: Option<BbSinkFn>, user: *mut c_void, text: &str) -> Result<(), BbStatus> {
    let sink = sink.ok_or(BB_ERR_ARG)?;
    // SAFETY: the caller's sink takes a string that is valid for the call, which `text` is.
    unsafe { sink(user, BbStr::new(text)) };
    Ok(())
}

/// Run a trampoline body. A panic becomes `BB_ERR_PANIC`; an error becomes its status; success becomes `BB_OK`.
pub(crate) fn guard(body: impl FnOnce() -> Result<(), BbStatus>) -> BbStatus {
    match caught(body) {
        Ok(()) => BB_OK,
        Err(status) => status,
    }
}

/// Run a body that yields a value. A panic becomes `Err(BB_ERR_PANIC)`.
pub(crate) fn caught<T>(body: impl FnOnce() -> Result<T, BbStatus>) -> Result<T, BbStatus> {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(Err(BB_ERR_PANIC))
}

/// One interface table handed to one mod. Owns the table and the release callback. Dropping it removes what the mod
/// still has registered, and only then frees the table.
pub struct Attached {
    id: &'static str,
    major: u16,
    minor: u16,
    header: *const BbIface,
    release: Box<dyn Fn()>,
    _table: Box<dyn Any>,
}

impl Attached {
    /// An interface table a provider built itself (see `InterfaceRegistry::add_provider`). `header` points at the table's
    /// first field, the [`BbIface`] header, and `owner` keeps the table alive: the attachment drops it when the mod unloads.
    /// The host never reads through `header`; the mod does, so the table in `owner` must stay where it is while `owner`
    /// lives, which a boxed table does.
    pub fn from_owned(id: &'static str, version: (u16, u16), header: *const BbIface, owner: Box<dyn Any>) -> Self {
        Attached { id, major: version.0, minor: version.1, header, release: Box::new(|| {}), _table: owner }
    }

    /// Wrap `table`, whose first field is the [`BbIface`] header, for interface `id` at `version`.
    pub(crate) fn new<T: 'static>(
        id: &'static str,
        version: (u16, u16),
        table: Box<T>,
        release: Box<dyn Fn()>,
    ) -> Self {
        let header = (&*table as *const T).cast::<BbIface>();
        Attached { id, major: version.0, minor: version.1, header, release, _table: table }
    }

    /// The interface id.
    pub(crate) fn id(&self) -> &'static str {
        self.id
    }

    /// The version the table implements, as `(major, minor)`.
    pub(crate) fn version(&self) -> (u16, u16) {
        (self.major, self.minor)
    }

    /// The table's header, the pointer `query_interface` returns.
    pub(crate) fn header(&self) -> *const BbIface {
        self.header
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        (self.release)();
    }
}
