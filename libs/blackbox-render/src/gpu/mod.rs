//! The wgpu implementation of the renderer (Vulkan, Direct3D 12, OpenGL).

mod adapters;
pub(crate) mod ao;
mod backend_impl;
mod capture;
mod contact;
mod display;
mod effects;
mod filtering;
mod frame;
mod glossy;
#[cfg(test)]
mod hdr_tests;
#[cfg(test)]
mod headless_tests;
mod init;
mod instances;
#[cfg(test)]
mod lean_tests;
mod meshes;
mod normal;
mod output;
#[cfg(test)]
mod parity;
mod pipelines;
mod post;
mod projected;
#[cfg(test)]
mod reflection_tests;
mod reflections;
mod resources;
mod shadows;
mod sky;
mod slots;
mod spatial_aa;
mod surface;
mod targets;
#[cfg(test)]
mod test_support;
mod textures;
mod timing;
mod ui;
mod upscale;
mod window;
#[cfg(test)]
mod world_material_tests;

pub use adapters::available_apis;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{RenderError, RendererOptions, clamp_render_scale, scaled_size};
use targets::{HDR_FORMAT, SceneInputs, SceneTargets};

pub struct Renderer {
    adapter: wgpu::Adapter,
    display: display::Display,
    output: output::Output,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
    supports_bc: bool,
    /// Who this renderer is and what it can run, for the [`RenderBackend`](blackbox_gfx::RenderBackend) impl.
    info: blackbox_gfx::BackendInfo,
    caps: blackbox_gfx::Capabilities,
    /// The effective settings as last returned by `apply_graphics`.
    graphics: blackbox_gfx::GraphicsSettings,
    /// Finished captures nobody has polled for yet.
    captures: capture::Captures,
    /// What the scene is drawn into: the surface itself, or an offscreen image at the internal render size.
    targets: SceneTargets,
    /// The format of the offscreen image when a pass needs HDR: `Rgba16Float`, or the surface format.
    hdr_format: wgpu::TextureFormat,
    post: post::PostChain,
    reflections: Option<reflections::ReflectionCapture>,
    projected: Option<projected::ProjectedCapture>,
    experiments: blackbox_gfx::RenderExperiments,
    experiment_stats: blackbox_gfx::ExperimentStats,
    shadows: Option<shadows::Shadows>,
    smaa: Option<spatial_aa::SpatialAa>,
    shadow_casters: Option<Vec<blackbox_gfx::Instance>>,
    ao: Option<ao::AmbientOcclusion>,
    contact: Option<contact::Contact>,
    surfaces: Option<surface::Surfaces>,
    reuse_prepass_depth: bool,
    timing: Option<timing::Timing>,
    render_scale: f32,
    upscale: upscale::Upscale,
    shared: resources::Shared,
    pipelines: pipelines::Pipelines,
    /// Texture bind groups. Slot 0 is the white fallback.
    textures: slots::Slots<wgpu::BindGroup>,
    texture_views: std::collections::HashMap<usize, wgpu::TextureView>,
    meshes: slots::Slots<meshes::GpuMesh>,
    instances: instances::InstanceBuffer,
    effects: effects::Effects,
    /// Created by the first glossy call, so a renderer that never shades anything glossy pays nothing.
    glossy: Option<glossy::Glossy>,
    sky: Option<sky::Sky>,
    normal: Option<normal::Normal>,
    window: Option<window::Window>,
    ui: ui::Ui,
    /// Texture slot -> slot drawn in its place (animated textures).
    redirects: std::collections::HashMap<usize, usize>,
}

impl Renderer {
    /// Reuse the AO/contact prepass depth in the colour pass (enabled by default).
    /// Disabling provides a reference render path for image comparisons and profiling.
    pub fn set_prepass_depth_reuse(&mut self, enabled: bool) {
        self.reuse_prepass_depth = enabled;
    }
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

    /// Create a renderer with no window or surface: frames are drawn into an off-screen texture of `size`
    /// pixels (width, height), so tests and tools run on a machine without a display. Everything but
    /// presenting works as in [`Self::new`]: [`Self::render`] draws into that texture and
    /// [`Self::capture`] (or `request_capture`) reads a frame back. The texture's format is
    /// `Rgba8Unorm`. Set [`RendererOptions::force_fallback_adapter`] to pick a software adapter such
    /// as lavapipe.
    pub fn headless(size: (u32, u32), options: RendererOptions) -> Result<Self, RenderError> {
        init::create_headless(size, options)
    }

    /// "GPU name (backend)" for logs and the window title.
    pub fn adapter_summary(&self) -> String {
        format!("{} ({:?})", self.adapter_info.name, self.adapter_info.backend)
    }

    /// Whether DXT (BC1â€“3) textures can be uploaded without CPU decoding.
    pub fn supports_bc(&self) -> bool {
        self.supports_bc
    }

    /// Follow a new surface size: the surface and the scene targets (at the current render scale)
    /// are recreated.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.output.resize(&self.device, (width, height));
        self.refresh_display(true);
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
        self.output.size()
    }

    /// Whether the scene uses a 16-bit float target (effects or HDR output).
    /// This alone does not establish HDR presentation; use [`Self::display_status`].
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
            hdr: self.post.effect_settings().needs_hdr() || self.post.display.is_some(),
            surface_format: self.output.format(),
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
        self.output.set_vsync(&self.device, vsync);
    }

    pub fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.output.size();
        width as f32 / height.max(1) as f32
    }

    /// Live meshes and textures (for stats overlays and leak checks).
    pub fn resource_counts(&self) -> (usize, usize) {
        (self.meshes.len(), self.textures.len())
    }
}
