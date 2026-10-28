# RIDE 6 engine, clutch, gearbox and brakes (clean-room spec)

Data fields: [ride6-physics-vehicle.md](../formats/ride6-physics-vehicle.md) (engine, transmission, brake) and
[ride6-physics-elements.md](../formats/ride6-physics-elements.md) (shift logic, joined brakes). Mapping to our model:
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Sources: [provenance](../provenance/ride6-physics.md).
Tags: **[B]** read from the binary, **[I]** inferred, **[D]** data only. Torque in the code is in Unreal units (1 N m =
10000); the formulas here use N m.

## Engine torque [B: `0x14514c850`]

Let `w = rpm`, `x = w / MaxRPM`.

```
T_max(w)  = PeakTorque * min(TorqueCurve(w), PeakTorqueLimit)                  # curve is normalised: 1.0 = PeakTorque
T_fric(w) = EngineFrictionPeakTorque * EngineFrictionCurve(x)                   # x = w/MaxRPM when NormalizeEngineFrictionCurveDomain
```

For the KTM: `PeakTorque 48`, curve 0.7375 at 1652 rpm to 1.0 at 7159 rpm to 0.6 at 11000 rpm (the **external** curve
`KTM_SMR_450_2022`); `EngineFrictionPeakTorque 30` and the curve `Base_EngineBrake` ((0,0), (1,1), (1.5,2), cubic): the drag
is 30 N m at the rev limit and about 10 N m at a third of it. `EngineFrictionPeakTorque` is therefore **the closed-throttle
drag torque at `MaxRPM`** (the earlier reading of the model was right, but the drag is not linear in rpm after the curve's
first segment).

Throttle: the engine keeps a smoothed `Throttle`:

```
zero_point = T_fric / (T_fric + T_max)            # the throttle at which the net torque is zero (1 if T_max <= 0 and T_fric > 0)
k          = ThrottleModulationIntensity if input > zero_point else EngineBrakeModulationIntensity   # ECU factors, 1 = no intervention
Throttle   = zero_point + (input - zero_point) * k          # electronic interventions scale the throttle about the zero-torque point
if below MinRPM (MinimumReached): Throttle = max(Throttle, ThrottleAtMinimum)                       # KTM 0.15
Throttle  *= 1 - PowerLossIntensity                                                  # fuel, turbo etc.
while the limiter is active:  Throttle *= 1 - limiter_modulator                      # cut (LimiterCutsFuelInjection)
```

The torque on the crankshaft, with `r` the engine-brake reduction from the ECU (0 to 1, from load, slip and lean tunings;
1 for the KTM defaults at full load):

```
T_closed   = -T_fric * (1 - r)
T_engine   = T_closed + (T_max - T_closed) * Throttle       # linear blend between closed-throttle drag and wide-open torque
```

If the engine is not running `T_engine = -T_fric`. `EnginePower = T_engine * rpm * 1.0472e-8` (kW with the Unreal torque
units). [B] for the blend and the drag, [I] for the exact smoothing intensities.

`bUseDeprecatedEngineBrakeReduction` is true by C++ default (the KTM does not set it): the engine-brake reduction `r` path above is the one used; with it false `T = Throttle (T_max + T_fric) - T_fric` without `r`.

**Rev limiter** [B: `0x1450f1b50`]: `EngineLimiter[i]` with `LimiterIndex` selecting the active entry. The limiter is reached when
`rpm/MaxRPM > MaxRPMScale` or, if `bLimitSpeed`, when the speed exceeds `MaxSpeed` (km/h). While reached, a modulator
(`DurationRange` 0.02 s, `FadeOutSpeed`) alternates the cut. KTM entries: 0 = 100 % of `MaxRPM` (the normal limiter); 1 = 64 %
with 45 km/h (launch limiter, used by TCS/launch control); 2 = speed limit 60 km/h; 3 = speed limit 99 km/h, modulator off.

## Clutch and drivetrain coupling [B: `0x14515ac20`]

