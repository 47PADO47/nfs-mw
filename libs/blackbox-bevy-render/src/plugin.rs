//! `BlackboxBevyRenderPlugin`: the plugin set that makes a Bevy `App` a Black Box renderer, and the factory that
//! hands the game its `RenderBackend` once the GPU device exists.
//!
//! The plugin joins the game's own `App` (the window, the loop and the input stay with `bevy_winit`, ADR 0001).
//! Bevy's `RenderPlugin` creates the wgpu instance when it is built, before the window exists, and the
//! window's surface is made later by Bevy's own window systems from the raw handle `bevy_winit` provides. So
//! the plugin is added to the `App` before `run()`, and the backend is created in a system once `RenderDevice`
//! is in the main world (after the plugins' `finish`).

#[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
use bevy_anti_alias::dlss::{
    DlssInitPlugin, DlssRayReconstructionSupported as RrSupport, DlssSuperResolutionSupported as DlssSupport,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetEventSystems, AssetPlugin};
use bevy_camera::CameraPlugin;
use bevy_camera::visibility::VisibilitySystems;
use bevy_core_pipeline::tonemapping::tonemapping;
use bevy_core_pipeline::upscaling::upscaling;
use bevy_core_pipeline::{Core3d, Core3dSystems, CorePipelinePlugin};
use bevy_diagnostic::FrameCountPlugin;
#[cfg(not(all(feature = "dlss", not(feature = "dlss-mock"))))]
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Res, SystemParam};
use bevy_image::ImagePlugin;
use bevy_light::LightPlugin;
use bevy_mesh::MeshPlugin;
use bevy_pbr::{MaterialPlugin, PbrPlugin};
use bevy_render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy_render::settings::{RenderCreation, WgpuSettings};
use bevy_render::{ExtractSchedule, RenderApp, RenderPlugin};
use blackbox_gfx::{BackendInfo, GraphicsApi, RtSupport};

use crate::apply::{self, WorldState};
use crate::facade::BevyBackend;
use crate::material::BlackboxMaterial;
use crate::ops::BlackboxBridge;
use crate::post;
use crate::post::fsr1::{self, Fsr1Render};
use crate::post::radial_blur::{self, RadialBlurRender};
use crate::probe::{api_of, backends_for};
use crate::systems::effects::{self, EffectsRender};
use crate::systems::ui::{self, UiRender};

/// Without the DLSS SDK path there is no Bevy resource to find: this is never inserted, so the factory sees no DLSS.
#[cfg(not(all(feature = "dlss", not(feature = "dlss-mock"))))]
#[derive(Resource)]
struct DlssSupport;

/// Without the DLSS SDK path there is no Bevy resource to find for Ray Reconstruction either.
#[cfg(not(all(feature = "dlss", not(feature = "dlss-mock"))))]
#[derive(Resource)]
struct RrSupport;

/// Adds the Black Box renderer to an `App` that already has the task pool, time and window plugins.
#[derive(Debug, Clone, Copy)]
pub struct BlackboxBevyRenderPlugin {
    /// The graphics API to ask wgpu for.
    pub api: GraphicsApi,
    /// Use the API's software adapter (lavapipe, WARP), for CI and tests.
    pub force_fallback_adapter: bool,
    /// Wait for pipeline compilation instead of skipping draws. Needed for reproducible captures and
    /// tests; the live window sets it false so steady-state frames never block on compilation.
    pub synchronous_pipeline_compilation: bool,
    /// Ray-traced lighting (Solari) from the start. The Solari plugins are added only when this is set and the
    /// adapter has the features they need (`solari::install`); the setting is restart-required for that reason.
    /// Without the `ray-tracing` feature it does nothing but log.
    pub ray_tracing: bool,
}

impl Default for BlackboxBevyRenderPlugin {
    fn default() -> Self {
        Self {
            api: GraphicsApi::Auto,
            force_fallback_adapter: false,
            synchronous_pipeline_compilation: true,
            ray_tracing: false,
        }
    }
}

/// Add Solari when it is requested and the adapter can run it (`solari.rs`). Returns whether it was added.
#[cfg(feature = "ray-tracing")]
fn install_solari(app: &mut App, requested: bool, api: GraphicsApi, force_fallback: bool) -> bool {
    crate::solari::install(app, requested, api, force_fallback)
}

/// Without the `ray-tracing` feature there is no Solari to add.
#[cfg(not(feature = "ray-tracing"))]
fn install_solari(_app: &mut App, requested: bool, _api: GraphicsApi, _force_fallback: bool) -> bool {
    if requested {
        log::warn!("ray tracing: this build has no Solari (it needs the ray-tracing cargo feature)");
    }
    false
}

/// The ray tracing the device offers, for the caps (`solari::support`).
#[cfg(feature = "ray-tracing")]
fn ray_tracing_of(device: &RenderDevice, rr_supported: bool) -> RtSupport {
    crate::solari::support(device.features(), rr_supported)
}

/// Without the `ray-tracing` feature the device offers no ray tracing.
#[cfg(not(feature = "ray-tracing"))]
fn ray_tracing_of(_device: &RenderDevice, _rr_supported: bool) -> RtSupport {
    RtSupport::None
}

/// Whether the device can run FSR 3 (`fsr3::support`). Without the `fsr3` feature nothing is offered.
#[cfg(feature = "fsr3")]
fn fsr3_of(device: &RenderDevice) -> bool {
    crate::fsr3::support(device)
}

