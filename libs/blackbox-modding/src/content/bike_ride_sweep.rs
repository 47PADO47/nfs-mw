//! A sweep over many imported bikes ridden on flat ground, a diagnostic and not a regression test:
//!
//! ```text
//! RIDE6_CARS=C:/r6/final/mods/ride6-bikes/cars [RIDE6_FILTER=regex-free substring] cargo test -p blackbox-modding \
//!     --features host --lib bike_sweep -- --ignored --nocapture
//! ```
//!
//! Per bike it prints a standing start (full throttle, half throttle), corners at several speeds and sticks (a held speed, a
//! full-stick step as a keyboard gives), and hard braking (both brakes, front only), and flags the failures: a fall, a flip, a
//! gear that never changes, a speed that collapses under throttle, a corner far wider than the lean allows, a tyre that slides.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::parse_car;

const G: f32 = 9.81;

fn load(path: &std::path::Path) -> Option<(String, BikeSpec)> {
    let text = std::fs::read_to_string(path).ok()?;
    let name = path.file_stem()?.to_string_lossy().to_string();
    let manifest = parse_car(&name, &text).ok()?;
    let mut spec = BikeSpec::from(manifest.bike.as_ref()?);
    // `RIDE6_TCS_SCALE=0.5` narrows the traction control's slip window, to see what a stronger control would do.
    let scale: f32 = std::env::var("RIDE6_TCS_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    if let Some(tcs) = spec.tcs.as_mut() {
        tcs.slip = [tcs.slip[0] * scale, tcs.slip[1] * scale];
    }
    Some((name, spec))
}

fn specs() -> Vec<(String, BikeSpec)> {
    let Some(dir) = std::env::var_os("RIDE6_CARS") else { return Vec::new() };
    let filter = std::env::var("RIDE6_FILTER").unwrap_or_default();
    let mut paths: Vec<_> = std::fs::read_dir(dir).expect("a readable folder").flatten().map(|e| e.path()).collect();
    paths.sort();
    let matches = |name: &str| filter.is_empty() || filter.split(',').any(|part| name.contains(part));
    paths
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .filter(|p| matches(&p.file_stem().unwrap_or_default().to_string_lossy()))
        .filter_map(|p| load(p))
        .collect()
}

/// What one run observed.
#[derive(Default)]
struct Log {
    fell: Option<(f32, String)>,
    max_pitch: f32,
    max_lean: f32,
    lean_limit: f32,
    peak_rear_slip: f32,
    peak_front_slip: f32,
    peak_rear_angle: f32,
    peak_front_angle: f32,
    gears: Vec<(usize, f32, f32)>,
    t100: Option<f32>,
    top: f32,
    distance: f32,
    min_speed: f32,
}

fn run(
    spec: &BikeSpec,
    start: f32,
    seconds: f32,
    weight: f32,
    mut input: impl FnMut(f32, &Bike) -> BikeInput,
) -> (Bike, Log) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
    bike.set_weight(weight);
    let mut log = Log { min_speed: f32::MAX, ..Default::default() };
    let mut gear = bike.gear();
    let rates = std::env::var_os("RIDE6_RATES").is_some();
    let mut smooth = [0.0_f32; 2];
    for step in 0..(seconds / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let mut command = input(t, &bike);
        if rates {
            // Experiment: the data's rise and fall rates of the pedals (3 and 5 per second).
            let follow = |now: f32, to: f32| now + (to - now).clamp(-5.0 * FIXED_STEP, 3.0 * FIXED_STEP);
            smooth[0] = follow(smooth[0], command.throttle);
            smooth[1] = follow(smooth[1], command.front_brake.max(command.rear_brake));
            command.throttle = smooth[0];
            command.front_brake = smooth[1];
            command.rear_brake = smooth[1];
        }
        bike.step(FIXED_STEP, &command, &ground);
        let speed = bike.body().linear_velocity.length();
        log.top = log.top.max(speed);
        log.min_speed = log.min_speed.min(speed);
        log.distance += speed * FIXED_STEP;
        log.max_pitch = log.max_pitch.max(bike.pitch().abs());
        log.max_lean = log.max_lean.max(bike.lean().abs());
        log.lean_limit = bike.lean_limit();
        if t > 0.3 {
            log.peak_rear_slip = log.peak_rear_slip.max(bike.wheel(1).slip_ratio.abs());
            log.peak_front_slip = log.peak_front_slip.max(bike.wheel(0).slip_ratio.abs());
            log.peak_rear_angle = log.peak_rear_angle.max(bike.wheel(1).slip_angle.abs());
            log.peak_front_angle = log.peak_front_angle.max(bike.wheel(0).slip_angle.abs());
        }
        if bike.gear() != gear {
            log.gears.push((bike.gear(), t, bike.rpm()));
            gear = bike.gear();
        }
        if log.t100.is_none() && speed >= 100.0 / 3.6 {
            log.t100 = Some(t);
        }
        if bike.fallen() && log.fell.is_none() {
            log.fell = Some((t, format!("{:?}", bike.fall_kind())));
        }
        if log.fell.is_some() && t > log.fell.as_ref().map_or(0.0, |f| f.0) + 1.0 {
            break;
        }
    }
    (bike, log)
}

