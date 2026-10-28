# Two-wheel model: RIDE 6 parameters and the changes they need

The wave-2 implementer codes from this page. It maps every RIDE 6 field to the generic
[`BikeSpec`](../../libs/blackbox-vehicle/src/bike/spec.rs) (units: SI, radians, as that file), says which "choices" of
[vehicle-two-wheel.md](vehicle-two-wheel.md) are now facts, and lists the model changes with their equations. Field
meanings: [ride6-physics-vehicle.md](../formats/ride6-physics-vehicle.md), [ride6-physics-tyre.md](../formats/ride6-physics-tyre.md),
[ride6-physics-elements.md](../formats/ride6-physics-elements.md). Behaviour specs: [ride6-lean-controller.md](ride6-lean-controller.md),
[ride6-tyre.md](ride6-tyre.md), [ride6-suspension.md](ride6-suspension.md), [ride6-engine-drivetrain.md](ride6-engine-drivetrain.md),
[ride6-rider-help.md](ride6-rider-help.md), [ride6-rider-body-aero.md](ride6-rider-body-aero.md). Provenance:
[ride6-physics.md](../provenance/ride6-physics.md). Library code stays generic: the RIDE 6 names live only in the importer
(`mods/ride6-bikes`), which converts units and fills a `BikeSpec`.

## 1. Field mapping (importer: RIDE 6 data -> the `[bike]` table)

The importer is `mods/ride6-bikes/src/physics/` ([ride6-bikes.md](../modding/ride6-bikes.md)); the keys are those of
[bikes.md](../modding/bikes.md) and map one to one to `BikeSpec`. Axes: the manifest uses x right, y up, z forward, RIDE 6
x forward, y right, z up (a vector `(x, y, z)` of the data becomes `(y, z, x)`). Lengths of the chassis are cm in the data,
suspension lengths m, speeds in tuning ranges km/h, angles degrees, the slipper's torque range Unreal units (1 N m = 10000).
A resolved asset that leaves a field out takes the constructor default named in section 6 or the spec cited; a field with no
default is an error of the importer (the field is named). **Status:** fact = in the data or the binary, default = a
constructor default recorded in a spec, choice = a named choice of the importer.

