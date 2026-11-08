//! The audit of how bikes turn at speed (`docs/specs/ride6-arcade-steering.md`), ridden on the imported bikes of a folder
//! (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b] [RIDE6_EVERY=n]`) as ignored tests, a diagnostic and not a regression test:
//!
//! - `bike_steer_audit`: per bike and speed (`RIDE6_SPEEDS`), the time of a full-stick flip from the left lean to the right one (to 80 and
//!   95 percent of the steady lean), the steady turn of a held full stick (radius, lateral g, lean against its limit, the heading error,
//!   the front slip angle and the handlebar, how much of its peak each tyre uses) and then the medians over the bikes.
//! - `bike_steer_sweep`: the falls of a held stick, a slalom, a flick and a tight flip at 8 to 80 m/s over the bikes.
//!
//! `RIDE6_ARCADE=reference` rides the bikes with the reference game's steering (`ArcadeSteering::REFERENCE`), anything else with the
//! game's own.

use blackbox_vehicle::bike::{ArcadeSteering, Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use glam::Vec3;

use super::bike_ride_sweep::{held_throttle, specs};

const G: f32 = 9.81;

fn list(key: &str, default: &[f32]) -> Vec<f32> {
    match std::env::var(key) {
        Ok(text) => text.split(',').filter_map(|p| p.parse().ok()).collect(),
        Err(_) => default.to_vec(),
    }
}

/// The arcade steering of the run: `RIDE6_ARCADE=reference` the reference game's, `RIDE6_ARCADE=grip=1.2,rate=3,...` the game's values with
/// those fields changed (`grip`, `rate`, `floor`, `mirror`, `spring`, `roll`, `assist`, `reach`), unset the game's values.
fn arcade() -> ArcadeSteering {
    let text = std::env::var("RIDE6_ARCADE").unwrap_or_default();
    if text == "reference" {
        return ArcadeSteering::REFERENCE;
    }
    // The steering the game had before the arcade steering (rounds 12 and 13): the data's controller, the speed curve held at 0.65, the stick curve mirrored.
    if text == "before" {
        return ArcadeSteering { time_scale_speed_floor: 0.65, mirror_stick_curve: true, ..ArcadeSteering::REFERENCE };
    }
    let mut arcade = ArcadeSteering::default();
    for part in text.split(',').filter(|p| !p.is_empty()) {
        let Some((key, value)) = part.split_once('=') else { continue };
        let Ok(v) = value.parse::<f32>() else { continue };
        match key {
            "grip" => arcade.lateral_grip = v,
            "lean" => arcade.lean_limit = v,
            "rate" => arcade.lean_rate = v,
            "floor" => arcade.time_scale_speed_floor = v,
            "mirror" => arcade.mirror_stick_curve = v > 0.5,
            "spring" => arcade.roll_spring = v,
            "help" => arcade.lean_help = v,
            "helplimit" => arcade.lean_help_limit = v,
            "spin" => arcade.fall_angular_speed = v,
            "stick" => arcade.stick_response = v,
            "bar" => arcade.handlebar_response = v,
            "assist" => arcade.cornering_assist = v,
            "reach" => arcade.stick_reach = v,
            _ => {}
        }
    }
    arcade
}

/// Every `RIDE6_EVERY`th bike of the folder (all of them by default), with the run's arcade steering.
fn bikes() -> Vec<(String, BikeSpec)> {
    let every: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
    let arcade = arcade();
    specs()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % every == 0)
        .map(|(_, (name, mut spec))| {
            spec.steering.arcade = arcade;
            (name, spec)
        })
        .collect()
}

fn median(mut values: Vec<f32>) -> f32 {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return f32::NAN;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    values[values.len() / 2]
}

/// The flat heading error (rad): the angle between the velocity and the heading.
fn heading_error(bike: &Bike) -> f32 {
    let (v, f) = (bike.velocity(), bike.orientation() * Vec3::Z);
    let (v, f) = (Vec3::new(v.x, 0.0, v.z), Vec3::new(f.x, 0.0, f.z));
    match v.length() > 1.0 && f.length() > 1e-3 {
        true => v.angle_between(f),
        false => 0.0,
    }
}

/// With `RIDE6_FALLS=1`, one line about a fall: when, how, the lean against its limits and the slip angles.
fn note_fall(what: &str, speed: f32, t: f32, bike: &Bike) {
    if std::env::var_os("RIDE6_FALLS").is_none() {
        return;
    }
    println!(
        "  fall {what} v{speed:.0} t {t:.2}s {:?}: lean {:.1} limit {:.1} fall limit {:.1} deg, heading {:.1} deg, slip f {:.1} r {:.1} deg, speed {:.1}",
        bike.fall_kind(),
        bike.lean().to_degrees(),
        bike.lean_limit().to_degrees(),
        bike.fall_lean_limit().to_degrees(),
        heading_error(bike).to_degrees(),
        bike.wheel(0).slip_angle.to_degrees(),
        bike.wheel(1).slip_angle.to_degrees(),
        bike.velocity().length()
    );
}

