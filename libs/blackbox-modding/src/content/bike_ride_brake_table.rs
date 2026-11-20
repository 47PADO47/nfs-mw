//! Full braking from several speeds on the imported bikes (`docs/specs/ride6-bike-braking.md`), ridden as an ignored test, a diagnostic and not a
//! regression test:
//!
//! ```text
//! RIDE6_CARS=<folder> RIDE6_FILTER=yzf-r1-2020,ktm_690,... [RIDE6_KMH=60,100,160,220] [RIDE6_SURFACE=0.6] [RIDE6_ARCADE=reference|...] \
//!   cargo test --release -p blackbox-modding --features host --lib bike_brake_table -- --ignored --nocapture
//! ```
//!
//! Per bike and speed: both levers fully pulled, no throttle, upright, until the bike stops. It prints the peak deceleration (g, over 0.1 s), the mean
//! (speed over the time to stop), the stopping distance, the most the rear wheel left the road (a stoppie, seconds and the nose-down pitch) and a fall.
//! `RIDE6_SURFACE` is the drive and lateral grip multiplier of the road (1 tarmac, 0.6 gravel).

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround, SurfaceGrip};

use super::bike_ride_steer_arcade::arcade;
use super::bike_ride_sweep::specs;

const G: f32 = 9.81;

/// What one full stop did.
struct Stop {
    fell: bool,
    peak: f32,
    mean: f32,
    distance: f32,
    /// Seconds the rear wheel was off the road, and the lowest pitch (rad, nose down is negative).
    rear_up: f32,
    nose_down: f32,
}

fn ground() -> FlatGround {
    let grip: f32 = std::env::var("RIDE6_SURFACE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let mut ground = FlatGround::new(0.0);
    ground.surface = SurfaceGrip { lateral: grip, drive: grip, ..SurfaceGrip::DEFAULT };
    ground
}

fn stop(spec: &BikeSpec, speed: f32, front: f32, rear: f32) -> Stop {
    let ground = ground();
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    // A second of settling at the speed (the throttle holds it), then the levers.
    let settle = (0.5 / FIXED_STEP) as usize;
    let mut out = Stop { fell: false, peak: 0.0, mean: 0.0, distance: 0.0, rear_up: 0.0, nose_down: 0.0 };
    let mut window: Vec<f32> = Vec::new();
    let (mut time, mut start_speed) = (0.0_f32, speed);
    for step in 0..(40.0 / FIXED_STEP) as usize {
        let v = bike.velocity().length();
        let braking = step >= settle;
        let throttle = if braking { 0.0 } else { (0.25 + 0.2 * (speed - v)).clamp(0.0, 1.0) };
        let input = BikeInput {
            throttle,
            front_brake: if braking { front } else { 0.0 },
            rear_brake: if braking { rear } else { 0.0 },
            steer: 0.0,
        };
        if step == settle {
            start_speed = v;
        }
        bike.step(FIXED_STEP, &input, &ground);
        if bike.fallen() {
            out.fell = true;
            break;
        }
        if !braking {
            continue;
        }
        let now = bike.velocity().length();
        out.distance += now * FIXED_STEP;
        time += FIXED_STEP;
        window.push(now);
        let lag = (0.1 / FIXED_STEP) as usize;
        if window.len() > lag {
            let decel = (window[window.len() - 1 - lag] - now) / 0.1 / G;
            // Only the part of the stop above walking pace counts (the last metres are the standing hold's).
            if now > 3.0 {
                out.peak = out.peak.max(decel);
            }
        }
        if !bike.wheel(1).on_ground {
            out.rear_up += FIXED_STEP;
        }
        out.nose_down = out.nose_down.min(bike.pitch());
        if now < 0.3 {
            break;
        }
    }
    out.mean = start_speed / time.max(1e-3) / G;
    out
}

fn median(mut values: Vec<f32>) -> f32 {
    values.retain(|v| v.is_finite());
    values.sort_by(|a, b| a.total_cmp(b));
    values.get(values.len() / 2).copied().unwrap_or(f32::NAN)
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_brake_table() {
    let kmh: Vec<f32> = std::env::var("RIDE6_KMH")
        .map(|t| t.split(',').filter_map(|p| p.parse().ok()).collect())
        .unwrap_or_else(|_| vec![60.0, 100.0, 160.0, 220.0]);
    let every: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
    let levers: (f32, f32) = match std::env::var("RIDE6_LEVERS").as_deref() {
        Ok("front") => (1.0, 0.0),
        Ok("rear") => (0.0, 1.0),
        _ => (1.0, 1.0),
    };
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
    println!("bike | class | km/h | peak g | mean g | distance m | rear up s | nose-down deg | fell");
    for &speed in &kmh {
        let mut means = Vec::new();
        let mut distances = Vec::new();
        let mut falls = 0;
        for (name, spec) in &bikes {
            let s = stop(spec, speed / 3.6, levers.0, levers.1);
            println!(
                "{name:36} | {:?} | {speed:4.0} | {:5.2} | {:5.2} | {:6.1} | {:4.2} | {:5.1} | {}",
                spec.terrain_class(),
                s.peak,
                s.mean,
                s.distance,
                s.rear_up,
                s.nose_down.to_degrees(),
                if s.fell { "FELL" } else { "-" }
            );
            means.push(s.mean);
            distances.push(s.distance);
            falls += s.fell as usize;
        }
        println!(
            "== {speed:4.0} km/h: median mean {:.2} g, median distance {:.1} m, falls {falls} of {}",
            median(means),
            median(distances),
            bikes.len()
        );
    }
}
