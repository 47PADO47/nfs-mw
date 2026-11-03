# RIDE 6 lean and handlebar controller (clean-room spec)

The rider of a RIDE 6 bike is not an inverted pendulum balanced by a PD controller. The input handler
(`UBikeInputHandler`, instance `UHandlebarInputHandler`) works the other way round: it **prescribes the roll motion** of the
bike with a second-order spring-damper towards a target lean angle, converts the roll torque this needs into a **desired
side force at the front tyre**, and chooses the **handlebar angle** that makes the front tyre produce that force. This page
states the algorithm in our own words, with the constants the binary and the data give. Data fields:
[ride6-physics-elements.md](../formats/ride6-physics-elements.md). Mapping to our model:
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). How it was found: [provenance](../provenance/ride6-physics.md).

Status tags: **[B]** read from the binary, **[I]** inferred from the structure of the code and the field names (not read
line by line), **[D]** data only.

## Notation

`dt` the physics step. `Steer` the stick, -1 to 1. `speed` in km/h (the code multiplies cm/s by 0.036). All angles are
radians inside, degrees in the data. `theta` is the lean angle of the chassis (`CurrentLeanAngle`), `theta'` its speed
(`CurrentLeanSpeed`). `h_cg` the height of the combined centre of gravity (`COMHeight`).

## The second-order filter used everywhere

`K`, `Damp`: stiffness (a squared natural frequency) and damping. If `Damp <= 0` the damping coefficient is
`c = 2 sqrt(K) z` with the damping ratio `z = |Damp|` (`z = 1` when `Damp = 0`, critical); if `Damp > 0` it is the absolute
coefficient `c`. The state `(x, v)` is the offset from the input; one step is the **exact analytic solution** of
`x'' = -K x - c v` over `dt` for the under-, critically and over-damped case, then `x_out = input + x`. [B: `0x1450f9020`
sets the coefficients, `0x1450f9120` steps, `0x1450f8da0` evaluates the acceleration]. Rust: implement the closed form so a
large `K dt^2` cannot go unstable. `LeanDamp -3.6` is therefore `z = 3.6`, an over-damped response.

## Step 1: the stick to the target lean

1. `lean_limit` (degrees) is the **effective** lean limit for this tick (see "The effective lean limit and its combination order" below).
2. `input_raw = lean_limit * (pi/180) * Steer`. `Steer_f` is `Steer` through the filter `(SteerK, SteerDamp)` (KTM 1000,
   z = 2.5). `InputLeanAngle = lean_limit * (pi/180) * Steer_f`. [B: `0x1450d8f10`]
3. Time scale: `ts = (1 - min(|theta| * LeanTimeScaleAttenuation, 0.99)) * LeanTimeScale *
   (1 + |InputLeanAngle - theta| * LeanTimeScaleSensitivity)`. (`LeanTimeScale` is also scaled by the speed and input
   tunings of the handlebar class: `LeanTimeScaleSpeedTuning` maps 10..200 km/h to 1..0.65, `LeanTimeScaleInputTuning` is
   a curve of the stick: 0.425 at -1, 1 at 0, 0.7 at 1.) [B] for the formula, [D] for the tuning tables.
4. Target lean `TargetLeanAngle`:
   - with `bUseKDampFilterForTargetLeanAngle` (the KTM): a second-order filter (`K = 1000`, `z = 2.25`) stepped by
     `dt * ts`, input `InputLeanAngle`. [B]
   - otherwise: `d = sqrt(|T - input|) - dt * ts`; `T = input + sign * d^2` while `d > 0`, else `T = input`. [B]
5. "Block fall" clamp: when `FallCheckEnabled`, `BlockFallActive`, `BlockFallLeanLimitRatio > 0` and the speed is above
   `BlockFallLeanLimitMinSpeedKmh`, clamp `|T|` to `BlockFallLeanLimitRatio * BlockFallLeanLimitDeg * (pi/180) * 0.99`
   (`BlockFallLeanLimitDeg` is the current fall lean limit, runtime). [B]

## Step 2: roll motion, torque and desired front force [B: `0x1450c8f70`]

