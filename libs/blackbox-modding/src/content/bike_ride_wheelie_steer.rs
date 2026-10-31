//! Steering with the front wheel up (`docs/specs/ride6-wheelie-steer.md`), ridden on the imported bikes of a folder
//! (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b]`) as ignored tests:
//!
//! - `bike_wheelie_steer`: a wheelie is started (full throttle, the weight lever back) and held to a pitch, then the stick is held
//!   at `RIDE6_STICKS` for `RIDE6_SECONDS` while the wheelie goes on. Prints the heading change, the pitch kept, the lean and how
//!   many bikes fall, against the same stick on the road with the front wheel down.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec, choices};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::{held_throttle, specs};

fn list(key: &str, default: &[f32]) -> Vec<f32> {
    match std::env::var(key) {
        Ok(text) => text.split(',').filter_map(|p| p.parse().ok()).collect(),
        Err(_) => default.to_vec(),
    }
}

/// What one ride measured.
#[derive(Clone, Debug, Default)]
pub(super) struct Turn {
    /// Whether the bike reached the wheelie before the stick (always true on the road).
    pub(super) lifted: bool,
    pub(super) fell: Option<f32>,
    /// The heading change over the stick's time (rad, positive to the right) and the time the front wheel was up in it (share).
    pub(super) heading: f32,
    pub(super) front_up: f32,
    pub(super) mean_pitch: f32,
    pub(super) max_lean: f32,
    pub(super) speed: f32,
}

/// The heading of the bike (rad, positive to the right) seen from above.
fn yaw(bike: &Bike) -> f32 {
    let forward = bike.orientation() * glam::Vec3::Z;
    forward.x.atan2(forward.z)
}

