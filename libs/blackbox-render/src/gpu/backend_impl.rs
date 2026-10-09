//! The native renderer as a [`RenderBackend`].
//!
//! Every method forwards to the inherent method of the same job, so existing callers keep working
//! until they migrate to the trait. Where the two differ in shape (sizes as arrays, `FrameStatus`
//! instead of a `bool`) the conversion happens here.

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend, RenderError,
    RenderStats, Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch, resolve,
    suggested_texture_lod_bias,
};

use super::Renderer;

impl RenderBackend for Renderer {
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

    fn set_vsync(&mut self, vsync: bool) {
        Renderer::set_vsync(self, vsync);
    }

    /// Resolves `requested` against [`Capabilities`] and applies the effective settings to the
    /// post chain, the render scale, the upscaler and its sharpness. The world's texture LOD bias
    /// follows the effective render scale, as the app does for a spatial upscaler.
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        let resolved = resolve(requested, &self.caps);
        let effective = resolved.effective;
        Renderer::set_post_effects(self, effective.post);
        Renderer::set_upscaler(self, effective.upscaler);
        Renderer::set_upscale_sharpness(self, effective.upscale_sharpness);
        Renderer::set_render_scale(self, effective.render_scale);
        Renderer::set_texture_lod_bias(self, suggested_texture_lod_bias(effective.render_scale));
        self.graphics = effective;
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

    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        self.take_capture(id).map(Ok)
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
