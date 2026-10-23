//! The trait every renderer implements.

use crate::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderError, RenderStats,
    Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch, ViewCaptureUi,
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

    /// Start the CPU frame before input sampling. True means the backend owns frame limiting.
    fn begin_latency_frame(&mut self, _settings: crate::LatencySettings) -> bool {
        false
    }
    fn end_latency_simulation(&mut self) {}
    fn latency_input(&mut self) {}
    fn latency_status(&self) -> crate::LatencyStatus {
        Default::default()
    }

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

    fn frame_performance(&self) -> crate::FramePerformance {
        Default::default()
    }

    /// Session-only prototypes; unsupported renderers keep defaults.
    /// SDR is retained by backends without explicit HDR presentation support.
    fn set_display_settings(&mut self, _settings: crate::DisplaySettings) {}
    fn display_status(&self) -> crate::DisplayStatus {
        Default::default()
    }
    fn set_render_experiments(&mut self, _options: crate::RenderExperiments) {}
    fn render_experiments(&self) -> crate::RenderExperiments {
        Default::default()
    }
    fn experiment_stats(&self) -> crate::ExperimentStats {
        Default::default()
    }
    fn set_shadow_casters(&mut self, _casters: Option<&[Instance]>) {}

    /// `None` requests the caller's explicit legacy sky fallback on unsupported backends.
    fn create_sky_material(&mut self, _desc: &crate::SkyMaterialDesc) -> Option<crate::SkyMaterialHandle> {
        None
    }
    fn destroy_sky_material(&mut self, _handle: crate::SkyMaterialHandle) {}
    fn set_sky_state(&mut self, _state: crate::SkyState) {}

    /// Unsupported backends return `None`, requesting the caller's legacy fallback.
    fn create_normal_material(&mut self, _desc: &crate::NormalMaterialDesc) -> Option<crate::NormalMaterialHandle> {
        None
    }
    fn destroy_normal_material(&mut self, _handle: crate::NormalMaterialHandle) {}

    /// Unsupported backends request an explicit legacy glass fallback.
    fn create_window_material(&mut self, _desc: &crate::WindowMaterialDesc) -> Option<crate::WindowMaterialHandle> {
        None
    }
    fn destroy_window_material(&mut self, _handle: crate::WindowMaterialHandle) {}

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

    /// Set the current scene's colour profile and optional gain map; `None` clears it.
    /// The gain map is display-space RGBA: blue multiplies colour. Other channels are reserved.
    fn set_colour_treatment(&mut self, _profile: Option<&crate::ColourTreatment>, _gain: Option<&RgbaImage>) {}
    /// Install a scene mask for the vignette; `None` releases it.
    /// The mask's green times its blue adds the vignette; red and alpha are ignored.
    /// Texels with zero green preserve the source; the UI follows.
    fn set_focus_mask(&mut self, _mask: Option<&RgbaImage>) {}
    /// Update retained parameters without replacing textures or pipelines: the vignette weight and the
    /// radial blur (strength and centre). A backend without the blur ignores it; `Capabilities::radial_blur`
    /// says whether it draws it.
    fn set_focus_filter(&mut self, _filter: crate::FocusFilter) {}
    /// Capture local surroundings for glossy surfaces, or restore the supplied static environment.
    /// Backends without local captures keep their static environment.
    fn set_reflection_scene(&mut self, _scene: Option<crate::ReflectionScene<'_>>) {}
    /// Refresh the independent projected surface image. Unsupported backends retain black.
    fn set_projected_reflection_scene(&mut self, _scene: Option<crate::ProjectedReflectionScene<'_>>) {}

    /// Transparent planar image through explicitly selected diffuse textures. Unsupported
    /// backends leave the authored receiver unchanged. `None` releases the image and bindings.
    fn set_planar_reflection_scene(&mut self, _scene: Option<crate::PlanarReflectionScene<'_>>) {}
    /// Supply persistent surface water independently of reflection capture quality.
    fn set_surface_water(&mut self, _water: crate::SurfaceWater) {}

    // --- layers ----------------------------------------------------------------------------------

    /// Replace the world effects drawn in every following frame, until replaced again.
    fn set_effects(&mut self, layer: &EffectLayer);

    /// Create, replace or partly update a UI texture.
    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>);
    fn free_ui_texture(&mut self, id: UiTextureId);

    /// The UI drawn over the scene in the next frames, at surface resolution and never post-processed.
    fn set_ui_layer(&mut self, layer: UiLayer);

    /// [`set_ui_layer`](Self::set_ui_layer) for a caller that keeps its layer and sends it again every frame (the HUD
    /// with the speed streaks on top). A backend that keeps its own copy can update it in place and allocate
    /// nothing; the default clones.
    fn set_ui_layer_ref(&mut self, layer: &UiLayer) {
        self.set_ui_layer(layer.clone());
    }

    // --- frames ----------------------------------------------------------------------------------

    /// Build ahead of time what the next frames would otherwise build on first use (pipelines, shader modules,
    /// the glossy resources), for the effective graphics settings and the resources created so far, so that no
    /// frame stalls on it. Call it after the settings are applied and a scene's materials exist, for example at
    /// the end of a loading screen. Calling it again with nothing changed is cheap. Backends that build
    /// everything up front keep the empty default.
    fn prewarm(&mut self) {}

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

    /// Start rendering a picture shown inside the game (a rear-view mirror) off-screen at `size`; poll it with
    /// [`poll_capture`](Self::poll_capture). Unlike [`request_capture`](Self::request_capture) the backend may leave
    /// out screen-space effects, shadows and the UI, and may finish it a few frames later. Defaults to a full capture.
    /// Backends that never draw the UI here report it in [`Capabilities::view_capture_without_ui`](crate::Capabilities).
    fn request_view_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        self.request_capture(size, frame, instances)
    }

    /// Whether [`request_view_capture_ui`](Self::request_view_capture_ui) works on this backend. Callers that prepare
    /// something for it (the mask texture) check this first and keep using
    /// [`request_view_capture`](Self::request_view_capture) when it is `false`.
    fn view_capture_to_ui(&self) -> bool {
        false
    }

    /// Like [`request_view_capture`](Self::request_view_capture), but the picture stays on the GPU: it is tinted and
    /// masked as `target` says and lands in the UI texture `target.texture` with no readback and no polling, in
    /// time for the next frame's UI. A backend that cannot (see [`view_capture_to_ui`](Self::view_capture_to_ui))
    /// returns an error and draws nothing. A capture already in flight leaves the texture as it was.
    fn request_view_capture_ui(
        &mut self,
        _size: [u32; 2],
        _frame: &FrameParams,
        _instances: &[Instance],
        _target: &ViewCaptureUi,
    ) -> Result<(), RenderError> {
        Err(RenderError::Device("this backend cannot show a view capture as a UI texture".into()))
    }

    /// The captured image once it is ready, `None` while the backend is still working. A backend
    /// that completes at once returns it on the first poll; others take a few frames. Each capture
    /// is returned once.
    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>>;

    /// Live resource counts and buffer capacities.
    fn stats(&self) -> RenderStats;
}
