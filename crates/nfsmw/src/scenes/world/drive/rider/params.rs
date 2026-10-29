//! The numbers that drive the rider's animation, from the bike's state (`docs/specs/ride6-rider-animation.md`, section 5,
//! and `ride6-rider-round6.md`, section 4). The graph's parameters are `lean`, `speed_kmh`, `tuck`, `wheelie`, `stoppie` and
//! three more for the wheelie and stoppie; the rider's body also shifts forward and back with the bike's acceleration.
//!
//! The signals are the ones the animation blueprint computes from the bike's inputs, with its constants (`Rider_Lean_Polish`,
//! `TuckedIn_Alpha`, `Wheelie/ Stoppie`), in the script language of `script.rs`.

use glam::Vec3;

use super::attach::Inertia;
use super::script::{Constant, Flashback, Interpolate, defuzzify, fuzzy_not, fuzzy_sum, ramp};

/// A slower step than this (s) does not move the filters: it is a pause, not time.
const MAX_STEP: f32 = 0.1;

/// `Rider_Lean_Polish(250, 400, 0.3, 0.4, 6, 0.5, 4, 0.4, 2, 3, 4, BikeLeanWeight, ...)`: above 250 km/h the lean fades to 0.3
/// of itself at 400, the lean is smoothed at 4 (the default; 2, 3 or 4 for a bike weight class 1 to 3, a variable the data
/// leaves at 0), and the multiplier at 2.2.
const LEAN_DECAY_SPEED: (f32, f32) = (250.0, 400.0);
const LEAN_DECAY: f32 = 0.3;
const LEAN_RATE: f32 = 4.0;
const LEAN_MULTIPLIER_RATE: f32 = 2.2;
/// The rider's own lean and the bike's, as the blueprint mixes them (0.3 and 0.7), and the mix at which the polished lean is
/// full (0.85); the polished lean is also faded in with the speed over -10 to 50 km/h.
const LEAN_MIX: (f32, f32) = (0.3, 0.7);
const LEAN_FULL: f32 = 0.85;
const LEAN_SPEED_FADE: (f32, f32) = (-10.0, 50.0);
/// `RiderLean_TerrainType_Multiplier` on tarmac (the surface the game rides on): the default of the rule, 1.
const TERRAIN_MULTIPLIER: f32 = 1.0;
/// `TuckedIn_Alpha`: the rates of the smoothers (the tuck input 6, the held throttle 1, the terrain 3, the brake 4) and the
/// time the throttle must have been held (0.2 s).
const TUCK_RATES: [f32; 4] = [6.0, 1.0, 3.0, 4.0];
const TUCK_THROTTLE_HOLD: f32 = 0.2;
/// `Wheelie/ Stoppie(0.15, 0.5, 0.12, 0.2, 0.2, 2.5, 3, 3)`: the range of the wheelie pose, its delay and smoothing, and the
/// stoppie smoothing; inside, the entry takes 8 per second, the stoppie clip's time is delayed 0.15 s and smoothed at 3.
const WHEELIE_RANGE: (f32, f32) = (0.15, 0.5);
const WHEELIE_DELAY: f32 = 0.2;
const WHEELIE_RATE: f32 = 2.5;
const WHEELIE_ENTRY_RATE: f32 = 8.0;
const STOPPIE_RATE: f32 = 3.0;
const STOPPIE_CLIP_DELAY: f32 = 0.15;
/// `BikeStoppie` above which the stoppie value goes to 1 (it goes to 0 at or below 0; between them it holds).
const STOPPIE_ON: f32 = 0.18;
/// `Wheelie_Direction`: the bike's lean beyond which the wheelie pose leans to a side (0.05 to 0.2).
const WHEELIE_SIDE: (f32, f32) = (0.05, 0.2);
/// `Wheelie_Standing_Alpha`: the rider stands on the pegs when his forward weight (`OriginalRiderWeightY`) is over 0.2 to 0.5
/// and the front wheel is up over 0.2 to 0.5 (`BikeWheelie`); the alpha follows at 3 per second.
const STANDING_WEIGHT: (f32, f32) = (0.2, 0.5);
const STANDING_WHEELIE: (f32, f32) = (0.2, 0.5);
const STANDING_RATE: f32 = 3.0;
/// `Wheelie_Entry_Explicit`: the entry ramp of the wheelie (0.2 to 0.4) times 0.5, delayed 0.4 s and followed at 4 per second.
const ENTRY_RAMP: (f32, f32) = (0.2, 0.4);
const ENTRY_SCALE: f32 = 0.5;
const ENTRY_DELAY: f32 = 0.4;
const ENTRY_RATE: f32 = 4.0;
/// The steering the standing wheelie's blend space reads: `VehicleOriginalSteer` followed at 6 per second.
const STEER_RATE: f32 = 6.0;
/// `Drift()`: the drift angle (degrees) from which a drift pose takes over (8) to where it is complete (20), on each side, and
/// the rate at which `Drift3BlendAlpha` follows (3.5 per second, at most that many units a second).
const DRIFT_ANGLE: (f32, f32) = (8.0, 20.0);
const DRIFT_RATE: f32 = 3.5;

