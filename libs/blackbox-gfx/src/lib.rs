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
pub mod display;
pub mod latency;
pub use latency::{LatencySettings, LatencyStatus, ReflexMode};
pub mod distortion;
pub mod effects;
pub use distortion::{DistortionRegion, ScreenDistortion};
pub mod error;
pub mod experiments;
pub mod focus;
pub mod frame;
pub use experiments::{ExperimentStats, LightingDebug, RenderExperiments};
pub mod handles;
pub mod material;
pub mod mesh;
pub mod normal;
pub mod projected;
pub mod reflection;
pub use projected::{PlanarReflectionScene, ProjectedReflectionScene};
pub mod settings;
pub mod shadow;
pub mod sky;
pub mod stats;
pub mod texture;
pub mod treatment;
pub mod ui;
pub mod window;

pub use api::{GraphicsApi, ParseGraphicsApiError};
pub use backend::RenderBackend;
pub use caps::{
    AaSet, Capabilities, Denoiser, Downgrade, EnumSet, Note, Resolved, RestartSet, RtSupport, SetMember, Setting,
    TonemapSet, UpscalerSet, resolve,
};
pub use capture::{RgbaImage, ViewCaptureUi};
pub use display::{DisplaySettings, DisplayStatus, HdrMode};
pub use effects::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex, TexturedEffect};
pub use error::RenderError;
pub use focus::FocusFilter;
pub use frame::{
    Fog, FrameParams, FrameStatus, Instance, InstanceKey, Projection, Scattering, ShadowFlags, SurfaceLight,
};
pub use handles::{
    CaptureId, GlossyMaterialHandle, MeshHandle, NormalMaterialHandle, SkyMaterialHandle, TextureHandle, UiTextureId,
    WindowMaterialHandle,
};
pub use material::{
    DirectionalLight, Environment, GlossyMaterial, GlossyModel, GlossySurface, LightingRig, SkyGradient, SurfaceKind,
};
pub use mesh::{BlendMode, DrawRange, MeshDesc, Shading, Vertex};
pub use normal::{NormalMaterialDesc, NormalModel};
pub use reflection::{ReflectionQuality, ReflectionScene};
pub use settings::{
    Antialiasing, AoMethod, AoQuality, AoSettings, DEFAULT_BLOOM_THRESHOLD, DEFAULT_RENDER_SCALE,
    DEFAULT_UPSCALE_SHARPNESS, GraphicsSettings, MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE,
    MAX_RENDER_SCALE, MAX_TEXTURE_LOD_BIAS, MIN_EXPOSURE, MIN_RENDER_SCALE, MIN_TEXTURE_LOD_BIAS, PostEffect,
    PostSettings, RayTracing, Tonemap, UpscaleQuality, Upscaler, clamp_render_scale, clamp_texture_lod_bias,
    clamp_upscale_sharpness, fsr1_active, rcas_stops, scaled_size, suggested_temporal_texture_lod_bias,
    suggested_texture_lod_bias,
};
pub use settings::{ShadowQuality, TextureFiltering};
pub use shadow::ShadowProjection;
pub use sky::{SkyLayerKind, SkyMaterialDesc, SkyState};
pub use stats::{BackendInfo, FramePerformance, RenderStats, WorkloadActivity};
pub use texture::{PixelFormat, TextureDesc};
pub use treatment::{ColourTreatment, ColourTreatmentOverride};
pub use ui::{UiLayer, UiMesh, UiTextureComposition, UiTexturePatch, UiVertex};
pub use window::WindowMaterialDesc;

/// The former name of [`GraphicsApi`], kept so existing callers compile unchanged.
pub type Backend = GraphicsApi;
/// The former name of [`ParseGraphicsApiError`].
pub type ParseBackendError = ParseGraphicsApiError;
