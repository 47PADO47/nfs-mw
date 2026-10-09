//! Renderer-neutral graphics interface for EA Black Box game reimplementations.
//!
//! No GPU API and no engine dependency: this crate holds the types a game hands to a renderer,
//! the [`RenderBackend`] trait renderers implement, what a renderer can do ([`Capabilities`]) and
//! the user's graphics options with the pure rule that maps them onto what is available.

pub mod api;
pub mod effects;
pub mod error;
pub mod frame;
pub mod handles;
pub mod material;
pub mod mesh;
pub mod texture;
pub mod ui;

pub use api::{GraphicsApi, ParseGraphicsApiError};
pub use effects::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex, TexturedEffect};
pub use error::RenderError;
pub use frame::{FrameParams, Instance};
pub use handles::{CaptureId, GlossyMaterialHandle, MeshHandle, TextureHandle, UiTextureId};
pub use material::{DirectionalLight, Environment, GlossyMaterial, LightingRig, SkyGradient};
pub use mesh::{BlendMode, DrawRange, MeshDesc, Shading, Vertex};
pub use texture::{PixelFormat, TextureDesc};
pub use ui::{UiLayer, UiMesh, UiTexturePatch, UiVertex};

/// The former name of [`GraphicsApi`], kept so existing callers compile unchanged.
pub type Backend = GraphicsApi;
/// The former name of [`ParseGraphicsApiError`].
pub type ParseBackendError = ParseGraphicsApiError;
