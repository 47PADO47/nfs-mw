//! The Bevy side of the input layer: reads `bevy_input` and resolves the actions.

use std::collections::{HashMap, HashSet};

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadInput};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseScrollUnit};
use bevy_input::{ButtonInput, InputSystems};
use bevy_time::Time;

use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};
use super::{Action, InputPresentation, bindings::Source};

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
            .init_resource::<InputPresentation>()
            .add_observer(super::device_settings::configure_added_gamepad)
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
    pads: Query<(Entity, &Gamepad)>,
    capture: Res<MouseCapture>,
    focus: Res<UiFocus>,
    time: Res<Time>,
    bindings: Res<Bindings>,
    settings: Option<Res<crate::settings::Settings>>,
    mut state: ResMut<ActionState>,
    mut presentation: ResMut<InputPresentation>,
    mut held: Local<HashSet<GamepadButton>>,
    mut logged_axes: Local<HashMap<GamepadAxis, f32>>,
) {
    presentation.sample_pads(&pads, &bindings);
    let key_activity = keys.get_just_pressed().any(|key| {
        bindings
            .0
            .iter()
            .any(|b| b.scale != 0.0 && b.action != Action::Console && matches!(b.source, Source::Key(v) if v == *key))
    });
    if key_activity
        || buttons.get_just_pressed().next().is_some()
        || motion.delta.length_squared() > 4.0
        || scroll.delta.length_squared() > 0.0
    {
        presentation.keyboard_activity();
    }
    let mut snapshot = Snapshot {
        controls: settings.as_deref().map_or_else(crate::settings::Controls::default, |s| s.controls),
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
    for (_, pad) in pads.iter().filter(|(e, _)| Some(*e) == presentation.active_pad) {
        snapshot.pad_buttons.extend(pad.get_pressed().chain(pad.get_just_pressed()).copied());
        // Bevy's analog store also contains buttons, including nonstandard wheel pedals.
        for input in pad.get_analog_axes() {
            let value = pad.get(*input).unwrap_or(0.0);
            match *input {
                GamepadInput::Button(button) => {
                    let entry = snapshot.pad_triggers.entry(button).or_insert(0.0);
                    *entry = entry.max(value);
                }
                GamepadInput::Axis(axis) => {
                    let entry = snapshot.pad_axes.entry(axis).or_insert(0.0);
                    if value.abs() > entry.abs() {
                        *entry = value;
                    }
                }
            }
        }
    }
    for code in newly_pressed_unnamed(&snapshot.pad_buttons, &held) {
        log::info!(
            "gamepad button {code} pressed (a button without a name: `paddle_up = {code}` or `paddle_down = {code}` in the config file binds it to a gear shift)"
        );
    }
    for (axis, value) in moved_wheel_axes(&snapshot.pad_axes, &mut logged_axes) {
        log::info!(
            "gamepad axis {axis:?} is at {value:.2} (a steering wheel's pedal: `bind throttle pedal:{axis:?}`, or `pedal_inv:{axis:?}` when it reads high while released; the brake and the clutch the same way)"
        );
    }
    *held = snapshot.pad_buttons.clone();
    state.update(&bindings, &snapshot, focus.0);
}

/// The codes of the buttons the platform has no name for that went down since `before`, in order.
fn newly_pressed_unnamed(now: &HashSet<GamepadButton>, before: &HashSet<GamepadButton>) -> Vec<u32> {
    let mut codes: Vec<u32> = now
        .iter()
        .filter(|b| !before.contains(b))
        .filter_map(|b| match b {
            GamepadButton::Other(code) => Some(*code),
            _ => None,
        })
        .collect();
    codes.sort_unstable();
    codes
}

/// The axes that are not a stick (a wheel's pedals arrive as these) which were first seen, or moved by half the
/// travel since they were last reported, in name order. `logged` remembers what was reported.
fn moved_wheel_axes(
    now: &HashMap<GamepadAxis, f32>,
    logged: &mut HashMap<GamepadAxis, f32>,
) -> Vec<(GamepadAxis, f32)> {
    let is_stick = |axis: &GamepadAxis| {
        matches!(
            axis,
            GamepadAxis::LeftStickX | GamepadAxis::LeftStickY | GamepadAxis::RightStickX | GamepadAxis::RightStickY
        )
    };
    let mut moved: Vec<(GamepadAxis, f32)> = now
        .iter()
        .filter(|(axis, value)| !is_stick(axis) && value.is_finite())
        .filter(|(axis, value)| logged.get(*axis).is_none_or(|before| (**value - before).abs() >= 0.5))
        .map(|(axis, value)| (*axis, *value))
        .collect();
    moved.sort_by_key(|(axis, _)| format!("{axis:?}"));
    logged.extend(moved.iter().copied());
    moved
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
    fn live_remapping_reaches_actions_in_the_next_input_frame() {
        let mut app = app();
        app.world_mut().resource_mut::<Bindings>().bind(Action::Throttle, "key:J", false).unwrap();
        key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
        app.update();
        assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), 0.0);
        key(&mut app, KeyCode::KeyJ, ButtonState::Pressed);
        app.update();
        assert!(app.world().resource::<ActionState>().just_pressed(Action::Throttle));
        key(&mut app, KeyCode::KeyJ, ButtonState::Released);
        app.update();
        assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), 0.0);
    }

    #[test]
    fn nonstandard_analog_button_pedals_preserve_partial_travel() {
        use bevy_input::gamepad::{
            GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
        };
        let mut app = app();
        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Bindings>().bind(Action::Throttle, "trigger:Other(7)", false).unwrap();
        app.world_mut().write_message(GamepadConnectionEvent::new(
            entity,
            GamepadConnection::Connected { name: "Software pedal".into(), vendor_id: None, product_id: None },
        ));
        for value in [0.4, 0.75, 1.0, 0.7, 0.65, 0.01, 0.0] {
            app.world_mut().write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::Other(7),
                value,
            )));
            app.update();
            assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), value);
        }
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

    #[test]
    fn pedal_axes_are_reported_on_first_sight_and_on_big_moves_only() {
        let other = GamepadAxis::Other(3);
        let mut logged = HashMap::new();
        let mut now: HashMap<_, _> = [(GamepadAxis::LeftStickX, 1.0), (other, -1.0)].into();
        assert_eq!(moved_wheel_axes(&now, &mut logged), vec![(other, -1.0)], "sticks are not reported");
        assert!(moved_wheel_axes(&now, &mut logged).is_empty(), "nothing moved");
        now.insert(other, -0.8);
        assert!(moved_wheel_axes(&now, &mut logged).is_empty(), "a small move is not reported");
        now.insert(other, 0.1);
        assert_eq!(moved_wheel_axes(&now, &mut logged), vec![(other, 0.1)]);
        now.insert(other, f32::NAN);
        assert!(moved_wheel_axes(&now, &mut logged).is_empty());
    }

    #[test]
    fn only_new_unnamed_buttons_are_reported() {
        let held: HashSet<_> = [GamepadButton::Other(3), GamepadButton::South].into();
        let now: HashSet<_> =
            [GamepadButton::Other(3), GamepadButton::Other(9), GamepadButton::Other(4), GamepadButton::East].into();
        assert_eq!(newly_pressed_unnamed(&now, &held), vec![4, 9]);
        assert!(newly_pressed_unnamed(&held, &held).is_empty());
    }
}