/// What a steady turn settled into (means over its last 3 s).
#[derive(Default, Clone)]
struct Steady {
    fell: bool,
    radius: f32,
    g: f32,
    lean: f32,
    limit: f32,
    base: f32,
    heading: f32,
    front_slip: f32,
    steer: f32,
    front_use: f32,
    rear_use: f32,
    speed: f32,
}

/// The lean limit before any cut: the front tyre's grip at its rest load, arctangent, held to the steepest lean.
fn base_limit(spec: &BikeSpec) -> f32 {
    let rest = (spec.front_share() * spec.total_mass() * G).max(1.0);
    let grip = spec.front.tyre.lateral_peak(rest * 1e-3) / rest;
    grip.atan().min(spec.steering.max_lean)
}

fn steady(spec: &BikeSpec, speed: f32, stick: f32, seconds: f32) -> Steady {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let steps = (seconds / FIXED_STEP) as usize;
    let from = steps - (3.0 / FIXED_STEP) as usize;
    let mut out = Steady { base: base_limit(spec), ..Default::default() };
    let (mut n, mut last, mut unwrapped, mut yaw0) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    for step in 0..steps {
        let v = bike.velocity().length();
        bike.step(
            FIXED_STEP,
            &BikeInput { throttle: held_throttle(speed, v), steer: stick, ..Default::default() },
            &ground,
        );
        if bike.fallen() {
            note_fall("turn", speed, step as f32 * FIXED_STEP, &bike);
            out.fell = true;
            return out;
        }
        let travel = bike.velocity();
        let yaw = travel.x.atan2(travel.z);
        let mut delta = yaw - last;
        while delta > std::f32::consts::PI {
            delta -= 2.0 * std::f32::consts::PI;
        }
        while delta < -std::f32::consts::PI {
            delta += 2.0 * std::f32::consts::PI;
        }
        last = yaw;
        unwrapped += delta;
        if step == from {
            yaw0 = unwrapped;
        }
        if step < from {
            continue;
        }
        n += 1.0;
        out.lean += bike.lean().abs();
        out.limit += bike.lean_limit();
        out.heading += heading_error(&bike);
        out.front_slip += bike.wheel(0).slip_angle.abs();
        out.steer += bike.steer_angle().abs();
        out.speed += v;
        for (index, share) in [(0, &mut out.front_use), (1, &mut out.rear_use)] {
            let w = bike.wheel(index);
            let tyre = match index {
                0 => &bike.spec().front.tyre,
                _ => &bike.spec().rear.tyre,
            };
            let peak = tyre.lateral_peak(w.load * 1e-3).max(1.0);
            *share += w.force.dot(w.side).abs() / peak;
        }
    }
    let rate = (unwrapped - yaw0) / 3.0;
    for value in [
        &mut out.lean,
        &mut out.limit,
        &mut out.heading,
        &mut out.front_slip,
        &mut out.steer,
        &mut out.front_use,
        &mut out.rear_use,
        &mut out.speed,
    ] {
        *value /= n.max(1.0);
    }
    out.radius = out.speed / rate.abs().max(1e-4);
    out.g = out.speed * rate.abs() / G;
    out
}

/// The flip of a full stick from the left to the right: the seconds to 80 and 95 percent of the steady lean, and that lean.
#[derive(Default, Clone, Copy)]
struct Flip {
    fell: bool,
    t80: f32,
    t95: f32,
    lean: f32,
    /// The seconds the yaw rate (the path) takes to reach 80 percent of its steady value on the other side.
    path80: f32,
}

