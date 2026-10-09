//! CPU checks of the scenes against a backend that only records what it is given.

use std::collections::HashMap;

use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsApi, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, PixelFormat,
    RenderBackend, RenderError, RenderStats, Resolved, RgbaImage, Shading, TextureDesc, TextureHandle, UiLayer,
    UiTextureId, UiTexturePatch, resolve,
};

use super::*;

/// Records resources, and completes a capture after `polls_needed` polls (rendering a frame in between).
struct Recorder {
    info: BackendInfo,
    caps: Capabilities,
    settings: GraphicsSettings,
    next: usize,
    textures: HashMap<usize, PixelFormat>,
    meshes: HashMap<usize, (usize, usize, Vec<blackbox_gfx::DrawRange>)>,
    glossy: usize,
    ui_textures: HashMap<u64, [u32; 2]>,
    effects: usize,
    ui_meshes: usize,
    rig_set: bool,
    frames: usize,
    polls: usize,
    polls_needed: usize,
}

impl Recorder {
    fn new(compressed_bc: bool, polls_needed: usize) -> Self {
        let mut caps = Capabilities::baseline("recorder", GraphicsApi::Vulkan);
        caps.compressed_bc = compressed_bc;
        let info = BackendInfo {
            renderer: "recorder",
            api: GraphicsApi::Vulkan,
            adapter: "none".into(),
            driver: String::new(),
        };
        Self {
            info,
            caps,
            settings: GraphicsSettings::default(),
            next: 1,
            textures: HashMap::new(),
            meshes: HashMap::new(),
            glossy: 0,
            ui_textures: HashMap::new(),
            effects: 0,
            ui_meshes: 0,
            rig_set: false,
            frames: 0,
            polls: 0,
            polls_needed,
        }
    }

    fn live(&self) -> usize {
        self.textures.len() + self.meshes.len() + self.glossy + self.ui_textures.len()
    }
}

impl RenderBackend for Recorder {
    fn info(&self) -> &BackendInfo {
        &self.info
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
    fn resize(&mut self, _: [u32; 2]) {}
    fn surface_size(&self) -> [u32; 2] {
        [256, 144]
    }
    fn set_vsync(&mut self, _: bool) {}
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        let resolved = resolve(requested, &self.caps);
        self.settings = resolved.effective;
        resolved
    }
    fn graphics(&self) -> &GraphicsSettings {
        &self.settings
    }
    fn render_size(&self) -> [u32; 2] {
        [256, 144]
    }
    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        self.next += 1;
        self.textures.insert(self.next, desc.format);
        TextureHandle::from_raw(self.next)
    }
    fn destroy_texture(&mut self, handle: TextureHandle) {
        assert!(self.textures.remove(&handle.raw()).is_some(), "destroyed twice or never created");
    }
    fn redirect_texture(&mut self, _: TextureHandle, _: Option<TextureHandle>) {}
    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        self.next += 1;
        self.meshes.insert(self.next, (desc.vertices.len(), desc.indices.len(), desc.draws.clone()));
        MeshHandle::from_raw(self.next)
    }
    fn destroy_mesh(&mut self, handle: MeshHandle) {
        assert!(self.meshes.remove(&handle.raw()).is_some(), "destroyed twice or never created");
    }
    fn create_glossy_material(&mut self, _: &GlossyMaterial) -> GlossyMaterialHandle {
        self.next += 1;
        self.glossy += 1;
        GlossyMaterialHandle::from_raw(self.next)
    }
    fn destroy_glossy_material(&mut self, _: GlossyMaterialHandle) {
        self.glossy -= 1;
    }
    fn set_lighting_rig(&mut self, _: &LightingRig) {
        self.rig_set = true;
    }
    fn set_environment(&mut self, _: Environment<'_>) {}
    fn set_effects(&mut self, layer: &EffectLayer) {
        self.effects = layer.surfaces.len()
            + layer.particles.len()
            + layer.streaks.len()
            + layer.glows.len()
            + layer.textured.len();
    }
    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>) {
        assert_eq!(patch.rgba.len(), (patch.size[0] * patch.size[1] * 4) as usize);
        if patch.offset.is_none() {
            self.ui_textures.insert(patch.id.raw(), patch.size);
        }
        let known = self.ui_textures.get(&patch.id.raw()).expect("a partial patch needs its texture");
        let [x, y] = patch.offset.unwrap_or([0, 0]);
        assert!(x + patch.size[0] <= known[0] && y + patch.size[1] <= known[1], "the patch fits");
    }
    fn free_ui_texture(&mut self, id: UiTextureId) {
        assert!(self.ui_textures.remove(&id.raw()).is_some());
    }
    fn set_ui_layer(&mut self, layer: UiLayer) {
        self.ui_meshes = layer.meshes.len();
        for mesh in &layer.meshes {
            assert!(self.ui_textures.contains_key(&mesh.texture.raw()), "a UI mesh names a live texture");
        }
    }
    fn render(&mut self, _: &FrameParams, _: &[Instance]) -> Result<FrameStatus, RenderError> {
        self.frames += 1;
        Ok(FrameStatus::Presented)
    }
    fn request_capture(&mut self, _: [u32; 2], _: &FrameParams, _: &[Instance]) -> Result<CaptureId, RenderError> {
        self.polls = 0;
        Ok(CaptureId::from_raw(1))
    }
    fn poll_capture(&mut self, _: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        self.polls += 1;
        (self.polls > self.polls_needed).then(|| RgbaImage::new(4, 4, vec![0; 64]))
    }
    fn stats(&self) -> RenderStats {
        RenderStats::default()
    }
}

