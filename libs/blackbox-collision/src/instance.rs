//! `CollisionInstance`: where one collision article sits in the world.

use crate::Result;
use crate::bytes::Reader;
use crate::math::{Vec3, cross, dot, sub};

/// Size of a record in the pack.
pub const INSTANCE_LEN: usize = 0x40;

/// Runtime flag: the y axis is not world up (`NeedsCrossProduct`).
pub const FLAG_Y_NOT_UP: u16 = 1;
/// Runtime flag: the instance is animated.
pub const FLAG_DYNAMIC: u16 = 2;

/// A placement of an [`crate::Article`], as a **world to local** transform plus extents.
///
/// A world point `p` becomes `p.x * row_x + p.y * row_y + p.z * row_z + translation` in the
/// article's local space (row-vector convention). Local space is centred on the instance's box.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// First row of the rotation.
    pub row_x: Vec3,
    /// Half the extent along the local x axis.
    pub half_width: f32,
    /// Third row of the rotation.
    pub row_z: Vec3,
    /// Half the extent along the local z axis.
    pub half_length: f32,
    /// World to local translation (the negated, rotated world position of the centre).
    pub translation: Vec3,
    /// Radius of the circle (in xz) around the centre that contains the instance.
    pub radius: f32,
    /// Half the extent along y.
    pub half_height: f32,
    /// Build-time scratch value (an "iteration stamp"); meaningless on disk.
    pub iter_stamp: u16,
    /// [`FLAG_Y_NOT_UP`], [`FLAG_DYNAMIC`], plus the runtime exclusion bits. Zero on disk.
    pub flags: u16,
    /// Scenery group number; 0 = always present.
    pub group: u16,
    /// Index of the instance's article in the pack (`fRenderInstanceInd`).
    pub article: u16,
}

impl Instance {
    pub(crate) fn decode(r: &Reader<'_>, at: usize) -> Result<Self> {
        let w = |o: usize| r.f32(at + o);
        Ok(Self {
            row_x: [w(0)?, w(4)?, w(8)?],
            half_width: w(0xC)?,
            iter_stamp: r.u16(at + 0x10)?,
            flags: r.u16(at + 0x12)?,
            half_height: w(0x14)?,
            group: r.u16(at + 0x18)?,
            article: r.u16(at + 0x1A)?,
            row_z: [w(0x20)?, w(0x24)?, w(0x28)?],
            half_length: w(0x2C)?,
            translation: [w(0x30)?, w(0x34)?, w(0x38)?],
            radius: w(0x3C)?,
        })
    }

    /// Whether the y row is derived (`row_z x row_x`) rather than world up.
    pub fn needs_cross(&self) -> bool {
        self.flags & (FLAG_Y_NOT_UP | FLAG_DYNAMIC) != 0
    }

    /// The second row of the rotation.
    pub fn row_y(&self) -> Vec3 {
        if self.needs_cross() { cross(self.row_z, self.row_x) } else { [0.0, 1.0, 0.0] }
    }

    /// World to local.
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        let (x, y, z) = (self.row_x, self.row_y(), self.row_z);
        let m = |i: usize| p[0] * x[i] + p[1] * y[i] + p[2] * z[i] + self.translation[i];
        [m(0), m(1), m(2)]
    }

    /// Local to world (the transform is orthonormal, so this is its inverse).
    pub fn to_world(&self, l: Vec3) -> Vec3 {
        let d = sub(l, self.translation);
        [dot(d, self.row_x), dot(d, self.row_y()), dot(d, self.row_z)]
    }

    /// A direction from local to world space (no translation).
    pub fn dir_to_world(&self, d: Vec3) -> Vec3 {
        [dot(d, self.row_x), dot(d, self.row_y()), dot(d, self.row_z)]
    }

    /// World position of the instance's centre.
    pub fn position(&self) -> Vec3 {
        self.to_world([0.0; 3])
    }

    /// Radius the broad phase uses: the xz radius, or the largest extent for tilted instances.
    pub fn broad_radius(&self) -> f32 {
        if self.needs_cross() {
            self.half_width.max(self.half_length).max(self.half_height).max(self.radius)
        } else {
            self.radius
        }
    }
}
