//! Renderer-neutral graphics interface for EA Black Box game reimplementations.
//!
//! No GPU API and no engine dependency: this crate holds the types a game hands to a renderer,
//! the [`RenderBackend`] trait renderers implement, what a renderer can do ([`Capabilities`]) and
//! the user's graphics options with the pure rule that maps them onto what is available.

pub mod api;

pub use api::{GraphicsApi, ParseGraphicsApiError};

/// The former name of [`GraphicsApi`], kept so existing callers compile unchanged.
pub type Backend = GraphicsApi;
/// The former name of [`ParseGraphicsApiError`].
pub type ParseBackendError = ParseGraphicsApiError;
