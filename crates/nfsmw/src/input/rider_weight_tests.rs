//! The rider's weight of a bike (wheelies and stoppies): G and V (or Numpad 8 and 2) and the pad's left stick Y, which
//! steers with X at the same time; J and the left stick's click cycle the ABS level. A car reads none of it.

use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
};

use super::action::Action;
use super::bindings::Source;
use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

fn weight_with(keys: &[KeyCode]) -> f32 {
    let mut state = ActionState::default();
    state.update_for_test(&Bindings::default(), keys, &[], false);
    state.value(Action::RiderWeight)
}

#[test]
fn g_leans_the_rider_forward_and_v_back() {
    assert_eq!(weight_with(&[]), 0.0);
    assert_eq!(weight_with(&[KeyCode::KeyG]), 1.0);
    assert_eq!(weight_with(&[KeyCode::KeyV]), -1.0);
    assert_eq!(weight_with(&[KeyCode::Numpad8]), 1.0);
    assert_eq!(weight_with(&[KeyCode::Numpad2]), -1.0);
    assert_eq!(weight_with(&[KeyCode::KeyG, KeyCode::KeyV]), 0.0, "both cancel");
}

fn weight_of_stick(x: f32, y: f32) -> f32 {
    let mut state = ActionState::default();
    let mut snap = Snapshot::default();
    snap.pad_axes.insert(GamepadAxis::LeftStickX, x);
    snap.pad_axes.insert(GamepadAxis::LeftStickY, y);
    state.update(&Bindings::default(), &snap, false);
    state.value(Action::RiderWeight)
}

fn steer_of_stick(x: f32, y: f32) -> f32 {
    let mut state = ActionState::default();
    let mut snap = Snapshot::default();
    snap.pad_axes.insert(GamepadAxis::LeftStickX, x);
    snap.pad_axes.insert(GamepadAxis::LeftStickY, y);
    state.update(&Bindings::default(), &snap, false);
    state.value(Action::Steer)
}

#[test]
fn the_pads_left_stick_steers_with_x_and_moves_the_weight_with_y() {
    let bindings = Bindings::default();
    let sources = |action: Action| -> Vec<Source> {
        bindings.0.iter().filter(|b| b.action == action).map(|b| b.source).collect()
    };
    assert_eq!(
        sources(Action::RiderWeight).iter().filter(|s| matches!(s, Source::PadAxis(_))).collect::<Vec<_>>(),
        vec![&Source::PadAxis(GamepadAxis::LeftStickY)]
    );
    // The right stick keeps the car's gear shifts and is not a weight input.
    assert!(sources(Action::ShiftUp).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    assert!(!sources(Action::RiderWeight).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    // Up is forward (a stoppie), down is back (a wheelie); full deflection reaches the ends.
    assert_eq!(weight_of_stick(0.0, 1.0), 1.0);
    assert_eq!(weight_of_stick(0.0, -1.0), -1.0);
    assert_eq!(weight_of_stick(0.0, 0.0), 0.0);
    assert!((steer_of_stick(1.0, 0.0) - 1.0).abs() < 1e-6, "steering is as before");
}

#[test]
fn a_stick_at_the_rim_on_the_diagonal_still_gives_the_full_weight_and_steers_at_once() {
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    // Pulled back and to the right: the wheelie lever at its end, and the steering about 0.7.
    assert!((weight_of_stick(diagonal, -diagonal) + 1.0).abs() < 1e-5, "{}", weight_of_stick(diagonal, -diagonal));
    let steer = steer_of_stick(diagonal, -diagonal);
    assert!(steer > 0.5 && steer < 0.8, "{steer}");
    assert!((weight_of_stick(-diagonal, diagonal) - 1.0).abs() < 1e-5);
    // A gentler push keeps its proportion and passes the dead zones' thresholds of the help (0.3 and 0.95) as it should.
    let half = weight_of_stick(0.0, -0.5);
    assert!(half < -0.4 && half > -0.5, "{half}");
    // A stick that is off the rim does not jump to the end.
    assert!(weight_of_stick(0.2, -0.2).abs() < 0.5);
    // Not finite: nothing.
    assert_eq!(weight_of_stick(0.0, f32::NAN), 0.0);
}

#[test]
fn j_and_the_left_stick_click_cycle_the_abs_level_and_no_other_driving_action_shares_them() {
    let bindings = Bindings::default();
    let on = |source: Source| -> Vec<Action> {
        bindings.0.iter().filter(|b| b.source == source).map(|b| b.action).collect()
    };
    assert_eq!(on(Source::Key(KeyCode::KeyJ)), vec![Action::AbsLevel]);
    let click = on(Source::PadButton(GamepadButton::LeftThumb));
    assert!(click.contains(&Action::AbsLevel));
    // The only other user is the free camera's boost, which is not a driving input.
    assert!(click.iter().all(|a| matches!(a, Action::AbsLevel | Action::Boost)), "{click:?}");
    assert_eq!(Action::parse("abs_level"), Ok(Action::AbsLevel));
}

#[test]
fn the_weight_axis_round_trips_by_name_and_no_key_is_shared_with_another_driving_action() {
    assert_eq!(Action::parse("rider_weight"), Ok(Action::RiderWeight));
    assert_eq!(Action::RiderWeight.name(), "rider_weight");
    assert!(Action::ALL.contains(&Action::RiderWeight));
    let bindings = Bindings::default();
    for key in [KeyCode::KeyG, KeyCode::KeyV, KeyCode::Numpad8, KeyCode::Numpad2] {
        let actions: Vec<Action> =
            bindings.0.iter().filter(|b| b.source == Source::Key(key)).map(|b| b.action).collect();
        assert_eq!(actions, vec![Action::RiderWeight], "{key:?}");
    }
}
