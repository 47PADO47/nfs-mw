//! The optional Bevy renderer (`blackbox-bevy-render`): the only place besides `native.rs` that names a renderer
//! crate. Everything here is behind the `renderer-bevy` cargo feature; without it the functions are stubs that
//! say the build has no Bevy renderer, so the rest of the app needs no `cfg`.

use bevy_app::{App, Update};
use blackbox_gfx::GraphicsApi;

/// Whether this PC can run the Bevy renderer on `api`, and if not, why. Logs what it found.
#[cfg(feature = "renderer-bevy")]
pub fn probe(api: GraphicsApi) -> Result<(), String> {
    match blackbox_bevy_render::probe(api) {
        Ok(found) => {
            log::info!(
                "bevy renderer probe: {} on {} (bc {}, ray query {}, binding arrays {}{})",
                found.adapter,
                found.api,
                found.compressed_bc,
                found.ray_query,
                found.binding_arrays,
                if found.software { ", software" } else { "" }
            );
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(feature = "renderer-bevy"))]
pub fn probe(_api: GraphicsApi) -> Result<(), String> {
    Err("this build has no bevy renderer".to_owned())
}

/// Join the renderer to the game's `App` (before `run`). Only called once the probe passed.
#[cfg(feature = "renderer-bevy")]
pub fn add_plugin(app: &mut App, api: GraphicsApi) {
    use bevy_ecs::schedule::IntoScheduleConfigs;
    app.add_plugins((
        blackbox_bevy_render::BlackboxBevyRenderPlugin {
            api,
            synchronous_pipeline_compilation: false,
            ..Default::default()
        },
        blackbox_bevy_render::BlackboxFpsOverlayPlugin,
    ))
    .add_systems(Update, sync_fps_overlay.run_if(super::using_bevy));
}

#[cfg(not(feature = "renderer-bevy"))]
pub fn add_plugin(_app: &mut App, _api: GraphicsApi) {}

/// Shows Bevy's frame-rate overlay when the `metrics` setting asks for it; `advanced` adds the frame-time graph.
#[cfg(feature = "renderer-bevy")]
pub fn sync_fps_overlay(
    settings: bevy_ecs::prelude::Res<crate::settings::Settings>,
    mut config: bevy_ecs::prelude::ResMut<blackbox_bevy_render::FpsOverlayConfig>,
) {
    let enabled = settings.show_metrics != crate::devtools::ShowMetrics::Off;
    let graph = settings.show_metrics == crate::devtools::ShowMetrics::Advanced;
    // Compare first: writing the resource through `ResMut` marks it changed every frame otherwise.
    if config.enabled == enabled && config.frame_time_graph_config.enabled == graph {
        return;
    }
    config.enabled = enabled;
    config.frame_time_graph_config.enabled = graph;
}

/// Create the Bevy backend once the device exists (the plugins have finished by the first `Update`).
#[cfg(feature = "renderer-bevy")]
pub fn create(
    mut host: bevy_ecs::prelude::NonSendMut<super::super::host::Host>,
    window: bevy_ecs::prelude::Single<&bevy_window::Window, bevy_ecs::prelude::With<bevy_window::PrimaryWindow>>,
    factory: blackbox_bevy_render::BackendFactory,
    settings: bevy_ecs::prelude::Res<crate::settings::Settings>,
    errors: bevy_ecs::prelude::Res<super::super::host::ErrorSlot>,
    mut capture: bevy_ecs::prelude::ResMut<crate::input::MouseCapture>,
    mut exit: bevy_ecs::prelude::MessageWriter<bevy_app::AppExit>,
) {
    if host.renderer.is_some() {
        return;
    }
    let size = (window.physical_width(), window.physical_height());
    let mut backend = factory.create([size.0, size.1]);
    // The native renderer takes this at creation; the Bevy window follows the facade.
    blackbox_gfx::RenderBackend::set_vsync(&mut backend, settings.vsync);
    let backend = Box::new(backend);
    match super::finish(&mut host, backend, size, &settings) {
        Ok(()) => capture.0 = host.scene.captures_mouse() && host.screenshot.is_none(),
        Err(e) => {
            errors.set(e);
            exit.write(bevy_app::AppExit::error());
        }
    }
}

/// Never runs: without the feature the bevy renderer is never chosen.
#[cfg(not(feature = "renderer-bevy"))]
pub fn create() {}
