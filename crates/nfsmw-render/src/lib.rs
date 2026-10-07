//! Renderer for the NFS: Most Wanted rewrite.
//!
//! The public API is backend-neutral: callers hand over vertices, indices and
//! textures and never see a `wgpu` type. Today everything runs on `wgpu`, which
//! gives Vulkan, Direct3D 12 and OpenGL. Direct3D 11 is accepted by
//! [`Backend`] but needs its own implementation, because wgpu dropped its D3D11
//! backend (see `docs/architecture.md`, "Graphics backends").

mod backend;
mod wgpu_renderer;

pub use backend::Backend;
pub use wgpu_renderer::Renderer;

use glam::{Mat4, Vec3};

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

/// The game's common 36-byte vertex, uploaded as-is.
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

/// A texture to upload. `mips` holds each level, largest first.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    /// Discard pixels with alpha below one half.
    AlphaTest,
    /// Standard alpha blending, drawn after everything opaque.
    AlphaBlend,
}

/// One draw call: a range of the mesh's indices with one texture.
#[derive(Debug, Clone, Copy)]
pub struct DrawRange {
    pub first_index: u32,
    pub index_count: u32,
    /// `None` draws with a plain white texture.
    pub texture: Option<TextureHandle>,
    pub blend: BlendMode,
}

#[derive(Debug, Clone)]
pub struct MeshDesc<'a> {
    pub label: &'a str,
    pub vertices: &'a [Vertex],
    pub indices: &'a [u16],
    pub draws: Vec<DrawRange>,
}

/// Per-frame scene parameters.
#[derive(Debug, Clone, Copy)]
pub struct FrameParams {
    pub view_proj: Mat4,
    /// Direction the light travels (world space).
    pub light_dir: Vec3,
    pub clear_color: [f64; 3],
}
