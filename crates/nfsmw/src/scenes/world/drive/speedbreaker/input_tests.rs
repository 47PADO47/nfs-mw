//! Physical input reaches the actual driving input and toggle meter, including saved wheel mappings.

use super::super::DriveInput;
use super::SpeedBreaker;
use crate::input::{Action, ActionState, Bindings, Snapshot};
use crate::settings::WheelOptions;
use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
    mouse::MouseButton,
};

fn step(meter: &mut SpeedBreaker, state: &mut ActionState, bindings: &Bindings, raw: &Snapshot) -> bool {
    state.update(bindings, raw, false);
    let input = DriveInput::from_actions(state, WheelOptions::default());
    meter.step(input.speedbreaker, 50.0, 1.0 / 120.0, 1.0 / 120.0)
}

fn exercise_toggle(bindings: &Bindings, pressed: Snapshot) {
    let mut state = ActionState::default();
    let mut meter = SpeedBreaker::default();
    let released = Snapshot::default();
    assert!(!step(&mut meter, &mut state, bindings, &released));
    assert!(step(&mut meter, &mut state, bindings, &pressed));
    assert!(state.just_pressed(Action::Speedbreaker));
    assert!(!state.pressed(Action::ShiftDown), "Speedbreaker must not also downshift");
    assert!(!state.pressed(Action::Nos), "Speedbreaker must not also burn nitrous");
    assert!(step(&mut meter, &mut state, bindings, &pressed), "holding must not retoggle");
    assert!(!state.just_pressed(Action::Speedbreaker));
    assert!(step(&mut meter, &mut state, bindings, &released), "release retains the active breaker");
    assert!(!step(&mut meter, &mut state, bindings, &pressed), "second press toggles off");
    assert!(!step(&mut meter, &mut state, bindings, &released));
    state.update(bindings, &pressed, true);
    assert!(!DriveInput::from_actions(&state, WheelOptions::default()).speedbreaker, "UI focus must block gameplay");
}

#[test]
fn keyboard_mouse_and_gamepad_defaults_toggle_speedbreaker_without_driving_conflicts() {
    let mut keyboard = Snapshot::default();
    keyboard.keys.insert(KeyCode::ControlLeft);
    let mut mouse = Snapshot::default();
    mouse.buttons.insert(MouseButton::Back);
    let mut gamepad = Snapshot::default();
    gamepad.pad_buttons.insert(GamepadButton::West);
    for snapshot in [keyboard, mouse, gamepad] {
        exercise_toggle(&Bindings::default(), snapshot);
    }
    let mut right_ctrl = Snapshot::default();
    right_ctrl.keys.insert(KeyCode::ControlRight);
    let mut state = ActionState::default();
    state.update(&Bindings::default(), &right_ctrl, false);
    assert!(state.just_pressed(Action::ShiftDown));
    assert!(!state.pressed(Action::Speedbreaker));
}

#[test]
fn persisted_wheel_button_and_analog_sources_reach_the_same_toggle_meter() {
    let mut wheel_button = Snapshot::default();
    wheel_button.pad_buttons.insert(GamepadButton::Other(21));
    let mut trigger = Snapshot::default();
    trigger.pad_triggers.insert(GamepadButton::Other(7), 1.0);
    let mut axis = Snapshot::default();
    axis.pad_axes.insert(GamepadAxis::Other(4), 1.0);
    let mut inverted = Snapshot::default();
    inverted.pad_axes.insert(GamepadAxis::Other(4), -1.0);
    for (source, pressed) in [
        ("button:Other(21)", wheel_button),
        ("trigger:Other(7)", trigger),
        ("axis:Other(4)", axis.clone()),
        ("pedal:Other(4)", axis),
        ("pedal_inv:Other(4)", inverted),
    ] {
        let mut bindings = Bindings::default();
        bindings.bind(Action::Speedbreaker, source, false).unwrap();
        let saved = bindings.merge_config("unknown_setting = 'preserved'").unwrap();
        let mut restored = Bindings::default();
        restored.apply_config(&saved, "synthetic saved controls");
        for action in Action::ALL {
            assert_eq!(
                bindings.0.iter().filter(|binding| binding.action == action).collect::<Vec<_>>(),
                restored.0.iter().filter(|binding| binding.action == action).collect::<Vec<_>>(),
                "{source}: {} must survive save and load",
                action.name()
            );
        }
        assert!(saved.contains("unknown_setting"));
        exercise_toggle(&restored, pressed);
        let mut old = Snapshot::default();
        old.pad_buttons.insert(GamepadButton::West);
        let mut state = ActionState::default();
        state.update(&restored, &old, false);
        assert!(!state.pressed(Action::Speedbreaker), "rebinding replaces the previous pad-family source");
    }
}

