//! How a [`Renderer`](crate::Renderer) is created.

use crate::Backend;

#[derive(Debug, Clone, Copy)]
pub struct RendererOptions {
    pub backend: Backend,
    pub vsync: bool,
    /// Ask for the API's software adapter (lavapipe for Vulkan, WARP for Direct3D 12) instead of a GPU.
    /// For CI machines and tests; a window renderer never needs it.
    pub force_fallback_adapter: bool,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self { backend: Backend::Auto, vsync: true, force_fallback_adapter: false }
    }
}
