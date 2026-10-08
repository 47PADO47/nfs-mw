//! `CollisionArticle`: the geometry shared by one or more instances: triangle strips, barriers and
//! the surface table. Spec: `docs/formats/collision.md` ("Articles").

use crate::Result;
use crate::bytes::{Reader, malformed};
use crate::math::Vec3;

const HEADER_LEN: usize = 16;
const SPHERE_LEN: usize = 16;
const BARRIER_LEN: usize = 0x20;
const VERT_LEN: usize = 8;
/// Packed vertex coordinates are fixed point with this many steps per metre.
pub const VERT_SCALE: f32 = 128.0;
/// Strip bounding radii are stored in 1/16 m.
pub const RADIUS_SCALE: f32 = 16.0;

/// Strip flag: the first triangle faces up (winding "right").
pub const STRIP_UP_FACING: u16 = 1;
/// Strip flag: no consistent winding; test triangles in both windings.
pub const STRIP_FACING_UNKNOWN: u16 = 2;
/// Triangle flag: skipped by ground-height point queries.
pub const SURFACE_NO_GROUND: u8 = 0x08;
/// Barrier flag: blocks from both sides (otherwise only against its normal).
pub const BARRIER_TWO_SIDED: u8 = 0x10;

/// One packed strip vertex. For the first two vertices of a strip `surface` and `flags` are the
/// bytes of the strip's vertex count and flags (see [`Strip`]); use [`Strip::triangles`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedVert {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub surface: u8,
    pub flags: u8,
}

/// A triangle of a strip, in the article's local space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub pts: [Vec3; 3],
    /// Index into [`Article::surfaces`] (taken from the third vertex).
    pub surface: u8,
    pub flags: u8,
}

/// A triangle strip with its bounding sphere.
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// Local-space origin of the strip's vertices (the sphere centre).
    pub center: Vec3,
    /// Bounding radius in metres (about the maximum 3-D distance of a vertex from `center`).
    pub radius: f32,
    /// [`STRIP_UP_FACING`], [`STRIP_FACING_UNKNOWN`].
    pub flags: u16,
    pub verts: Vec<PackedVert>,
}

impl Strip {
    pub fn triangle_count(&self) -> usize {
        self.verts.len().saturating_sub(2)
    }

    /// Local position of vertex `i`.
    pub fn point(&self, i: usize) -> Vec3 {
        let v = self.verts[i];
        let c = self.center;
        [c[0] + f32::from(v.x) / VERT_SCALE, c[1] + f32::from(v.y) / VERT_SCALE, c[2] + f32::from(v.z) / VERT_SCALE]
    }

    /// Triangle `i` (vertices `i`, `i + 1`, `i + 2`).
    pub fn triangle(&self, i: usize) -> Triangle {
        let last = self.verts[i + 2];
        Triangle {
            pts: [self.point(i), self.point(i + 1), self.point(i + 2)],
            surface: last.surface,
            flags: last.flags,
        }
    }

    pub fn triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        (0..self.triangle_count()).map(|i| self.triangle(i))
    }
}

/// A vertical wall segment ("barrier", "edge").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Barrier {
    pub p0: Vec3,
    pub p1: Vec3,
    /// Index into [`Article::surfaces`].
    pub surface: u8,
    pub flags: u8,
    /// `1 / (xz length)`, stored by the original tools.
    pub inv_xz_len: f32,
}

impl Barrier {
    pub fn y_bottom(&self) -> f32 {
        self.p0[1].min(self.p1[1])
    }

    pub fn y_top(&self) -> f32 {
        self.p0[1].max(self.p1[1])
    }

    /// Horizontal unit normal, `(dz, 0, -dx)` of the segment.
    pub fn normal(&self) -> Vec3 {
        let (dx, dz) = (self.p1[0] - self.p0[0], self.p1[2] - self.p0[2]);
        let l = (dx * dx + dz * dz).sqrt();
        if l > 0.0 { [dz / l, 0.0, -dx / l] } else { [0.0, 0.0, 0.0] }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Article {
    pub strips: Vec<Strip>,
    pub barriers: Vec<Barrier>,
    /// Hashes of the surface types (`simsurface` collection keys) the strips and barriers refer to.
    pub surfaces: Vec<u32>,
    /// Build-time value; meaningless in the game.
    pub intermediate_object: u16,
    pub flags: i16,
}

impl Article {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let r = Reader::new(data, "collision article");
        let strip_count = usize::from(r.u16(0)?);
        let strips_size = usize::from(r.u16(2)?);
        let barrier_count = usize::from(r.u16(4)?);
        let barriers_size = usize::from(r.u16(6)?);
        let surface_count = usize::from(r.u8(9)?);
        if barriers_size != barrier_count * BARRIER_LEN {
            return Err(malformed("collision article", format!("{barriers_size} B of barriers for {barrier_count}")));
        }
        let strips_end = HEADER_LEN + strips_size;
        let barriers_end = strips_end + barriers_size;
        if strip_count * SPHERE_LEN > strips_size {
            return Err(malformed("collision article", "sphere table larger than the strip area"));
        }

        let mut strips = Vec::with_capacity(strip_count);
        for i in 0..strip_count {
            let s = HEADER_LEN + i * SPHERE_LEN;
            let at = HEADER_LEN + usize::from(r.u16(s + 14)?);
            let verts_n = usize::from(r.u16(at + 6)?);
            if verts_n < 2 || at + verts_n * VERT_LEN > strips_end {
                return Err(malformed("collision article", format!("strip {i}: {verts_n} vertices at 0x{at:X}")));
            }
            let verts = (0..verts_n)
                .map(|v| {
                    let o = at + v * VERT_LEN;
                    Ok(PackedVert {
                        x: r.i16(o)?,
                        y: r.i16(o + 2)?,
                        z: r.i16(o + 4)?,
                        surface: r.u8(o + 6)?,
                        flags: r.u8(o + 7)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            strips.push(Strip {
                center: r.vec3(s)?,
                radius: f32::from(r.u16(s + 12)?) / RADIUS_SCALE,
                flags: r.u16(at + 14)?,
                verts,
            });
        }

        let barriers = (0..barrier_count)
            .map(|i| {
                let o = strips_end + i * BARRIER_LEN;
                let tag = r.u32(o + 12)?;
                Ok(Barrier {
                    p0: r.vec3(o)?,
                    p1: r.vec3(o + 16)?,
                    surface: tag as u8,
                    flags: (tag >> 8) as u8,
                    inv_xz_len: r.f32(o + 28)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let surfaces = (0..surface_count).map(|i| r.u32(barriers_end + i * 4)).collect::<Result<Vec<_>>>()?;
        Ok(Self { strips, barriers, surfaces, intermediate_object: r.u16(0xC)?, flags: r.i16(0xE)? })
    }

    /// The surface hash for a triangle or barrier surface index, if the table has it.
    pub fn surface_hash(&self, index: u8) -> Option<u32> {
        self.surfaces.get(usize::from(index)).copied()
    }
}
