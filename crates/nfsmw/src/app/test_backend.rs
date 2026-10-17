//! A renderer that draws nothing, for tests of the code around the renderer (console, menus). It reports the
//! capabilities it is given and resolves graphics requests against them like a real backend.

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend, RenderError,
    RenderStats, Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch, resolve,
};

#[derive(Default)]
pub struct ResourceCounts {
    pub meshes: usize,
    pub textures: usize,
    pub texture_destructions: usize,
    pub glossy_destructions: usize,
    pub sky_materials: usize,
    pub normal_materials: usize,
    pub normal_destructions: usize,
    pub window_materials: usize,
    pub window_destructions: usize,
    pub mesh_destructions: usize,
    pub sky_destructions: usize,
}

pub struct NullBackend {
    pub projected: Option<(FrameParams, [u32; 2], bool, Vec<Instance>)>,
    info: BackendInfo,
    caps: Capabilities,
    settings: GraphicsSettings,
    experiments: blackbox_gfx::RenderExperiments,
    pub lighting_rig: Option<LightingRig>,
    pub lighting_updates: usize,
    pub glossy_materials: std::collections::HashMap<GlossyMaterialHandle, GlossyMaterial>,
    track_glossy: bool,
    next_glossy: usize,
    pub mesh_labels: std::collections::HashMap<MeshHandle, String>,
    pub mesh_draws: std::collections::HashMap<MeshHandle, Vec<blackbox_gfx::DrawRange>>,
    pub mesh_tangents: std::collections::HashMap<MeshHandle, Vec<[f32; 4]>>,
    pub normal_descriptors:
        std::collections::HashMap<blackbox_gfx::NormalMaterialHandle, blackbox_gfx::NormalMaterialDesc>,
    pub window_descriptors:
        std::collections::HashMap<blackbox_gfx::WindowMaterialHandle, blackbox_gfx::WindowMaterialDesc>,
    next_texture: usize,
    textures: std::collections::HashSet<TextureHandle>,
    track_textures: bool,
    track_meshes: bool,
    next_mesh: usize,
    track_sky: bool,
    next_sky: usize,
    normal_materials: std::collections::HashSet<blackbox_gfx::NormalMaterialHandle>,
    track_normal: bool,
    next_normal: usize,
    next_window: usize,
    pub resources: std::sync::Arc<std::sync::Mutex<ResourceCounts>>,
}

impl NullBackend {
    pub fn new(caps: Capabilities) -> Self {
        let info =
            BackendInfo { renderer: caps.renderer, api: caps.api, adapter: "Null GPU".into(), driver: String::new() };
        Self {
            projected: None,
            info,
            caps,
            settings: GraphicsSettings::default(),
            experiments: Default::default(),
            lighting_rig: None,
            lighting_updates: 0,
            glossy_materials: Default::default(),
            track_glossy: false,
            next_glossy: 0,
            mesh_labels: Default::default(),
            mesh_draws: Default::default(),
            mesh_tangents: Default::default(),
            normal_descriptors: Default::default(),
            window_descriptors: Default::default(),
            next_texture: 0,
            textures: Default::default(),
            track_textures: false,
            track_meshes: false,
            next_mesh: 0,
            track_sky: false,
            next_sky: 0,
            normal_materials: Default::default(),
            track_normal: false,
            next_normal: 0,
            next_window: 0,
            resources: Default::default(),
        }
    }

    pub fn with_texture_tracking(mut self) -> Self {
        self.track_textures = true;
        self
    }

    pub fn with_mesh_tracking(mut self) -> Self {
        self.track_meshes = true;
        self
    }

    pub fn with_glossy_tracking(mut self) -> Self {
        self.track_glossy = true;
        self
    }

    pub fn with_normal_tracking(mut self) -> Self {
        self.track_normal = true;
        self
    }

