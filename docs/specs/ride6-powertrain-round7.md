# RIDE 6 powertrain and brakes, round 7 (clean-room spec)

The brake reduction of the ABS element, the engine-brake reduction, the over-limit upshift, the wheelie help's torque ramp and the end
of the feathering start. Continues [ride6-powertrain-round6.md](ride6-powertrain-round6.md); function addresses in
[provenance/ride6-physics-A.md](../provenance/ride6-physics-A.md). Sources: the RIDE 6 executable, build `5.6.1-1697004`, and the
resolved data of the 319 vehicles. Tags: **[B]** read from the binary, **[D]** data. The KTM's handler is the road-bike handler
(`UMotorbikeInputHandler`, constructor `0x1450fbfa0`); the fields read here belong to the base classes or to the ABS element and the
engine, which every handler shares.

## 1. Brake reduction, deprecated logic [B]

`bUseDeprecatedLogic` is true by constructor default and no bike sets it, so the element uses `0x1450ce620`. Per wheel the six
tunings of `FBrakeReductionTuning` are range translators `T(x) = out.x + ramp(x; in)^linearity * (out.y - out.x)`, with the input
replaced by its absolute value where `UseAbsoluteValue` is set. The constructor of the struct (`0x14509cd80`) fixes the defaults:
**input and output ranges all zero** (a translator nobody set outputs 0 for every input), linearity 1, no curve, and
`UseAbsoluteValue` **true for the lean and the drift translators, false for the pitch, weight, load and throttle ones**. (This settles
the doubt of round 6.) The inputs are the IMU channels and the rider's original action values:

```
speed_t    = S(IMU.SpeedKMH)                    # the element's BrakeReductionSpeedTuning
sum        = T_weight(WeightLongitudinal) * T_pitch(predicted pitch angle)
           + T_drift(DriftAngleDeg) + T_lean(LeanPercentage) + T_load(TireLoad / max(RestLoad, 1e-4))
reduction  = speed_t * Intensity * clamp01(sum) * T_throttle(original Throttle)
```

The wheel's brake multiplier is `(1 - reduction) * (1 - abs)` (round 6, section 5). The reduction is filtered like the ABS release,
rising at 10000 per second and falling at `BrakeReductionFallRate` (constructor default 1000, no bike sets it).
KTM [D]: the speed term 10 to 25 km/h gives 0 to 1. The **front** states lean (0 to 0.7 gives 0 to 0.2), drift (0 to 30 deg gives 0
to 1) and load (0 to 1 gives 1 to 0) but **no throttle tuning**, so its throttle term is the all-zero default and the front reduction is
**0 at every moment**; the **rear** has lean (0 to 0.9 gives 0 to 0.5), drift (0 to 35 deg gives 0 to 0.2), load (1 to 0 gives 0 to 0.8) and
throttle (0 to 0.05 gives 1 to 0), so it loses the brake as the rear unloads, the bike leans or drifts, with the throttle shut and above
10 to 25 km/h. No bike of the install states a weight, pitch, curve or linearity tuning, nor a `UseAbsoluteValue` other than the default.

Modelled (`bike/brake_reduction.rs`, `AntiLock::update_reduction`); the weight and pitch terms are not (their product is 0 for every
bike). The inputs: the lean over the lean limit, the angle between the heading and the velocity in degrees, the wheel's load over its
load at rest (the weight of the bike on that wheel, from the mass and the centre of gravity), the rider's own throttle. The manifest
table is `[bike.brakes.abs.reduction]` with `.front` and `.rear`; each translator is `[in low, in high, out low, out high]`.

## 2. Engine-brake reduction [B]

`0x14514c850` (the engine step) calls `0x1451507b0` for the share `r` of the closed-throttle drag that is dropped
(`bUseDeprecatedEngineBrakeReduction`, true by default, kept by every bike; with it false the torque is `Throttle (T_max + T_fric) - T_fric`).
With `F` the friction curve value at the engine speed (1 at the rev limit):

```
over = max(F - FrictionPeakTorqueLimit, 0)                                    # constructor default 1: only the curve past the limit
c    = Clutch lever if bLerpEngineFrictionSettings else 0                     # default false
k    = (FullClutchEngineBrakeReductionFactor - EngineBrakeReductionFactor) * c + EngineBrakeReductionFactor
r    = clamp01( over + T_lean(|NormalizedLeanAngle|) * T_load(NormalizedTireLoad.y) * k * T_slip(RearSlip) )
T_closed = -T_fric * (1 - r)         T_engine = T_closed + (T_max - T_closed) * Throttle
```

