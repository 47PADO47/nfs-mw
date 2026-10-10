//! `BlackboxBevyRenderPlugin`: the plugin set that makes a Bevy `App` a Black Box renderer, and the factory that
//! hands the game its `RenderBackend` once the GPU device exists.
//!
//! The plugin joins the game's own `App` (the window, the loop and the input stay with `bevy_winit`, ADR 0001).
//! Bevy's `RenderPlugin` creates the wgpu instance when it is built, before the window exists, and the
//! window's surface is made later by Bevy's own window systems from the raw handle `bevy_winit` provides. So
//! the plugin is added to the `App` before `run()`, and the backend is created in a system once `RenderDevice`
//! is in the main world (after the plugins' `finish`).

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetEventSystems, AssetPlugin};
use bevy_camera::CameraPlugin;
use bevy_camera::visibility::VisibilitySystems;
use bevy_core_pipeline::upscaling::upscaling;
use bevy_core_pipeline::{Core3d, Core3dSystems, CorePipelinePlugin};
use bevy_diagnostic::FrameCountPlugin;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Res, SystemParam};
use bevy_image::ImagePlugin;
use bevy_light::LightPlugin;
use bevy_mesh::MeshPlugin;
use bevy_pbr::{MaterialPlugin, PbrPlugin};
use bevy_render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy_render::settings::{RenderCreation, WgpuSettings};
use bevy_render::{ExtractSchedule, RenderApp, RenderPlugin};
use blackbox_gfx::{BackendInfo, GraphicsApi};

use crate::apply::{self, WorldState};
use crate::facade::BevyBackend;
use crate::material::BlackboxMaterial;
use crate::ops::BlackboxBridge;
use crate::probe::{api_of, backends_for};
use crate::systems::effects::{self, EffectsRender};
use crate::systems::ui::{self, UiRender};

/// Adds the Black Box renderer to an `App` that already has the task pool, time and window plugins.
#[derive(Debug, Clone, Copy)]
pub struct BlackboxBevyRenderPlugin {
    /// The graphics API to ask wgpu for.
    pub api: GraphicsApi,
    /// Use the API's software adapter (lavapipe, WARP), for CI and tests.
    pub force_fallback_adapter: bool,
}

impl Default for BlackboxBevyRenderPlugin {
    fn default() -> Self {
        Self { api: GraphicsApi::Auto, force_fallback_adapter: false }
    }
}

impl Plugin for BlackboxBevyRenderPlugin {
    fn build(&self, app: &mut App) {
        let settings = WgpuSettings {
            backends: backends_for(self.api),
            force_fallback_adapter: self.force_fallback_adapter,
            ..WgpuSettings::default()
        };
        // The render world extracts the frame count; the game's own plugins do not add it.
        if !app.is_plugin_added::<FrameCountPlugin>() {
            app.add_plugins(FrameCountPlugin);
        }
        app.add_plugins((
            AssetPlugin::default(),
            RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(settings)),
                // Frames must be reproducible (screenshots, tests): wait for pipelines instead of skipping draws.
                synchronous_pipeline_compilation: true,
                ..RenderPlugin::default()
            },
            ImagePlugin::default(),
            MeshPlugin,
            CameraPlugin,
            LightPlugin,
            CorePipelinePlugin,
            bevy_anti_alias::AntiAliasPlugin,
        ));
        // Before the material plugin: it starts loading the shader when it is built.
        crate::material::load_shaders(app);
        app.add_plugins((
            PbrPlugin { prepass_enabled: false, add_default_deferred_lighting_plugin: false, ..PbrPlugin::default() },
            MaterialPlugin::<BlackboxMaterial>::default(),
        ));
        app.init_resource::<BlackboxBridge>().init_resource::<WorldState>().add_systems(
            PostUpdate,
            apply::apply
                .before(AssetEventSystems)
                .before(VisibilitySystems::VisibilityPropagate)
                .before(VisibilitySystems::CheckVisibility),
        );

        // The render world's own copy of the bridge: the same `Arc`, so effects and the UI layer are
        // read straight from the facade's queue with no extra frame of latency from Bevy's extract step.
        let bridge = app.world().resource::<BlackboxBridge>().clone();
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .insert_resource(bridge)
            .init_resource::<EffectsRender>()
            .init_resource::<UiRender>()
            .init_resource::<effects::ExtractedWorld>()
            .add_systems(ExtractSchedule, effects::extract_world)
            .add_systems(
                Core3d,
                (
                    (effects::effects_main_pass, effects::effects_soft_pass)
                        .chain()
                        .in_set(Core3dSystems::EarlyPostProcess),
                    ui::ui_pass.after(upscaling),
                ),
            );
    }
}

/// Creates the game's `RenderBackend` from a running Bevy `App`. Use it as a system parameter in a system that
/// runs once `RenderDevice` exists, for example in `Update`.
#[derive(SystemParam)]
pub struct BackendFactory<'w> {
    bridge: Res<'w, BlackboxBridge>,
    device: Res<'w, RenderDevice>,
    adapter: Res<'w, RenderAdapterInfo>,
}

impl BackendFactory<'_> {
    /// The backend for a surface of `size` pixels. The world draws into the primary window.
    pub fn create(&self, size: [u32; 2]) -> BevyBackend {
        backend(&self.bridge, &self.device, &self.adapter, size)
    }
}

/// The backend for `bridge`, on the device and adapter Bevy created.
pub fn backend(
    bridge: &BlackboxBridge,
    device: &RenderDevice,
    adapter: &RenderAdapterInfo,
    size: [u32; 2],
) -> BevyBackend {
    let api = api_of(adapter.backend);
    let compressed_bc = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    let info = BackendInfo {
        renderer: "bevy",
        api,
        adapter: adapter.name.clone(),
        driver: format!("{} {}", adapter.driver, adapter.driver_info).trim().to_owned(),
    };
    BevyBackend::new(bridge.share(), info, crate::caps::capabilities(api, compressed_bc), size)
}
