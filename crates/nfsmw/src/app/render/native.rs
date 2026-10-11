//! The native Black Box renderer (`blackbox-render`): the only place the app names that crate.

use bevy_window::RawHandleWrapper;
use bevy_winit::DisplayHandleWrapper;
use blackbox_gfx::RenderBackend;
use blackbox_render::{Renderer, RendererOptions};

use crate::settings::Settings;

/// Create the native renderer for the window behind `raw`, `size` pixels large.
#[allow(unsafe_code)]
pub fn create(
    raw: &RawHandleWrapper,
    size: (u32, u32),
    display: &DisplayHandleWrapper,
    settings: &Settings,
) -> anyhow::Result<Box<dyn RenderBackend>> {
    let options = RendererOptions { backend: settings.backend, vsync: settings.vsync, ..RendererOptions::default() };
    // SAFETY: this runs in a system that takes `NonSendMut`, so it is on the main thread, which is
    // what `get_handle` requires.
    let handle = unsafe { raw.get_handle() };
    let renderer = Renderer::new(handle, size, display.0.clone(), options)?;
    Ok(Box::new(renderer))
}
