//! The application: a Bevy `App` that owns the window, the loop, the input and the schedule, and
//! hands each frame to `blackbox-render` (see `docs/decisions/0001-bevy.md`).
//!
//! The seam to the renderer is [`render`]: it is the only place that creates the renderer or draws.

mod cursor;
mod host;
pub mod pacing;
mod render;
mod screenshot;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use bevy_a11y::AccessibilityPlugin;
use bevy_app::{App, AppExit, Last, TaskPoolPlugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_gilrs::GilrsPlugin;
use bevy_input::InputPlugin;
use bevy_time::TimePlugin;
use bevy_window::{Window, WindowPlugin, WindowResolution};
use bevy_winit::WinitPlugin;

use crate::input::InputLayerPlugin;
use crate::settings::Settings;
use crate::viewer::Scene;
use host::{ErrorSlot, Host};

/// Open a window and run `scene` until the user quits (or write `screenshot` and exit).
pub fn run(scene: Box<dyn Scene>, settings: &Settings, screenshot: Option<PathBuf>) -> Result<()> {
    let error = ErrorSlot(Arc::new(Mutex::new(None)));
    let mut window = Window { title: scene.title(), ..Window::default() };
    if screenshot.is_some() {
        let (w, h) = screenshot::SIZE;
        window.visible = false;
        window.resolution = WindowResolution::new(w, h).with_scale_factor_override(1.0);
    }

    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        TimePlugin,
        InputPlugin,
        AccessibilityPlugin,
        WindowPlugin { primary_window: Some(window), ..WindowPlugin::default() },
        WinitPlugin::default(),
        GilrsPlugin,
        InputLayerPlugin,
    ))
    .insert_resource(*settings)
    .insert_resource(error.clone())
    .insert_non_send(Host::new(scene, settings, screenshot))
    .add_systems(
        Update,
        (render::create_renderer, cursor::update, render::resize, render::update_scene, render::draw).chain(),
    )
    .add_systems(Last, pacing::end_of_frame);

    match app.run() {
        AppExit::Success => error.take().map_or(Ok(()), Err),
        AppExit::Error(code) => Err(error.take().unwrap_or_else(|| anyhow!("the app exited with code {code}"))),
    }
}
