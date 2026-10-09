//! The main-world system that turns the facade's queue into Bevy entities.
//!
//! It runs in `PostUpdate` of every tick, before Bevy computes visibility and extracts the world for
//! rendering, so what the game handed to `render()` during `Update` is drawn by the render that follows in the
//! same tick.

pub mod assets;
pub mod camera;
pub mod capture;
pub mod instances;
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
    mut placed: instances::PlacedQuery,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let started = std::time::Instant::now();
    apply_queue(&bridge, &mut state, &mut stores, &mut commands, &mut placed, &mut windows);
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
    placed: &mut instances::PlacedQuery,
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
    for op in ops {
        assets::apply_op(state, stores, op);
    }

    if let Some(request) = request {
        state.capture = Some(capture::begin(commands, &mut stores.images, request, &settings));
    }

    // A running capture decides what the world shows until it is done.
    if let Some(mut active) = state.capture.take() {
        let data = active.data.clone();
        draw(state, stores, commands, placed, &data);
        camera::follow(commands, active.camera, &data.frame, &settings);
        let size = capture_size(stores, &active);
        let done = capture::advance(commands, &mut active, &bridge.share(), size);
        state.capture = (!done).then_some(active);
        return;
    }

    let Some(data) = frame else { return };
    draw(state, stores, commands, placed, &data);
    drive_screen(state, commands, windows, &data, &settings, surface);
}

/// Bring the materials and the pool up to date with a frame.
fn draw(
    state: &mut WorldState,
    stores: &mut Stores,
    commands: &mut Commands,
    placed: &mut instances::PlacedQuery,
    data: &FrameData,
) {
    assets::sync_params(state, stores, Params::of(&data.frame));
    instances::sync(state, stores, commands, placed, &data.instances);
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
        ScreenCamera { entity: commands.spawn(bundle).id(), fxaa: !settings.fxaa, render_size: None, vsync: None }
    });
    camera::follow(commands, screen.entity, &data.frame, settings);
    let post = (screen.fxaa, screen.render_size) != (settings.fxaa, camera::render_override(surface, settings));
    if post {
        camera::set_post(commands, screen.entity, settings, surface);
        screen.fxaa = settings.fxaa;
        screen.render_size = camera::render_override(surface, settings);
    }
    if screen.vsync != Some(settings.vsync) {
        // Mailbox, not AutoNoVsync: Bevy's no-vsync fallback list tries Immediate first, which on Mesa under
        // Wayland and Xwayland still ran at the display rate (60 fps) where Mailbox ran unthrottled; wgpu's own
        // AutoNoVsync, which the native renderer uses, picks Mailbox. Mailbox falls back to Immediate and Fifo.
        window.present_mode = if settings.vsync { PresentMode::AutoVsync } else { PresentMode::Mailbox };
        screen.vsync = Some(settings.vsync);
    }
}
