//! The stick flicked from side to side while the throttle opens (`docs/specs/ride6-feel-round11.md`, section 1), ridden on the imported
//! bikes of a folder (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b]`) as ignored tests:
//!
//! - `bike_flick`: a pad: the throttle held at 1.0 (the bike's own axis rate opens it, 3 per second in the data), the stick moved at
//!   `1 / rise` per second towards +1 and -1 in turn (`RIDE6_RISES`, `RIDE6_HOLDS`), from a standstill and from several speeds
//!   (`RIDE6_SPEEDS`), through what the game's `command()` does to the stick (the share by speed). Prints how many bikes fall in
//!   every configuration, how, and which.
//!
//! `RIDE6_WEIGHT=arc` adds the thumb's arc: the weight lever (the stick's other axis) is pulled back by a quarter of the stick's
//! deflection. `RIDE6_VERBOSE=1` prints a line for every fall.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec, choices};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::specs;

fn list(key: &str, default: &[f32]) -> Vec<f32> {
    match std::env::var(key) {
        Ok(text) => text.split(',').filter_map(|p| p.parse().ok()).collect(),
        Err(_) => default.to_vec(),
    }
}

/// How one flick ride ended.
#[derive(Clone, Debug, Default)]
pub(super) struct Outcome {
    pub(super) fell: Option<(f32, String)>,
    pub(super) detail: String,
    pub(super) top_pitch: f32,
}

/// The stick at time `t`: it moves at `1 / rise` per second towards +1 for `half` seconds, then towards -1 for `half`, and so on,
/// starting from the middle at `begin`.
pub(super) fn flick(t: f32, begin: f32, rise: f32, half: f32) -> f32 {
    if t < begin {
        return 0.0;
    }
    let mut value = 0.0_f32;
    let mut clock = begin;
    let mut side = 1.0_f32;
    let rate = 1.0 / rise.max(1e-3);
    while clock + half < t {
        value = (value + side * rate * half).clamp(-1.0, 1.0);
        clock += half;
        side = -side;
    }
    (value + side * rate * (t - clock)).clamp(-1.0, 1.0)
}

/// Ride `spec` from `start` m/s on flat ground for `seconds` with full throttle and the flicked stick.
pub(super) fn ride(spec: &BikeSpec, start: f32, seconds: f32, rise: f32, half: f32, arc: bool) -> Outcome {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
    let mut out = Outcome::default();
    let verbose = std::env::var_os("RIDE6_VERBOSE").is_some();
    for step in 0..(seconds / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let stick = flick(t, 0.3, rise, half);
        // What the game's `command()` does to the stick: the share by speed.
        let share = choices::stick_share(bike.forward_speed());
        bike.set_weight(if arc { -0.25 * stick.abs() } else { 0.0 });
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, steer: stick * share, ..Default::default() }, &ground);
        out.top_pitch = out.top_pitch.max(bike.pitch());
        if bike.fallen() {
            out.fell = Some((t, format!("{:?}", bike.fall_kind())));
            let (front, rear) = (bike.wheel(0), bike.wheel(1));
            let rate = bike.body().angular_velocity;
            out.detail = format!(
                "t {t:.2} v {:.1} gear {} stick {stick:.2} lean {:.2} limit {:.2} pitch {:.2} front {} rear slip {:.2} angle {:.2} \
                 rear load {:.0} rate ({:.1} {:.1} {:.1})",
                bike.forward_speed(),
                bike.gear(),
                bike.lean(),
                bike.lean_limit(),
                bike.pitch(),
                front.on_ground,
                rear.slip_ratio,
                rear.slip_angle,
                rear.load,
                rate.x,
                rate.y,
                rate.z
            );
            break;
        }
        let every = list("RIDE6_EVERY", &[24.0])[0] as usize;
        if verbose && step % every.max(1) == 0 && t >= list("RIDE6_FROM", &[0.0])[0] {
            println!(
                "  t{t:.2} v{:.1} g{} stick{stick:.2} lean{:.2}/{:.2} tgt{:.2} steer{:.3} pitch{:.2} f{}:{:.0}/{:.2} r{}:{:.0} slip{:.2}/{:.2} roll{:.2}",
                bike.forward_speed(),
                bike.gear(),
                bike.lean(),
                bike.lean_limit(),
                bike.target_lean(),
                bike.steer_angle(),
                bike.pitch(),
                u8::from(bike.wheel(0).on_ground),
                bike.wheel(0).load,
                bike.wheel(0).slip_angle,
                u8::from(bike.wheel(1).on_ground),
                bike.wheel(1).load,
                bike.wheel(1).slip_ratio,
                bike.wheel(1).slip_angle,
                bike.body().angular_velocity.z
            );
        }
    }
    out
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_flick() {
    let bikes = specs();
    let arc = std::env::var("RIDE6_WEIGHT").is_ok_and(|w| w == "arc");
    let seconds = list("RIDE6_SECONDS", &[8.0])[0];
    let mut total = std::collections::BTreeMap::<String, usize>::new();
    for start in list("RIDE6_SPEEDS", &[0.0, 5.0, 10.0, 15.0, 20.0, 25.0]) {
        for rise in list("RIDE6_RISES", &[0.1, 0.2, 0.3]) {
            for hold in list("RIDE6_HOLDS", &[0.0, 0.25]) {
                let half = 2.0 * rise + hold;
                let mut fallen = Vec::new();
                let mut kinds = std::collections::BTreeMap::<String, usize>::new();
                for (name, spec) in &bikes {
                    let out = ride(spec, start, seconds, rise, half, arc);
                    let Some((time, kind)) = out.fell else { continue };
                    *kinds.entry(kind).or_default() += 1;
                    fallen.push((name.clone(), time));
                    *total.entry(name.clone()).or_default() += 1;
                    if std::env::var_os("RIDE6_FALLS").is_some() {
                        println!("    {name}: {}", out.detail);
                    }
                }
                let names: Vec<&str> = fallen.iter().take(5).map(|(n, _)| n.as_str()).collect();
                println!(
                    "flick from {start:.0} m/s rise {rise} hold {hold}: {} of {} fall {kinds:?} {names:?}",
                    fallen.len(),
                    bikes.len()
                );
            }
        }
    }
    let mut worst: Vec<_> = total.into_iter().collect();
    worst.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    println!("==== bikes by number of falling configurations: {:?}", &worst[..worst.len().min(30)]);
    println!("({} bikes ever fell)", worst.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stick_swings_from_side_to_side_at_the_rate_of_the_rise() {
        assert_eq!(flick(0.0, 0.3, 0.2, 0.4), 0.0);
        assert!((flick(0.4, 0.3, 0.2, 0.4) - 0.5).abs() < 1e-5);
        assert!((flick(0.7, 0.3, 0.2, 0.4) - 1.0).abs() < 1e-5);
        assert!(flick(0.9, 0.3, 0.2, 0.4).abs() < 1e-5);
        assert!((flick(1.1, 0.3, 0.2, 0.4) + 1.0).abs() < 1e-5);
    }
}