/// Without the `fsr3` feature there is no FSR 3 to offer.
#[cfg(not(feature = "fsr3"))]
fn fsr3_of(_device: &RenderDevice) -> bool {
    false
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
        // The game adds these before its window (the cursor needs them); a bare harness does not.
        if !app.is_plugin_added::<AssetPlugin>() {
            app.add_plugins(AssetPlugin::default());
        }
        if !app.is_plugin_added::<ImagePlugin>() {
            app.add_plugins(ImagePlugin::default());
        }
        // DLSS configures the instance and device creation that `RenderPlugin` runs, so it is added first, and it
        // reads the project id from the app when it is built.
        #[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
        {
            app.insert_resource(crate::dlss::sdk::project_id());
            app.add_plugins(DlssInitPlugin);
        }
        app.add_plugins((
            RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(settings)),
                synchronous_pipeline_compilation: self.synchronous_pipeline_compilation,
                ..RenderPlugin::default()
            },
            MeshPlugin,
            CameraPlugin,
            LightPlugin,
            CorePipelinePlugin,
            bevy_anti_alias::AntiAliasPlugin,
            // Bloom, depth of field and motion blur moved out of `bevy_core_pipeline` in 0.20; only bloom is
            // used (`post/mod.rs`), but the plugin adds all of them, and the others stay dormant unused.
            bevy_post_process::PostProcessPlugin,
        ));
        // After the render plugin (Solari reads the device's features when it is built), before the PBR plugin
        // (Solari's deferred path needs its prepasses).
        let solari = install_solari(app, self.ray_tracing, self.api, self.force_fallback_adapter);
        // Before the material plugin: it starts loading the shader when it is built.
        crate::material::load_shaders(app);
        app.add_plugins((
            PbrPlugin { prepass_enabled: solari, add_default_deferred_lighting_plugin: false, ..PbrPlugin::default() },
            MaterialPlugin::<BlackboxMaterial>::default(),
        ));
        // FSR 3 (`fsr3.rs`): the render-world jitter and upscale, and the `Fsr3Upscale` sync. Added with the other
        // render-world systems; it needs the blit pipeline and the effect layer (`systems::effects`) that are above.
        #[cfg(feature = "fsr3")]
        crate::fsr3::install(app);
        app.init_resource::<BlackboxBridge>().init_resource::<WorldState>().add_systems(
            PostUpdate,
            apply::apply
                .before(bevy_light::SimulationLightSystems::UpdateDirectionalLightCascades)
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
            .init_resource::<Fsr1Render>()
            .init_resource::<RadialBlurRender>()
            .init_resource::<effects::ExtractedWorld>()
            .add_systems(ExtractSchedule, effects::extract_world)
            .add_systems(
                Core3d,
                (
                    // Before `Core3dSystems::Prepass`, not merely inside it: the depth and motion-vector
                    // prepasses (when TAA is on) size themselves from `MainPassResolutionOverride` too
                    // (`bevy_core_pipeline::prepass::node`), and systems in the same set run in no
                    // particular order unless chained — this one has to win the race every time, not most
                    // of the time, so it is ordered before the whole set instead.
                    post::scale::apply_resolution_override.before(Core3dSystems::Prepass),
                    (effects::effects_main_pass, effects::effects_soft_pass)
                        .chain()
                        .in_set(Core3dSystems::EarlyPostProcess),
                    // The speed blur smears the finished frame (after tone mapping, before the upscale and the
                    // HUD), like native's radial blur pass; it does nothing at strength 0.
                    radial_blur::radial_blur_pass.in_set(Core3dSystems::PostProcess).after(tonemapping),
                    // FSR 1 replaces Bevy's own bilinear blit when it is the active upscaler (see
                    // `post/fsr1.rs` for why it runs after, not instead of, `upscaling` rather than
                    // suppressing it); the UI layer draws last, at the output's own resolution, whichever
                    // one ran.
                    fsr1::fsr1_pass.after(upscaling),
                    ui::ui_pass.after(upscaling).after(fsr1::fsr1_pass),
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
    /// Present when Bevy found DLSS Super Resolution on this machine (`DlssPlugin::finish`).
    dlss: Option<Res<'w, DlssSupport>>,
    /// Present when Bevy found DLSS Ray Reconstruction on this machine (the DLSS-RR denoiser for Solari).
    rr: Option<Res<'w, RrSupport>>,
}

impl BackendFactory<'_> {
    /// The backend for a surface of `size` pixels. The world draws into the primary window.
    pub fn create(&self, size: [u32; 2]) -> BevyBackend {
        backend_with_dlss(&self.bridge, &self.device, &self.adapter, size, self.dlss.is_some(), self.rr.is_some())
    }
}

/// The backend for `bridge`, on the device and adapter Bevy created. It does not offer DLSS: the factory knows
/// whether Bevy supports it, a bare harness does not look (the `dlss-mock` build offers it anyway).
pub fn backend(
    bridge: &BlackboxBridge,
    device: &RenderDevice,
    adapter: &RenderAdapterInfo,
    size: [u32; 2],
) -> BevyBackend {
    backend_with_dlss(bridge, device, adapter, size, false, false)
}

fn backend_with_dlss(
    bridge: &BlackboxBridge,
    device: &RenderDevice,
    adapter: &RenderAdapterInfo,
    size: [u32; 2],
    dlss_supported: bool,
    rr_supported: bool,
) -> BevyBackend {
    let api = api_of(adapter.backend);
    let compressed_bc = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    let info = BackendInfo {
        renderer: "bevy",
        api,
        adapter: adapter.name.clone(),
        driver: format!("{} {}", adapter.driver, adapter.driver_info).trim().to_owned(),
    };
    let ray_tracing = ray_tracing_of(device, rr_supported);
    let fsr3 = fsr3_of(device);
    let caps = crate::caps::capabilities_with_ray_tracing(api, compressed_bc, dlss_supported, ray_tracing, fsr3);
    BevyBackend::new(bridge.share(), info, caps, size)
}
