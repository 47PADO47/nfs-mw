//! Monitor selection and exclusive-video-mode validation.

use bevy_ecs::prelude::*;
use bevy_window::{Monitor, OnMonitor, PrimaryMonitor, VideoMode, Window, WindowPosition};
use bevy_winit::WinitMonitors;
use raw_window_handle::RawWindowHandle;

use crate::settings::{self, Resolution};

#[derive(Clone)]
pub(in crate::app) struct Screen {
    pub entity: Entity,
    pub index: Option<usize>,
    pub primary: bool,
    pub monitor: Monitor,
}

pub(super) fn inventory(
    query: &Query<(Entity, Ref<Monitor>, Has<PrimaryMonitor>)>,
    native: &WinitMonitors,
) -> Vec<Screen> {
    query
        .iter()
        .map(|(entity, monitor, primary)| {
            let handle = native.find_entity(entity);
            let index =
                handle.and_then(|handle| (0..query.iter().count()).find(|&i| native.nth(i).as_ref() == Some(&handle)));
            Screen { entity, index, primary, monitor: (*monitor).clone() }
        })
        .collect()
}

/// Missing indices fall back to the primary monitor; `current` follows the actual window.
pub(super) fn select<'a>(
    screens: &'a [Screen],
    current: Option<&OnMonitor>,
    target: settings::Monitor,
) -> Option<(&'a Screen, bool)> {
    let selected = match target {
        settings::Monitor::Current => current.and_then(|on| screens.iter().find(|s| s.entity == on.0)),
        settings::Monitor::Primary => screens.iter().find(|s| s.primary),
        settings::Monitor::Index(index) => screens.iter().find(|s| s.index == Some(index)),
    };
    if let Some(screen) = selected {
        return Some((screen, false));
    }
    let fallback = screens.iter().find(|s| s.primary).or_else(|| screens.first())?;
    Some((fallback, target != settings::Monitor::Current))
}

/// Never manufacture a video mode. Native preserves the desktop refresh rate where available.
pub(super) fn video_mode(monitor: &Monitor, resolution: Resolution) -> Option<VideoMode> {
    let (width, height) = match resolution {
        Resolution::Native => (monitor.physical_width, monitor.physical_height),
        Resolution::Pixels { width, height } => (width, height),
    };
    let modes = monitor.video_modes.iter().filter(|mode| mode.physical_size.to_array() == [width, height]);
    if resolution == Resolution::Native {
        let refresh = monitor.refresh_rate_millihertz.unwrap_or(60000);
        return modes
            .max_by_key(|m| (std::cmp::Reverse(m.refresh_rate_millihertz.abs_diff(refresh)), m.bit_depth))
            .copied();
    }
    modes.max_by_key(|m| (m.refresh_rate_millihertz, m.bit_depth)).copied()
}

/// winit supports exclusive mode on the desktop Windows, X11 and macOS backends.
/// Wayland exposes video modes but ignores exclusive requests, so it must fall back explicitly.
pub(super) fn supports_exclusive(handle: RawWindowHandle) -> bool {
    matches!(
        handle,
        RawWindowHandle::Win32(_) | RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) | RawWindowHandle::AppKit(_)
    )
}

/// Leave space for the title bar/taskbar; the OS retains responsibility for the work area.
pub(super) fn fit_size(size: [u32; 2], monitor: &Monitor) -> [u32; 2] {
    let margin = (64.0 * monitor.scale_factor) as u32;
    [
        size[0].min(monitor.physical_width.saturating_sub(margin).max(160)),
        size[1].min(monitor.physical_height.saturating_sub(margin).max(160)),
    ]
}

/// Restore a saved top-left only while a useful part of the window remains on a connected screen.
pub(super) fn visible_position(position: WindowPosition, size: [u32; 2], screens: &[Screen]) -> bool {
    let WindowPosition::At(at) = position else { return false };
    screens.iter().any(|screen| {
        let m = &screen.monitor;
        let [x, y] = at.to_array().map(i64::from);
        let [left, top] = m.physical_position.to_array().map(i64::from);
        let overlap_x = (x + i64::from(size[0])).min(left + i64::from(m.physical_width)) - x.max(left);
        let overlap_y = (y + i64::from(size[1])).min(top + i64::from(m.physical_height)) - y.max(top);
        overlap_x >= 64 && overlap_y >= 64
    })
}

pub(super) fn describe(screens: &[Screen]) -> String {
    screens
        .iter()
        .map(|s| {
            let m = &s.monitor;
            format!(
                "monitor {}: {} {}x{} scale {:.2}, {:.3} Hz, {} video modes{}",
                s.index.map_or_else(|| "?".to_owned(), |i| i.to_string()),
                m.name.as_deref().unwrap_or("unnamed"),
                m.physical_width,
                m.physical_height,
                m.scale_factor,
                m.refresh_rate_millihertz.unwrap_or(0) as f64 / 1000.0,
                m.video_modes.len(),
                match s.primary {
                    true => " (primary)",
                    false => "",
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn status(window: &Window) -> String {
    format!(
        "window {:?}: {}x{} physical, {:.0}x{:.0} logical, scale {:.2}, position {:?}, focused {}, visible {}",
        window.mode,
        window.physical_width(),
        window.physical_height(),
        window.width(),
        window.height(),
        window.scale_factor(),
        window.position,
        window.focused,
        window.visible
    )
}
