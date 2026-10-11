//! Mesh uploads: the vertex, how a draw blends and how it is shaded.

use crate::{GlossyMaterialHandle, TextureHandle};

/// The common 36-byte vertex of EA Black Box solids, uploaded as-is.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// D3DCOLOR bytes: B, G, R, A.
    pub color_bgra: [u8; 4],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    /// Discard pixels with alpha below one half.
    AlphaTest,
    /// Standard alpha blending, drawn after everything opaque.
    AlphaBlend,
    /// `src * alpha + dest`: lights, glows. Drawn last.
    Additive,
}

/// How a draw is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shading {
    /// Texture × vertex colour × (ambient + directional light): models without baked lighting (cars).
    Lit,
    /// Texture × vertex colour × 2, no dynamic light: pre-lit world geometry, whose
    /// vertex colours use 0x80 for full brightness.
    Prelit,
    /// Like [`Shading::Prelit`] but never fogged: sky domes, which sit beyond the fog.
    Sky,
    /// Texture × three-light rig, sun highlight and environment reflection, with the constants
    /// of a registered material (`create_glossy_material`) and the rig set with `set_lighting_rig`.
    /// Vehicle paint, rims and glass.
    Glossy(GlossyMaterialHandle),
}

impl Shading {
    /// Whether both use the same pipeline (every glossy material does).
    pub fn same_pipeline(self, other: Shading) -> bool {
        match (self, other) {
            (Shading::Glossy(_), Shading::Glossy(_)) => true,
            (a, b) => a == b,
        }
    }
}

/// One draw call: a range of the mesh's indices with one texture.
#[derive(Debug, Clone, Copy)]
pub struct DrawRange {
    pub first_index: u32,
    pub index_count: u32,
    /// Added to every index (for meshes made of several vertex buffers).
    pub base_vertex: i32,
    /// `None` draws with a plain white texture.
    pub texture: Option<TextureHandle>,
    pub blend: BlendMode,
    pub shading: Shading,
}

#[derive(Debug, Clone)]
pub struct MeshDesc<'a> {
    pub label: &'a str,
    pub vertices: &'a [Vertex],
    pub indices: &'a [u16],
    pub draws: Vec<DrawRange>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vertex_is_36_bytes() {
        assert_eq!(size_of::<Vertex>(), 36);
    }

    #[test]
    fn every_glossy_material_shares_one_pipeline() {
        let a = Shading::Glossy(GlossyMaterialHandle::from_raw(1));
        let b = Shading::Glossy(GlossyMaterialHandle::from_raw(2));
        assert!(a.same_pipeline(b));
        assert!(a.same_pipeline(Shading::Glossy(GlossyMaterialHandle::ANY)));
        assert!(Shading::Lit.same_pipeline(Shading::Lit));
        assert!(!Shading::Lit.same_pipeline(Shading::Prelit));
        assert!(!Shading::Sky.same_pipeline(a));
    }
}
