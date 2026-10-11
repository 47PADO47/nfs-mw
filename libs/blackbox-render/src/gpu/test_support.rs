//! Shared setup for the GPU tests that run a whole [`Renderer`] headless.
//!
//! Environment switches (all optional):
//! - `BLACKBOX_TEST_API=vulkan|dx12|gl|auto`: the graphics API to test (default Vulkan);
//! - `BLACKBOX_GPU_FALLBACK=1`: use the API's software adapter (lavapipe for Vulkan).

use crate::{Backend, Renderer, RendererOptions};
use blackbox_gfx::{BackendInfo, GraphicsApi};

/// Hold this for the whole of a test that creates a GPU device (see the helper's docs).
pub(super) use blackbox_gpu_passes::test_support::serial;

/// The API the tests use.
pub(super) fn test_api() -> Backend {
    let Ok(name) = std::env::var("BLACKBOX_TEST_API") else { return Backend::Vulkan };
    name.parse().unwrap_or_else(|e| panic!("BLACKBOX_TEST_API: {e}"))
}

pub(super) fn options() -> RendererOptions {
    let fallback = std::env::var("BLACKBOX_GPU_FALLBACK").is_ok_and(|v| !v.is_empty() && v != "0");
    RendererOptions { backend: test_api(), vsync: false, force_fallback_adapter: fallback }
}

/// A headless renderer of `size`, or `None` (with a note on stderr) when this machine has no adapter
/// for the chosen API, so the test is skipped instead of failing.
pub(super) fn headless(size: (u32, u32)) -> Option<Renderer> {
    match Renderer::headless(size, options()) {
        Ok(renderer) => Some(renderer),
        Err(e) => {
            eprintln!("skipping: {e}");
            None
        }
    }
}

/// The kinds of GPU a set of recorded digests exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Family {
    /// An Intel Xe (Iris Xe, UHD, Arc) GPU on Vulkan with the Mesa driver.
    IntelXeVulkanMesa,
}

impl Family {
    /// The family of `info`, or `None` for a GPU nothing was recorded on.
    pub(super) fn of(info: &BackendInfo) -> Option<Self> {
        let adapter = info.adapter.to_ascii_lowercase();
        let intel = adapter.contains("intel");
        let xe = adapter.contains("xe") || adapter.contains("iris") || adapter.contains("arc");
        let mesa = info.driver.to_ascii_lowercase().contains("mesa");
        (info.api == GraphicsApi::Vulkan && intel && xe && mesa).then_some(Self::IntelXeVulkanMesa)
    }
}