/// What the animation needs of the bike at one moment. The bike's physics make it (`CarSim::rider_sample`); the fields
/// are in radians, metres per second and units of 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RiderSample {
    /// The rider's lean from upright (the bike's plus his body's own against it), positive to the right.
    pub lean: f32,
    /// The steepest lean the bike model allows.
    pub max_lean: f32,
    /// The bike's roll and the controller's target lean, over `max_lean` (-1 to 1, positive to the right): the blueprint's
    /// `BikeLean` and `RiderLean` inputs.
    pub bike_lean: f32,
    pub rider_lean: f32,
    /// Speed along the bike.
    pub speed: f32,
    pub throttle: f32,
    pub front_brake: f32,
    pub rear_brake: f32,
    /// The pitch of the bike's nose above the horizon, and the height of its forward axis above the horizontal (the sine of
    /// the pitch): the blueprint's `BikePitch` is the second.
    pub pitch_up: f32,
    pub elevation: f32,
    /// The tuck level of the rider (`Bike::tuck_level`): the blueprint's `TuckedIn`.
    pub tuck_level: f32,
    pub front_down: bool,
    pub rear_down: bool,
    /// Where the rider's centre of mass is from his rest place, in the bike's physics axes (x right, y up, z forward), metres:
    /// the place the weight, the lean and the speed ask for plus how far his spring has let him move.
    pub offset: Vec3,
    /// His body against the bike's attitude: roll to the right, pitch nose up and yaw to the right, radians.
    pub rotation: Vec3,
    /// The rider's weight along the bike, -1 (back) to 1 (forward): the blueprint's `OriginalRiderWeightY`.
    pub weight: f32,
    /// The steering input, -1 (left) to 1 (right): `VehicleOriginalSteer`.
    pub steer: f32,
    /// The angle between the bike's path and its heading, degrees, positive when the heading is to the right of the path (the
    /// rear has slid out in a right-hand turn): `VehicleDriftAngleDeg`.
    pub drift_deg: f32,
}

impl RiderSample {
    pub fn speed_kmh(&self) -> f32 {
        self.speed.max(0.0) * 3.6
    }

    /// The harder of the two brake levers, 0 to 1.
    pub fn braking(&self) -> f32 {
        self.front_brake.max(self.rear_brake).clamp(0.0, 1.0)
    }

    /// The blueprint's `BikeWheelie`: the height of the nose (the sine of the pitch) while the front tyre is off the ground
    /// and the rear one is on it, else 0.
    pub fn wheelie(&self) -> f32 {
        match !self.front_down && self.rear_down {
            true => self.elevation.max(0.0),
            false => 0.0,
        }
    }

    /// The blueprint's `BikeStoppie`: the depth of the nose while the rear tyre is off the ground and the front one is on it.
    pub fn stoppie(&self) -> f32 {
        match self.front_down && !self.rear_down {
            true => (-self.elevation).max(0.0),
            false => 0.0,
        }
    }
}

/// The values the graph's parameters take.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Values {
    /// `RiderLeaning`: -1 (left) to 1 (right).
    pub lean: f32,
    pub speed_kmh: f32,
    /// `RiderTuckedIn_Alpha`.
    pub tuck: f32,
    /// `BikeWheelie` held to 8 per second (the entry of the wheelie pose), and `Wheelie_Alpha` (the minimum-to-maximum pose).
    pub wheelie: f32,
    pub wheelie_alpha: f32,
    /// `Wheelie_Direction`, -1 to 1: which side the wheelie pose leans to.
    pub wheelie_direction: f32,
    /// `Stoppie_Alpha`, and the clip's time (`Stoppie_Explicit` over 1.27 s).
    pub stoppie: f32,
    pub stoppie_time: f32,
    /// `Wheelie_Standing_Alpha`, the smoothed steering and `Wheelie_Entry_Explicit`: the standing wheelie's weight and the
    /// two axes of its blend space.
    pub standing: f32,
    pub steer: f32,
    pub standing_entry: f32,
    /// `Drift3BlendAlpha`, -1 (the left drift pose) to 1 (the right one).
    pub drift: f32,
}

