//! The native renderer as a [`RenderBackend`].
//!
//! Every method forwards to the inherent method of the same job, so existing callers keep working
//! until they migrate to the trait. Where the two differ in shape (sizes as arrays, `FrameStatus`
//! instead of a `bool`) the conversion happens here.

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend, RenderError,
    RenderStats, Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch, resolve,
    suggested_temporal_texture_lod_bias, suggested_texture_lod_bias,
};

use super::Renderer;

impl RenderBackend for Renderer {
    fn set_display_settings(&mut self, settings: blackbox_gfx::DisplaySettings) {
        Renderer::set_display_settings(self, settings);
    }
    fn display_status(&self) -> blackbox_gfx::DisplayStatus {
        Renderer::display_status(self)
    }
    fn create_window_material(
        &mut self,
        desc: &blackbox_gfx::WindowMaterialDesc,
    ) -> Option<blackbox_gfx::WindowMaterialHandle> {
        Renderer::create_window_material(self, desc)
    }
    fn destroy_window_material(&mut self, handle: blackbox_gfx::WindowMaterialHandle) {
        Renderer::destroy_window_material(self, handle);
    }
    fn create_normal_material(
        &mut self,
        desc: &blackbox_gfx::NormalMaterialDesc,
    ) -> Option<blackbox_gfx::NormalMaterialHandle> {
        Renderer::create_normal_material(self, desc)
    }
    fn destroy_normal_material(&mut self, handle: blackbox_gfx::NormalMaterialHandle) {
        Renderer::destroy_normal_material(self, handle)
    }
    fn create_sky_material(&mut self, desc: &blackbox_gfx::SkyMaterialDesc) -> Option<blackbox_gfx::SkyMaterialHandle> {
        Renderer::create_sky_material(self, desc)
    }
    fn destroy_sky_material(&mut self, handle: blackbox_gfx::SkyMaterialHandle) {
        Renderer::destroy_sky_material(self, handle);
    }
    fn set_sky_state(&mut self, state: blackbox_gfx::SkyState) {
        Renderer::set_sky_state(self, state);
    }
    fn set_render_experiments(&mut self, options: blackbox_gfx::RenderExperiments) {
        let old_screen = self.experiments.screen_reflections;
        self.experiments = options.sanitized();
        if old_screen != self.experiments.screen_reflections {
            self.refresh_targets();
        }
        self.experiments.shadow_resolution =
            self.experiments.shadow_resolution.min(self.device.limits().max_texture_dimension_2d);
        self.experiments.shadow_detail_resolution =
            self.experiments.shadow_detail_resolution.min(self.device.limits().max_texture_dimension_2d);
    }
    fn render_experiments(&self) -> blackbox_gfx::RenderExperiments {
        self.experiments
    }
    fn experiment_stats(&self) -> blackbox_gfx::ExperimentStats {
        let mut stats = self.experiment_stats;
        if let Some(sample) = self.timing.as_ref().and_then(|t| t.latest) {
            stats.gpu_frame_ms = Some(sample[0]);
            stats.gpu_shadow_ms = Some(sample[1]);
            stats.gpu_contact_ms = Some(sample[2]);
            stats.gpu_ao_ms = Some(sample[3]);
        }
        stats
    }
    fn set_shadow_casters(&mut self, casters: Option<&[Instance]>) {
        if let Some(casters) = casters {
            let stored = self.shadow_casters.get_or_insert_with(Vec::new);
            stored.clear();
            stored.extend_from_slice(casters);
            return;
        }
        self.shadow_casters = None;
    }
    fn set_focus_mask(&mut self, mask: Option<&RgbaImage>) {
        self.post.set_focus_mask(&self.device, &self.queue, mask);
        self.refresh_targets();
    }
    fn set_focus_filter(&mut self, filter: blackbox_gfx::FocusFilter) {
        self.post.set_focus_filter(&self.device, filter);
    }
    fn set_colour_treatment(&mut self, profile: Option<&blackbox_gfx::ColourTreatment>, gain: Option<&RgbaImage>) {
        self.set_scene_treatment(profile, gain);
    }
    fn set_projected_reflection_scene(&mut self, scene: Option<blackbox_gfx::ProjectedReflectionScene<'_>>) {
        self.capture_projected(scene);
    }
    fn set_reflection_scene(&mut self, scene: Option<blackbox_gfx::ReflectionScene<'_>>) {
        self.capture_reflections(scene);
    }
    fn info(&self) -> &BackendInfo {
        &self.info
    }

    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    fn resize(&mut self, size: [u32; 2]) {
        Renderer::resize(self, size[0], size[1]);
    }

    fn surface_size(&self) -> [u32; 2] {
        let (width, height) = Renderer::surface_size(self);
        [width, height]
    }

