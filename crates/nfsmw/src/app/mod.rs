//! The application: a Bevy `App` that owns the window, the loop, the input and the schedule, and
//! hands each frame to a renderer behind the `blackbox-gfx` `RenderBackend` trait (see
//! `docs/decisions/0001-bevy.md`).
//!
//! The seam to the renderer is [`render`]: it is the only place that creates the renderer or draws, and
//! `render/native.rs` is the only file that names the native `blackbox-render` crate.

mod bench;
mod cursor;
mod graphics;
mod host;
pub mod pacing;
mod render;
mod screenshot;
#[cfg(test)]
pub use screenshot::Plan;
pub(crate) mod test_backend;
pub mod window;

pub use cursor::update as cursor_update;
pub use host::Host;
pub use render::select::BEVY_COMPILED;

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
    /// Physical off-screen target size; `None` preserves the default screenshot dimensions.
    pub screenshot_size: Option<[u32; 2]>,
    /// Seconds to wait after the scene is ready before the first capture.
    pub screenshot_delay: f32,
    /// How many captures to take (numbered files when more than one).
    pub screenshot_count: u32,
    /// Seconds between captures.
    pub screenshot_interval: f32,
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
    /// Measure frame times for this many seconds once the scene is loaded, print them and exit (`--bench-seconds`).
    pub bench_seconds: Option<f32>,
}

/// Open a window and run `scene` until the user quits (or write the screenshot and exit).
pub fn run(scene: Box<dyn Scene>, settings: &Settings, options: RunOptions) -> Result<()> {
    let RunOptions {
        screenshot,
        screenshot_size,
        screenshot_delay,
        screenshot_count,
        screenshot_interval,
        exec,
        open_console,
        hud,
        hud_demo,
        audio,
        frontend,
        bench_seconds,
    } = options;
    let plan = screenshot
        .clone()
        .map(|path| screenshot::Plan::new(path, screenshot_delay, screenshot_count, screenshot_interval));
    let error = ErrorSlot(Arc::new(Mutex::new(None)));
    let mut window = Window { title: scene.title(), ..Window::default() };
    if screenshot.is_some() {
        let [w, h] = screenshot_size.unwrap_or([screenshot::SIZE.0, screenshot::SIZE.1]);
        window.visible = false;
        window.focused = false;
        window.position = bevy_window::WindowPosition::At([-32768, -32768].into());
        window.resolution = WindowResolution::new(w, h).with_scale_factor_override(1.0);
    }

    // The renderer is fixed here, before the App exists: a bevy request this build or PC cannot satisfy falls back
    // to the native renderer, because Bevy panics when it finds no adapter once the app is built.
    let renderer = render::select::choose(settings.renderer, settings.backend);
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
    .insert_resource(render::ActiveRenderer(renderer))
    .insert_resource(window::WindowModes::new(screenshot.is_some()))
    .insert_resource(Bindings::load(settings))
    .insert_resource(error.clone())
    .insert_non_send(Host::new(scene, settings, plan).with_bench(bench_seconds))
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
        (
            window::shortcut,
            window::update,
            render::create_renderer.run_if(render::using_blackbox),
            render::create_bevy_renderer.run_if(render::using_bevy),
            cursor::update,
            render::resize,
        )
            .chain()
            .in_set(FrameSet::Prepare),
    )
    .add_systems(Update, render::update_scene.in_set(FrameSet::SceneUpdate))
    .add_systems(Update, graphics::apply.in_set(FrameSet::Draw).before(render::draw))
    .add_systems(Update, render::draw.in_set(FrameSet::Draw))
    .add_systems(Last, pacing::end_of_frame);
    if renderer == crate::settings::RendererKind::Bevy {
        render::add_bevy_plugin(&mut app, settings.backend);
    }
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