/// The state of the blueprint's smoothers and delay lines.
#[derive(Debug, Clone, Default, PartialEq)]
struct Script {
    lean: Interpolate,
    lean_multiplier: Interpolate,
    tuck_input: Interpolate,
    tuck_throttle: Interpolate,
    tuck_terrain: Interpolate,
    tuck_brake: Interpolate,
    throttle_before: Flashback,
    wheelie_entry: Constant,
    wheelie_delay: Flashback,
    wheelie_alpha: Interpolate,
    stoppie_value: f32,
    stoppie_alpha: Constant,
    stoppie_delay: Flashback,
    stoppie_time: Constant,
    standing: Constant,
    steer: Constant,
    entry_delay: Flashback,
    entry: Constant,
    drift: Constant,
}

impl Script {
    /// `Rider_Lean_Polish`: the lean of the rider's animation, -1 to 1.
    fn lean(&mut self, sample: &RiderSample, speed: f32, dt: f32) -> f32 {
        let mix = LEAN_MIX.0 * sample.rider_lean + LEAN_MIX.1 * sample.bike_lean;
        let fade = ramp(speed, LEAN_SPEED_FADE.0, LEAN_SPEED_FADE.1);
        let polished = defuzzify(&[(ramp(mix, 0.0, LEAN_FULL), fade), (ramp(mix, 0.0, -LEAN_FULL), -fade)], 0.0);
        let smooth = self.lean.step(polished, LEAN_RATE, dt);
        let decay = ramp(speed, LEAN_DECAY_SPEED.0, LEAN_DECAY_SPEED.1);
        let wanted = TERRAIN_MULTIPLIER * defuzzify(&[(decay, LEAN_DECAY)], 1.0);
        smooth * self.lean_multiplier.step(wanted, LEAN_MULTIPLIER_RATE, dt)
    }

    /// `TuckedIn_Alpha`: the tuck of the rider's animation, 0 to 1.
    fn tuck(&mut self, sample: &RiderSample, wheelie_alpha: f32, dt: f32) -> f32 {
        let held = self.throttle_before.step(sample.throttle.clamp(0.0, 1.0), TUCK_THROTTLE_HOLD, dt);
        let on = f32::from(sample.throttle > 0.0) * f32::from(held > 0.0);
        let level = sample.tuck_level.clamp(0.0, 1.0);
        let by_input = self.tuck_input.step(level, TUCK_RATES[0], dt);
        let by_throttle = self.tuck_throttle.step(fuzzy_sum(level, on), TUCK_RATES[1], dt);
        let terrain = self.tuck_terrain.step(TERRAIN_MULTIPLIER.clamp(0.0, 1.0), TUCK_RATES[2], dt);
        let not_braking = self.tuck_brake.step(fuzzy_not(sample.braking()), TUCK_RATES[3], dt);
        by_input.max(by_throttle) * terrain * not_braking * fuzzy_not(wheelie_alpha.clamp(0.0, 1.0))
    }
}

/// `Wheelie_Direction`: the bike's lean to either side beyond a dead band, weighted by itself.
fn wheelie_direction(bike_lean: f32) -> f32 {
    let right = ramp(bike_lean, WHEELIE_SIDE.0, WHEELIE_SIDE.1);
    let left = ramp(bike_lean, -WHEELIE_SIDE.0, -WHEELIE_SIDE.1);
    defuzzify(&[(right, right), (left, -left)], 0.0)
}

/// `Drift()`: the drift angle on either side beyond 8 degrees, weighted by itself, -1 to 1 (before the smoothing).
fn drift_alpha(angle_deg: f32) -> f32 {
    let right = ramp(angle_deg, DRIFT_ANGLE.0, DRIFT_ANGLE.1);
    let left = ramp(angle_deg, -DRIFT_ANGLE.0, -DRIFT_ANGLE.1);
    defuzzify(&[(right, right), (left, -left)], 0.0)
}

/// The inputs after the blueprint's script, and the rider's shift along the bike.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Smoothed {
    values: Values,
    script: Script,
    /// The rider's displacement from his rest place in the car's axes (x forward, y left, z up), metres, after the clamp to the
    /// ranges of the animation and the smoothing.
    shift: Vec3,
    /// His body's turn against the bike's: roll to the right, pitch nose up, yaw to the right (rad).
    rotation: Vec3,
}

