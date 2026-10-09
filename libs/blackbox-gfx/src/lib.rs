//! Renderer-neutral graphics interface for EA Black Box game reimplementations.
//!
//! No GPU API and no engine dependency: this crate holds the types a game hands to a renderer,
//! the [`RenderBackend`] trait renderers implement, what a renderer can do ([`Capabilities`]) and
//! the user's graphics options with the pure rule that maps them onto what is available.

pub mod api;
pub mod backend;
#[cfg(test)]
mod backend_tests;
pub mod caps;
pub mod capture;
pub mod effects;
pub mod error;
pub mod frame;
pub mod handles;
pub mod material;
pub mod mesh;
pub mod settings;
pub mod stats;
pub mod texture;
pub mod ui;

pub use api::{GraphicsApi, ParseGraphicsApiError};
pub use backend::RenderBackend;
pub use caps::{
    AaSet, Capabilities, Denoiser, Downgrade, EnumSet, Note, Resolved, RestartSet, RtSupport, SetMember, Setting,
    TonemapSet, UpscalerSet, resolve,
};
pub use capture::RgbaImage;
pub use effects::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex, TexturedEffect};
pub use error::RenderError;
pub use frame::{FrameParams, FrameStatus, Instance};
pub use handles::{CaptureId, GlossyMaterialHandle, MeshHandle, TextureHandle, UiTextureId};
pub use material::{DirectionalLight, Environment, GlossyMaterial, LightingRig, SkyGradient};
pub use mesh::{BlendMode, DrawRange, MeshDesc, Shading, Vertex};
pub use settings::{
    Antialiasing, DEFAULT_BLOOM_THRESHOLD, DEFAULT_RENDER_SCALE, DEFAULT_UPSCALE_SHARPNESS, GraphicsSettings,
    MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE, MAX_RENDER_SCALE, MAX_TEXTURE_LOD_BIAS, MIN_EXPOSURE,
    MIN_RENDER_SCALE, MIN_TEXTURE_LOD_BIAS, PostEffect, PostSettings, RayTracing, Tonemap, UpscaleQuality, Upscaler,
    clamp_render_scale, clamp_texture_lod_bias, clamp_upscale_sharpness, fsr1_active, rcas_stops, scaled_size,
    suggested_temporal_texture_lod_bias, suggested_texture_lod_bias,
};
pub use stats::{BackendInfo, RenderStats};
pub use texture::{PixelFormat, TextureDesc};
pub use ui::{UiLayer, UiMesh, UiTexturePatch, UiVertex};

/// The former name of [`GraphicsApi`], kept so existing callers compile unchanged.
pub type Backend = GraphicsApi;
/// The former name of [`ParseGraphicsApiError`].
pub type ParseBackendError = ParseGraphicsApiError;
