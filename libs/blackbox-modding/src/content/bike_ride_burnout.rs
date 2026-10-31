//! The burnout sweep (`docs/specs/ride6-burnout.md`), ridden on the imported bikes of a folder:
//!
//! ```text
//! RIDE6_CARS=mods/ride6-bikes/cars [RIDE6_FILTER=a,b] cargo test -p blackbox-modding --features host --lib bike_burnout -- --ignored --nocapture
//! ```
//!
//! Each bike stands on flat ground with one of the [`HOLDS`] (the game's brake pedal and the throttle at their ends, the front lever alone, a light pull, the stick over, half the throttle). The sweep checks that the rear wheel
//! spins up (the slip ratio goes over 1 within a second), that the bike stays up and within a metre of where it stood, and that the
//! brake let go launches it without a fall or a flip.

use blackbox_vehicle::bike::{Bike, BikeInput};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::specs;

/// How long the burnout is held (s) and how long the bike is ridden after the brake is let go.
const HOLD: f32 = 3.0;
const LAUNCH: f32 = 4.0;
/// The slip ratio the rear wheel has to pass within [`SPIN_WITHIN`] seconds, and the farthest the bike may move meanwhile (m).
const SPIN_SLIP: f32 = 1.0;
const SPIN_WITHIN: f32 = 1.0;
const MAX_DRIFT: f32 = 1.0;
/// The pitch (rad) past which a bike counts as flipped.
const FLIP_PITCH: f32 = 1.2;

/// What one burnout and launch showed.
#[derive(Default, Debug)]
pub(super) struct Outcome {
    pub(super) spun_at: Option<f32>,
    pub(super) drift: f32,
    pub(super) fell_during: bool,
    pub(super) peak_slip: f32,
    pub(super) peak_pitch: f32,
    pub(super) fell_after: bool,
    pub(super) flipped: bool,
    pub(super) speed_after: f32,
    pub(super) rear_slip_after: f32,
}

/// What the rider holds during the burnout: the throttle, the two brake levers and the stick.
#[derive(Clone, Copy)]
pub(super) struct Hold {
    pub(super) label: &'static str,
    pub(super) throttle: f32,
    pub(super) front: f32,
    pub(super) rear: f32,
    pub(super) steer: f32,
}

/// The ways of holding it that must work: the game's brake pedal (both levers), the front lever alone (a pad with two triggers), a light
/// pull, the pedal with the stick over, and half the throttle (which only has to stay put and launch).
pub(super) const HOLDS: [Hold; 5] = [
    Hold { label: "pedal", throttle: 1.0, front: 1.0, rear: 1.0, steer: 0.0 },
    Hold { label: "front lever alone", throttle: 1.0, front: 1.0, rear: 0.0, steer: 0.0 },
    Hold { label: "light pedal 0.3", throttle: 1.0, front: 0.3, rear: 0.3, steer: 0.0 },
    Hold { label: "pedal with the stick over", throttle: 1.0, front: 1.0, rear: 1.0, steer: 1.0 },
    Hold { label: "pedal, half throttle", throttle: 0.5, front: 1.0, rear: 1.0, steer: 0.0 },
];

/// Ride the burnout: `hold` for [`HOLD`] seconds, then the brake and the stick let go (the throttle held) for [`LAUNCH`].
pub(super) fn burnout(spec: &blackbox_vehicle::bike::BikeSpec, hold: &Hold) -> Outcome {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    let start = bike.body().position;
    let mut out = Outcome::default();
    let steps = ((HOLD + LAUNCH) / FIXED_STEP) as usize;
    for step in 0..steps {
        let t = step as f32 * FIXED_STEP;
        let holding = t < HOLD;
        let input = match holding {
            true => {
                BikeInput { throttle: hold.throttle, front_brake: hold.front, rear_brake: hold.rear, steer: hold.steer }
            }
            false => BikeInput { throttle: hold.throttle, ..Default::default() },
        };
        bike.step(FIXED_STEP, &input, &ground);
        let slip = bike.wheel(1).slip_ratio;
        out.peak_pitch = out.peak_pitch.max(bike.pitch().abs());
        if holding {
            out.peak_slip = out.peak_slip.max(slip);
            if out.spun_at.is_none() && slip > SPIN_SLIP {
                out.spun_at = Some(t);
            }
            let moved = bike.body().position - start;
            out.drift = out.drift.max(moved.x.hypot(moved.z));
            out.fell_during |= bike.fallen();
            continue;
        }
        out.fell_after |= bike.fallen();
        out.flipped |= bike.pitch().abs() > FLIP_PITCH;
        out.speed_after = bike.forward_speed();
        out.rear_slip_after = out.rear_slip_after.max(slip);
    }
    out
}

