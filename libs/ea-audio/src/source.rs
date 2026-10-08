//! Random access to bytes that may not fit in memory (the 533 MB music file).
//!
//! The library never opens files. Callers implement [`ReadAt`] over a file or a memory map; `[u8]`,
//! `Vec<u8>` and references already implement it.

use crate::error::{Error, Result};

/// A read-only byte source with random access.
pub trait ReadAt {
    /// Total length in bytes.
    fn len(&self) -> u64;

    /// Copy up to `buf.len()` bytes starting at `offset`; returns how many were copied (0 at or past the end).
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize;

    /// True when the source holds no bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ReadAt for [u8] {
    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        let Ok(start) = usize::try_from(offset) else { return 0 };
        let Some(rest) = self.get(start..) else { return 0 };
        let n = rest.len().min(buf.len());
        buf[..n].copy_from_slice(&rest[..n]);
        n
    }
}

impl ReadAt for Vec<u8> {
    fn len(&self) -> u64 {
        self.as_slice().len() as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        self.as_slice().read_at(offset, buf)
    }
}

impl<T: ReadAt + ?Sized> ReadAt for &T {
    fn len(&self) -> u64 {
        (**self).len()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        (**self).read_at(offset, buf)
    }
}

/// Read exactly `N` bytes at `offset`.
pub fn read_array<const N: usize, S: ReadAt + ?Sized>(src: &S, offset: u64) -> Result<[u8; N]> {
    let mut buf = [0u8; N];
    if src.read_at(offset, &mut buf) < N {
        return Err(Error::Truncated { offset, needed: N });
    }
    Ok(buf)
}

/// Read `len` bytes at `offset` into a new vector.
pub fn read_vec<S: ReadAt + ?Sized>(src: &S, offset: u64, len: usize) -> Result<Vec<u8>> {
    let available = src.len().saturating_sub(offset);
    if (len as u64) > available {
        return Err(Error::Truncated { offset, needed: len });
    }
    let mut buf = vec![0u8; len];
    src.read_at(offset, &mut buf);
    Ok(buf)
}
