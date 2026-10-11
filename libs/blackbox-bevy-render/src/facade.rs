//! `BevyBackend`: the `RenderBackend` the game talks to.
//!
//! Handles are numbered here, at once; the work happens when [`crate::apply`] runs in the Bevy schedule, and
//! when the render world's Core 3D systems read [`crate::ops::Shared`] directly (see `systems/`).

use std::sync::{Arc, Mutex};

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend, RenderError,
    RenderStats, Resolved, RgbaImage, SkyGradient, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch,
    resolve, scaled_size,
};

use crate::ops::{CameraSettings, CaptureRequest, FrameData, Op, Shared, UiOp, lock};

/// Log a missing feature once per kind, not once per frame.
#[derive(Default)]
struct Warned {
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
    /// Whether an environment (the caller's own, or the lazy default below) has been queued yet.
    environment_requested: bool,
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
            environment_requested: false,
        }
    }

    fn push(&self, op: Op) {
        lock(&self.shared).ops.push(op);
    }

    fn set_camera(&self, change: impl FnOnce(&mut CameraSettings)) {
        change(&mut lock(&self.shared).settings);
    }

    /// Native lazily builds a default procedural sky the first time any glossy resource is touched
    /// (`Glossy::new`, `libs/blackbox-render/src/gpu/glossy/mod.rs`), so a caller that never calls
    /// [`RenderBackend::set_environment`] — true of every scene in `crates/nfsmw` today — still gets a
    /// believable, dim reflection instead of no reflection at all. Without an equivalent here, a glossy
    /// material's `environment` field stays `None` forever and Bevy's `AsBindGroup` substitutes its
    /// generic missing-texture fallback: an opaque *white* 1x1 cube (`Image::default()`), not native's
    /// muted sky. That washes every glossy surface's env term toward white, reading as "too shiny" and
    /// pushing saturated paint toward pink; it is worst on high-`envmap` chrome and exhaust tips, which
    /// should reflect a dim sky tint and instead reflect pure white. Called from both
    /// [`RenderBackend::create_glossy_material`] and [`RenderBackend::set_lighting_rig`], matching
    /// native's `ensure()`, which both of those also trigger.
    fn ensure_default_environment(&mut self) {
        if self.environment_requested {
            return;
        }
        self.environment_requested = true;
        let image = crate::apply::environment::build(Environment::Sky(SkyGradient::default()))
            .expect("the default sky always builds");
        self.push(Op::SetEnvironment(Box::new(image)));
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
            settings.post = effective.post;
            settings.upscaler = effective.upscaler;
            settings.upscale_sharpness = effective.upscale_sharpness;
            settings.render_scale = effective.render_scale;
            settings.mip_bias = crate::post::scale::mip_bias(effective.post.antialiasing, effective.render_scale);
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

    fn redirect_texture(&mut self, from: TextureHandle, to: Option<TextureHandle>) {
        self.push(Op::SetRedirect { from, to });
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

    fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        self.ensure_default_environment();
        let handle = GlossyMaterialHandle::from_raw(self.next_glossy);
        self.next_glossy += 1;
        self.push(Op::AddGlossyMaterial { handle, params: crate::material::GlossyUniform::of(material) });
        handle
    }

    fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        self.push(Op::RemoveGlossyMaterial(handle));
    }

    fn set_lighting_rig(&mut self, rig: &LightingRig) {
        self.ensure_default_environment();
        self.push(Op::SetLightingRig(crate::material::RigUniform::of(rig)));
    }

    fn set_environment(&mut self, environment: Environment<'_>) {
        // A caller-provided environment always wins, including over a default already queued: this
        // runs after it in the same op queue, so it is applied last regardless.
        self.environment_requested = true;
        match crate::apply::environment::build(environment) {
            Some(image) => self.push(Op::SetEnvironment(Box::new(image))),
            None => once(&mut self.warned.environment, "a too-small environment face"),
        }
    }

    fn set_effects(&mut self, layer: &EffectLayer) {
        lock(&self.shared).effects = layer.clone();
    }

    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>) {
        let op = UiOp::Update { id: patch.id, offset: patch.offset, size: patch.size, rgba: patch.rgba.to_vec() };
        lock(&self.shared).ui_ops.push(op);
    }

    fn free_ui_texture(&mut self, id: UiTextureId) {
        lock(&self.shared).ui_ops.push(UiOp::Free(id));
    }

    fn set_ui_layer(&mut self, layer: UiLayer) {
        lock(&self.shared).ui_layer = layer;
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
        let capacities = lock(&self.shared).effect_capacities.unwrap_or_default();
        RenderStats {
            meshes: self.meshes,
            textures: self.textures,
            effect_capacities: [capacities.surfaces, capacities.particles],
            streak_capacity: capacities.streaks,
        }
    }
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::GraphicsApi;

    use super::*;

    fn backend() -> BevyBackend {
        let info =
            BackendInfo { renderer: "bevy", api: GraphicsApi::Vulkan, adapter: String::new(), driver: String::new() };
        let caps = crate::caps::capabilities(GraphicsApi::Vulkan, false);
        BevyBackend::new(Arc::new(Mutex::new(Shared::default())), info, caps, [1, 1])
    }

    fn environment_ops(backend: &BevyBackend) -> usize {
        lock(&backend.shared).ops.iter().filter(|op| matches!(op, Op::SetEnvironment(_))).count()
    }

    /// Nothing in `crates/nfsmw` ever calls `set_environment` (native gets a default lazily, from
    /// `Glossy::new`; see `ensure_default_environment`'s doc comment). Without a matching default here,
    /// every glossy material's environment field stays `None` forever and samples Bevy's opaque white
    /// fallback instead, which is what made cars look "too shiny" and pushed paint towards pink/white.
    #[test]
    fn creating_a_glossy_material_queues_a_default_environment_once() {
        let mut backend = backend();
        assert_eq!(environment_ops(&backend), 0, "nothing queued before any glossy call");
        backend.create_glossy_material(&GlossyMaterial::default());
        assert_eq!(environment_ops(&backend), 1, "the lazy default is queued on first use");
        backend.create_glossy_material(&GlossyMaterial::default());
        backend.set_lighting_rig(&LightingRig::default());
        assert_eq!(environment_ops(&backend), 1, "later glossy calls do not queue it again");
    }

    #[test]
    fn setting_the_lighting_rig_alone_also_queues_the_default() {
        let mut backend = backend();
        backend.set_lighting_rig(&LightingRig::default());
        assert_eq!(environment_ops(&backend), 1);
    }

    /// A caller that does set its own environment (the testkit's `glossy` scene, unlike the real game)
    /// must never have it clobbered by the lazy default, however the two calls are ordered.
    #[test]
    fn an_explicit_environment_set_before_any_glossy_call_is_not_overwritten() {
        let mut backend = backend();
        backend.set_environment(Environment::Sky(SkyGradient::default()));
        backend.create_glossy_material(&GlossyMaterial::default());
        backend.set_lighting_rig(&LightingRig::default());
        assert_eq!(environment_ops(&backend), 1, "only the explicit call queued an environment");
    }

    #[test]
    fn an_explicit_environment_set_after_a_glossy_call_still_applies() {
        let mut backend = backend();
        backend.create_glossy_material(&GlossyMaterial::default());
        backend.set_environment(Environment::Sky(SkyGradient::default()));
        // One from the lazy default, one from the explicit call; `apply_op` applies them in order, so
        // the explicit one (queued second) is what every material ends up with.
        assert_eq!(environment_ops(&backend), 2);
    }
}