```
target' = TargetLeanAngle - RiderLeanAngle * RiderLeanAngleInfluence
z       = |LeanDamp| * LeanComponent.LeanDampMultiplier           # multiplier 1 to 1.5 by weight, tensor, height, wheelbase
a       = -LeanK * (theta - target') - 2 z sqrt(LeanK) * theta'   # CurrentLeanAcceleration
if LeanAccelerationMaxValue != 0:  a = M * tanh-like(a / M)       # soft clamp, M = LeanAccelerationMaxValue
I_roll  = m_rider * RiderMassScale * COMOffset.z^2 + I_x         # I_x = InertiaTensorScale.x * total mass
tau     = I_roll * a                                              # DesiredLeanTorque
F_front = -tau / h_cg                                             # DesiredFrontSideForce
```

KTM: `LeanK = 1000` (natural frequency about 5 Hz), `z = 3.6`, rider mass 70 kg at `COMOffset.z = 40 cm`, `I_x = 0.2 x
(113 + 70)`. In SI units the rider term is 70 x 0.4^2 = 11.2 kg m^2 and the chassis term is 36.6 kg m^2.

## The range translator used everywhere

`FClampedRangeTranslator(x)` [B: `0x1450cb320`, `0x1450cb5c0`]: `t = clamp01((x - In.x) / (In.y - In.x))` (a step at `In.y` if the
range is empty), with `x` first replaced by `|x|` when `UseAbsoluteValue`; if its `Curve` is set the result is `Curve(x)`,
otherwise `Out.x + (Out.y - Out.x) * t^Linearity` (`Linearity` default 1).

## Step 3: from the roll torque to the handlebar [B: `0x1450c9320`, `0x1450c90d0`, `0x1450c8f70`]

1. **Desired front side force.** `F_des = tau / max(arm, 0.05)`, where `tau` is the roll torque of step 2 and `arm` the
   perpendicular distance of the front contact's side direction from the roll axis, formed from the chassis attitude
   (the `COMHeight` used in step 2 is a first estimate that this call overwrites; for a small lean `arm` is the height of
   the centre of gravity). `DesiredFrontSideForce = F_des` (signed).
2. **Desired slip angle** `alpha_des` (rad) from the **inverse of the front tyre's lateral force curve** at the front wheel's
   current load `Fz` and camber:
   - default (`bUseLinearizedInverseFunction` false, the KTM): `alpha_des = invFy(F_des / LateralGripScale, Fz, camber)`, the
     model's inverse [B: `0x1451547c0`, model virtual `0xB0`, `0x1451546e0`, solver `0x14515c560`]. With the Magic Formula
     coefficients of the tyre at `(Fz [kN], camber [deg])` (tyre spec) and `y = F - Sv`: if `|y| < 1.2e-7` the result is 0; if
     `|y| < 0.99 D` solve `D sin(C atan(B x - E (B x - atan(B x)))) = y` for `x = alpha + Sh` (degrees) with a bracketed root
     finder started at `x0 = |y| / (B C D)` and relative tolerance `NonLinearSolverRelativeTolerance` (1e-4); otherwise
     return the slip angle of the force peak. The sign follows `y`. Convert degrees to radians.
   - with the flag set, a secant between two points of the curve: `L = clamp(LinearizationFactor, 1e-4, 1)`,
     `a = (s+ - s-)(1 - L)/2 + s-`, `b = ((L + 1) s+ + (1 - L) s-)/2`,
     `alpha = a + (F - Fy(a)) (b - a) / (Fy(b) - Fy(a))` clamped to `[s-, s+]` (`s-`, `s+` the lateral slips of the force peaks). [B]
   `DesiredSlipAngle = alpha_des`.
