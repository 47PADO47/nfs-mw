//! Bounds-checked little-endian reads.

use glam::Vec3;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    what: &'static str,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, what }
    }

    fn array<const N: usize>(&self, offset: usize) -> Result<[u8; N]> {
        let bytes = offset.checked_add(N).and_then(|end| self.data.get(offset..end));
        let Some(bytes) = bytes else {
            return Err(Error::Truncated { what: self.what, offset, need: N, len: self.data.len() });
        };
        Ok(bytes.try_into().unwrap())
    }

    pub fn u8(&self, o: usize) -> Result<u8> {
        Ok(self.array::<1>(o)?[0])
    }

    pub fn i8(&self, o: usize) -> Result<i8> {
        Ok(self.u8(o)? as i8)
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

    pub fn vec3(&self, o: usize) -> Result<Vec3> {
        Ok(Vec3::new(self.f32(o)?, self.f32(o + 4)?, self.f32(o + 8)?))
    }
}