fn flip(spec: &BikeSpec, speed: f32) -> Flip {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let (hold, after) = (5.0_f32, 3.0_f32);
    let mut out = Flip { t80: f32::NAN, t95: f32::NAN, path80: f32::NAN, ..Default::default() };
    let (mut sum, mut n, mut yaw_sum) = (0.0_f32, 0.0_f32, 0.0_f32);
    for step in 0..((hold + after) / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let v = bike.velocity().length();
        let stick = match t < hold {
            true => -1.0,
            false => 1.0,
        };
        bike.step(
            FIXED_STEP,
            &BikeInput { throttle: held_throttle(speed, v), steer: stick, ..Default::default() },
            &ground,
        );
        if bike.fallen() {
            note_fall("flip", speed, t, &bike);
            out.fell = true;
            return out;
        }
        if t > hold - 0.5 && t < hold {
            sum += -bike.lean();
            yaw_sum += -bike.body().angular_velocity.y;
            n += 1.0;
            out.lean = sum / n;
        }
        if t < hold {
            continue;
        }
        let since = t - hold;
        if out.t80.is_nan() && bike.lean() >= 0.8 * out.lean {
            out.t80 = since;
        }
        if out.t95.is_nan() && bike.lean() >= 0.95 * out.lean {
            out.t95 = since;
        }
        if out.path80.is_nan() && bike.body().angular_velocity.y >= 0.8 * yaw_sum / n.max(1.0) {
            out.path80 = since;
        }
    }
    out
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_steer_audit() {
    let speeds = list("RIDE6_SPEEDS", &[10.0, 15.0, 20.0, 30.0, 40.0, 60.0]);
    let verbose = std::env::var_os("RIDE6_VERBOSE").is_some();
    let all = bikes();
    let mut table: Vec<Vec<(Steady, Flip)>> = vec![Vec::new(); speeds.len()];
    for (name, spec) in &all {
        for (i, &speed) in speeds.iter().enumerate() {
            let s = steady(spec, speed, 1.0, 10.0);
            let f = flip(spec, speed);
            if verbose {
                println!(
                    "{name} v{speed:.0}: {} R {:.0} m g {:.2} lean {:.0} / {:.0} (base {:.0}) deg heading {:.1} deg front slip {:.1} deg bar {:.1} deg use f {:.2} r {:.2} | flip t80 {:.2} t95 {:.2} lean {:.0} {}",
                    if s.fell { "FELL" } else { "ok" },
                    s.radius,
                    s.g,
                    s.lean.to_degrees(),
                    s.limit.to_degrees(),
                    s.base.to_degrees(),
                    s.heading.to_degrees(),
                    s.front_slip.to_degrees(),
                    s.steer.to_degrees(),
                    s.front_use,
                    s.rear_use,
                    f.t80,
                    f.t95,
                    f.lean.to_degrees(),
                    if f.fell { "FELL" } else { "" }
                );
            }
            table[i].push((s, f));
        }
    }
    println!("==== medians over {} bikes (full stick held 10 s; flip left to right after 5 s)", all.len());
    println!(
        "speed | falls | radius m | g | lean deg | limit deg | base deg | heading deg | f slip deg | bar deg | f use | r use | t80 s | t95 s | worst t80 | path t80 s"
    );
    for (i, &speed) in speeds.iter().enumerate() {
        let rows = &table[i];
        let ok: Vec<&Steady> = rows.iter().map(|r| &r.0).filter(|s| !s.fell).collect();
        let pick = |f: &dyn Fn(&Steady) -> f32| median(ok.iter().map(|s| f(s)).collect::<Vec<_>>());
        let flips: Vec<Flip> = rows.iter().map(|r| r.1).filter(|f| !f.fell).collect();
        let worst = flips.iter().map(|f| f.t80).filter(|t| t.is_finite()).fold(0.0_f32, f32::max);
        let stuck = flips.iter().filter(|f| !f.t80.is_finite()).count();
        println!(
            "{speed:>4.0} | {} turn, {} flip | {:>6.0} | {:.2} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.2} | {:.2} | {:.2} | {:.2} | {worst:.2} ({stuck} never)",
            rows.iter().filter(|r| r.0.fell).count(),
            rows.iter().filter(|r| r.1.fell).count(),
            pick(&|s| s.radius),
            pick(&|s| s.g),
            pick(&|s| s.lean.to_degrees()),
            pick(&|s| s.limit.to_degrees()),
            pick(&|s| s.base.to_degrees()),
            pick(&|s| s.heading.to_degrees()),
            pick(&|s| s.front_slip.to_degrees()),
            pick(&|s| s.steer.to_degrees()),
            pick(&|s| s.front_use),
            pick(&|s| s.rear_use),
            median(flips.iter().map(|f| f.t80).collect::<Vec<_>>()),
            median(flips.iter().map(|f| f.t95).collect::<Vec<_>>()),
        );
        println!(
            "       path t80 (yaw rate to 80 percent on the other side): median {:.2} s",
            median(flips.iter().map(|f| f.path80).collect::<Vec<_>>())
        );
    }
}

/// How a manoeuvre drives the stick at time `t`.
#[derive(Clone, Copy)]
enum Manoeuvre {
    Held(f32),
    Slalom(f32),
    Flick,
    Flip,
}

impl Manoeuvre {
    fn stick(self, t: f32) -> f32 {
        match self {
            Manoeuvre::Held(s) => s,
            Manoeuvre::Slalom(s) => match ((t / 1.2) as usize).is_multiple_of(2) {
                true => s,
                false => -s,
            },
            Manoeuvre::Flick => match ((t / 0.35) as usize).is_multiple_of(2) {
                true => 1.0,
                false => -1.0,
            },
            Manoeuvre::Flip => match ((t / 0.6) as usize).is_multiple_of(2) {
                true => 1.0,
                false => -1.0,
            },
        }
    }
    fn name(self) -> String {
        match self {
            Manoeuvre::Held(s) => format!("held {s:.1}"),
            Manoeuvre::Slalom(s) => format!("slalom {s:.1}"),
            Manoeuvre::Flick => "flick".to_string(),
            Manoeuvre::Flip => "flip 0.6 s".to_string(),
        }
    }
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_steer_sweep() {
    let speeds = list("RIDE6_SPEEDS", &[8.0, 12.0, 16.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]);
    let all = bikes();
    let manoeuvres = [
        Manoeuvre::Held(1.0),
        Manoeuvre::Held(0.8),
        Manoeuvre::Held(0.6),
        Manoeuvre::Held(0.45),
        Manoeuvre::Held(0.3),
        Manoeuvre::Slalom(1.0),
        Manoeuvre::Flick,
        Manoeuvre::Flip,
    ];
    println!("==== falls over {} bikes in 10 s", all.len());
    let mut total = 0;
    let only = std::env::var("RIDE6_ONLY").unwrap_or_default();
    for m in manoeuvres {
        if !only.is_empty() && !only.split(',').any(|part| m.name().starts_with(part)) {
            continue;
        }
        let mut line = format!("{:>11}:", m.name());
        for &speed in &speeds {
            let mut fell = 0;
            let mut names = Vec::new();
            for (name, spec) in &all {
                let ground = FlatGround::new(0.0);
                let mut bike = Bike::new(spec.clone());
                assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
                for step in 0..(10.0 / FIXED_STEP) as usize {
                    let t = step as f32 * FIXED_STEP;
                    let v = bike.velocity().length();
                    bike.step(
                        FIXED_STEP,
                        &BikeInput { throttle: held_throttle(speed, v), steer: m.stick(t), ..Default::default() },
                        &ground,
                    );
                    if bike.fallen() {
                        fell += 1;
                        names.push(format!("{name}@{t:.1}s/{:?}", bike.fall_kind()));
                        break;
                    }
                }
            }
            total += fell;
            line += &format!(" {speed:.0}:{fell}");
            if fell > 0 && std::env::var_os("RIDE6_VERBOSE").is_some() {
                println!("  {} {speed:.0} m/s fell: {names:?}", m.name());
            }
        }
        println!("{line}");
    }
    println!("{total} falls in all");
}

/// `RIDE6_TRACE=1`: the first 1.5 s of a flip of the first bike at the first of `RIDE6_SPEEDS`, every 40 ms.
#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_steer_trace() {
    let speed = list("RIDE6_SPEEDS", &[20.0])[0];
    let all = bikes();
    let (name, spec) = &all[0];
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    println!(
        "{name} at {speed} m/s: t lean target limit | roll rate | bar deg | front force wanted / actual | slips f r | yaw rate | speed | front load, down | rear load"
    );
    for step in 0..(7.0 / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let v = bike.velocity().length();
        let stick = if t < 5.0 { -1.0 } else { 1.0 };
        bike.step(
            FIXED_STEP,
            &BikeInput { throttle: held_throttle(speed, v), steer: stick, ..Default::default() },
            &ground,
        );
        if t < 4.95 || ((t - 5.0) * 25.0).fract() > 0.04 && (t - 5.0) * 25.0 < 38.0 {
            if t < 4.95 || t > 6.6 {
                continue;
            }
            if (step % 3) != 0 {
                continue;
            }
        }
        let w = bike.wheel(0);
        let omega = bike.body().angular_velocity;
        let roll_rate = -omega.dot(bike.orientation() * Vec3::Z);
        println!(
            "{:.2} {:6.1} {:6.1} {:5.1} | {:6.2} | {:5.1} | {:7.0} / {:7.0} | {:5.1} {:5.1} | {:5.2} | {:.1} | {:.0} {} | {:.0}",
            t - 5.0,
            bike.lean().to_degrees(),
            bike.target_lean().to_degrees(),
            bike.lean_limit().to_degrees(),
            roll_rate,
            bike.steer_angle().to_degrees(),
            bike.wanted_front_force(),
            w.force.dot(w.side),
            w.slip_angle.to_degrees(),
            bike.wheel(1).slip_angle.to_degrees(),
            omega.y,
            v,
            w.load,
            w.on_ground,
            bike.wheel(1).load
        );
    }
}
