//! The arcade help of braking on the example bike (`docs/specs/ride6-bike-braking.md`): the deceleration a full lever gives with and without it, the
//! stop in metres, the rear wheel that stays down, the stoppie a rider asks for, and what the help leaves alone (the tyres' slip, a bike that
//! stands, the throttle).

use super::*;
use crate::bike::{ArcadeBrake, ArcadeSteering, BikeSpec};
use crate::{FIXED_STEP, FlatGround};

const G: f32 = 9.81;

/// The example bike as a road bike (a short fork) with the given braking help.
fn road(brake: ArcadeBrake) -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.front_suspension.max_travel = 0.12;
    spec.steering.arcade = ArcadeSteering { brake, ..ArcadeSteering::default() };
    spec
}

/// What a stop did.
struct Stop {
    fell: bool,
    /// The mean deceleration (g) from the lever to a stop, the distance (m), the seconds the rear wheel was off the road, the lowest pitch (rad) and the
    /// sum of the tyres' braking force over the first second (N s).
    mean: f32,
    distance: f32,
    rear_up: f32,
    nose_down: f32,
    tyre_impulse: f32,
}

/// Both levers fully pulled at `speed` m/s with the rider's weight at `weight`, until the bike stands.
fn stop(spec: &BikeSpec, speed: f32, weight: f32, throttle: f32) -> Stop {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    bike.set_weight(weight);
    let mut out = Stop { fell: false, mean: 0.0, distance: 0.0, rear_up: 0.0, nose_down: 0.0, tyre_impulse: 0.0 };
    let mut time = 0.0;
    for step in 0..(40.0 / FIXED_STEP) as usize {
        let input = BikeInput { throttle, front_brake: 1.0, rear_brake: 1.0, steer: 0.0 };
        bike.step(FIXED_STEP, &input, &ground);
        if bike.fallen() {
            out.fell = true;
            break;
        }
        let now = bike.velocity().length();
        out.distance += now * FIXED_STEP;
        time += FIXED_STEP;
        if !bike.wheel(REAR).on_ground {
            out.rear_up += FIXED_STEP;
        }
        out.nose_down = out.nose_down.min(bike.pitch());
        if time < 1.0 {
            out.tyre_impulse -= (bike.wheel(FRONT).traction + bike.wheel(REAR).traction) * FIXED_STEP;
        }
        if now < 0.3 || step + 1 == (40.0 / FIXED_STEP) as usize {
            break;
        }
    }
    out.mean = speed / time.max(1e-3) / G;
    out
}

#[test]
fn a_full_brake_stops_a_road_bike_in_about_30_m_from_100_kmh_where_the_reference_needs_more() {
    let speed = 100.0 / 3.6;
    let helped = stop(&road(ArcadeBrake::default()), speed, 0.0, 0.0);
    let reference = stop(&road(ArcadeBrake::REFERENCE), speed, 0.0, 0.0);
    println!(
        "100 km/h: helped {:.2} g {:.1} m, reference {:.2} g {:.1} m",
        helped.mean, helped.distance, reference.mean, reference.distance
    );
    assert!(!helped.fell && !reference.fell);
    assert!(helped.mean >= 1.25, "mean {} g", helped.mean);
    assert!(helped.distance <= 33.0, "{} m", helped.distance);
    assert!(
        reference.mean < 1.05 && reference.distance > 1.25 * helped.distance,
        "{} g {} m",
        reference.mean,
        reference.distance
    );
}

#[test]
fn the_stop_from_200_kmh_is_about_120_m_or_less_without_a_fall() {
    let helped = stop(&road(ArcadeBrake::default()), 200.0 / 3.6, 0.0, 0.0);
    println!(
        "200 km/h: {:.2} g {:.1} m rear up {:.2} s nose down {:.1} deg",
        helped.mean,
        helped.distance,
        helped.rear_up,
        helped.nose_down.to_degrees()
    );
    assert!(!helped.fell);
    assert!(helped.distance <= 125.0, "{} m", helped.distance);
}

#[test]
fn the_rear_wheel_stays_down_and_the_nose_does_not_go_over_in_a_full_brake() {
    for speed in [60.0_f32, 100.0, 160.0, 220.0] {
        let helped = stop(&road(ArcadeBrake::default()), speed / 3.6, 0.0, 0.0);
        println!("{speed} km/h: rear up {:.2} s nose down {:.1} deg", helped.rear_up, helped.nose_down.to_degrees());
        assert!(!helped.fell, "{speed} km/h");
        assert!(helped.rear_up < 0.5, "{speed} km/h: the rear wheel was up for {} s", helped.rear_up);
        assert!(
            helped.nose_down > -8.0_f32.to_radians(),
            "{speed} km/h: the nose went down to {} deg",
            helped.nose_down.to_degrees()
        );
    }
}

