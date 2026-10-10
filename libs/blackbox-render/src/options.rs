//! How a [`Renderer`](crate::Renderer) is created.

use crate::Backend;

#[derive(Debug, Clone, Copy)]
pub struct RendererOptions {
    pub backend: Backend,
    pub vsync: bool,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self { backend: Backend::Auto, vsync: true }
    }
}
