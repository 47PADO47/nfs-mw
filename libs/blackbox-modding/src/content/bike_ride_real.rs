//! A real imported bike ridden on flat ground: `RIDE6_CAR=C:/r6/final/mods/ride6-bikes/cars/ktm450.toml cargo test -p
//! blackbox-modding --features host --lib bike_ride_real -- --ignored --nocapture`. A diagnostic, not a regression test: it prints
//! what a corner at some speed does to the bike.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::parse_car;

fn spec() -> Option<BikeSpec> {
    let path = std::env::var_os("RIDE6_CAR")?;
    let text = std::fs::read_to_string(&path).expect("a readable manifest");
    let manifest = parse_car("ktm450", &text).expect("a valid manifest");
    Some(BikeSpec::from(manifest.bike.as_ref().expect("a bike")))
}

/// `RIDE6_KNOBS=a,b`: switch a term of the model off to see what it did (shared with the sweeps).
pub(super) fn apply_knobs(spec: &mut BikeSpec) {
    for knob in std::env::var("RIDE6_KNOBS").unwrap_or_default().split(',') {
        match knob {
            "nogyro" => (spec.gyro_roll_scale, spec.gyro_yaw_scale) = (0.0, 0.0),
            "nohang" => (spec.rider.lean_influence, spec.rider.weight_influence) = (0.0, 0.0),
            "nohelp" => spec.steering.lean.help_limit = 0.0,
            "noskid" => spec.steering.lean.skid_reduction.clear(),
            "tcs" => {
                if let Some(t) = spec.tcs.as_mut() {
                    t.slip = [0.03, 0.1];
                }
            }
            "norider" => spec.chassis.rider_mass = 0.0,
            "nohelpspeed" => spec.steering.lean.help_speed.clear(),
            "grip2" => {
                spec.front.tyre.a[2] *= 1.3;
                spec.rear.tyre.a[2] *= 1.3;
            }
            "noengbrake" => spec.engine.friction_torque *= 0.1,
            "stiff" => spec.steering.response *= 4.0,
            "nostability" => spec.steering.lean.stability.speed.clear(),
            "nofall" => spec.fall.lean = 10.0,
            _ => {}
        }
    }
}

#[test]
#[ignore = "needs RIDE6_CAR"]
fn bike_ride_real() {
    let Some(spec) = spec() else { return };
    for (speed, steer, throttle) in
        [(10.0, 0.3, 0.2), (15.0, 0.5, 0.2), (20.0, 0.5, 0.2), (25.0, 0.5, 0.3), (30.0, 0.3, 0.3)]
    {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec.clone());
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
        let input = BikeInput { throttle, steer, ..Default::default() };
        let mut fell_at = None;
        let mut max_lean = 0.0_f32;
        let (mut yaw0, mut yaw1) = (0.0, 0.0);
        for step in 0..(8.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &input, &ground);
            max_lean = max_lean.max(bike.lean().abs());
            if bike.fallen() && fell_at.is_none() {
                fell_at = Some(step as f32 * FIXED_STEP);
            }
            let travel = bike.body().linear_velocity;
            let yaw = travel.x.atan2(travel.z);
            if step == (2.0 / FIXED_STEP) as usize {
                yaw0 = yaw;
            }
            yaw1 = yaw;
        }
        let v = bike.body().linear_velocity;
        let rate = (yaw1 - yaw0) / 6.0;
        let radius = v.length() / rate.abs().max(1e-3);
        println!(
            "v0 {speed:4.1} steer {steer:.1}: fell {fell_at:?} max lean {:.2} rad (target {:.2}, limit {:.2}) v {:.1} yawrate {rate:.3} radius {radius:.0} m",
            max_lean,
            bike.target_lean(),
            bike.lean_limit(),
            v.length()
        );
    }
}

#[test]
#[ignore = "needs RIDE6_CAR"]
fn bike_launch_real() {
    let Some(spec) = spec() else { return };
    for (throttle, brake, steer, weight) in
        [(1.0, 0.0, 0.0, 0.0), (1.0, 0.0, 0.5, 0.0), (0.5, 0.0, 0.0, 0.0), (0.0, 1.0, 0.0, 0.0)]
    {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec.clone());
        let start = if brake > 0.0 { 25.0 } else { 0.0 };
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
        let input = BikeInput { throttle, front_brake: brake, rear_brake: brake, steer };
        bike.set_weight(weight);
        let mut report = String::new();
        for step in 0..(8.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &input, &ground);
            if step % 60 == 59 {
                report += &format!(
                    " [{:.1}s v{:.1} pitch{:.2} lean{:.2} gear{} rpm{:.0}{}]",
                    (step + 1) as f32 * FIXED_STEP,
                    bike.body().linear_velocity.length(),
                    bike.pitch(),
                    bike.lean(),
                    bike.gear(),
                    bike.rpm(),
                    if bike.fallen() { " FALLEN" } else { "" }
                );
            }
        }
        println!("throttle {throttle} brake {brake} steer {steer}:{report}");
    }
}

