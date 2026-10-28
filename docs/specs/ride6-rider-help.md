# RIDE 6 wheelie, stoppie, rider help, ABS and traction control (clean-room spec)

Data: [ride6-physics-elements.md](../formats/ride6-physics-elements.md). Mapping to our model:
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Sources: [provenance](../provenance/ride6-physics.md).
Tags: **[B]** read from the binary, **[I]** inferred from the structure and the field names, **[D]** data only. The
controllers are `UVehicleElement`s ticked in `SortingOrder` after the input handler; they change **inputs and torques**, not
the tyre model.

## Wheelie and stoppie control (`UWheelieControl`)

Two instances of the same parameter set `FWheelieControlParams`: `WheelieParams` (front wheel up, rear tyre on the ground) and
`StoppieParams` (rear wheel up). The C++ defaults [B: `0x1450b3fa0`] are: `InputDeadzone 0.1`, `MinSpeed 15` km/h,
`MaxAngle 30` deg, `MaxAngleFactorInputTuning (0, 1)`, `MaxAngleFactorSpeedTuning (0, 0)`, `GroundDistanceFactorRange (0, 0)`,
`HelpIntensityLeanTuning (0, 1)`, `HelpIntensityTorqueTuning (0.1, 0.2)`, `HelpIntensityScale 1`, `Acceleration 10`,
`AccelerationTime 0.005`, `LookaheadTime 0`, `bFilterTargetAngle true`, `timescale 10`. The KTM overrides the dead zone
(0.3 / 0.95), `MaxAngle` (35 / 45), speed tuning (0..15), ground range (0..1), lean tuning (0.2..1), torque tuning (0, 0),
`AccelerationTime 0.1`, `bFilterTargetAngle` false, stoppie look-ahead 1 s and `timescale 2.5`, and the speed curve
`StoppieControl`.

**Input.** The rider's longitudinal weight input `W` (-1 back to 1 forward; with `UseCombinedInput` the mean of the weight and
the rider-position input). [B: `0x145136ea0`]

```
if W >= 0:  target_position =  clamp01((W - Stoppie.InputDeadzone) / (1 - Stoppie.InputDeadzone))       # stoppie side
else:       target_position = -clamp01((-W - Wheelie.InputDeadzone) / (1 - Wheelie.InputDeadzone))      # wheelie side
```

The help acts when `bApplyOnWheelie`/`bApplyOnStoppie` is set, the wheel is in contact with one of the listed surfaces, the opposite
pedal is not pressed (a stoppie needs the throttle below 0.1, a wheelie the brake below 0.1) and the speed is at least
`MinSpeed`. [B: `0x145122dc0`]

**Target and tracker.** [B: `0x145100f80`, `0x145137060`]

```
a*   = sign * |target_position|-scaled MaxAngle * input_factor(|target_position|) * speed_ramp          # radians, MaxAngle in deg
a    = CurrentWheelieAngle - TerrainInclination * TerrainInclinationInfluence - PitchSpeed * LookaheadTime
a_cmd = tracker(a*, a)       # angular acceleration command, |a_cmd| <= Acceleration (rad/s^2), time-optimal (bang-bang with a
                             # square-root approach profile so the pitch arrives at a* with zero rate), AccelerationTime sets the
                             # smoothing of the switch; with bFilterTargetAngle a* is filtered first
```

`speed_ramp = clamp01((speed - MaxAngleFactorSpeedTuning.x) / (... .y - ... .x))` and `input_factor` interpolates
`MaxAngleFactorInputTuning` by `|target_position|`.

**Help torque and its arm.** There is **no force and no arm**: a pure torque is applied to the rigid body about the chassis
pitch axis (the chassis Y axis, rotated into the world):

```
help      = (1 - ramp(LeanPercentage; HelpIntensityLeanTuning)) * speedCurve(speed_kmh)   # default 1 where the curve is empty
          * (1 - ramp(wheel_gap; GroundDistanceFactorRange)) * ramp(normalised_engine_torque(rpm); HelpIntensityTorqueTuning)
          * HelpIntensityScale
tau_help  = help * a_cmd * (CHASSIS.Mass * InertiaTensorScale.y) * 10000        # Unreal torque units, pitch inertia of the bike WITHOUT rider
```

`ramp(x; (a, b)) = clamp01((x - a) / (b - a))` (a step at `b` if `a == b`); `wheel_gap` is the height of the wheel that should
rise above the ground (the help fades once that wheel is far off the ground); `normalised_engine_torque` is the torque curve
value at the current rpm. [B for the factors and the torque form; the exact sign of the wheel-gap term and the operand of
the speed curve are read from the register use, [I]] For the KTM the torque tuning is (0, 0), so that ramp is 1; the wheelie
speed curve is empty and evaluates to its default 1, so only the lean ramp, the ground ramp and the scale shape the help.

The result keeps a wheelie at the angle the rider's weight asks for (up to 35 deg) and a stoppie at up to 45 deg instead of
letting the bike flip.

## Rider help (`URiderHelpVehicleElement`)

An assistant that moves the rider's longitudinal weight (`FRiderComponent` position, see the rider-body spec) and
modulates throttle and brake, when `bEnableAutoRider` and the speed is above `AutoRiderMinSpeed` (5 km/h). All of it
works on input channels, not on forces. [D + I]

