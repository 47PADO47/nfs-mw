//! Numbers to compare with a capture of the original game (tire loads, forces and slip angles).

use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle};
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::PhysicsData;

use crate::install;

fn row(v: &Vehicle) -> String {
    let w: Vec<_> = (0..4).map(|i| v.wheel(i)).collect();
    format!(
        "v {:5.1} load F {:6.0} R {:6.0} total {:6.0} | lat F {:7.0} R {:7.0} | slip deg F {:+5.1} R {:+5.1} | steer {:+.3} {:+.3}",
        v.forward_speed(),
        w[0].load + w[1].load,
        w[2].load + w[3].load,
        w.iter().map(|w| w.load).sum::<f32>(),
        w[0].lateral_force + w[1].lateral_force,
        w[2].lateral_force + w[3].lateral_force,
        w[0].slip_angle.to_degrees(),
        w[2].slip_angle.to_degrees(),
        w[0].steer_angle,
        w[1].steer_angle
    )
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn compare_with_original() {
    let Some(dir) = install() else { return };
    let name = std::env::var("NFSMW_CAR").unwrap_or_else(|_| "BMWM3GTR".into());
    let data = PhysicsData::load(&dir).unwrap();
    let spec = data.car(&name).unwrap().spec;
    println!("== {name}: {} kg", spec.mass);
    let g = FlatGround::new(0.0);
    let mut v = Vehicle::new(spec.clone());
    v.place_on_ground(&g, 0.0, 0.0, 5.0, 0.0);
    for _ in 0..180 {
        v.step(FIXED_STEP, &InputState::default(), &g);
    }
    println!("rest      {}", row(&v));
    for speed in [10.0f32, 20.0, 30.0, 40.0] {
        let mut c = Vehicle::new(spec.clone());
        c.config.auto_reverse = false;
        c.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, speed);
        for _ in 0..90 {
            c.step(FIXED_STEP, &InputState { throttle: 0.2, ..Default::default() }, &g);
        }
        println!("straight  {}", row(&c));
        for steer in [0.25f32, 0.5, 1.0] {
            let mut t = c.clone();
            let mut max_ay = 0.0f32;
            for i in 0..240 {
                t.step(FIXED_STEP, &InputState { throttle: 0.3, steer, ..Default::default() }, &g);
                if i > 100 {
                    max_ay = max_ay.max(t.speed() * t.angular_velocity().y.abs());
                }
            }
            println!("steer {steer:>4} {}  ay {:.2} g", row(&t), max_ay / 9.81);
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn steering_lock_by_speed() {
    let Some(dir) = install() else { return };
    let name = std::env::var("NFSMW_CAR").unwrap_or_else(|_| "BMWM3GTR".into());
    let spec = PhysicsData::load(&dir).unwrap().car(&name).unwrap().spec;
    let g = FlatGround::new(0.0);
    for speed in [5.0f32, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0] {
        let mut c = Vehicle::new(spec.clone());
        c.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, speed);
        let mut last = (0.0, 0.0);
        for _ in 0..30 {
            c.step(FIXED_STEP, &InputState { throttle: 1.0, steer: 1.0, ..Default::default() }, &g);
            let a = c.wheel_angles();
            last = (a.left, a.right);
        }
        println!(
            "lock at {speed:>4} m/s (actual {:.1}): left {:.3} right {:.3} rad",
            c.forward_speed(),
            last.0,
            last.1
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn steering_step_response() {
    let Some(dir) = install() else { return };
    let name = std::env::var("NFSMW_CAR").unwrap_or_else(|_| "BMWM3GTR".into());
    let spec = PhysicsData::load(&dir).unwrap().car(&name).unwrap().spec;
    let g = FlatGround::new(0.0);
    for speed in [5.0f32, 15.0, 30.0] {
        let mut c = Vehicle::new(spec.clone());
        c.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, speed);
        for _ in 0..30 {
            c.step(FIXED_STEP, &InputState { throttle: 1.0, ..Default::default() }, &g);
        }
        let mut line = format!("speed {speed:>4} step to full lock, right wheel deg every 3 frames:");
        for i in 0..40 {
            c.step(FIXED_STEP, &InputState { throttle: 1.0, steer: 1.0, ..Default::default() }, &g);
            if i % 3 == 2 {
                line += &format!(" {:.0}", c.wheel_angles().right.to_degrees());
            }
        }
        println!("{line}");
        let mut line = String::from("   release to centre:");
        for i in 0..40 {
            c.step(FIXED_STEP, &InputState { throttle: 1.0, steer: 0.0, ..Default::default() }, &g);
            if i % 3 == 2 {
                line += &format!(" {:.0}", c.wheel_angles().right.to_degrees());
            }
        }
        println!("{line}");
    }
}

/// One row of a capture of the original game (`orig_drive.csv`, see the replay test).
#[derive(Clone, Copy, Default)]
struct Sample {
    t: f32,
    steer: [f32; 2],
    drive: f32,
    brake: f32,
    road: f32,
    lat_f: f32,
    lat_r: f32,
    load: f32,
}

fn read_capture(path: &str) -> Vec<Sample> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = |name: &str| header.iter().position(|h| *h == name).unwrap();
    let (t, w0, w1) = (col("t"), col("steerW0"), col("steerW1"));
    let (d2, d3, b0, b1) = (col("drive2"), col("drive3"), col("brake0"), col("brake1"));
    let road: Vec<usize> = (0..4).map(|i| col(&format!("road{i}"))).collect();
    let lat: Vec<usize> = (0..4).map(|i| col(&format!("lat{i}"))).collect();
    let load: Vec<usize> = (0..4).map(|i| col(&format!("load{i}"))).collect();
    lines
        .filter_map(|l| {
            let v: Vec<f32> = l.split(',').map(|x| x.parse().unwrap_or(f32::NAN)).collect();
            (v.len() == header.len() && v.iter().all(|x| x.is_finite())).then(|| Sample {
                t: v[t],
                steer: [v[w0], v[w1]],
                drive: v[d2] + v[d3],
                brake: v[b0] + v[b1],
                road: road.iter().map(|&i| v[i]).sum::<f32>() / 4.0,
                lat_f: v[lat[0]] + v[lat[1]],
                lat_r: v[lat[2]] + v[lat[3]],
                load: load.iter().map(|&i| v[i]).sum(),
            })
        })
        .collect()
}

/// `NFSMW_CAPTURE=orig_drive.csv NFSMW_FROM=60 NFSMW_TO=75 NFSMW_CAR=911GT2`: drive our car with the wheel
/// angles, throttle and brake of a capture and print both sides' speed, lateral forces and load.
#[test]
#[ignore = "needs the game and a capture of the original (NFSMW_CAPTURE)"]
fn replay_capture() {
    let Some(dir) = install() else { return };
    let Ok(path) = std::env::var("NFSMW_CAPTURE") else { return };
    let num = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let (from, to) = (num("NFSMW_FROM", 60.0), num("NFSMW_TO", 70.0));
    let name = std::env::var("NFSMW_CAR").unwrap_or_else(|_| "911GT2".into());
    let spec = PhysicsData::load(&dir).unwrap().car(&name).unwrap().spec;
    let cap: Vec<Sample> = read_capture(&path).into_iter().filter(|s| s.t >= from && s.t <= to).collect();
    assert!(cap.len() > 10, "no samples between {from} s and {to} s");
    let at = |t: f32| *cap.iter().min_by(|a, b| (a.t - t).abs().total_cmp(&(b.t - t).abs())).unwrap();
    let g = FlatGround::new(0.0);
    let mut v = Vehicle::new(spec);
    v.config.auto_reverse = false;
    v.place_moving(Vec3::new(0.0, 0.8, 0.0), Quat::IDENTITY, cap[0].road);
    println!("t      | original: speed lat F  lat R  load  | ours: speed lat F  lat R  load  | wheel deg  gas brake");
    let steps = ((to - from) / FIXED_STEP) as usize;
    for i in 0..steps {
        let t = from + i as f32 * FIXED_STEP;
        let s = at(t);
        let (gas, brake) = (f32::from(u8::from(s.drive > 800.0)), f32::from(u8::from(s.brake.abs() > 800.0)));
        v.forced_wheel_angles = Some(blackbox_vehicle::steering::WheelAngles { left: s.steer[0], right: s.steer[1] });
        v.step(FIXED_STEP, &InputState { throttle: gas, brake, ..Default::default() }, &g);
        if i % 30 == 0 {
            let w: Vec<_> = (0..4).map(|k| v.wheel(k)).collect();
            println!(
                "{t:6.2} | {:6.1} {:7.0} {:7.0} {:6.0} | {:6.1} {:7.0} {:7.0} {:6.0} | {:+5.1} {gas:.0} {brake:.0}",
                s.road,
                s.lat_f,
                s.lat_r,
                s.load,
                v.forward_speed(),
                w[0].lateral_force + w[1].lateral_force,
                w[2].lateral_force + w[3].lateral_force,
                w.iter().map(|w| w.load).sum::<f32>(),
                s.steer[0].to_degrees()
            );
        }
    }
}
