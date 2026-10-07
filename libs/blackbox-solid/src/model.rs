//! The decoded data model.

/// The common 36-byte vertex: position, normal, D3DCOLOR, uv. Wider formats
/// (60 bytes: skinning or normal mapping) start with the same 36 bytes.
pub const BASE_VERTEX_STRIDE: usize = 36;

/// One vertex's common attributes (the first 36 bytes of every vertex format).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// D3DCOLOR as stored: bytes B, G, R, A.
    pub color_bgra: [u8; 4],
    pub uv: [f32; 2],
}

/// One on-disk vertex buffer, after decoding into [`Solid::vertices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexBuffer {
    /// Bytes per vertex in the file (36, 44 or 60 seen in NFS: MW).
    pub stride: usize,
    /// Where this buffer's vertices start in [`Solid::vertices`].
    pub first_vertex: u32,
    pub vertex_count: u32,
}

/// One material within a solid: a range of indices drawn with one effect and texture set.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadingGroup {
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Indices into [`Solid::texture_hashes`]: diffuse, normal, height, specular, opacity.
    pub texture_slots: [u8; 5],
    /// Index into [`Solid::light_material_hashes`], 0xFF for none.
    pub light_material: u8,
    pub effect_id: u32,
    pub flags: u32,
    pub num_vertices: u32,
    pub num_triangles: u32,
    /// First index of this group in [`Solid::indices`].
    pub first_index: u32,
    pub num_indices: u32,
    /// Added to every index of the group to address [`Solid::vertices`]
    /// (indices are relative to the group's vertex buffer on disk).
    pub base_vertex: u32,
    /// Which entry of [`Solid::vertex_buffers`] the group draws from.
    pub vertex_buffer: u32,
}

impl ShadingGroup {
    /// Hash of the diffuse texture, if the slot points at a listed texture.
    pub fn diffuse_texture(&self, solid: &Solid) -> Option<u32> {
        solid.texture_hashes.get(usize::from(self.texture_slots[0])).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Solid {
    pub name: String,
    pub name_hash: u32,
    pub version: u8,
    pub flags: u16,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Pivot matrix, row-major as stored.
    pub transform: [f32; 16],
    /// Polygon count from the header (0 in car files; use the shading groups there).
    pub num_polys: u16,
    /// Polygon density from the header; scenery LOD selection uses it.
    pub density: f32,
    /// bStringHash of each texture the solid uses.
    pub texture_hashes: Vec<u32>,
    pub light_material_hashes: Vec<u32>,
    pub vertex_buffers: Vec<VertexBuffer>,
    /// Every vertex buffer decoded and concatenated, in buffer order.
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub groups: Vec<ShadingGroup>,
}