/// Ride `spec` from `start` m/s: a wheelie to `pitch` rad (or, with `wheelie` false, the road at the same speed with the throttle that holds
/// it), then the stick held at `stick` for `seconds`. The stick goes through the share the game gives it by speed.
pub(super) fn ride(
    spec: &BikeSpec,
    start: f32,
    wheelie: bool,
    pitch: f32,
    stick: f32,
    seconds: f32,
    lever: f32,
) -> Turn {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
    let mut out = Turn::default();
    let (mut heading0, mut time, mut up, mut sum_pitch) = (0.0, 0.0_f32, 0.0_f32, 0.0_f32);
    let mut began = false;
    let verbose = std::env::var_os("RIDE6_VERBOSE").is_some();
    for step in 0..((8.0 + seconds) / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        if !began && (!wheelie || bike.pitch() >= pitch) && t > 0.5 {
            began = true;
            out.lifted = bike.pitch() >= pitch || !wheelie;
            heading0 = yaw(&bike);
        }
        if !began && t > 8.0 {
            return out;
        }
        let share = choices::stick_share(bike.forward_speed());
        let steer = if began { stick * share } else { 0.0 };
        let throttle = match wheelie {
            true => 1.0,
            false => held_throttle(start, bike.forward_speed()),
        };
        bike.set_weight(if wheelie { lever } else { 0.0 });
        bike.step(FIXED_STEP, &BikeInput { throttle, steer, ..Default::default() }, &ground);
        if bike.fallen() {
            out.fell = Some(t);
            return out;
        }
        if !began {
            continue;
        }
        time += FIXED_STEP;
        up += f32::from(u8::from(!bike.wheel(0).on_ground)) * FIXED_STEP;
        sum_pitch += bike.pitch() * FIXED_STEP;
        out.max_lean = out.max_lean.max(bike.lean().abs());
        out.heading = yaw(&bike) - heading0;
        out.speed = bike.forward_speed();
        if verbose && step % 12 == 0 {
            println!(
                "  t{time:.2} v{:.1} pitch{:.2} lean{:.2}/{:.2} tgt{:.2} steer{:.3} yawrate{:.2} f{} want{:.0} gear{} rearslip{:.3} rearFy{:.0} rearload{:.0} vdir{:.1}",
                bike.forward_speed(),
                bike.pitch(),
                bike.lean(),
                bike.lean_limit(),
                bike.target_lean(),
                bike.steer_angle(),
                bike.body().angular_velocity.y,
                u8::from(bike.wheel(0).on_ground),
                bike.wanted_front_force(),
                bike.gear(),
                bike.wheel(1).slip_angle,
                bike.wheel(1).force.dot(bike.orientation() * glam::Vec3::X),
                bike.wheel(1).load,
                (bike.velocity().x.atan2(bike.velocity().z) - yaw(&bike)).to_degrees()
            );
        }
        if time >= seconds {
            break;
        }
    }
    out.front_up = up / time.max(1e-3);
    out.mean_pitch = sum_pitch / time.max(1e-3);
    out
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_wheelie_steer() {
    let mut bikes = specs();
    // `RIDE6_CUT=<rad>` replaces the wheelie spec's lean limit (the data's 1 degree); `choices::WHEELIE_LEAN_FLOOR` still holds it up.
    if let Some(cut) = list("RIDE6_CUT", &[]).first().copied() {
        for (_, spec) in &mut bikes {
            if let Some(assist) = spec.wheelie.as_mut() {
                assist.wheelie.lean_limit = cut;
            }
        }
    }
    let seconds = list("RIDE6_SECONDS", &[3.0])[0];
    let lever = list("RIDE6_LEVER", &[-1.0])[0];
    for start in list("RIDE6_SPEEDS", &[8.0, 15.0]) {
        for degrees in list("RIDE6_PITCHES", &[20.0]) {
            for stick in list("RIDE6_STICKS", &[0.5, 1.0]) {
                let (mut wheelie_heading, mut road_heading, mut counted) = (0.0, 0.0, 0);
                let (mut fell, mut lifted, mut pitch_sum, mut lean) = (Vec::new(), 0, 0.0, 0.0_f32);
                for (name, spec) in &bikes {
                    let up = ride(spec, start, true, degrees.to_radians(), stick, seconds, lever);
                    let road = ride(spec, start, false, 0.0, stick, seconds, 0.0);
                    if up.fell.is_some() {
                        fell.push(name.clone());
                    }
                    if !up.lifted || up.fell.is_some() {
                        continue;
                    }
                    lifted += 1;
                    pitch_sum += up.mean_pitch;
                    lean = lean.max(up.max_lean);
                    if road.fell.is_none() {
                        counted += 1;
                        wheelie_heading += up.heading;
                        road_heading += road.heading;
                    }
                    if std::env::var_os("RIDE6_EACH").is_some() {
                        println!(
                            "    {name}: wheelie heading {:.1} deg road {:.1} deg pitch {:.0} deg lean {:.2}",
                            up.heading.to_degrees(),
                            road.heading.to_degrees(),
                            up.mean_pitch.to_degrees(),
                            up.max_lean
                        );
                    }
                }
                let n = counted.max(1) as f32;
                println!(
                    "wheelie steer from {start:.0} m/s to {degrees:.0} deg, stick {stick}, {seconds} s: {lifted} of {} lift; mean heading change {:.1} deg \
                     in the wheelie against {:.1} deg on the road; mean pitch {:.0} deg; largest lean {lean:.2}; {} fall {:?}",
                    bikes.len(),
                    (wheelie_heading / n).to_degrees(),
                    (road_heading / n).to_degrees(),
                    (pitch_sum / lifted.max(1) as f32).to_degrees(),
                    fell.len(),
                    fell.iter().take(4).collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_rear_camber_thrust() {
    for (name, spec) in specs() {
        let tyre = &spec.rear.tyre;
        for gamma in [0.0_f32, 10.0, 25.0, 40.0, 50.0] {
            let load = 2.6;
            println!(
                "{name}: rear at {load} kN camber {gamma} deg: slip 0 -> {:.0} N, slip -1 deg -> {:.0}, +1 deg {:.0}; peak {:.0}",
                tyre.lateral(load, 0.0, gamma),
                tyre.lateral(load, -1.0, gamma),
                tyre.lateral(load, 1.0, gamma),
                tyre.lateral_peak(load)
            );
        }
    }
}
