//! The wgpu implementation of the renderer (Vulkan, Direct3D 12, OpenGL).

mod capture;
mod effects;
mod frame;
mod glossy;
mod init;
mod instances;
#[cfg(test)]
mod lean_tests;
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
mod upscale;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{RenderError, RendererOptions, clamp_render_scale, scaled_size};
use targets::{HDR_FORMAT, SceneInputs, SceneTargets};

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    adapter_info: wgpu::AdapterInfo,
    supports_bc: bool,
    /// What the scene is drawn into: the surface itself, or an offscreen image at the internal render size.
    targets: SceneTargets,
    /// The format of the offscreen image when a pass needs HDR: `Rgba16Float`, or the surface format.
    hdr_format: wgpu::TextureFormat,
    post: post::PostChain,
    render_scale: f32,
    upscale: upscale::Upscale,
    shared: resources::Shared,
    pipelines: pipelines::Pipelines,
    /// Texture bind groups. Slot 0 is the white fallback.
    textures: slots::Slots<wgpu::BindGroup>,
    meshes: slots::Slots<meshes::GpuMesh>,
    instances: instances::InstanceBuffer,
    effects: effects::Effects,
    /// Created by the first glossy call, so a renderer that never shades anything glossy pays nothing.
    glossy: Option<glossy::Glossy>,
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

    /// Follow a new surface size: the surface and the scene targets (at the current render scale)
    /// are recreated.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.refresh_targets();
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
        self.sync_upscale_passes();
    }

    /// The current render scale (after clamping).
    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    /// The internal size (width, height) the scene is drawn at: the surface size times the render scale.
    pub fn render_size(&self) -> (u32, u32) {
        self.targets.size()
    }

    /// The size (width, height) of the surface the frame is presented to.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Whether the scene is drawn in a 16-bit float HDR format. Only bloom and tone mapping ask for it;
    /// otherwise the scene is drawn in the surface's format.
    pub fn is_hdr(&self) -> bool {
        self.targets.color_format() == HDR_FORMAT
    }

    /// Whether the scene is drawn straight into the surface. That is the case while no post effect or
    /// upscaler runs and the render scale gives the surface's own size: there is then no offscreen colour
    /// image and no resolve copy, only a depth buffer.
    pub fn draws_directly(&self) -> bool {
        self.targets.offscreen().is_none()
    }

    /// The post-process passes that run each frame, in order, ending with the built-in `resolve`. They only
    /// run while the scene is drawn offscreen ([`Self::draws_directly`] is false).
    pub fn post_passes(&self) -> Vec<&'static str> {
        self.post.names()
    }

    /// Where a scene for an `output`-sized image is drawn, given the render scale and the passes in the chain.
    fn scene_plan(&self, output: (u32, u32)) -> targets::ScenePlan {
        targets::plan_scene(&SceneInputs {
            output,
            render: scaled_size(output, self.render_scale),
            passes: self.post.has_passes(),
            hdr: self.post.effect_settings().needs_hdr(),
            surface_format: self.config.format,
            hdr_format: self.hdr_format,
        })
    }

    /// Recreate the scene targets when the surface size, the render scale or the passes in the chain
    /// ask for different ones, and free the post chain's images when the scene goes straight to the surface.
    fn refresh_targets(&mut self) {
        let plan = self.scene_plan(self.surface_size());
        if self.targets.matches(&plan) {
            return;
        }
        self.targets = SceneTargets::new(&self.device, &plan);
        if plan.direct {
            self.post.release_scratch();
        }
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
