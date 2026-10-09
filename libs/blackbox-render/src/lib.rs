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
//! - `gpu/`: the wgpu implementation (device setup, resources, pipelines, the offscreen HDR
//!   targets, the post-process chain, frames).

mod api;
mod backend;
mod effects;
mod gpu;
mod render_scale;
mod ui;

pub use api::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, MeshHandle, PixelFormat, RenderError, RendererOptions,
    Shading, TextureDesc, TextureHandle, Vertex,
};
pub use backend::{Backend, ParseBackendError};
pub use effects::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex, TexturedEffect};
pub use gpu::Renderer;
pub use render_scale::{DEFAULT_RENDER_SCALE, MAX_RENDER_SCALE, MIN_RENDER_SCALE, clamp_render_scale, scaled_size};
pub use ui::{UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};
