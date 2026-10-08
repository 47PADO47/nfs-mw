//! `CollisionObject`: a free-standing oriented box or cylinder. The shipped PC data has none
//! (`docs/formats/collision.md`); the record is read for completeness.

use crate::Result;
use crate::bytes::Reader;

/// Size of a record in the pack.
pub const OBJECT_LEN: usize = 0x70;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// Centre (xyz) and bounding radius (w).
    pub pos_radius: [f32; 4],
    /// Half extents (xyz).
    pub dimensions: [f32; 4],
    /// 0 = box, 1 = cylinder.
    pub kind: u8,
    pub shape: u8,
    /// Bit 0 = dynamic.
    pub flags: u16,
    pub render_instance: u16,
    pub surface: u8,
    pub surface_flags: u8,
    /// Row-major 4x4 matrix.
    pub matrix: [f32; 16],
}

impl Object {
    pub(crate) fn decode(r: &Reader<'_>, at: usize) -> Result<Self> {
        let v4 = |o: usize| {
            Ok::<_, crate::Error>([r.f32(at + o)?, r.f32(at + o + 4)?, r.f32(at + o + 8)?, r.f32(at + o + 12)?])
        };
        let mut matrix = [0.0; 16];
        for (i, m) in matrix.iter_mut().enumerate() {
            *m = r.f32(at + 0x30 + i * 4)?;
        }
        Ok(Self {
            pos_radius: v4(0)?,
            dimensions: v4(0x10)?,
            kind: r.u8(at + 0x20)?,
            shape: r.u8(at + 0x21)?,
            flags: r.u16(at + 0x22)?,
            render_instance: r.u16(at + 0x24)?,
            surface: r.u8(at + 0x26)?,
            surface_flags: r.u8(at + 0x27)?,
            matrix,
        })
    }
}