#[test]
fn analog_threshold_release_allows_another_press_and_live_listing_tracks_rebinding() {
    let mut bindings = Bindings::default();
    bindings.bind(Action::Speedbreaker, "trigger:Other(7)", false).unwrap();
    bindings.bind(Action::Speedbreaker, "key:J", false).unwrap();
    let text = bindings.describe();
    let line = text.lines().find(|line| line.contains("Speedbreaker (press to toggle)")).unwrap();
    assert!(line.contains("J") && line.contains("Pad button 7"), "{line}");
    assert!(!line.contains("Left Ctrl"), "prompts must reflect the rebound key");
    let mut state = ActionState::default();
    let mut meter = SpeedBreaker::default();
    let mut raw = Snapshot::default();
    raw.pad_triggers.insert(GamepadButton::Other(7), 0.5);
    assert!(!step(&mut meter, &mut state, &bindings, &raw));
    raw.pad_triggers.insert(GamepadButton::Other(7), 0.51);
    assert!(step(&mut meter, &mut state, &bindings, &raw));
    raw.pad_triggers.insert(GamepadButton::Other(7), 0.5);
    assert!(step(&mut meter, &mut state, &bindings, &raw));
    raw.pad_triggers.insert(GamepadButton::Other(7), 0.51);
    assert!(!step(&mut meter, &mut state, &bindings, &raw));
}

#[test]
fn physical_mouse_taps_survive_bevy_input_updates_and_holds_do_not_repeat_the_toggle() {
    use crate::input::{InputLayerPlugin, UiFocus};
    use bevy_app::App;
    use bevy_ecs::entity::Entity;
    use bevy_input::{ButtonInput, ButtonState, InputPlugin, mouse::MouseButtonInput};
    use bevy_time::{Real, Time};

    let mut app = App::new();
    app.add_plugins((InputPlugin, InputLayerPlugin)).init_resource::<Time<Real>>();
    let mut meter = SpeedBreaker::default();
    let event = |app: &mut App, state| {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Back,
            state,
            window: Entity::PLACEHOLDER,
        });
    };
    let update = |app: &mut App, meter: &mut SpeedBreaker| {
        app.update();
        let input = DriveInput::from_actions(app.world().resource::<ActionState>(), WheelOptions::default());
        meter.step(input.speedbreaker, 50.0, 1.0 / 240.0, 1.0 / 240.0)
    };

    // A complete physical pulse arrives between renders, leaving no held mouse button.
    event(&mut app, ButtonState::Pressed);
    event(&mut app, ButtonState::Released);
    assert!(update(&mut app, &mut meter));
    assert!(!app.world().resource::<ButtonInput<MouseButton>>().pressed(MouseButton::Back));
    assert!(app.world().resource::<ActionState>().just_pressed(Action::Speedbreaker));
    assert!(update(&mut app, &mut meter), "the following released frame must retain active state");
    assert!(!app.world().resource::<ActionState>().pressed(Action::Speedbreaker));

    event(&mut app, ButtonState::Pressed);
    assert!(!update(&mut app, &mut meter), "a second press toggles off");
    for _ in 0..3 {
        assert!(!update(&mut app, &mut meter), "a sustained mouse hold must not pulse again");
        assert!(!app.world().resource::<ActionState>().just_pressed(Action::Speedbreaker));
    }
    event(&mut app, ButtonState::Released);
    assert!(!update(&mut app, &mut meter));
    event(&mut app, ButtonState::Pressed);
    event(&mut app, ButtonState::Released);
    assert!(update(&mut app, &mut meter), "another physical tap toggles back on");
    assert!(update(&mut app, &mut meter));

    app.world_mut().resource_mut::<UiFocus>().0 = true;
    event(&mut app, ButtonState::Pressed);
    event(&mut app, ButtonState::Released);
    assert!(update(&mut app, &mut meter));
    assert!(!app.world().resource::<ActionState>().pressed(Action::Speedbreaker), "UI focus suppresses even fast taps");
}
