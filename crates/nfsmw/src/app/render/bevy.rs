//! The optional Bevy renderer (`blackbox-bevy-render`): the only place besides `native.rs` that names a renderer
//! crate. Everything here is behind the `renderer-bevy` cargo feature; without it the functions are stubs that
//! say the build has no Bevy renderer, so the rest of the app needs no `cfg`.

use bevy_app::App;
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
    app.add_plugins(blackbox_bevy_render::BlackboxBevyRenderPlugin { api, ..Default::default() });
}

#[cfg(not(feature = "renderer-bevy"))]
pub fn add_plugin(_app: &mut App, _api: GraphicsApi) {}

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
