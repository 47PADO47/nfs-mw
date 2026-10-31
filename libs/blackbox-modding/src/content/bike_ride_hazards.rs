//! Diagnostics for what the players reported after round 8 (`docs/specs/ride6-feel-round9.md`, sections 4 to 7), ridden on the
//! imported bikes of a folder (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b]`), as ignored tests:
//!
//! - `bike_square_stick`: the stick thrown from side to side as a square wave (a pad flicked left-right), at several speeds,
//!   frequencies and amplitudes;
//! - `bike_curb`: a front tyre meeting a kerb-high step straight on and at angles, at several speeds;
//! - `bike_wheelie_landing`: a wheelie held to a pitch and then let down, with and without a little stick.
//!
//! Each prints how many bikes of the folder fall in every configuration, and the names of the first few that do.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, Ground, GroundHit, SurfaceGrip};
use glam::Vec3;

use super::bike_ride_sweep::{held_throttle, run, specs};

/// A road with a step across it: beyond the line whose unit normal on the ground is `(sin angle, cos angle)`, at distance `distance`
/// from the origin, the ground is `step` high; the rise takes `bevel` metres across the line (0 for a vertical face, as a vertical ray
/// sees it), and the face of the bevel has its own normal, as a collision mesh gives it.
struct StepGround {
    step: f32,
    angle: f32,
    distance: f32,
    bevel: f32,
}

impl StepGround {
    fn flat() -> Self {
        Self { step: 0.0, angle: 0.0, distance: 0.0, bevel: 0.0 }
    }

    /// The height and the normal at `(x, z)`.
    fn at(&self, x: f32, z: f32) -> (f32, Vec3) {
        let across = x * self.angle.sin() + z * self.angle.cos() - self.distance;
        if self.bevel <= 0.0 || across >= self.bevel {
            return (if across >= 0.0 { self.step } else { 0.0 }, Vec3::Y);
        }
        if across < 0.0 {
            return (0.0, Vec3::Y);
        }
        let slope = self.step / self.bevel;
        let toward = Vec3::new(self.angle.sin(), 0.0, self.angle.cos());
        (across * slope, (Vec3::Y - toward * slope).normalize())
    }
}

impl Ground for StepGround {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        if dir.y >= -1e-6 {
            return None;
        }
        let (height, normal) = self.at(origin.x, origin.z);
        let distance = (height - origin.y) / dir.y;
        (distance >= 0.0 && distance <= max_distance).then_some(GroundHit {
            distance,
            normal,
            surface: SurfaceGrip::DEFAULT,
        })
    }
}

fn list(key: &str, default: &[f32]) -> Vec<f32> {
    match std::env::var(key) {
        Ok(text) => text.split(',').filter_map(|p| p.parse().ok()).collect(),
        Err(_) => default.to_vec(),
    }
}

/// Print a configuration's count of falls and the first few names.
fn report(label: &str, total: usize, fallen: &[String]) {
    let names: Vec<&str> = fallen.iter().take(4).map(String::as_str).collect();
    println!("{label}: {} of {total} fall {names:?}", fallen.len());
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_square_stick() {
    let bikes = specs();
    for speed in list("RIDE6_SPEEDS", &[20.0, 30.0, 40.0]) {
        for hz in list("RIDE6_HZ", &[0.5, 1.0, 2.0]) {
            for amplitude in list("RIDE6_AMPS", &[0.5, 1.0]) {
                let mut fallen = Vec::new();
                for (name, spec) in &bikes {
                    let wave = move |t: f32, bike: &Bike| {
                        let side = if (t * hz * 2.0) as usize % 2 == 0 { amplitude } else { -amplitude };
                        let v = bike.body().linear_velocity.length();
                        BikeInput { throttle: held_throttle(speed, v), steer: side, ..Default::default() }
                    };
                    if run(spec, speed, 10.0, 0.0, wave).1.fell.is_some() {
                        fallen.push(name.clone());
                    }
                }
                report(&format!("square stick {speed:.0} m/s {hz} Hz +-{amplitude}"), bikes.len(), &fallen);
            }
        }
    }
}

/// A bike coasting at `speed` towards a step of height `height` met at `angle` to the straight line; true when it stays up for 4 s.
fn crosses(spec: &BikeSpec, speed: f32, angle: f32, height: f32, bevel: f32) -> bool {
    let ground = StepGround { step: height, angle, distance: 6.0, bevel };
    let flat = StepGround::flat();
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&flat, 0.0, 0.0, 2.0, 0.0, speed));
    let mut worst = 0.0_f32;
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        let v = bike.body().linear_velocity.length();
        bike.step(FIXED_STEP, &BikeInput { throttle: held_throttle(speed, v).min(0.3), ..Default::default() }, &ground);
        worst = worst.max(bike.body().angular_velocity.length());
        if bike.fallen() {
            return false;
        }
    }
    if std::env::var_os("RIDE6_VERBOSE").is_some() {
        println!(
            "  crossed at {speed} m/s, {angle:.2} rad: height {:.2} m, worst angular speed {worst:.1} rad/s",
            bike.body().position.y
        );
    }
    true
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_curb() {
    let bikes = specs();
    for speed in list("RIDE6_SPEEDS", &[5.0, 10.0, 15.0, 25.0, 35.0]) {
        for degrees in list("RIDE6_ANGLES", &[0.0, 20.0, 45.0]) {
            for height in list("RIDE6_HEIGHTS", &[0.10, 0.15]) {
                for bevel in list("RIDE6_BEVELS", &[0.0, 0.03, 0.10]) {
                    let fallen: Vec<String> = bikes
                        .iter()
                        .filter(|(_, spec)| !crosses(spec, speed, degrees.to_radians(), height, bevel))
                        .map(|(name, _)| name.clone())
                        .collect();
                    report(
                        &format!("curb {speed:.0} m/s {degrees:.0} deg {height} m bevel {bevel}"),
                        bikes.len(),
                        &fallen,
                    );
                }
            }
        }
    }
}

