//! A sweep over the imported bikes of what the rider's weight lever does at speed (`docs/specs/ride6-weight-round9.md`), a diagnostic
//! and not a regression test:
//!
//! ```text
//! RIDE6_CARS=%APPDATA%/nfsmw/config/mods/ride6-bikes/cars [RIDE6_FILTER=a,b] cargo test -p blackbox-modding \
//!     --features host --lib bike_weight -- --ignored --nocapture
//! ```
//!
//! `bike_weight_brake` brakes hard from `RIDE6_SPEED` (30 m/s) with the lever at 0, -1 and +1 and prints the highest the rear rises
//! (the angle of the line through the hubs) and the body's nose-down pitch; `bike_weight_accel` holds a speed, opens the throttle
//! and prints where the lever and the tuck end for several sticks.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::parse_car;

fn specs() -> Vec<(String, BikeSpec)> {
    let Some(dir) = std::env::var_os("RIDE6_CARS") else { return Vec::new() };
    let filter = std::env::var("RIDE6_FILTER").unwrap_or_default();
    let mut paths: Vec<_> = std::fs::read_dir(dir).expect("a readable folder").flatten().map(|e| e.path()).collect();
    paths.sort();
    let matches = |name: &str| filter.is_empty() || filter.split(',').any(|part| name.contains(part));
    let step: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    paths
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .filter(|p| matches(&p.file_stem().unwrap_or_default().to_string_lossy()))
        .enumerate()
        .filter(|(n, _)| n % step == 0)
        .filter_map(|(_, p)| {
            let name = p.file_stem()?.to_string_lossy().to_string();
            let manifest = parse_car(&name, &std::fs::read_to_string(p).ok()?).ok()?;
            let mut spec = BikeSpec::from(manifest.bike.as_ref()?);
            // `RIDE6_STRICT_TUCK=1`: the reference's own tuck rule, a lever beyond the dead zone on either side cancels it.
            let strict = std::env::var_os("RIDE6_STRICT_TUCK").is_some();
            if let Some(rule) = spec.wheelie.as_mut().and_then(|w| w.help.auto_tuck.as_mut()).filter(|_| strict) {
                rule.forward_weight_tucks = false;
            }
            Some((name, spec))
        })
        .collect()
}

