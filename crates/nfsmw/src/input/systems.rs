//! The Bevy side of the input layer: reads `bevy_input` and resolves the actions.

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadButton, GamepadInput};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseScrollUnit};
use bevy_input::{ButtonInput, InputSystems};
use bevy_time::Time;

use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

/// Whether the cursor is captured for mouse look. The window code sets it; the input layer reads it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MouseCapture(pub bool);

/// The UI (the console) has the keyboard: game actions go quiet.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UiFocus(pub bool);

pub struct InputLayerPlugin;

impl Plugin for InputLayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bindings>()
            .init_resource::<ActionState>()
            .init_resource::<MouseCapture>()
            .init_resource::<UiFocus>()
            .add_systems(PreUpdate, update_actions.after(InputSystems));
    }
}

/// Pixels per line, for devices that scroll in pixels.
const PIXELS_PER_LINE: f32 = 60.0;

#[allow(clippy::too_many_arguments)]
fn update_actions(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    pads: Query<&Gamepad>,
    capture: Res<MouseCapture>,
    focus: Res<UiFocus>,
    time: Res<Time>,
    bindings: Res<Bindings>,
    mut state: ResMut<ActionState>,
) {
    let mut snapshot = Snapshot {
        // Keep a complete down/up pulse visible for one action frame, even when the key is no
        // longer held by the time this system runs (console/camera/gear shortcuts need the edge).
        keys: keys.get_pressed().chain(keys.get_just_pressed()).copied().collect(),
        buttons: buttons.get_pressed().copied().collect(),
        mouse_delta: (motion.delta.x, motion.delta.y),
        scroll: match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
        },
        mouse_captured: capture.0,
        dt: time.delta_secs(),
        ..Snapshot::default()
    };
    for pad in &pads {
        snapshot.pad_buttons.extend(pad.get_pressed().copied());
        for button in [GamepadButton::LeftTrigger2, GamepadButton::RightTrigger2] {
            if let Some(value) = pad.get(button) {
                let entry = snapshot.pad_triggers.entry(button).or_insert(0.0);
                *entry = entry.max(value);
            }
        }
        for input in pad.get_analog_axes() {
            let GamepadInput::Axis(axis) = *input else { continue };
            let value = pad.get(axis).unwrap_or(0.0);
            let entry = snapshot.pad_axes.entry(axis).or_insert(0.0);
            if value.abs() > entry.abs() {
                *entry = value;
            }
        }
    }
    state.update(&bindings, &snapshot, focus.0);
}

#[cfg(test)]
mod tests {
    use bevy_input::keyboard::{Key, KeyboardInput};
    use bevy_input::{ButtonState, InputPlugin};

    use super::*;
    use crate::input::Action;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((InputPlugin, InputLayerPlugin)).init_resource::<Time>();
        app
    }

    fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Enter,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    #[test]
    fn fast_keyboard_taps_reach_switch_actions_for_one_frame() {
        for (key_code, action) in [(KeyCode::F12, Action::Console), (KeyCode::KeyF, Action::ToggleCamera)] {
            let mut app = app();
            key(&mut app, key_code, ButtonState::Pressed);
            key(&mut app, key_code, ButtonState::Released);
            app.update();
            assert!(app.world().resource::<ActionState>().just_pressed(action));
            assert!(!app.world().resource::<ButtonInput<KeyCode>>().pressed(key_code));
            app.update();
            let actions = app.world().resource::<ActionState>();
            assert!(!actions.pressed(action));
            assert!(!actions.just_pressed(action));
        }
    }

    #[test]
    fn held_keyboard_controls_keep_their_previous_response() {
        let mut app = app();
        key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
        app.update();
        assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), 1.0);
        assert!(app.world().resource::<ActionState>().just_pressed(Action::Throttle));
        app.update();
        assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), 1.0);
        assert!(!app.world().resource::<ActionState>().just_pressed(Action::Throttle));
        key(&mut app, KeyCode::KeyW, ButtonState::Released);
        app.update();
        assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), 0.0);
    }

    #[test]
    fn ui_focus_suppresses_game_controls_even_for_fast_taps() {
        let mut app = app();
        app.world_mut().resource_mut::<UiFocus>().0 = true;
        for key_code in [KeyCode::F12, KeyCode::KeyF, KeyCode::KeyW] {
            key(&mut app, key_code, ButtonState::Pressed);
            key(&mut app, key_code, ButtonState::Released);
        }
        app.update();
        let actions = app.world().resource::<ActionState>();
        assert!(actions.just_pressed(Action::Console));
        assert!(!actions.pressed(Action::ToggleCamera));
        assert_eq!(actions.value(Action::Throttle), 0.0);
    }
}
