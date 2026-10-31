//! The modelling choices of the two-wheel model: every number or structural decision that neither the data of a bike nor
//! a published formula nor the recovered behaviour of the reference game fixes. Each has its reason and the spec that names
//! it as open (`docs/specs/vehicle-two-wheel.md`, `docs/specs/vehicle-two-wheel-parameters.md`). When a fact turns up, change
//! the constant here and nowhere else.

/// How fast ABS releases once the wheel recovers (1/s): the `ABSFallRate` constructor default, 10000, which no bike's data
/// overrides (recovered, `ride6-powertrain-round6.md`, section 5). A release of 1 takes a tenth of a millisecond, so the
/// release follows its target in effect at once; the constant stays a rate so a slower one can be tested.
pub const ABS_FALL_RATE: f32 = 10_000.0;

/// The factor on the arctangent of the lateral grip in the lean limit: the wheel's `LateralGripTolerance`, a constructor
/// default of 1 (recovered, `ride6-suspension.md`, "Unset C++ defaults").
pub const LATERAL_GRIP_TOLERANCE: f32 = 1.0;

/// The least value of the arm the lean controller divides the wanted front force by (`lean::lean_arm`): the binary floors it at 0.05
/// (`ride6-feel-round9.md`, section 1; `0x1450c9320`).
pub const MIN_LEAN_ARM: f32 = 0.05;

/// How far to the side (m) the rider can throw his weight to hold the bike's lean on its rear wheel alone: 0.25 m, the largest side
/// offset any bike's rider data states (the KTM's stopped offset; most bikes state none). A modelling choice: the reference's own balance
/// of a bike on one wheel was not found (its lean help acts at the front contact only, `0x1450c3ae0`, and is zero with the front wheel
/// off the road), and without a torque of some kind a stick turns a wheelie over (`ride6-feel-round9.md`, section 6).
pub const BALANCE_REACH: f32 = 0.25;

/// The share of the efficiency loss that goes to the longitudinal force: the `TireEfficiencyBalance` constructor default of the
/// tyre data (recovered, `ride6-tyre-thermal.md` section 1; the lateral force gets the rest).
pub const TYRE_EFFICIENCY_BALANCE: f32 = 0.5;

/// Numerical guard: slower than this (m/s) the bike counts as standing for the tests that divide by the speed.
pub const STANDING_SPEED: f32 = 0.5;

/// The least lean limit (rad) the rider's place divides the target lean by, so a limit that has not been planned yet (0) asks for
/// no lean shift rather than a division by zero. Numerical guard.
pub const MIN_LEAN_LIMIT: f32 = 0.05;

/// The longest sub-step (s) of a bike step. The reference game divides a vehicle step longer than the chassis' `MaxDeltaTime` into at
/// most `MaxSubsteps` equal parts (recovered, `ride6-chassis-round7.md` section 1); the values come from the project's physics
/// simulation settings, which are not among the files read. 1/480 s is the longest at which this model's lean loop, with the stick
/// held at 0.8 or more, kept 287 of the 288 bikes of the sweep up for 10 s at any speed from 8 to 40 m/s (at 1/240 the loop's
/// one-step delay made the bike weave at 2 Hz at the lean limit and fall within seconds, and a stick of 1.0 fell on a third of the
/// bikes; `ride6-feel-round8.md`, section 3). Open.
pub const MAX_SUBSTEP: f32 = 1.0 / 480.0;

/// The most sub-steps of a step (a guard against a very long host step).
pub const MAX_SUBSTEPS: usize = 32;

/// The share of the traction control's slip window (`TCSSlipThresholdsDry`) the model uses: the control starts to cut the throttle at
/// this share of the data's lower slip and cuts fully at this share of the upper one. A modelling choice: the reference shapes the
/// cut by the lean and the yaw rate and its ECU element reduces the throttle by slip, wheelie angle and lean, none of which was recovered
/// (`ride6-powertrain-round6.md`, section 13; the bikes read carry no throttle-reduction tuning), and with the data's window alone a
/// bike launched with the stick held spins its rear up and throws itself (14 of 20 bikes of the sweep fell within 5 s); at half the
/// window none did, at the price of half a second on the time to 100 km/h (3.8 s to 4.4 s on average).
pub const TCS_WINDOW_SCALE: f32 = 0.5;

