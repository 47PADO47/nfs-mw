//! Renderer for EA Black Box game reimplementations.
//!
//! The public API is backend-neutral: callers hand over meshes (the games'
//! common 36-byte vertex), textures and per-frame instance lists, and never see
//! a `wgpu` type. Today everything runs on `wgpu`, which gives Vulkan,
//! Direct3D 12 and OpenGL. Direct3D 11 is accepted by [`Backend`] but needs its
//! own implementation, because wgpu dropped its D3D11 backend.
//!
//! Layout:
//! - [`api`](crate::api): the types callers use;
//! - [`backend`](crate::Backend): the user-selectable graphics backend;
//! - `gpu/`: the wgpu implementation (device setup, resources, pipelines, frames).

mod api;
mod backend;
mod gpu;

pub use api::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, MeshHandle, PixelFormat, RenderError, RendererOptions,
    Shading, TextureDesc, TextureHandle, Vertex,
};
pub use backend::{Backend, ParseBackendError};
pub use gpu::Renderer;
