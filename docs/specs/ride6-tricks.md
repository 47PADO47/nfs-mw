# Wheelies and stoppies of a bike (clean-room spec)

How the reference game (RIDE 6, build `5.6.1-1697004`) lifts a wheel on purpose and holds it, written in our own words
from the binary and the data. It replaces the open items of [ride6-rider-help.md](ride6-rider-help.md) (which keeps the
element overview). Code: `libs/blackbox-vehicle/src/bike/{pitch_assist,trick,trick_spec}.rs`; the model around it is
[vehicle-two-wheel.md](vehicle-two-wheel.md). Provenance (function addresses, how each was read):
[provenance](../provenance/ride6-physics.md), "Round 4". Tags: **[B]** read from the binary, **[D]** data, **[I]** inferred.

**Only a bike does this.** There is no wheelie code for a four-wheel `Vehicle`: the rider's weight is an input of
`Bike::set_weight`, the game hands it to a bike's ride only (`sim/bike.rs`), and a car's `InputState` has no such field
(tests: `sim/tests/tricks.rs`). The reference's `UTrickHelpVehicleElement`, `FTrickInputTuning` and `FTrickHelpTuning` are
not part of this: they belong to the monster truck's input handler (air tricks) and no bike uses them.

## 0. The mechanism

Nothing lifts the front wheel but physics. The bike is one free rigid body with the real mass distribution (the sprung mass
with the rider, the rider's weight moved by the rider component); the chain pulls the rear suspension, the brake couples to
the fork, the engine's torque reaches the rear tyre. Under hard throttle in a low gear the rear tyre's thrust, acting below
the centre of gravity, unloads the front wheel; under hard front braking the front tyre's force does the same to the rear.
The tricks add only three things on top: **a pure pitch torque** that holds the angle the rider asks for (sections 1 to 3),
**interventions** on throttle and brake when a wheel rises further than it should (section 4) and an **auto-rider** that
shifts the rider's weight against a lift (section 5). The torque is small: at most `Mass x InertiaTensorScale.pitch x
Acceleration` (the KTM: 113 kg x 0.2 x 10 rad/s^2 = 226 N m), far less than the weight of the bike times its overhang, so
the engine or the brake must do the lifting and the help only steers the angle.

## 1. When the help acts (`UWheelieControl`, two parameter sets)

The weight input `W` (-1 back to 1 forward) is the **player's raw** longitudinal weight; the lever position is
[B: `0x145136ea0`]

```
pos = W >= 0 ? +ramp(W; Stoppie.InputDeadzone .. 1) : -ramp(-W; Wheelie.InputDeadzone .. 1)       # ramp(x; a..b) = clamp01((x-a)/(b-a)), a step at b if a == b
```

(`UseCombinedInput` would average it with the rider-position input; a bike here has one weight axis.) The help acts when
[B: `0x145122dc0`]

| | Wheelie (`pos < 0`) | Stoppie (`pos > 0`) |
|---|---|---|
| wheel that must touch the road | the **rear** (`Wheels[1]`) | the **front** (`Wheels[0]`) |
| other pedal | front brake `< 0.1` | throttle `< 0.1` |
| pedal that scales the request | **throttle** | **front brake** (`Brake`, the front lever) |
| surfaces | a set (`SurfaceType1`, `SurfaceType14`) | the same |

and `speed >= MinSpeed` (km/h) and the rider is not down. The surface sets are a data filter of the reference's materials; every
road of this game counts as allowed [choice]. The pedal is read **after** the rider help's direct interventions
(section 4). The request given to the target is `pos x pedal`, not `pos`: a wheelie at half throttle asks for half the
angle, and with the pedal released the target is level (the help then holds the nose down).

## 2. The target and the tracker [B: `0x145100f80`, `0x145137060`, `0x1450f1290`]

All angles are radians, positive nose up here (the reference uses the opposite sign and its torque follows; the two are
mirror images).

```
drive   = min(1, |pos x pedal|)
reach   = lerp(InputFactor.x, InputFactor.y, drive) * ramp(speed_kmh; MaxAngleFactorSpeedTuning) * MaxAngle     # InputFactor = (0, 1)
target  = -sign(pos) * reach                                  # wheelie up, stoppie down
current = pitch - TerrainInclination * TerrainInclinationInfluence + pitch_rate * LookaheadTime
aim     = bFilterTargetAngle ? approach(current, target, timescale * dt) : target
```

`pitch` is the **body's** pitch (the elevation of its forward axis), `TerrainInclination` the road's slope along the heading
(positive uphill); the influence is 1 for the wheelie and 0 (the constructor default) for the stoppie [D, C++]. `approach(x,
t, s)` moves `x` towards `t` by a square-root profile: `d = sqrt|x - t| - s; d > 0 ? t + sign(x - t) d^2 : t` [I: the same pattern is
read in the weight filter; the filter function itself was not read]. In the base data the wheelie does not filter its target
and the stoppie does (`timescale` 2.5), so a stoppie's target creeps in from the current angle.

**The tracker** is a time-optimal bang-bang plan for a double integrator, re-planned every step from the measured state
`(x = current, v = pitch_rate)` with acceleration limit `A = Acceleration` (10 rad/s^2) [B]. With first leg `a = +-A`, the
peak rate `u` satisfies `u^2 = a (aim - x) + v^2/2`; the total time is `T = (2u - v)/a` and the switch comes at
`t_s = (a T - v)/(2 a) + 1e-5`. Roots with `T < 0`, `t_s < 0` or `T < t_s` are void; of the (up to four) candidates the shortest
wins, a plan that starts with `-A` is returned with a negative `T`; no plan (already on the target at rest) commands 0. The command
averages the plan over the window `W = AccelerationTime` (KTM 0.1 s):

```
T > W  and t_s > W :  a_cmd = a                      # still accelerating through the window
T > W  and t_s <= W:  a_cmd = (2 t_s / W - 1) a      # the switch falls inside it
T <= W              :  a_cmd = (2 t_s / T - 1) a      # the plan ends inside it
```

## 3. The help torque [B]

```
help   = (1 - ramp(LeanPercentage; HelpIntensityLeanTuning)) * speed_curve(speed_kmh; default 1) * (1 - ramp(gap; GroundDistanceFactorRange))
         * ramp(engine_torque_share; HelpIntensityTorqueTuning) * HelpIntensityScale
torque = help * a_cmd * (CHASSIS.Mass * InertiaTensorScale.pitch)         # about the body's right axis, nose up for a positive a_cmd
```

Operands that the older spec marked open: **the speed curve is read at the speed in km/h** (`Speed x 0.036`) with 1 for an
empty curve (the stoppie's `StoppieControl` is 0 at 10 km/h, 1 at 80, 0 at 300) [B, from the argument registers]; **`gap`**
is `max(0, Compression - CompressionToContact)` of the suspension of the wheel that must stay down (it is 0 while that tyre carries load, so the
factor is `1 - ramp(0; range)`: 1 for the base range `(0, 1)`, but 0 for a range left at the constructor's `(0, 0)`);
**the torque ramp** reads the engine's torque curve at its current operand (normalised torque; the KTM's `(0, 0)` is 1) and
is not modelled. `LeanPercentage` is the lean over the lean limit. The torque is applied only while the help acts (section 1).

## 4. The rider help's throttle and brake interventions (`URiderHelpVehicleElement`) [B: `0x14510cbb0`, `0x145126b40`, `0x145110010`, `0x145105b30`]

**The lift it works from** (degrees in the reference, radians here): the body's pitch minus `AutoRiderTerrainInfluence x
inclination` (the influence is 0 by default), replaced by the angle of the line through the hubs (`IMU.WheelieAngle`: nothing
with both wheels down, the body pitch otherwise) when that is smaller in magnitude (`bCorrectAngleWithWheelDistance`), plus
`pitch_rate x AutoRiderLookaheadTime` (limited to the sign of the angle). If the wheel that should lift (front for a positive
lift, rear otherwise) still touches the road, the lift is 0. A positive lift calls the **throttle** control, anything else
the **brake** control (skipped while the bike stands). Every intervention value is multiplied by `CurrentAutoRiderIntensity`
(`AutoRiderIntensity`, 1) and clamped to 0..1.

*Asked* flags: the rider asks for a wheelie when `W < -WeightLongitudinalInputDeadzone` (0.75) **and** the raw throttle `> 0.1`;
for a stoppie when `W > +dead zone` and the raw front brake `> 0.1`.

| intervention | value | applied |
|---|---|---|
| throttle, lift angle, not asked | `ramp(lift; lerp(Launch, Wheelie range, f)) x lerp(LaunchMult, ThrottleMult, f)`, `f = clamp01(speed_kmh / AutoRiderLaunchSpeedThreshold)` (1 if the threshold is 0) | engine modulation if `bEnablePitchAngleElectronicThrottleModulation`, else the pedal |
| throttle, lift angle, asked | `ramp(lift; AutoRiderForcedWheelieThrottleControl)` (33 to 45 deg) | the pedal (`Throttle x (1 - v)`) |
| throttle, lift speed | `ramp(max(0, rate); PitchSpeedWheelieThrottleControl)` | engine modulation if `bEnablePitchSpeedElectronicThrottleModulation` |
| brake, nose-down angle, not asked | `ramp(-lift; AutoRiderStoppieBrakeControl) x AutoRiderBrakeControlMultiplier` | brake system multipliers if `bEnablePitchAngleElectronicBrakeModulation`, else the pedal |
| brake, asked | `ramp(-lift; AutoRiderForcedStoppieBrakeControl)` (30 to 90 deg by default) | the pedal |
| brake, nose-down speed | `ramp(max(0, -rate); PitchSpeedStoppieBrakeControl)` | as the angle one, per its own flag |

Engine modulation: the electronic values of the throttle are **summed**, clamped to 0..1, passed through a fading modulator
(which with a zero fade-out speed and no hold time follows its input) and multiply the engine's throttle-modulation factor by
`1 - sum`; the engine blends the throttle towards the zero-torque point, the same path as the traction control (the library
combines the two cuts as `1 - (1 - tcs)(1 - help)`). A pedal intervention multiplies the pedal in turn. Brake system multipliers
and the pedal both scale the brake torque; only the pedal value feeds the stoppie target. The g-force interventions
(`GForce*ThrottleControl`, `GForceStoppieBrakeControl`, with multipliers that default to 0 and that no bike sets) and the weight-curve
overrides `AutoRiderThrottleControlTuning`, `AutoRiderBrakeControlTuning` (no bike sets them) are not modelled.

KTM data [D]: electronic throttle (angle and speed), pedal brake; `AutoRiderLaunchSpeedThreshold` 20 km/h with `LaunchThrottleControlMultiplier` 0, so
below 20 km/h the angle cut fades out and a launch wheelie is free; wheelie cut 0 to 4 deg, forced 33 to 45 deg, speed 0.2 to 0.6
rad/s; stoppie 0 to 4 deg and 0 to 0.6 rad/s.

**ABS and a stoppie.** The ABS element (`0x1450e0240`, intensity `0x1450cac70`) has no term for the pitch or the rider help: the
intensity is `Intensity x (1 + wet share x wet scale) x speed ramp x slip ramp`, the same in a stoppie, and the KTM has both ABS
channels on (intensity 1, front normalised slip -0.3 to -0.5, rear -0.5 to -1, speed ramp 10 to 25 km/h). With the fall rate
`ABSFallRate` = 10000 /s (constructor default, no bike sets it) the release is instant, so with ABS up the front brake is held near
the force of half the peak slip and the rear lifts only if the brake force alone still unloads it (the example bike's does not:
about 5.7 m/s^2 against 8 m/s^2 without ABS, a nose-down pitch of 0.08 rad against 0.24 rad in 1.5 s from 80 km/h). The
rider turns ABS down in the reference: `SETTINGS_ABS` is a tuning-component entry (default level 1, steps of 0.25, up/down actions
`ABS_Level`) whose rows, for the KTM the table `TuningComponentData_SettingsMotard5Gear`, set: the front ABS intensity times a
curve of the level (0: 0.1, 0.25: 0.2, 0.5: 0.45, 0.75 and 1: 1); the rear intensity times a ranged multiplier from 0 to 1 over
the level [I: the level itself]; `bRearABSEnabled` by a curve that is 0 below level 0.5 and 1 from it; the start of
`AutoRiderStoppieBrakeControl` (the brake relief range) times a curve (1, 0.75, 0.5, 0, 0; the KTM's start is 0 already); and
the front brake reduction's lean output, which stays 1. Other classes have their own curves (the front one at level 0 runs
from 0.05 to 0.22). The dash shows "ABS disabled". So at level 0 the front ABS is a tenth of its intensity and the rear
is off: a stoppie is done there. The model: `Bike::set_abs_level` (snapped to the quarter steps, default from the bike's
`abs_level.default`), applied to the ABS spec at the one place the anti-lock is updated; a bike whose manifest states no curves
takes the level as the factor [choice]. The game key cycles 1 -> 0.75 -> 0.5 -> 0.25 -> 0 -> 1 (section 6).

## 5. The auto-rider's weight [B: `0x1450ff610`, `0x145110af0`]

The rider component's longitudinal position (-1 back to 1 forward, which moves his mass by the chassis' backward and forward
shifts) is the **aid** position, not the raw input. If `|W|` is beyond the dead zone the player has the weight (`aim = W`, filtered at
`ManualPositionEngageSpeed`). Within it:

```
rest  = throttle_curve(throttle) * gain + brake_curve(front_brake) * gain        # base data: (0,0)-(0.1,1) and (0,0)-(0.1,-1): forward under throttle, back under brake
A     = ramp(|lift|; lift > 0 ? AutoRiderWheelieWeightControl : AutoRiderStoppieWeightControl) * sign(lift) * multiplier        # KTM 3.6 to 4.0 deg
B     = ramp(|rate|; PitchSpeedWeightControl) * sign(rate)                       # the constructor default range (0, -1) is always 0
S     = clamp(A + B, -1, 1) * intensity
aim   = |S| <= tiny ? rest * intensity : rest + (sign(S) - rest) |S|
```

then the weight follows `aim` with the square-root approach at `AutoRiderPositionEngageSpeed` (KTM 5). Within the dead zone a lift of
4 degrees therefore sends the rider fully forward (wheelie) or back (stoppie), and with the throttle open he rests forward: to wheelie, the
player must hold the weight beyond the dead zone (`|W| > 0.75`) and open the throttle. The raw weight, not the aid, drives
section 1. Not modelled: the rider start position (`bEnableAutoRiderStartControl`, off by default), the g-force variant of the weight
control (`bEnableGForceWeightControl`, off in the data), the speed and lean curves of the rest position (none set), and the minimum
speed `AutoRiderMinSpeed` (it zeroes an output that the weight control rewrites in the same tick).

## 6. Controls

The reference's physics takes `WeightLongitudinal` (-1 back to 1 forward), optionally `RiderPosition`, `Throttle` and `Brake` in `FVehicleActions`; how a device
reaches `WeightLongitudinal` is the game's input layer (outside the physics module, not recovered). This game:

| Input | Default | |
|---|---|---|
| `rider_weight` axis, forward | G, Numpad 8, pad **left stick up** | the stoppie side, `+1` |
| `rider_weight` axis, back | V, Numpad 2, pad **left stick down** | the wheelie side, `-1` |
| `abs_level` button | J, pad left-stick click | next lower ABS level, 1 -> 0.75 -> 0.5 -> 0.25 -> 0 -> 1; logs `ABS level <n>` |

A key travels the lever at once; the stick travels it in proportion (the dead zones 0.3 for a wheelie and 0.95 for a stoppie are inside the
help). The throttle and the brake key set the strength of the trick as in section 1. A car ignores both actions. The pad's left stick
does steering (X) and weight (Y) together; the weight axis opens the stick's circular gate to a square (`y x min(1, r) / max(|x|, |y|)`),
so a stick at the rim on a diagonal gives the full lever, and the input layer's dead zone is then applied unchanged (the thresholds
0.75 and 0.95 stay reachable). The right stick stays the car's gear shifter; a bike's gearbox is automatic. The ABS level has no
HUD notice (the game has none for a setting changed by a key; the headlights are the same): it is logged.

## Choices that remain (`choices.rs`, `docs/specs/vehicle-two-wheel.md`)

The filter function of the target angle (`approach`) was read in round 6 (`0x512cb60`, [ride6-rider-round6.md](ride6-rider-round6.md)
section 3) and is as written above; the road-surface sets are not applied; the road
inclination comes from the contact normal of the grounded wheel; the torque ramp is not modelled; the g-force interventions and
the auto-rider's start control and g-force variant are not modelled; the forced downshift under braking was not looked for in the
binary (it is not on the trick path: `ride6-gearbox-auto.md` has the shift decision) and is not modelled; the mapping of devices to the weight axis is this game's own.