| Manifest key | RIDE 6 field | Conversion | Status |
|---|---|---|---|
| `drag` | `Aerodynamic.DragCoefficient` | none (`F = Cd v^2`, [aero spec](ride6-aero-drag.md)) | fact |
| `gyro_roll_scale`, `gyro_yaw_scale` | handler `GyroscopicRollMomentumScale`, `...Yaw...`; 0 when `SimulateGyroscopicEffects` is false | unset: 1 | default (lean spec) |
| `chassis.mass_kg` | `CHASSIS.Mass` | none | fact |
| `chassis.rider_mass_kg` | handler `Rider.Mass` (when `SimulateRider`) | none | fact |
| `chassis.rider_offset_m` | handler `Rider.COMOffset` | cm to m, axes; from the bike's own centre of gravity (the rider sits `CGPosition.z + COMOffset.z` high) | fact; the origin is [I] |
| `chassis.rider_forward_shift_m`, `..._backward_shift_m` | `Rider.ForwardOffset` (set by no asset: 0), `Rider.BackwardOffset` | cm to m, axes | fact / default (an `FVector` starts at 0) |
| `chassis.rider_weight_rate` | `InputData.WeightLongitudinal.InputRate` | per second | fact |
| `chassis.half_dimensions_m` | the converted body's bounds | none | mesh |
| `chassis.tensor_scale` | `CHASSIS.InertiaTensorScale` (x roll, y pitch, z yaw) | reorder to (pitch, yaw, roll) = (Y, Z, X) | fact |
| `chassis.cg_height_m`, `front_weight`, `wheelbase_m` | `CGPosition.z`, `WeightBalance`, `Skeleton.OverrideWheelBase` | cm to m, none, cm to m | fact; `WeightBalance` is a reading |
| `chassis.rake_rad`, `trail_m` | `Skeleton.RakeTrailOverride[0]` | deg to rad, cm to m | fact |
| `front`/`rear.radius_m`, `width_m` | the wheel meshes (`ShapeRadius`, `ShapeWidth` are set from them at spawn) | metres | mesh |
| `.toroid_factor`, `.mass_kg`, `.inertia_kgm2` | `ShapeToroidFactor`, `Mass`, `Inertia` | none | fact |
| `.tyre.a`, `.b`, `.c` | the `Tarmac` row of the tyre's grip table | `A0..A10, A111, A112, A12, A13`; `B0..B10`; `C0..C17` | fact |
| `.rolling` | `RollingResistanceConstant`, `...Velocity`, `...SqrVelocity` of the row | none | fact |
| `.lateral_grip_limit` | `LateralGripLimit` | none (informational: the code uses `atan` of the lateral grip, M3) | data |
| `.tire_stiffness_n_per_m`, `.max_tire_deformation_m`, `.compression_shape_factor` | `TireStiffness`, `MaxTireDeformation`, `TireCompressionShapeFactor` | none, cm to m, none | fact |
| `.slip_correction`, `.rest_slip_correction`, `.rest_speed_threshold_mps` | `SlipCorrection`, `RestSlipCorrection`, `RestSpeedThreshold` | none, none, km/h to m/s | data |
| `*_suspension.max_travel_m`, `.spring_n_per_m` | `MaxCompression` (m, at the wheel), `SpringStrength` (N/m; a **shock** rate for the rear) | none | fact |
| `.top_out_stiffness_n_per_m`, `.top_out_length_m`, `.rest_offset_m`, `.max_speed_mps` | `TopOutSpringStiffness`, `TopOutSpringLength`, `RestCompressionOffset`, `SpringMaxSpeed` | none (m, m/s) | fact |
| `.leverage` | `SuspensionLeverageRatio` (external curve if set, else the embedded keys) | share of travel, wheel travel per shock travel, never below 1; a cubic curve is sampled at 8 points per span | fact; the sampling approximates the spline |
| `.damper.*` | `DamperSlowBump`, `DamperFastBump`, `DamperFastBumpThreshold`, `...Rebound...` | none (N s/m, m/s) | fact |
| `.progressive` | `LinearSpringCompressionRatio`, `ProgressiveSpringFactor`, `...Linearity`; written only when the factor is not 0 | none | fact / default |
| `.hydraulic` | `HydraulicComponentStiffness`, `LinearHydraulicComponentCompressionRatio`, `ProgressiveHydraulicComponentFactor`, `...Linearity` | none | fact / default |
| `.coupling` | `ChainPullFactor`, `TractionInfluenceOnLoad`, `BrakeTorqueCouplingFactor` and the `...CompressionFactor` remaps | the remap's output at the travel share 0 and 1 (identity when unset); a factor that is 0 is left out | fact |
| `engine.peak_torque_nm`, `.moi_kgm2`, `.friction_torque_nm`, `.throttle_at_idle` | `PeakTorque`, `MOI`, `EngineFrictionPeakTorque`, `ThrottleAtMinimum` | none | fact |
| `engine.torque_curve` | the **external** curve of `TorqueCurve` (`(rpm, normalised)`) | none; over 32 points the keys beyond the rev limit go, then the least significant | fact; the reduction is a choice |
| `engine.friction_curve` | `EngineFrictionCurve` | rpm over `MaxRPM` when `NormalizeEngineFrictionCurveDomain` is false | fact |
| `engine.idle_rpm`, `.max_rpm`, `.limiter_cut_time_s` | `MinRPM` (else `MinRPMLimit`), `MaxRPM`, the mean of `EngineLimiter[0]...DurationRange` | none | fact |
| `transmission.gears`, `.final_ratio` | `ForwardGears[].Ratio`, `FinalRatio` | none | fact |
| `transmission.efficiency` | none found in the binary | 1.0 | default |
| `transmission.up_shift`, `.down_shift` | the points the handler generates from the torque curve, the ratios and `GearChangePoints[].Change{Up,Down}Tolerance` ([gearbox spec](ride6-gearbox-auto.md) section 1) | the lowest change-up share over the gears; the lowest change-down share, each as a share of the rev limit in the gear the bike is in | fact for the points; **choice** for the reduction to one number each |
| `transmission.up_shift_time_s`, `.down_shift_time_s`, `.blip_throttle` | handler `GearUpChangeTime`, `GearDownChangeTime`, `BlipperThrottleOnShift` | none | fact |
| `transmission.clutch_linearity`, `.clutch_max_factor` | `Transmission.ClutchLinearity`; the C++ default `ClutchMaxTorqueFactor` | none; 1 | fact / default |
| `transmission.slipper_range_nm`, `.slipper_intensity` | `SlipperClutchDeltaTorqueRange`, `SlipperClutchMaxIntensity` | Unreal units to N m (/10000) | fact |
| `transmission.release_speed_mps`, `.feather_factor`, `.feather_max_factor` | `ClucthReleaseVelocityRange.Y`; `ClutchFeatheringPeakTorqueRPMFactor`, `...MaxRPMFactor` (when `bEnable`) | km/h to m/s; none | fact |
| `brakes.front_force_n` | `Brake.BrakeForce * BrakeBalance` (balance 1 for every bike; another balance is refused) | none (N at the patch, `T = F R`) | fact |
| `brakes.rear_force_n` | `Brake.HandbrakeForce` of the wheel with `bAffectedByHandbrake` (the rear), else 0 | none | fact |
| `brakes.joined_factor`, `.joined_front`, `.joined_rear` | handler `JoinedBrakesFactor`, `JoinedBrakesFrontFactor`, `...RearFactor` (unset: 0) | none | fact / default |
| `brakes.abs.*` | the ABS element: `FrontABSTuning`/`RearABSTuning` `.Intensity` (when enabled), `NormalizedSlipTuning.InputRange`, `ABSSpeedTuning.InputRange` | km/h to m/s for the speeds | fact |
| `steering.max_angle_rad` | `FrontWheel.SteerAngle` | deg to rad | fact |
| `steering.response`, `.damping_ratio` | `HandlebarK`, `HandlebarDamp` | ratio `z = abs(damp)` (a positive damp is the coefficient `c = 2 sqrt(K) z`) | fact (M3) |
| `steering.max_lean_rad`, `fall.lean_rad` | `FallLeanLimitAirborne` (70 deg for all 317 bikes) | deg to rad | **choice**: the ground limit is a run-time value (grip based); the upper bound of the lean limit is the fall lean |
| `steering.lean.k`, `.damping_ratio` | `LeanK`, `LeanDamp` | ratio as above | fact |
| `steering.lean.damping_multiplier` | `LeanComponent` ranges and the bike's size | the formula after this table | structure [B], operands [I] |
| `steering.lean.steer_k`, `.steer_damping_ratio`, `.target_k`, `.target_damping_ratio` | `SteerK/SteerDamp`, `TargetLeanAngleFilter.KValue/DampValue` | ratio as above | fact |
| `steering.lean.time_scale*` | `LeanTimeScale`, `LeanTimeScaleSpeedTuning` (km/h), `LeanTimeScaleInputTuning`; attenuation and sensitivity unset: 0 | speeds to m/s; a cubic curve is sampled | fact / default |
| `steering.lean.g_tuning`, `.skid_*`, `.stopped_*` | `LeanLimitGForceTuning`; `LeanLimitSkidRange` (deg), `...Linearity`, `...Reduction` (km/h); `StoppedLeanLimit` (deg), `StoppedSpeedThreshold` (km/h) | deg to rad, km/h to m/s | fact |
| `steering.lean.blend_range_mps` | `SteerAlignmentRange` (2 to 4) | none | the lean spec calls it `LowSpeedSteerBlendRange`; no asset sets that name |
| `steering.lean.rider_mass_scale`, `.help_speed`, `.help_limit` | `RiderMassScale`; `LeanHelpFactorSpeedTuning` (km/h); `LeanHelpLimitSpeedTuning` output at the top of its range | km/h to m/s | fact; `help_limit` is one number where the data has a speed curve |
| `fall.pitch_rad`, `.speed_direction_rad`, `.angular_speed`, `.min_speed_mps` | `FallPitchLimit`, `FallSpeedDirAngleLimit`, `FallAngularSpeedLimit`, `FallMinSpeed` | deg to rad; km/h to m/s | fact |
| `wheelie.wheelie.*`, `.stoppie.*` | `WheelieParams`, `StoppieParams` (unset fields: the constructor defaults of the [rider-help spec](ride6-rider-help.md)); `lean_limit_rad`, `lean_pitch_range_rad` from the handler's `WheelieLeanLimitTuning`, `StoppieLeanLimitTuning` | deg to rad, km/h to m/s | fact / default |
| `wheelie.throttle_pitch_rad`, `.forced_throttle_pitch_rad`, `.throttle_pitch_speed`, `.brake_pitch_rad`, `.brake_pitch_speed` | rider help `AutoRiderWheelieThrottleControl`, `...ForcedWheelie...`, `PitchSpeedWheelieThrottleControl`, `AutoRiderStoppieBrakeControl`, `PitchSpeedStoppieBrakeControl` | deg to rad | fact |

