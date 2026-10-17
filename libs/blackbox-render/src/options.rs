//! How a [`Renderer`](crate::Renderer) is created.

use crate::Backend;

#[derive(Debug, Clone)]
pub struct RendererOptions {
    pub backend: Backend,
    pub vsync: bool,
    /// Ask for the API's software adapter (lavapipe for Vulkan, WARP for Direct3D 12) instead of a GPU.
    /// For CI machines and tests; a window renderer never needs it.
    pub force_fallback_adapter: bool,
    /// Directory for the Vulkan pipeline cache, which lets the next start compile fewer pipelines. The file inside
    /// is named after the adapter, so one directory serves every GPU. `None` keeps no cache on disk.
    pub pipeline_cache_dir: Option<std::path::PathBuf>,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self { backend: Backend::Auto, vsync: true, force_fallback_adapter: false, pipeline_cache_dir: None }
    }
}