/// A held corner traced second by second: `RIDE6_TRACE=speed,steer[,step]`.
#[test]
#[ignore = "needs RIDE6_CAR and RIDE6_TRACE"]
fn bike_trace_real() {
    let Some(spec) = spec() else { return };
    let Ok(arg) = std::env::var("RIDE6_TRACE") else { return };
    let parts: Vec<f32> = arg.split(',').filter_map(|p| p.parse().ok()).collect();
    let (speed, steer) = (parts[0], parts[1]);
    let ground = FlatGround::new(0.0);
    let mut spec = spec;
    apply_knobs(&mut spec);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    for step in 0..(6.0 / FIXED_STEP) as usize {
        let v = bike.body().linear_velocity.length();
        let t = step as f32 * FIXED_STEP;
        let mut throttle = (0.25 + 0.2 * (speed - v)).clamp(0.0, 1.0);
        let mut lever = 0.0;
        match std::env::var("RIDE6_SCENARIO").as_deref() {
            Ok("trail") if (3.0..5.0).contains(&t) => {
                (throttle, lever) = (0.0, std::env::var("RIDE6_LEVER").ok().and_then(|v| v.parse().ok()).unwrap_or(0.5))
            }
            Ok("launch") => throttle = 1.0,
            _ => {}
        }
        let share: f32 = std::env::var("RIDE6_REARSHARE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let input = BikeInput { throttle, front_brake: lever, rear_brake: lever * share, steer };
        bike.step(FIXED_STEP, &input, &ground);
        let from: usize = std::env::var("RIDE6_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        let every: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
        if step >= from * 60 && step % every == every - 1 {
            let f = bike.wheel(0);
            let r = bike.wheel(1);
            println!(
                "t{:.2} v{:.1} lean{:.3} tgt{:.3} lim{:.3} steer{:.3} fAng{:.3} rAng{:.3} fLoad{:.0} rLoad{:.0} rSlip{:.3} fCam{:.2} fFy{:.0} fall{:.3} yaw{:.3} fcomp{:.4} fcs{:.2} pitch{:.3} gear{} want{:.0} rFy{:.0} {}",
                (step + 1) as f32 * FIXED_STEP,
                v,
                bike.lean(),
                bike.target_lean(),
                bike.lean_limit(),
                bike.steer_angle(),
                f.slip_angle,
                r.slip_angle,
                f.load,
                r.load,
                r.slip_ratio,
                f.camber,
                f.force.x,
                bike.fall_lean_limit(),
                bike.body().angular_velocity.y,
                f.compression,
                f.compression_speed,
                bike.pitch(),
                bike.gear(),
                bike.wanted_front_force(),
                r.force.x,
                if bike.fallen() { "FALLEN" } else { "" }
            );
        }
    }
}

/// The first seconds of a start traced: `RIDE6_THROTTLE=1.0`.
#[test]
#[ignore = "needs RIDE6_CAR"]
fn bike_launch_trace_real() {
    let Some(spec) = spec() else { return };
    let throttle: f32 = std::env::var("RIDE6_THROTTLE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    bike.set_weight(std::env::var("RIDE6_WEIGHT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0));
    for step in 0..(12.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
        let from: usize = std::env::var("RIDE6_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        let every: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
        if step >= from * 60 && step % every == every - 1 {
            println!(
                "t{:.2} v{:.1} gear{} rpm{:.0} front[load{:.0}] rear[slip{:.3} load{:.0} fz{:.0} spin{:.1} comp{:.3}] pitch{:.3} ",
                (step + 1) as f32 * FIXED_STEP,
                bike.body().linear_velocity.length(),
                bike.gear(),
                bike.rpm(),
                bike.wheel(0).load,
                bike.wheel(1).slip_ratio,
                bike.wheel(1).load,
                bike.wheel(1).force.z,
                bike.wheel(1).spin,
                bike.wheel(1).compression,
                bike.pitch()
            );
        }
    }
}

/// The lateral force of the front and rear tyres against the slip angle at several loads and cambers (N): what the tyres can give.
#[test]
#[ignore = "needs RIDE6_CAR"]
fn bike_tyre_curve_real() {
    let Some(spec) = spec() else { return };
    for (name, wheel) in [("front", &spec.front), ("rear", &spec.rear)] {
        for load in [600.0_f32, 900.0, 1300.0, 2000.0] {
            for camber_deg in [0.0_f32, 25.0, 50.0, 60.0] {
                // The wheel feeds the formula the negated camber (a bike leaning right is pushed right).
                let curve: Vec<f32> = [0.5_f32, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 12.0]
                    .iter()
                    .map(|&alpha| -wheel.tyre.lateral(load * 1e-3, alpha, -camber_deg))
                    .collect();
                let best = (0..400)
                    .map(|n| -wheel.tyre.lateral(load * 1e-3, n as f32 * 0.05, -camber_deg))
                    .fold(f32::MIN, f32::max);
                let zero = -wheel.tyre.lateral(load * 1e-3, 0.0, -camber_deg);
                println!(
                    "{name} load {load:4.0} camber {camber_deg:2.0}: Fy(0) {zero:6.0} peak {best:6.0} at alpha 0.5,1,2,3,4,5,6,8,12 deg: {:?}",
                    curve.iter().map(|f| f.round() as i32).collect::<Vec<_>>()
                );
            }
        }
    }
}
