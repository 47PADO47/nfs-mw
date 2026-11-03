# RIDE 6 chassis, round 7: sub-steps, the closure without stand-ins, the stability help, the terrain terms (clean-room spec)

What round 7 found in the shipping executable (build `5.6.1-1697004`) about the points [round 6](ride6-chassis-round6.md) left open, and what
the model now does. Addresses are in [the provenance page](../provenance/ride6-physics-B.md). Tags as in round 6: **[B]** binary, **[D]**
data, **[I]** inferred, **[M]** a choice of this project's model. Handler: the road-bike one (`UMotorbikeInputHandler`, constructor
`0x1450fbfa0`).

## 1. The step is divided into sub-steps: why a held lean was lost [B]

**The loss was the step, not the tyre.** The lean loop (the prescribed roll, the handlebar filter, the front tyre's force, the help) has a
damping term `2 z sqrt(K) = 228 /s` (`z = 3.6`, `K = 1000`), so `228 dt` is 3.8 at 60 Hz. The model's bike, run at 60 Hz, weaves with a
growing yaw rate of period about 0.45 s and falls: at 30 m/s with the stick at half it is gone at 3.8 s; at 20 m/s with the stick full, in 2.4 s
with the turn-force feed-forward of round 6 and in 1.9 s without. Divided into four sub-steps of 1/240 (round 8 found that this still leaves a weave at the lean limit and divides into 1/480 s instead: [ride6-feel-round8.md](ride6-feel-round8.md), section 3) s the same bike holds the stick at
0.9 up to 20 m/s and 0.8 at 30 m/s indefinitely, **without** the feed-forward, and settles under a degree and a half off its target, as
round 6 computed for the reference's closure.

**The game does the same division** [B]. The vehicle's per-frame function runs the elements' pre-tick, then the vehicle's own step: if the
frame's `dt` is no longer than the chassis' `MaxDeltaTime` (`FChassisData`; an unset field takes a global default of -1: no division) it
is one step; otherwise `n = ceil(dt / MaxDeltaTime)`, held to `MaxSubsteps`, and the vehicle step runs `n` times with `dt / n`. The elements'
post-tick follows with the whole `dt`. The values come from the project's physics simulation settings (`PhysicsVehicleMaxDeltaTime`,
`PhysicsVehicleMaxSubsteps`, a data asset that is not among the files read; the KTM's chassis leaves both unset). **[M]** This model divides
a step into at most 8 parts of at most 1/240 s (`choices::MAX_SUBSTEP`): the longest sub-step at which the loop held the lean limit up to
20 m/s (at 1/120 s it fell at 3.5 s).

*In the model.* `Bike::respond(dt)` runs the response `n` times and integrates the body between the sub-steps; the forces of the last
sub-step stay in the body and `begin_body` integrates them for that sub-step's length only, so a host that calls `begin_body(dt)` and
`respond(dt)` still advances the bike by exactly `dt`. The fall tests run once per step (the reference's `FallCheckInterval`), before the
body moves. A fallen bike is not sub-stepped (the host's contacts move it), and the stronger closure rights a bike that a 60 Hz
step let go over: a roll rate of 4 rad/s at 25 m/s no longer tips the example bike (10 rad/s does), and the anti-lock costs less braking distance
than at 60 Hz; the host tests were adjusted to match.

## 2. The closure, without stand-ins

With the sub-steps, round 6's two stand-ins are gone:

- **The couple is the reference's moment**, `(1 - B) (P - COM) x F_applied u` about the centre of gravity (`lean_help::couple`), yaw component
  included. The compensated roll torque of round 6 is deleted. A standing bike rolled by 4.6 degrees is stood up in about a second with no
  jitter and no residue (it was a chatter and 1.5 degrees at 60 Hz), moving under 1 cm; the contact's lead over the centre of gravity turns
  the bike by under half a degree about the vertical while it rights itself.
- **`LEAN_FORCE_INCLUDES_TURN_FORCE` is deleted.** The desired front force is `-tau / h_cg` alone. A held lean sits `0.013 tan(lean)` rad off its
  target (0.75 degrees at 45 degrees) because that error is what asks the front tyre for the turn's force; at the lean limit itself
  (stick full) that offset puts the bike a degree beyond what the grip carries and it falls after a few seconds, as a bike held at its limit
  would.

## 3. The stability help [B]

The desired yaw rate is confirmed from the function. Per step, when the speed is above the lower end of `StabilityHelpSpeedRange` (constructor
10 to 15 km/h) and the distance from the ground is within `StabilityHelpMaxDistanceFromGround` (30):

```
x     = lean + (input_lean - lean) StabilityHelpInputLeanFactor           # factor 0 by default: x = lean
x     = x + terrain_slope c StabilityHelpTerrainInfluence  if slope x > 0  # influence 0 by default
if StabilityHelpMaxLeanAngle > 0:  x = clamp(lean, +-MaxLeanAngle)          # 75 deg by default; replaces the blend
w_des = tan(x) |g| / speed                                                  # g the body's gravity vector
w_act = angular velocity . n          n: world up, or the up perpendicular to the front-rear contact line (UsesTerrainNormal)
h_s   = (1 - collision reduction) S + collision addition                    # S the stability factor, round 6 section 4
a     = h_s (w_des - w_act) / max(dt, 0.001)                                # scaled by 1 - gap / MaxGroundDistance
a     = MaxAcceleration tanh(a / MaxAcceleration)      if MaxAcceleration > 0 (unset: none)
a    *= ramp(speed; StabilityHelpSpeedRange)
torque = (inertia tensor of the body) (a n)
```

The desired yaw rate is the steady-turn rate `g tan(lean) / v`, so the earlier guess was right. For the bikes other than the supermoto and the maxi-enduro the factor
`S` is the speed tuning (0.05 up to 80 km/h falling to 0 at 120; the baggers 0.03; the scooter 0.025 at 60 falling to 0 at 100) times
`max(0, 1 - |normalised lean|)` (`StabilityHelpFactorLeanInfluence` is -1 for them and 0 for the supermoto and the maxi-enduro), times
`(1 - throttle)` (the throttle-reduction translator is 0 to 1 over 0 to 0.05 km/h, so it is 1 at any speed), so the help acts when the rider
is off the throttle, upright, below about 100 km/h. The sign: a turn to the right is positive for both the lean and the yaw rate. Whether the
torque goes through the inertia tensor is **[I]** (the add-torque routine is handed the vehicle's mass and I could not read its body);
without it the term would be negligible.

*In the model.* `StabilitySpec` (manifest keys `stability_*`): speed curve, lean term, throttle and brake cuts, speed range, lean limit and
the axis choice; `lean_help::stability_factor`, `desired_yaw_rate`, `stability_acceleration`; `Bike::apply_stability_help` applies the torque
about the road's up with the body's inertia. The distance from the ground is approximated: both wheels on the road means 0, otherwise the
help is off. The collision alteration, the input-lean blend, the terrain influence and the tanh limit are not carried (unset or zero in the
data).

## 4. The terrain terms [B, one sign by consistency]

`TerrainSlope` (degrees) is the tilt of the road under the bike across its heading: the terrain normal is the normalised sum of the front and
rear contacts' normals (the rear one scaled by `TerrainNormalBlendFactor`, 1, when the front touches; reset to up if its vertical part is under
`TerrainInclinationMinNormalZ`, -1: never), the heading is projected onto the road's plane, and the slope is the angle of the normal across it,
smoothed with `TerrainSlopeSmoothingFactor` (1: not smoothed). `TerrainInclination` is the same along the heading and is used only by the
wheelie control and the pitch test (influence 0).

**Sign.** The arithmetic of the routine ends in a math stub whose argument is a register that was not read, so the sign could be traced only up
to that stub. Both consumers fix it: the fall test shifts the lean by `-f x slope` (the lean relative to the road, which for a banked turn
is the absolute lean less the bank), and the lean limit adds the bank to the limit for a turn into a rising road. Both are right only if the
slope is **positive when the road is lower on the right of the heading** (the normal tilts to the right), the same sign as a lean to the right. The
model uses that sign.

Lean limit, after the stopped blend and before the g-force term:

```
d = clamp( T_terrain(speed) x slope x (stick < 0 ? -1 : 1),  +- TerrainInfluenceMaxAngle )
L += (d <= 0 ? TerrainLeanLimitReductionScale : TerrainLeanLimitIncrementScale) x d
```

Data: speed tuning 10..50 km/h to 0..1 (10..60 for most others, 10..40 for the scooters), maximum 15 degrees, increment 0.667, reduction scale unset (constructor 1). So a
banked right-hand turn at 100 km/h allows two thirds of the bank in extra lean, and an off-camber one takes the whole bank away. Fall test:
the lean seen is `lean - f x slope` with `f` = `FallLeanLimitTerrainInfluence` (1) when the slope and the lean have the same sign, else
`FallLeanLimitTerrainReduction` (constructor 1), the shift held to `FallLeanLimitMaxDelta` (45 or 50 degrees). *In the model:*
`Bike::terrain_slope`, `TerrainLimit`, `FallExtra::terrain_*`, manifest keys `terrain_*`; tested on a laterally banked road.

## 5. The low-speed constraints and the standing bike [B]

`bForceLowSpeedConstraints` and `bInhibitLowSpeedContraint` belong to a **brake hold**, not to the bike's balance. Each step the vehicle sets a
"stopped" state when the speed relative to the road is under `StoppedSpeedThreshold` (10 cm/s) horizontally and `StoppedVerticalSpeedThreshold`,
the angular speeds are under `StoppedAngularSpeedThreshold` (0.2 rad/s) and `StoppedProjAngularSpeedThreshold` (0.05), the front wheel is down and a
brake or the handbrake is above 0.1 (and the clutch/gear state allows it). While stopped it overwrites the body's velocity and angular
velocity with the road's and zeroes the wheels' spin, so a braked bike at rest does not creep. It cannot hold a bike up: the roll at a
standstill is the lean help's moment (round 6 section 2.2), now run at the game's sub-step. Not modelled.

## 6. What stays open

- The values of `MaxDeltaTime` and `MaxSubsteps` (project settings not found): 1/480 s is a choice (round 8; it was 1/240 s).
- The inertia scaling of the stability torque; the stability help's gap scaling (approximated by "wheels down").
- The sign of `TerrainSlope` is fixed by consistency, not by the arithmetic.
- The airborne and off-track terms of the help, the collision alteration, the throttle/brake/slip reductions of the lean help (unset for every bike).
- The game's lateral g-force for the lean limit (the IMU) against this model's `v x yaw rate / g`.
