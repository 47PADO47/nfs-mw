use super::*;
use crate::devtools::Console;
use crate::settings::Partial;
use bevy_app::{App, Update};
use bevy_input::{
    ButtonState,
    keyboard::{Key, KeyCode, KeyboardInput},
};
use bevy_window::{VideoMode, WindowEvent, WindowFocused};

fn screen(index: usize, primary: bool) -> monitors::Screen {
    monitors::Screen {
        entity: Entity::from_bits(index as u64 + 1),
        index: Some(index),
        primary,
        monitor: Monitor {
            name: Some(format!("screen {index}")),
            physical_width: 1920,
            physical_height: 1080,
            physical_position: (index as i32 * 1920, 0).into(),
            refresh_rate_millihertz: Some(60000),
            scale_factor: 1.5,
            video_modes: vec![video([1920, 1080], 60000), video([1920, 1080], 144000), video([1280, 720], 60000)],
        },
    }
}

fn video(size: [u32; 2], refresh_rate_millihertz: u32) -> VideoMode {
    VideoMode { physical_size: size.into(), bit_depth: 32, refresh_rate_millihertz }
}

fn defaults() -> Settings {
    Partial::default().into()
}

#[test]
fn monitor_indices_follow_native_inventory_and_bad_indices_fall_back() {
    let screens = [screen(1, false), screen(0, true)];
    let select = |target| monitors::select(&screens, None, target).unwrap();
    assert_eq!(select(settings::Monitor::Index(1)).0.entity, screens[0].entity);
    assert!(!select(settings::Monitor::Index(1)).1);
    assert_eq!(select(settings::Monitor::Index(99)).0.entity, screens[1].entity);
    assert!(select(settings::Monitor::Index(99)).1);
    assert_eq!(select(settings::Monitor::Current).0.entity, screens[1].entity);
    let current = OnMonitor(screens[0].entity);
    assert_eq!(monitors::select(&screens, Some(&current), settings::Monitor::Current).unwrap().0.entity, current.0);
    assert!(monitors::select(&[], None, settings::Monitor::Primary).is_none());
}

#[test]
fn exclusive_uses_real_modes_native_preserves_refresh_and_bad_modes_are_refused() {
    let screen = screen(0, true);
    assert_eq!(monitors::video_mode(&screen.monitor, Resolution::Native).unwrap().refresh_rate_millihertz, 60000);
    let size = Resolution::pixels(1920, 1080).unwrap();
    assert_eq!(monitors::video_mode(&screen.monitor, size).unwrap().refresh_rate_millihertz, 144000);
    assert!(monitors::video_mode(&screen.monitor, Resolution::pixels(1600, 900).unwrap()).is_none());
    let mut monitor = screen.monitor;
    monitor.video_modes.clear();
    assert!(monitors::video_mode(&monitor, Resolution::Native).is_none());
}

#[test]
fn unsupported_window_backends_fall_back_even_when_modes_are_advertised() {
    use raw_window_handle::{RawWindowHandle, WaylandWindowHandle, Win32WindowHandle};
    let windows = RawWindowHandle::Win32(Win32WindowHandle::new(std::num::NonZeroIsize::new(1).unwrap()));
    let wayland = RawWindowHandle::Wayland(WaylandWindowHandle::new(std::ptr::NonNull::dangling()));
    assert!(monitors::supports_exclusive(windows));
    assert!(!monitors::supports_exclusive(wayland));
    let screens = [screen(0, true)];
    let mut settings = defaults();
    settings.window_mode = WindowMode::Exclusive;
    let mut window = Window::default();
    let mut modes = WindowModes::new(false);
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, false);
    assert_eq!(settings.window_mode, WindowMode::Borderless);
    assert!(matches!(window.mode, BevyMode::BorderlessFullscreen(_)));
}

#[test]
fn mode_switches_restore_physical_size_and_position_after_intermediate_events() {
    let screens = [screen(0, true)];
    let position = WindowPosition::At([100, 150].into());
    let mut window = Window { position, resolution: (800, 600).into(), ..Window::default() };
    window.resolution.set_scale_factor(1.5);
    let mut settings = defaults();
    let mut modes = WindowModes::new(false);
    modes.applied = Some(Preference::from(&settings));
    settings.window_mode = WindowMode::Borderless;
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    modes.applied = Some(Preference::from(&settings));
    window.resolution.set_physical_resolution(1920, 1080);
    window.position = WindowPosition::At([0, 0].into());
    settle(&mut window, &mut modes);
    settings.window_mode = WindowMode::Windowed;
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    modes.applied = Some(Preference::from(&settings));
    modes.settle = 3;
    for _ in 0..3 {
        // A late fullscreen resize must not replace the remembered windowed placement.
        window.resolution.set_physical_resolution(1920, 1080);
        window.position = WindowPosition::At([0, 0].into());
        settle(&mut window, &mut modes);
        assert_eq!([window.physical_width(), window.physical_height()], [800, 600]);
        assert_eq!(window.position, position);
        assert_eq!(window.scale_factor(), 1.5);
    }
    settle(&mut window, &mut modes);
    // Normal user resizes after the transition become the next remembered size.
    window.resolution.set_physical_resolution(900, 650);
    settle(&mut window, &mut modes);
    assert_eq!(modes.windowed.unwrap().size, [900, 650]);
}

#[test]
fn unsupported_exclusive_falls_back_and_windowed_sizes_fit_the_desktop() {
    let screens = [screen(0, true)];
    let mut window = Window::default();
    let mut settings = defaults();
    settings.window_mode = WindowMode::Exclusive;
    settings.resolution = Resolution::pixels(1600, 900).unwrap();
    let mut modes = WindowModes::new(false);
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    assert_eq!(settings.window_mode, WindowMode::Borderless);
    assert!(matches!(window.mode, BevyMode::BorderlessFullscreen(_)));
    assert_eq!(modes.fullscreen, WindowMode::Borderless);
    settings.window_mode = WindowMode::Windowed;
    settings.resolution = Resolution::pixels(16384, 16384).unwrap();
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    assert_eq!([window.physical_width(), window.physical_height()], [1824, 984]);
}