The inputs come from the vehicle's IMU (`FVehicleIMUData`): `NormalizedLeanAngle` is the lean over the lean limit, signed; `NormalizedTireLoad.y`
is the rear tyre's load over a reference load of the handler (its load at rest); `RearSlip` is `(R w - v) / max(v, 1 cm/s)` of the rear
wheel (negative when the wheel is held back). The constructor defaults of the engine data (`0x1450f9ff0`): `FrictionPeakTorqueLimit` 1,
`EngineBrakeReductionFactor` 0, no lerp, `FullClutch...Factor` 0, and the three translators with input `(0, 0)` and output `(1, 1)`, that is **1
for every input** (the lean one is also read by its absolute value always). The same `r` multiplies the friction in the drive torque.

KTM and 316 other vehicles [D]: factor 1; load tuning 1 to 0 gives 1 to 0.9 (0.5 for some); slip tuning 0 to -0.33 gives 0 to 1 (-0.5, -0.6
for others); no lean tuning, no limit. So `r = ` the slip term times the load term: the drag fades out as the rear wheel is held back
by 0 to 33 percent of the road speed, and is gone beyond; and `r` is 0 while the rear rolls. Modelled in `Powertrain::engine_torque`
(`bike/engine_brake.rs`); the manifest keys of `[bike.engine]` are `brake_reduction_factor`, `brake_reduction_load` and
`brake_reduction_slip` (each `[in low, in high, out low, out high]`; a tuning left out gives 1).

## 3. Over-limit upshift [B]

Round 6, section 2 has the rule. The constructor defaults [B: `0x1450bd6d0`]: `GearChangeOverLimitEnabled` true,
`OverLimitMinChangeSpeed` 10 km/h, `OverLimitChangeTime` 1 s, `OverLimitChangeTimeThreshold` 0.5 s, `OverLimitRpmPercentage` 0.99. Data [D]:
`OverLimitMinChangeSpeed` 60 (50 for 17 bikes) km/h, `OverLimitChangeTime` 0.064 (0.3 for 412), `OverLimitChangeTimeThreshold` 1.0 (0.65),
`OverLimitRpmPercentage` 0.94 (0.91 for 3). The time since the last change restarts when a change is requested, and the time over the
limit restarts with it. Modelled (`OverLimit` in `clutch_control.rs`; manifest keys `over_limit_*`); the handbrake and burnout conditions are
not (the model has no handbrake input at the gearbox and no burnout).

## 4. The wheelie help's torque ramp [B]

`0x145100f80` multiplies the help by `ramp(TorqueCurve(EngineRPM); HelpIntensityTorqueTuning)`: the **engine's torque curve** (the normalised
`TorqueCurve` of the engine data, evaluated at the engine speed; the argument register holds `Engine.EngineRPM`), not the torque now.
`HelpIntensityTorqueTuning` is `(0, 0)` in the base asset and in every vehicle, a step at 0, and the curve is never negative, so the factor is
**1 for every bike**. It is not wired into `pitch_assist.rs`: it would be a constant. (Round 6 said it was 0 under engine braking; that
read the operand wrong.)

## 5. The end of the feathering start [B]

`0x1450c0290` ends with: if the clutch input is not exactly 0, return; else reset the feathering timer. With `0x1450d7cd0` (the start ends
when the clutch input is 0) that makes the controller's result 0 end the start, which happens when it is inactive, when its stop condition
holds, or when the clutch torque asked for is more than the clutch carries. The library now ends the start in exactly those cases. In the
model's launch the result never reaches 0 before the release speed (a full-throttle start without the bound stays in first gear with
the clutch slipping at 19 m/s), so the bound at `ClucthReleaseVelocityRange.Y` stays as a documented stand-in; what makes the
reference's clutch input reach 0 at speed (its clutch capacity or its engine torque differing from this model's) was not found.
Round 8 ([ride6-feel-round8.md](ride6-feel-round8.md), section 5) re-read the controller (for the KTM the exit is out of reach in first gear), replaced the
model's lock and slip logic, which chattered, and kept the bound.
