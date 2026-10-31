//! The look of a bike's helmet camera: the pad's right stick X, dead-zoned and scaled as the other camera looks are. Its Y stays the
//! gear shift of the driving profile, so it is not a look.

use bevy_input::gamepad::GamepadAxis;

use super::action::Action;
use super::bindings::Source;
use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

fn look_of_stick(x: f32, y: f32) -> (f32, f32, bool, bool) {
    let mut state = ActionState::default();
    let mut snap = Snapshot::default();
    snap.pad_axes.insert(GamepadAxis::RightStickX, x);
    snap.pad_axes.insert(GamepadAxis::RightStickY, y);
    state.update(&Bindings::default(), &snap, false);
    (
        state.value(Action::HeadLookX),
        state.value(Action::HeadLookY),
        state.value(Action::ShiftUp) > 0.5,
        state.value(Action::ShiftDown) > 0.5,
    )
}

#[test]
fn the_right_sticks_x_looks_and_its_y_still_shifts_gear() {
    let bindings = Bindings::default();
    let sources = |action: Action| -> Vec<Source> {
        bindings.0.iter().filter(|b| b.action == action).map(|b| b.source).collect()
    };
    assert_eq!(sources(Action::HeadLookX), vec![Source::PadAxis(GamepadAxis::RightStickX)]);
    assert!(sources(Action::HeadLookY).is_empty(), "the stick's Y is the gear shift on a pad");
    assert!(sources(Action::ShiftUp).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    assert!(sources(Action::ShiftDown).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    // Pushed up or down the stick shifts and does not look; pushed sideways it looks and does not shift.
    assert_eq!(look_of_stick(0.0, 1.0), (0.0, 0.0, true, false));
    assert_eq!(look_of_stick(0.0, -1.0), (0.0, 0.0, false, true));
    assert_eq!(look_of_stick(1.0, 0.0), (1.0, 0.0, false, false));
    assert_eq!(look_of_stick(-1.0, 0.0), (-1.0, 0.0, false, false));
}

#[test]
fn the_camera_dead_zone_keeps_a_resting_stick_from_turning_the_head_and_the_rim_reaches_the_limit() {
    assert_eq!(look_of_stick(0.0, 0.0).0, 0.0);
    assert_eq!(look_of_stick(0.1, 0.0).0, 0.0, "inside the dead zone (15 percent by default)");
    assert_eq!(look_of_stick(-0.14, 0.0).0, 0.0);
    let (x, ..) = look_of_stick(0.575, 0.0);
    assert!(x > 0.4 && x < 0.6, "rescaled past the dead zone: {x}");
    assert!((look_of_stick(1.0, 0.0).0 - 1.0).abs() < 1e-6);
    assert_eq!(look_of_stick(f32::NAN, 0.0).0, 0.0);
}

#[test]
fn the_actions_round_trip_by_name() {
    assert_eq!(Action::parse("head_look_x"), Ok(Action::HeadLookX));
    assert_eq!(Action::parse("head_look_y"), Ok(Action::HeadLookY));
    assert_eq!(Action::HeadLookX.name(), "head_look_x");
    assert!(Action::ALL.contains(&Action::HeadLookY));
}
