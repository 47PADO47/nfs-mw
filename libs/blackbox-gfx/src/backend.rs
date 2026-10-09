//! The trait every renderer implements.

use crate::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderError, RenderStats,
    Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch,
};

/// A renderer: owns the GPU resources, draws frames.
///
/// Callers hold a `Box<dyn RenderBackend>` (or borrow a `&mut dyn RenderBackend`), so the trait has no
/// generic methods and no constructors. Each backend has its own factory, because creating one needs
/// things only that backend knows (window handles for the native renderer, an app for a Bevy one).
///
/// Handles are allocated by the backend synchronously, so a caller can use one right after the
/// `create_*` call that returned it, even if the backend defers the actual upload.
pub trait RenderBackend {
    /// Which renderer and GPU this is.
    fn info(&self) -> &BackendInfo;

    /// What this backend can run, fixed for its lifetime.
    fn capabilities(&self) -> &Capabilities;

    // --- surface ---------------------------------------------------------------------------------

    /// Follow a new surface size in pixels (width, height). A zero size is ignored.
    fn resize(&mut self, size: [u32; 2]);

    /// The size in pixels (width, height) of the surface the frame is presented to.
    fn surface_size(&self) -> [u32; 2];

    fn set_vsync(&mut self, vsync: bool);

    /// Surface width over height.
    fn aspect_ratio(&self) -> f32 {
        let [width, height] = self.surface_size();
        width as f32 / height.max(1) as f32
    }

    // --- graphics settings -----------------------------------------------------------------------

    /// Apply `requested`, mapped onto this backend's capabilities with [`resolve`](crate::resolve).
    /// Returns the effective settings and why anything differs from the request.
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved;

    /// The effective settings, as last returned by [`apply_graphics`](Self::apply_graphics).
    fn graphics(&self) -> &GraphicsSettings;

    /// The size in pixels (width, height) the scene is drawn at: the surface size times the effective
    /// render scale.
    fn render_size(&self) -> [u32; 2];

    // --- resources -------------------------------------------------------------------------------

    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle;
    fn destroy_texture(&mut self, handle: TextureHandle);

    /// Draw `to` wherever `from` is used, or stop redirecting `from` when `to` is `None`.
    fn redirect_texture(&mut self, from: TextureHandle, to: Option<TextureHandle>);

    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle;
    fn destroy_mesh(&mut self, handle: MeshHandle);

    /// Register a material for [`Shading::Glossy`](crate::Shading::Glossy) draws.
    fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle;
    fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle);

    // --- lighting --------------------------------------------------------------------------------

    /// The lights glossy draws use, until replaced.
    fn set_lighting_rig(&mut self, rig: &LightingRig);

    /// What glossy surfaces reflect, until replaced.
    fn set_environment(&mut self, environment: Environment<'_>);

    // --- layers ----------------------------------------------------------------------------------

    /// Replace the world effects drawn in every following frame, until replaced again.
    fn set_effects(&mut self, layer: &EffectLayer);

    /// Create, replace or partly update a UI texture.
    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>);
    fn free_ui_texture(&mut self, id: UiTextureId);

    /// The UI drawn over the scene in the next frames, at surface resolution and never post-processed.
    fn set_ui_layer(&mut self, layer: UiLayer);

    // --- frames ----------------------------------------------------------------------------------

    /// Draw `instances` and present. Instances of the same mesh should be adjacent.
    fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<FrameStatus, RenderError>;

    /// Start rendering one frame off-screen at `size` (width, height); poll the result with
    /// [`poll_capture`](Self::poll_capture). The scene is drawn like on screen, UI included.
    fn request_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError>;

    /// The captured image once it is ready, `None` while the backend is still working. A backend
    /// that completes at once returns it on the first poll; others take a few frames. Each capture
    /// is returned once.
    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>>;

    /// Live resource counts and buffer capacities.
    fn stats(&self) -> RenderStats;
}
