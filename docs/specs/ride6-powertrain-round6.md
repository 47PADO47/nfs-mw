# RIDE 6 powertrain and brakes, round 6 (clean-room spec)

What this round recovered about the timing of a gear change, the starting rules of the gearbox handler, the filters of the slipper
clutch and of the anti-lock brake, the source of `MaxTorqueRPM` and the sign of the brake coupling, and what is still open.
It corrects and completes [ride6-gearbox-auto.md](ride6-gearbox-auto.md) and
[ride6-engine-drivetrain.md](ride6-engine-drivetrain.md). Sources: the RIDE 6 executable, build `5.6.1-1697004`, and the
data assets of the KTM 450 SMR 2022; the function list is in [provenance/ride6-physics-A.md](../provenance/ride6-physics-A.md).
Tags: **[B]** read from the binary, **[D]** data, **[I]** inferred. Speeds are cm/s inside the reference and m/s here, `p` is the
progress of a gear change (elapsed time over its total time, 0 to 1).

## 1. Timing of a gear change [B]

The handler (`UInputHandler`) keeps the length of the change under way `T` (`TotalGearChangeTime`, `GearUpChangeTime` or
`GearDownChangeTime` when a change is requested), the time since it started `t` (`TimeSinceLastGearChange`) and the requested
gear. A request is accepted only if no change is under way (the requested gear equals the present one) or its length does not
exceed the running one. Every tick `t` grows by the step and, once `t > T > 0`, `T` is set to 0: the change is over.

While `T > 0` the handler runs the timing below on the action struct of the vehicle. With `ClutchTimingOnGearChange = (a, b)` and
`ThrottleTimingOnGearChange = (c, d)` (all fractions of the change; `T` below `1e-8` makes `p` 1 once `t >= T`, else 0):

```
ramp(x, from, to) = clamp((x - from) / (to - from), 0, 1)           # a step at `to` when the two are equal
S(p, (u, v))      = ramp(p, u, 0)  if p < u  else  ramp(p, v, 1)     # 1 at p = 0, 0 at p = u, 0 up to v, 1 at p = 1

clutch   = max(clutch, 1 - S(p, (a, b)))            # 0 at the start, 1 at p = a, 1 up to p = b, back to 0 at p = 1
throttle:
    if (c, d) != (0, 0):          throttle *= S(p, (c, d))                    # 1 -> 0 over [0, c], shut to d, back over [d, 1]
    elif ReleaseThrottleOnShift:  throttle  = 0
    elif ModulateThrottleOnUpShift or the change is down:   throttle *= S(p, (a, b))
```

The gear itself changes (`SetGear`, which also sets the ratio) in the step where `t >= T * a`, that is when the clutch has just
gone fully out. So the clutch lever of a change is a trapezoid on the rider's clutch input, and the drive is cut for the part of
the change in which the lever is high. KTM: `(a, b) = (0.125, 0.2)`, `(c, d) = (0.05, 0.9)`, `ModulateThrottleOnUpShift` false,
`ReleaseThrottleOnShift` false (default), `GearUpChangeTime` 0.06 s, `GearDownChangeTime` 0.15 s [D]: the lever is 1 from 7.5 ms
(up) or 19 ms (down) to 12 ms or 30 ms, then falls linearly to 0 at the end; the throttle is shut from 3 ms to 54 ms (up).
With both timings `(0, 0)` (constructor default) the lever falls linearly from 1 to 0 over the whole change and the gear changes at
once.

The automatic decision is not taken while `T > 0` or while `InStartingSequence` is true (section 3).

**Blipper.** After the timing, for a change down (`bChangedUp` false) with `BlipperThrottleOnShift > 0`: if
`bUseDeprecatedBlipperLogic` (constructor default true, KTM leaves it) and the rear wheel's rim speed `R * w` is below the vehicle's
forward speed, `throttle = max(throttle, BlipperThrottleOnShift)` (KTM 0.3). The other branch (`bUseDeprecatedBlipperLogic` false:
`BlipperMinClutch`, `BlipperNoSlipSpeedFactor`, a comparison of the engine speed with the speed the wheel forces in the new gear)
is read but no bike uses it; it is not modelled.

## 2. The writers of the timers [B]

- `NoGearChangeTimer`, `NoGearUpChangeTimer`, `NoGearDownChangeTimer` are set only by a **manual** gear request on an automatic
  bike (`AutomaticGearOverrideTime`, or the up and down timers if `bAutomaticGearOverrideChangeBackOnly`), and decay by the step
  to 0. The KTM's handler has no manual path in use, so they stay 0.
