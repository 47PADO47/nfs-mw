//! Renderer for EA Black Box game reimplementations.
//!
//! The public API is backend-neutral: callers hand over meshes (the games'
//! common 36-byte vertex), textures and per-frame instance lists, and never see
//! a `wgpu` type. Everything runs on `wgpu`, which gives Vulkan, Direct3D 12
//! and OpenGL.
//!
//! The renderer-neutral types (meshes, textures, frame parameters, effect and UI layers, post and
//! upscale settings, the graphics API enum) live in `blackbox-gfx` and are re-exported here, so
//! callers keep importing them from this crate.
//!
//! Layout:
//! - [`options`](crate::RendererOptions): how a renderer is created;
//! - `gpu/`: the wgpu implementation (device setup, resources, pipelines, the scene targets (the
//!   surface itself, or an offscreen image), the post-process chain, frames).

mod gpu;
mod options;
#[cfg(test)]
mod shader_tests;

pub use blackbox_gfx::{
    Antialiasing, Backend, BlendMode, DEFAULT_BLOOM_THRESHOLD, DEFAULT_RENDER_SCALE, DEFAULT_SOFT_DISTANCE,
    DEFAULT_UPSCALE_SHARPNESS, DirectionalLight, DrawRange, EffectLayer, EffectVertex, FrameParams, GlossyMaterial,
    GlossyMaterialHandle, GraphicsApi, Instance, LightingRig, MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE,
    MAX_RENDER_SCALE, MAX_TEXTURE_LOD_BIAS, MIN_EXPOSURE, MIN_RENDER_SCALE, MIN_TEXTURE_LOD_BIAS, MeshDesc, MeshHandle,
    ParseBackendError, ParseGraphicsApiError, PixelFormat, PostEffect, PostSettings, RenderError, Shading, SkyGradient,
    TextureDesc, TextureHandle, TexturedEffect, Tonemap, UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex,
    Upscaler, Vertex, clamp_render_scale, clamp_texture_lod_bias, clamp_upscale_sharpness, scaled_size,
    suggested_texture_lod_bias,
};
pub use gpu::Renderer;
pub use options::RendererOptions;
