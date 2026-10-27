use chameleon_bundle::Import;

use crate::error::{Error, Result};

/// Bounds-checked little-endian reads over one resource block.
#[derive(Clone, Copy)]
pub(crate) struct Block<'a> {
    pub data: &'a [u8],
    pub what: &'static str,
}

impl<'a> Block<'a> {
    pub fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, what }
    }

    pub fn slice(&self, offset: usize, len: usize) -> Result<&'a [u8]> {
        let end = offset.saturating_add(len);
        self.data.get(offset..end).ok_or(Error::OutOfRange {
            what: self.what,
            start: offset,
            end,
            len: self.data.len(),
        })
    }

    pub fn u8(&self, offset: usize) -> Result<u8> {
        Ok(self.slice(offset, 1)?[0])
    }

    pub fn u16(&self, offset: usize) -> Result<u16> {
        let bytes = self.slice(offset, 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn u32(&self, offset: usize) -> Result<u32> {
        let bytes = self.slice(offset, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn f32(&self, offset: usize) -> Result<f32> {
        Ok(f32::from_bits(self.u32(offset)?))
    }

    pub fn f32s<const N: usize>(&self, offset: usize) -> Result<[f32; N]> {
        let mut out = [0.0; N];
        for (n, slot) in out.iter_mut().enumerate() {
            *slot = self.f32(offset + 4 * n)?;
        }
        Ok(out)
    }
}

/// The id of the resource a pointer slot at `offset` imports.
pub(crate) fn import_at(imports: &[Import], offset: u32, what: &'static str) -> Result<u64> {
    imports
        .iter()
        .find(|import| import.offset == offset)
        .map(|import| import.id)
        .ok_or(Error::MissingImport { what, offset })
}
