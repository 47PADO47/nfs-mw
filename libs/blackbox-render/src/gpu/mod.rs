//! The wgpu implementation of the renderer (Vulkan, Direct3D 12, OpenGL).

mod capture;
mod effects;
mod frame;
mod glossy;
mod init;
mod instances;
mod meshes;
mod pipelines;
mod post;
mod resources;
mod slots;
#[cfg(test)]
mod soft_particle_tests;
mod soft_particles;
#[cfg(test)]
mod streak_tests;
mod targets;
mod textured_effects;
mod textures;
mod ui;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{RenderError, RendererOptions, clamp_render_scale, scaled_size};

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    adapter_info: wgpu::AdapterInfo,
    supports_bc: bool,
    /// The offscreen HDR scene image and depth buffer, at the internal render size.
    targets: targets::FrameTargets,
    post: post::PostChain,
    render_scale: f32,
    shared: resources::Shared,
    pipelines: pipelines::Pipelines,
    /// Texture bind groups. Slot 0 is the white fallback.
    textures: slots::Slots<wgpu::BindGroup>,
    meshes: slots::Slots<meshes::GpuMesh>,
    instances: instances::InstanceBuffer,
    effects: effects::Effects,
    glossy: glossy::Glossy,
    ui: ui::Ui,
    /// Texture slot -> slot drawn in its place (animated textures).
    redirects: std::collections::HashMap<usize, usize>,
}

impl Renderer {
    /// Create a renderer drawing into `window`, which is `size` pixels (width, height) now.
    ///
    /// `window` is anything with raw window and display handles, such as a winit window in an
    /// `Arc`. `display` is the event loop's display handle (needed by OpenGL on Wayland/X11); pass
    /// the window itself if there is nothing better.
    pub fn new<W>(
        window: W,
        size: (u32, u32),
        display: impl HasDisplayHandle + std::fmt::Debug + Send + Sync + 'static,
        options: RendererOptions,
    ) -> Result<Self, RenderError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        init::create(window, size, display, options)
    }

    /// "GPU name (backend)" for logs and the window title.
    pub fn adapter_summary(&self) -> String {
        format!("{} ({:?})", self.adapter_info.name, self.adapter_info.backend)
    }

    /// Whether DXT (BC1–3) textures can be uploaded without CPU decoding.
    pub fn supports_bc(&self) -> bool {
        self.supports_bc
    }

    /// Follow a new surface size: the surface and the offscreen targets (at the current render
    /// scale) are recreated.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.recreate_targets();
    }

    /// Draw the scene at `scale` times the surface size per axis (clamped to
    /// [`MIN_RENDER_SCALE`](crate::MIN_RENDER_SCALE)..=[`MAX_RENDER_SCALE`](crate::MAX_RENDER_SCALE);
    /// 1.0 is native) and resolve it to the surface. The UI always draws at surface resolution.
    pub fn set_render_scale(&mut self, scale: f32) {
        let scale = clamp_render_scale(scale);
        if scale == self.render_scale {
            return;
        }
        self.render_scale = scale;
        self.recreate_targets();
    }

    /// The current render scale (after clamping).
    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    /// The internal size (width, height) the scene is drawn at: the surface size times the render scale.
    pub fn render_size(&self) -> (u32, u32) {
        self.targets.size
    }

    /// The size (width, height) of the surface the frame is presented to.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Whether the scene is drawn in a 16-bit float HDR format (otherwise in the surface's format).
    pub fn is_hdr(&self) -> bool {
        self.targets.color_format == targets::HDR_FORMAT
    }

    fn recreate_targets(&mut self) {
        let size = scaled_size(self.surface_size(), self.render_scale);
        if size == self.targets.size {
            return;
        }
        self.targets = targets::FrameTargets::new(&self.device, self.targets.color_format, size);
    }

    /// Turn vertical sync on or off without recreating the renderer.
    pub fn set_vsync(&mut self, vsync: bool) {
        self.config.present_mode = if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        self.surface.configure(&self.device, &self.config);
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    /// Live meshes and textures (for stats overlays and leak checks).
    pub fn resource_counts(&self) -> (usize, usize) {
        (self.meshes.len(), self.textures.len())
    }
}
