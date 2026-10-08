//! Bounds-checked little-endian reads.

use crate::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Reader<'a> {
    pub data: &'a [u8],
    pub what: &'static str,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, what }
    }

    pub fn slice(&self, offset: usize, len: usize) -> Result<&'a [u8]> {
        offset.checked_add(len).and_then(|end| self.data.get(offset..end)).ok_or(Error::Truncated {
            what: self.what,
            offset,
            need: len,
            len: self.data.len(),
        })
    }

    fn array<const N: usize>(&self, offset: usize) -> Result<[u8; N]> {
        Ok(self.slice(offset, N)?.try_into().unwrap())
    }

    pub fn u8(&self, o: usize) -> Result<u8> {
        Ok(self.array::<1>(o)?[0])
    }

    pub fn u16(&self, o: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array(o)?))
    }

    pub fn i16(&self, o: usize) -> Result<i16> {
        Ok(i16::from_le_bytes(self.array(o)?))
    }

    pub fn u32(&self, o: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array(o)?))
    }

    pub fn f32(&self, o: usize) -> Result<f32> {
        Ok(f32::from_le_bytes(self.array(o)?))
    }

    pub fn vec3(&self, o: usize) -> Result<[f32; 3]> {
        Ok([self.f32(o)?, self.f32(o + 4)?, self.f32(o + 8)?])
    }
}

pub(crate) fn malformed(what: &'static str, detail: impl Into<String>) -> Error {
    Error::Malformed { what, detail: detail.into() }
}