#[test]
fn the_hold_keeps_the_rear_wheel_down_longer_than_the_force_alone() {
    let speed = 160.0 / 3.6;
    let held = stop(&road(ArcadeBrake::default()), speed, 0.0, 0.0);
    let free = stop(&road(ArcadeBrake { pitch_hold: 0.0, ..ArcadeBrake::default() }), speed, 0.0, 0.0);
    println!(
        "hold: rear up {:.2} s, {:.1} deg; none: {:.2} s, {:.1} deg",
        held.rear_up,
        held.nose_down.to_degrees(),
        free.rear_up,
        free.nose_down.to_degrees()
    );
    assert!(held.rear_up <= free.rear_up && held.nose_down >= free.nose_down - 1e-4);
}

#[test]
fn a_rider_who_shifts_his_weight_forward_still_lifts_the_rear_wheel() {
    let speed = 100.0 / 3.6;
    let helped = stop(&road(ArcadeBrake::default()), speed, 1.0, 0.0);
    let reference = stop(&road(ArcadeBrake::REFERENCE), speed, 1.0, 0.0);
    println!(
        "weight forward: helped nose {:.1} deg rear {:.2} s, reference {:.1} deg {:.2} s",
        helped.nose_down.to_degrees(),
        helped.rear_up,
        reference.nose_down.to_degrees(),
        reference.rear_up
    );
    assert!(!helped.fell);
    // The hold is gone with the weight forward: the rear wheel goes up and the nose far down (the stop is shorter, so the stoppie is shorter than the
    // reference's, which lasts for the 4 s it takes the bike to stop).
    assert!(
        helped.nose_down < -10.0_f32.to_radians(),
        "the nose went down to {} deg only",
        helped.nose_down.to_degrees()
    );
    assert!(
        helped.rear_up > 1.0 && reference.rear_up > helped.rear_up,
        "{} s against {} s",
        helped.rear_up,
        reference.rear_up
    );
}

#[test]
fn the_tyres_give_what_they_gave_and_the_force_makes_up_the_rest() {
    let speed = 100.0 / 3.6;
    let helped = stop(&road(ArcadeBrake::default()), speed, 0.0, 0.0);
    let reference = stop(&road(ArcadeBrake::REFERENCE), speed, 0.0, 0.0);
    println!(
        "tyre impulse in the first second: helped {:.0} N s, reference {:.0} N s",
        helped.tyre_impulse, reference.tyre_impulse
    );
    // The tyres are not asked for much more than the reference's: the hold keeps load on the rear tyre, which then brakes a little harder (+18 percent
    // measured), and the force at the centre of gravity does the rest.
    assert!(
        helped.tyre_impulse <= 1.3 * reference.tyre_impulse,
        "{} against {}",
        helped.tyre_impulse,
        reference.tyre_impulse
    );
    assert!(helped.tyre_impulse >= 0.9 * reference.tyre_impulse);
}

#[test]
fn a_scooter_aims_lower_and_the_throttle_takes_the_force_away() {
    let speed = 100.0 / 3.6;
    let mut scooter = road(ArcadeBrake::default());
    scooter.rear.radius = 0.28;
    let slow = stop(&scooter, speed, 0.0, 0.0);
    let fast = stop(&road(ArcadeBrake::default()), speed, 0.0, 0.0);
    println!("scooter {:.2} g, road {:.2} g", slow.mean, fast.mean);
    assert!(slow.mean < fast.mean - 0.2);
    let on_throttle = stop(&road(ArcadeBrake::default()), speed, 0.0, 0.8);
    let reference = stop(&road(ArcadeBrake::REFERENCE), speed, 0.0, 0.8);
    println!("brake and throttle: helped {:.2} g, reference {:.2} g", on_throttle.mean, reference.mean);
    assert!(on_throttle.mean < reference.mean + 0.1, "no help against the throttle");
}

#[test]
fn a_bike_that_stands_is_left_alone_by_the_help() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(road(ArcadeBrake::default()));
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    for _ in 0..300 {
        bike.step(FIXED_STEP, &BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() }, &ground);
    }
    assert!(bike.velocity().length() < 0.05 && bike.pitch().abs() < 0.05 && !bike.fallen());
}

#[test]
fn the_help_is_neutral_without_a_lever_and_in_the_reference_steering() {
    let ground = FlatGround::new(0.0);
    let run = |spec: BikeSpec| {
        let mut bike = Bike::new(spec);
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 25.0));
        for _ in 0..240 {
            bike.step(FIXED_STEP, &BikeInput { throttle: 0.2, ..Default::default() }, &ground);
        }
        bike.velocity()
    };
    assert_eq!(run(road(ArcadeBrake::default())), run(road(ArcadeBrake::REFERENCE)), "no lever, no help");
    let mut reference = BikeSpec::example();
    reference.steering.arcade = ArcadeSteering::REFERENCE;
    assert_eq!(reference.steering.arcade.brake, ArcadeBrake::REFERENCE);
}
