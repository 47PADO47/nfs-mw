//! Alt+Enter from ordered window events, including chords pressed and released within one frame.

use bevy_ecs::prelude::*;
use bevy_input::{ButtonState, keyboard::KeyCode};
use bevy_window::{PrimaryWindow, Window, WindowEvent};

use super::WindowModes;
use crate::devtools::Console;
use crate::settings::{Settings, WindowMode};

#[derive(Default)]
pub(in crate::app) struct Held {
    left_alt: bool,
    right_alt: bool,
    enter: bool,
}

/// The common WindowEvent stream preserves keyboard/focus ordering from winit. Reading only
/// ButtonInput's final state loses a chord when both Alt and Enter have already been released.
pub(in crate::app) fn update(
    mut events: MessageReader<WindowEvent>,
    mut held: Local<Held>,
    console: Res<Console>,
    mut settings: ResMut<Settings>,
    modes: Res<WindowModes>,
    window: Single<(Entity, &Window), With<PrimaryWindow>>,
) {
    let (entity, window) = *window;
    if modes.hidden || console.open || !window.focused {
        *held = Held::default();
        events.clear();
        return;
    }
    let mut focused = true;
    for event in events.read() {
        match event {
            WindowEvent::WindowFocused(event) if event.window == entity => {
                focused = event.focused;
                if !focused {
                    *held = Held::default();
                }
            }
            WindowEvent::KeyboardFocusLost(_) => {
                focused = false;
                *held = Held::default();
            }
            WindowEvent::KeyboardInput(input) if input.window == entity && focused => {
                let pressed = input.state == ButtonState::Pressed;
                match input.key_code {
                    KeyCode::AltLeft => held.left_alt = pressed,
                    KeyCode::AltRight => held.right_alt = pressed,
                    KeyCode::Enter => {
                        if pressed && !input.repeat && !held.enter && (held.left_alt || held.right_alt) {
                            settings.window_mode = match settings.window_mode {
                                WindowMode::Windowed => modes.fullscreen,
                                _ => WindowMode::Windowed,
                            };
                        }
                        held.enter = pressed;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
