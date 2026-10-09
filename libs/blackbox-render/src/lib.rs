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
mod post_settings;
mod render_scale;
#[cfg(test)]
mod shader_tests;
mod upscale;

pub use blackbox_gfx::{
    Backend, BlendMode, DEFAULT_SOFT_DISTANCE, DirectionalLight, DrawRange, EffectLayer, EffectVertex, FrameParams,
    GlossyMaterial, GlossyMaterialHandle, GraphicsApi, Instance, LightingRig, MeshDesc, MeshHandle, ParseBackendError,
    ParseGraphicsApiError, PixelFormat, RenderError, Shading, SkyGradient, TextureDesc, TextureHandle, TexturedEffect,
    UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex, Vertex,
};
pub use gpu::Renderer;
pub use options::RendererOptions;
pub use post_settings::{
    Antialiasing, DEFAULT_BLOOM_THRESHOLD, MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE, MIN_EXPOSURE,
    PostEffect, PostSettings, Tonemap,
};
pub use render_scale::{DEFAULT_RENDER_SCALE, MAX_RENDER_SCALE, MIN_RENDER_SCALE, clamp_render_scale, scaled_size};
pub use upscale::{
    DEFAULT_UPSCALE_SHARPNESS, MAX_TEXTURE_LOD_BIAS, MIN_TEXTURE_LOD_BIAS, Upscaler, clamp_texture_lod_bias,
    clamp_upscale_sharpness, suggested_texture_lod_bias,
};
