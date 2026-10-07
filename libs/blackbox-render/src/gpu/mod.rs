//! The wgpu implementation of the renderer (Vulkan, Direct3D 12, OpenGL).

mod capture;
mod frame;
mod init;
mod instances;
mod meshes;
mod pipelines;
mod resources;
mod slots;
mod textures;

use std::sync::Arc;

use winit::window::Window;

use crate::{RenderError, RendererOptions};

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    adapter_info: wgpu::AdapterInfo,
    supports_bc: bool,
    depth: wgpu::TextureView,
    shared: resources::Shared,
    pipelines: pipelines::Pipelines,
    /// Texture bind groups. Slot 0 is the white fallback.
    textures: slots::Slots<wgpu::BindGroup>,
    meshes: slots::Slots<meshes::GpuMesh>,
    instances: instances::InstanceBuffer,
}

impl Renderer {
    /// Create a renderer drawing into `window`.
    ///
    /// `display` is the event loop's display handle (needed by OpenGL on Wayland/X11).
    pub fn new(
        window: Arc<Window>,
        display: winit::event_loop::OwnedDisplayHandle,
        options: RendererOptions,
    ) -> Result<Self, RenderError> {
        init::create(window, display, options)
    }

    /// "GPU name (backend)" for logs and the window title.
    pub fn adapter_summary(&self) -> String {
        format!("{} ({:?})", self.adapter_info.name, self.adapter_info.backend)
    }

    /// Whether DXT (BC1–3) textures can be uploaded without CPU decoding.
    pub fn supports_bc(&self) -> bool {
        self.supports_bc
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth = resources::create_depth(&self.device, width, height);
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    /// Live meshes and textures (for stats overlays and leak checks).
    pub fn resource_counts(&self) -> (usize, usize) {
        (self.meshes.len(), self.textures.len())
    }
}