The engine and the drivetrain are two inertias joined by a clutch. Engine inertia `I_e = MOI`. Wheels side: the rotating
momentum `L_w` and inertia `I_w` of the driven wheel(s) (the rear wheel's `Inertia`), referred to the engine side by the
gear ratio `G = gear * FinalRatio`:

```
w_e'  = w_e + T_drive dt / I_e                     # engine speed after the drive torque (rad/s)
w_w   = (L_w / I_w) * G                            # wheel speed seen by the engine
back  = (w_w - w_clutch) * (I_w / G^2) / dt        # "BackTorque": torque the drivetrain pushes back
delta = back - T_drive
slipper = clamp((delta - R.x) / (R.y - R.x), 0, 1) * SlipperClutchMaxIntensity         # R = SlipperClutchDeltaTorqueRange
slipper = rate-limited towards its target by SlipperClutchFilterSpeed
C       = max(clutch_input, slipper)               # opened by the rider or by the slipper
e       = clamp((1 - C)^ClutchLinearity, 0, 1)     # engagement
```

If `e >= 1` the clutch is locked: the engine and the wheels turn together with the combined inertia `I_e + I_w / G^2`
(the engine speed follows the wheel). Otherwise it slips: `T_clutch = sgn(w_e - w_w) * e * T_clutch_max`, applied to the engine
(slowing it) and, times `G`, to the wheels, with a check that the two speeds do not cross inside the step (if they would,
the clutch locks for that step). `T_clutch_max` follows `ClutchMaxTorqueType`/`ClutchMaxTorqueFactor` [B: `0x145157d80`]: type 0 `Factor` N m; type 1 (the C++ default, used by the KTM) `PeakTorque * Factor` with `Factor = 1`, i.e. **48 N m**; type 2 the torque that cancels the speed difference over one step (limited by the inertias); type 3 `|engine torque| * Factor`. The torque at the wheel is `T_clutch * G`
(no efficiency factor was found), shared by the differential ratios (rear drive: 1 to the rear wheel, 0 to the front).
`ClutchRPM` is the clutch's speed in rpm. The idle clutch (`PressClutchAtMinRPM`, `MinRPMSpeedScaleRange`) opens the clutch
when the engine falls to the minimum rpm. Engine-brake torque is the negative `T_engine` passed through the same clutch.

## Joined brakes [B: `0x1450e0480`]

The inputs are `Brake` (front lever) and `HandBrake` (the rear brake). Unless at a standstill (`IsAtLowSpeed`):

```
if JoinedBrakesFrontFactor <= 0 and JoinedBrakesRearFactor <= 0:   rear = max(rear, front * JoinedBrakesFactor)
else:   m = max(front, rear);  front = JoinedBrakesFrontFactor * m;  rear = JoinedBrakesRearFactor * m
```

KTM: `JoinedBrakesFactor 1` and both others 0, so pulling the front lever applies **at least the same** rear brake input.
At very low speed `LowSpeedBrakeTuning` adds braking authority (0.2 at 1 km/h falling to 0 at 20 km/h, cubic).

## Brake torque and its distribution [B: `0x145136640` setup, `0x145134de0` application, `0x14513fe00` disc temperature]

Setup (once, `UPhysicsVehicleBrakeSystem`): for each of the `n` wheels (the first `n/2` in the wheel array are the front)

```
share_i        = BrakeBalance            if i <  n/2      else 1 - BrakeBalance
BrakeTorque_i  = (1/n) * R_i[cm] * 200 * share_i * BrakeForce            # Unreal torque units
HandbrakeTorque_i = (wheel.bAffectedByHandbrake ? 200 : 0) * HandbrakeForce * (1/n) * R_i[cm]
```

with `R_i` the wheel's `ShapeRadius`. One newton metre is 10000 Unreal torque units, so for a two-wheeler (`n = 2`) this is
**`T_i = F * R` in newton metres**: `BrakeForce` and `HandbrakeForce` are **forces in newtons at the tyre patch**.
Every bike asset has `BrakeBalance = 1`, so the whole `BrakeForce` goes to the **front** wheel (share 1) and the rear gets
share 0 from `BrakeForce`; the rear brake is the **handbrake channel**: `bAffectedByHandbrake` is true only on the rear
wheel and `HandbrakeForce` drives it. KTM: front brake 1880 N, rear brake 1000 N.

Application (each step), with `brake` = the front brake input and `hand` = the rear brake input, both clamped to [0, 1]
(these are the `Brake` and `HandBrake` channels of the action struct, after joined brakes and ABS):

```
T_wheel_i = HandbrakeTorque_i * HandbrakeMultiplier_i * hand + BrakeTorque_i * BrakeMultiplier_i * brake
if the disc has a DiscTemperatureToEfficiency curve:
    eff_i   = clamp(max(curve(disc_temperature_i), EfficiencyLimit), 0, 1)
    T_wheel_i *= (eff_i - 1) * BrakeEfficiencyFactor + 1
```

The multipliers start at 1 each step and are lowered by ABS and brake reduction (see the rider-help spec); the front
multiplier acts on `BrakeTorque`, the rear's `ABS` result also acts on the handbrake multiplier of wheels with
`bAffectedByHandbrake`. The disc temperature integrates `|w| * T_wheel` heating against a cooling term that grows with the
speed (up to 83.3 m/s) and the ambient, clamped between the minimum and maximum disc temperatures
(`FBrakeDiscData`: C++ defaults 650 optimum, 1075 maximum, 20 minimum, mass 3 kg). [B for the structure; the cooling
constants `0.108`, `0.00026`, `1.14e-7` are read, their physical names are [I]]. The bikes keep the default
`DiscTemperatureToEfficiency` (no curve set), so the efficiency stays 1 for the stock KTM.

The wheel brake torque opposes the wheel spin in the spin integration (the sign is that of `w`). [I]

## Automatic shifting [B]

The decision, the generated change points, the feathering controller and the defaults are in
[ride6-gearbox-auto.md](ride6-gearbox-auto.md). Summary: for handlers with `bAutomaticGearChangePoints` (the KTM 450's `Motard_InputHandler`) the table
rows are **generated** from the torque curve and the gear ratios (the data's `ChangeUp = 0`, `ChangeDown = 0` and
`ChangeUpWithThrottle = 0.9` are not used; the tolerances `ChangeUp/DownTolerance` are used by that generation). The setup of the
display tables [B: `0x1450f2c70`] turns the fractions into rpm (`* MaxRPM`) and into road speeds with
`speed(rpm, n) = r_eff rpm (2 pi / 60) / (G_n FinalRatio)`; the live decision compares the road speed with
`speed(f MaxRPM, n)` where `n` is the current gear for upshifts and the lower gear for downshifts. The earlier note that the
decision was not recovered and that "the tolerances scale the compared speed" is superseded.

Shift timing [B: `0x1450c1660`]: `GearUpChangeTime` 0.06 s and `GearDownChangeTime` 0.15 s; during the shift the clutch input
follows `ClutchTimingOnGearChange` (0.125 to 0.2 of the shift: ramp out and back in) and the throttle follows
`ThrottleTimingOnGearChange` (0.05 to 0.9). Hold-offs: `PreventGearUpChangeLean*`, `GearWhenStopped`, `OverLimit*`
(`OverLimitRpmPercentage` 0.94 of `MaxRPM`, speed above 60 km/h, for 0.064 s), `ForceGearDownChangeBrake*`.

Launch clutch [B: `0x1450d7cd0`], when `ClutchFeatheringOnStart` is off: `clutch = max(clutch, (1 - clamp((rpm - MinPct * MaxRPM) /
((MaxPct - MinPct) * MaxRPM), 0, 1)) * (1 - clamp((v_kmh - R.x) / (R.y - R.x), 0, 1)))` with `MinPct = ClucthMinRPMPercentage` (**C++
default 0.4**), `MaxPct = ClucthMaxRPMPercentage` (0.98 for the KTM), `R = ClucthReleaseVelocityRange` (0 to 50 km/h; the second
factor is dropped when `R = (0, 0)`). With feathering on (KTM) the controller of [ride6-gearbox-auto.md](ride6-gearbox-auto.md)
section 3 runs: it holds the engine near `R = MaxRPM a + MaxTorqueRPM b` (the two feathering factors 0.7 and 0.85, summed, not
a minimum) by raising the clutch input until the wheel catches up.
