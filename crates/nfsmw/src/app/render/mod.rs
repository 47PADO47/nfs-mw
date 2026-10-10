//! The render bridge: the one place where the app creates the renderer and draws a frame.
//!
//! The app only knows [`RenderBackend`](blackbox_gfx::RenderBackend). Each renderer has a factory in its own
//! file here ([`native`] is the Black Box renderer); swapping the renderer replaces that factory, not the game
//! code (docs/decisions/0001-bevy.md, docs/decisions/0004-swappable-renderers.md).

mod native;

use std::time::Instant;

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, RawHandleWrapper, Window};
use bevy_winit::DisplayHandleWrapper;

use super::host::{ErrorSlot, Host};
use super::screenshot;
use super::upscale;
use crate::gui::UiOutput;
use crate::input::{ActionState, MouseCapture};
use crate::settings::Settings;

/// Longest step a scene is asked to advance by, so a stall does not throw the camera across the map.
const MAX_STEP: f32 = 0.1;
/// Frames a screenshot run lets the UI and the metrics settle for before capturing.
const SCREENSHOT_SETTLE_FRAMES: u32 = 3;

/// Create the renderer as soon as the window has a native handle.
#[allow(clippy::too_many_arguments)]
pub fn create_renderer(
    mut host: NonSendMut<Host>,
    window: Single<(&RawHandleWrapper, &Window), With<PrimaryWindow>>,
    display: Res<DisplayHandleWrapper>,
    settings: Res<Settings>,
    errors: Res<ErrorSlot>,
    mut capture: ResMut<MouseCapture>,
    mut exit: MessageWriter<AppExit>,
) {
    if host.renderer.is_some() {
        return;
    }
    let (raw, win) = *window;
    let size = (win.physical_width(), win.physical_height());
    match start(&mut host, raw, size, &display, &settings) {
        Ok(()) => capture.0 = host.scene.captures_mouse() && host.screenshot.is_none(),
        Err(e) => {
            errors.set(e);
            exit.write(AppExit::error());
        }
    }
}

fn start(
    host: &mut Host,
    raw: &RawHandleWrapper,
    size: (u32, u32),
    display: &DisplayHandleWrapper,
    settings: &Settings,
) -> anyhow::Result<()> {
    let mut renderer = native::create(raw, size, display, settings)?;
    log::info!("renderer: {} (requested backend: {})", renderer.info().summary(), settings.backend);
    upscale::apply(renderer.as_mut(), settings);
    host.scene.init(renderer.as_mut())?;
    host.scene.set_transmission(host.transmission);
    host.scene.set_wheel_options(host.wheel);
    host.size = size;
    host.renderer = Some(renderer);
    Ok(())
}

/// Follow the window's size.
pub fn resize(mut host: NonSendMut<Host>, window: Single<&Window, With<PrimaryWindow>>) {
    let size = (window.physical_width(), window.physical_height());
    let host = &mut *host;
    if size != host.size
        && let Some(r) = host.renderer.as_mut()
    {
        r.resize([size.0, size.1]);
        host.size = size;
    }
}

pub fn update_scene(mut host: NonSendMut<Host>, actions: Res<ActionState>, time: Res<Time>, settings: Res<Settings>) {
    let host = &mut *host;
    let Some(renderer) = host.renderer.as_mut() else { return };
    host.transmission = settings.transmission;
    host.scene.set_transmission(settings.transmission);
    host.wheel = settings.wheel_options();
    host.scene.set_wheel_options(host.wheel);
    // Startup console settings have run in Commands before a screenshot's scripted simulation.
    if host.screenshot.is_some() && host.frames == 0 {
        screenshot::wait_ready(host.scene.as_mut(), renderer.as_mut());
    }
    host.scene.update(renderer.as_mut(), &actions, time.delta_secs().min(MAX_STEP));
}

/// Draw the scene and the UI over it, or (screenshot runs) capture it once the UI has settled.
pub fn draw(
    mut host: NonSendMut<Host>,
    mut ui: ResMut<UiOutput>,
    errors: Res<ErrorSlot>,
    mut exit: MessageWriter<AppExit>,
) {
    let host = &mut *host;
    let Some(renderer) = host.renderer.as_mut() else { return };
    for patch in ui.patches.drain(..) {
        renderer.update_ui_texture(&patch.as_patch());
    }
    if let Some(layer) = ui.layer.take() {
        renderer.set_ui_layer(layer);
    }
    for id in ui.freed.drain(..) {
        renderer.free_ui_texture(id);
    }

    let (params, instances) = host.scene.frame(renderer.aspect_ratio());
    let result = if let Some(plan) = host.screenshot.as_mut() {
        host.frames += 1;
        if host.pending_capture.is_none() && (host.frames < SCREENSHOT_SETTLE_FRAMES || host.hold_capture) {
            return;
        }
        let started = match host.pending_capture.take() {
            Some(pending) => Ok(pending),
            None => match plan.due(Instant::now()) {
                None => return,
                Some(path) => {
                    let last = plan.finished();
                    screenshot::request(renderer.as_mut(), &params, instances, host.size).map(|id| (id, path, last))
                }
            },
        };
        // A renderer that works on the capture for a few frames is asked again every frame until it is done.
        let written = started.and_then(|(id, path, last)| {
            if screenshot::poll(renderer.as_mut(), id, &path)? {
                return Ok(Some(last));
            }
            host.pending_capture = Some((id, path, last));
            Ok(None)
        });
        written.map(|done| {
            let Some(last) = done else { return };
            if let Some(status) = host.scene.status() {
                println!("{status}");
            }
            if last {
                exit.write(AppExit::Success);
            }
        })
    } else {
        renderer.render(&params, instances).map(|_| host.frames += 1).map_err(Into::into)
    };
    if let Err(e) = result {
        errors.set(e);
        exit.write(AppExit::error());
    }
}
