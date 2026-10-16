//! The only SDK code that reads an interface table or calls its function pointers (ADR 0005 sections 1 and 8).
//!
//! The table's `size` is checked before each field is read, so a host built for an older minor, with a shorter table,
//! answers `None` for the fields it lacks, which the wrappers report as `Error::Unsupported`. A record is passed to the host
//! with its own size word set, so the host fills only what the mod knows about.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to host table, io and slot modules, and sdk::ffi.

pub(crate) mod career;
pub(crate) mod pursuit;
pub(crate) mod races;

use core::mem::{MaybeUninit, size_of};

use blackbox_modding::abi::{BbIface, BbStatus, has_field};
use blackbox_modding::sdk::{Error, ModResult};

/// Read one field of an interface table, or `absent` when the table's `size` does not cover the field.
///
/// # Safety
/// `base` must point to an interface table that stays valid while the mod is loaded, and `size` must be the table's own
/// size word. The field at `offset` must have type `T` in that table's layout.
pub(crate) unsafe fn read_field<T: Copy>(base: *const BbIface, size: u32, offset: usize, absent: T) -> T {
    if !has_field(size, offset + size_of::<T>()) {
        return absent;
    }
    // SAFETY: the table covers the field (checked above) and stays valid while the mod is loaded (see `# Safety`).
    unsafe { base.cast::<u8>().add(offset).cast::<T>().read_unaligned() }
}

/// Turn a host status into the SDK's result: a non-negative status is success.
pub(crate) fn check(status: BbStatus) -> ModResult<()> {
    match Error::from_status(status) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// An all-zero value of a plain record type.
///
/// # Safety
/// `T` must be plain data: integers, floats, booleans as `u8`, arrays and `repr(C)` structs of those, with no references,
/// no pointers that must be valid, and no enum. The all-zero bytes must be a valid value of `T`.
pub(crate) unsafe fn zeroed<T: Copy>() -> T {
    // SAFETY: the caller guarantees that all-zero bytes are a valid `T` (see `# Safety`).
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

/// A zeroed size-prefixed record, with its size word set to its own size. The host fills at most that many bytes.
///
/// # Safety
/// As for [`zeroed`]; additionally `T` must start with a `u32` size word.
pub(crate) unsafe fn sized_out<T: Copy>() -> T {
    // SAFETY: `T` is plain data with a size word first (see `# Safety`).
    let mut value: T = unsafe { zeroed() };
    // SAFETY: the record starts with its `u32` size word, so this write is in bounds.
    unsafe { (core::ptr::from_mut(&mut value)).cast::<u32>().write_unaligned(size_of::<T>() as u32) };
    value
}
