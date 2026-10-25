//! What a converter hands to the writer.

use crate::{PositionMarker, Vertex};

/// The `SolidInfo` flags of every stock car solid but a few (0x0040). The only bit with a known meaning is 0x80
/// (sways in the wind), which a car never sets.
pub const DEFAULT_SOLID_FLAGS: u16 = 0x0040;

/// The effect id of every stock car shading group but 22 (CarShader).
pub const CAR_EFFECT_ID: u32 = 4;

/// Shading-group flags seen on about half of the stock car groups. Their meaning is not decoded.
pub const COMMON_GROUP_FLAGS: u32 = 0x4180;

/// Row-major identity, the pivot matrix of a solid that is not moved.
pub const IDENTITY: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// One material of a solid: a triangle list with its own vertices, drawn with one effect and texture set.
///
/// Only the plain 36-byte vertex layout is written, so `effect_id` must not be one of the wide ones: 1 and 3
/// (tangents, 60 bytes), 2 (skinned, 60) and 19 (second UV, 44) are refused. The ids that are safe are the ones the
/// install uses with 36-byte vertices: 4 ([`CAR_EFFECT_ID`], every stock car group), 0, 5 and 6 (world shaders).
/// Consecutive groups with the same effect id share one vertex buffer on disk; the writer takes care of that.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupSource {
    /// Indices into the solid's `texture_hashes`: diffuse, normal, height, specular, opacity.
    pub texture_slots: [u8; 5],
    /// Index into the solid's `light_material_hashes`, 0xFF for none.
    pub light_material: u8,
    pub effect_id: u32,
    pub flags: u32,
    /// The material name (`BODY_CHROME`, `WINDOW_FRONT`, ...) the car's attributes look up for this group. Stored in
    /// the solid's `MeshVltMaterials` chunks, which exist for every group or for none: when any group has a name,
    /// the others get an empty one.
    pub material_name: Option<String>,
    /// The group's bounds as the record carries them. `None` gives [`padded_bounds`] of the vertices, which is what
    /// the stock files have; add-on cars made with other tools can differ.
    pub bounds: Option<([f32; 3], [f32; 3])>,
    pub vertices: Vec<Vertex>,
    /// Triangle list. Indices count from this group's own first vertex.
    pub indices: Vec<u16>,
}

impl GroupSource {
    /// A car group (effect [`CAR_EFFECT_ID`]) using texture 0 for every slot, with no light material.
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Self {
        Self {
            texture_slots: [0; 5],
            light_material: 0xFF,
            effect_id: CAR_EFFECT_ID,
            flags: COMMON_GROUP_FLAGS,
            material_name: None,
            bounds: None,
            vertices,
            indices,
        }
    }
}

/// One solid (a car part, one level of detail) to write.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidSource {
    /// `<CAR>_<PART>_<LOD>`; its `bStringHash` is the key every other file refers to the solid by.
    pub name: String,
    pub flags: u16,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Pivot matrix, row-major.
    pub transform: [f32; 16],
    /// Polygon density the header carries (scenery LOD selection). `None` computes it the way the stock files have
    /// it: polygons divided by the volume of the bounds.
    pub density: Option<f32>,
    /// The polygon count the header carries. `None` counts the triangles of the groups, which is what the stock
    /// files have for almost every solid (a few say 1 or 2 more).
    pub polygon_count: Option<u16>,
    /// `bStringHash` of each texture the groups' slots point into.
    pub texture_hashes: Vec<u32>,
    pub light_material_hashes: Vec<u32>,
    pub markers: Vec<PositionMarker>,
    pub groups: Vec<GroupSource>,
}

impl SolidSource {
    /// A solid with the stock defaults: bounds around the groups' vertices ([`padded_bounds`]), identity pivot, no textures or markers
    /// listed yet.
    pub fn new(name: impl Into<String>, groups: Vec<GroupSource>) -> Self {
        let (bounds_min, bounds_max) = padded_bounds(groups.iter().flat_map(|g| g.vertices.iter()));
        Self {
            name: name.into(),
            flags: DEFAULT_SOLID_FLAGS,
            bounds_min,
            bounds_max,
            transform: IDENTITY,
            density: None,
            polygon_count: None,
            texture_hashes: Vec::new(),
            light_material_hashes: Vec::new(),
            markers: Vec::new(),
            groups,
        }
    }
}

/// How far the stock files' bounds, of a shading group and of a solid alike, stand outside the vertices they hold
/// (checked on the vertices of the M3 GTR: every group is 0.01 larger on all six sides).
pub const BOUNDS_MARGIN: f32 = 0.01;

/// The bounds the stock files record for some vertices: their axis-aligned box, [`BOUNDS_MARGIN`] larger on each
/// side. Both corners are the margin away from zero when there are no vertices.
pub fn padded_bounds<'a>(vertices: impl Iterator<Item = &'a Vertex>) -> ([f32; 3], [f32; 3]) {
    let (min, max) = bounds_of(vertices);
    (min.map(|v| v - BOUNDS_MARGIN), max.map(|v| v + BOUNDS_MARGIN))
}

/// Axis-aligned bounds of some vertices; both corners are zero when there are none.
fn bounds_of<'a>(vertices: impl Iterator<Item = &'a Vertex>) -> ([f32; 3], [f32; 3]) {
    let mut bounds: Option<([f32; 3], [f32; 3])> = None;
    for v in vertices {
        let (min, max) = bounds.get_or_insert((v.position, v.position));
        for axis in 0..3 {
            min[axis] = min[axis].min(v.position[axis]);
            max[axis] = max[axis].max(v.position[axis]);
        }
    }
    bounds.unwrap_or_default()
}