3. **Handlebar direction.** Take the front contact point's velocity, remove its component along the front wheel's up axis
   (`FrontVelocityUp`, the steering-plane normal) and normalise: `v_hat`. `side = up x v_hat` (`FrontVelocitySide`). The wanted
   wheel heading is `h = v_hat + alpha_des * side` (`sin(alpha)` is treated as `alpha`, as the wheel's own slip is). Rotate `h`
   and the steering axis `a` into the chassis frame (x forward, y right, z up); the steer angle is the heading of `h`
   **projected onto the ground plane along the steering axis**: `delta = atan2(h'_y, h'_x)` with `h' = h - (h_z / a_z) a`
   [B: `0x145158890`]. The sign is that of this `atan2` in the chassis frame; a force asking for a right lean therefore
   produces the counter-steer without any extra rule.
4. Below `LowSpeedSteerBlendRange` (KTM 2 to 4 m/s) `delta` blends linearly to `Steer * FrontWheel.SteerAngle * (pi/180)`;
   `UseForcedSteerAngle` overrides it. [B]
5. The result passes the handlebar filter (`HandlebarK = 1000`, `HandlebarDamp = -3`: ratio 3, `HandlebarTimeScale = 1`, stepped
   by `dt * HandlebarTimeScale`), except during a stoppie braking with `DisableHandlebarFilterInStoppie`. `CurrentSteerAngle`
   is the output. [B]
6. `CurrentHandlebarSteer = clamp(deg(CurrentSteerAngle) / FrontWheel.SteerAngle, -1, 1)` replaces the stick in
   `Actions.Steer` for the rest of the step. [B]

There is no separate stabiliser: the lean is prescribed (step 2) and the handlebar makes the front tyre supply the force.

## The effective lean limit and its combination order [B: `0x14510b180`]

`LeanLimit` (degrees) is rebuilt each tick, in this order (cases 1 exit early):

1. `bOverrideLeanLimit`, airborne and `LeanLimitAirborne > 0` (KTM 0): `L = LeanLimitAirborne`. Burnout and
   `BurnoutLeanLimit > 0` (KTM 10): `L = BurnoutLeanLimit`.
2. Base: grip based (KTM) `L = atan(LateralGripPeak) * FrontWheel.LateralGripTolerance` in degrees (`LateralGripPeak`: the front
   tyre's lateral friction coefficient; tolerance default 1); otherwise the speed table `LeanLimitSpeedTuning` (wet variant).
3. `L += LeanLimitExtraAngle`; below `StoppedSpeedThreshold` km/h `L = lerp(StoppedLeanLimit, L, speed / threshold)` (KTM 30 km/h, 1 deg).
4. Inputs: `L += L * (brake^BrakeLeanDeltaAngleLinearity * BrakeLeanDeltaAnglePercentage + throttle^ThrottleLeanDeltaAngleLinearity * ThrottleLeanDeltaAnglePercentage)` (defaults 0).
5. Terrain: `d = clamp(TerrainInfluenceSpeedTuning(speed) * TerrainInclination * sign, -TerrainInfluenceMaxAngle, TerrainInfluenceMaxAngle)`;
   `L += TerrainLeanLimitReductionScale * d` if `d <= 0`, else `TerrainLeanLimitIncrementScale * d` (KTM max 15 deg, increment 0.667).
6. g-force: `L *= LeanLimitGForceTuning(|g|)`, `g` the lateral g-force of the IMU (KTM 1.0 at 0 g to 0.9 at 1.1 g).
7. Skid: `skid = clamp01((|VelocityDirectionAngle| [deg] - SkidRange.x) / (SkidRange.y - SkidRange.x))^LeanLimitSkidLinearity`;
   `L *= clamp(1 - LeanLimitSkidReduction(speed) * LeanLimitSkidReductionInputTuning(input) * LeanLimitSkidScale * skid, 0, 1)`
   (KTM range 10..40 deg, linearity 1.8, reduction 0..30 km/h -> 0..0.2, input tuning 1).
8. Trick lean (`TrickIntensity` 0 for the KTM: inactive).

`StoppieLeanLimitTuning` / `WheelieLeanLimitTuning` (pitch 0..20 and 0..40 deg) scale the limit while pitched; the `FallLeanLimit*`
values are separate runtime limits used by the fall check.

## Lean help [B: `0x1450c3ae0`]

Lean help adds a **side force at the front tyre** that pulls the actual side force towards the desired one:

```
u        = unit side direction of the front wheel;   F_act = F_contact . u                  # ActualSideForce
h        = clamp((1 - CurrentHelpReduction) * LeanHelpFactor * (1 - R) + A, 0, 1)            # CurrentLeanHelpFactor
F_ideal  = F_act + h (F_des - F_act)                                                          # IdealSideForce
cap      = LeanHelpLimit * Fy_peak(Fz) * sqrt(1 - (|F_contact - F_act u| / Fx_peak(Fz))^2)  # what the tyre can still give
F_total  = (|F_act| < |F_ideal|) ? clamp(F_ideal, -cap, cap) : F_ideal                       # TotalSideForce
F_applied= (F_total - F_act) u                                                                # AppliedSideForce, added at the front wheel
```

`R` and `A` are the **collision alteration** of the help (a reduction and an addition that follow a collision and fade), 0 without one; the
speed, throttle, brake, slip and skid tunings form the factor `h_factor` that replaces the plain `LeanHelpFactor` of this formula, and
the force is split into a force at the front contact and a couple by the balance. Both are in
[ride6-chassis-round6.md](ride6-chassis-round6.md) sections 2.1 and 2.2, with the pairing of every reduction with its input (resolved
in round 6). KTM: speed tuning 0..10 km/h -> 0..0.2 blended to 1 at a standstill, skid range 10..45 deg with linearity 2, balance 0
below 10 km/h and 1 above 30 km/h. `Fy_peak`, `Fx_peak` are the
peak forces of the front tyre model at its load (model virtuals `0x40`, `0x38` times the grip scales).

## Stability help [B structure: `0x14510e810`, `0x1450c6560`; pairing and application in round 6, [ride6-chassis-round6.md](ride6-chassis-round6.md) section 4]

```
S0 = StabilityHelpFactorSpeedTuning(speed) * max(0, |NormalizedLean|^StabilityHelpFactorLeanSensitivity * StabilityHelpFactorLeanInfluence + 1)
r_k = clamp01(T_k(x_k) * StabilityHelpFactorReductionScale * input_k * Lr)     k = throttle (input throttle), brake (input brake), front slip, rear slip
S   = S0 * (1 - r_brake) * (1 - r_front_slip) * (1 - r_rear_slip) * (1 - r_throttle)      # which input pairs with which slip term is [I]
S  *= (1 - skid term) * StabilityHelpFactorMultiplier ; S = 0 during a burnout ; S *= ramp over StabilityHelpFactorStoppedSpeedRange
S   = clamp01(((AntiFallStabilityHelpFactor - 1) * AntiFallFactor + 1) * (S + terrain term))
```

The KTM sets `StabilityHelpFactorSpeedTuning` to 0 and `StabilityHelpFactorLeanInfluence` 0, so `S0 = 0`: the stability help is
**off** for the stock bike except `OutOfTrackStabilityHelpFactorAddition` (0.025 on grass, sand and kerb). When active:
`h = (1 - R_s) S + A_s`, `a_des = h (omega_target - omega_roll) / max(dt, 0.001)` with `omega_roll` the roll rate and
`omega_target` the roll rate the current turn needs (`speed * yaw rate / ...`, [I]), scaled by `1 - gap / StabilityHelpMaxDistanceFromGround`
(wheel gap above ground), soft-limited to `StabilityHelpMaxAcceleration` by a tanh, multiplied by the speed ramp
`StabilityHelpSpeedRange` (active above its lower end) and applied as a roll torque `I_roll * a_des` about the forward axis.
Stored as `StabilityDesiredAcceleration`.

## Gyroscopic terms [B: `0x1450c7270` roll, `0x1450c4c00` yaw]

```
tau_roll = GyroscopicRollMomentumScale * cos(CurrentLeanAngle) * (I_front w_front + I_rear w_rear) * yaw_rate * 10000    # about the chassis forward axis
tau_yaw  = -GyroscopicYawMomentumScale * (I_front w_front + I_rear w_rear) * roll_rate * 10000                           # about the chassis vertical axis
```

`I` is the wheel `Inertia` (kg m^2), `w` its spin (rad/s), `yaw_rate` and `roll_rate` the Z and X components of the chassis
local angular velocity (rad/s). Both scales default to 1 and the stock bikes keep them, so the earlier unknown factor `c` is
`cos(lean)`.

## Fall detection [B, round 6]

Each `FallCheckInterval` (1/60 s) the handler tests the lean beyond `FallLeanLimit` (a running value that follows the front tyre's
grip, `FallLeanLimitAirborne` 70 in the air), the pitch beyond `FallPitchLimit` 65, the angle between the velocity and the heading
beyond `FallSpeedDirAngleLimit` 85 (180 airborne) **above `FallSpeedDirAngleMinSpeed`**, the angular speed beyond `FallAngularSpeedLimit`
4.4 rad/s (7 airborne) **at any speed**, the rider's forces beyond `FallForceLimit` 2e6 (the speed gate `FallMinSpeed` 10 km/h belongs to
these), collision impulses. Any of them starts the fall (`StartFall`, event `BikeInputFallenEvent`); the "block fall"
assist (clamp above) keeps the lean at 97.5 % of the limit for up to `BlockFallTimeLimit` 1 s so a slide does not end the
run, then waits `BlockFallReEnableWaitTimeLimit` 5 s. The exact tests, their gates and the terrain terms are in
[ride6-chassis-round6.md](ride6-chassis-round6.md) section 3.
