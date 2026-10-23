//! A network seat's input reaches the car exactly: every wire code (`nfsmw_net::input::InputCode`) becomes the car's
//! input and goes back to the same code, the edges and buttons included.

use blackbox_vehicle::InputState;
use nfsmw_net::input::{CLUTCH_MAX, HANDBRAKE_MAX, InputCode, PEDAL_MAX, STEER_MAX};

use crate::scenes::world::drive::DriveInput;

/// The wire code of `input` after it went through the car's input: decoded, taken by the car, handed to the vehicle.
fn through_the_car(code: InputCode) -> InputCode {
    let car = DriveInput::from_vehicle(&code.to_input());
    InputCode::quantise(&car.to_vehicle())
}

#[test]
fn every_steering_code_survives_the_car() {
    for steer in 0..=STEER_MAX {
        let code = InputCode { steer, ..InputCode::NEUTRAL };
        assert_eq!(through_the_car(code), code, "steer code {steer}");
        let car = DriveInput::from_vehicle(&code.to_input());
        assert_eq!(car.steer, code.to_input().steer, "the steering value itself, not just its code");
    }
}

#[test]
fn every_pedal_and_clutch_code_survives_the_car() {
    for pedal in 0..=PEDAL_MAX {
        let code = InputCode { throttle: pedal, brake: PEDAL_MAX - pedal, ..InputCode::NEUTRAL };
        assert_eq!(through_the_car(code), code, "pedal code {pedal}");
    }
    for clutch in 0..=CLUTCH_MAX {
        let code = InputCode { clutch, ..InputCode::NEUTRAL };
        assert_eq!(through_the_car(code), code, "clutch code {clutch}");
    }
}

#[test]
fn the_handbrake_is_on_from_half_travel() {
    for handbrake in 0..=HANDBRAKE_MAX {
        let on = handbrake >= HANDBRAKE_MAX.div_ceil(2);
        let back = through_the_car(InputCode { handbrake, ..InputCode::NEUTRAL }).handbrake;
        let expected = match on {
            true => HANDBRAKE_MAX,
            false => 0,
        };
        assert_eq!(back, expected, "handbrake code {handbrake}");
    }
}

#[test]
fn buttons_edges_and_gears_survive_the_car() {
    let codes = [
        InputCode { nos: true, ..InputCode::NEUTRAL },
        InputCode { shift_up: true, ..InputCode::NEUTRAL },
        InputCode { shift_down: true, ..InputCode::NEUTRAL },
        InputCode { gear_select: Some(0), ..InputCode::NEUTRAL },
        InputCode { gear_select: Some(15), ..InputCode::NEUTRAL },
        InputCode { throttle: 90, steer: 17, handbrake: 15, nos: true, shift_up: true, ..InputCode::NEUTRAL },
    ];
    for code in codes {
        assert_eq!(through_the_car(code), code);
    }
}

#[test]
fn a_shift_edge_drives_one_step_only() {
    // The server sets each tick's input before the tick; the next tick's input has no edge, so the car sees it once.
    let edge = DriveInput::from_vehicle(&InputState { shift_up: true, ..InputState::default() });
    let next = DriveInput::from_vehicle(&InputState::default());
    assert!(edge.shift_up && !next.shift_up);
}

#[test]
fn hostile_input_never_leaves_the_car_out_of_range() {
    let values = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, -0.0, 1.0e-30, 2.0, -2.0];
    for &x in &values {
        let wild = InputState { throttle: x, brake: x, steer: x, handbrake: x, clutch: x, ..InputState::default() };
        let car = DriveInput::from_vehicle(&wild);
        for (value, min) in [(car.throttle, 0.0), (car.brake, 0.0), (car.steer, -1.0), (car.clutch, 0.0)] {
            assert!(value.is_finite() && (min..=1.0).contains(&value), "{x} gave {value}");
        }
        assert!(InputCode::quantise(&car.to_vehicle()).is_valid());
    }
}
