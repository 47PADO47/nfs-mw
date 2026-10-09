//! The application: a Bevy `App` that owns the window, the loop, the input and the schedule, and
//! hands each frame to `blackbox-render` (see `docs/decisions/0001-bevy.md`).
//!
//! The seam to the renderer is [`render`]: it is the only place that creates the renderer or draws.

mod cursor;
mod host;
pub mod pacing;
mod render;
mod screenshot;
pub mod window;

pub use cursor::update as cursor_update;
pub use host::Host;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use bevy_a11y::AccessibilityPlugin;
use bevy_app::{App, AppExit, Last, TaskPoolPlugin, Update};
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy_gilrs::GilrsPlugin;
use bevy_input::InputPlugin;
use bevy_time::TimePlugin;
use bevy_window::{Window, WindowPlugin, WindowResolution};
use bevy_winit::WinitPlugin;

use crate::devtools::DevToolsPlugin;
use crate::gui::GuiPlugin;
use crate::input::{Bindings, InputLayerPlugin};
use crate::settings::Settings;
use crate::viewer::Scene;
use host::ErrorSlot;

/// The order of a frame's work in `Update`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameSet {
    /// Create the renderer, handle the cursor, follow the window size.
    Prepare,
    /// Apply console commands and live settings before scene simulation.
    Commands,
    /// The front end: menus, pause, switching scenes.
    Frontend,
    SceneUpdate,
    /// Build the UI (overlay, console).
    Ui,
    /// The HUD goes under the UI layer.
    Hud,
    /// Hand the frame to the renderer.
    Draw,
}

/// What to do besides showing the scene.
#[derive(Default)]
pub struct RunOptions {
    /// Write a PNG and exit instead of opening an interactive window.
    pub screenshot: Option<PathBuf>,
    /// Console commands to run once the renderer is up (`--exec`).
    pub exec: Vec<String>,
    /// Start with the console open.
    pub open_console: bool,
    /// Show the in-game HUD, reading its data from this install.
    pub hud: Option<game_install::GameDir>,
    /// Numbers the HUD shows until the scene supplies its own (`--hud-demo`).
    pub hud_demo: Option<crate::hud::HudState>,
    /// Open the sound device and play the scene's sound, reading banks from this install.
    pub audio: Option<game_install::GameDir>,
    /// Run the front end (menus, game flow) on top of the scene.
    pub frontend: Option<crate::frontend::FrontendPlugin>,
}

/// Open a window and run `scene` until the user quits (or write the screenshot and exit).
pub fn run(scene: Box<dyn Scene>, settings: &Settings, options: RunOptions) -> Result<()> {
    let RunOptions { screenshot, exec, open_console, hud, hud_demo, audio, frontend } = options;
    let error = ErrorSlot(Arc::new(Mutex::new(None)));
    let mut window = Window { title: scene.title(), ..Window::default() };
    if screenshot.is_some() {
        let (w, h) = screenshot::SIZE;
        window.visible = false;
        window.focused = false;
        window.position = bevy_window::WindowPosition::At([-32768, -32768].into());
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
        GuiPlugin,
        DevToolsPlugin,
    ))
    .insert_resource(*settings)
    .insert_resource(window::WindowModes::new(screenshot.is_some()))
    .insert_resource(Bindings::load(settings))
    .insert_resource(error.clone())
    .insert_non_send(Host::new(scene, settings, screenshot))
    .configure_sets(
        Update,
        (
            FrameSet::Prepare,
            FrameSet::Commands,
            FrameSet::Frontend,
            FrameSet::SceneUpdate,
            FrameSet::Ui,
            FrameSet::Hud,
            FrameSet::Draw,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (window::shortcut, window::update, render::create_renderer, cursor::update, render::resize)
            .chain()
            .in_set(FrameSet::Prepare),
    )
    .add_systems(Update, render::update_scene.in_set(FrameSet::SceneUpdate))
    .add_systems(Update, render::draw.in_set(FrameSet::Draw))
    .add_systems(Last, pacing::end_of_frame);
    if let Some(dir) = hud {
        app.add_plugins(crate::hud::HudPlugin { dir, initial: hud_demo.unwrap_or_default() });
    }
    if let Some(dir) = audio {
        app.add_plugins(crate::audio::AudioPlugin { dir });
    }
    app.add_plugins(crate::movie::FullscreenPlugin);
    if let Some(plugin) = frontend {
        app.add_plugins(plugin);
    }
    crate::devtools::start_console(&mut app, exec, open_console);

    match app.run() {
        AppExit::Success => error.take().map_or(Ok(()), Err),
        AppExit::Error(code) => Err(error.take().unwrap_or_else(|| anyhow!("the app exited with code {code}"))),
    }
}