fn number(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// What a braking run saw.
#[derive(Default)]
struct Brake {
    /// The highest the rear rose: the angle of the line through the hubs less what it is at rest (deg), while the rear was off the road.
    lift: f32,
    /// The deepest nose-down pitch of the body (deg).
    dive: f32,
    /// How long the rear was off the road (s).
    off: f32,
    /// The strongest deceleration (m/s^2) over a 0.2 s window and the speed at the end.
    decel: f32,
    fell: bool,
}

fn lift_deg(bike: &Bike) -> f32 {
    let (front, rear) = (bike.wheel_pose(0).0, bike.wheel_pose(1).0);
    let spec = bike.spec();
    let run = (front - rear).length().max(1e-3);
    let at_rest = (spec.front.radius - spec.rear.radius) / run;
    (((front.y - rear.y) / run) - at_rest).clamp(-1.0, 1.0).asin().to_degrees()
}

fn brake_run(spec: &BikeSpec, speed: f32, weight: f32, level: Option<f32>, lever: f32) -> Brake {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    if let Some(level) = level {
        bike.set_abs_level(level);
    }
    bike.set_weight(weight);
    let mut out = Brake::default();
    let input = BikeInput { front_brake: lever, rear_brake: lever, ..Default::default() };
    let mut window = std::collections::VecDeque::new();
    for _ in 0..(number("RIDE6_SECONDS", 6.0) / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        if bike.fallen() {
            out.fell = true;
            break;
        }
        window.push_back(bike.forward_speed());
        if window.len() > 12 {
            let old = window.pop_front().unwrap_or(0.0);
            out.decel = out.decel.max((old - bike.forward_speed()) / (12.0 * FIXED_STEP));
        }
        out.dive = out.dive.max(-bike.pitch().to_degrees());
        if !bike.wheel(1).on_ground {
            out.off += FIXED_STEP;
            out.lift = out.lift.max(-lift_deg(&bike));
        }
        if bike.forward_speed() < 3.0 {
            break;
        }
    }
    out
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_weight_brake() {
    let speed = number("RIDE6_SPEED", 30.0);
    let lever = number("RIDE6_LEVER", 1.0);
    let level = std::env::var("RIDE6_ABS").ok().and_then(|v| v.parse().ok());
    let weights = [0.0_f32, -1.0, 0.85, 1.0];
    println!(
        "hard brake from {speed} m/s, lever {lever}, abs {level:?}: rear lift (deg) / body dive (deg) / peak decel (m/s^2) for the weights {weights:?}"
    );
    let mut sums = [0.0_f32; 4];
    let mut count = 0.0;
    for (name, spec) in specs() {
        let runs = weights.map(|w| brake_run(&spec, speed, w, level, lever));
        let cells: Vec<String> = runs
            .iter()
            .map(|r| format!("{:5.1}/{:5.1}/{:4.1}{}", r.lift, r.dive, r.decel, if r.fell { "F" } else { " " }))
            .collect();
        println!("{name:34} {}", cells.join(" | "));
        for (n, r) in runs.iter().enumerate() {
            sums[n] += r.lift;
        }
        count += 1.0;
    }
    if count > 0.0 {
        println!(
            "MEAN lift {:.1} {:.1} {:.1} {:.1}",
            sums[0] / count,
            sums[1] / count,
            sums[2] / count,
            sums[3] / count
        );
    }
}

/// Holds `speed`, then opens the throttle fully for 4 s with the stick at each of `sticks`: the lever, the tuck level and the pitch.
#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_weight_accel() {
    let speed = number("RIDE6_SPEED", 30.0);
    for (name, spec) in specs() {
        let mut line = String::new();
        for stick in [-1.0_f32, -0.5, 0.0, 0.5, 0.8, 1.0] {
            let ground = FlatGround::new(0.0);
            let mut bike = Bike::new(spec.clone());
            assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
            bike.set_weight(stick);
            let (mut max_pitch, mut min_tuck, mut at) = (0.0_f32, 1.0_f32, 0.0_f32);
            for step in 0..(4.0 / FIXED_STEP) as usize {
                bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
                max_pitch = max_pitch.max(bike.pitch().to_degrees());
                if step as f32 * FIXED_STEP > 1.0 {
                    min_tuck = min_tuck.min(bike.tuck_level());
                }
                at = bike.rider_weight();
            }
            line += &format!(" | s{stick:+.1}: lever {at:+.2} tuck {:.2} pitch {max_pitch:.1}", min_tuck);
        }
        println!("{name:34}{line}");
    }
}

/// One braking run traced every `RIDE6_EVERY` steps (default 6): speed, pitch, lift, loads, tyre forces and slips.
#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_weight_trace() {
    let speed = number("RIDE6_SPEED", 30.0);
    let lever = number("RIDE6_LEVER", 1.0);
    let weight = number("RIDE6_WEIGHT", 0.0);
    let every = number("RIDE6_TRACE_EVERY", 6.0) as usize;
    let level = std::env::var("RIDE6_ABS").ok().and_then(|v| v.parse().ok());
    for (name, spec) in specs().into_iter().take(1) {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec);
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
        if let Some(level) = level {
            bike.set_abs_level(level);
        }
        bike.set_weight(weight);
        let input = BikeInput { front_brake: lever, rear_brake: lever, ..Default::default() };
        println!("{name}");
        let (switch_at, second) = (number("RIDE6_SWITCH_AT", f32::MAX), number("RIDE6_WEIGHT2", 0.0));
        for step in 0..(4.0 / FIXED_STEP) as usize {
            if step as f32 * FIXED_STEP >= switch_at {
                bike.set_weight(second);
            }
            bike.step(FIXED_STEP, &input, &ground);
            if step % every != 0 {
                continue;
            }
            let (f, r) = (bike.wheel(0), bike.wheel(1));
            println!(
                "t{:.2} v{:.1} pitch{:+.1} lift{:+.1} aid{:+.2} fLoad{:.0} rLoad{:.0} fFx{:.0} rFx{:.0} fSlip{:+.3} rSlip{:+.3} fN{:+.2} rN{:+.2} fOn{} rOn{}",
                step as f32 * FIXED_STEP,
                bike.forward_speed(),
                bike.pitch().to_degrees(),
                lift_deg(&bike),
                bike.rider_weight(),
                f.load,
                r.load,
                f.force.dot(bike.orientation() * glam::Vec3::Z),
                r.force.dot(bike.orientation() * glam::Vec3::Z),
                f.slip_ratio,
                r.slip_ratio,
                f.normalised_x,
                r.normalised_x,
                u8::from(f.on_ground),
                u8::from(r.on_ground),
            );
        }
    }
}
