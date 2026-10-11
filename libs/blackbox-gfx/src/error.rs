//! Errors a renderer reports.

use crate::GraphicsApi;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("could not create a rendering surface: {0}")]
    Surface(String),
    #[error("no suitable GPU adapter for {backend}: {detail}")]
    NoAdapter { backend: GraphicsApi, detail: String },
    #[error("could not create the GPU device: {0}")]
    Device(String),
}
