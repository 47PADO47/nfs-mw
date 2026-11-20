//! A wheelie held with the throttle alone (`docs/specs/ride6-wheelie-hold.md`): with the weight lever back, a burst of throttle lifts the
//! front wheel, a bang-bang throttle (on below the target pitch, off above it) holds it, a throttle that is shut lowers the front gently,
//! and a blip raises it again. Ridden on the example bike (a 450 cm3 supermoto, the data of the KTM 450 SMR) and on a superbike of its
//! own (a heavier frame, three times the torque, a gearing for 90 m/s).

use super::{Bike, BikeInput, BikeSpec, FRONT, choices};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

fn supermoto() -> BikeSpec {
    BikeSpec::example()
}

fn superbike() -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.chassis.mass = 195.0;
    spec.engine.peak_torque *= 3.0;
    spec.transmission.final_ratio = 5.0;
    spec
}

fn bike_at(spec: &BikeSpec, speed: f32) -> (Bike, FlatGround) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    bike.set_weight(-1.0);
    (bike, ground)
}

/// What a bang-bang rider's wheelie did: the lowest and highest pitch (rad) over the last 5 s of 9, whether the front wheel was up in all of
/// them, and whether the bike fell.
struct Held {
    low: f32,
    high: f32,
    up: bool,
    fell: bool,
}

/// A burst of throttle for 0.6 s from `speed`, then the throttle on below `target` (rad) and off above it, for 9 s.
fn hold(spec: &BikeSpec, speed: f32, target: f32) -> Held {
    let (mut bike, ground) = bike_at(spec, speed);
    let mut held = Held { low: f32::MAX, high: f32::MIN, up: true, fell: false };
    for step in 0..(9.0 / FIXED_STEP) as usize {
        let time = step as f32 * FIXED_STEP;
        let throttle = f32::from(time < 0.6 || bike.pitch() < target);
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
        held.fell |= bike.fallen();
        if time >= 4.0 {
            held.low = held.low.min(bike.pitch());
            held.high = held.high.max(bike.pitch());
            held.up &= !bike.wheel(FRONT).on_ground;
        }
    }
    held
}

#[test]
fn a_bang_bang_throttle_holds_a_wheelie_of_30_to_45_degrees_for_5_seconds_on_a_supermoto_and_a_superbike() {
    let target = 38.0_f32.to_radians();
    for (name, spec) in [("supermoto", supermoto()), ("superbike", superbike())] {
        for speed in [10.0_f32, 20.0, 30.0] {
            let held = hold(&spec, speed, target);
            let (low, high) = (held.low.to_degrees(), held.high.to_degrees());
            assert!(!held.fell, "{name} at {speed} m/s fell");
            // Down to 28 degrees: a gear change cuts the thrust for a moment.
            assert!(
                held.up && low > 28.0 && high < 46.0,
                "{name} at {speed} m/s: pitch {low:.1} to {high:.1}, up {}",
                held.up
            );
        }
    }
}

#[test]
fn a_supermoto_launched_in_first_gear_at_full_throttle_with_the_weight_back_lifts_its_front_wheel_within_1_5_s() {
    let (mut bike, ground) = bike_at(&supermoto(), 0.0);
    let mut lifted = None;
    for step in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
        if lifted.is_none() && bike.pitch() > 10.0_f32.to_radians() {
            lifted = Some((step + 1) as f32 * FIXED_STEP);
            assert_eq!(bike.gear(), 1, "still in first gear");
            assert!(!bike.wheel(FRONT).on_ground);
        }
    }
    let time = lifted.expect("10 degrees of pitch");
    assert!(time < 1.5 && !bike.fallen(), "10 degrees after {time} s");
}

