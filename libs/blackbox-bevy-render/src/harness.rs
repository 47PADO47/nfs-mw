//! `HeadlessBevy`: the Bevy renderer in its own `App`, without a window, for tests and tools.
//!
//! It owns a small `App` (task pool, time, no primary window) with [`BlackboxBevyRenderPlugin`], creates the
//! backend like the game does, and runs one `App::update` for every `render()` call. Captures therefore finish
//! after a few `render()` calls, which is what `blackbox_gfx_testkit::capture_scene` does.

use bevy_app::{App, TaskPoolPlugin};
use bevy_render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy_time::TimePlugin;
use bevy_window::{ExitCondition, WindowPlugin};
use blackbox_gfx::{
    BackendInfo, Capabilities, CaptureId, EffectLayer, Environment, FrameParams, FrameStatus, GlossyMaterial,
    GlossyMaterialHandle, GraphicsApi, GraphicsSettings, Instance, LightingRig, MeshDesc, MeshHandle, RenderBackend,
    RenderError, RenderStats, Resolved, RgbaImage, TextureDesc, TextureHandle, UiLayer, UiTextureId, UiTexturePatch,
};

use crate::facade::BevyBackend;
use crate::ops::BlackboxBridge;
use crate::plugin::{BlackboxBevyRenderPlugin, backend};
use crate::probe::{ProbeError, probe_with};

pub struct HeadlessBevy {
    app: App,
    backend: BevyBackend,
}

impl HeadlessBevy {
    /// A renderer for `size`-pixel captures on `api`, or why this machine cannot run one. With
    /// `force_fallback_adapter` the API's software adapter is used (lavapipe, WARP).
    pub fn new(size: [u32; 2], api: GraphicsApi, force_fallback_adapter: bool) -> Result<Self, ProbeError> {
        // Bevy panics when it finds no adapter once the app is being built, so look first.
        probe_with(api, force_fallback_adapter)?;
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            TimePlugin,
            WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                close_when_requested: false,
                ..WindowPlugin::default()
            },
            BlackboxBevyRenderPlugin { api, force_fallback_adapter, ..Default::default() },
        ));
        app.finish();
        app.cleanup();
        let world = app.world();
        let backend = backend(
            world.resource::<BlackboxBridge>(),
            world.resource::<RenderDevice>(),
            world.resource::<RenderAdapterInfo>(),
            size,
        );
        Ok(Self { app, backend })
    }

    /// Run one update of the Bevy app: apply the queue, extract, render.
    pub fn pump(&mut self) {
        self.app.update();
    }

    /// The Bevy app, for tests that look at entities.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The Bevy app, for tests that drive it directly (spawning their own camera, running commands)
    /// rather than going through [`blackbox_gfx::RenderBackend`].
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }
}

impl RenderBackend for HeadlessBevy {
    fn info(&self) -> &BackendInfo {
        self.backend.info()
    }

    fn capabilities(&self) -> &Capabilities {
        self.backend.capabilities()
    }

    fn resize(&mut self, size: [u32; 2]) {
        self.backend.resize(size);
    }

    fn surface_size(&self) -> [u32; 2] {
        self.backend.surface_size()
    }

    fn set_vsync(&mut self, vsync: bool) {
        self.backend.set_vsync(vsync);
    }

    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        self.backend.apply_graphics(requested)
    }

    fn graphics(&self) -> &GraphicsSettings {
        self.backend.graphics()
    }

    fn render_size(&self) -> [u32; 2] {
        self.backend.render_size()
    }

    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        self.backend.create_texture(desc)
    }

    fn destroy_texture(&mut self, handle: TextureHandle) {
        self.backend.destroy_texture(handle);
    }

    fn redirect_texture(&mut self, from: TextureHandle, to: Option<TextureHandle>) {
        self.backend.redirect_texture(from, to);
    }

    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        self.backend.create_mesh(desc)
    }

    fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.backend.destroy_mesh(handle);
    }

    fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        self.backend.create_glossy_material(material)
    }

    fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        self.backend.destroy_glossy_material(handle);
    }

    fn set_lighting_rig(&mut self, rig: &LightingRig) {
        self.backend.set_lighting_rig(rig);
    }

    fn set_environment(&mut self, environment: Environment<'_>) {
        self.backend.set_environment(environment);
    }

    fn set_effects(&mut self, layer: &EffectLayer) {
        self.backend.set_effects(layer);
    }

    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>) {
        self.backend.update_ui_texture(patch);
    }

    fn free_ui_texture(&mut self, id: UiTextureId) {
        self.backend.free_ui_texture(id);
    }

    fn set_ui_layer(&mut self, layer: UiLayer) {
        self.backend.set_ui_layer(layer);
    }

    /// Queues the frame and runs one update of the app.
    fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<FrameStatus, RenderError> {
        let status = self.backend.render(frame, instances)?;
        self.pump();
        Ok(status)
    }

    fn request_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        self.backend.request_capture(size, frame, instances)
    }

    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        self.backend.poll_capture(id)
    }

    fn stats(&self) -> RenderStats {
        self.backend.stats()
    }
}