**The lean damping multiplier.** The handler's tick (function `0x1450e0480`, build `5.6.1-1697004`) sets
`LeanComponent.LeanDampMultiplier` each tick from four reference ranges of the component (`W` weight 180 to 352 kg, `T` tensor
x 0.2 to 0.3, `H` height 80 to 51 cm, `L` wheelbase 131 to 167 cm) and the output range `D` (1 to 1.5):
`a = clamp01((J - W0 T0) / (W1 T1 - W0 T0))`, `b = clamp01((r - H0/L0) / (H1/L1 - H0/L0))`, `multiplier = D0 + (D1 - D0) a b`,
with `J` the roll inertia reference of the bike and `r` the height of the combined centre of gravity over the wheelbase. The
structure is read; the two operands are inferred: `J = I_x_scale (M + m_r) + m_r z_r^2` and `r = h_com / wheelbase` with
`h_com = (M h_cg + m_r (h_cg + z_r)) / (M + m_r)`. The library keeps one number, so the importer evaluates it at rest. For the
KTM it is about 1.012 (the whole range is 1 to 1.5, so a wrong operand moves the damping ratio by a few percent at most).

Not carried (no key in the manifest): the TCS, ECU, fuel and tyre-wear elements, tyre temperature and wear, the stability
help, the lean help's reductions, the swingarm's arc, `LeanHelpBalance`, `MaxAngleFactorInputTuning`,
`HelpIntensityTorqueTuning`, the `timescale` of the wheelie parameters, the rider's spring constants and the rider's drag.

