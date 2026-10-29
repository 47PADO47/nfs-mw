# RIDE 6 rider, round 6: the choices turned into facts (clean-room spec)

What the rider work of the earlier rounds had to choose, and what the binary (build `5.6.1-1697004`) and the animation
blueprint say. It corrects [ride6-rider-body-aero.md](ride6-rider-body-aero.md), [ride6-rider-animation.md](ride6-rider-animation.md),
[ride6-rider-help.md](ride6-rider-help.md) and [ride6-tricks.md](ride6-tricks.md) where they disagree. Function addresses:
[provenance](../provenance/ride6-physics-C.md). Tags: **[B]** binary, **[D]** data (including the animation blueprint's own
script, read with CUE4Parse), **[I]** inferred. Code: `libs/blackbox-vehicle/src/bike/{rider,rider_place,trick,trick_spec}.rs`,
`crates/nfsmw/src/scenes/world/drive/rider/{script,params}.rs`.

## 1. The rider's own air drag [B]

Each physics step the input handler builds a drag force on the rider and adds it to the rider component's force, after the
reaction on the bike was applied. With `v` the rigid body's velocity, `|v|` its speed, `W` the longitudinal weight the rider
help leaves (`Actions.WeightLongitudinal`, -1 back to 1 forward) and `T` the tuck action (`Actions.TuckIn`):

```
c       = DragCurve(W) * DragScaleCurve(T) * DragScale                  # an unset DragCurve is 0, an unset DragScaleCurve is 1
F_rider = - c * (1 - s) * |v| * v                                       # newtons, v in m/s; s = clamp01(Slipstream * SlipstreamScale)
```

The unit constant is the vehicle drag's `-0.01` (so `c` is a coefficient like `Cd`, 100 force units per newton, speeds in cm/s).
There is **no orientation term** `g(theta)` on the rider's force. The KTM: `DragCurve` is the one-point curve `RiderAirDrag`
(0.1 at every weight), `DragScale` 1.25, `DragScaleCurve` 1.2 at `T = 0` falling linearly to 0.65 at `T = 1`: the rider adds
`c = 0.15` upright and `0.081` tucked, to the bike's own `Cd = 0.15`. The earlier rule (the posture ratio on the whole drag)
was a choice and is gone (`choices.rs`: `RIDER_DRAG_RELATIVE_TO_UPRIGHT`).

The force acts on the rider alone: it pushes him back on his spring (his relative acceleration gets `- F / m_rider`) and the
spring hands it to the frame, with the lag of the spring. This model has one body whose centre of gravity follows the
combined COM, so the force is applied to the body (through the centre of gravity: the pitch torque of the rider's height
above it, under 80 N m at 150 km/h, is not added) and to the rider's spring (`Bike::rider_drag_force`, `shift_weight`).
The rider's tuck action is the **instant** 0 or 1 of section 2, not the animation's tuck level.

