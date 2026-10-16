//! Moving plain records across the boundary (ADR 0005 sections 1 and 4). A size-prefixed record is copied by the bytes both
//! sides know, so a mod built against an older minor still works. Arrays and scalars are copied element by element. No
//! pointer is kept past the call.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to host table, io and slot modules.

use core::mem::{MaybeUninit, size_of};
use core::ptr;

use blackbox_modding::abi::{BB_ERR_ARG, BbStatus};

/// Write one scalar result into the mod's memory. A null pointer is `BB_ERR_ARG`.
///
/// # Safety
/// `out`, when not null, must be valid for one write of `T`.
pub(crate) unsafe fn put<T: Copy>(out: *mut T, value: T) -> Result<(), BbStatus> {
    if out.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller guarantees `out` is valid for one write of `T` (see `# Safety`).
    unsafe { out.write_unaligned(value) };
    Ok(())
}

/// Write a size-prefixed record into the mod's memory. The mod sets the record's first `u32` to its own size, and the host
/// copies only the bytes both sides know: the smaller of the two. The copy then carries the byte count in the size word, so
/// the mod reads back how much was filled.
///
/// # Safety
/// `out`, when not null, must be valid for reads and writes of the bytes its own size word declares, and that size must be
/// at least four.
pub(crate) unsafe fn put_sized<T: Copy>(out: *mut T, value: &T) -> Result<(), BbStatus> {
    if out.is_null() {
        return Err(BB_ERR_ARG);
    }
    let raw = out.cast::<u8>();
    // SAFETY: the mod's size word is the first four bytes of its record (see `# Safety`).
    let given = unsafe { raw.cast::<u32>().read_unaligned() } as usize;
    if given < size_of::<u32>() {
        return Err(BB_ERR_ARG);
    }
    let n = given.min(size_of::<T>());
    let src = ptr::from_ref(value).cast::<u8>();
    // SAFETY: `value` holds `size_of::<T>()` bytes and the mod's record holds `given` bytes, so `n` bytes are valid on both
    // sides; the two do not overlap, since `value` is a host value.
    unsafe { ptr::copy_nonoverlapping(src, raw, n) };
    // SAFETY: the size word is in bounds (checked above, `given >= 4`).
    unsafe { raw.cast::<u32>().write_unaligned(n as u32) };
    Ok(())
}

/// Read a size-prefixed record from the mod's memory. The first `u32` says how many bytes the mod filled; the host copies
/// that many, up to its own size. Bytes it does not get stay zero, which is a valid value of a plain record.
///
/// # Safety
/// `src`, when not null, must be valid for reads of the bytes its own size word declares, and that size must be at least
/// four.
pub(crate) unsafe fn read_sized<T: Copy>(src: *const T) -> Result<T, BbStatus> {
    if src.is_null() {
        return Err(BB_ERR_ARG);
    }
    let raw = src.cast::<u8>();
    // SAFETY: the size word is the first four bytes of the record (see `# Safety`).
    let given = unsafe { raw.cast::<u32>().read_unaligned() } as usize;
    if given < size_of::<u32>() {
        return Err(BB_ERR_ARG);
    }
    let n = given.min(size_of::<T>());
    let mut value = MaybeUninit::<T>::zeroed();
    // SAFETY: `n` bytes are readable at `raw` (see `# Safety`) and writable in `value`, which holds `size_of::<T>()` bytes.
    unsafe { ptr::copy_nonoverlapping(raw, value.as_mut_ptr().cast::<u8>(), n) };
    // SAFETY: the record is plain data, so the all-zero bytes of the unread tail, with the copied head, form a valid `T`.
    Ok(unsafe { value.assume_init() })
}

/// Read a list the mod passes: `len` elements at `items`. At most `max` elements, and a null pointer only for an empty list.
///
/// # Safety
/// `items`, when not null and `len > 0`, must be valid for reads of `len` elements of `T`.
pub(crate) unsafe fn read_list<T: Copy>(items: *const T, len: u32, max: u32) -> Result<Vec<T>, BbStatus> {
    if len > max {
        return Err(BB_ERR_ARG);
    }
    if len == 0 {
        return Ok(Vec::new());
    }
    if items.is_null() {
        return Err(BB_ERR_ARG);
    }
    let mut out = Vec::with_capacity(len as usize);
    for index in 0..len as usize {
        // SAFETY: `index < len` and the caller guarantees `len` readable elements (see `# Safety`).
        out.push(unsafe { items.add(index).read_unaligned() });
    }
    Ok(out)
}

/// Write a list the mod asked for: at most `cap` elements of `values`, into `out`. Writes nothing when `values` is empty or
/// `cap` is zero, so a null `out` is fine for a count-only call.
///
/// # Safety
/// `out`, when not null and when at least one element is written, must be valid for writes of `cap` elements of `T`.
pub(crate) unsafe fn write_list<T: Copy>(out: *mut T, cap: u32, values: &[T]) -> Result<(), BbStatus> {
    let n = values.len().min(cap as usize);
    if n == 0 {
        return Ok(());
    }
    if out.is_null() {
        return Err(BB_ERR_ARG);
    }
    for (index, value) in values[..n].iter().enumerate() {
        // SAFETY: `index < n <= cap` and the caller guarantees `cap` writable elements (see `# Safety`).
        unsafe { out.add(index).write_unaligned(*value) };
    }
    Ok(())
}