#[test]
fn resolution_changed_during_borderless_is_used_when_returning_windowed() {
    let screens = [screen(0, true)];
    let mut settings = defaults();
    let mut window = Window { resolution: (800, 600).into(), ..Window::default() };
    let mut modes = WindowModes::new(false);
    modes.applied = Some(Preference::from(&settings));
    settings.window_mode = WindowMode::Borderless;
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    modes.applied = Some(Preference::from(&settings));
    settings.resolution = Resolution::pixels(1000, 700).unwrap();
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    modes.applied = Some(Preference::from(&settings));
    settings.window_mode = WindowMode::Windowed;
    apply(&mut window, &mut settings, &mut modes, &screens[0], &screens, true);
    assert_eq!([window.physical_width(), window.physical_height()], [1000, 700]);
}

#[test]
fn removed_monitor_positions_are_recentered_and_negative_desktop_positions_are_valid() {
    let mut screens = [screen(0, true)];
    assert!(!monitors::visible_position(WindowPosition::At([9000, 0].into()), [800, 600], &screens));
    screens[0].monitor.physical_position = [-1920, 0].into();
    assert!(monitors::visible_position(WindowPosition::At([-1800, 150].into()), [800, 600], &screens));
}

fn shortcut_app(hidden: bool, open_console: bool, focused: bool) -> App {
    let mut app = App::new();
    app.insert_resource(defaults())
        .insert_resource(WindowModes::new(hidden))
        .insert_resource(Console::default())
        .add_message::<WindowEvent>()
        .add_systems(Update, shortcut);
    app.world_mut().resource_mut::<Console>().open = open_console;
    app.world_mut().spawn((Window { focused, ..Window::default() }, PrimaryWindow));
    app
}

fn primary(app: &mut App) -> Entity {
    let world = app.world_mut();
    world.query_filtered::<Entity, With<PrimaryWindow>>().single(world).unwrap()
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState, repeat: bool) {
    let window = primary(app);
    app.world_mut().write_message(WindowEvent::KeyboardInput(KeyboardInput {
        key_code,
        logical_key: Key::Enter,
        state,
        text: None,
        repeat,
        window,
    }));
}

#[test]
fn alt_enter_chord_pressed_and_released_in_one_frame_toggles_once() {
    for alt in [KeyCode::AltLeft, KeyCode::AltRight] {
        let mut app = shortcut_app(false, false, true);
        key(&mut app, alt, ButtonState::Pressed, false);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        key(&mut app, KeyCode::Enter, ButtonState::Released, false);
        key(&mut app, alt, ButtonState::Released, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Borderless);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Borderless);
    }
}

#[test]
fn alt_enter_accepts_either_alt_ignores_repeat_and_returns_to_the_last_fullscreen() {
    for alt in [KeyCode::AltLeft, KeyCode::AltRight] {
        let mut app = shortcut_app(false, false, true);
        key(&mut app, alt, ButtonState::Pressed, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Windowed);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Borderless);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, true);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Borderless);
        key(&mut app, KeyCode::Enter, ButtonState::Released, false);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Windowed);
        app.world_mut().resource_mut::<WindowModes>().fullscreen = WindowMode::Exclusive;
        key(&mut app, KeyCode::Enter, ButtonState::Released, false);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Exclusive);
    }
}

#[test]
fn shortcut_is_inactive_for_screenshots_console_and_unfocused_windows() {
    for (hidden, console, focused) in [(true, false, true), (false, true, true), (false, false, false)] {
        let mut app = shortcut_app(hidden, console, focused);
        key(&mut app, KeyCode::AltLeft, ButtonState::Pressed, false);
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        app.update();
        assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Windowed);
    }
}

#[test]
fn focus_loss_resets_held_modifiers_even_if_focus_returns_in_the_same_frame() {
    let mut app = shortcut_app(false, false, true);
    let window = primary(&mut app);
    key(&mut app, KeyCode::AltLeft, ButtonState::Pressed, false);
    app.update();
    app.world_mut().write_message(WindowEvent::WindowFocused(WindowFocused { window, focused: false }));
    app.world_mut().write_message(WindowEvent::WindowFocused(WindowFocused { window, focused: true }));
    key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
    app.update();
    assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Windowed);
}

#[test]
fn pressing_alt_after_enter_does_not_count_as_alt_enter() {
    let mut app = shortcut_app(false, false, true);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
    key(&mut app, KeyCode::AltLeft, ButtonState::Pressed, false);
    app.update();
    assert_eq!(app.world().resource::<Settings>().window_mode, WindowMode::Windowed);
}

#[test]
fn hidden_window_never_receives_interactive_preferences() {
    let mut app = App::new();
    let settings = Settings::from(Partial {
        window_mode: Some(WindowMode::Exclusive),
        resolution: Some(Resolution::pixels(3840, 2160).unwrap()),
        ..Partial::default()
    });
    app.insert_resource(settings)
        .insert_resource(WindowModes::new(true))
        .insert_resource(WinitMonitors::default())
        .add_systems(Update, update);
    let entity = app.world_mut().spawn((Window { visible: false, ..Window::default() }, PrimaryWindow)).id();
    app.update();
    let window = app.world().get::<Window>(entity).unwrap();
    assert!(!window.visible);
    assert_eq!(window.mode, BevyMode::Windowed);
    assert_eq!([window.physical_width(), window.physical_height()], [1280, 720]);
}
