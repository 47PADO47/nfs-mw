//! Tangent-space lit scenery material inputs, independent of game effect indices.

use crate::TextureHandle;

/// Arithmetic used by a tangent-space surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NormalModel {
    #[default]
    Bump,
    /// Doubled highlights, projected reflection masked by texture and vertex alpha.
    Reflective,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalMaterialDesc {
    pub model: NormalModel,
    pub diffuse: TextureHandle,
    pub normal: TextureHandle,
    pub specular: TextureHandle,
    /// Explicit projected texture, or the live projected scene for reflective materials.
    /// Without either source the backend binds black, never an unrelated environment cube.
    pub reflection: Option<TextureHandle>,
    /// Shared UV translation for all three texture samples.
    pub uv_offset: [f32; 2],
}
