//! Whole-car handling numbers with real data on flat ground.

use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle};
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::PhysicsData;

use crate::install;

fn car(name: &str) -> Option<Vehicle> {
    let dir = install()?;
    let data = PhysicsData::load(&dir).unwrap();
    Some(Vehicle::new(data.car(name).ok()?.spec))
}

fn report(name: &str) {
    let Some(mut v) = car(name) else { return };
    let g = FlatGround::new(0.0);
    let spec = v.spec().clone();
    println!("gravity {} drag {:?}", spec.body.gravity, spec.body.drag);
    println!(
        "engine braking {:?} shift {} clutch {}",
        spec.engine.engine_braking, spec.transmission.shift_speed, spec.transmission.clutch_slip
    );
    if false {
        println!(
            "tires {:?}
brakes {:?}
engine torque {:?} idle {} red {}
gears {:?} final {} split {} diff {:?}
aero {:?}
chassis {:?}",
            spec.tires,
            spec.brakes,
            spec.engine.torque,
            spec.engine.idle,
            spec.engine.red_line,
            spec.transmission.gear_ratio,
            spec.transmission.final_gear,
            spec.transmission.torque_split,
            spec.transmission.differential,
            spec.aero,
            spec.chassis
        );
    }
    println!("== {name}: {} kg, dim {:?}, wheelbase {:.2}", spec.mass, spec.dimension, spec.chassis.wheel_base);
    v.place_on_ground(&g, 0.0, 0.0, 5.0, 0.0);
    for _ in 0..180 {
        v.step(FIXED_STEP, &InputState::default(), &g);
    }
    println!("rest: y {:.3} wheels {}", v.position().y, v.wheels_on_ground());
    let throttle = InputState { throttle: 1.0, ..Default::default() };
    let (mut t60, mut t100, mut t200, mut gears) = (None, None, None, vec![v.gear()]);
    for i in 0..(40.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &throttle, &g);
        let t = i as f32 * FIXED_STEP;
        let kmh = v.forward_speed() * 3.6;
        if t60.is_none() && kmh > 96.6 {
            t60 = Some(t);
        }
        if t100.is_none() && kmh > 160.9 {
            t100 = Some(t);
        }
        if t200.is_none() && kmh > 200.0 {
            t200 = Some(t);
        }
        if *gears.last().unwrap() != v.gear() {
            gears.push(v.gear());
        }
    }
    println!(
        "0-60mph {t60:?}  0-100mph {t100:?}  0-200kmh {t200:?}  top after 40 s {:.0} km/h gears {gears:?}",
        v.forward_speed() * 3.6
    );
    // Braking from 100 km/h.
    let mut b = Vehicle::new(spec.clone());
    b.config.auto_reverse = false;
    b.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, 27.8);
    for _ in 0..30 {
        b.step(FIXED_STEP, &InputState::default(), &g);
    }
    let z0 = b.position().z;
    let v0 = b.forward_speed();
    let mut t = 0.0;
    while b.forward_speed() > 0.3 && t < 10.0 {
        b.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &g);
        t += FIXED_STEP;
    }
    assert!(b.position().z - z0 > 12.0 && b.position().z - z0 < 50.0, "{name}: braking {} m", b.position().z - z0);
    println!("brake from {:.1} m/s: {:.1} m in {t:.2} s", v0, b.position().z - z0);
    // Coasting from 40 m/s.
    let mut d = Vehicle::new(spec.clone());
    d.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, 40.0);
    for _ in 0..30 {
        d.step(FIXED_STEP, &InputState::default(), &g);
    }
    let (v1, g1) = (d.forward_speed(), d.gear());
    for _ in 0..60 {
        d.step(FIXED_STEP, &InputState::default(), &g);
    }
    println!(
        "coast: {:.1} -> {:.1} m/s in 1 s (gear {g1}, rpm {:.0}) decel {:.2} m/s2",
        v1,
        d.forward_speed(),
        d.rpm(),
        v1 - d.forward_speed()
    );
    // A jump: 50 m/s, 1.5 m up, over flat ground.
    let mut j = Vehicle::new(spec.clone());
    j.place_moving(Vec3::new(0.0, 2.3, 0.0), Quat::IDENTITY, 50.0);
    let mut t = 0.0;
    let z0 = j.position().z;
    while j.wheels_on_ground() == 0 && t < 5.0 {
        j.step(FIXED_STEP, &InputState { throttle: 1.0, ..Default::default() }, &g);
        t += FIXED_STEP;
    }
    println!(
        "fall 1.5 m at 50 m/s: {t:.2} s (ballistic {:.2} s), {:.1} m",
        (2.0 * 1.5 / 9.81f32).sqrt(),
        j.position().z - z0
    );
    // Skidpad: steady lateral accel at 20 m/s, steer 1.
    let mut c = Vehicle::new(spec);
    c.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, 20.0);
    let mut peak = 0.0f32;
    for i in 0..(8.0 / FIXED_STEP) as usize {
        c.step(FIXED_STEP, &InputState { throttle: 0.5, steer: 1.0, ..Default::default() }, &g);
        if i > 120 {
            peak = peak.max(c.speed() * c.angular_velocity().y.abs());
        }
    }
    assert!(peak / 9.81 > 0.9 && peak / 9.81 < 2.6, "{name}: cornering {} g", peak / 9.81);
    println!("full-lock lateral accel peak {:.1} m/s2 ({:.2} g), final speed {:.1}", peak, peak / 9.81, c.speed());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn handling_report() {
    for n in ["BMWM3GTR", "PORSCHE911", "CORVETTE", "CAMARO", "MUSTANGGT", "CLK500", "TRAFFICPIZZA"] {
        report(n);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn shift_trace() {
    let Some(mut v) = car("BMWM3GTR") else { return };
    let g = FlatGround::new(0.0);
    v.place_on_ground(&g, 0.0, 0.0, 5.0, 0.0);
    for _ in 0..180 {
        v.step(FIXED_STEP, &InputState::default(), &g);
    }
    let mut last = v.forward_speed();
    let mut gear = v.gear();
    let mut until = 0;
    for i in 0..600 {
        v.step(FIXED_STEP, &InputState { throttle: 1.0, ..Default::default() }, &g);
        if v.gear() != gear {
            gear = v.gear();
            until = i + 40;
            println!("--- shift to {gear} at t={:.2}", i as f32 / 60.0);
        }
        if i < until {
            let a = (v.forward_speed() - last) * 60.0;
            println!(
                "t {:.2} gear {} rpm {:5.0} accel {:5.2} drive_torque {:6.0} clutch {:?}",
                i as f32 / 60.0,
                v.gear(),
                v.rpm(),
                a,
                v.powertrain().drive_torque(),
                v.powertrain().clutch.state
            );
        }
        last = v.forward_speed();
    }
}