**KTM 450 SMR 2022 in these units** (for tests): total mass 183 kg; `I_roll = 36.6`, `I_pitch = 73.2`, `I_yaw = 82.35 kg m^2`;
cg height 0.67 m; wheelbase 1.45 m; rake 26.1 deg; trail 0.105 m; front travel 0.285 m, spring 9000 N/m, rear travel 0.266 m,
shock spring 51000 N/m, leverage 2.8 to 1.97; wheel masses 18 and 20 kg; peak torque 48 N m (curve 0.7375 at 1652 rpm,
1.0 at 7159 rpm, 0.6 at 11000), friction 30 N m at 11000 rpm, `MOI 0.015`; gears 2.0, 1.625, 1.333, 1.142, 0.954, final 7.45;
front brake 1880 N, rear brake 1000 N.

## 2. What the data turned from a choice into a fact

| Item of the old table | Result |
|---|---|
| `BrakeForce` units and distribution | Newtons at the tyre patch: `BrakeForce` is the front brake (`BrakeBalance` is 1 for every bike), `HandbrakeForce` the rear brake of the wheel with `bAffectedByHandbrake` ([drivetrain spec](ride6-engine-drivetrain.md), "Brake torque and its distribution"); **joined brakes** known (see M8) |
| `EngineFrictionPeakTorque` | drag torque in N m at `MaxRPM` times the friction curve `f(rpm / MaxRPM)`; closed throttle torque is `-T_fric (1 - r)` with `r` the ECU reduction |
| `InertiaTensorScale` base | **inertia = scale x total mass** (kg m^2), not a box; x roll, y pitch, z yaw; total mass includes the rider |
| `RestCompressionOffset`, `Hydraulic*`, `ProgressiveSpring*` | static preload (m); a second progressive spring term; the progressive term of the spring law (see the suspension spec) |
| `SlipCorrection`, `RestSlipCorrection` | combined-slip damping `1 / (abs(nx) k + 1)` with `k` blending from `SlipCorrection` to `RestSlipCorrection` at low speed [I]; the low-speed slip regularisation is a separate smoothstep (tyre spec) |
| `LateralGripLimit` | data only; the code uses the lateral grip peak `atan(mu)` for the lean limit |
| `TireStiffness` | series radial spring splitting the ground penetration between tyre and suspension |
| `ShapeToroidFactor`, `ShapeRadius`, `ShapeWidth` | `r_eff = R - 0.5 (1 - abs(cos camber)) factor width - compression`; radius and width from the mesh |
| Rider mass, `COMOffset`, weights | rider 70 kg, `COMOffset (-5, 0, 40) cm` (z used in the roll inertia term), separate spring-damper mass |
| Gear change points, tolerances, clutch feathering | generated points, the shift decision and the feathering controller: [ride6-gearbox-auto.md](ride6-gearbox-auto.md) |
| `LeanLimit*`, `LeanTimeScale*`, `LeanHelp*`, `LeanDamp`, `HandlebarK`, `SteerK`, `SteerDamp` | the lean controller of the lean spec replaces the pendulum controller |
| `WheelieControl`, `RiderHelp`, ABS, TCS | structure and thresholds in the rider-help spec |
| Tyre sections temperature and wear | integration, weights across the width and the efficiency: [ride6-tyre-thermal.md](ride6-tyre-thermal.md) |