    fn begin_latency_frame(&mut self, settings: blackbox_gfx::LatencySettings) -> bool {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            let output_changed = (self.presentation.request.generated_frames > 0) != (settings.generated_frames > 0);
            let pacing = self.presentation.begin(settings);
            if output_changed {
                self.refresh_display(true);
            }
            return pacing;
        }
        let _ = settings;
        false
    }
    fn end_latency_simulation(&mut self) {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.render_start();
        }
    }
    fn latency_input(&mut self) {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.marker(7);
        }
    }
    fn latency_status(&self) -> blackbox_gfx::LatencyStatus {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            return self.presentation.status;
        }
        Default::default()
    }

    fn set_vsync(&mut self, vsync: bool) {
        Renderer::set_vsync(self, vsync);
    }

    /// Resolves `requested` against [`Capabilities`] and applies the effective settings to the
    /// post chain, the render scale, the upscaler and its sharpness. The world's texture LOD bias
    /// follows the effective render scale: the spatial bias for a spatial upscaler, and one mip sharper for a
    /// temporal one, whose history supplies the samples.
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() && *requested != self.graphics {
            self.presentation.suspend(&self.device);
        }
        let resolved = resolve(requested, &self.caps);
        let effective = resolved.effective;
        #[cfg(all(windows, feature = "streamline"))]
        if effective.upscale_quality != self.graphics.upscale_quality {
            self.post.set_temporal(None);
            self.temporal.stop();
            self.graphics.upscale_quality = effective.upscale_quality;
        }
        Renderer::set_texture_filtering(self, effective.texture_filtering);
        Renderer::set_post_effects(self, effective.post);
        Renderer::set_upscaler(self, effective.upscaler);
        Renderer::set_upscale_sharpness(self, effective.upscale_sharpness);
        Renderer::set_render_scale(self, effective.render_scale);
        let bias = match effective.upscaler.is_temporal() {
            true => suggested_temporal_texture_lod_bias(effective.render_scale),
            false => suggested_texture_lod_bias(effective.render_scale),
        };
        Renderer::set_texture_lod_bias(self, bias);
        if !effective.ao.active() {
            self.ao = None;
        }
        self.graphics = effective;
        #[cfg(all(windows, feature = "streamline"))]
        self.sync_upscale_passes();
        resolved
    }

    fn graphics(&self) -> &GraphicsSettings {
        &self.graphics
    }

    fn render_size(&self) -> [u32; 2] {
        let (width, height) = Renderer::render_size(self);
        [width, height]
    }

    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        Renderer::create_texture(self, desc)
    }

    fn destroy_texture(&mut self, handle: TextureHandle) {
        Renderer::destroy_texture(self, handle);
    }

    fn redirect_texture(&mut self, from: TextureHandle, to: Option<TextureHandle>) {
        Renderer::redirect_texture(self, from, to);
    }

    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        Renderer::create_mesh(self, desc)
    }

    fn destroy_mesh(&mut self, handle: MeshHandle) {
        Renderer::destroy_mesh(self, handle);
    }

    fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        Renderer::create_glossy_material(self, material)
    }

    fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        Renderer::destroy_glossy_material(self, handle);
    }

    fn set_lighting_rig(&mut self, rig: &LightingRig) {
        Renderer::set_lighting_rig(self, rig);
    }

    fn set_environment(&mut self, environment: Environment<'_>) {
        match environment {
            Environment::Sky(sky) => self.set_environment_sky(&sky),
            Environment::Faces { size, faces } => self.set_environment_faces(size, faces),
        }
    }

    fn set_effects(&mut self, layer: &EffectLayer) {
        Renderer::set_effects(self, layer);
    }

    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>) {
        Renderer::update_ui_texture(self, patch);
    }

    fn free_ui_texture(&mut self, id: UiTextureId) {
        Renderer::free_ui_texture(self, id);
    }

    fn set_ui_layer(&mut self, layer: UiLayer) {
        Renderer::set_ui_layer(self, layer);
    }

    fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<FrameStatus, RenderError> {
        let presented = Renderer::render(self, frame, instances)?;
        Ok(if presented { FrameStatus::Presented } else { FrameStatus::Skipped })
    }

    /// The frame is drawn and read back before this returns, so the first poll has the image.
    fn request_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        self.begin_capture(size, frame, instances)
    }

    /// Reduced and not waited for: the picture is read back on a later frame (see `begin_view_capture`).
    fn request_view_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        self.begin_view_capture(size, frame, instances)
    }

    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        if let Some(image) = self.take_capture(id) {
            return Some(Ok(image));
        }
        self.poll_view_capture(id.raw())
    }

    fn frame_performance(&self) -> blackbox_gfx::FramePerformance {
        let mut performance = self.performance;
        performance.activity = self.activity.sample();
        if let Some(timing) = &self.timing {
            performance.gpu_render_ms = timing.latest.map(|sample| sample[0]);
            performance.gpu_sample = timing.completed;
        }
        performance
    }

    fn stats(&self) -> RenderStats {
        let (meshes, textures) = self.resource_counts();
        RenderStats {
            meshes,
            textures,
            effect_capacities: self.effect_capacities(),
            streak_capacity: self.streak_capacity(),
        }
    }
}
