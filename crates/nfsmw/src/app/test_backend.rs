//! A renderer that draws nothing, for tests of the code around the renderer (console, menus). It reports the
//! capabilities it is given and resolves graphics requests against them like a real backend.

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend, RenderError,
    RenderStats, Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch, resolve,
};

pub struct NullBackend {
    info: BackendInfo,
    caps: Capabilities,
    settings: GraphicsSettings,
    pub mesh_labels: std::collections::HashMap<MeshHandle, String>,
    /// The live textures of a tracking backend, by handle, with their labels.
    pub texture_labels: std::collections::HashMap<TextureHandle, String>,
    track_meshes: bool,
    next_mesh: usize,
    next_texture: usize,
}

impl NullBackend {
    pub fn new(caps: Capabilities) -> Self {
        let info =
            BackendInfo { renderer: caps.renderer, api: caps.api, adapter: "Null GPU".into(), driver: String::new() };
        Self {
            info,
            caps,
            settings: GraphicsSettings::default(),
            mesh_labels: Default::default(),
            texture_labels: Default::default(),
            track_meshes: false,
            next_mesh: 0,
            next_texture: 0,
        }
    }

    /// Track the live meshes and textures (`mesh_labels`, `texture_labels`) instead of handing out dummy handles.
    pub fn with_mesh_tracking(mut self) -> Self {
        self.track_meshes = true;
        self
    }
}

impl RenderBackend for NullBackend {
    fn info(&self) -> &BackendInfo {
        &self.info
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
    fn resize(&mut self, _size: [u32; 2]) {}
    fn surface_size(&self) -> [u32; 2] {
        [1280, 720]
    }
    fn set_vsync(&mut self, _vsync: bool) {}
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        let resolved = resolve(requested, &self.caps);
        self.settings = resolved.effective;
        resolved
    }
    fn graphics(&self) -> &GraphicsSettings {
        &self.settings
    }
    fn render_size(&self) -> [u32; 2] {
        [1280, 720]
    }
    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        if !self.track_meshes {
            return TextureHandle::from_raw(0);
        }
        let handle = TextureHandle::from_raw(self.next_texture);
        self.next_texture += 1;
        self.texture_labels.insert(handle, desc.label.to_owned());
        handle
    }
    fn destroy_texture(&mut self, handle: TextureHandle) {
        self.texture_labels.remove(&handle);
    }
    fn redirect_texture(&mut self, _from: TextureHandle, _to: Option<TextureHandle>) {}
    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        if !self.track_meshes {
            return MeshHandle::from_raw(0);
        }
        let handle = MeshHandle::from_raw(self.next_mesh);
        self.next_mesh += 1;
        self.mesh_labels.insert(handle, desc.label.to_owned());
        handle
    }
    fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.mesh_labels.remove(&handle);
    }
    fn create_glossy_material(&mut self, _material: &GlossyMaterial) -> GlossyMaterialHandle {
        GlossyMaterialHandle::from_raw(0)
    }
    fn destroy_glossy_material(&mut self, _handle: GlossyMaterialHandle) {}
    fn set_lighting_rig(&mut self, _rig: &LightingRig) {}
    fn set_environment(&mut self, _environment: Environment<'_>) {}
    fn set_effects(&mut self, _layer: &EffectLayer) {}
    fn update_ui_texture(&mut self, _patch: &UiTexturePatch<'_>) {}
    fn free_ui_texture(&mut self, _id: UiTextureId) {}
    fn set_ui_layer(&mut self, _layer: UiLayer) {}
    fn render(&mut self, _frame: &FrameParams, _instances: &[Instance]) -> Result<FrameStatus, RenderError> {
        Ok(FrameStatus::Presented)
    }
    fn request_capture(
        &mut self,
        _size: [u32; 2],
        _frame: &FrameParams,
        _instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        Ok(CaptureId::from_raw(0))
    }
    fn poll_capture(&mut self, _id: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        None
    }
    fn stats(&self) -> RenderStats {
        RenderStats::default()
    }
}