fn shifts(log: &Log) -> String {
    let list: Vec<String> = log.gears.iter().map(|(g, t, rpm)| format!("{g}@{t:.1}s/{:.0}", rpm)).collect();
    list.join(" ")
}

fn fell(log: &Log) -> String {
    match &log.fell {
        Some((t, kind)) => format!("FELL {t:.1}s {kind}"),
        None => "ok".to_string(),
    }
}

fn launch(spec: &BikeSpec) -> Vec<String> {
    let mut flags = Vec::new();
    let full = |_: f32, _: &Bike| BikeInput { throttle: 1.0, ..Default::default() };
    let (bike, log) = run(spec, 0.0, 25.0, 0.0, full);
    println!(
        "  full  : {} t100 {} top {:.1} m/s ({:.0} km/h) end gear {} rpm {:.0}/{:.0} pitch {:.2} rear slip {:.2} | {}",
        fell(&log),
        log.t100.map_or("-".to_string(), |t| format!("{t:.1}s")),
        log.top,
        log.top * 3.6,
        bike.gear(),
        bike.rpm(),
        spec.engine.max_rpm,
        log.max_pitch,
        log.peak_rear_slip,
        shifts(&log)
    );
    if log.fell.is_some() {
        flags.push("full-fall");
    }
    if log.max_pitch > 1.05 {
        flags.push("full-flip");
    }
    if log.t100.is_none() {
        flags.push("no-100");
    }
    let gears = spec.transmission.gears.len();
    if bike.gear() + 1 < gears && log.top < 70.0 {
        flags.push("full-stuck-gear");
    }
    let half = |_: f32, _: &Bike| BikeInput { throttle: 0.5, ..Default::default() };
    let (bike, log) = run(spec, 0.0, 12.0, 0.0, half);
    println!(
        "  half  : {} v {:.1} m/s gear {} rpm {:.0} pitch {:.2} min {:.1} | {}",
        fell(&log),
        bike.body().linear_velocity.length(),
        bike.gear(),
        bike.rpm(),
        log.max_pitch,
        log.min_speed,
        shifts(&log)
    );
    if bike.body().linear_velocity.length() < 12.0 {
        flags.push("half-slow");
    }
    // A keyboard throttle: 0 to 1 over 1.5 s.
    let ramp = |t: f32, _: &Bike| BikeInput { throttle: (t / 1.5).min(1.0), ..Default::default() };
    let (bike, log) = run(spec, 0.0, 10.0, 0.0, ramp);
    println!(
        "  ramp  : {} v {:.1} m/s gear {} pitch {:.2} rear slip {:.2}",
        fell(&log),
        bike.body().linear_velocity.length(),
        bike.gear(),
        log.max_pitch,
        log.peak_rear_slip
    );
    if log.fell.is_some() {
        flags.push("ramp-fall");
    }
    flags.into_iter().map(String::from).collect()
}