/// How much of the brake levers a bike at its lean limit gives up: the levers are multiplied by `1 - this x (lean / limit)^2`. A
/// modelling choice. The reference's ABS releases the front brake only by the longitudinal slip, and its brake reduction of the front
/// wheel is 0 for every bike read (`ride6-powertrain-round7.md`, section 1), so a pedal held down in a corner takes the grip the lean
/// needs: the rear wheel lifts or slides and the bike falls inward (15 of the 20 bikes of the subset, a lever of 1.0 at 25 m/s with the
/// stick at 0.8). At 0.8 one of them falls; a straight stop (lean 0) is untouched. It should go if the reference's own relief is found.
pub const BRAKE_LEAN_RELIEF: f32 = 0.8;

/// What a collision adds to the help of the controllers, `(addition, fade rate per second)`: the reference's `CollisionLeanHelpAlteration` and
/// `CollisionStabilityHelpAlteration` (`MaxHelpAddition`, `HelpAlterationFadeRate`), equal in the six road-bike input handlers of the
/// data; the two scooters' differ (lean help 0.5 and 4, one frontal fade of 5) and are not modelled
/// (`docs/specs/ride6-chassis-round6.md`, section 2; `ride6-feel-round10.md`, section 3). The lean help's addition is 2 (so the factor is
/// 1 whatever its tuning) for 0.4 s; the stability help's 0.4 for 0.08 s, and 0.2 fading at 1 per second for a frontal collision. Whether the
/// addition fades linearly is not known: a linear fade is a modelling choice.
pub const COLLISION_LEAN_HELP: (f32, f32) = (2.0, 5.0);
pub const COLLISION_STABILITY_HELP: (f32, f32) = (0.4, 5.0);
pub const COLLISION_STABILITY_HELP_FRONTAL: (f32, f32) = (0.2, 1.0);

/// The speed (m/s) up to which a full stick is the whole of the lean limit, and the speed from which it gives only [`STICK_SHARE_AT_TOP`] of it.
pub const STICK_FULL_UNTIL: f32 = 30.0;
pub const STICK_REDUCED_AT: f32 = 40.0;
/// The share of the lean limit a full stick gives from [`STICK_REDUCED_AT`] on. 1.0: none is held back. A user-tunable modelling choice
/// (`ride6-feel-round11.md`, section 2): the reference's full stick asks for the whole lean limit at any speed; rounds 9 and 10 held it to
/// 0.9 from 40 m/s because the 1000 cm3 sports bikes weaved at that speed when the throttle moved (267 to 283 of 288 held a full stick at
/// 40 m/s). With the lean help of round 11 (the grip floor at the rest load, the g-force of the lean limit) 287 of 288 hold a full stick
/// from 8 to 50 m/s with a moving throttle (the exception is the Italjet Dragster 700, whose fall limit is its lean limit), so the cap went.
/// Set it under 1.0 (0.9 was the old value) to give the stick less of the lean limit at speed.
pub const STICK_SHARE_AT_TOP: f32 = 1.0;

/// The share of the stick a host gives the lean controller at this speed (m/s), for a stick pushed to its end: a full stick up to
/// [`STICK_FULL_UNTIL`], falling to [`STICK_SHARE_AT_TOP`] by [`STICK_REDUCED_AT`] (no fall at all with the default of 1.0). A radius at
/// those speeds is `v^2 / (g tan lean)`; a share under 1.0 widens it.
pub fn stick_share(speed: f32) -> f32 {
    share_falling_to(speed, STICK_SHARE_AT_TOP)
}

