//! The modelling choices of the two-wheel model: every number or structural decision that neither the data of a bike nor
//! a published formula nor the recovered behaviour of the reference game fixes. Each has its reason and the spec that names
//! it as open (`docs/specs/vehicle-two-wheel.md`, `docs/specs/vehicle-two-wheel-parameters.md`). When a fact turns up, change
//! the constant here and nowhere else.

/// How fast ABS releases once the wheel recovers (1/s): the `ABSFallRate` constructor default, 10000, which no bike's data
/// overrides (recovered, `ride6-powertrain-round6.md`, section 5). A release of 1 takes a tenth of a millisecond, so the
/// release follows its target in effect at once; the constant stays a rate so a slower one can be tested.
pub const ABS_FALL_RATE: f32 = 10_000.0;

/// The desired front side force is `-tau / arm` (recovered) plus the front's share of the force a steady turn needs
/// (`m g tan(lean)`, the share being the front's weight share). The binary has no such feed-forward: it closes the roll with the
/// front tyre and the lean help alone, so a held lean sits a fraction of a degree off its target (`ride6-chassis-round6.md`, "The
/// roll closure"). It stays because without it this model's bike, held at the lean limit (full stick at 20 m/s), loses its front
/// tyre after about a second and falls; every other test passes without it. Whether the cause is the model's tyre or a term not
/// recovered was not settled.
pub const LEAN_FORCE_INCLUDES_TURN_FORCE: bool = true;

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