impl Smoothed {
    /// Advance by `dt` seconds with the bike in state `sample`; the script carries the blueprint's own rates.
    pub fn step(&mut self, sample: &RiderSample, inertia: &Inertia, dt: f32) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let dt = dt.min(MAX_STEP);
        let speed = sample.speed_kmh();
        let (wheelie, stoppie) = (sample.wheelie(), sample.stoppie());
        let s = &mut self.script;
        let lean = s.lean(sample, speed, dt);
        // `Wheelie_Alpha`: the ramp of the wheelie, delayed and smoothed.
        let ramped = ramp(wheelie, WHEELIE_RANGE.0, WHEELIE_RANGE.1);
        let delayed = s.wheelie_delay.step(ramped, WHEELIE_DELAY, dt);
        let wheelie_alpha = s.wheelie_alpha.step(delayed, WHEELIE_RATE, dt);
        let tuck = s.tuck(sample, wheelie_alpha, dt);
        // `StoppieValue`: 1 above 0.18, 0 at or below 0, and the value it had in between.
        let on = f32::from(stoppie > STOPPIE_ON);
        let off = f32::from(stoppie <= 0.0);
        s.stoppie_value = defuzzify(&[(on, 1.0), (off, 0.0)], s.stoppie_value);
        let delayed = s.stoppie_delay.step(s.stoppie_value, STOPPIE_CLIP_DELAY, dt);
        let standing = ramp(sample.weight, STANDING_WEIGHT.0, STANDING_WEIGHT.1)
            * ramp(wheelie, STANDING_WHEELIE.0, STANDING_WHEELIE.1);
        let entry = s.entry_delay.step(ramp(wheelie, ENTRY_RAMP.0, ENTRY_RAMP.1) * ENTRY_SCALE, ENTRY_DELAY, dt);
        self.values = Values {
            lean,
            speed_kmh: speed,
            tuck,
            wheelie: s.wheelie_entry.step(wheelie, WHEELIE_ENTRY_RATE, dt),
            wheelie_alpha,
            wheelie_direction: wheelie_direction(sample.bike_lean),
            stoppie: s.stoppie_alpha.step(s.stoppie_value, STOPPIE_RATE, dt),
            stoppie_time: s.stoppie_time.step(delayed, STOPPIE_RATE, dt),
            standing: s.standing.step(standing, STANDING_RATE, dt),
            steer: s.steer.step(sample.steer.clamp(-1.0, 1.0), STEER_RATE, dt),
            standing_entry: s.entry.step(entry, ENTRY_RATE, dt),
            drift: s.drift.step(drift_alpha(sample.drift_deg), DRIFT_RATE, dt),
        };
        // The shift: the rider's displacement from the physics, held to the ranges of the animation's inertia input.
        let target = shift_target(sample.offset, inertia);
        self.shift += (target - self.shift) * (inertia.rate * dt).clamp(0.0, 1.0);
        self.rotation = match sample.rotation.is_finite() {
            true => sample.rotation,
            false => Vec3::ZERO,
        };
    }

    pub fn values(&self) -> Values {
        self.values
    }

    /// The rider's displacement from his rest place in the car's axes (x forward, y left, z up), metres.
    pub fn shift(&self) -> Vec3 {
        self.shift
    }

    /// His body's turn against the bike's: roll to the right, pitch nose up, yaw to the right (rad). The physics already
    /// filters it (the rider's body is a spring and a damper), so it is not smoothed again.
    pub fn rotation(&self) -> Vec3 {
        self.rotation
    }
}

/// Where the rider's shift goes for the displacement `offset` of his centre of mass (the bike's physics axes: x right, y up,
/// z forward, metres): the same vector in the car's axes (x forward, y left, z up), held to the range of the animation's
/// inertia input (`inertia.min` to `inertia.max`, the blueprint's `InertiaDisplacement_RidingMin/Max`).
pub fn shift_target(offset: Vec3, inertia: &Inertia) -> Vec3 {
    if !offset.is_finite() {
        return Vec3::ZERO;
    }
    let car = Vec3::new(offset.z, -offset.x, offset.y);
    car.clamp(Vec3::from_array(inertia.min), Vec3::from_array(inertia.max))
}

#[cfg(test)]
#[path = "params_tests.rs"]
mod tests;