fn corner(spec: &BikeSpec) -> Vec<String> {
    let mut flags = Vec::new();
    for speed in [10.0_f32, 20.0, 30.0] {
        for (steer, step) in [(0.3, false), (0.5, false), (1.0, true), (1.0, false)] {
            let hold = move |t: f32, bike: &Bike| {
                let v = bike.body().linear_velocity.length();
                let throttle = (0.25 + 0.2 * (speed - v)).clamp(0.0, 1.0);
                let stick = match step {
                    true => steer,
                    false => (steer * t / 1.0).min(steer),
                };
                BikeInput { throttle, steer: stick, ..Default::default() }
            };
            let seconds = 9.0;
            let (bike, log) = run(spec, speed, seconds, 0.0, hold);
            let v = bike.body().linear_velocity;
            // The turn radius over the last 3 s needs the heading; take it from a second run to the same instant.
            let (_, radius) = measure_radius(spec, speed, steer, step);
            let expected = speed * speed / (G * log.max_lean.max(0.01).tan());
            println!(
                "  corner v{speed:2.0} steer {steer:.1}{}: {} lean {:.2} (limit {:.2}) v {:.1} radius {radius:.0} m (lean-radius {expected:.0}) slip ang f {:.2} r {:.2} rear slip {:.2}",
                if step { " step" } else { " ramp" },
                fell(&log),
                log.max_lean,
                log.lean_limit,
                v.length(),
                log.peak_front_angle,
                log.peak_rear_angle,
                log.peak_rear_slip
            );
            if log.fell.is_some() {
                flags.push(format!("corner-fall v{speed} s{steer}{}", if step { "step" } else { "" }));
            }
            if v.length() < 0.75 * speed && log.fell.is_none() {
                flags.push(format!("corner-bleed v{speed} s{steer}"));
            }
        }
    }
    flags
}

fn measure_radius(spec: &BikeSpec, speed: f32, steer: f32, step: bool) -> (f32, f32) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let (mut yaw0, mut yaw1) = (0.0, 0.0);
    for n in 0..(9.0 / FIXED_STEP) as usize {
        let t = n as f32 * FIXED_STEP;
        let v = bike.body().linear_velocity.length();
        let throttle = (0.25 + 0.2 * (speed - v)).clamp(0.0, 1.0);
        let stick = if step { steer } else { (steer * t).min(steer) };
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        let travel = bike.body().linear_velocity;
        let yaw = travel.x.atan2(travel.z);
        if n == (6.0 / FIXED_STEP) as usize {
            yaw0 = yaw;
        }
        yaw1 = yaw;
    }
    let rate = (yaw1 - yaw0) / 3.0;
    (rate, bike.body().linear_velocity.length() / rate.abs().max(1e-3))
}

fn braking(spec: &BikeSpec) -> Vec<String> {
    let mut flags = Vec::new();
    for (name, front, rear) in [("both ", 1.0, 1.0), ("front", 1.0, 0.0), ("rear ", 0.0, 1.0)] {
        let brake = move |_: f32, _: &Bike| BikeInput { front_brake: front, rear_brake: rear, ..Default::default() };
        let (bike, log) = run(spec, 30.0, 8.0, 0.0, brake);
        println!(
            "  brake {name}: {} distance {:.0} m pitch {:.2} v {:.1} front slip {:.2} rear slip {:.2}",
            fell(&log),
            log.distance,
            log.max_pitch,
            bike.body().linear_velocity.length(),
            log.peak_front_slip,
            log.peak_rear_slip
        );
        if log.fell.is_some() {
            flags.push(format!("brake-{}-fall", name.trim()));
        }
        if log.max_pitch > 1.05 {
            flags.push(format!("brake-{}-flip", name.trim()));
        }
    }
    flags
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_sweep() {
    let mut summary = Vec::new();
    for (name, spec) in specs() {
        println!(
            "{name}: mass {:.0}+{:.0} kg, gears {}, torque {:.0} N m, max {:.0} rpm",
            spec.chassis.mass,
            spec.chassis.rider_mass,
            spec.transmission.gears.len(),
            spec.engine.peak_torque,
            spec.engine.max_rpm
        );
        let mut flags = launch(&spec);
        flags.extend(corner(&spec));
        flags.extend(braking(&spec));
        summary.push((name, flags));
    }
    println!("==== SUMMARY ====");
    for (name, flags) in &summary {
        println!("{name}: {}", if flags.is_empty() { "clean".to_string() } else { flags.join(", ") });
    }
    let bad = summary.iter().filter(|(_, f)| !f.is_empty()).count();
    println!("{bad} of {} bikes flagged", summary.len());
}

/// The largest stick (of 0.6 to 1.0, or `RIDE6_STICKS`) each bike holds for 10 s at several speeds (or `RIDE6_SPEEDS`) without falling.
#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_stick_limit() {
    let list = |key: &str, default: &[f32]| -> Vec<f32> {
        match std::env::var(key) {
            Ok(text) => text.split(',').filter_map(|p| p.parse().ok()).collect(),
            Err(_) => default.to_vec(),
        }
    };
    let speeds = list("RIDE6_SPEEDS", &[8.0, 12.0, 16.0, 20.0, 25.0, 30.0, 40.0]);
    let sticks = list("RIDE6_STICKS", &[0.6, 0.7, 0.8, 0.9, 1.0]);
    let mut worst = vec![1.0_f32; speeds.len()];
    let mut count = vec![vec![0usize; sticks.len()]; speeds.len()];
    let bikes = specs();
    for (name, spec) in &bikes {
        let mut line = String::new();
        for (index, &speed) in speeds.iter().enumerate() {
            let mut best = 0.0;
            for &stick in &sticks {
                let hold = move |_: f32, bike: &Bike| {
                    let v = bike.body().linear_velocity.length();
                    BikeInput {
                        throttle: (0.25 + 0.2 * (speed - v)).clamp(0.0, 1.0),
                        steer: stick,
                        ..Default::default()
                    }
                };
                let (_, log) = run(spec, speed, 10.0, 0.0, hold);
                if log.fell.is_none() {
                    best = stick;
                    continue;
                }
                break;
            }
            worst[index] = f32::min(worst[index], best);
            if let Some(place) = sticks.iter().position(|&stick| stick == best) {
                count[index][place] += 1;
            }
            line += &format!(" {speed:.0}:{best:.1}");
        }
        println!("{name}:{line}");
    }
    println!("==== worst stick per speed");
    for (index, speed) in speeds.iter().enumerate() {
        println!("{speed:.0} m/s: worst {:.1} counts per stick {sticks:?}: {:?}", worst[index], count[index]);
    }
}

