//! The render bridge: the one place where the app creates the renderer and draws a frame.
//!
//! Moving to Bevy's own renderer (docs/decisions/0001-bevy.md, option B) replaces this file and the
//! implementation behind `blackbox-render`'s API, not the game code.

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, RawHandleWrapper, Window};
use bevy_winit::DisplayHandleWrapper;
use blackbox_render::{Renderer, RendererOptions};

use super::host::{ErrorSlot, Host};
use super::screenshot;
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
    let options = RendererOptions { backend: settings.backend, vsync: settings.vsync };
    match start(&mut host, raw, size, &display, options) {
        Ok(()) => capture.0 = host.scene.captures_mouse() && host.screenshot.is_none(),
        Err(e) => {
            errors.set(e);
            exit.write(AppExit::error());
        }
    }
}

#[allow(unsafe_code)]
fn start(
    host: &mut Host,
    raw: &RawHandleWrapper,
    size: (u32, u32),
    display: &DisplayHandleWrapper,
    options: RendererOptions,
) -> anyhow::Result<()> {
    // SAFETY: this runs in a system that takes `NonSendMut`, so it is on the main thread, which is
    // what `get_handle` requires.
    let handle = unsafe { raw.get_handle() };
    let mut renderer = Renderer::new(handle, size, display.0.clone(), options)?;
    log::info!("renderer: {} (requested backend: {})", renderer.adapter_summary(), options.backend);
    host.scene.init(&mut renderer)?;
    host.scene.set_transmission(host.transmission);
    if host.screenshot.is_some() {
        screenshot::wait_ready(host.scene.as_mut(), &mut renderer);
    }
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
        r.resize(size.0, size.1);
        host.size = size;
    }
}

pub fn update_scene(mut host: NonSendMut<Host>, actions: Res<ActionState>, time: Res<Time>, settings: Res<Settings>) {
    let host = &mut *host;
    let Some(renderer) = host.renderer.as_mut() else { return };
    host.transmission = settings.transmission;
    host.scene.set_transmission(settings.transmission);
    host.scene.update(renderer, &actions, time.delta_secs().min(MAX_STEP));
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
    let result = if let Some(path) = &host.screenshot {
        host.frames += 1;
        if host.frames < SCREENSHOT_SETTLE_FRAMES || host.hold_capture {
            return;
        }
        screenshot::capture(renderer, &params, instances, path).map(|()| {
            if let Some(status) = host.scene.status() {
                println!("{status}");
            }
            exit.write(AppExit::Success);
        })
    } else {
        renderer.render(&params, instances).map(|_| host.frames += 1).map_err(Into::into)
    };
    if let Err(e) = result {
        errors.set(e);
        exit.write(AppExit::error());
    }
}
