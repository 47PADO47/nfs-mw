//! The modelling choices of the two-wheel model: every number or structural decision that neither the data of a bike nor
//! a published formula nor the recovered behaviour of the reference game fixes. Each has its reason and the spec that names
//! it as open (`docs/specs/vehicle-two-wheel.md`, `docs/specs/vehicle-two-wheel-parameters.md`). When a fact turns up, change
//! the constant here and nowhere else.

/// The clutch is open for the whole of a gear change and engaged afterwards (open: the data's `ClutchTimingOnGearChange`
/// ramps the clutch out and back in over fractions of the change, but the exact ramps are not pinned down). The engine runs
/// free during the change, as the drive is cut.
pub const CLUTCH_OPEN_DURING_SHIFT: bool = true;

/// The time the clutch takes to engage again after a gear change (s). Open, with `CLUTCH_OPEN_DURING_SHIFT`: without a ramp
/// the engine and the wheel would be forced to one speed in a single step. About the time of a change up.
pub const CLUTCH_ENGAGE_TIME: f32 = 0.06;

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

/// The share of the efficiency loss that goes to the longitudinal force: the `TireEfficiencyBalance` constructor default of the
/// tyre data (recovered, `ride6-tyre-thermal.md` section 1; the lateral force gets the rest).
pub const TYRE_EFFICIENCY_BALANCE: f32 = 0.5;

/// The air and the road temperature (deg C) the tyre cools towards. Open: the track and the weather set them and no default was
/// found (`ride6-tyre-thermal.md` section 1). A mild day with a road a little warmer than the air.
pub const AIR_TEMPERATURE: f32 = 20.0;
pub const ASPHALT_TEMPERATURE: f32 = 30.0;

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

/// Numerical guard: slower than this (m/s) the bike counts as standing for the tests that divide by the speed.
pub const STANDING_SPEED: f32 = 0.5;

/// The road speed (m/s) below which the bike counts as being at a start: with the throttle shut the clutch is pulled (the engine
/// idles), with it open the launch clutch acts. Open: `InStartingSequence` and the neutral logic of the handler were read but
/// not written up (`ride6-gearbox-auto.md`, section 2).
pub const START_SPEED: f32 = 1.0;

/// The least lean limit (rad) the rider's place divides the target lean by, so a limit that has not been planned yet (0) asks for
/// no lean shift rather than a division by zero. Numerical guard.
pub const MIN_LEAN_LIMIT: f32 = 0.05;

/// Which side the normalised lean that places the rider is on: the sign of the binary's target lean is that of the stick, and
/// the stick to the right is a lean to the right in this model, so a positive lean shifts the rider by the lateral offset of the
/// data as it stands (x toward the right for a positive x). Open: the binary does not say which side its positive lean is; the
/// data is consistent with it (a supermoto's lateral offset puts the rider on the outside of the lean, upright over the bike,
/// and a scooter's on the inside) (`ride6-rider-body-aero.md`, "Target place").
pub const POSITIVE_LEAN_IS_RIGHT: bool = true;

/// The lateral weight input moves at the rate of the longitudinal one (`InputData.WeightLongitudinal.InputRate`). Open: the
/// lateral channel's own rate was not read.
pub const LATERAL_WEIGHT_RATE_IS_LONGITUDINAL: bool = true;

/// The rider's spring error is the plain difference between the asked-for place and his place. The binary's newer formulation
/// (`bUseNewRiderDelta`, set by every bike) measures it as an arc about the bike's centre of gravity; for the few centimetres
/// the rider moves the two differ by well under a millimetre, so the linear difference stands in.
pub const RIDER_DELTA_IS_LINEAR: bool = true;

/// How the posture changes the air drag: the multiplier is the drag curve of the rider's posture (his tuck, 0 to 1) over its
/// value upright, applied to the whole drag. Open: the binary builds a drag force on the rider from `DragCurve`, `DragScale` and
/// `DragScaleCurve` in a function that was not read; the bike's own drag coefficient stays the measured upright one.
pub const RIDER_DRAG_RELATIVE_TO_UPRIGHT: bool = true;
