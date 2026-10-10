//! Renderer for EA Black Box game reimplementations.
//!
//! The public API is backend-neutral: callers hand over meshes (the games'
//! common 36-byte vertex), textures and per-frame instance lists, and never see
//! a `wgpu` type. Everything runs on `wgpu`, which gives Vulkan, Direct3D 12
//! and OpenGL.
//!
//! Layout:
//! - [`api`](crate::api): the types callers use;
//! - [`backend`](crate::Backend): the user-selectable graphics backend;
//! - [`ui`](crate::UiLayer): the 2D layer drawn over the scene (consoles, overlays, menus);
//! - [`render_scale`](crate::scaled_size): the internal render size relative to the surface;
//! - [`post_settings`](crate::PostSettings): which post-process effects (bloom, tone mapping, FXAA) run;
//! - [`upscale`](crate::Upscaler): how a scene drawn below the surface size is brought back up (bilinear, FSR 1);
//! - `gpu/`: the wgpu implementation (device setup, resources, pipelines, the offscreen HDR
//!   targets, the post-process chain, frames).

mod api;
mod backend;
mod effects;
mod glossy;
mod gpu;
mod post_settings;
mod render_scale;
#[cfg(test)]
mod shader_tests;
mod ui;
mod upscale;

pub use api::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, MeshHandle, PixelFormat, RenderError, RendererOptions,
    Shading, TextureDesc, TextureHandle, Vertex,
};
pub use backend::{Backend, ParseBackendError};
pub use effects::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex, TexturedEffect};
pub use glossy::{DirectionalLight, GlossyMaterial, GlossyMaterialHandle, LightingRig, SkyGradient};
pub use gpu::Renderer;
pub use post_settings::{
    Antialiasing, DEFAULT_BLOOM_THRESHOLD, MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE, MIN_EXPOSURE,
    PostEffect, PostSettings, Tonemap,
};
pub use render_scale::{DEFAULT_RENDER_SCALE, MAX_RENDER_SCALE, MIN_RENDER_SCALE, clamp_render_scale, scaled_size};
pub use ui::{UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};
pub use upscale::{
    DEFAULT_UPSCALE_SHARPNESS, MAX_TEXTURE_LOD_BIAS, MIN_TEXTURE_LOD_BIAS, Upscaler, clamp_texture_lod_bias,
    clamp_upscale_sharpness, suggested_texture_lod_bias,
};
