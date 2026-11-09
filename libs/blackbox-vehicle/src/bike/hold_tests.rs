use crate::FIXED_STEP;
use crate::bike::{Bike, BikeInput, BikeSpec, ReverseSpec, ReverseState};
use crate::ground::{FlatGround, Ground, SlopedGround, SurfaceGrip};

/// A road rising `degrees` in the bike's heading (negative: falling), on which the bike is placed at rest at the origin facing +z.
fn road(degrees: f32) -> SlopedGround {
    SlopedGround { height: 0.0, slope: degrees.to_radians().tan(), surface: SurfaceGrip::DEFAULT }
}

fn spec() -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.reverse = Some(ReverseSpec { force: 1500.0, max_speed: 2.5, hold_time: 0.5 });
    spec
}

/// The bike at rest on `ground`, with the brakes off.
fn parked(ground: &dyn Ground) -> Bike {
    let mut bike = Bike::new(spec());
    assert!(bike.place_on_ground(ground, 0.0, 0.0, 2.0, 0.0));
    bike
}

/// Step `seconds` with `input` and return the most the bike was away from where it was at the start along z and its last speed along z.
fn ride(bike: &mut Bike, ground: &dyn Ground, input: BikeInput, seconds: f32) -> (f32, f32) {
    let start = bike.position().z;
    let mut farthest = 0.0_f32;
    for _ in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, ground);
        assert!(!bike.fallen(), "the bike fell");
        farthest = farthest.max((bike.position().z - start).abs());
    }
    (farthest, bike.velocity().z)
}

#[test]
fn a_bike_with_no_throttle_stays_still_on_a_10_and_a_20_degree_slope_either_way() {
    for degrees in [10.0_f32, 20.0, -10.0, -20.0] {
        let ground = road(degrees);
        let mut bike = parked(&ground);
        let (farthest, speed) = ride(&mut bike, &ground, BikeInput::default(), 6.0);
        assert!(farthest < 0.35, "{degrees} degrees: moved {farthest} m");
        assert!(speed.abs() < 0.3, "{degrees} degrees: still at {speed} m/s");
        assert!(bike.held_on_slope(), "{degrees} degrees: not held");
    }
}

#[test]
fn a_bike_on_a_slope_does_not_roll_back_fast_enough_for_the_reverse_walk() {
    // The rider's reverse walk starts at -1 km/h (-0.28 m/s): a bike held on a slope never gets there.
    let ground = road(15.0);
    let mut bike = parked(&ground);
    let mut slowest = 0.0_f32;
    for _ in 0..(6.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput::default(), &ground);
        slowest = slowest.min(bike.forward_speed());
    }
    assert!(slowest > -0.28, "rolled back at {slowest} m/s");
}

#[test]
fn a_flat_road_is_not_held_and_the_hold_has_no_effect_on_it() {
    let ground = FlatGround::new(0.0);
    let mut bike = parked(&ground);
    let (farthest, _) = ride(&mut bike, &ground, BikeInput::default(), 2.0);
    assert!(!bike.held_on_slope());
    assert!(farthest < 0.05, "{farthest}");
}

#[test]
fn a_slope_past_the_limit_is_not_held() {
    let ground = road(40.0);
    let mut bike = parked(&ground);
    for _ in 0..(1.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    assert!(!bike.held_on_slope(), "a rider cannot hold a bike on 40 degrees");
}

#[test]
fn the_throttle_lets_the_hold_go() {
    let ground = road(10.0);
    let mut bike = parked(&ground);
    ride(&mut bike, &ground, BikeInput::default(), 1.0);
    assert!(bike.held_on_slope());
    let (farthest, speed) = ride(&mut bike, &ground, BikeInput { throttle: 0.7, ..Default::default() }, 3.0);
    assert!(!bike.held_on_slope());
    assert!(farthest > 2.0 && speed > 1.0, "moved {farthest} m, at {speed} m/s");
}

#[test]
fn a_bike_that_rolls_is_not_held() {
    let ground = road(10.0);
    let mut bike = Bike::new(spec());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 5.0));
    for _ in 0..5 {
        bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    assert!(!bike.held_on_slope());
}

#[test]
fn a_brake_held_through_the_stop_on_a_slope_does_not_walk_the_bike_back_but_a_fresh_pull_does() {
    let ground = road(10.0);
    let mut bike = Bike::new(spec());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 3.0));
    let brake = BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() };
    // Braked to a stop and held for 4 s: the rider holds the bike.
    let mut reversed = false;
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &brake, &ground);
        reversed |= bike.reversing();
    }
    assert!(!reversed, "the held brake walked the bike back");
    assert!(bike.velocity().z.abs() < 0.3, "{}", bike.velocity().z);
    let stopped_at = bike.position().z;
    // Let go, wait, pull again on the standing bike: that is the reverse.
    ride(&mut bike, &ground, BikeInput::default(), 0.8);
    let mut walked = false;
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &brake, &ground);
        walked |= bike.reversing();
    }
    assert!(walked, "a fresh pull of the lever did not ask for the reverse");
    assert!(bike.position().z < stopped_at - 0.3, "the bike did not go back: {} from {stopped_at}", bike.position().z);
}

#[test]
fn on_flat_ground_the_held_brake_still_walks_the_bike_back() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 3.0));
    let brake = BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() };
    let mut walked = false;
    for _ in 0..(5.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &brake, &ground);
        walked |= bike.reversing();
    }
    assert!(walked, "the reverse of the flat road changed");
}

#[test]
fn the_reverse_state_ignores_a_lever_carried_through_the_stop_only_on_a_slope() {
    let reverse = ReverseSpec { force: 1000.0, max_speed: 2.5, hold_time: 0.5 };
    let lever = BikeInput { front_brake: 1.0, ..Default::default() };
    for (on_slope, asks) in [(true, false), (false, true)] {
        let mut state = ReverseState::default();
        // The lever is pulled while the bike rolls at 3 m/s and held on as it stops.
        state.step_on(&reverse, 0.01, 3.0, &lever, on_slope);
        let mut drive = Default::default();
        for _ in 0..80 {
            drive = state.step_on(&reverse, 0.01, 0.0, &lever, on_slope);
        }
        assert_eq!(state.pushing(), asks, "on_slope {on_slope}: {drive:?}");
        // Let go for a step and pull again: that asks, on a slope too.
        state.step_on(&reverse, 0.01, 0.0, &BikeInput::default(), on_slope);
        for _ in 0..80 {
            drive = state.step_on(&reverse, 0.01, 0.0, &lever, on_slope);
        }
        assert!(state.pushing(), "on_slope {on_slope}: a fresh pull asks: {drive:?}");
    }
}