    pub fn with_sky_tracking(mut self) -> Self {
        self.track_sky = true;
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
    fn set_render_experiments(&mut self, options: blackbox_gfx::RenderExperiments) {
        self.experiments = options.sanitized();
    }
    fn render_experiments(&self) -> blackbox_gfx::RenderExperiments {
        self.experiments
    }
    fn set_projected_reflection_scene(&mut self, scene: Option<blackbox_gfx::ProjectedReflectionScene<'_>>) {
        self.projected = scene.map(|scene| (scene.frame, scene.size, scene.blur, scene.instances.to_vec()));
    }
    fn create_texture(&mut self, _desc: &TextureDesc<'_>) -> TextureHandle {
        if self.track_normal || self.track_textures {
            let handle = TextureHandle::from_raw(self.next_texture);
            self.next_texture += 1;
            if self.track_textures {
                self.textures.insert(handle);
                self.resources.lock().unwrap().textures += 1;
            }
            return handle;
        }
        TextureHandle::from_raw(0)
    }
    fn destroy_texture(&mut self, handle: TextureHandle) {
        if self.textures.remove(&handle) {
            let mut counts = self.resources.lock().unwrap();
            counts.textures -= 1;
            counts.texture_destructions += 1;
        }
    }
    fn redirect_texture(&mut self, _from: TextureHandle, _to: Option<TextureHandle>) {}
    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        if !self.track_meshes {
            return MeshHandle::from_raw(0);
        }
        let handle = MeshHandle::from_raw(self.next_mesh);
        self.next_mesh += 1;
        self.mesh_labels.insert(handle, desc.label.to_owned());
        if self.track_normal {
            self.mesh_draws.insert(handle, desc.draws.clone());
            if desc.draws.iter().any(|draw| matches!(draw.shading, blackbox_gfx::Shading::WorldNormal(_))) {
                self.mesh_tangents.insert(handle, desc.tangents.unwrap_or_default().to_vec());
            }
        }
        self.resources.lock().unwrap().meshes += 1;
        handle
    }
    fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.mesh_draws.remove(&handle);
        self.mesh_tangents.remove(&handle);
        if self.mesh_labels.remove(&handle).is_some() {
            let mut counts = self.resources.lock().unwrap();
            counts.meshes -= 1;
            counts.mesh_destructions += 1;
        }
    }
    fn create_normal_material(
        &mut self,
        desc: &blackbox_gfx::NormalMaterialDesc,
    ) -> Option<blackbox_gfx::NormalMaterialHandle> {
        if !self.track_normal {
            return None;
        }
        let handle = blackbox_gfx::NormalMaterialHandle::from_raw(self.next_normal);
        self.next_normal += 1;
        self.normal_materials.insert(handle);
        self.normal_descriptors.insert(handle, *desc);
        self.resources.lock().unwrap().normal_materials += 1;
        Some(handle)
    }
    fn create_window_material(
        &mut self,
        desc: &blackbox_gfx::WindowMaterialDesc,
    ) -> Option<blackbox_gfx::WindowMaterialHandle> {
        if !self.track_normal {
            return None;
        }
        let handle = blackbox_gfx::WindowMaterialHandle::from_raw(self.next_window);
        self.next_window += 1;
        self.window_descriptors.insert(handle, *desc);
        self.resources.lock().unwrap().window_materials += 1;
        Some(handle)
    }
    fn destroy_window_material(&mut self, handle: blackbox_gfx::WindowMaterialHandle) {
        if self.window_descriptors.remove(&handle).is_some() {
            let mut counts = self.resources.lock().unwrap();
            counts.window_materials -= 1;
            counts.window_destructions += 1;
        }
    }
    fn destroy_normal_material(&mut self, handle: blackbox_gfx::NormalMaterialHandle) {
        self.normal_descriptors.remove(&handle);
        if self.normal_materials.remove(&handle) {
            let mut counts = self.resources.lock().unwrap();
            counts.normal_materials -= 1;
            counts.normal_destructions += 1;
        }
    }
    fn create_sky_material(
        &mut self,
        _desc: &blackbox_gfx::SkyMaterialDesc,
    ) -> Option<blackbox_gfx::SkyMaterialHandle> {
        if !self.track_sky {
            return None;
        }
        let handle = blackbox_gfx::SkyMaterialHandle::from_raw(self.next_sky);
        self.next_sky += 1;
        self.resources.lock().unwrap().sky_materials += 1;
        Some(handle)
    }
    fn destroy_sky_material(&mut self, _handle: blackbox_gfx::SkyMaterialHandle) {
        let mut counts = self.resources.lock().unwrap();
        counts.sky_materials -= 1;
        counts.sky_destructions += 1;
    }
    fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        if !self.track_glossy {
            return GlossyMaterialHandle::from_raw(0);
        }
        let handle = GlossyMaterialHandle::from_raw(self.next_glossy);
        self.next_glossy += 1;
        self.glossy_materials.insert(handle, *material);
        handle
    }
    fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        if self.glossy_materials.remove(&handle).is_some() {
            self.resources.lock().unwrap().glossy_destructions += 1;
        }
    }
    fn set_lighting_rig(&mut self, rig: &LightingRig) {
        self.lighting_rig = Some(*rig);
        self.lighting_updates += 1;
    }
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
