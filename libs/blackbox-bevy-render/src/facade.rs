//! `BevyBackend`: the `RenderBackend` the game talks to.
//!
//! Handles are numbered here, at once; the work happens when [`crate::apply`] runs in the Bevy schedule. What
//! this spike does not draw yet (glossy shading, the effect layer, the UI layer) is accepted and logged once.

use std::sync::{Arc, Mutex};

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend, RenderError,
    RenderStats, Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch, resolve,
    scaled_size, suggested_texture_lod_bias,
};

use crate::ops::{CameraSettings, CaptureRequest, FrameData, Op, Shared, lock};

/// Log a missing feature once per kind, not once per frame.
#[derive(Default)]
struct Warned {
    glossy: bool,
    effects: bool,
    ui: bool,
    redirect: bool,
    environment: bool,
}

pub struct BevyBackend {
    shared: Arc<Mutex<Shared>>,
    info: BackendInfo,
    caps: Capabilities,
    graphics: GraphicsSettings,
    next_texture: usize,
    next_mesh: usize,
    next_glossy: usize,
    next_capture: u64,
    textures: usize,
    meshes: usize,
    surface: [u32; 2],
    warned: Warned,
}

impl BevyBackend {
    pub(crate) fn new(shared: Arc<Mutex<Shared>>, info: BackendInfo, caps: Capabilities, surface: [u32; 2]) -> Self {
        lock(&shared).surface = surface;
        Self {
            shared,
            info,
            caps,
            graphics: GraphicsSettings::default(),
            // Handle 0 is never handed out, so it can stand for "none" in a caller's tables.
            next_texture: 1,
            next_mesh: 1,
            next_glossy: 1,
            next_capture: 1,
            textures: 0,
            meshes: 0,
            surface,
            warned: Warned::default(),
        }
    }

    fn push(&self, op: Op) {
        lock(&self.shared).ops.push(op);
    }

    fn set_camera(&self, change: impl FnOnce(&mut CameraSettings)) {
        change(&mut lock(&self.shared).settings);
    }
}

/// Say that something is not drawn yet, the first time only.
fn once(flag: &mut bool, what: &str) {
    if std::mem::replace(flag, true) {
        return;
    }
    log::warn!("bevy renderer: {what} is not implemented yet and is ignored");
}

impl RenderBackend for BevyBackend {
    fn info(&self) -> &BackendInfo {
        &self.info
    }

    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    fn resize(&mut self, size: [u32; 2]) {
        if size[0] == 0 || size[1] == 0 {
            return;
        }
        self.surface = size;
        lock(&self.shared).surface = size;
    }

    fn surface_size(&self) -> [u32; 2] {
        self.surface
    }

    fn set_vsync(&mut self, vsync: bool) {
        self.set_camera(|settings| settings.vsync = vsync);
    }

    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        let resolved = resolve(requested, &self.caps);
        let effective = resolved.effective;
        self.set_camera(|settings| {
            settings.fxaa = effective.post.antialiasing == blackbox_gfx::Antialiasing::Fxaa;
            settings.render_scale = effective.render_scale;
            settings.mip_bias = suggested_texture_lod_bias(effective.render_scale);
        });
        self.graphics = effective;
        resolved
    }

    fn graphics(&self) -> &GraphicsSettings {
        &self.graphics
    }

    fn render_size(&self) -> [u32; 2] {
        let (width, height) = scaled_size((self.surface[0], self.surface[1]), self.graphics.render_scale);
        [width, height]
    }

    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        let handle = TextureHandle::from_raw(self.next_texture);
        self.next_texture += 1;
        self.textures += 1;
        // A texture that cannot be built stays a handle without an image: draws using it are white.
        if let Some(image) = crate::texture::image(desc, self.caps.compressed_bc) {
            self.push(Op::AddTexture { handle, image: Box::new(image) });
        }
        handle
    }

    fn destroy_texture(&mut self, handle: TextureHandle) {
        self.textures = self.textures.saturating_sub(1);
        self.push(Op::RemoveTexture(handle));
    }

    fn redirect_texture(&mut self, _from: TextureHandle, _to: Option<TextureHandle>) {
        once(&mut self.warned.redirect, "texture redirection (animated textures)");
    }

    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        let handle = MeshHandle::from_raw(self.next_mesh);
        self.next_mesh += 1;
        self.meshes += 1;
        self.push(Op::AddMesh { handle, ranges: crate::mesh::split(desc) });
        handle
    }

    fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes = self.meshes.saturating_sub(1);
        self.push(Op::RemoveMesh(handle));
    }

    fn create_glossy_material(&mut self, _material: &GlossyMaterial) -> GlossyMaterialHandle {
        once(&mut self.warned.glossy, "glossy shading (draws fall back to lit)");
        let handle = GlossyMaterialHandle::from_raw(self.next_glossy);
        self.next_glossy += 1;
        handle
    }

    fn destroy_glossy_material(&mut self, _handle: GlossyMaterialHandle) {}

    fn set_lighting_rig(&mut self, _rig: &LightingRig) {
        once(&mut self.warned.glossy, "glossy shading (draws fall back to lit)");
    }

    fn set_environment(&mut self, _environment: Environment<'_>) {
        once(&mut self.warned.environment, "the glossy environment");
    }

    fn set_effects(&mut self, layer: &EffectLayer) {
        let empty = layer.surfaces.is_empty()
            && layer.particles.is_empty()
            && layer.streaks.is_empty()
            && layer.glows.is_empty()
            && layer.textured.is_empty();
        if !empty {
            once(&mut self.warned.effects, "the effect layer");
        }
    }

    fn update_ui_texture(&mut self, _patch: &UiTexturePatch<'_>) {
        once(&mut self.warned.ui, "the UI layer");
    }

    fn free_ui_texture(&mut self, _id: UiTextureId) {}

    fn set_ui_layer(&mut self, layer: UiLayer) {
        if !layer.meshes.is_empty() {
            once(&mut self.warned.ui, "the UI layer");
        }
    }

    fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<FrameStatus, RenderError> {
        lock(&self.shared).frame = Some(FrameData { frame: *frame, instances: instances.to_vec() });
        Ok(FrameStatus::Presented)
    }

    fn request_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        if size[0] == 0 || size[1] == 0 {
            return Err(RenderError::Device(format!("cannot capture a {}x{} image", size[0], size[1])));
        }
        let id = CaptureId::from_raw(self.next_capture);
        self.next_capture += 1;
        let data = FrameData { frame: *frame, instances: instances.to_vec() };
        lock(&self.shared).captures.push_back(CaptureRequest { id, size, data });
        Ok(id)
    }

    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        lock(&self.shared).finished.remove(&id.raw())
    }

    fn stats(&self) -> RenderStats {
        RenderStats { meshes: self.meshes, textures: self.textures, effect_capacities: [0; 2], streak_capacity: 0 }
    }
}
