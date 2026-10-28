//! Little-endian readers and writers for the pack.

use glam::{Mat4, Quat, Vec3};

use crate::error::{Error, Result};
use crate::transform::Transform;

/// Appends little-endian values to a byte vector.
#[derive(Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    pub fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    pub fn u16(&mut self, v: u16) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub fn f32(&mut self, v: f32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// A length that must fit in 32 bits.
    pub fn count(&mut self, v: usize) {
        self.u32(u32::try_from(v).unwrap_or(u32::MAX));
    }

    /// A string: `u16` byte length, then the bytes (cut at 65535).
    pub fn str(&mut self, s: &str) {
        let bytes = &s.as_bytes()[..s.len().min(usize::from(u16::MAX))];
        self.u16(bytes.len() as u16);
        self.bytes.extend_from_slice(bytes);
    }

    pub fn vec3(&mut self, v: Vec3) {
        v.to_array().into_iter().for_each(|x| self.f32(x));
    }

    pub fn quat(&mut self, q: Quat) {
        q.to_array().into_iter().for_each(|x| self.f32(x));
    }

    /// Rotation (x, y, z, w), translation, scale.
    pub fn transform(&mut self, t: &Transform) {
        self.quat(t.rotation);
        self.vec3(t.translation);
        self.vec3(t.scale);
    }

    pub fn mat4(&mut self, m: &Mat4) {
        m.to_cols_array().into_iter().for_each(|x| self.f32(x));
    }

    /// A bone index; `None` is written as `u32::MAX`.
    pub fn bone(&mut self, bone: Option<usize>) {
        self.u32(bone.map_or(u32::MAX, |b| u32::try_from(b).unwrap_or(u32::MAX - 1)));
    }
}

/// Reads little-endian values from a slice, failing instead of panicking when it ends early.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).ok_or(Error::UnexpectedEnd)?;
        let slice = self.bytes.get(self.at..end).ok_or(Error::UnexpectedEnd)?;
        self.at = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    /// A count of items that each take at least `min_item_bytes`; fails if the data left cannot hold them.
    pub fn count(&mut self, min_item_bytes: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        let needed = n.checked_mul(min_item_bytes).ok_or(Error::UnexpectedEnd)?;
        if needed > self.bytes.len() - self.at {
            return Err(Error::UnexpectedEnd);
        }
        Ok(n)
    }

    pub fn str(&mut self) -> Result<String> {
        let n = usize::from(self.u16()?);
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| Error::BadString)
    }

    pub fn vec3(&mut self) -> Result<Vec3> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }

    pub fn quat(&mut self) -> Result<Quat> {
        Ok(Quat::from_xyzw(self.f32()?, self.f32()?, self.f32()?, self.f32()?))
    }

    pub fn transform(&mut self) -> Result<Transform> {
        Ok(Transform { rotation: self.quat()?, translation: self.vec3()?, scale: self.vec3()? })
    }

    pub fn mat4(&mut self) -> Result<Mat4> {
        let mut cols = [0.0_f32; 16];
        for c in &mut cols {
            *c = self.f32()?;
        }
        Ok(Mat4::from_cols_array(&cols))
    }

    pub fn bone(&mut self) -> Result<Option<usize>> {
        let v = self.u32()?;
        Ok((v != u32::MAX).then_some(v as usize))
    }

    /// `true` when every byte has been read.
    #[must_use]
    pub const fn is_done(&self) -> bool {
        self.at == self.bytes.len()
    }
}