/// A bike held in a wheelie to `pitch` (rad), then let down with the stick at `stick`; the result is whether it stays up for 4 s
/// after the wheelie is let go, and the largest pitch it reached.
fn landing(spec: &BikeSpec, pitch: f32, stick: f32, start: f32, during: bool) -> (bool, f32) {
    let flat = StepGround::flat();
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&flat, 0.0, 0.0, 2.0, 0.0, start));
    let (mut released, mut best, mut after, mut step) = (false, 0.0_f32, 0.0_f32, 0_usize);
    for _ in 0..(14.0 / FIXED_STEP) as usize {
        best = best.max(bike.pitch());
        released |= bike.pitch() >= pitch;
        let input = match released {
            false => BikeInput { throttle: 1.0, steer: if during { stick } else { 0.0 }, ..Default::default() },
            true => BikeInput { steer: stick, ..Default::default() },
        };
        bike.set_weight(if released { 1.0 } else { -1.0 });
        bike.step(FIXED_STEP, &input, &flat);
        step += 1;
        if std::env::var_os("RIDE6_VERBOSE").is_some() && step % 6 == 0 {
            println!(
                "  t{:.2} v{:.1} pitch{:.2} lean{:.2} limit{:.2} steer{:.2} want{:.0} front{} {}{}",
                step as f32 * FIXED_STEP,
                bike.body().linear_velocity.length(),
                bike.pitch(),
                bike.lean(),
                bike.lean_limit(),
                bike.steer_angle(),
                bike.wanted_front_force(),
                bike.wheel(0).on_ground,
                if released { "released " } else { "" },
                if bike.fallen() { format!("FELL {:?}", bike.fall_kind()) } else { String::new() }
            );
        }
        if released {
            after += FIXED_STEP;
        }
        if bike.fallen() {
            return (false, best);
        }
        if after > 4.0 {
            break;
        }
    }
    (true, best)
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_wheelie_landing() {
    let bikes = specs();
    // `RIDE6_START` is the speed the wheelie starts at (m/s), `RIDE6_DURING=1` holds the stick through the wheelie, not only after it.
    let start = list("RIDE6_START", &[0.0])[0];
    let during = std::env::var_os("RIDE6_DURING").is_some();
    for degrees in list("RIDE6_PITCHES", &[20.0, 35.0]) {
        for stick in list("RIDE6_STICKS", &[0.0, 0.1, 0.3]) {
            let (mut fallen, mut reached) = (Vec::new(), 0);
            for (name, spec) in &bikes {
                let (up, best) = landing(spec, degrees.to_radians(), stick, start, during);
                reached += usize::from(best >= degrees.to_radians());
                if !up {
                    fallen.push(name.clone());
                }
            }
            println!("({reached} of {} reach the pitch)", bikes.len());
            report(
                &format!(
                    "wheelie {degrees:.0} deg landing from {start} m/s, stick {stick}{}",
                    if during { " held throughout" } else { "" }
                ),
                bikes.len(),
                &fallen,
            );
        }
    }
}
