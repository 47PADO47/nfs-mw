# RIDE 6 chassis, round 6: how the roll is closed, the fall tests, the day, the end of the travel (clean-room spec)

What the shipping executable (build `5.6.1-1697004`) does with the pieces that earlier rounds left as modelling choices:
the **closure of the prescribed roll** (what actually rolls the bike), the exact **lean-help factor**, its **split into a force and
a couple**, the **pairing of the help reductions with their inputs** (lean help and stability help), the **fall tests**, the
**temperature of the day** that the tyre thermal model cools towards, the **slip-correction multiplier**, the **end of the
suspension travel** and the **sign of the brake coupling force**. Addresses are in
[the provenance page](../provenance/ride6-physics-B.md); this page keeps behaviour, formulas and constants. Earlier pages:
[lean controller](ride6-lean-controller.md), [suspension](ride6-suspension.md), [tyre thermal](ride6-tyre-thermal.md),
[parameters](vehicle-two-wheel-parameters.md). Tags: **[B]** read from the binary, **[D]** from the data assets, **[I]** inferred,
**[M]** a choice of this project's model.

Two classes of input handler exist in the game: the **road bike** one (`UMotorbikeInputHandler`, every road bike and the KTM
450 SMR) and the **motocross** one (`UMXBikeInputHandler`). They share the base class and have the same algorithms with
different field offsets. Everything below is the road-bike handler; the motocross one differs only in what is stated.

Units as in the other specs: speeds in the formulas are km/h where the data's tuning ranges are (the code multiplies the speed in
cm/s by 0.036), angles degrees where the data are, forces newtons.

## 1. The roll closure: what rolls the bike

The handler computes the roll acceleration the rider prescribes, `a = -K (theta - target) - 2 z sqrt(K) theta'` (soft-limited by
`LeanAccelerationMaxValue`), turns it into a torque `tau = I_roll a` and then into a **desired front side force**
`F_des = -tau / h_cg` ([lean controller](ride6-lean-controller.md), step 2). A search of the whole physics module for readers of
the stored acceleration (`CurrentLeanAcceleration`) and torque (`DesiredLeanTorque`) shows that **nothing else uses them** [B]:
the torque is never applied to the body. The roll of the bike is the result of

1. the **front tyre force**, which the handlebar angle sets (the angle is chosen so that the front tyre's lateral force curve gives
   `F_des`),
2. the **lean help** (section 2): an extra side force at the front contact, or a couple, that closes part of the gap between the
   tyre's force and `F_des`,
3. the **stability help** (section 4): a roll torque, zero for the stock supermoto (and nearly all bikes on tarmac),
4. the gyroscopic terms, the rider's reaction torque and the lean the rider's weight causes,
5. the rear tyre's lateral force, which nothing controls.

There is no "rider torque" with a strength of its own. So `ROLL_TORQUE_AUTHORITY` of this project's first model (a direct roll
torque of up to two weights times the height of the centre of gravity) has **no counterpart** in the game; what bounds the help
instead is the tyre: `LeanHelpLimit x (peak lateral force of the front tyre at its load)` (section 2).

**Held lean.** At a constant lean `a = 0`, so `F_des = 0` and the front tyre is asked for no force; the force that a steady turn
needs from it is not asked for but obtained from the error: the lean settles `Delta theta` off the target, where
`K I Delta theta / h_cg` equals that force. For the KTM (front weight share 0.5, `I = 47.8 kg m^2`, `K = 1000`,
`h_cg = 0.7 m`) it is `0.013 tan(theta)` rad, 0.75 degrees at 45 degrees of lean.

