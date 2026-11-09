//! The air control's axes (`docs/specs/air-control.md`): the keys that drive are a stick, the left stick pitches and rolls, the right stick's X
//! and Q and E spin, and the mouse is a stick of its own; none of them is a new key that something else uses.

use bevy_input::{gamepad::GamepadAxis, keyboard::KeyCode, mouse::MouseButton};

use super::action::Action;
use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

fn axes(snapshot: &Snapshot) -> [f32; 3] {
    let mut state = ActionState::default();
    state.update(&Bindings::default(), snapshot, false);
    [state.value(Action::AirPitch), state.value(Action::AirRoll), state.value(Action::AirYaw)]
}

fn keys(held: &[KeyCode]) -> [f32; 3] {
    let mut snapshot = Snapshot::default();
    snapshot.keys.extend(held.iter().copied());
    axes(&snapshot)
}

#[test]
fn the_driving_keys_are_the_stick_and_q_and_e_spin() {
    assert_eq!(keys(&[]), [0.0; 3]);
    assert_eq!(keys(&[KeyCode::KeyW]), [1.0, 0.0, 0.0], "forward is the nose down");
    assert_eq!(keys(&[KeyCode::KeyS]), [-1.0, 0.0, 0.0]);
    assert_eq!(keys(&[KeyCode::ArrowUp]), [1.0, 0.0, 0.0]);
    assert_eq!(keys(&[KeyCode::ArrowDown]), [-1.0, 0.0, 0.0]);
    assert_eq!(keys(&[KeyCode::KeyD]), [0.0, 1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::KeyA]), [0.0, -1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::ArrowRight]), [0.0, 1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::ArrowLeft]), [0.0, -1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::KeyE]), [0.0, 0.0, 1.0]);
    assert_eq!(keys(&[KeyCode::KeyQ]), [0.0, 0.0, -1.0]);
    assert_eq!(keys(&[KeyCode::KeyW, KeyCode::KeyS]), [0.0; 3], "both cancel");
    assert_eq!(keys(&[KeyCode::KeyW, KeyCode::KeyD]), [1.0, 1.0, 0.0], "diagonals ask for both axes");
}

#[test]
fn the_left_stick_pitches_and_rolls_and_the_right_stick_spins() {
    let stick = |left: (f32, f32), right_x: f32| {
        let mut snapshot = Snapshot::default();
        snapshot.pad_axes.insert(GamepadAxis::LeftStickX, left.0);
        snapshot.pad_axes.insert(GamepadAxis::LeftStickY, left.1);
        snapshot.pad_axes.insert(GamepadAxis::RightStickX, right_x);
        axes(&snapshot)
    };
    assert_eq!(stick((0.0, 0.0), 0.0), [0.0; 3]);
    let forward = stick((0.0, 1.0), 0.0);
    assert!(forward[0] > 0.9 && forward[1] == 0.0 && forward[2] == 0.0, "{forward:?}");
    let right = stick((1.0, 0.0), 0.0);
    assert!(right[1] > 0.9 && right[0] == 0.0, "{right:?}");
    let back_left = stick((-1.0, -1.0), 0.0);
    assert!(back_left[0] < -0.9 && back_left[1] < -0.9);
    let spin = stick((0.0, 0.0), 1.0);
    assert!(spin[2] > 0.9 && spin[0] == 0.0 && spin[1] == 0.0, "{spin:?}");
}

#[test]
fn the_mouse_is_a_stick_that_pitches_and_spins_while_the_cursor_is_captured() {
    let moved = |delta: (f32, f32), captured: bool| {
        let snapshot = Snapshot { mouse_delta: delta, mouse_captured: captured, dt: 1.0 / 60.0, ..Snapshot::default() };
        axes(&snapshot)
    };
    assert_eq!(moved((30.0, 0.0), false), [0.0; 3], "a free cursor is no stick");
    let right = moved((30.0, 0.0), true);
    assert!(right[2] > 0.0 && right[0] == 0.0, "{right:?}");
    let left = moved((-30.0, 0.0), true);
    assert!(left[2] < 0.0);
    let up = moved((0.0, -30.0), true);
    assert!(up[0] > 0.0 && up[2] == 0.0, "the mouse forward pushes the nose down like a stick forward: {up:?}");
    let down = moved((0.0, 30.0), true);
    assert!(down[0] < 0.0);
    let mut held = Snapshot { mouse_delta: (30.0, 0.0), dt: 1.0 / 60.0, ..Snapshot::default() };
    held.buttons.insert(MouseButton::Right);
    assert!(axes(&held)[2] > 0.0, "the right button is the other way to look");
    for axis in moved((10_000.0, -10_000.0), true) {
        assert!(axis.abs() <= 1.0, "a stick never reads past its end");
    }
}

#[test]
fn the_actions_are_named_and_in_the_list() {
    for (name, action) in [("air_pitch", Action::AirPitch), ("air_roll", Action::AirRoll), ("air_yaw", Action::AirYaw)]
    {
        assert_eq!(Action::parse(name), Ok(action));
        assert_eq!(action.name(), name);
        assert!(Action::ALL.contains(&action));
    }
}
