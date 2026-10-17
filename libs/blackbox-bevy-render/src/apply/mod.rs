//! The main-world system that turns the facade's queue into Bevy entities.
//!
//! It runs in `PostUpdate` of every tick, before Bevy computes visibility and extracts the world for
//! rendering, so what the game handed to `render()` during `Update` is drawn by the render that follows in the
//! same tick.

pub mod assets;
pub mod camera;
pub mod capture;
pub mod environment;
pub mod instances;
pub mod redirects;
pub mod shadows;
pub mod state;

use bevy_camera::RenderTarget;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use bevy_window::{PresentMode, PrimaryWindow, Window, WindowRef};

pub use assets::Stores;
pub use state::WorldState;

use crate::material::Params;
use crate::ops::{BlackboxBridge, FrameData};
use state::ScreenCamera;

/// Time spent in [`apply`], logged at debug level every [`REPORT_EVERY`] frames.
#[derive(Default)]
pub struct ApplyTimer {
    frames: u32,
    total: std::time::Duration,
}

const REPORT_EVERY: u32 = 300;

pub fn apply(
    mut timer: bevy_ecs::system::Local<ApplyTimer>,
    bridge: Res<BlackboxBridge>,
    mut state: ResMut<WorldState>,
    mut stores: Stores,
    mut commands: Commands,
    mut surfaces: instances::Surfaces,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let started = log::log_enabled!(log::Level::Debug).then(std::time::Instant::now);
    apply_queue(&bridge, &mut state, &mut stores, &mut commands, &mut surfaces, &mut windows);
    let Some(started) = started else { return };
    timer.total += started.elapsed();
    timer.frames += 1;
    if timer.frames < REPORT_EVERY {
        return;
    }
    let (objects, entities) = instances::counts(&state);
    log::debug!(
        "bevy renderer: apply {:.3} ms per frame; {objects} objects, {entities} entities, {} materials",
        timer.total.as_secs_f64() * 1000.0 / f64::from(timer.frames),
        state.materials.by_key.len()
    );
    *timer = ApplyTimer::default();
}

fn apply_queue(
    bridge: &BlackboxBridge,
    state: &mut WorldState,
    stores: &mut Stores,
    commands: &mut Commands,
    surfaces: &mut instances::Surfaces,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    let (ops, frame, request, settings, surface) = {
        let mut shared = bridge.lock();
        let request = match state.capture.is_some() {
            true => None,
            false => shared.captures.pop_front(),
        };
        (std::mem::take(&mut shared.ops), shared.frame.take(), request, shared.settings, shared.surface)
    };
    // The `dlss-mock` build draws a DLSS request as TAA plus FSR 1 (`dlss::mock`); everything below sees that.
    #[cfg(feature = "dlss-mock")]
    let settings = crate::dlss::mock::stand_in(settings);
    // Solari is on when the stand-in cache exists (`solari::install` at startup): its meshes are prepared as
    // they are added, and its stand-ins are drawn from the stores below.
    let solari = surfaces.cache.is_some();
    for op in ops {
        assets::apply_op(state, stores, instances::prepare_op(op, solari));
    }
    let mut stand_ins = match surfaces.cache.as_deref_mut() {
        Some(cache) => Some(instances::StandIns { standard: &mut surfaces.standard, cache }),
        None => None,
    };

    let shadow_frame = state
        .capture
        .as_ref()
        .map(|c| &c.data.frame)
        .or_else(|| request.as_ref().map(|c| &c.data.frame))
        .or_else(|| frame.as_ref().map(|f| &f.frame));
    if let Some(frame) = shadow_frame {
        let tuning = frame.shadow_tuning.unwrap_or(blackbox_gfx::settings::ShadowTuning::world(settings.shadows));
        shadows::sync(commands, &mut state.sun, frame, tuning);
    }

    if let Some(request) = request {
        state.capture = Some(capture::begin(commands, &mut stores.images, request, &settings));
    }

    // A running capture decides what the world shows until it is done.
    if let Some(mut active) = state.capture.take() {
        let data = active.data.clone();
        draw(state, stores, commands, &mut surfaces.placed, stand_ins.as_mut(), &data, &settings);
        camera::follow(commands, active.camera, &data.frame, &settings);
        let size = capture_size(stores, &active);
        let done = capture::advance(commands, &mut active, &bridge.share(), size);
        state.capture = (!done).then_some(active);
        return;
    }

    let Some(data) = frame else { return };
    draw(state, stores, commands, &mut surfaces.placed, stand_ins.as_mut(), &data, &settings);
    drive_screen(state, commands, windows, &data, &settings, surface);
}

/// Bring the scene buffer and the pool up to date with a frame.
fn draw(
    state: &mut WorldState,
    stores: &mut Stores,
    commands: &mut Commands,
    placed: &mut instances::PlacedQuery,
    stand_ins: Option<&mut instances::StandIns>,
    data: &FrameData,
    settings: &crate::ops::CameraSettings,
) {
    let params =
        Params::with_tonemap(&data.frame, settings.post.tonemap).with_original_world(settings.original_world_materials);
    assets::sync_scene(state, stores, params);
    instances::sync(state, stores, commands, placed, &data.instances, stand_ins);
}

/// The pixel size of a capture's image.
fn capture_size(stores: &Stores, active: &capture::ActiveCapture) -> [u32; 2] {
    stores.images.get(&active.image).map_or([1, 1], |image| [image.width(), image.height()])
}

/// The window camera and the window settings that follow the facade.
fn drive_screen(
    state: &mut WorldState,
    commands: &mut Commands,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
    data: &FrameData,
    settings: &crate::ops::CameraSettings,
    surface: [u32; 2],
) {
    let Ok(mut window) = windows.single_mut() else { return };
    let screen = state.screen.get_or_insert_with(|| {
        let bundle = camera::bundle(&data.frame, RenderTarget::Window(WindowRef::Primary), settings);
        ScreenCamera { entity: commands.spawn(bundle).id(), applied: None, render_size: None, vsync: None }
    });
    camera::follow(commands, screen.entity, &data.frame, settings);
    let render_size = camera::render_override(surface, settings.render_scale);
    let post = screen.applied != Some(*settings) || screen.render_size != render_size;
    if post {
        camera::set_post(commands, screen.entity, settings);
        screen.applied = Some(*settings);
        screen.render_size = render_size;
    }
    // Every frame, after `set_post`: it can take the prepasses that DLSS needs away, so they go back on here.
    #[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
    crate::dlss::sdk::set(commands, screen.entity, settings, data.frame.camera_cut);
    // The same for FSR 3 (`fsr3::set`): its prepasses, jitter, `Hdr` and the camera-cut reset.
    #[cfg(feature = "fsr3")]
    crate::fsr3::set(commands, screen.entity, settings, data.frame.camera_cut);
    if screen.vsync != Some(settings.vsync) {
        // Mailbox, not AutoNoVsync: Bevy's no-vsync fallback list tries Immediate first, which on Mesa under
        // Wayland and Xwayland still ran at the display rate (60 fps) where Mailbox ran unthrottled; wgpu's own
        // AutoNoVsync, which the native renderer uses, picks Mailbox. Mailbox falls back to Immediate and Fifo.
        window.present_mode = if settings.vsync { PresentMode::AutoVsync } else { PresentMode::Mailbox };
        screen.vsync = Some(settings.vsync);
    }
}