/// [`stick_share`] for a share `top` from [`STICK_REDUCED_AT`] on.
fn share_falling_to(speed: f32, top: f32) -> f32 {
    let t = ((speed.abs() - STICK_FULL_UNTIL) / (STICK_REDUCED_AT - STICK_FULL_UNTIL)).clamp(0.0, 1.0);
    1.0 + (top - 1.0) * t
}

/// The least lean limit (rad) a wheelie leaves the stick: the reference cuts the lean limit with the wheelie angle down to 1 degree at 40 to
/// 45 degrees (`WheelieLeanLimitTuning`, data), which keeps a wheelie straight; here the cut stops at this limit. A modelling choice
/// (`ride6-wheelie-steer.md`, section 3): with the data's cut a full stick at the height the wheelie assist holds (35 degrees) leans the
/// bike 9 to 12 degrees and the heading changes by 5 to 10 degrees in 3 s, against 90 to 150 degrees on the road. 0.35 rad (20 degrees)
/// turns the wheelie at about a third of the road's rate, and the lean help, the wheelie assist and the fall tests keep the bike up (the
/// sweeps of the spec fall on no bike). 0 or less gives the data's cut back.
pub const WHEELIE_LEAN_FLOOR: f32 = 0.35;

/// The share of the gap between the yaw rate of a steady turn at the bike's lean and its own that the yaw torque closes in a sub-step
/// while the front wheel is up and the rear is on the road (`wheelie_steer::apply_wheelie_yaw`): the reference's stability help has a
/// factor for exactly this state, `StabilityHelpFactorWheelie`, whose constructor default is 0 and which no bike sets, so the
/// reference leaves the heading of a leaned wheelie to the rear tyre. A modelling choice: 1 holds the yaw rate to the turn the lean
/// needs; 0 switches the torque off (`ride6-wheelie-steer.md`, section 3).
pub const WHEELIE_YAW_FACTOR: f32 = 1.0;

/// The wheelie angles (rad, of the line through the hubs) over which the yaw torque of a wheelie comes in: none under the first, all over the
/// second. A modelling choice: a front wheel that hovers a few centimetres over the road (a pitch of 0.03 to 0.05 rad, as a throttle opened
/// with the stick flicked makes it, `ride6-feel-round11.md`, section 1.2) is a bike on the road, and a yaw torque on it made 4 bikes of 288 fall in
/// the flick sweep; 8 to 20 degrees is under the height the wheelie assist holds (35 degrees on most bikes) with room to spare.
pub const WHEELIE_YAW_PITCH_RANGE: [f32; 2] = [0.15, 0.35];

/// How far over its limit (as a multiple of `max_rpm`) the engine may turn before the fuel stays cut instead of alternating. A modelling
/// choice. The reference alternates the cut at the limiter (`ride6-engine-drivetrain.md`, "Rev limiter"), which averages to a positive
/// torque when the torque curve is held flat past its end, as `curve` holds it; an engine driven from the wheel through a clutch that is
/// locked, which is what a burnout is, then climbs without a bound (31000 rpm on a 15500 rpm bike in 3 s of a burnout). A real engine
/// is cut and dragged back, so past this share the cut is held. 1.1 sits above any overshoot of a slipping clutch at the limiter
/// (the alternation is tested at 1.05) and well below what a free wheel reaches.
pub const OVERREV_SHARE: f32 = 1.1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stick_has_its_whole_range_at_any_speed_unless_a_share_at_the_top_is_set() {
        for speed in [0.0, 30.0, 35.0, 40.0, -80.0] {
            assert_eq!(stick_share(speed), 1.0);
        }
    }

    #[test]
    fn a_share_under_one_at_the_top_falls_linearly_between_the_two_speeds() {
        assert_eq!(share_falling_to(30.0, 0.9), 1.0);
        assert!((share_falling_to(35.0, 0.9) - 0.95).abs() < 1e-6);
        assert!((share_falling_to(40.0, 0.9) - 0.9).abs() < 1e-6 && (share_falling_to(-80.0, 0.9) - 0.9).abs() < 1e-6);
    }
}
