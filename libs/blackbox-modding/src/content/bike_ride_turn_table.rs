//! The turning circle of a full stick at the speeds of the turn law's table (`docs/specs/ride6-bike-turn-radius.md`), ridden on the imported bikes
//! of a folder as an ignored test, a diagnostic and not a regression test:
//!
//! ```text
//! RIDE6_CARS=<folder> [RIDE6_FILTER=a,b] [RIDE6_EVERY=n] [RIDE6_ARCADE=reference|round2|...] [RIDE6_KMH=15,30,50,80,120,180] \
//!   cargo test --release -p blackbox-modding --features host --lib bike_turn_table -- --ignored --nocapture
//! ```
//!
//! Per bike and speed (km/h, held by the throttle): the radius of the circle after the lean has built up (the speed over the yaw rate of the last 3 s of
//! 10), the seconds the yaw rate takes to reach 80 percent of that after the stick is stepped on from straight running, and a fall. Then the median of
//! each over the bikes against the table of the turn law (`TurnLaw::GAME`), and the falls of a flip of the stick from side to side at each speed.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec, TurnLaw};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_steer_arcade::arcade;
use super::bike_ride_sweep::{held_throttle, specs};

/// What a held full stick did at one speed.
struct Run {
    fell: bool,
    radius: f32,
    t80: f32,
}

/// The heading (rad) of the bike velocity.
fn heading(bike: &Bike) -> f32 {
    let travel = bike.velocity();
    travel.x.atan2(travel.z)
}

fn wrap(mut delta: f32) -> f32 {
    while delta > std::f32::consts::PI {
        delta -= 2.0 * std::f32::consts::PI;
    }
    while delta < -std::f32::consts::PI {
        delta += 2.0 * std::f32::consts::PI;
    }
    delta
}

/// A full stick stepped on after a second of straight running at `speed` m/s, held for 10 s.
fn run(spec: &BikeSpec, speed: f32) -> Run {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let (lead, total) = (1.0_f32, 10.0_f32);
    let mut rates = Vec::new();
    let (mut last, mut speeds) = (heading(&bike), Vec::new());
    for step in 0..(total / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let stick = if t < lead { 0.0 } else { 1.0 };
        let v = bike.velocity().length();
        let throttle = held_throttle(speed, v);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        if bike.fallen() {
            return Run { fell: true, radius: f32::NAN, t80: f32::NAN };
        }
        let now = heading(&bike);
        rates.push(wrap(now - last).abs() / FIXED_STEP);
        speeds.push(bike.velocity().length());
        last = now;
    }
    let tail = (3.0 / FIXED_STEP) as usize;
    let rate = rates.iter().rev().take(tail).sum::<f32>() / tail as f32;
    let mean_speed = speeds.iter().rev().take(tail).sum::<f32>() / tail as f32;
    let from = (lead / FIXED_STEP) as usize;
    let t80 = rates.iter().skip(from).position(|&r| r >= 0.8 * rate).map_or(f32::NAN, |at| at as f32 * FIXED_STEP);
    Run { fell: false, radius: mean_speed / rate.max(1e-4), t80 }
}

/// Whether a flip of the stick from side to side every 0.8 s for 6 s keeps the bike up at `speed` m/s.
fn flips_without_a_fall(spec: &BikeSpec, speed: f32) -> bool {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    for step in 0..(6.0 / FIXED_STEP) as usize {
        let stick = if (step as f32 * FIXED_STEP / 0.8) as usize % 2 == 0 { 1.0 } else { -1.0 };
        let throttle = held_throttle(speed, bike.velocity().length());
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        if bike.fallen() {
            return false;
        }
    }
    true
}

fn median(mut values: Vec<f32>) -> f32 {
    values.retain(|v| v.is_finite());
    values.sort_by(|a, b| a.total_cmp(b));
    values.get(values.len() / 2).copied().unwrap_or(f32::NAN)
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_turn_table() {
    let kmh: Vec<f32> = std::env::var("RIDE6_KMH")
        .map(|t| t.split(',').filter_map(|p| p.parse().ok()).collect())
        .unwrap_or_else(|_| vec![15.0, 30.0, 50.0, 80.0, 120.0, 180.0]);
    let every: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
    let steering = arcade();
    let bikes: Vec<(String, BikeSpec)> = specs()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % every == 0)
        .map(|(_, (name, mut spec))| {
            spec.steering.arcade = steering;
            (name, spec)
        })
        .collect();
    assert!(!bikes.is_empty(), "no bike read from RIDE6_CARS");
    let law = TurnLaw::GAME;
    println!(
        "{} bikes; radius after the lean has built up, seconds to 80 percent of the yaw rate, falls (full stick held)",
        bikes.len()
    );
    println!("km/h | table m | median radius m | min..max m | median t80 s | falls (turn / flip)");
    for &speed in &kmh {
        let ms = speed / 3.6;
        let runs: Vec<Run> = bikes.iter().map(|(_, spec)| run(spec, ms)).collect();
        let radii: Vec<f32> = runs.iter().map(|r| r.radius).collect();
        let (low, high) =
            radii.iter().filter(|r| r.is_finite()).fold((f32::MAX, 0.0_f32), |(l, h), &r| (l.min(r), h.max(r)));
        let turn_falls = runs.iter().filter(|r| r.fell).count();
        let flip_falls = bikes.iter().filter(|(_, spec)| !flips_without_a_fall(spec, ms)).count();
        println!(
            "{speed:4.0} | {:7.1} | {:15.1} | {low:.1}..{high:.1} | {:12.2} | {turn_falls} / {flip_falls}",
            law.radius(ms),
            median(radii.clone()),
            median(runs.iter().map(|r| r.t80).collect()),
        );
    }
}
