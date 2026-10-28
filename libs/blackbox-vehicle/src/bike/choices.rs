//! The modelling choices of the two-wheel model: every number or structural decision that neither the data of a bike nor
//! a published formula nor the recovered behaviour of the reference game fixes. Each has its reason and the spec that names
//! it as open (`docs/specs/vehicle-two-wheel.md`, `docs/specs/vehicle-two-wheel-parameters.md`). When a fact turns up, change
//! the constant here and nowhere else.

/// The engine speed, as a share of the rev limit, below which the automatic gearbox changes down, when a bike does not
/// state one. Open in `ride6-engine-drivetrain.md` ("Automatic shifting": the comparison and the use of the tolerances were
/// not located). It is a little under the engine speed an upshift at the up-shift point leaves behind in the next gear, so the
/// box does not hunt.
pub const DOWN_SHIFT_SHARE: f32 = 0.45;

/// The throttle above which the automatic box changes up (open, same section). A closed or nearly closed throttle holds the
/// gear, as a rider coasting would.
pub const UP_SHIFT_MIN_THROTTLE: f32 = 0.3;

/// The clutch is open for the whole of a gear change and engaged afterwards (open: the data's `ClutchTimingOnGearChange`
/// ramps the clutch out and back in over fractions of the change, but the exact ramps are not pinned down). The engine runs
/// free during the change, as the drive is cut.
pub const CLUTCH_OPEN_DURING_SHIFT: bool = true;

/// The time the clutch takes to engage again after a gear change (s). Open, with `CLUTCH_OPEN_DURING_SHIFT`: without a ramp
/// the engine and the wheel would be forced to one speed in a single step. About the time of a change up.
pub const CLUTCH_ENGAGE_TIME: f32 = 0.06;

/// Whether the launch clutch is engaged by engine speed (a feathering rider holds the engine near a speed while the bike
/// gets going). Open: the feathering control law was not read (`ride6-engine-drivetrain.md`, "Launch clutch"); the target
/// engine speed is recovered (`feather_factor` and `feather_max_factor` of the spec). The engagement is the larger of the
/// share of the way the engine is from idle to that speed and the share of the release speed the bike has reached.
pub const LAUNCH_ENGAGEMENT_FROM_ENGINE_SPEED: bool = true;

/// The engine speed, as a multiple of the idle speed, at which the launch clutch starts to take up the drive. Open (same
/// section); the data's `MinRPMSpeedScaleRange` tops out at 1.1. The margin keeps an idling engine from dragging the bike.
pub const LAUNCH_ENGAGE_START_FACTOR: f32 = 1.1;

/// How fast the slipper clutch closes again once the back torque is gone (1/s). Open: `SlipperClutchFilterSpeed` is a
/// constructor default that was not listed.
pub const SLIPPER_FALL_RATE: f32 = 10.0;

/// How fast ABS releases once the wheel recovers (1/s). Open: `ABSFallRate` is a constructor default that was not listed.
pub const ABS_FALL_RATE: f32 = 8.0;

/// The roll torque the rider may add directly, to realise the prescribed lean, as a multiple of weight times the height of
/// the centre of gravity (N m). Open: the binary prescribes the roll motion and closes it with the tyre forces and the lean
/// help (a bounded side force at the front tyre, which this model has too); the rest of that closure (the stability help,
/// off for the stock bike, and the pairing of the help reductions) was not recovered (`ride6-lean-controller.md`). The limit
/// must exceed the tipping torque at the steepest lean and the acceleration of a full-stick change, so the stand-in can hold
/// a bike at a standstill but cannot hold one that is sliding.
pub const ROLL_TORQUE_AUTHORITY: f32 = 2.0;

/// The desired front side force is `-tau / arm` (recovered) plus the front's share of the force a steady turn needs
/// (`m g tan(lean)`, the share being the front's weight share). Open: the binary's force balance for a held lean was not
/// recovered; without the second term nothing would turn this model's bike once the lean is reached.
pub const LEAN_FORCE_INCLUDES_TURN_FORCE: bool = true;

/// The factor on the arctangent of the lateral grip in the lean limit: the wheel's `LateralGripTolerance`, a constructor
/// default of 1 (recovered, `ride6-suspension.md`, "Unset C++ defaults").
pub const LATERAL_GRIP_TOLERANCE: f32 = 1.0;

/// Efficiency of the tyre (temperature and wear are not integrated: open, `ride6-tyre.md` section 7) and the share of the
/// efficiency loss that goes to the longitudinal force (`UTireData::TireEfficiencyBalance` default, recovered).
pub const TYRE_EFFICIENCY: f32 = 1.0;
pub const TYRE_EFFICIENCY_BALANCE: f32 = 0.5;

/// The runtime multiplier of the slip correction (wet and burnout surfaces change it; 1 on a dry road).
pub const SLIP_CORRECTION_MULTIPLIER: f32 = 1.0;

/// The bump stop at the end of the travel, as a multiple of the spring rate, used when a tyre states no radial stiffness
/// (with one, the tyre takes the overload). Open: the binary only clamps the travel (`ride6-suspension.md`).
pub const BUMP_STOP_FACTOR: f32 = 12.0;

/// The sign of the brake coupling force on the suspension: the force is `(T_applied - T_back)` times a factor, and the sign of
/// that difference for a braking wheel is not pinned down. -1 makes braking compress the fork (dive), the reading of the
/// factor's name (`ride6-suspension.md`, "Chain pull, traction and brake coupling").
pub const BRAKE_COUPLING_SIGN: f32 = -1.0;

/// The fall tests of the lean and the pitch run at every speed; those of the heading angle and the angular speed only above
/// the minimum speed of the spec. Open: the exact tests are inferred (`ride6-lean-controller.md`, "Fall detection").
pub const LEAN_AND_PITCH_FALL_AT_ANY_SPEED: bool = true;

/// The rider help that cuts the throttle when the front lifts and eases the front brake when the rear does is a linear ramp
/// of the lift angle and of its speed between the two numbers of each range, as the weight control of the same element is.
/// Open: its own formulas were not read (`ride6-rider-help.md`, "Rider help"). The lift angle is the angle of the line through
/// the hubs from the ground (no estimate of the pitch from the body).
pub const RIDER_HELP_RAMPS_ARE_LINEAR: bool = true;

/// Rolling resistance has no slip term (`RollingResistanceSlip` of the tyre row): the model's formula was not read.
pub const ROLLING_HAS_SLIP_TERM: bool = false;

/// Numerical guard: slower than this (m/s) the bike counts as standing for the tests that divide by the speed.
pub const STANDING_SPEED: f32 = 0.5;

/// Whether the rider leads the handlebar by the time constant of its filter (`Bike::lead_the_handlebar`). Open: the reference
/// steers the front wheel along the direction of its own contact point through a lagging handlebar filter (`K` 1000, ratio 3:
/// a time constant of 0.18 s), and without a stabiliser (the stability help is off for the stock bike) such a wheel pushes the
/// nose of a braking bike round: the front tyre is stiffer than the rear one when the weight is forward, the lag makes the
/// front force add to the yaw instead of damping it, and the bike spins out within a second of an ordinary stop. What keeps
/// the reference stable (a finer step, a tyre relaxation, a rider term) was not recovered, so the rider here advances the
/// handlebar's input by the filter's own time constant. The value is derived from the filter, not tuned.
pub const HANDLEBAR_LEAD: bool = true;