/// A wheelie held at full throttle from `speed` for 4 s, then the throttle shut: the fastest the nose went down (rad/s), the rate at which the
/// front wheel touched the road, the seconds from the throttle shut to the touch, and whether the bike fell.
fn let_down(spec: &BikeSpec, speed: f32) -> (f32, f32, f32, bool) {
    let (mut bike, ground) = bike_at(spec, speed);
    let (mut fastest, mut touch_rate, mut touch_time) = (0.0_f32, f32::MAX, f32::MAX);
    let mut last = 0.0_f32;
    for step in 0..(8.0 / FIXED_STEP) as usize {
        let time = step as f32 * FIXED_STEP;
        let throttle = f32::from(time < 4.0);
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
        let rate = (last - bike.pitch()) / FIXED_STEP;
        last = bike.pitch();
        if time < 4.0 || touch_time < f32::MAX {
            continue;
        }
        fastest = fastest.max(rate);
        if bike.wheel(FRONT).on_ground {
            (touch_rate, touch_time) = (rate, time - 4.0);
        }
    }
    (fastest, touch_rate, touch_time, bike.fallen())
}

#[test]
fn a_throttle_that_is_shut_lowers_the_front_wheel_gently_and_it_touches_the_road_slowly() {
    for (name, spec) in [("supermoto", supermoto()), ("superbike", superbike())] {
        let (fastest, touch_rate, touch_time, fell) = let_down(&spec, 20.0);
        assert!(!fell, "{name} fell");
        // Without the lowering torque the nose drops at 110 to 125 degrees a second from 35 and lands at 20 or more (here at most 45 and 15).
        assert!(
            fastest < 55.0_f32.to_radians(),
            "{name}: the nose went down at {} degrees a second",
            fastest.to_degrees()
        );
        assert!(
            touch_rate < 20.0_f32.to_radians(),
            "{name}: the front wheel touched at {} degrees a second",
            touch_rate.to_degrees()
        );
        assert!(touch_time > 0.5 && touch_time < 2.0, "{name}: down after {touch_time} s");
    }
}

#[test]
fn a_blip_of_throttle_raises_the_front_wheel_again_after_it_has_been_let_down() {
    for (name, spec) in [("supermoto", supermoto()), ("superbike", superbike())] {
        let (mut bike, ground) = bike_at(&spec, 20.0);
        // Down for 1 s, a blip of 0.6 s, down for 2 s, the same again.
        let script = [(1.0, 0.0), (0.6, 1.0), (2.0, 0.0), (0.6, 1.0), (2.0, 0.0)];
        let mut peaks = Vec::new();
        for (seconds, throttle) in script {
            let mut peak = f32::MIN;
            for _ in 0..(seconds / FIXED_STEP) as usize {
                bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
                peak = peak.max(bike.pitch());
            }
            if throttle > 0.0 {
                peaks.push(peak);
            }
        }
        assert!(!bike.fallen() && bike.wheel(FRONT).on_ground, "{name}: down and on its wheels at the end");
        for peak in peaks {
            assert!(
                peak > 6.0_f32.to_radians() && peak < 40.0_f32.to_radians(),
                "{name}: a blip raised the nose to {} degrees",
                peak.to_degrees()
            );
        }
    }
}

#[test]
fn the_throttle_held_with_the_weight_back_never_loops_the_bike_out() {
    for (name, spec) in [("supermoto", supermoto()), ("superbike", superbike())] {
        for speed in [0.0_f32, 10.0, 25.0] {
            let (mut bike, ground) = bike_at(&spec, speed);
            let fall = bike.spec().fall.pitch;
            let mut worst = f32::MIN;
            for _ in 0..(10.0 / FIXED_STEP) as usize {
                bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
                worst = worst.max(bike.pitch());
            }
            assert!(
                !bike.fallen() && worst < fall - 0.1,
                "{name} from {speed} m/s: pitch {worst} against the fall limit {fall}"
            );
        }
    }
}

#[test]
fn the_bike_is_a_sixth_lighter_than_the_data_and_the_rider_is_not() {
    let (bike, _) = bike_at(&supermoto(), 0.0);
    let data = supermoto().chassis;
    assert!(
        (bike.spec().chassis.mass - choices::BIKE_MASS_SCALE * data.mass).abs() < 1e-4
            && choices::BIKE_MASS_SCALE < 1.0
    );
    assert_eq!(bike.spec().chassis.rider_mass, data.rider_mass);
}
