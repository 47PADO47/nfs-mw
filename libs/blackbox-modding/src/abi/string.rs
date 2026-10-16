//! `BbStr`: the UTF-8 string that crosses the boundary (ADR 0005 section 4).

/// A UTF-8 string passed across the boundary: a pointer and a byte length.
///
/// The bytes are not NUL-terminated and are valid only for the call that carries the `BbStr`. The receiver
/// copies whatever it keeps. A `BbStr` must not be stored past that call.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbStr {
    /// First byte of the string. Null is allowed only with `len == 0`.
    pub ptr: *const u8,
    /// Length in bytes.
    pub len: u64,
}

// SAFETY: a `BbStr` is plain data: a pointer and a length. Copying one between threads copies the pointer
// only, and the bytes are read under the lifetime rule above. This lets a `static` hold one (`BbModInfo`).
unsafe impl Send for BbStr {}
// SAFETY: as for `Send`. Shared reads of the bytes are allowed; the string is never written through a `BbStr`.
unsafe impl Sync for BbStr {}

impl BbStr {
    /// The empty string.
    pub const EMPTY: BbStr = BbStr { ptr: core::ptr::null(), len: 0 };

    /// A `BbStr` that borrows `text`. The borrow must last for the call that carries it.
    pub const fn new(text: &str) -> Self {
        BbStr { ptr: text.as_ptr(), len: text.len() as u64 }
    }

    /// Read the bytes as `&str`. `None` when the pointer is null for a non-empty string, the length cannot be a
    /// Rust slice, or the bytes are not UTF-8.
    ///
    /// # Safety
    /// For a non-empty string, `ptr` must be valid for reads of `len` bytes for the whole lifetime `'a`.
    pub unsafe fn as_str<'a>(&self) -> Option<&'a str> {
        if self.len == 0 {
            return Some("");
        }
        if self.ptr.is_null() {
            return None;
        }
        let len = usize::try_from(self.len).ok()?;
        if len > isize::MAX as usize {
            return None;
        }
        // SAFETY: the caller guarantees `len` readable bytes at `ptr`; null and oversize were rejected above.
        let bytes = unsafe { core::slice::from_raw_parts(self.ptr, len) };
        core::str::from_utf8(bytes).ok()
    }
}