**This project's model** (round 7, [ride6-chassis-round7.md](ride6-chassis-round7.md)): the closure is the game's. The round-6 stand-ins (a
compensated roll torque for the couple, the feed-forward of the turn's force) were the 60 Hz step's doing and are gone: the reference
divides a vehicle step into sub-steps, and so does the model.

## 2. The lean help

Per step, on the front wheel in contact [B]:

```
u        = unit side direction of the front wheel;          F_act = F_contact . u
h        = clamp01( (1 - CurrentHelpReduction) h_factor (1 - R) + A )          # h_factor: section 2.1
F_ideal  = F_act + h (F_des - F_act)
cap      = LeanHelpLimit * Fy_peak(Fz, camber) * sqrt( 1 - ( k_t |F_contact - F_act u| / Fx_peak(Fz, camber) )^2 )    # 0 when the ratio reaches 1
F_total  = ( |F_act| < |F_ideal| ) ? clamp(F_ideal, -cap, cap) : F_ideal
F_applied = (F_total - F_act) u
```

`R` and `A` are the **collision alteration** of the help (a reduction and an addition that follow a collision and fade away,
`CollisionLeanHelpAlteration`: `MaxHelpAddition` 2, `HelpAlterationFadeRate` 5 for the road bikes [D]); they are 0 without a
collision. The earlier statement that they come from the speed, throttle and slip tunings was wrong: those form `h_factor`.
`CurrentHelpReduction` is the burnout reduction, 0 otherwise. `k_t` (`LeanHelpLimitFrontTractionFactor`, 1 on tarmac, 0 on the
three off-track surfaces) is looked up by the surface under the front wheel; `Fx_peak`, `Fy_peak` are the peaks of the tyre object
times the grip scales of the wheel.

### 2.1 The help factor `h_factor` [B]

With `s` the speed in km/h (absolute value when a translator's `UseAbsoluteValue` is set), `NL` the normalised lean, `Thr`,
`Brk` the throttle and brake levers, `P = Thr - Brk`, and `T_x(s)` the translators `LeanHelpFactor{Speed,Throttle,Brake,FrontSlip,
RearSlip,Skid}...` of the handler **evaluated at the speed**:

```
S   = T_speed(s) * max(0, |NL|^LeanHelpFactorLeanSensitivity * LeanHelpFactorLeanInfluence + 1)
Lr  = max(0, |NL|^ReductionLeanSensitivity * ReductionLeanInfluence + 1)
r_thr  = clamp01( T_thr(s)  * RS * Thr                                   * Lr )
r_brk  = clamp01( T_brk(s)  * RS * Brk                                   * Lr )
r_fs   = clamp01( T_fs(s)   * RS * infl_f(P) * clamp01(front CombinedSlip) * Lr )
r_rs   = clamp01( T_rs(s)   * RS * infl_r(P) * clamp01(rear  CombinedSlip) * Lr )
S  *= (1 - r_thr)(1 - r_brk)(1 - r_fs)(1 - r_rs)                                 # RS = LeanHelpFactorReductionScale
skid = clamp01( (|VelocityDirectionAngle| [deg] - SkidRange.x) / (SkidRange.y - SkidRange.x) ) ^ SkidLinearity
S  *= clamp01( 1 - T_skid(s) * RS * infl_s(P) * skid ) * LeanHelpFactorMultiplier
if burnout:  S = 1
elif StoppedSpeedRange.y > 0:  S = 1 + ramp(s; StoppedSpeedRange) (S - 1)        # full help at a standstill
g = max(0, gap of the front suspension)                                        # airborne
if g > AirborneRange.x:  S = lerp(S, LeanHelpFactorAirborne, ramp(g; AirborneRange))
h_factor = clamp01( ((AntiFallFactor_n - 1) AntiFall + 1) (OutOfTrackAddition(surface) + S) )
```

`infl_f`, `infl_r`, `infl_s` are the runtime curves `...FrontSlipInputInfluence`, `...RearSlipInputInfluence`,
`...SkidInputInfluence` of the lever difference `P`; an unset curve is the factor 1. Defaults of the constructor: `Multiplier 1`,
`LeanInfluence 0`, `LeanSensitivity 1`, `ReductionScale 1`, `ReductionLeanInfluence 0`, `ReductionLeanSensitivity 1`,
`LeanHelpFactorAirborne 1`, every range `(0, 0)`; an unset translator (both ranges `(0, 0)`) gives 0. `OutOfTrackAddition` is 1 on
the three off-track surfaces (`SurfaceType2`, `SurfaceType3`, `SurfaceType10`) and 0 elsewhere, so the help is full there.

**Pairing** (open until now): throttle reduction with the throttle lever, brake with the brake lever, front slip with the front
wheel's `CombinedSlip` shaped by `P`, rear slip with the rear wheel's, skid with the speed tuning, the skid angle and `P`. The
handbrake lever has no reduction of its own. All reductions multiply, `S x prod (1 - r)`.

**Data [D].** No bike of the install sets the throttle, brake, front-slip or rear-slip translators: they are 0 and only the skid
cut acts. The 8 handler assets differ only in a few numbers:

| | supermoto, maxi-enduro (the KTM) | default, naked, baggers, 250 two-stroke, scooter | geared scooter |
|---|---|---|---|
| speed tuning, km/h to factor | 0..10 to 0..0.2 | 0..10 to 0..0.16 | 0..10 to 0..0.22 |
| stopped range | 0..10 km/h | same | same |
| skid angle range, deg | 10..45 | 5..25 (naked 5..30) | 10..30 |
| skid linearity | 2 (maxi-enduro 1) | 3 | 2 |
| skid translator | 0..30 km/h to 0..1 | 0..15 km/h to 0..1 | 0..15 km/h to 0..1 |
| skid input curve | `P` -0.1: 0, `P` 0: 1 (flat beyond) | same | same |
| balance, km/h to share | 10..30 to 0..1 | 10..40 to 0..1 | 10..40 to 0..1 |
| airborne range (gap) | 20..35 | 20..35 | 0..20 |

So on tarmac, upright, the help factor is **1 at a standstill, falling to 0.2 by 10 km/h** (the speed tuning, reached by the stopped
blend), and only a slide (a heading error beyond 10 degrees at the KTM's, with the throttle open or off) cuts it; braking
(`P < -0.1`) removes the cut.

### 2.2 Where the force acts: the balance [B]

The force is split in two by the **balance** `B`:

```
B0 = 0 when the front wheel is not on the ground
else B0 = clamp01( T_balance(s) * LeanHelpBalanceMultiplier )    # bOverrideLeanHelpBalance is set by every bike; else B0 = LeanHelpBalance (constructor 0)
     if stopped threshold > 0 and s < threshold: B0 *= s / threshold   (the threshold is 0 for the bikes)
     if burnout: B0 = 0
B  = B0 * (speed < 3 m/s ? ramp(speed [m/s]; 1..3) : 1)             # in the force function, once more, every step
force at the front contact P_c:     B F_applied u
couple:                             (1 - B) * (P - COM) x (F_applied u)
```

(The motocross handler also takes the balance to 0 over its airborne range; the road handler does not.)

The point `P` is the front contact point for the stock bikes: the "neutral" blend of the point towards the foot of the centre of gravity
(`LeanHelpNeutral...`) is 0 because its factor and ranges are unset. The total moment about the centre of gravity is
`(P_c - COM) x F_applied u` whatever `B` is; only the **net force** is `B F_applied`. For the KTM the balance is 0 up to 10 km/h
and 1 from 30 km/h (40 for the others): below 10 km/h the help is a pure moment, which at a standstill (help factor 1) is
`F_total x h_cg` about the roll axis (and `F_total x a` about the vertical one, `a` the contact's distance ahead of the centre of
gravity), as far as the tyre's cap `LeanHelpLimit x Fy_peak` allows. **That is what holds a bike up at a standstill** [B]. The
1 to 3 m/s fade only matters for a tuning that has force at a crawl.

`LeanHelpLimit` is `T_limit(s) x LeanHelpLimitMultiplier (1) + OutOfTrackAddition (1 on the three surfaces)`, then the anti-fall
factor; the KTM's speed tuning is 1 to 1 (0.99 at the top for the others).

### 2.3 In this project's model

`lean_help.rs`: `help_factor` is the speed curve (`help_speed`), cut by the skid term (`help_skid_*`, with the lever difference),
blended to 1 at a standstill (`help_stopped_range_mps`); `balance` is the table `help_balance_speed` times the 1..3 m/s fade; the
force part acts at the front contact, the rest as the compensated roll torque of section 1. The throttle, brake and slip
reductions, the airborne blend, the off-track surfaces and the collision alteration are not modelled (no bike uses the first four; the
rest need the road surface and collisions).

## 3. The fall tests [B]

Every `FallCheckInterval` (1/60 s). `air` is true when the vehicle's distance from the ground is beyond `FallMaxDistanceFromGround`
(constructor 0: no wheel on the road). The first test that fires ends the run; the code says which kind of fall it was.

| Test | Limit | Gate | Kind |
|---|---|---|---|
| lean (ground) | `FallLeanLimit`, see below; the lean seen by the test is `lean - f x slope` (`slope` the terrain slope in degrees, `f` = `FallLeanLimitTerrainInfluence` if it has the lean's sign, else `...TerrainReduction`; the shift is clamped to `FallLeanLimitMaxDelta`; the data sets influence 1, reduction defaults to 1: lean relative to the ground) | beyond the limit for `FallLeanLimitDuration` (0.01 s, so two consecutive checks) | 2 |
| lean (air) | `FallLeanLimitAirborne` (70 deg; 55 for the scooter) | at once | 1 |
| upside down (air) | `FallUpsideDownLimit` (0: off) on the cosine of the tilt | at once | 1 |
| pitch | `FallPitchLimit` ground, `FallPitchLimitAirborne` air (65 deg both); the pitch is the body's wheelie angle, shifted by `-f x inclination` with `f` the terrain influence (default 0) | at once | 1 |
| velocity against heading | `FallSpeedDirAngleLimit` ground (85 deg supermoto, 63.5 the others), `...Airborne` (180: never) on `VelocityDirectionAngle` | speed above `FallSpeedDirAngleMinSpeed` (10 km/h supermoto, 0 the rest) and not reversing | 3 if the lean is under `FallHighsideLeanLimit` (70 or 50), else 2 |
| angular speed | `FallAngularSpeedLimit` ground (4.4 supermoto, 4.3 to 4.4), `...Airborne` (7); `...Landing` just after a landing (while the landing timer runs, `FallAngularSpeedLandingTimer`) | **none** | 2 |
| rider force | `FallForceLimit` (2e6) against the force at the rider; with the speed above `FallMinSpeed` (10 km/h), the horizontal and vertical limits too | `CheckRiderForce` | 1 |
| collisions | impulse and impact limits, `FallCollisionDuration` | | 1 |

So the **lean and pitch tests run at any speed, and so does the angular-speed test**; only the velocity-direction test has a speed
gate (its own, `FallSpeedDirAngleMinSpeed`), and `FallMinSpeed` gates only the rider-force tests. The earlier reading (angular speed and
heading gated by `FallMinSpeed`) was wrong.

**The ground lean limit is a running value.** With `bOverrideFallLeanLimit` and `bFallLeanLimitBasedOnGrip` (every bike), each step

```
target     = atan(LateralGripPeak) * FrontWheel.LateralGripTolerance      [deg]
FallLeanLimit += (target - FallLeanLimit) * (target >= FallLeanLimit ? FallLeanLimitUpFactor : FallLeanLimitDownFactor)
```

with up 0.025 and down 0.01 for every bike, starting from 65 degrees (the constructor default). It is the same base as the lean
limit's (`atan` of the front tyre's lateral friction peak coefficient). Otherwise it is `FallLeanLimitSpeedTuning(speed)`. The block-fall
cap of the target lean is `BlockFallLeanLimitRatio x BlockFallLeanLimitDeg x 0.99` with `BlockFallLeanLimitDeg = min(BlockFallLeanLimitMaxDeg
(90), FallLeanLimit - (|lean_seen| - |lean|))` on the ground and 720 (no cap) in the air.

**Terrain.** `TerrainSlope` is the signed lateral inclination of the ground under the bike relative to its heading, in degrees, filtered with
`TerrainSlopeSmoothingFactor`; `TerrainInclination` is the longitudinal one, in radians, filtered with
`TerrainInclinationSmoothingFactor`. The terrain normal is the normalised sum of the two wheels' contact normals (the rear's scaled by
`TerrainNormalBlendFactor` when the front touches) and is reset to up when its vertical part is below `TerrainInclinationMinNormalZ`.
The lean limit's terrain term (`ride6-lean-controller.md`, "The effective lean limit", point 5) uses the slope: `d = clamp(T_terrain(s) x slope
x (stick >= 0 ? 1 : -1), +-TerrainInfluenceMaxAngle)`, then `+TerrainLeanLimitIncrementScale d` for `d > 0` (0.667) or `+TerrainLeanLimitReductionScale d`
for `d <= 0`. On flat ground all of these are 0. The sign of the slope and the model's use of it are in round 7, section 4.

### In this project's model

`fall.rs`: lean and pitch at any speed (the lean for `lean_duration_s` on the ground, at once in the air), the angular speed at any
speed, the velocity direction above `min_speed_mps`; the ground lean limit follows `atan(mu) x tolerance` with `lean_rise` and `lean_fall`
(`lean_by_grip`); the airborne limits are `airborne_*`. Not modelled: the upside-down test, the landing limit, the rider-force and
collision tests, the terrain terms. The kind of fall (low side, high side, generic) is modelled and decides how the rider is thrown
([ride6-rider-fall.md](ride6-rider-fall.md)).

## 4. The stability help [B, one operand I]

Same factor structure as section 2.1 with the `StabilityHelpFactor...` fields, with these differences: the result is 0 while the
distance from the ground exceeds `StabilityHelpMaxDistanceFromGround` (30), 0 in a burnout, multiplied by `ramp(s; StoppedSpeedRange)`
when that range is set, and the off-track addition is 0.025 [D] (not 1). The speed tuning is 0..0 for the supermoto, so the
help is **off** on tarmac for it; the others a speed tuning of 80..120 km/h to 0.05..0 (0.03 for the baggers, scooter 60..100 km/h to 0.025..0): a small help, 0.05 up to 80 km/h and fading to 0 by 120. The reductions
are paired as in 2.1.

When `h_s > 0` and the speed is above `StabilityHelpSpeedRange.x`: `omega_act` is the bike's angular velocity about an axis (world up, or the
up direction perpendicular to the line between the two contacts when `StabilityHelpUsesTerrainNormal`), `omega_des` the yaw rate a steady
turn at the current lean needs (**[I]**: `g tan(lean) / v`), `a = h_s (omega_des - omega_act) / dt`, scaled by `1 - gap / MaxGroundDistance`,
soft-limited to `StabilityHelpMaxAcceleration` with a tanh, times `ramp(s; SpeedRange)`, and applied as the torque of that angular
acceleration through the world inertia tensor. Round 7 confirms the desired yaw rate and carries the help ([ride6-chassis-round7.md](ride6-chassis-round7.md) section 3).

## 5. The day: air and road temperature

The two temperatures the tyre cools towards are fields of the vehicle (`AirTemperature`, `AsphaltTemperature`); no constructor sets
them. They are written **by the game**, from the weather system: at race set-up and when the weather state is broadcast, the vehicle gets
the air and the track temperature of the current weather state [B]. The weather state takes them from a data table [D] (the time-of-day interpolation between the five keyed values and the random range were not read), one row per
track and weather (51 tracks, 7 weather names, 298 rows) with five times of day and a random range:

| | clear, morning (median of the 51 tracks, range) | clear, afternoon | cloudy, morning | rain, morning |
|---|---|---|---|---|
| air, deg C | 23 (14..29) | 27 (15..33) | 20 (11..24) | 18 (10..23) |
| road, deg C | 28 (18..44) | 32 (22..52) | 26 (18..42) | 16 (16..32) |

(`DawnAir/Track`, `Morning`, `Afternoon`, `Sunset`, `Night` temperatures; `TemperatureRandomRange` is 2 for every row.) So the temperatures are
**inputs of the world**, not constants of a bike. This project's `Bike::set_environment` takes them; until the host calls it
`Environment::default()` is 23 / 28 (clear morning, median) as a stand-in.

## 6. The slip-correction multiplier

The wheel's runtime `SlipCorrectionMultiplier` is written in one place: the handler's per-step wheel set-up. It is **1** except in a
burnout, where it is the handler's `BurnoutSlipCorrectionMultiplier` (0.9 for the supermoto, 1 default); the same set-up gives the rear wheel's two grip scales the
handler's `BurnoutGripMultiplier` (0.15 for the supermoto) and 1 otherwise [B, the setters' names I]. Wet roads act through the tyre's grip tables and `WetPercentage`, not here. The model has no burnout, so 1.

## 7. The end of the suspension travel

The compression is clamped to `[0, MaxCompression]` after every step. Past the stop the speed is set to 0 (inelastic); the amount
beyond is stored (`bump overshoot`) and read by nothing that makes a force [B]. With a tyre of radial stiffness the ground
penetration is split between tyre and suspension as in [ride6-suspension.md](ride6-suspension.md) "Ground contact"; the tyre's share is
capped at `MaxTireDeformation`. **No stiff stop spring exists**, so `BUMP_STOP_FACTOR` is gone: this project's model keeps the tyre's radial
spring for the overload and adds nothing for a rigid tyre.

## 8. The brake coupling sign

The coupling force is `(T_applied - T_back) x 0.01 / R x factor x f(s)`. The wheel update writes `T_applied` as the net torque that changed
the wheel's spin and `T_back` as that less the braking and drag torque, so for a wheel turning forward `T_applied - T_back` is **minus** the
brake torque [B]; the force is negative, takes load off the spring, and the fork dives: the sign is -1, as the model has it. (A wheel
turning backward flips it.)

## 9. What stays open

Round 7 closed the turn-force feed-forward, the terrain terms, the desired yaw rate and the standing-bike chatter; what is left is listed
in [ride6-chassis-round7.md](ride6-chassis-round7.md) section 6.