## 3. Model changes (in priority order)

**M1 Inertia.** Replace the box inertia by `I = tensor_scale * (mass + rider_mass)` per axis (kg m^2), with the axis
reorder above. The rigid body then uses the total mass (bike + rider) at the combined CG (M2).

**M2 Rider as a mass.** The rider (70 kg) is attached at `COMOffset` (cm, bike frame: forward -5, up 40, relative to the
bike's frame origin of the asset) through a spring-damper (`RiderK`, `RiderDamp`, `RiderAngularK` 2500, `RiderAngularDamp`
250). Simplest faithful version: rigid body mass `M + m_r`, CG `= (M cg + m_r p_r) / (M + m_r)`, and the weight inputs shift
`p_r` through second-order filters (rider-body spec). The roll torque calculation (M3) uses `I_roll = scale_x (M + m_r)` plus
`m_r z_r^2`.

**M3 Lean controller (replaces `controller.rs`).** Implement the lean spec exactly: the stick scales the effective lean limit
(its order of terms is now recovered); the target lean passes the second-order filter (`K 1000, z 2.25`, stepped by
`dt * time_scale`); the spring-damper `a = -K (theta - target) - 2 z sqrt(K) theta'` (`z = |LeanDamp| * damp_multiplier`)
gives `tau = I_roll_total a`, `F_des = tau / arm` (`arm` ~ CG height); invert the front tyre's lateral curve
(`invFy`, MF94 with a bracketed root finder, tyre spec section 6) to get `alpha_des`; the wanted wheel heading is
`v_hat + alpha_des * side` and the steer angle is its heading projected onto the ground plane along the steering axis
(lean spec step 3); filter it (`HandlebarK 1000, z 3`). Lean help adds a side force at the front tyre
`(F_total - F_act) u` with the capped formula of the lean spec; the stability help is off for the stock bike. Below 2 to 4 m/s
blend to `stick * max_angle`.