- **Weight control** (`bEnableRiderHelpWeightControl`) [B: `0x1450ff610`]: with `theta` the estimated pitch angle in degrees
  (positive front up; look-ahead `AutoRiderLookaheadTime`), the commanded longitudinal position is

  ```
  range  = theta > 0 ? AutoRiderWheelieWeightControl : AutoRiderStoppieWeightControl          # KTM both (3.6, 4.0) deg
  A      = clamp01((|theta| - range.x) / (range.y - range.x)) * sign(theta) * AutoRiderWeightControlMultiplier   # multiplier default 1
  B      = bEnableGForceWeightControl ? GForceWeightControlMultiplier * ramp(|g|; GForceWeightControl)         # KTM (0.5, 0.75) g
                                      : ramp(|pitch speed|; PitchSpeedWeightControl) * sign
  position = clamp(A + B, -1, 1)  ;  blended into the rider by AutoRiderPositionEngageSpeed / ManualPositionEngageSpeed
  ```

  In the start control (`bEnableAutoRiderStartControl`, defaults: position 1 dry / wet, speed limit 10 km/h, threshold 50 km/h,
  throttle limit 0.8, brake limit 0.2) the position is pushed to `AutoRiderStartPositionDry/Wet`. Defaults: `AutoRiderMinSpeed`
  10 (KTM 5), `WeightLongitudinalInputDeadzone` 0.8 (KTM 0.75): the auto rider yields to the player's input beyond the dead zone.
  The throttle and brake terms (`AutoRiderWeightControlOnThrottleTuning`: ramp 0 at no throttle to 1 at 10 % throttle;
  `...OnBrakeTuning`) enter A in the same way. The sign convention of B and the engage filter are [I].
- **Throttle control** (`bEnableRiderHelpThrottleControl`): cuts the throttle when a wheelie goes beyond the pitch ranges
  (`AutoRiderWheelieThrottleControl` 0 to 4 deg pitch, `AutoRiderForcedWheelieThrottleControl` 33 to 45 deg, pitch speed 0.2
  to 0.6 rad/s) through the engine's electronic throttle modulation; interventions are recorded in
  `ThrottleControlInterventionData`.
- **Brake control** (`bEnableRiderHelpBrakeControl`): relieves the front brake when a stoppie grows (pitch 0 to 4 deg,
  pitch speed 0 to 0.6, g-force 1.6 to 2.0).
- **Tuck-in** (`bAutoTuckIn`): above 90 km/h, with the lean below the maximum percentage 1, throttle above 0 and brake below
  0.1, the rider tucks in (`TuckInPosition` 2): reduced air drag.
- **Lateral weight**: moves the rider to the inside of a turn from the banking angle (`LateralWeightControlBankingThresholds`
  60 and 60 percent of the lean limit), with the curve `Racing_RiderHelp_LateralWeightSteerInfluence` of the steer.

The curves and ranges are in the dump. The mapping from the pitch estimate to the commanded position is a linear ramp
between the two numbers of each range [I].

## ABS (`UABSVehicleElement`) [B: `0x1450e0240`; thresholds D]

Per wheel and per tick the element computes two intensities in 0..1, each rising to its target at once and falling at a
fixed rate (`ABSFallRate`, `BrakeReductionFallRate`), and multiplies the wheel's brake multiplier in the brake system by
`(1 - brake_reduction) * (1 - abs)` (it also writes the handbrake multiplier of wheels with `bAffectedByHandbrake`, so the rear brake is reduced the same way):

- **ABS** `abs = Intensity * speed_ramp * ramp(NormalizedSlipRatio)`: `speed_ramp` goes from 0 at 10 km/h to 1 at 25 km/h; the
  ramp goes from 0 at a normalised slip ratio of -0.3 to 1 at -0.5 for the front (-0.5 to -1 for the rear). The normalised
  slip ratio is the wheel's `nx` of the tyre spec (-1 is the force peak), so the ABS starts to release the brake well
  before the peak. [I: the product]
- **Brake reduction** (the cornering/lift protection): reduces brake input as a function of the lean angle (front: 0 at 0 lean
  to 0.2 reduction at 0.7 normalised lean; rear 0 to 0.5 at 0.9), the drift angle (front 0 to 1 over 0..30 deg; rear 0 to
  0.2 over 0..35 deg), the tyre load (front: full reduction at zero load, none at full load; rear: none at full load rising to 0.8 at zero load)
  and, for the rear, the throttle input (full reduction at no throttle, none from 5 % throttle). [D: ranges; I: the form]

## Traction control (`UTCSVehicleElement`) and engine electronics (`UECUVehicleElement`)

TCS watches the driven wheel's slip: between `TCSSlipThresholdsDry` (0.09 to 0.2, wet 0.07 to 0.15) it raises an intensity
that cuts the throttle through `ThrottleModulationIntensity` of the engine (the engine blends the throttle towards the
zero-torque point, see the drivetrain spec), shaped by the lean (`ThrottleModulationNormalizedLeanRange` 1 to 1) and the yaw
rate (`ThrottleModulationYawRange` 0.08 to 0.17). Below 5 km/h (`TCSSpeedThresholds`) it is off; `LimiterIndex 1` selects
the launch limiter (64 % of the rev limit, up to 45 km/h) for a start. The ECU element reduces the engine-brake torque
(`EngineBrakeReduction`, from speed, slip, lean and drift tunings) and the throttle (`EngineThrottleReduction`, from slip,
wheelie angle and lean). All of them are products of 0..1 ramps over the listed ranges. [D + I]

## What was not recovered

The exact weight-control target function of the rider help, the ramp functions of the TCS/ECU/ABS (the ranges are the
data; the interpolation is linear between them unless the asset gives a curve), the torque arm of the wheelie help and
the application point of its torque.
