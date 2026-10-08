//! `BoundsPack` (`0x8003B900`): collision bounds of cars (`GlobalB.lzc`) and props (world file).
//! Each `0x3B901` child is one object's tree of boxes and spheres plus optional point clouds.
//! Spec: `docs/formats/collision.md` ("Bounds").

use blackbox_chunk::{Chunk, ids};

use crate::Result;
use crate::bytes::{Reader, malformed};
use crate::math::Vec3;

const PACK_ALIGN: usize = 16;
const HEADER_LEN: usize = 16;
const NODE_LEN: usize = 0x30;
/// Positions and half extents are stored in millimetres.
const VECTOR_SCALE: f32 = 1000.0;
const QUAT_SCALE: f32 = 32767.0;

/// `fFlags` bits of a node.
pub mod flags {
    pub const DISABLED: u16 = 1;
    pub const PRIM_VS_WORLD: u16 = 1 << 1;
    pub const PRIM_VS_OBJECTS: u16 = 1 << 2;
    pub const PRIM_VS_GROUND: u16 = 1 << 3;
    pub const MESH_VS_GROUND: u16 = 1 << 4;
    pub const INTERNAL: u16 = 1 << 5;
    pub const BOX: u16 = 1 << 6;
    pub const SPHERE: u16 = 1 << 7;
    pub const CONSTRAINT_CONICAL: u16 = 1 << 8;
    pub const CONSTRAINT_PRISMATIC: u16 = 1 << 9;
    pub const JOINT_FEMALE: u16 = 1 << 10;
    pub const JOINT_MALE: u16 = 1 << 11;
    pub const MALE_POST: u16 = 1 << 12;
    pub const JOINT_INVERT: u16 = 1 << 13;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Box,
    Sphere,
    /// Neither bit set: a node that only groups children, or a mesh-only node.
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bounds {
    /// Quaternion `(x, y, z, w)`.
    pub orientation: [f32; 4],
    /// Offset from the parent's pivot, in metres.
    pub position: Vec3,
    /// See [`flags`].
    pub flags: u16,
    /// Box half extents (a sphere repeats its radius in all three).
    pub half_dimensions: Vec3,
    pub child_count: u8,
    /// Index into [`BoundsSet::point_clouds`] for a mesh-vs-ground node.
    pub point_cloud: Option<u8>,
    /// Absolute centre of the shape in the object's space.
    pub pivot: Vec3,
    /// Index of the first child in [`BoundsSet::nodes`].
    pub first_child: i16,
    pub radius: f32,
    /// Surface type hash (`simsurface` key); 0 = the object's default.
    pub surface: u32,
    pub name_hash: u32,
}

impl Bounds {
    pub fn shape(&self) -> Shape {
        if self.flags & flags::BOX != 0 {
            Shape::Box
        } else if self.flags & flags::SPHERE != 0 {
            Shape::Sphere
        } else {
            Shape::None
        }
    }

