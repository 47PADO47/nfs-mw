//! The renderer's public, backend-neutral types.

use glam::{Mat4, Vec3};

use crate::Backend;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("{0}")]
    BackendUnavailable(String),
    #[error("could not create a rendering surface: {0}")]
    Surface(String),
    #[error("no suitable GPU adapter for {backend}: {detail}")]
    NoAdapter { backend: Backend, detail: String },
    #[error("could not create the GPU device: {0}")]
    Device(String),
}

#[derive(Debug, Clone, Copy)]
pub struct RendererOptions {
    pub backend: Backend,
    pub vsync: bool,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self { backend: Backend::Auto, vsync: true }
    }
}

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

/// Pixel formats the renderer accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// DXT1
    Bc1,
    /// DXT3
    Bc2,
    /// DXT5
    Bc3,
    Rgba8,
}

/// A texture to upload. `mips` holds each level, largest first. Block-compressed
/// textures need a width and height that are multiples of 4.
#[derive(Debug, Clone)]
pub struct TextureDesc<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub mips: Vec<&'a [u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshHandle(pub(crate) usize);

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

/// One placed copy of a mesh.
#[derive(Debug, Clone, Copy)]
pub struct Instance {
    pub mesh: MeshHandle,
    /// Object-to-world transform.
    pub transform: Mat4,
}

/// Per-frame scene parameters.
#[derive(Debug, Clone, Copy)]
pub struct FrameParams {
    pub view_proj: Mat4,
    pub camera_position: Vec3,
    /// Direction the light travels (world space).
    pub light_dir: Vec3,
    pub clear_color: [f32; 3],
    /// Linear fog between these distances from the camera, towards `clear_color`.
    /// Use `f32::MAX` for no fog.
    pub fog_start: f32,
    pub fog_end: f32,
}