#[test]
fn every_scene_builds_valid_geometry_and_releases_everything() {
    for scene in SceneId::ALL {
        let mut backend = Recorder::new(true, 0);
        let built = scene.build(&mut backend, [256, 144]);
        assert!(!built.instances.is_empty(), "{}", scene.name());
        for instance in &built.instances {
            let (vertices, indices, draws) = backend.meshes.get(&instance.mesh.raw()).expect("a live mesh");
            assert!(!draws.is_empty() && *vertices > 0);
            for d in draws {
                assert!(
                    (d.first_index + d.index_count) as usize <= *indices,
                    "{}: draw range inside the indices",
                    scene.name()
                );
                assert_eq!(d.index_count % 3, 0);
                if let Some(t) = d.texture {
                    assert!(backend.textures.contains_key(&t.raw()), "{}: draws use live textures", scene.name());
                }
            }
        }
        assert!(built.frame.view_proj().is_finite());
        built.release(&mut backend);
        assert_eq!(backend.live(), 0, "{}: nothing left behind", scene.name());
        assert_eq!((backend.effects, backend.ui_meshes), (0, 0), "{}: layers cleared", scene.name());
    }
}

#[test]
fn scenes_are_deterministic() {
    for scene in SceneId::ALL {
        let a = scene.build(&mut Recorder::new(true, 0), [256, 144]);
        let b = scene.build(&mut Recorder::new(true, 0), [256, 144]);
        assert_eq!(a.frame.view_proj(), b.frame.view_proj());
        assert_eq!(a.instances.len(), b.instances.len());
        assert_eq!(a.effects.surfaces, b.effects.surfaces);
    }
}

#[test]
fn bc1_textures_become_rgba8_on_a_backend_without_bc() {
    for compressed in [true, false] {
        let mut backend = Recorder::new(compressed, 0);
        SceneId::Grid.build(&mut backend, [256, 144]);
        let bc = backend.textures.values().filter(|f| **f == PixelFormat::Bc1).count();
        assert_eq!(bc > 0, compressed);
        assert!(backend.textures.values().any(|f| *f == PixelFormat::Rgba8));
    }
}

#[test]
fn only_the_scenes_that_need_them_set_layers_and_the_rig() {
    for scene in SceneId::ALL {
        let mut backend = Recorder::new(true, 0);
        let built = scene.build(&mut backend, [256, 144]);
        built.apply_layers(&mut backend);
        assert_eq!(backend.effects > 0, scene == SceneId::Effects, "{}", scene.name());
        assert_eq!(backend.ui_meshes > 0, scene == SceneId::Ui, "{}", scene.name());
        assert_eq!(backend.rig_set, scene == SceneId::GlossySphere, "{}", scene.name());
        let glossy_draws =
            backend.meshes.values().flat_map(|m| &m.2).filter(|d| matches!(d.shading, Shading::Glossy(_))).count();
        assert_eq!(glossy_draws > 0, scene == SceneId::GlossySphere, "{}", scene.name());
    }
}

#[test]
fn capture_scene_renders_frames_until_the_capture_is_ready() {
    let mut backend = Recorder::new(true, 3);
    let image = capture_scene(&mut backend, SceneId::Grid, [256, 144]).unwrap();
    assert_eq!((image.width, image.height), (4, 4));
    assert_eq!(backend.frames, 3, "one frame between each unsuccessful poll");
    assert_eq!(backend.live(), 0, "released afterwards");
}

#[test]
fn a_capture_that_never_completes_is_an_error() {
    let mut backend = Recorder::new(true, usize::MAX);
    assert!(capture_scene(&mut backend, SceneId::Ui, [256, 144]).is_err());
    assert_eq!(backend.live(), 0, "cleaned up even on failure");
}

#[test]
fn scene_names_are_unique_and_fidelity_matches_the_plan() {
    let names: std::collections::HashSet<_> = SceneId::ALL.iter().map(|s| s.name()).collect();
    assert_eq!(names.len(), SceneId::ALL.len());
    assert_eq!(SceneId::DepthProbe.fidelity(), Fidelity::Strict);
    assert_eq!(SceneId::BlendStack.fidelity().tolerance().mean, 4.0);
    assert_eq!(SceneId::Ui.fidelity().tolerance().max, 1);
    assert_eq!(SceneId::GlossySphere.fidelity().tolerance().p99, 8);
}