**M4 Suspension.** (damper: the two-slope law of the suspension spec, `F_wheel = F(v/LR)/LR`, speeds in m/s). (a) Spring law `k (x + f sgn(x) max(|x| - r M, 0)^p)` plus the hydraulic term with its own `k, r, f, p`
(inputs in cm inside; see the suspension spec) and the top-out spring `k_t max(L_t - x, 0)`. (b) Rear leverage: shock
compression `x / LR(x/M)`, rate at the wheel `k / LR^2`. (c) Preload `RestCompressionOffset`. (d) The tyre's radial
stiffness `TireStiffness` in series: the share of the penetration taken by the tyre is `min(1, (k_s / LR) / TireStiffness)`
capped at `MaxTireDeformation` (3 cm); the code integrates the wheel mass, our "wheel has no mass in the ground solve"
approach is an accepted approximation if the wheel weight is added to the tyre load (kept). (e) Chain pull and brake
coupling: not recovered; optional.

**M5 Tyre.** (a) `kappa` and `alpha` replaced by the RIDE 6 definition: slip speed divided by the **total** contact speed,
with the smoothstep below 1 m/s (tyre spec section 2). (b) Combination of `Fx`, `Fy` by the normalised-slip rule with the
tyre's own peak slips (tyre spec section 3) instead of the force ellipse. The peaks come from the root equation of
[ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 3 (cache a 50 x 25 table per tyre: load 0.0365 to 3.6782 kN, camber 0 to 61 deg;
outside the load/camber range solve exactly, do not clamp).
(c) Efficiency factor `1 + 2 min(0.5, b)(e - 1)` for `Fx` and `1 + 2 min(0.5, 1 - b)(e - 1)` for `Fy` (constant 0.5, verified; `e = 1` without a
wear model, or the model of [ride6-tyre-thermal.md](ride6-tyre-thermal.md)); lateral model force sign-flipped; rolling resistance
`Fz (c0 + c1 v + c2 v^2 + cS |Slip.y|^p)` with the vehicle speed ([ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 6). (d) Aligning moment on the front wheel
only. (e) The camber sign: keep the earlier convention (the camber thrust pushes towards the lean).

**M6 Engine.** `T = T_closed + (T_max - T_closed) * throttle` with `T_closed = -T_fric(rpm) (1 - r)`, the throttle blended
about the zero-torque point `T_fric / (T_fric + T_max)` (ECU factors 1 when no electronics), the idle floor
`throttle >= 0.15` below `MinRPM`, and the limiter as an on/off cut of the throttle (period 0.04 s) above `MaxRPM`.
Crank inertia `MOI` and the clutch of the drivetrain spec replace the "launch speed" hack: the engine speed is a state;
the clutch is locked (shared inertia) or slipping (`T_clutch = e T_clutch_max sgn(delta w)`), `e = (1 - C)^1`; the
slipper opens when the back torque exceeds 25 N m (the 250000 to 400000 units /10000). `T_clutch_max = PeakTorque * ClutchMaxTorqueFactor` (type 1, the C++ default: 48 N m for the KTM). Fact.

**M7 Gearbox.** Fact, see [ride6-gearbox-auto.md](ride6-gearbox-auto.md). Generate the change points from the torque curve (sweep from
the rpm of peak torque, 1 % steps: shift up where the next gear's wheel force exceeds the current one, tolerance 1; shift down
at the rpm where it falls below `ChangeDownTolerance` times it, deprecated mode uses the lower gear's tolerance); every tick
compare the road speed (`|VehicleForwardSpeed|` for the KTM) with `speed(a' MaxRPM, g)` for an upshift and
`speed(f, g-1)` for a downshift (`speed = r_eff rpm 2 pi / 60 / (G FinalRatio)`, `r_eff` of the rear wheel). `GearUpChangeTime`
0.06 s and `GearDownChangeTime` 0.15 s, held in 1st at standstill; the lean / throttle hold-off factors raise the thresholds.
Not recovered: the forced downshift under braking and the over-limit timer writers.

**M8 Brakes.** `joined`: `rear = max(rear, front * 1.0)` when moving (`JoinedBrakesFactor`). Wheel brake torque:
front `T = brake_input * BrakeForce * R_eff_nominal * mult`, rear `T = hand_input * HandbrakeForce * R * mult` with `R`
the wheel's nominal radius (not `r_eff`) [fact: `0x145136640`, `0x145134de0`]. `mult = (1 - reduction)(1 - abs)` from ABS. ABS
ramp: from a normalised slip ratio of -0.3 (front), -0.5 (rear) to full at -0.5, -1, above 10 to 25 km/h.

**M9 Wheelie/stoppie and rider help.** Optional. Without them a bike with the engine torque and the rider shifting weight
will flip; implement the pitch tracker of the rider-help spec (target 35 deg wheelie, 45 deg stoppie, dead zones 0.3 / 0.95)
or clamp the pitch.

**M10 Gyroscopic torques.** `tau_roll = cos(lean) (I_f w_f + I_r w_r) yaw_rate` about the forward axis and `tau_yaw = -(I_f w_f + I_r w_r) roll_rate` about the vertical axis (both scales 1).

**M11 Air drag.** Fact: `F = Cd (1 - s) g v^2` newtons opposing the velocity, `Cd 0.15`, `s` the slipstream wake (0 alone), `g = 1`
(orientation tuning unset); the factor 2 of the earlier reading was wrong ([ride6-aero-drag.md](ride6-aero-drag.md)). The rider's tuck
reduces it separately.

**M12 Fall.** Use the thresholds of the lean spec (lean 70 deg, pitch 65 deg, speed-direction angle 85 deg, angular speed
4.4 rad/s) to end a run; the "block fall" assist (clamp the lean at 97.5 % of the limit for up to 1 s) is optional.

## 4. Still unknown (stated, not guessed)

(The brake torque code, the front/rear distribution and the damper law were resolved in round 3.) the inverse of the lateral curve and the exact geometry of the
handlebar angle (signs); the lean/stability help equations; the wheelie torque arm; the C++ defaults of the `FVehicleData` fields that
the assets do not set (`ProgressiveSpringFactor`, the camber terms; the earlier `ShapeRadius` source); the exact rider
spring constants (`RiderK`, `RiderDamp`, `PilotWeight*K`, C++ defaults);
the semantics of `WeightBalance` (front share by the magnitudes, not read in code) and of `bAdjustCGHeight`.

## 5. Status and what remains

**Resolved by the binary (rounds 2 and 3):** brake torque and distribution; the damper law; the C++ defaults of the
suspension, wheel, rider component, tyre data, transmission, brake data and wheelie parameters; the gear-change tables and the
launch clutch; the exact Magic Formula 94 evaluation, its coefficient formulas and its inverse; the handlebar-angle
geometry and the desired slip angle; the order of the lean-limit terms; the lean-help force; the stability-help structure
(off for the stock bike); the gyroscopic terms (`c = cos(lean)`); the wheelie/stoppie help torque (a pure pitch torque on the
bike's pitch inertia, no arm); chain pull, traction and brake coupling of the swingarm; the weight-control formula of the rider help.

**Resolved in round 4 (tyre/engine pass):** the MF94 evaluation object (equations, peak solver and tables, rolling resistance, no
combined solver exists), the automatic shift decision with the generated change points, the clutch feathering controller, air drag
and slipstream, tyre temperature/wear/efficiency (specs: [mf94](ride6-tyre-mf94.md), [thermal](ride6-tyre-thermal.md),
[gearbox](ride6-gearbox-auto.md), [aero](ride6-aero-drag.md)).

**Still open (with the addresses tried):** the forced downshift under braking (`ForceGearDownChangeBrake*`) and the writers of
`TimeOverLimit`, `NoGear*ChangeTimer`, `ClutchFeatheringTimer` (searched near `0x1450f1c20`, `0x1450d84a0`; the SDK offsets
`0x1858` to `0x3114` belong to a class not matched); the pairing of the four
stability/lean-help reductions with their inputs and the target roll rate; the arm used for `F_des`; the speed argument of the
wheelie help; the rider help throttle and brake control formulas (same ramp structure as the weight control, constants in the
elements page); the world-side inputs `AirTemperature` / `AsphaltTemperature` (see section 6).

## 6. C++ defaults the assets do not set (round 4) and what is still missing

Found in the constructors (all [B]; the bike assets override some, the KTM values are in the specs named):

| Object | Default values the bike model uses | Where |
|---|---|---|
| `FAerodynamicData` (`0x1450f9cb0`) | `DragCoefficient 0.35` (KTM asset 0.15), `WaterDragCoefficient 0`, `SlipstreamScale 1`, `SlipstreamTemperatureFactor 0`, orientation tuning `In (0,0)`, `Out (1,1)`, `Linearity 1`, no curve (so the factor is 1) | [ride6-aero-drag.md](ride6-aero-drag.md) |
| `USlipstreamPhysicsElement` (`0x1450bdcb0`) | `Persistence 0`, `MaxDistance 100` (asset 70), `DistanceFactor 0.05` (asset 0.085), `DistanceExponent 1`, `SideDistanceFactor 1.5` (asset 2), `SideDistanceExponent 2`, `UseVehicleWidth false` | idem |
| `UInputHandler` (`0x1450bd6d0`) | `bUseDeprecatedAutomaticTransmission true`, `AutomaticTransmissionSpeedFactor 0.5`, `ClucthMinRPMPercentage 0.4`, `ClucthMaxRPMPercentage 0.7`, `GearWhenStopped 0`, shift times 0.1 / 0.3, over-limit 10 km/h / 1 s / 0.5 s / 0.99, `NeutralGearMaxSpeed 50`, `ChangeUpMinSpeed 10`, `ChangeUp/DownSpeedCorrectionFactor 1` | [ride6-gearbox-auto.md](ride6-gearbox-auto.md) section 5 |
| `FClutchFeathering` (two instances) | `bEnable false`, `MaxRPMFactor 0`, `PeakTorqueRPMFactor 1`, `TargetRPMTolerance 0.6`, `ExitTolerance 0.8`, `ThrottleCap 1`, `MinSlip 0.1`, `DeltaTime 0.01`, `RPMRaiseFactor 0.05`, `ClutchReleaseFactor 1` | idem section 3 |
| `UTireWearVehicleElement` (`0x1450fe840`) | `TireDecayDistanceFactor 1`, `StaticTemperatureTimeScale 0.1`, `DynamicTemperatureTimeScale 1e-6`, `WearDecayIntensity 1`, `Enabled true`, `UpdateEfficiency true`, `UniformTiresComputation false` (bike element asset: scales 1 and 1, uniform true) | [ride6-tyre-thermal.md](ride6-tyre-thermal.md) |
| `FDecayData` (`UTireData.Decay`, `0x145146810`) | `MaxTemperature 200`, `NominalWearDecayLength 500000`, `TireSectionsDependency 1`, `TireSectionsAngleRad 1`, `TireSectionsHeatTransfer 0` (and Spess 3, SpessLimit 0, Temperature 70, AsphaltImportance 0.8, overridden by the KTM tyre) | idem |
| `FWheelData` (`0x1450fa6a0`) | `LongitudinalHelpFactor`, `LateralHelpFactor` not assigned (0 by zero-initialisation [I]; the KTM does not set them) | [ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 5 |

**Still missing for the bike model after this pass** (not in the assets, not found in a constructor or a writer):
`AirTemperature` and `AsphaltTemperature` of the vehicle (world/weather inputs, used by the tyre thermal model; a model needs a
choice, for example 20 / 30 deg C, to be stated as such); the source of `EngineData.MaxTorqueRPM` (`engine+0x50`, the start of the
shift sweep and the feathering target: use the maximum of `PeakTorque * curve`, 7159 rpm for the KTM, [I]); the writers of the
shift hold-off timers and of `TimeOverLimit`; the forced downshift under braking; the wheel's external efficiency factor
(`wheel+0x31C`, 1 at setup) and the `SlipCorrectionMultiplier` / grip-reduction writers (wet, burnout); the rider help and lean
items listed in section 5 (owned by the lean/stability pass).
