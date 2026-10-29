use super::*;
use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
};

#[test]
fn batch12_controller_slots_preserve_keyboard_menu_and_other_steering_buttons() {
    let mut bindings = Bindings::default();
    let keyboard = bindings.keyboard_keys(Control::button(Action::Throttle));
    let menu: Vec<_> = bindings.0.iter().filter(|binding| binding.action == Action::MenuBack).copied().collect();
    bindings.set_gamepad_slot(Control::button(Action::Throttle), 0, Source::PadButton(GamepadButton::East), 1.0);
    assert_eq!(bindings.keyboard_keys(Control::button(Action::Throttle)), keyboard);
    assert_eq!(
        bindings.0.iter().filter(|binding| binding.action == Action::MenuBack).copied().collect::<Vec<_>>(),
        menu
    );
    bindings.set_gamepad_slot(Control::axis(Action::Steer, -1.0), 0, Source::PadButton(GamepadButton::DPadLeft), 1.0);
    bindings.set_gamepad_slot(Control::axis(Action::Steer, 1.0), 0, Source::PadButton(GamepadButton::DPadRight), 1.0);
    let steering: Vec<_> = bindings
        .0
        .iter()
        .filter(|binding| binding.action == Action::Steer && matches!(binding.source, Source::PadButton(_)))
        .collect();
    assert_eq!(steering.len(), 2);
    assert_eq!(steering[0].scale, -1.0);
    assert_eq!(steering[1].scale, 1.0);
    assert!(bindings.keyboard_keys(Control::axis(Action::Steer, -1.0)).contains(&KeyCode::ArrowLeft));
}

#[test]
fn batch12_capturing_either_steering_direction_keeps_one_full_axis() {
    let mut bindings = Bindings::default();
    for (direction, travel, scale) in [(-1.0, -0.9, 1.0), (1.0, 0.8, 1.0), (-1.0, 0.8, -1.0)] {
        bindings.set_gamepad_slot(
            Control::axis(Action::Steer, direction),
            0,
            Source::PadAxis(GamepadAxis::LeftStickX),
            travel,
        );
        let axes: Vec<_> = bindings
            .0
            .iter()
            .filter(|binding| binding.action == Action::Steer && matches!(binding.source, Source::PadAxis(_)))
            .collect();
        assert_eq!(axes.len(), 1, "a full axis is not duplicated for left and right rows");
        assert_eq!(axes[0].scale, scale);
        assert_eq!(bindings.gamepad_labels(Control::axis(Action::Steer, -1.0)).len(), 1);
        assert_eq!(bindings.gamepad_labels(Control::axis(Action::Steer, 1.0)).len(), 1);
    }
}

#[test]
fn batch12_controller_slots_persist_buttons_triggers_and_axes_without_losing_config() {
    let mut bindings = Bindings::default();
    bindings.set_gamepad_slot(Control::button(Action::Handbrake), 1, Source::PadButton(GamepadButton::Other(7)), 1.0);
    bindings.set_gamepad_slot(
        Control::button(Action::Throttle),
        0,
        Source::PadTrigger(GamepadButton::RightTrigger2),
        1.0,
    );
    bindings.set_gamepad_slot(Control::axis(Action::Steer, -1.0), 0, Source::PadAxis(GamepadAxis::RightStickX), -0.8);
    let path = std::env::temp_dir().join(format!("nfsmw-controller-slots-{}.toml", std::process::id()));
    std::fs::write(&path, "game_dir='preserved'\n").unwrap();
    bindings.save_to(&path).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("preserved"));
    let mut loaded = Bindings::default();
    loaded.apply_config(&saved, "controller slot test");
    for control in
        [Control::button(Action::Handbrake), Control::button(Action::Throttle), Control::axis(Action::Steer, -1.0)]
    {
        assert_eq!(loaded.gamepad_labels(control), bindings.gamepad_labels(control));
        assert_eq!(loaded.keyboard_keys(control), bindings.keyboard_keys(control));
    }
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn batch12_controller_reassignment_moves_a_trigger_away_from_competing_driving_actions() {
    let mut bindings = Bindings::default();
    let before = bindings.0.clone();
    bindings.set_gamepad_slot(
        Control::button(Action::Throttle),
        0,
        Source::PadTrigger(GamepadButton::LeftTrigger2),
        1.0,
    );
    let mut physical = super::super::snapshot::Snapshot::default();
    physical.pad_triggers.insert(GamepadButton::LeftTrigger2, 1.0);
    let mut state = super::super::ActionState::default();
    state.update(&bindings, &physical, false);
    assert_eq!(state.value(Action::Throttle), 1.0, "the captured trigger accelerates");
    assert_eq!(state.value(Action::Brake), 0.0, "the old brake owner cannot fire at the same time");
    for action in [Action::MenuTriggerLeft, Action::MenuZoomOut, Action::OrbitY, Action::MoveForward] {
        assert_eq!(
            bindings.0.iter().filter(|binding| binding.action == action).collect::<Vec<_>>(),
            before.iter().filter(|binding| binding.action == action).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        bindings.keyboard_keys(Control::button(Action::Brake)),
        Bindings(before).keyboard_keys(Control::button(Action::Brake))
    );
}

#[test]
fn batch12_controller_steering_axis_remap_preserves_camera_axis_bindings() {
    let mut bindings = Bindings::default();
    let orbit = bindings.0.iter().filter(|binding| binding.action == Action::OrbitY).copied().collect::<Vec<_>>();
    bindings.set_gamepad_slot(Control::axis(Action::Steer, -1.0), 0, Source::PadAxis(GamepadAxis::RightStickY), -0.8);
    let mut physical = super::super::snapshot::Snapshot::default();
    physical.pad_axes.insert(GamepadAxis::RightStickY, -0.5);
    let mut state = super::super::ActionState::default();
    state.update(&bindings, &physical, false);
    assert!(state.value(Action::Steer) < 0.0);
    assert_eq!(state.value(Action::ShiftDown), 0.0);
    assert_eq!(state.value(Action::ShiftUp), 0.0);
    assert_eq!(
        bindings.0.iter().filter(|binding| binding.action == Action::OrbitY).copied().collect::<Vec<_>>(),
        orbit
    );
}

#[test]
fn batch12_controller_full_axis_capture_replaces_a_pedal_alias_of_that_axis() {
    let mut bindings = Bindings::default();
    bindings.0.push(Binding::new(Action::Brake, Source::PedalAxis { axis: GamepadAxis::LeftZ, inverted: false }, 1.0));
    bindings.set_gamepad_slot(Control::button(Action::Throttle), 0, Source::PadAxis(GamepadAxis::LeftZ), 0.9);
    let mut physical = super::super::snapshot::Snapshot::default();
    physical.pad_axes.insert(GamepadAxis::LeftZ, 0.9);
    let mut state = super::super::ActionState::default();
    state.update(&bindings, &physical, false);
    assert!(state.value(Action::Throttle) > 0.0);
    assert_eq!(state.value(Action::Brake), 0.0, "the pedal alias must not retain a second vehicle-control owner");
}
