//! The steady turn at a full stick (`docs/specs/ride6-feel-round11.md`, section 2): the radius a bike holds at 20, 30, 40 and 50 m/s
//! against what the data's lean limit predicts, ridden on the imported bikes of a folder (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b]`).
//!
//! The stick is the whole of its travel at any speed, the throttle holds the speed (the same
//! proportional controller as the other sweeps), the radius is the speed over the yaw rate of the last 3 seconds of 12.

use blackbox_vehicle::bike::{Bike, BikeInput, choices};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::{held_throttle, specs};

const G: f32 = 9.81;

/// What a turn held at one speed settled into.
pub(super) struct Turn {
    pub(super) fell: bool,
    pub(super) radius: f32,
    pub(super) lean: f32,
    pub(super) limit: f32,
    pub(super) speed: f32,
    pub(super) lateral_g: f32,
}

/// Ride `spec` into a turn at `speed` m/s with the stick at `stick` for `seconds` and measure the last 3 s.
pub(super) fn turn(spec: &blackbox_vehicle::bike::BikeSpec, speed: f32, stick: f32, seconds: f32) -> Turn {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let steps = (seconds / FIXED_STEP) as usize;
    let from = steps - (3.0 / FIXED_STEP) as usize;
    let (mut yaw0, mut yaw1, mut lean, mut limit) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    let mut unwrapped = 0.0_f32;
    let mut last = 0.0_f32;
    for step in 0..steps {
        let v = bike.body().linear_velocity.length();
        // The stick rises to its end in a second, as a hand does; the sweeps of the earlier rounds stepped it.
        let ramped = (step as f32 * FIXED_STEP * 2.0).min(1.0) * stick;
        bike.step(
            FIXED_STEP,
            &BikeInput { throttle: held_throttle(speed, v), steer: ramped, ..Default::default() },
            &ground,
        );
        if bike.fallen() {
            return Turn { fell: true, radius: f32::NAN, lean, limit, speed: v, lateral_g: 0.0 };
        }
        let travel = bike.body().linear_velocity;
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
        yaw1 = unwrapped;
        if step >= from {
            lean = lean.max(bike.lean().abs());
            limit = limit.max(bike.lean_limit());
        }
    }
    let rate = (yaw1 - yaw0) / 3.0;
    let v = bike.body().linear_velocity.length();
    Turn { fell: false, radius: v / rate.abs().max(1e-4), lean, limit, speed: v, lateral_g: v * rate.abs() / G }
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_radius() {
    let speeds: Vec<f32> = std::env::var("RIDE6_SPEEDS")
        .map(|t| t.split(',').filter_map(|p| p.parse().ok()).collect())
        .unwrap_or_else(|_| vec![20.0, 30.0, 40.0, 50.0]);
    let stick: f32 = std::env::var("RIDE6_STICK").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    for (name, spec) in specs() {
        // The lean limit the data gives before the lateral-g term: the front tyre's grip at its rest load, arctangent, times the
        // tolerance, held to the steepest lean.
        let rest = (spec.front_share() * spec.total_mass() * G).max(1.0);
        let grip = spec.front.tyre.lateral_peak(rest * 1e-3) / rest;
        let base = (grip.atan() * choices::LATERAL_GRIP_TOLERANCE).min(spec.steering.max_lean);
        let mut line = format!(
            "{name}: base limit {base:.2} rad (tan {:.2}) rider offset {:?} influence {} cg height {:.2}",
            base.tan(),
            spec.rider.lateral_offset,
            spec.rider.lean_influence,
            spec.chassis.cg_height
        );
        for &speed in &speeds {
            let t = turn(&spec, speed, stick, 12.0);
            let predicted = speed * speed / (G * base.tan());
            match t.fell {
                true => line += &format!(" | v{speed:.0}: FELL"),
                false => {
                    line += &format!(
                        " | v{speed:.0}: R {:.0} m (data {predicted:.0}) lean {:.2} limit {:.2} v {:.1} g {:.2}",
                        t.radius, t.lean, t.limit, t.speed, t.lateral_g
                    )
                }
            }
        }
        println!("{line}");
    }
}
