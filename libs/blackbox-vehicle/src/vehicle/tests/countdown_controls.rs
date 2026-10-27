use super::*;
use crate::LaunchConfig;
use crate::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};

fn staged(automatic: bool) -> Vehicle {
    let mut car = parked();
    car.config.automatic = automatic;
    car.set_launch_config(Some(LaunchConfig {
        band_fraction: 0.25,
        redline_margin: 500.0,
        duration: 4.5,
        speed_cutoff: 26.8218,
        traction_gain: 0.25,
        staging_traction_scale: 0.25,
        cancel_on_shift: false,
    }));
    car.set_staging(true);
    car
}

#[test]
fn countdown_blocks_button_shifts_above_first_but_releases_at_go() {
    for automatic in [false, true] {
        let mut car = staged(automatic);
        let up = InputState { throttle: 0.7, brake: 0.6, shift_up: true, ..Default::default() };
        car.step(FIXED_STEP, &up, &flat());
        assert_eq!(car.gear(), GEAR_FIRST);
        assert_eq!((car.controls.gas, car.controls.brake), (0.7, 0.6));
        car.set_staging(false);
        if !automatic {
            car.step(FIXED_STEP, &up, &flat());
            assert_eq!(car.gear(), GEAR_FIRST + 1);
        }
    }
}

#[test]
fn countdown_keeps_neutral_reverse_and_pedal_priority() {
    let mut car = staged(false);
    car.step(FIXED_STEP, &InputState { shift_down: true, ..Default::default() }, &flat());
    assert_eq!(car.gear(), GEAR_NEUTRAL);
    car.step(FIXED_STEP, &InputState { throttle: 0.7, brake: 0.6, shift_up: true, ..Default::default() }, &flat());
    assert_eq!(car.gear(), GEAR_FIRST);
    assert_eq!((car.controls.gas, car.controls.brake), (0.7, 0.6));
    car.step(FIXED_STEP, &InputState { brake: 1.0, shift_down: true, ..Default::default() }, &flat());
    assert_eq!(car.gear(), GEAR_REVERSE);
    assert_eq!((car.controls.gas, car.controls.brake), (1.0, 0.0));
    // The button is ignored in reverse; the gas then selects first, not second.
    car.step(FIXED_STEP, &InputState { throttle: 1.0, shift_up: true, ..Default::default() }, &flat());
    assert_eq!(car.gear(), GEAR_FIRST);
    assert_eq!((car.controls.gas, car.controls.brake), (1.0, 0.0));
}

#[test]
fn shift_is_processed_before_reverse_pedal_swap_at_reverse_exit() {
    let mut car = staged(false);
    car.powertrain.shift(GEAR_REVERSE);
    car.step(FIXED_STEP, &InputState { throttle: 0.7, brake: 0.6, shift_up: true, ..Default::default() }, &flat());
    assert_eq!(car.gear(), GEAR_FIRST);
    assert_eq!((car.controls.gas, car.controls.brake), (0.7, 0.6));
    car.step(FIXED_STEP, &InputState { brake: 0.6, handbrake: 1.0, ..Default::default() }, &flat());
    assert_eq!(car.gear(), GEAR_REVERSE);
    assert_eq!((car.controls.gas, car.controls.brake, car.controls.handbrake), (0.6, 0.0, 1.0));
}
