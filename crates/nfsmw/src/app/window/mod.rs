//! Windowed, borderless and exclusive mode changes through Bevy/winit.
//!
//! The renderer follows the window's physical size in the existing render bridge. UI keeps the
//! operating system's scale factor; screenshot runs bypass all interactive window preferences.

mod hotkey;
mod monitors;
#[cfg(test)]
mod tests;

use bevy_ecs::prelude::*;
use bevy_window::{
    Monitor, MonitorSelection, OnMonitor, PrimaryMonitor, PrimaryWindow, RawHandleWrapper, VideoModeSelection, Window,
    WindowMode as BevyMode, WindowPosition,
};
use bevy_winit::WinitMonitors;

use crate::settings::{self, Resolution, Settings, WindowMode};
pub(super) use hotkey::update as shortcut;

pub(super) type WindowState<'w, 's> = Single<
    'w,
    's,
    (&'static mut Window, Option<&'static OnMonitor>, Option<&'static RawHandleWrapper>),
    With<PrimaryWindow>,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Preference {
    mode: WindowMode,
    monitor: settings::Monitor,
    resolution: Resolution,
}

impl From<&Settings> for Preference {
    fn from(settings: &Settings) -> Self {
        Self { mode: settings.window_mode, monitor: settings.monitor, resolution: settings.resolution }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    size: [u32; 2],
    position: WindowPosition,
}

#[derive(PartialEq)]
struct Readout {
    mode: BevyMode,
    placement: Placement,
    scale: f32,
    focused: bool,
    visible: bool,
}

impl From<&Window> for Readout {
    fn from(window: &Window) -> Self {
        Self {
            mode: window.mode,
            placement: Placement::from(window),
            scale: window.scale_factor(),
            focused: window.focused,
            visible: window.visible,
        }
    }
}

impl From<&Window> for Placement {
    fn from(window: &Window) -> Self {
        Self { size: [window.physical_width(), window.physical_height()], position: window.position }
    }
}

/// Runtime state, separate from the preferences so fullscreen resizes do not overwrite windowed size.
#[derive(Resource)]
pub struct WindowModes {
    hidden: bool,
    applied: Option<Preference>,
    windowed: Option<Placement>,
    windowed_resolution: Resolution,
    restore: Option<Placement>,
    settle: u8,
    fullscreen: WindowMode,
    /// The current monitor inventory for the developer console.
    pub monitors: String,
    last_status: Option<Readout>,
}

impl WindowModes {
    pub fn new(hidden: bool) -> Self {
        Self {
            hidden,
            applied: None,
            windowed: None,
            windowed_resolution: Resolution::Native,
            restore: None,
            settle: 0,
            fullscreen: WindowMode::Borderless,
            monitors: String::new(),
            last_status: None,
        }
    }

    pub fn status(window: &Window) -> String {
        monitors::status(window)
    }
}

pub(super) fn update(
    mut settings: ResMut<Settings>,
    mut modes: ResMut<WindowModes>,
    window: WindowState,
    query: Query<(Entity, Ref<Monitor>, Has<PrimaryMonitor>)>,
    native: Res<WinitMonitors>,
    mut screens: Local<Vec<monitors::Screen>>,
) {
    let (mut window, current, raw) = window.into_inner();
    if modes.hidden {
        return;
    }
    if native.is_changed()
        || query.iter().count() != screens.len()
        || query.iter().any(|(_, monitor, _)| monitor.is_changed())
    {
        *screens = monitors::inventory(&query, &native);
        modes.monitors = monitors::describe(&screens);
    }
    let Some((screen, fallback)) = monitors::select(&screens, current, settings.monitor) else { return };
    if fallback {
        log::warn!("monitor {} is unavailable; using the primary monitor", settings.monitor);
        settings.monitor = match screen.primary {
            true => settings::Monitor::Primary,
            false => screen.index.map(settings::Monitor::Index).unwrap_or(settings::Monitor::Current),
        };
    }
    if let BevyMode::BorderlessFullscreen(MonitorSelection::Entity(entity))
    | BevyMode::Fullscreen(MonitorSelection::Entity(entity), _) = window.mode
        && !screens.iter().any(|s| s.entity == entity)
    {
        modes.applied = None;
    }
    let requested = Preference::from(&*settings);
    if modes.applied != Some(requested) {
        if requested.mode == WindowMode::Exclusive && raw.is_none() {
            return;
        }
        let exclusive_supported = raw.is_some_and(|raw| monitors::supports_exclusive(raw.get_window_handle()));
        apply(&mut window, &mut settings, &mut modes, screen, &screens, exclusive_supported);
        modes.applied = Some(Preference::from(&*settings));
        modes.settle = 3;
    }
    settle(&mut window, &mut modes);
    let readout = Readout::from(&*window);
    if modes.last_status.as_ref() != Some(&readout) {
        log::info!("{}", monitors::status(&window));
        modes.last_status = Some(readout);
    }
}

fn apply(
    window: &mut Window,
    settings: &mut Settings,
    modes: &mut WindowModes,
    screen: &monitors::Screen,
    screens: &[monitors::Screen],
    exclusive_supported: bool,
) {
    let requested = Preference::from(&*settings);
    if window.mode == BevyMode::Windowed && (modes.settle == 0 || modes.windowed.is_none()) {
        modes.windowed = Some(Placement::from(&*window));
    }
    if requested.mode == WindowMode::Windowed {
        restore_windowed(window, modes, requested, screen, screens);
        return;
    }
    modes.restore = None;
    let target = MonitorSelection::Entity(screen.entity);
    if requested.mode == WindowMode::Borderless {
        window.mode = BevyMode::BorderlessFullscreen(target);
        modes.fullscreen = WindowMode::Borderless;
        return;
    }
    if !exclusive_supported {
        log::warn!("exclusive fullscreen is unsupported by this window backend; using borderless fullscreen");
        settings.window_mode = WindowMode::Borderless;
        window.mode = BevyMode::BorderlessFullscreen(target);
        modes.fullscreen = WindowMode::Borderless;
        return;
    }
    let Some(video) = monitors::video_mode(&screen.monitor, requested.resolution) else {
        log::warn!(
            "exclusive resolution {} is unavailable on monitor {}; using borderless fullscreen",
            requested.resolution,
            settings.monitor
        );
        settings.window_mode = WindowMode::Borderless;
        window.mode = BevyMode::BorderlessFullscreen(target);
        modes.fullscreen = WindowMode::Borderless;
        return;
    };
    window.mode = BevyMode::Fullscreen(target, VideoModeSelection::Specific(video));
    modes.fullscreen = WindowMode::Exclusive;
}

fn restore_windowed(
    window: &mut Window,
    modes: &mut WindowModes,
    requested: Preference,
    screen: &monitors::Screen,
    screens: &[monitors::Screen],
) {
    let mut placement = modes.windowed.unwrap_or_else(|| Placement::from(&*window));
    let resolution_changed = modes.applied.is_none() || modes.windowed_resolution != requested.resolution;
    if resolution_changed {
        placement.size = match requested.resolution {
            Resolution::Native => [1280, 720],
            Resolution::Pixels { width, height } => [width, height],
        };
    }
    let size = monitors::fit_size(placement.size, &screen.monitor);
    if size != placement.size {
        log::warn!(
            "window size {}x{} exceeds the desktop; using {}x{}",
            placement.size[0],
            placement.size[1],
            size[0],
            size[1]
        );
    }
    placement.size = size;
    let monitor_changed = modes.applied.is_none_or(|p| p.monitor != requested.monitor);
    if monitor_changed || !monitors::visible_position(placement.position, placement.size, screens) {
        placement.position = WindowPosition::Centered(MonitorSelection::Entity(screen.entity));
    }
    window.mode = BevyMode::Windowed;
    modes.windowed = Some(placement);
    modes.windowed_resolution = requested.resolution;
    modes.restore = Some(placement);
    set_placement(window, placement);
}

fn set_placement(window: &mut Window, placement: Placement) {
    window.resolution.set_physical_resolution(placement.size[0], placement.size[1]);
    window.position = placement.position;
}

fn settle(window: &mut Window, modes: &mut WindowModes) {
    if modes.settle > 0 {
        modes.settle -= 1;
        if let Some(restore) = modes.restore {
            set_placement(window, restore);
        }
        return;
    }
    modes.restore = None;
    if window.mode == BevyMode::Windowed && window.physical_width() > 0 && window.physical_height() > 0 {
        modes.windowed = Some(Placement::from(&*window));
    }
}