Manifest: `[bike.rider] drag_weight_curve` (new: `DragCurve`, `[weight, coefficient]` points), `drag_curve` (`DragScaleCurve`
over the tuck), `drag_scale`. A manifest written before the key existed has no rider drag (the reference's empty curve is 0);
regenerate it.

## 2. Tuck-in [B]

Two things carry the name, both fed by `Actions.TuckIn`:

- **The action** (`URiderHelpVehicleElement`, tick `0x512b870`). A held tuck input above **0.5** tucks the rider. Otherwise, with
  `bAutoTuckIn`, he tucks when none of these hold: the speed is at most `AutoTuckInMinSpeed` (km/h); `AutoTuckInMaxLeanPercentage
  >= 0` and it is at most the size of the normalised lean angle (roll over the steepest lean); the player's raw weight is at
  least `WeightLongitudinalInputDeadzone` in size; the raw throttle is at most `AutoTuckInMinThrottle`; the raw front brake is at
  least `AutoTuckInMaxBrake`. The action is then exactly 1, else 0 (no ramp). The KTM (the base rider help): 90 km/h, 1, 0, 0.1,
  dead zone 0.75, so it tucks above 90 km/h with any throttle and the brake released, and the lean limit is never reached.
  A tucked rider's weight can also be placed: `TuckInPosition` in [-1, 1] sets it; outside, `TuckInWeightLongitudinalRange` is
  blended by `(W + 1) / 2` unless it is the default range, which leaves the weight alone. The KTM's `TuckInPosition` is 2 and its
  range is not set, so nothing moves (the constructor default of the range was not read: [I] that it is the default one).
- **The level** (`UMotorbikeInputHandler`, `0x5125190`): while the action is above 0.5 the level `TuckInLevel` rises by
  `TuckInRaiseRate * dt`, else falls by `TuckInFallRate * dt`, held to 0..1. Constructor defaults **2 and 2** (no asset sets them).
  The level is what the animation reads as `TuckedIn` (`IIgnitionMotorbikeRider` slot 2, `0x5b48910`).

Code: `AutoTuckSpec`, `tuck_requested`, `tuck_level` and `Bike::{tuck_action, tuck_level}` (`trick.rs`, `trick_spec.rs`);
manifest `[bike.wheelie.help.auto_tuck]` (`min_speed_mps`, `max_normalised_lean`, `min_throttle`, `max_brake`). `Bike::set_tuck`
is the player's input (held above 0.5). Wired: the bike tucks by itself in the game once its manifest has the table.

## 3. The rider body

- **Sign of a positive lean [B].** The lean angle of the bike (`0x50f4b10`) is the *roll* of the chassis as an Unreal rotator, and
  Unreal's positive roll lowers the right side. The stick adds to the target lean with its own sign (`0x50d8f10`), the steer axis
  being positive to the right. So a positive lean is a lean to the **right**; the sign is no longer a choice. The animation's
  `BikeLean` is the same roll, read as `-asin(right.z) / MaxLean` (`0x5d62540`), positive to the right. (`RiderSpec`'s lateral
  offset has `y` to the right: the KTM's -12 cm puts the rider 12 cm to the left of a bike leaning right.)
- **The lateral weight channel's rate [B].** `WeightLateral` of `InputData` is a bidirectional axis whose constructor defaults
  (`0x50a5f20`) are linearity 1, `bUseDeprecatedRate` true, deprecated rise and fall 10000 and **`InputRate` 10000**. Every asset
  sets `bUseDeprecatedRate` false and no asset sets the channel's `InputRate`, so the channel moves at 10000 per second: at once
  (`LATERAL_WEIGHT_RATE`). The longitudinal channel's rate stays the data's (6 for the KTM).
- **The rider spring's error with `bUseNewRiderDelta` [B, `0x50cc510`].** With `P` the rider's place, `T` its target and `C` the
  chassis frame's origin, `u = (P - C)/|P - C|`, `theta = asin(|u x ((T - C)/|T - C|)|)`, `e` the unit vector of `T - P` with
  its part along `u` taken off: `error = theta |P - C| e + (|T - C| - |P - C|) u` (the straight difference `T - P` when a
  point is at `C`). It replaces the linear difference (`rider::arc_error`; the two differ by less than a millimetre for a few
  centimetres of travel, the arc is now what is computed).
- **The `approach` filter [B, `0x512cb60`].** `approach(x, t, s) = t + sign(x - t) * (sqrt|x - t| - s)^2` when `sqrt|x - t| > s`,
  else `t`. It is the wheelie target filter's function (called with the current angle, the unfiltered target and `dt` times
  the filter's timescale, `0x5100f80`) and the one `trick.rs` already used for the auto-rider's position; the inference of
  [ride6-tricks.md](ride6-tricks.md) is confirmed.
- **The ramps [B].** Every `ramp(x; a, b)` of the rider help, wheelie control and rider component is the linear
  `clamp01((x - a)/(b - a))` with a step at `b` when `|b - a| <= 1e-8`; `curve::ramp` is that.

## 4. The animation script

The rider's animation blueprint computes its signals with a small script language (`UIgnitionAnimScriptLibrary`,
`...FuzzyLibrary`): `Load*` push values, the operators below act on a stack, `Store*` write variables. The operators were read:

| Operator | Meaning [B] |
|---|---|
| `SmoothFloat_Interpolate(x, r)` | `y += clamp01(r dt) (x - y)`; `y = x` when `(x - y)^2 < 1e-8` or `r <= 0`; `y` starts at 0 |
| `SmoothFloat_Constant(x, r)` | `y` moves towards `x` by at most `r dt` |
| `FlashbackFloat(x, T)` | the input of `T` seconds ago (a ring buffer; 0 until it is full) |
| `DecayFloat(x, r)` | takes a larger magnitude at once, else `y *= 1 - clamp01(r dt)` |
| `FuzzifyRamp(x, a, b)` | the ramp of section 3 |
| `DefineFloatRuleEx(w, v)`, `DefuzzifyFloatEx(rules, d)` | with `W = sum w`, `S = sum w v`: `S / W` when `W >= 1`, else `S + (1 - W) d` |
| `LogicalMin` / `Max` / `Not` / `Product` / `Sum` | `min`, `max`, `1 - a`, `a b`, `a + b - a b` |

The blueprint's constants are data [D]; the signals (`params.rs` computes them, `script.rs` is the language):

- **`Rider_Lean_Polish(250, 400, 0.3, 0.4, 6, 0.5, 4, 0.4, 2, 3, 4, BikeLeanWeight)`.** `x = 0.3 RiderLean + 0.7 BikeLean`;
  `f = ramp(speed_kmh; -10, 50)`; `polished = Defuzzify([(ramp(x; 0, 0.85), f), (ramp(x; 0, -0.85), -f)], 0)`, which is
  `f clamp(x / 0.85, -1, 1)`; `RiderLeaning = Interpolate(polished + preturn, rate) * Interpolate(M, 2.2)` with the rate 4 (the
  default; 2, 3 or 4 for `BikeLeanWeight` 1 to 3, a variable the data leave at 0), `preturn` the spline-based pre-turn lean
  (0 here), `M = terrain * Defuzzify([(ramp(speed; 250, 400), 0.3)], 1)` and `terrain` 1 on tarmac (rules on the front tyre's
  surface: 1.5 on surface 6, 0.5 when it is not tarmac, 0.7 on a kerb, averaged when several hold). The graph's `lean` is `RiderLeaning`. `RiderLean` is the controller's target
  lean over the handler's maximum lean (`0x50d6980`) and `BikeLean` the roll over the bike's `MaxLean` (here both are over
  `steering.max_lean`): the animation's lean follows the *target*, not the rider's own roll.
- **`TuckedIn_Alpha(0, 0, 0.8, 1.85, 4)`.** `RiderTuckedIn_Alpha = max(I(L, 6), I(L + h - L h, 1)) * I(terrain, 3) *
  I(1 - brake, 4) * (1 - Wheelie_Alpha)` with `L` the tuck level of section 2, `h` 1 when the throttle is open now and was
  open 0.2 s ago, and `I` the interpolating smoother. The blueprint's `SpeedThresholdBasedOnBike` (210/160/140 km/h) is set
  by the blueprint and **read by nothing**; the speed threshold of the old rule was no fact.
- **`Wheelie/ Stoppie(0.15, 0.5, 0.12, 0.2, 0.2, 2.5, 3, 3)`.** `BikeWheelie` is the nose's height `sin(pitch)` while the
  front tyre is off the ground and the rear one is on it, else 0; `BikeStoppie` is `-sin(pitch)` while the rear tyre is off and
  the front one on (`0x5b1fb60`). `Wheelie_Alpha = Interpolate(Flashback(ramp(BikeWheelie; 0.15, 0.5), 0.2), 2.5)`;
  `Wheelie_Entry_Alpha` follows `BikeWheelie` at 8; `StoppieValue = Defuzzify([(BikeStoppie > 0.18, 1), (BikeStoppie <= 0, 0)],
  StoppieValue)` (1, 0, or what it was); `Stoppie_Alpha = Constant(StoppieValue, 3)`; `Stoppie_Explicit = Constant(
  Flashback(1.27 StoppieValue, 0.15), 3)` seconds into `motard_stoppie`; `Wheelie_Direction = Defuzzify([(r, r), (l, -l)], 0)`
  with `r = ramp(BikeLean; 0.05, 0.2)` and `l = ramp(BikeLean; -0.05, -0.2)`. The wheelie and stoppie are no longer the nose
  pitch over the statics' balance angle: that was a choice and is replaced.
- **`InertiaDisplacement_Evaluation`.** The pelvis displacement is held to a range that depends on the speed and the lean
  (the data), smoothed at 4, and delayed by 0.2 s and smoothed at 1.65 for the head, which turns by `(Y * 0, X * -2, Z * 5)`
  (yaw, roll, pitch multipliers) of that delayed displacement; the unit of the multipliers (degrees per centimetre?) and the
  axis meaning of the native `InertiaDisplacement` were not recovered, so the head is not driven.
- The rate of the shift (`inertia.rate`, 4) is an `Interpolate` too: `clamp01(rate dt)` per step, not an exponential.

The pose graph's parameters are now `lean`, `speed_kmh`, `tuck`, `wheelie` (the entry ramp 0.2 to 0.4 on `BikeWheelie` held at 8),
`stoppie` (`Stoppie_Alpha`, the weight), `wheelie_alpha` (the minimum-to-maximum pose weight, ramp 0.15 to 0.5 in the script),
`stoppie_time` and `wheelie_direction` (left and right by `(d + 1) / 2`). Which node reads which variable is in cooked binary
data; the binding of the entry ramp and of the aero set to `RiderTuckedIn_Alpha` is the reading of the names. A graph made before
these parameters reads the new ones as 0: regenerate it with the importer.

## 5. What stays open

- The lateral weight *controls*: no data asset sets `bAutomaticLateralWeightControl` (its constructor default was not written, so
  it is off), so no bike uses the rider help's lateral logic; the device-to-axis mapping (`EvaluateRiderWeightLateral`) is
  Blueprint data of the input component, not native code. This game has no key for the lateral weight axis.
- `BikeLeanWeight` of a rider asset, the bike component's `MaxLean`, the constructor default of `TuckInWeightLongitudinalRange`,
  the units of the head multipliers, the graph's pin bindings.
