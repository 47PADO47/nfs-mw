//! Little-endian reads that report where they ran out instead of panicking.

use crate::error::{Error, Result};

/// A block of bytes read at offsets.
#[derive(Clone, Copy)]
pub(crate) struct Block<'a> {
    bytes: &'a [u8],
    what: &'static str,
}

impl<'a> Block<'a> {
    pub(crate) fn new(bytes: &'a [u8], what: &'static str) -> Self {
        Self { bytes, what }
    }

    pub(crate) fn slice(&self, at: usize, len: usize) -> Result<&'a [u8]> {
        let end = at.saturating_add(len);
        self.bytes.get(at..end).ok_or(Error::Truncated { what: self.what, start: at, end, len: self.bytes.len() })
    }

    pub(crate) fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.slice(at, 2)?.try_into().unwrap_or([0; 2])))
    }

    pub(crate) fn u32(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.slice(at, 4)?.try_into().unwrap_or([0; 4])))
    }

    pub(crate) fn f32(&self, at: usize) -> Result<f32> {
        Ok(f32::from_bits(self.u32(at)?))
    }

    pub(crate) fn f32s<const N: usize>(&self, at: usize) -> Result<[f32; N]> {
        let mut out = [0.0; N];
        for (n, value) in out.iter_mut().enumerate() {
            *value = self.f32(at + 4 * n)?;
        }
        Ok(out)
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }
}