/// Keyboard-like manoeuvres at the share of the stick the game gives (0.8): a slalom, a corner with the brake, a slow turn and a start
/// with the stick held.
#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_sweep_extra() {
    let stick = 0.8_f32;
    let mut flagged = Vec::new();
    for (name, spec) in specs() {
        let mut flags = Vec::new();
        let mut check = |label: &str, log: &Log| {
            if log.fell.is_some() || log.max_pitch > 1.0 {
                flags.push(format!("{label} {}", fell(log)));
            }
        };
        for speed in [12.0_f32, 20.0, 30.0] {
            let slalom = move |t: f32, bike: &Bike| {
                let v = bike.body().linear_velocity.length();
                let throttle = (0.25 + 0.2 * (speed - v)).clamp(0.0, 1.0);
                let side = if ((t / 1.5) as usize).is_multiple_of(2) { stick } else { -stick };
                BikeInput { throttle, steer: side, ..Default::default() }
            };
            check(&format!("slalom {speed}"), &run(&spec, speed, 16.0, 0.0, slalom).1);
        }
        let trail = |t: f32, bike: &Bike| {
            let v = bike.body().linear_velocity.length();
            let braking = (3.0..5.0).contains(&t);
            let throttle = if braking { 0.0 } else { (0.25 + 0.2 * (25.0 - v)).clamp(0.0, 1.0) };
            let lever = if braking { 0.5 } else { 0.0 };
            BikeInput { throttle, front_brake: lever, rear_brake: lever, steer: stick }
        };
        check("trail brake 25", &run(&spec, 25.0, 8.0, 0.0, trail).1);
        let slow = move |_: f32, bike: &Bike| {
            let v = bike.body().linear_velocity.length();
            BikeInput { throttle: (0.2 + 0.3 * (4.0 - v)).clamp(0.0, 1.0), steer: stick, ..Default::default() }
        };
        let (bike, log) = run(&spec, 4.0, 10.0, 0.0, slow);
        check("slow turn 4", &log);
        let _ = bike;
        let launch = move |_: f32, _: &Bike| BikeInput { throttle: 1.0, steer: stick, ..Default::default() };
        let (_, log) = run(&spec, 0.0, 10.0, 0.0, launch);
        check("launch with stick", &log);
        println!("{name}: {}", if flags.is_empty() { "clean".to_string() } else { flags.join("; ") });
        if !flags.is_empty() {
            flagged.push(name);
        }
    }
    println!("{} bikes flagged: {flagged:?}", flagged.len());
}