fn passes(o: &Outcome, hold: &Hold) -> bool {
    let spun = hold.throttle < 1.0 || o.spun_at.is_some_and(|t| t <= SPIN_WITHIN);
    spun && o.drift <= MAX_DRIFT && !o.fell_during && !o.fell_after && !o.flipped && o.speed_after > 5.0
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_burnout() {
    let bikes = specs();
    for hold in &HOLDS {
        let (mut ok, mut slow, mut moved, mut fell, mut flipped, mut stuck) = (0, 0, 0, 0, 0, 0);
        let (mut latest, mut farthest, mut tipped) = (0.0_f32, 0.0_f32, 0.0_f32);
        let mut failures = Vec::new();
        for (name, spec) in &bikes {
            let o = burnout(spec, hold);
            if std::env::var_os("RIDE6_VERBOSE").is_some() {
                println!(
                    "{name}: spun {} peak slip {:.1} drift {:.2} m, fell {}/{} flipped {} pitch {:.2}, speed after {:.1} m/s",
                    o.spun_at.map_or("never".to_string(), |t| format!("{t:.2}s")),
                    o.peak_slip,
                    o.drift,
                    o.fell_during,
                    o.fell_after,
                    o.flipped,
                    o.peak_pitch,
                    o.speed_after
                );
            }
            ok += usize::from(passes(&o, hold));
            slow += usize::from(hold.throttle >= 1.0 && o.spun_at.is_none_or(|t| t > SPIN_WITHIN));
            moved += usize::from(o.drift > MAX_DRIFT);
            fell += usize::from(o.fell_during || o.fell_after);
            flipped += usize::from(o.flipped);
            stuck += usize::from(o.speed_after <= 5.0);
            latest = latest.max(o.spun_at.unwrap_or(f32::INFINITY));
            farthest = farthest.max(o.drift);
            tipped = tipped.max(o.peak_pitch);
            if !passes(&o, hold) {
                failures.push(name.clone());
            }
        }
        println!(
            "==== {}: {ok} of {} bikes burn out and launch; no spin within {SPIN_WITHIN} s: {slow}, moved over {MAX_DRIFT} m: {moved}, fell: {fell}, flipped: {flipped}, no launch: {stuck}; latest spin {latest:.2} s, farthest {farthest:.2} m, worst pitch {tipped:.2} rad",
            hold.label,
            bikes.len()
        );
        if !failures.is_empty() {
            println!("failures: {failures:?}");
        }
    }
}

/// The first bike of the filter, step by step round the moment the brake is let go: `RIDE6_FILTER=name`.
#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_burnout_trace() {
    let ground = FlatGround::new(0.0);
    let Some((name, spec)) = specs().into_iter().next() else { return };
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    println!("{name}");
    for step in 0..((HOLD + 2.0) / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let held = if t < HOLD { 1.0 } else { 0.0 };
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, front_brake: held, rear_brake: held, steer: 0.0 }, &ground);
        if t > HOLD - 0.1 && step % 6 == 0 {
            println!(
                "t {t:.2} pitch {:.2} v {:.1} rear slip {:.1} traction {:.0} N load {:.0} rpm {:.0} gear {} front down {} help-weight {:.2} burnout {}",
                bike.pitch(),
                bike.forward_speed(),
                bike.wheel(1).slip_ratio,
                bike.wheel(1).traction,
                bike.wheel(1).load,
                bike.rpm(),
                bike.gear(),
                bike.wheel(0).on_ground,
                bike.rider_weight(),
                bike.burnout()
            );
        }
    }
}
