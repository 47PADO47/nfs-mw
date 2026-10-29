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
