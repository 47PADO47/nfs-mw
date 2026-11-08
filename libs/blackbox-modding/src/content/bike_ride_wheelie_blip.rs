//! The wheelie held with the throttle alone (`docs/specs/ride6-wheelie-hold.md`), on the imported bikes of a folder
//! (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b] [RIDE6_START=20] [RIDE6_TARGET=37]`): per bike, a full-throttle launch from a standstill with the weight lever
//! released and with it back (the time to 10 degrees of pitch), and a wheelie at the start speed with the lever back, a throttle burst and then
//! a bang-bang throttle (on below the target pitch, off above) for 8 s: whether the pitch stays between 25 and 50 degrees for the last 5 s, whether the
//! bike fell, and the pitch range.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::specs;

fn env_number(name: &str, default: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// A launch with full throttle: the time the pitch first passes 10 degrees (`None` if it never does in 4 s) and whether the bike fell.
fn launch(spec: &BikeSpec, lever: f32) -> (Option<f32>, bool) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    bike.set_weight(lever);
    for step in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
        if bike.fallen() {
            return (None, true);
        }
        if bike.pitch() > 10.0_f32.to_radians() {
            return (Some(step as f32 * FIXED_STEP), false);
        }
    }
    (None, false)
}

/// The held wheelie: `(lowest, highest pitch over the last 5 s in degrees, whether it stayed in 25..50, whether it fell)`.
fn hold(spec: &BikeSpec, start: f32, target: f32) -> (f32, f32, bool, bool) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
    bike.set_weight(-1.0);
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    for step in 0..(8.0 / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let throttle = f32::from(t < 0.6 || bike.pitch() < target.to_radians());
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
        if bike.fallen() {
            return (low.to_degrees(), high.to_degrees(), false, true);
        }
        if t >= 3.0 {
            low = low.min(bike.pitch());
            high = high.max(bike.pitch());
        }
    }
    let kept = low > 25.0_f32.to_radians() && high < 50.0_f32.to_radians();
    (low.to_degrees(), high.to_degrees(), kept, false)
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_wheelie_blip() {
    let (start, target) = (env_number("RIDE6_START", 20.0), env_number("RIDE6_TARGET", 37.0));
    let (mut total, mut lifts_free, mut lifts_lever, mut holds, mut falls) = (0, 0, 0, 0, 0);
    for (name, spec) in specs() {
        total += 1;
        let (free, free_fell) = launch(&spec, 0.0);
        let (lever, lever_fell) = launch(&spec, -1.0);
        let (low, high, kept, fell) = hold(&spec, start, target);
        lifts_free += usize::from(free.is_some_and(|t| t < 1.5));
        lifts_lever += usize::from(lever.is_some_and(|t| t < 1.5));
        holds += usize::from(kept);
        falls += usize::from(fell || free_fell || lever_fell);
        println!(
            "{name}: launch to 10 deg lever 0 {free:?} lever back {lever:?}; held at {start} m/s: pitch {low:.0}..{high:.0} {}{}",
            if kept { "HELD" } else { "lost" },
            if fell { " FELL" } else { "" }
        );
    }
    println!(
        "{total} bikes: lift in 1.5 s with the lever released {lifts_free}, with it back {lifts_lever}; hold 5 s {holds}; fell {falls}"
    );
}