- `TimeOverLimit` grows by the step while (the engine's limiter is reached, or `OverLimitRpmPercentage * MaxRPM < rpm`) and the
  handbrake is off and the transmission is automatic and the speed is above `OverLimitMinChangeSpeed` (km/h); else it is 0. An
  upshift is requested when `bGearUpOverLimitEnabled` and `TimeOverLimit > OverLimitChangeTime` and
  `TimeSinceLastGearChange > OverLimitChangeTimeThreshold` and no burnout. **Not modelled** (the generated change point of the KTM
  lies at 93.5 to 94.4 % of `MaxRPM`, the over-limit test at `OverLimitRpmPercentage` 94 %, so it only adds a change a little
  earlier under load).
- `ClutchFeatheringTimer`: 0 stays 0; a positive value grows by the step; a negative one grows more negative. The feathering
  controller sets it to a tiny positive value to start (the phrase "advanced elsewhere" of the gearbox spec).

## 3. The handler's state flags and the starting rules [B]

Flags, recomputed each tick from the action struct (`T0 = 0.1` is a constant of the binary):

```
IsAccelerating   = Throttle > 0.1 and the engine runs        IsBraking = Brake > 0.1         IsUsingHandbrake = HandBrake > 0.1
IsPressingClutch = Clutch > 0.1                              IsInReverse = gear < 0
IsAtLowSpeed     = RelativeSpeed < 1 m/s                     RequestingReverse = IsInReverse and IsBraking and not handbrake
IsDoingBurnout   = |forward speed| < BurnoutMinSpeed (km/h) and accelerating and braking and not handbrake and not clutch
                   and gear > 0 (gear >= 0 with bEnableBurnoutInNeutral)
HasStopped       = not accelerating and (not braking or bReverseOnlyWhenStopped) and (gear = 0 or Clutch = 1) and IsAtLowSpeed
```

Both speed tests of the handler are **1 m/s**: `IsAtLowSpeed` on `RelativeSpeed`, and "moving" (`forward speed >= 1 m/s`, signed) in
the start logic. This is the fact behind the model's low-speed threshold.

Start logic of an automatic bike with the clutch not held (the order of the reference):

1. *Moving* means `forward speed >= 1 m/s and gear != 0`. If moving, or `InStartingSequence` and not (accelerating and in
   reverse): only "in reverse gear and rolling forward fast (> 1 m/s) and not starting" asks for neutral (gear 0).
2. Otherwise (slow, or neutral), if not (braking and not handbrake and not accelerating and not in reverse and gear >= 0):
   - if `IsAccelerating and not IsBraking and (IsAtLowSpeed or gear <= 0)`: **`InStartingSequence = true`, no reverse request,
     request gear 1 with a change time of 0** (immediate);
   - else, if not burning out: with throttle, brake and clutch all off and the engine at or below `MinRPM`, request
     `GearWhenStopped` (KTM 1; if it is not 0 the clutch is set to 1); with a burnout request gear 1.
3. Braking at a standstill (the rest): reverse is requested (`RequestingReverse`) when the bike has stopped (or reverse is allowed
   moving), else with gear > 0 and not `EnableReversePressingBrakeWhenStopped` neutral is requested.

The starting sequence ends (`InStartingSequence = false`, `T = 0`) when the whole clutch input is back to exactly 0, or when the
throttle is released (then the clutch is set to **1**: fully pulled).

**Launch clutch**, while `InStartingSequence`: as in [ride6-gearbox-auto.md](ride6-gearbox-auto.md) section 3, unchanged.

**Clutch before the tick's logic** [B: handler pre-update]: if the rider's throttle is at most 0.1: `IsAtLowSpeed` pulls the clutch
(1); else with `PressClutchAtMinRPM` (constructor default true, KTM true) the **idle clutch**:

```
v_idle = MinRPM * (2 pi / 60) / GearRatio * R_rear                # road speed at which the present gear turns the engine at its idle
clutch = max(clutch, 1 - ramp(R_rear * w_rear, v_idle * lo, v_idle * hi))         # (lo, hi) = MinRPMSpeedScaleRange = (1.05, 1.1)
```

So a coasting bike gets the clutch pulled between 105 % and 110 % of the idle road speed of its gear and the engine is not dragged
below its idle. `GearRatio` includes the final ratio.

## 4. `MaxTorqueRPM` [B]

The engine setup walks the **keys** of the torque curve (the external curve), not the interpolated curve. `MaxTorqueRPM` is the
rpm of the first key with a value strictly larger than every key before it, the running maximum starting at 0 (so 0 if no key is
positive); `MaxPowerRPM` and `MaxPower` take the key with the largest `rpm * value`, and `MaxPower = PeakTorque * 1.0472e-8 *
max(rpm * value)` in the units of the engine power. For the KTM the curve peaks at its key at 7159 rpm (value 1.0), which is
`MaxTorqueRPM`. The library takes it from the same keys.

## 5. Filters [B]

**Slipper clutch** (`SlipperClutchFilterSpeed`, a field of `FTransmissionData` at `0x3C`, constructor default **0**, set by no
bike). The unfiltered opening is `target = ramp(delta_torque, R.x, R.y) * SlipperClutchMaxIntensity`; if the filter speed is above
`1e-4` the opening `s` is filtered in the square-root domain, both ways:

```
gap = s_old - target ;  r = sqrt(|gap|) - FilterSpeed * dt
s   = target                         if r <= 0
s   = target + r^2  (gap > 0)  or  target - r^2  (gap < 0)  otherwise
```

With the default 0 the slipper follows its target at once. (The earlier modelling choice, a linear fall of 10 per second, is gone.)

**ABS** (`ABSFallRate` at `0x2A8` of the element, constructor default **10000**, in no bike's data): each wheel's release is
filtered by the rate limiter used for several elements: `diff = target - release`; if the release moves away from 0 (or from 0 to
anything) the rate is 10000, otherwise `ABSFallRate`; with `step = rate * dt > 0` the result is `release + clamp(diff, -step,
step)`, with `step <= 0` the target. With 10000 per second and a step above a tenth of a millisecond, **the release follows its
target at once** in both directions. The brake multiplier of each wheel is `(1 - brake_reduction) * (1 - abs_release)`; for the wheel
with `bAffectedByHandbrake` (the rear) the same multiplier acts on the handbrake torque. (The earlier modelling choice, 8 per second,
is gone; the library keeps the rate as a parameter of `AntiLock::update_at` so a slower one can be tested.)

**Brake reduction** (`BrakeReductionFallRate` constructor default **1000**, `bUseDeprecatedLogic` constructor default **true**,
neither set by the KTM data). With `bUseDeprecatedLogic` the element uses an older function (`ce620` in the provenance list) in
place of the one that reads `BrakeReduction*Tuning` as described in the data; it adds range-translated terms of the lean (IMU
channel `0x183C`), a second IMU channel (`0x1848`), the wheel's load share (`TireLoad / RestLoad`), a drift angle (rear and front
alike) and, for the rear, the throttle, clamps the sum to 0..1 and scales it by the intensity. The pairing of the terms is in
section 9; the reduction is not modelled yet.

## 6. Sign of the brake coupling [B]

Per wheel the drivetrain update writes `AppliedTorque` (the net torque that changed the wheel's speed over the step) and
`BackTorque = AppliedTorque - Z`, where `Z = -sgn(w) (BrakeTorque + DragTorque)` is the signed torque of the brake and the drag
(`w` the wheel's rotation speed; the sign of the tyre torque when `w = 0`). So `AppliedTorque - BackTorque = Z`: for a wheel that
rolls forward (`w > 0`, the drive adds positive torque, the rolling speed `v / R` is positive) it is **negative** while it brakes.
The suspension's brake coupling is `F_brake = (AppliedTorque - BackTorque) * 0.01 / R_cm * BrakeTorqueCouplingFactor * f_brake(s)`
(factor 30 rear, 35 front, positive), added without a sign flip to the spring and damper force, which is positive when it resists
compression (the chain pull and the traction terms are negated in their writers; this one is not). Hence **braking compresses the
suspension** (the fork dives): `BRAKE_COUPLING_SIGN = -1` for the magnitude of the brake torque. Reversing (`w < 0`) would flip it.

## 7. What was checked and found not to exist

- `StartSpeed`-like constants: none; the two 1 m/s tests of section 3 are the only speeds of the start logic.
- A torque-ramp parameter of the engine: the engine data (`FEngineData`) has none. The "torque ramp" is the wheelie help's
  `HelpIntensityTorqueTuning` ([ride6-tricks.md](ride6-tricks.md)); its operand is read in round 7
  ([ride6-powertrain-round7.md](ride6-powertrain-round7.md), section 4), and it is 1 for every bike. (This line first said it was
  0 under engine braking; the operand is the torque curve at the engine speed, not the torque now.)
- Engine electronics for the KTM: its `VehicleElements` list has no `ECUVehicleElement`, so `ThrottleModulationIntensity`,
  `EngineBrakeModulationIntensity` and `PowerLossIntensity` stay 1, 1 and 0 for it. The engine's own **engine-brake reduction**
  (`EngineBrakeReductionFactor` 1 with the load and slip tunings, and `bUseDeprecatedEngineBrakeReduction`) is separate and is
  specified in [ride6-powertrain-round7.md](ride6-powertrain-round7.md), section 2.

## 8. Forced downshift under braking [B]

The KTM's handler derives from the road-bike handler (`UMotorbikeInputHandler`, constructor `0x1450fbfa0`, virtual table at image
offset `0x9e06b30`); the SDK offsets of its own fields (`0x1858` on) are the real ones. (The motocross handler `UMXBikeInputHandler`,
constructor `0x1450fb230`, lays the same fields out differently; an earlier reading of this section took it for the road bike's.)
The road-bike class overrides three virtuals of the shared tick, each calling the shared one first: the hold-off update
(`0x145137390`, over `0x1450f1c20`), the timers (`0x1451405c0`, over `0x1450f8a80`) and the property serializer. The shift
timing, start logic and flags of sections 1 to 3 sit in virtual slots that all the handler family shares (checked slot by slot),
and the fields they read are those of `UInputHandler` and `UBikeInputHandler`, whose offsets agree in every class below `0x1860`.

The hold-offs `PreventGearUpChangeFactor` and `PreventGearDownChangeFactor` are **per tick**: the brake update of the base handler
(`0x1450e0480`) zeroes them, the base hold-off update adds the lean and throttle terms of [ride6-gearbox-auto.md](ride6-gearbox-auto.md)
section 2 (the down factor clamped to 0..1), and then the road-bike override adds the forced downshift:

```
b      = ramp(front lever of the rider, ForceGearDownChangeBrakeInputRange)            # the lever before the linking of the brakes
timer  = 0 if b <= 0 else timer + b * dt                                              # every tick, in the timer override
if ForceGearDownChangeBrakeDuration < timer:
    f = clamp01( ramp(speed_kmh, ForceGearDownChangeSpeedInputRange) * b^ForceGearDownChangeBrakeLinearity * ForceGearDownChangeBrakeFactor )
    PreventGearUpChangeFactor   = max(PreventGearUpChangeFactor, f)
    PreventGearDownChangeFactor = PreventGearDownChangeFactor - f                     # may go negative: a downshift made more likely
```

A negative down factor `P` moves the change-down speed threshold up (`ride6-gearbox-auto.md`, section 2); at `P = -1` the downshift
is requested whenever the lower gear would not pass the rev limit. KTM: lever range 0.5 to 1, duration 0.3, factor 1, linearity 3,
speed range 150 to 150 km/h (a step at 150 km/h), so it acts only when braking hard from 150 km/h or faster. Constructor defaults of
the road-bike handler: factor 0 (off), duration 0, linearity 2, ranges zero. Modelled (`ForcedDownshift` in `clutch_control.rs`).

## 9. Brake reduction, deprecated logic [B]

The KTM's ABS element leaves `bUseDeprecatedLogic` at its true default, so the reduction comes from the older function
(`0x1450ce620`). The tunings of `FBrakeReductionTuning` that the data does not set are the translator constructor's all-zero ranges,
which output 0. Per wheel, with `S` the element's `BrakeReductionSpeedTuning` of the speed in km/h, `I` the `Intensity`:

```
sum       = T_weight(WeightLongitudinal of the rider) * T_pitch(predicted pitch) + T_drift(|drift angle|) + T_lean(lean percentage) + T_load(TireLoad / RestLoad)
reduction = S(speed_kmh) * I * clamp01(sum) * T_throttle(rider's throttle)
```

(`T_*` are the six range translators of `FBrakeReductionTuning`; the IMU channels are `LeanPercentage` (`+0x24`) and `DriftAngleDeg`
(`+0x30`), the throttle and weight are the rider's original action values, the load share is the wheel's `TireLoad / RestLoad`.) The
brake multiplier is then `(1 - reduction) * (1 - abs)` with the filter of section 5. For the KTM: the **front** sets no
`ThrottleInputTuning`, so its reduction is 0; the **rear** has speed 10 to 25 km/h to 0..1, lean 0 to 0.9 to 0..0.5, drift 0 to 35
deg to 0..0.2, load share 1 to 0 to 0..0.8 and throttle 0 to 0.05 to 1..0, so it takes the rear brake away when the rear unloads, the
bike leans or drifts and the throttle is shut. The `UseAbsoluteValue` doubt is settled in round 7: it is true by default for the lean and drift translators and false for the others
([ride6-powertrain-round7.md](ride6-powertrain-round7.md), section 1), where the reduction is also modelled.

## 10. Still open

Sections 9 and the engine-brake reduction, the over-limit upshift and the torque ramp moved on in
[ride6-powertrain-round7.md](ride6-powertrain-round7.md). What is left:

| Item | State |
|---|---|
| Non-deprecated blipper, burnout, reverse, manual shifts | Read in part, no bike path uses them. |
| Exit of the feathering start | Round 7, section 5: the controller's own exit is implemented, but the model does not reach it before the release speed, a bound the reference does not state for feathering. |