    fn decode(r: &Reader<'_>, at: usize) -> Result<Self> {
        let q = |o: usize| r.i16(at + o).map(|v| f32::from(v) / QUAT_SCALE);
        let m = |o: usize| r.i16(at + o).map(|v| f32::from(v) / VECTOR_SCALE);
        let pc = r.u8(at + 0x17)? as i8;
        Ok(Self {
            orientation: [q(0)?, q(2)?, q(4)?, q(6)?],
            position: [m(8)?, m(0xA)?, m(0xC)?],
            flags: r.u16(at + 0xE)?,
            half_dimensions: [m(0x10)?, m(0x12)?, m(0x14)?],
            child_count: r.u8(at + 0x16)?,
            point_cloud: u8::try_from(pc).ok(),
            pivot: [m(0x18)?, m(0x1A)?, m(0x1C)?],
            first_child: r.i16(at + 0x1E)?,
            radius: r.f32(at + 0x20)?,
            surface: r.u32(at + 0x24)?,
            name_hash: r.u32(at + 0x28)?,
        })
    }
}

/// The bounds of one object (a car or a prop).
#[derive(Debug, Clone, PartialEq)]
pub struct BoundsSet {
    /// AttribSys name hash of the car type name (cars) or the object name hash (props).
    pub name_hash: u32,
    /// Node 0 is the root; children are contiguous runs ([`Bounds::first_child`]).
    pub nodes: Vec<Bounds>,
    /// Convex-hull points (xyz; w is 0) used by mesh-vs-ground nodes.
    pub point_clouds: Vec<Vec<[f32; 3]>>,
}

impl BoundsSet {
    /// Parses the payload of a `0x3B901` chunk (as returned by `Chunk::aligned_payload(16)`).
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let r = Reader::new(payload, "bounds");
        let count = usize::try_from(r.u32(4)?).unwrap_or(usize::MAX);
        if count == 0 || count > 4096 {
            return Err(malformed("bounds", format!("{count} nodes")));
        }
        let nodes = (0..count).map(|i| Bounds::decode(&r, HEADER_LEN + i * NODE_LEN)).collect::<Result<Vec<_>>>()?;
        for (i, n) in nodes.iter().enumerate() {
            let first = usize::try_from(n.first_child).unwrap_or(usize::MAX);
            if n.child_count > 0 && first.saturating_add(usize::from(n.child_count)) > count {
                return Err(malformed("bounds", format!("node {i}: children {first}+{} of {count}", n.child_count)));
            }
        }
        let mut at = HEADER_LEN + count * NODE_LEN;
        let clouds = usize::try_from(r.u32(at)?).unwrap_or(usize::MAX);
        if clouds > 256 {
            return Err(malformed("bounds", format!("{clouds} point clouds")));
        }
        at += HEADER_LEN;
        let mut point_clouds = Vec::with_capacity(clouds);
        for _ in 0..clouds {
            let n = usize::try_from(r.u32(at)?).unwrap_or(usize::MAX);
            at += HEADER_LEN;
            let pts = (0..n).map(|i| r.vec3(at + i * 16)).collect::<Result<Vec<_>>>()?;
            at += n * 16;
            point_clouds.push(pts);
        }
        Ok(Self { name_hash: r.u32(0)?, nodes, point_clouds })
    }

    pub fn from_chunk(chunk: &Chunk<'_>) -> Result<Self> {
        Self::parse(chunk.aligned_payload(PACK_ALIGN))
    }

    pub fn root(&self) -> &Bounds {
        &self.nodes[0]
    }

    /// The children of `node` (empty for a leaf).
    pub fn children<'a>(&'a self, node: &Bounds) -> &'a [Bounds] {
        let first = usize::try_from(node.first_child).unwrap_or(0);
        self.nodes.get(first..first + usize::from(node.child_count)).unwrap_or(&[])
    }

    /// Whether the nodes form a tree reachable from the root, each node once.
    pub fn is_tree(&self) -> bool {
        let mut seen = vec![false; self.nodes.len()];
        seen[0] = true;
        for node in &self.nodes {
            let first = usize::try_from(node.first_child).unwrap_or(0);
            for slot in seen.iter_mut().skip(first).take(usize::from(node.child_count)) {
                if std::mem::replace(slot, true) {
                    return false;
                }
            }
        }
        seen.iter().all(|&s| s)
    }
}

/// Every bounds set in `data` (a file with a `BoundsPack`), in file order.
pub fn read_bounds_sets(data: &[u8]) -> Result<Vec<BoundsSet>> {
    blackbox_chunk::find_all(data, ids::BOUNDS).iter().map(BoundsSet::from_chunk).collect()
}

/// The set for one name hash.
pub fn find_bounds(sets: &[BoundsSet], name_hash: u32) -> Option<&BoundsSet> {
    sets.iter().find(|s| s.name_hash == name_hash)
}
